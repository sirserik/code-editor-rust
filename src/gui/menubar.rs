use super::*;

impl CodeEditorApp {
    /// Thin strip above the menu bar that reserves space for the macOS traffic-light
    /// buttons (which float over our content thanks to `fullsize_content_view`) and
    /// shows the project/branch title centered, like the Ferrite mockup.
    pub(super) fn render_title_strip(&mut self, ctx: &egui::Context) {
        let tc = self.tc;
        // Recompute title only on project/branch change — formatting it every frame
        // burned a string allocation on the render thread for nothing.
        let project_root = self.app.file_tree.root_path.clone();
        let branch = self
            .app
            .git_status
            .as_ref()
            .filter(|g| g.is_repo)
            .map(|g| g.branch.clone());
        let key = (project_root.clone(), branch.clone());
        if key != self.cached_title_key {
            let project_name = project_root
                .as_ref()
                .and_then(|p| std::path::Path::new(p).file_name())
                .map(|n| n.to_string_lossy().to_string());
            let path_display = project_root.as_ref().map(|p| {
                let home = dirs::home_dir()
                    .map(|h| h.to_string_lossy().to_string())
                    .unwrap_or_default();
                if !home.is_empty() && p.starts_with(&home) {
                    format!("~{}", &p[home.len()..])
                } else {
                    p.clone()
                }
            });
            self.cached_title = match (project_name, branch, path_display) {
                (Some(p), Some(b), Some(path)) => format!("{} — {} · {}", p, b, path),
                (Some(p), None, Some(path)) => format!("{} · {}", p, path),
                (Some(p), _, None) => p,
                _ => "Ferrite".to_string(),
            };
            self.cached_title_key = key;
        }
        egui::TopBottomPanel::top("title_strip")
            .exact_height(32.0)
            .frame(egui::Frame::NONE.fill(tc.tab_bar_bg).inner_margin(0.0))
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    &self.cached_title,
                    FontId::monospace(12.5),
                    tc.fg,
                );

                // Right-side icon row: search · run · theme. Mirrors the Ferrite mockup.
                let btn_size = 22.0;
                let pad = 14.0;
                let mut x = rect.max.x - pad;
                let actions: &[(fn(&egui::Painter, Rect, Color32), &str, u8)] = &[
                    (super::icons::sun,    "Toggle theme",      0),
                    (super::icons::bolt,   "Run last command",  1),
                    (super::icons::search, "Find in Project",   2),
                ];
                for (draw, tip, action) in actions {
                    let icon_rect = Rect::from_min_size(
                        Pos2::new(x - btn_size, rect.center().y - btn_size * 0.5),
                        Vec2::splat(btn_size),
                    );
                    let resp = ui.interact(
                        icon_rect,
                        egui::Id::new(("title_strip_btn", *action)),
                        egui::Sense::click(),
                    );
                    let resp = resp.on_hover_text(*tip);
                    if resp.hovered() {
                        ui.painter().rect_filled(
                            icon_rect,
                            CornerRadius::same(4),
                            tc.hover_bg,
                        );
                    }
                    let inner = Rect::from_center_size(icon_rect.center(), Vec2::splat(14.0));
                    let color = if resp.hovered() { tc.fg } else { tc.fg_dim };
                    draw(ui.painter(), inner, color);
                    if resp.clicked() {
                        match action {
                            0 => {
                                let themes = Theme::ALL;
                                let idx = themes
                                    .iter()
                                    .position(|t| *t == self.app.settings.theme)
                                    .unwrap_or(0);
                                self.app.settings.theme = themes[(idx + 1) % themes.len()];
                                self.app.settings.save();
                            }
                            1 => self.app.run_last(),
                            2 => {
                                self.app.focus = Focus::GlobalSearch;
                                self.app.sidebar_tab = SidebarTab::Search;
                                self.app.show_sidebar = true;
                            }
                            _ => {}
                        }
                    }
                    x -= btn_size + 6.0;
                }
            });
    }

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
                        if ui.button("Close Project    ⌘⇧W").clicked() {
                            self.app.close_project();
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
                    ui.menu_button(RichText::new("Selection").font(small()).color(self.tc.fg), |ui| {
                        if ui.button("Select All           ⌘A").clicked() {
                            self.app.active_editor_mut().select_all();
                            ui.close_menu();
                        }
                        if ui.button("Select Line          ⌘L").clicked() {
                            self.app.active_editor_mut().select_line();
                            ui.close_menu();
                        }
                        if ui.button("Select Word at Cursor").clicked() {
                            self.app.active_editor_mut().select_word_at_cursor();
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Expand Selection     ⌘D").clicked() {
                            // ⌘D — already wired in keys.rs as "select next occurrence".
                            // Trigger by selecting word at cursor.
                            self.app.active_editor_mut().select_word_at_cursor();
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
                        let zoom_pct = (self.app.settings.font_size / DEFAULT_FONT_SIZE * 100.0) as u32;
                        if ui.button(format!("Zoom In            ⌘+  ({}%)", zoom_pct)).clicked() {
                            self.app.settings.font_size = (self.app.settings.font_size + 2.0).min(48.0);
                            self.app.settings.save();
                            ui.close_menu();
                        }
                        if ui.button("Zoom Out           ⌘-").clicked() {
                            self.app.settings.font_size = (self.app.settings.font_size - 2.0).max(8.0);
                            self.app.settings.save();
                            ui.close_menu();
                        }
                        if ui.button("Reset Zoom         ⌘0").clicked() {
                            self.app.settings.font_size = DEFAULT_FONT_SIZE;
                            self.app.settings.save();
                            ui.close_menu();
                        }
                    });
                    ui.menu_button(RichText::new("Go").font(small()).color(self.tc.fg), |ui| {
                        if ui.button("Go to File…          ⌘P").clicked() {
                            self.app.focus = Focus::QuickOpen;
                            self.app.quick_open_input.clear();
                            self.app.quick_open_results.clear();
                            ui.close_menu();
                        }
                        if ui.button("Go to Line…          ⌘G").clicked() {
                            self.app.focus = Focus::GoToLine;
                            self.app.goto_input.clear();
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Top of File          ⌘↑").clicked() {
                            self.app.active_editor_mut().move_to_top();
                            ui.close_menu();
                        }
                        if ui.button("Bottom of File       ⌘↓").clicked() {
                            self.app.active_editor_mut().move_to_bottom();
                            ui.close_menu();
                        }
                        ui.separator();
                        let n = self.app.editors.len();
                        if ui.add_enabled(n > 1, egui::Button::new("Next Tab           ⌃⇥")).clicked() {
                            self.app.active_editor = (self.app.active_editor + 1) % n;
                            ui.close_menu();
                        }
                        if ui.add_enabled(n > 1, egui::Button::new("Previous Tab     ⌃⇧⇥")).clicked() {
                            self.app.active_editor = if self.app.active_editor == 0 { n - 1 } else { self.app.active_editor - 1 };
                            ui.close_menu();
                        }
                    });
                    ui.menu_button(RichText::new("Run").font(small()).color(self.tc.fg), |ui| {
                        let has_project = self.app.file_tree.root_path.is_some();
                        let running = self.app.output.is_running();
                        if ui.add_enabled(has_project, egui::Button::new("Run Build           ⌘R")).clicked() {
                            self.app.run_build();
                            ui.close_menu();
                        }
                        if ui.add_enabled(has_project, egui::Button::new("Run Tests          ⇧⌘R")).clicked() {
                            self.app.run_tests();
                            ui.close_menu();
                        }
                        let can_rerun = self.app.output.last_command.is_some();
                        if ui.add_enabled(can_rerun, egui::Button::new("Re-run Last Command")).clicked() {
                            self.app.run_last();
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Run Custom Command…").clicked() {
                            self.app.run_command_input.clear();
                            self.app.focus = Focus::RunCommandDialog;
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.add_enabled(running, egui::Button::new("Stop Running        ⌘.")).clicked() {
                            self.app.output.stop();
                            ui.close_menu();
                        }
                        if ui.button("Clear Output").clicked() {
                            self.app.output.clear();
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Show Output Panel  ⇧⌘U").clicked() {
                            self.app.execute_palette_action(PaletteAction::ToggleOutput);
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
        let dark = !self.app.settings.theme.is_light();
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
                        let bg = if active { self.tc.tab_active_bg } else { Color32::TRANSPARENT };
                        let rounding = CornerRadius { nw: 8, ne: 8, sw: 0, se: 0 };

                        let frame = egui::Frame::NONE.fill(bg).corner_radius(rounding)
                            .inner_margin(egui::Margin { left: 10, right: 4, top: 4, bottom: 4 });

                        // File-type dot color (matches the file-tree icon scheme).
                        let icon_color = file_icon_color(&name, dark);

                        let frame_resp = frame.show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 6.0;
                                let mut job = egui::text::LayoutJob::default();
                                let dot_fmt = egui::text::TextFormat {
                                    font_id: small(),
                                    color: icon_color,
                                    ..Default::default()
                                };
                                job.append("●  ", 0.0, dot_fmt);
                                let name_fmt = egui::text::TextFormat {
                                    font_id: small(),
                                    color: text_c,
                                    ..Default::default()
                                };
                                job.append(&name, 0.0, name_fmt);
                                if dirty {
                                    let dirty_fmt = egui::text::TextFormat {
                                        font_id: small(),
                                        color: self.tc.orange,
                                        ..Default::default()
                                    };
                                    job.append(" ●", 0.0, dirty_fmt);
                                }
                                let label_resp = ui.add(
                                    egui::Label::new(job).sense(egui::Sense::click())
                                );
                                if label_resp.clicked() {
                                    self.app.active_editor = i;
                                    self.app.focus = Focus::Editor;
                                }
                                // Tab context menu
                                label_resp.context_menu(|ui| {
                                    if ui.button("Close").clicked() { tab_to_close = Some(i); ui.close_menu(); }
                                    if self.app.editors.len() > 1 {
                                        if ui.button("Close Others").clicked() {
                                            // Keep only tab i
                                            let kept = self.app.editors.remove(i);
                                            self.app.editors.clear();
                                            self.app.editors.push(kept);
                                            self.app.active_editor = 0;
                                            ui.close_menu();
                                        }
                                        if i + 1 < self.app.editors.len() {
                                            if ui.button("Close to the Right").clicked() {
                                                self.app.editors.truncate(i + 1);
                                                if self.app.active_editor > i { self.app.active_editor = i; }
                                                ui.close_menu();
                                            }
                                        }
                                        ui.separator();
                                        if ui.button("Close All").clicked() {
                                            self.app.editors.clear();
                                            self.app.editors.push(crate::editor::Editor::new());
                                            self.app.active_editor = 0;
                                            ui.close_menu();
                                        }
                                    }
                                });
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
                                    ui.painter().rect_filled(cr, CornerRadius::same(3), self.tc.red);
                                    ui.painter().text(cr.center(), egui::Align2::CENTER_CENTER, "×",
                                        FontId::monospace(14.0), Color32::WHITE);
                                }
                                if close_resp.clicked() {
                                    tab_to_close = Some(i);
                                }
                            });
                        });

                        // Active tab marker — IntelliJ New UI: a 2px accent rule
                        // spanning the full tab, flush with the bottom of the strip.
                        if active {
                            let tab_rect = frame_resp.response.rect;
                            ui.painter().rect_filled(
                                Rect::from_min_size(
                                    Pos2::new(tab_rect.min.x, panel_rect.max.y - 2.0),
                                    Vec2::new(tab_rect.width(), 2.0),
                                ),
                                CornerRadius::ZERO, self.tc.tab_underline,
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
        let status_bg = tc.status_bg;
        let status_fg = tc.status_fg;
        let status_accent = tc.fg;
        let sf = FontId::monospace(11.5);

        egui::TopBottomPanel::bottom("status")
            .exact_height(26.0)
            .frame(egui::Frame::NONE.fill(status_bg).inner_margin(egui::Margin::symmetric(10, 3)))
            .show(ctx, |ui| {
                let sr = ui.max_rect();
                ui.painter().line_segment(
                    [Pos2::new(sr.min.x - 10.0, sr.min.y - 3.0), Pos2::new(sr.max.x + 10.0, sr.min.y - 3.0)],
                    Stroke::new(1.0, tc.border),
                );
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
                        ui.label(RichText::new("● Modified").font(sf.clone()).color(tc.orange));
                    }
                    let err_count = ed.diagnostics.len();
                    if err_count > 0 {
                        let err_color = tc.red;
                        ui.label(RichText::new(format!("⚠ {}", err_count)).font(sf.clone()).color(err_color));
                        // Show first diagnostic on the current line, if any
                        if let Some(diag) = ed.diagnostics.iter().find(|d| d.line == ed.cursor.line) {
                            ui.label(RichText::new(&diag.message).font(sf.clone()).color(err_color));
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 16.0;
                        // Perf HUD: total frame + per-section breakdown
                        if !self.frame_times_ms.is_empty() {
                            let n = self.frame_times_ms.len() as f32;
                            let avg = self.frame_times_ms.iter().sum::<f32>() / n;
                            let max = self.frame_times_ms.iter().cloned().fold(0.0_f32, f32::max);
                            let perf_color = if max > 16.7 { tc.orange } else { status_fg };
                            let [mb, tabs, st, sb, ed_s] = self.section_times_ms;
                            ui.label(RichText::new(format!(
                                "ed {:.1} sb {:.1} tabs {:.1} mb {:.1} st {:.1}  |  {:.1}/{:.1}ms",
                                ed_s, sb, tabs, mb, st, avg, max
                            )).font(sf.clone()).color(perf_color));
                        }
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
                            ui.label(RichText::new("Auto-Save").font(sf.clone()).color(tc.green));
                        }
                    });
                });
            });
    }
}
