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
        egui::TopBottomPanel::top("tabs")
            .exact_height(34.0)
            .frame(egui::Frame::NONE.fill(self.tc.tab_bar_bg).inner_margin(egui::Margin { left: 4, right: 4, top: 4, bottom: 0 }))
            .show(ctx, |ui| {
                let r = ui.max_rect();
                ui.painter().line_segment(
                    [Pos2::new(r.min.x, r.max.y), Pos2::new(r.max.x, r.max.y)],
                    Stroke::new(1.0, self.tc.border),
                );
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing.x = 1.0;
                    for i in 0..self.app.editors.len() {
                        let name = self.app.editors[i].file_name();
                        let dirty = self.app.editors[i].is_dirty;
                        let active = i == self.app.active_editor;
                        let tc = if active { self.tc.fg } else { self.tc.fg_dim };
                        let bg = if active { self.tc.bg } else { self.tc.tab_bar_bg };
                        let rounding = Rounding { nw: 6, ne: 6, sw: 0, se: 0 };

                        let frame = egui::Frame::NONE.fill(bg).rounding(rounding)
                            .stroke(if active { Stroke::new(1.0, self.tc.border) } else { Stroke::NONE })
                            .inner_margin(egui::Margin { left: 8, right: 2, top: 2, bottom: 2 });

                        frame.show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 2.0;
                                let dot = if dirty { " ●" } else { "" };
                                let label_resp = ui.add(egui::Label::new(
                                    RichText::new(format!("{}{}", name, dot)).font(small()).color(tc)
                                ).sense(egui::Sense::click()));
                                if label_resp.clicked() {
                                    self.app.active_editor = i;
                                    self.app.focus = Focus::Editor;
                                }
                                let close_resp = ui.add(
                                    egui::Button::new(
                                        RichText::new(" × ").font(small()).color(self.tc.fg_dim)
                                    )
                                    .frame(false)
                                    .min_size(egui::vec2(20.0, 20.0))
                                );
                                if close_resp.clicked() {
                                    tab_to_close = Some(i);
                                }
                            });
                        });
                    }
                });
            });
        if let Some(i) = tab_to_close {
            self.app.close_tab(i);
        }
    }

    pub(super) fn render_status(&mut self, ctx: &egui::Context) {
        let tc = self.tc;
        egui::TopBottomPanel::bottom("status")
            .exact_height(24.0)
            .frame(egui::Frame::NONE.fill(tc.accent.linear_multiply(0.15)).inner_margin(egui::Margin::symmetric(8, 2)))
            .show(ctx, |ui| {
                let ed = &self.app.editors[self.app.active_editor];
                let lang = ed.file_path.as_ref().map(|p| syntax::detect_language(p)).unwrap_or("Text");
                let line = ed.cursor.line + 1;
                let col = ed.cursor.col + 1;
                let dirty = ed.is_dirty;
                let theme_name = self.app.settings.theme.name();
                ui.horizontal_centered(|ui| {
                    ui.label(RichText::new(self.app.status_message.as_str()).font(small()).color(tc.fg));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 12.0;
                        let theme_btn = ui.add(egui::Button::new(
                            RichText::new(format!("🎨 {}", theme_name)).font(small()).color(tc.fg)
                        ).frame(false));
                        if theme_btn.clicked() {
                            let themes = Theme::ALL;
                            let idx = themes.iter().position(|t| *t == self.app.settings.theme).unwrap_or(0);
                            self.app.settings.theme = themes[(idx + 1) % themes.len()];
                            self.app.settings.save();
                        }
                        theme_btn.on_hover_text("Click to switch theme");
                        if dirty {
                            ui.label(RichText::new("● Modified").font(small()).color(tc.orange));
                        }
                        let err_count = ed.diagnostics.len();
                        if err_count > 0 {
                            ui.label(RichText::new(format!("⚠ {}", err_count)).font(small()).color(Color32::from_rgb(247, 118, 142)));
                        }
                        if self.app.auto_save_enabled {
                            ui.label(RichText::new("Auto-Save").font(FontId::monospace(10.0)).color(tc.green));
                        }
                        let font_size = self.app.settings.font_size;
                        ui.label(RichText::new(format!("{}px", font_size as u32)).font(small()).color(tc.fg_dim));
                        ui.label(RichText::new(lang).font(small()).color(tc.fg_dim));
                        ui.label(RichText::new(format!("Ln {}, Col {}", line, col)).font(small()).color(tc.fg));
                        ui.label(RichText::new("UTF-8").font(small()).color(tc.fg_dim));
                    });
                });
            });
    }
}
