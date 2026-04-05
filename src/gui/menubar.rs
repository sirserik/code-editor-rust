use super::*;

impl CodeEditorApp {
    pub(super) fn render_menu_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menubar")
            .exact_height(26.0)
            .frame(egui::Frame::NONE.fill(self.tc.tab_bar_bg).inner_margin(egui::Margin { left: 8, right: 8, top: 3, bottom: 3 }))
            .show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    ui.menu_button(RichText::new("File").font(small()).color(self.tc.fg), |ui| {
                        if ui.button("New File          ⌘N").clicked() {
                            self.app.editors.push(crate::editor::Editor::new());
                            self.app.active_editor = self.app.editors.len() - 1;
                            self.app.focus = Focus::Editor;
                            ui.close_menu();
                        }
                        if ui.button("Open Folder...    ⌘O").clicked() {
                            self.app.pending_action = Some(PaletteAction::OpenFolder);
                            ui.close_menu();
                        }
                        if ui.button("Open File...").clicked() {
                            self.app.pending_action = Some(PaletteAction::OpenFile);
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Save              ⌘S").clicked() {
                            self.app.save_current();
                            ui.close_menu();
                        }
                        if ui.button("Close Tab         ⌘W").clicked() {
                            let i = self.app.active_editor;
                            self.app.close_tab(i);
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Quit              ⌘Q").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.menu_button(RichText::new("Edit").font(small()).color(self.tc.fg), |ui| {
                        if ui.button("Undo              ⌘Z").clicked() {
                            self.app.active_editor_mut().undo();
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Find              ⌘F").clicked() {
                            self.app.focus = Focus::FindReplace;
                            ui.close_menu();
                        }
                        if ui.button("Go to Line        ⌘G").clicked() {
                            self.app.focus = Focus::GoToLine;
                            self.app.goto_input.clear();
                            ui.close_menu();
                        }
                    });
                    ui.menu_button(RichText::new("View").font(small()).color(self.tc.fg), |ui| {
                        if ui.button("Command Palette   ⇧⌘P").clicked() {
                            self.app.focus = Focus::CommandPalette;
                            self.app.palette_input.clear();
                            self.app.palette_selected = 0;
                            ui.close_menu();
                        }
                        if ui.button("Quick Open        ⌘P").clicked() {
                            self.app.focus = Focus::QuickOpen;
                            self.app.quick_open_input.clear();
                            self.app.quick_open_results.clear();
                            ui.close_menu();
                        }
                        ui.separator();
                        let sidebar_label = if self.app.show_sidebar { "Hide Sidebar      ⌘B" } else { "Show Sidebar      ⌘B" };
                        if ui.button(sidebar_label).clicked() {
                            self.app.show_sidebar = !self.app.show_sidebar;
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Toggle Word Wrap").clicked() {
                            self.app.settings.word_wrap = !self.app.settings.word_wrap;
                            self.app.settings.save();
                            ui.close_menu();
                        }
                        if ui.button("Toggle Line Numbers").clicked() {
                            self.app.settings.show_line_numbers = !self.app.settings.show_line_numbers;
                            self.app.settings.save();
                            ui.close_menu();
                        }
                        let minimap_label = if self.app.show_minimap { "Hide Minimap" } else { "Show Minimap" };
                        if ui.button(minimap_label).clicked() {
                            self.app.show_minimap = !self.app.show_minimap;
                            ui.close_menu();
                        }
                        let bc_label = if self.app.show_breadcrumbs { "Hide Breadcrumbs" } else { "Show Breadcrumbs" };
                        if ui.button(bc_label).clicked() {
                            self.app.show_breadcrumbs = !self.app.show_breadcrumbs;
                            ui.close_menu();
                        }
                        let as_label = if self.app.auto_save_enabled { "Disable Auto-Save" } else { "Enable Auto-Save" };
                        if ui.button(as_label).clicked() {
                            self.app.auto_save_enabled = !self.app.auto_save_enabled;
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Zoom In            ⌘+").clicked() {
                            self.app.settings.font_size = (self.app.settings.font_size + 1.0).min(32.0);
                            self.app.settings.save();
                            ui.close_menu();
                        }
                        if ui.button("Zoom Out           ⌘-").clicked() {
                            self.app.settings.font_size = (self.app.settings.font_size - 1.0).max(8.0);
                            self.app.settings.save();
                            ui.close_menu();
                        }
                        if ui.button("Reset Zoom         ⌘0").clicked() {
                            self.app.settings.font_size = DEFAULT_FONT_SIZE;
                            self.app.settings.save();
                            ui.close_menu();
                        }
                    });
                    ui.menu_button(RichText::new("Help").font(small()).color(self.tc.fg), |ui| {
                        if ui.button("Keyboard Shortcuts").clicked() {
                            self.app.focus = Focus::About;
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("About Code Editor").clicked() {
                            self.app.focus = Focus::About;
                            ui.close_menu();
                        }
                    });
                    ui.menu_button(RichText::new("Theme").font(small()).color(self.tc.fg), |ui| {
                        for &theme in Theme::ALL {
                            let active = self.app.settings.theme == theme;
                            let label = if active { format!("● {}", theme.name()) } else { format!("  {}", theme.name()) };
                            if ui.button(label).clicked() {
                                self.app.settings.theme = theme;
                                self.app.settings.save();
                                ui.close_menu();
                            }
                        }
                    });
                });
            });
    }

    pub(super) fn render_tabs(&mut self, ctx: &egui::Context) {
        let mut tab_to_close: Option<usize> = None;
        let dark = self.app.settings.theme != Theme::Light;
        egui::TopBottomPanel::top("tabs")
            .exact_height(36.0)
            .frame(egui::Frame::NONE.fill(self.tc.tab_bar_bg).inner_margin(egui::Margin { left: 6, right: 6, top: 4, bottom: 0 }))
            .show(ctx, |ui| {
                let panel_rect = ui.max_rect();
                // Bottom border
                ui.painter().line_segment(
                    [Pos2::new(panel_rect.min.x, panel_rect.max.y), Pos2::new(panel_rect.max.x, panel_rect.max.y)],
                    Stroke::new(1.0, self.tc.border),
                );
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    for i in 0..self.app.editors.len() {
                        let name = self.app.editors[i].file_name();
                        let dirty = self.app.editors[i].is_dirty;
                        let active = i == self.app.active_editor;
                        let text_c = if active { self.tc.fg } else { self.tc.fg_dim };
                        let bg = if active { self.tc.bg } else { Color32::TRANSPARENT };
                        let rounding = Rounding { nw: 6, ne: 6, sw: 0, se: 0 };

                        let frame = egui::Frame::NONE.fill(bg).rounding(rounding)
                            .inner_margin(egui::Margin { left: 10, right: 4, top: 4, bottom: 4 });

                        let frame_resp = frame.show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 4.0;
                                // Dirty indicator as colored dot
                                let dot = if dirty { " ●" } else { "" };
                                let dot_color = if dirty { self.tc.orange } else { text_c };
                                let label_resp = ui.add(egui::Label::new(
                                    RichText::new(format!("{}{}", name, dot)).font(small()).color(
                                        if dirty && !dot.is_empty() { text_c } else { text_c }
                                    )
                                ).sense(egui::Sense::click()));
                                if label_resp.clicked() {
                                    self.app.active_editor = i;
                                    self.app.focus = Focus::Editor;
                                }
                                // Close button with hover state
                                let close_resp = ui.add(
                                    egui::Button::new(
                                        RichText::new("×").font(FontId::monospace(14.0)).color(self.tc.fg_dim)
                                    )
                                    .frame(false)
                                    .min_size(egui::vec2(18.0, 18.0))
                                );
                                // Red hover effect on close button
                                if close_resp.hovered() {
                                    let cr = close_resp.rect;
                                    ui.painter().rect_filled(cr, Rounding::same(3),
                                        if dark { Color32::from_rgb(180, 50, 50) } else { Color32::from_rgb(220, 80, 80) });
                                    ui.painter().text(cr.center(), egui::Align2::CENTER_CENTER, "×",
                                        FontId::monospace(14.0), Color32::WHITE);
                                }
                                if close_resp.clicked() {
                                    tab_to_close = Some(i);
                                }
                            });
                        });

                        // Active tab accent underline
                        if active {
                            let tab_rect = frame_resp.response.rect;
                            ui.painter().rect_filled(
                                Rect::from_min_size(
                                    Pos2::new(tab_rect.min.x + 2.0, tab_rect.max.y - 2.0),
                                    Vec2::new(tab_rect.width() - 4.0, 2.0),
                                ),
                                Rounding::same(1), self.tc.accent,
                            );
                        }
                    }
                });
            });
        if let Some(i) = tab_to_close {
            self.app.close_tab(i);
        }
    }

    pub(super) fn render_status(&mut self, ctx: &egui::Context) {
        let tc = self.tc;
        let dark = self.app.settings.theme != Theme::Light;
        let status_bg = if dark {
            Color32::from_rgb(24, 24, 36)
        } else {
            Color32::from_rgb(0, 122, 204) // VS Code blue status bar for light theme
        };
        let status_fg = if dark { tc.fg_dim } else { Color32::WHITE };
        let status_accent = if dark { tc.fg } else { Color32::WHITE };
        let sf = FontId::monospace(11.5);

        egui::TopBottomPanel::bottom("status")
            .exact_height(26.0)
            .frame(egui::Frame::NONE.fill(status_bg).inner_margin(egui::Margin::symmetric(10, 3)))
            .show(ctx, |ui| {
                let ed = &self.app.editors[self.app.active_editor];
                let lang = ed.file_path.as_ref().map(|p| syntax::detect_language(p)).unwrap_or("Text");
                let line = ed.cursor.line + 1;
                let col = ed.cursor.col + 1;
                let dirty = ed.is_dirty;
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing.x = 16.0;
                    // Left: branch + status
                    if let Some(ref gs) = self.app.git_status {
                        if gs.is_repo {
                            ui.label(RichText::new(format!("⎇ {}", gs.branch)).font(sf.clone()).color(status_accent));
                        }
                    }
                    if dirty {
                        ui.label(RichText::new("● Modified").font(sf.clone()).color(
                            if dark { tc.orange } else { Color32::from_rgb(255, 220, 150) }
                        ));
                    }
                    let err_count = ed.diagnostics.len();
                    if err_count > 0 {
                        ui.label(RichText::new(format!("⚠ {}", err_count)).font(sf.clone()).color(
                            if dark { tc.red } else { Color32::from_rgb(255, 180, 180) }
                        ));
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 16.0;
                        // Right side items (rendered right-to-left)
                        let theme_name = self.app.settings.theme.name();
                        let theme_btn = ui.add(egui::Button::new(
                            RichText::new(theme_name).font(sf.clone()).color(status_fg)
                        ).frame(false));
                        if theme_btn.clicked() {
                            let themes = Theme::ALL;
                            let idx = themes.iter().position(|t| *t == self.app.settings.theme).unwrap_or(0);
                            self.app.settings.theme = themes[(idx + 1) % themes.len()];
                            self.app.settings.save();
                        }
                        theme_btn.on_hover_text("Click to switch theme");

                        ui.label(RichText::new("UTF-8").font(sf.clone()).color(status_fg));
                        ui.label(RichText::new(lang).font(sf.clone()).color(status_fg));
                        ui.label(RichText::new(format!("Ln {}, Col {}", line, col)).font(sf.clone()).color(status_accent));
                        if self.app.auto_save_enabled {
                            ui.label(RichText::new("Auto-Save").font(sf.clone()).color(
                                if dark { tc.green } else { Color32::from_rgb(180, 255, 180) }
                            ));
                        }
                    });
                });
            });
    }
}
