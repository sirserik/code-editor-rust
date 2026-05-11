use super::*;

impl CodeEditorApp {
    pub(super) fn render_terminal(&mut self, ctx: &egui::Context) {
        let dark = self.app.settings.theme.resolved() != Theme::Light;
        let tc = self.tc;
        let term_bg = if dark { Color32::from_rgb(30, 32, 38) } else { Color32::from_rgb(255, 255, 255) };
        let term_fg = if dark { Color32::from_rgb(200, 204, 212) } else { Color32::from_rgb(30, 30, 30) };

        // Update terminal grid with new output
        if let Some(id) = self.app.active_terminal {
            self.app.terminal.update_grid(id);
        }

        egui::TopBottomPanel::bottom("terminal")
            .resizable(true)
            .default_height(200.0)
            .min_height(100.0)
            .max_height(500.0)
            .frame(egui::Frame::NONE.fill(term_bg).inner_margin(0.0))
            .show(ctx, |ui| {
                // Top border + header
                let header_rect = ui.max_rect();
                ui.painter().line_segment(
                    [Pos2::new(header_rect.min.x, header_rect.min.y),
                     Pos2::new(header_rect.max.x, header_rect.min.y)],
                    Stroke::new(1.0, tc.border),
                );

                // Header bar
                let (hdr, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 28.0), egui::Sense::hover());
                let hdr_bg = if dark { Color32::from_rgb(38, 40, 48) } else { Color32::from_rgb(243, 243, 243) };
                ui.painter().rect_filled(hdr, CornerRadius::ZERO, hdr_bg);
                ui.painter().text(
                    Pos2::new(hdr.min.x + 12.0, hdr.min.y + 7.0),
                    egui::Align2::LEFT_TOP,
                    "Terminal", FontId::monospace(11.5), tc.fg_dim,
                );

                // Close button
                let close_rect = Rect::from_min_size(
                    Pos2::new(hdr.max.x - 28.0, hdr.min.y + 4.0),
                    Vec2::new(20.0, 20.0),
                );
                let close_resp = ui.allocate_rect(close_rect, egui::Sense::click());
                ui.painter().text(close_rect.center(), egui::Align2::CENTER_CENTER,
                    "×", FontId::monospace(14.0), tc.fg_dim);
                if close_resp.hovered() {
                    ui.painter().rect_filled(close_rect, CornerRadius::same(3),
                        if dark { Color32::from_rgb(180, 50, 50) } else { Color32::from_rgb(220, 80, 80) });
                    ui.painter().text(close_rect.center(), egui::Align2::CENTER_CENTER,
                        "×", FontId::monospace(14.0), Color32::WHITE);
                }
                if close_resp.clicked() {
                    self.app.show_terminal = false;
                    self.app.focus = Focus::Editor;
                }

                // Terminal content
                if let Some(id) = self.app.active_terminal {
                    if let Some(grid) = self.app.terminal.grids.get(&id) {
                        let font = FontId::monospace(13.0);
                        let lh = ui.fonts(|f| f.row_height(&font)) + 2.0;

                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .stick_to_bottom(true)
                            .show(ui, |ui| {
                                let lines = grid.visible_lines();
                                for (row, line) in lines.iter().enumerate() {
                                    if !line.is_empty() {
                                        let (rect, _) = ui.allocate_exact_size(
                                            Vec2::new(ui.available_width(), lh),
                                            egui::Sense::hover(),
                                        );
                                        // Cursor highlight
                                        if row == grid.cursor_row {
                                            ui.painter().rect_filled(rect, CornerRadius::ZERO,
                                                if dark { Color32::from_rgb(40, 44, 52) } else { Color32::from_rgb(248, 248, 248) });
                                        }
                                        ui.painter().text(
                                            Pos2::new(rect.min.x + 8.0, rect.min.y + 1.0),
                                            egui::Align2::LEFT_TOP,
                                            line, font.clone(), term_fg,
                                        );
                                    } else {
                                        ui.allocate_exact_size(Vec2::new(ui.available_width(), lh), egui::Sense::hover());
                                    }
                                }
                            });

                        // Handle terminal input when focused
                        if self.app.focus == Focus::Terminal {
                            let events = ctx.input(|i| i.events.clone());
                            for event in &events {
                                match event {
                                    egui::Event::Text(text) => {
                                        let _ = self.app.terminal.write(id, text.as_bytes());
                                    }
                                    egui::Event::Key { key, pressed: true, modifiers, .. } => {
                                        let data: Option<&[u8]> = match key {
                                            egui::Key::Enter => Some(b"\r"),
                                            egui::Key::Backspace => Some(b"\x7f"),
                                            egui::Key::Tab => Some(b"\t"),
                                            egui::Key::Escape => Some(b"\x1b"),
                                            egui::Key::ArrowUp => Some(b"\x1b[A"),
                                            egui::Key::ArrowDown => Some(b"\x1b[B"),
                                            egui::Key::ArrowRight => Some(b"\x1b[C"),
                                            egui::Key::ArrowLeft => Some(b"\x1b[D"),
                                            egui::Key::C if modifiers.command => {
                                                // Ctrl+C -> send SIGINT
                                                Some(b"\x03")
                                            }
                                            egui::Key::D if modifiers.command => {
                                                Some(b"\x04") // EOF
                                            }
                                            _ => None,
                                        };
                                        if let Some(d) = data {
                                            let _ = self.app.terminal.write(id, d);
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            // Request repaint for terminal updates
                            ctx.request_repaint_after(std::time::Duration::from_millis(50));
                        }
                    }
                } else {
                    // No terminal — show prompt to create
                    ui.vertical_centered(|ui| {
                        ui.add_space(20.0);
                        if ui.add(egui::Button::new(
                            RichText::new("Start Terminal").font(FontId::monospace(13.0)).color(tc.accent)
                        ).fill(Color32::TRANSPARENT)).clicked() {
                            let dir = self.app.file_tree.root_path.as_deref();
                            if let Ok(id) = self.app.terminal.spawn(dir) {
                                self.app.active_terminal = Some(id);
                                self.app.focus = Focus::Terminal;
                            }
                        }
                    });
                }

                // Click to focus terminal
                let term_rect = ui.max_rect();
                if ui.input(|i| i.pointer.any_pressed()) {
                    if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
                        if term_rect.contains(pos) && self.app.active_terminal.is_some() {
                            self.app.focus = Focus::Terminal;
                        }
                    }
                }
            });
    }
}
