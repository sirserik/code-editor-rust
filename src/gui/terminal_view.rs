//! Bottom panel: hosts the integrated PTY terminal and the Output console as tabs.
//!
//! Kept in `terminal_view.rs` to avoid file-rename churn; the module renders both tabs.

use super::*;
use crate::app::BottomTab;
use crate::output::OutputKind;

const HEADER_H: f32 = 28.0;

impl CodeEditorApp {
    pub(super) fn render_bottom_panel(&mut self, ctx: &egui::Context) {
        let dark = self.app.settings.theme.resolved() != Theme::Light;
        let tc = self.tc;
        let bg = if dark {
            Color32::from_rgb(30, 32, 38)
        } else {
            Color32::from_rgb(255, 255, 255)
        };

        // Pull pending PTY bytes into the grid once per frame.
        if let Some(id) = self.app.active_terminal {
            self.app.terminal.update_grid(id);
        }
        // Drain output events; force a repaint shortly after if anything moved.
        let output_changed = self.app.output.poll();
        if output_changed || self.app.output.is_running() {
            ctx.request_repaint_after(std::time::Duration::from_millis(80));
        }

        egui::TopBottomPanel::bottom("bottom_panel")
            .resizable(true)
            .default_height(220.0)
            .min_height(120.0)
            .max_height(700.0)
            .frame(egui::Frame::NONE.fill(bg).inner_margin(0.0))
            .show(ctx, |ui| {
                let panel_rect = ui.max_rect();
                // 1 px top border so the panel reads as a separate region.
                ui.painter().line_segment(
                    [
                        Pos2::new(panel_rect.min.x, panel_rect.min.y),
                        Pos2::new(panel_rect.max.x, panel_rect.min.y),
                    ],
                    Stroke::new(1.0, tc.border),
                );

                self.render_bottom_header(ui, dark);

                // Content takes whatever's left after the header.
                let content_rect = Rect::from_min_max(
                    Pos2::new(panel_rect.min.x, panel_rect.min.y + HEADER_H + 1.0),
                    panel_rect.max,
                );
                let mut content_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(content_rect)
                        .layout(egui::Layout::top_down(egui::Align::LEFT)),
                );

                match self.app.bottom_tab {
                    BottomTab::Terminal => self.render_terminal_content(&mut content_ui, ctx, dark),
                    BottomTab::Output => self.render_output_content(&mut content_ui, ctx, dark),
                }
            });
    }

    fn render_bottom_header(&mut self, ui: &mut egui::Ui, dark: bool) {
        let tc = self.tc;
        let panel_rect = ui.max_rect();
        let hdr = Rect::from_min_size(
            Pos2::new(panel_rect.min.x, panel_rect.min.y),
            Vec2::new(panel_rect.width(), HEADER_H),
        );
        let hdr_bg = if dark {
            Color32::from_rgb(38, 40, 48)
        } else {
            Color32::from_rgb(243, 243, 243)
        };
        ui.painter().rect_filled(hdr, CornerRadius::ZERO, hdr_bg);

        // Tabs (left-aligned).
        let tab_font = FontId::monospace(11.5);
        let tab_padding = 14.0;
        let mut x = hdr.min.x + 4.0;
        let tabs: &[(BottomTab, &str)] =
            &[(BottomTab::Terminal, "Terminal"), (BottomTab::Output, "Output")];
        for (tab, label) in tabs {
            let galley = ui.painter().layout_no_wrap(
                (*label).to_string(),
                tab_font.clone(),
                tc.fg,
            );
            let tab_w = galley.size().x + 2.0 * tab_padding;
            let tab_rect = Rect::from_min_size(Pos2::new(x, hdr.min.y), Vec2::new(tab_w, HEADER_H));
            let resp = ui.allocate_rect(tab_rect, egui::Sense::click());
            let active = self.app.bottom_tab == *tab;
            let text_color = if active { tc.fg } else { tc.fg_dim };
            ui.painter().text(
                tab_rect.center(),
                egui::Align2::CENTER_CENTER,
                *label,
                tab_font.clone(),
                text_color,
            );
            if active {
                ui.painter().rect_filled(
                    Rect::from_min_size(
                        Pos2::new(tab_rect.min.x + tab_padding * 0.5, tab_rect.max.y - 2.0),
                        Vec2::new(tab_rect.width() - tab_padding, 2.0),
                    ),
                    CornerRadius::same(1),
                    tc.accent,
                );
            }
            if resp.clicked() {
                self.app.bottom_tab = *tab;
                self.app.focus = match tab {
                    BottomTab::Terminal => Focus::Terminal,
                    BottomTab::Output => Focus::Output,
                };
            }
            x += tab_w;
        }

        // Right-aligned: per-tab actions + close.
        let mut right_x = hdr.max.x - 4.0;
        let btn_size = 22.0;

        let close_rect = Rect::from_min_size(
            Pos2::new(right_x - btn_size, hdr.min.y + 3.0),
            Vec2::new(btn_size, btn_size),
        );
        let close_resp = ui.allocate_rect(close_rect, egui::Sense::click());
        if close_resp.hovered() {
            ui.painter().rect_filled(
                close_rect,
                CornerRadius::same(3),
                if dark {
                    Color32::from_rgb(180, 50, 50)
                } else {
                    Color32::from_rgb(220, 80, 80)
                },
            );
            ui.painter().text(
                close_rect.center(),
                egui::Align2::CENTER_CENTER,
                "×",
                FontId::monospace(14.0),
                Color32::WHITE,
            );
        } else {
            ui.painter().text(
                close_rect.center(),
                egui::Align2::CENTER_CENTER,
                "×",
                FontId::monospace(14.0),
                tc.fg_dim,
            );
        }
        if close_resp.clicked() {
            self.app.show_bottom_panel = false;
            self.app.focus = Focus::Editor;
        }
        right_x -= btn_size + 4.0;

        if self.app.bottom_tab == BottomTab::Output {
            // Stop / Clear / Re-run buttons + status text.
            if self.action_button(ui, &mut right_x, hdr, dark, "↻", "Re-run last command") {
                self.app.run_last();
            }
            if self.action_button(ui, &mut right_x, hdr, dark, "✕", "Clear output") {
                self.app.output.clear();
            }
            let stop_enabled = self.app.output.is_running();
            if stop_enabled
                && self.action_button(ui, &mut right_x, hdr, dark, "■", "Stop running command")
            {
                self.app.output.stop();
            }

            // Status label (running indicator / exit code).
            let (label, color) = if self.app.output.is_running() {
                ("● running".to_string(), tc.orange)
            } else if let Some(code) = self.app.output.exit_code {
                if code == 0 {
                    ("● ok".to_string(), tc.green)
                } else {
                    (format!("● exit {}", code), tc.red)
                }
            } else {
                (String::new(), tc.fg_dim)
            };
            if !label.is_empty() {
                let font = FontId::monospace(11.0);
                let g = ui.painter().layout_no_wrap(label.clone(), font.clone(), color);
                let w = g.size().x;
                right_x -= w + 8.0;
                ui.painter().text(
                    Pos2::new(right_x, hdr.center().y),
                    egui::Align2::LEFT_CENTER,
                    label,
                    font,
                    color,
                );
            }
            let _ = right_x; // last consumer; keep variable for symmetry with other branches.
        }
    }

    /// Small 22×22 header icon button. Returns true when clicked.
    fn action_button(
        &self,
        ui: &mut egui::Ui,
        right_x: &mut f32,
        hdr: Rect,
        dark: bool,
        glyph: &str,
        tooltip: &str,
    ) -> bool {
        let tc = self.tc;
        let size = 22.0;
        let rect = Rect::from_min_size(
            Pos2::new(*right_x - size, hdr.min.y + 3.0),
            Vec2::new(size, size),
        );
        let resp = ui.allocate_rect(rect, egui::Sense::click());
        let resp = resp.on_hover_text(tooltip);
        let hover_bg = if dark {
            Color32::from_rgb(58, 62, 72)
        } else {
            Color32::from_rgb(225, 225, 225)
        };
        if resp.hovered() {
            ui.painter().rect_filled(rect, CornerRadius::same(3), hover_bg);
        }
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            glyph,
            FontId::monospace(13.0),
            tc.fg_dim,
        );
        *right_x -= size + 2.0;
        resp.clicked()
    }

    fn render_terminal_content(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        dark: bool,
    ) {
        let tc = self.tc;
        let term_fg = if dark {
            Color32::from_rgb(200, 204, 212)
        } else {
            Color32::from_rgb(30, 30, 30)
        };

        if let Some(id) = self.app.active_terminal {
            if let Some(grid) = self.app.terminal.grids.get(&id) {
                let font = FontId::monospace(13.0);
                let lh = ui.fonts(|f| f.row_height(&font)) + 2.0;

                let avail = ui.available_size();
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .stick_to_bottom(true)
                    .max_height(avail.y)
                    .show(ui, |ui| {
                        let lines = grid.visible_lines();
                        for (row, line) in lines.iter().enumerate() {
                            let (rect, _) = ui.allocate_exact_size(
                                Vec2::new(ui.available_width(), lh),
                                egui::Sense::hover(),
                            );
                            if row == grid.cursor_row {
                                ui.painter().rect_filled(
                                    rect,
                                    CornerRadius::ZERO,
                                    if dark {
                                        Color32::from_rgb(40, 44, 52)
                                    } else {
                                        Color32::from_rgb(248, 248, 248)
                                    },
                                );
                            }
                            if !line.is_empty() {
                                ui.painter().text(
                                    Pos2::new(rect.min.x + 8.0, rect.min.y + 1.0),
                                    egui::Align2::LEFT_TOP,
                                    line,
                                    font.clone(),
                                    term_fg,
                                );
                            }
                        }
                    });

                // Handle terminal input when the terminal tab is focused.
                if self.app.focus == Focus::Terminal {
                    let events = ctx.input(|i| i.events.clone());
                    for event in &events {
                        match event {
                            egui::Event::Text(text) => {
                                let _ = self.app.terminal.write(id, text.as_bytes());
                            }
                            egui::Event::Key {
                                key,
                                pressed: true,
                                modifiers,
                                ..
                            } => {
                                let data: Option<&[u8]> = match key {
                                    egui::Key::Enter => Some(b"\r"),
                                    egui::Key::Backspace => Some(b"\x7f"),
                                    egui::Key::Tab => Some(b"\t"),
                                    egui::Key::Escape => Some(b"\x1b"),
                                    egui::Key::ArrowUp => Some(b"\x1b[A"),
                                    egui::Key::ArrowDown => Some(b"\x1b[B"),
                                    egui::Key::ArrowRight => Some(b"\x1b[C"),
                                    egui::Key::ArrowLeft => Some(b"\x1b[D"),
                                    egui::Key::C if modifiers.command => Some(b"\x03"),
                                    egui::Key::D if modifiers.command => Some(b"\x04"),
                                    _ => None,
                                };
                                if let Some(d) = data {
                                    let _ = self.app.terminal.write(id, d);
                                }
                            }
                            _ => {}
                        }
                    }
                    ctx.request_repaint_after(std::time::Duration::from_millis(50));
                }

                // Click anywhere in the content area to focus the terminal.
                let term_rect = ui.max_rect();
                if ui.input(|i| i.pointer.any_pressed()) {
                    if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
                        if term_rect.contains(pos) {
                            self.app.focus = Focus::Terminal;
                        }
                    }
                }
                return;
            }
        }

        // No terminal yet — show start button.
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);
            if ui
                .add(
                    egui::Button::new(
                        RichText::new("Start Terminal")
                            .font(FontId::monospace(13.0))
                            .color(tc.accent),
                    )
                    .fill(Color32::TRANSPARENT),
                )
                .clicked()
            {
                let dir = self.app.file_tree.root_path.as_deref();
                if let Ok(id) = self.app.terminal.spawn(dir) {
                    self.app.active_terminal = Some(id);
                    self.app.focus = Focus::Terminal;
                }
            }
        });
    }

    fn render_output_content(
        &mut self,
        ui: &mut egui::Ui,
        _ctx: &egui::Context,
        dark: bool,
    ) {
        let tc = self.tc;
        let stdout_color = if dark {
            Color32::from_rgb(200, 204, 212)
        } else {
            Color32::from_rgb(30, 30, 30)
        };
        let stderr_color = if dark {
            Color32::from_rgb(240, 138, 138)
        } else {
            Color32::from_rgb(180, 40, 40)
        };
        let system_color = if dark {
            Color32::from_rgb(150, 180, 220)
        } else {
            Color32::from_rgb(40, 80, 160)
        };
        let link_color = tc.accent;

        // Empty-state hint.
        if self.app.output.lines.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(24.0);
                ui.label(
                    RichText::new("No output yet")
                        .font(FontId::monospace(13.0))
                        .color(tc.fg_dim),
                );
                ui.add_space(6.0);
                ui.label(
                    RichText::new("Run a command with ⌘R (build) or ⇧⌘R (tests)")
                        .font(FontId::monospace(11.5))
                        .color(tc.fg_dim),
                );
                ui.add_space(10.0);
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new(" Run Custom Command… ")
                                .font(FontId::monospace(12.0))
                                .color(tc.accent),
                        )
                        .stroke(Stroke::new(1.0, tc.border)),
                    )
                    .clicked()
                {
                    self.app.run_command_input.clear();
                    self.app.focus = Focus::RunCommandDialog;
                }
            });
            return;
        }

        // Collect click targets first (avoid mutating self while iterating lines).
        let mut nav_target: Option<crate::output::FileLink> = None;
        let font = FontId::monospace(12.5);
        let lh = ui.fonts(|f| f.row_height(&font)) + 1.0;
        let avail = ui.available_size();

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .stick_to_bottom(self.app.output.auto_scroll && self.app.output.is_running())
            .max_height(avail.y)
            .show(ui, |ui| {
                let line_count = self.app.output.lines.len();
                for i in 0..line_count {
                    let line = &self.app.output.lines[i];
                    let color = match line.kind {
                        OutputKind::Stdout => stdout_color,
                        OutputKind::Stderr => stderr_color,
                        OutputKind::System => system_color,
                    };

                    let (rect, _) = ui.allocate_exact_size(
                        Vec2::new(ui.available_width(), lh),
                        egui::Sense::hover(),
                    );

                    // The text galley itself is the click target when a link exists.
                    if let Some(link) = &line.link {
                        let resp = ui.interact(
                            rect,
                            egui::Id::new(("oc-line", i)),
                            egui::Sense::click(),
                        );
                        if resp.hovered() {
                            ui.painter().rect_filled(
                                rect,
                                CornerRadius::ZERO,
                                if dark {
                                    Color32::from_rgb(45, 48, 56)
                                } else {
                                    Color32::from_rgb(238, 240, 244)
                                },
                            );
                            ctx_set_cursor(ui.ctx(), egui::CursorIcon::PointingHand);
                        }
                        if resp.clicked() {
                            nav_target = Some(link.clone());
                        }
                        ui.painter().text(
                            Pos2::new(rect.min.x + 8.0, rect.min.y),
                            egui::Align2::LEFT_TOP,
                            &line.text,
                            font.clone(),
                            link_color,
                        );
                    } else {
                        ui.painter().text(
                            Pos2::new(rect.min.x + 8.0, rect.min.y),
                            egui::Align2::LEFT_TOP,
                            &line.text,
                            font.clone(),
                            color,
                        );
                    }
                }
            });

        if let Some(link) = nav_target {
            self.open_link(link);
        }
    }

    fn open_link(&mut self, link: crate::output::FileLink) {
        let path_str = link.path.to_string_lossy().to_string();
        if !link.path.exists() {
            self.app.status_message = format!("File not found: {}", path_str);
            return;
        }
        self.app.open_file(&path_str);
        let ed = self.app.active_editor_mut();
        ed.go_to_line(link.line);
        if link.col > 0 {
            let max_col = ed.buffer.line_len(link.line);
            ed.cursor.col = link.col.min(max_col);
        }
        ed.scroll_into_view();
        self.app.focus = Focus::Editor;
    }
}

fn ctx_set_cursor(ctx: &egui::Context, icon: egui::CursorIcon) {
    ctx.set_cursor_icon(icon);
}
