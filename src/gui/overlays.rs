use super::*;

impl CodeEditorApp {
    pub(super) fn render_drag_overlay(&self, ctx: &egui::Context) {
        if let Some((_, ref src_path)) = self.drag_source {
            let file_name = std::path::Path::new(src_path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| src_path.clone());

            let is_dir = std::path::Path::new(src_path).is_dir();
            let icon = if is_dir { "📁 " } else { "📄 " };

            let label = if let Some((_, ref dst_path)) = self.drop_target {
                let dst_name = std::path::Path::new(dst_path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| dst_path.clone());
                format!("{}{} → 📂 {}", icon, file_name, dst_name)
            } else {
                format!("{}{}", icon, file_name)
            };

            if let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) {
                let painter = ctx.layer_painter(egui::LayerId::new(
                    egui::Order::Tooltip,
                    egui::Id::new("drag_overlay"),
                ));

                let font = FontId::monospace(12.0);
                let galley = painter.layout_no_wrap(label, font, self.tc.fg);
                let text_size = galley.size();
                let padding = Vec2::new(8.0, 4.0);
                let offset = Vec2::new(12.0, -8.0);

                let bg_rect = Rect::from_min_size(
                    pos + offset,
                    text_size + padding * 2.0,
                );

                painter.rect_filled(
                    bg_rect.translate(Vec2::new(2.0, 2.0)),
                    Rounding::same(4), Color32::from_black_alpha(80),
                );
                painter.rect_filled(bg_rect, Rounding::same(4), self.tc.sidebar_bg);
                painter.rect_stroke(bg_rect, Rounding::same(4), Stroke::new(1.0, self.tc.accent), egui::StrokeKind::Outside);
                painter.galley(bg_rect.min + padding, galley, Color32::PLACEHOLDER);
            }

            ctx.request_repaint();
        }
    }

    pub(super) fn render_overlays(&mut self, ctx: &egui::Context) {
        let dark = self.app.settings.theme != Theme::Light;
        let popup_frame = |accent: Color32| -> egui::Frame {
            let popup_bg = if dark { Color32::from_rgb(25, 26, 36) } else { Color32::from_rgb(255, 255, 255) };
            let shadow_alpha = if dark { 80 } else { 30 };
            egui::Frame::NONE
                .fill(popup_bg)
                .stroke(Stroke::new(1.0, accent))
                .rounding(Rounding::same(8))
                .inner_margin(12.0)
                .shadow(egui::epaint::Shadow {
                    offset: [0, 4],
                    blur: 16,
                    spread: 0,
                    color: Color32::from_black_alpha(shadow_alpha),
                })
        };

        match self.app.focus {
            Focus::CommandPalette => {
                egui::Area::new(egui::Id::new("pal")).fixed_pos(egui::pos2(ctx.screen_rect().width() * 0.25, 50.0)).show(ctx, |ui| {
                    popup_frame(self.tc.accent).show(ui, |ui| {
                        ui.set_width(ctx.screen_rect().width() * 0.5);
                        let r = ui.add(egui::TextEdit::singleline(&mut self.app.palette_input)
                            .font(mono()).hint_text("> Command...").desired_width(ui.available_width()).text_color(self.tc.fg));
                        r.request_focus();
                        let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                        let up = ui.input(|i| i.key_pressed(egui::Key::ArrowUp));
                        let down = ui.input(|i| i.key_pressed(egui::Key::ArrowDown));
                        if up && self.app.palette_selected > 0 { self.app.palette_selected -= 1; }
                        let items = self.app.filtered_palette_items();
                        if down && self.app.palette_selected + 1 < items.len() { self.app.palette_selected += 1; }
                        ui.add_space(4.0);
                        ui.painter().line_segment(
                            [Pos2::new(ui.max_rect().min.x, ui.cursor().min.y), Pos2::new(ui.max_rect().max.x, ui.cursor().min.y)],
                            Stroke::new(1.0, self.tc.border),
                        );
                        ui.add_space(4.0);
                        for (i, item) in items.iter().enumerate().take(12) {
                            let s = i == self.app.palette_selected;
                            let bg = if s { self.tc.selection_bg } else { Color32::TRANSPARENT };
                            let rr = ui.add(egui::Button::new(
                                RichText::new(format!("  {}", item.name)).font(small()).color(if s { self.tc.fg } else { self.tc.fg_dim })
                            ).fill(bg).min_size(Vec2::new(ui.available_width(), 26.0)).rounding(Rounding::same(4)).stroke(Stroke::NONE));
                            if rr.clicked() || (enter && s) {
                                let a = item.action.clone();
                                self.app.focus = Focus::Editor;
                                self.app.execute_palette_action(a);
                                return;
                            }
                        }
                        if enter && !items.is_empty() {
                            let a = items[self.app.palette_selected].action.clone();
                            self.app.focus = Focus::Editor;
                            self.app.execute_palette_action(a);
                        }
                    });
                });
            }
            Focus::QuickOpen => {
                egui::Area::new(egui::Id::new("qo")).fixed_pos(egui::pos2(ctx.screen_rect().width() * 0.25, 50.0)).show(ctx, |ui| {
                    popup_frame(self.tc.accent).show(ui, |ui| {
                        ui.set_width(ctx.screen_rect().width() * 0.5);
                        let old = self.app.quick_open_input.clone();
                        let r = ui.add(egui::TextEdit::singleline(&mut self.app.quick_open_input)
                            .font(mono()).hint_text("File name...").desired_width(ui.available_width()).text_color(self.tc.fg));
                        r.request_focus();
                        if self.app.quick_open_input != old {
                            self.app.quick_open_results = self.app.file_tree.fuzzy_search(&self.app.quick_open_input);
                            self.app.quick_open_selected = 0;
                        }
                        let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                        let up = ui.input(|i| i.key_pressed(egui::Key::ArrowUp));
                        let down = ui.input(|i| i.key_pressed(egui::Key::ArrowDown));
                        if up && self.app.quick_open_selected > 0 { self.app.quick_open_selected -= 1; }
                        if down && self.app.quick_open_selected + 1 < self.app.quick_open_results.len() { self.app.quick_open_selected += 1; }
                        ui.add_space(4.0);
                        let res = self.app.quick_open_results.clone();
                        for (i, e) in res.iter().enumerate().take(12) {
                            let s = i == self.app.quick_open_selected;
                            let rr = ui.add(egui::Button::new(
                                RichText::new(format!("  {}", e.name)).font(small()).color(if s { self.tc.fg } else { self.tc.fg_dim })
                            ).fill(if s { self.tc.selection_bg } else { Color32::TRANSPARENT }).min_size(Vec2::new(ui.available_width(), 26.0)).rounding(Rounding::same(4)).stroke(Stroke::NONE));
                            if rr.clicked() || (enter && s) { let p = e.path.clone(); self.app.open_file(&p); self.app.focus = Focus::Editor; return; }
                        }
                        if enter && !res.is_empty() { let p = res[self.app.quick_open_selected].path.clone(); self.app.open_file(&p); self.app.focus = Focus::Editor; }
                    });
                });
            }
            Focus::FindReplace => {
                let enter_find = ctx.input(|i| i.key_pressed(egui::Key::Enter));
                egui::Area::new(egui::Id::new("fr")).anchor(egui::Align2::RIGHT_TOP, [-20.0, 50.0]).show(ctx, |ui| {
                    popup_frame(self.tc.accent).show(ui, |ui| {
                        ui.set_width(350.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Find").font(small()).color(self.tc.fg_dim));
                            let r = ui.add(egui::TextEdit::singleline(&mut self.app.find_input)
                                .font(mono()).desired_width(280.0).text_color(self.tc.fg));
                            if r.changed() {
                                self.app.update_find_matches();
                            }
                            if enter_find && r.has_focus() {
                                self.app.goto_next_match();
                            }
                            r.request_focus();
                        });
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Replace").font(small()).color(self.tc.fg_dim));
                            ui.add(egui::TextEdit::singleline(&mut self.app.replace_input)
                                .font(mono()).desired_width(260.0).text_color(self.tc.fg));
                        });
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            if ui.add(egui::Button::new(RichText::new("◀").font(small())).min_size(Vec2::new(30.0, 22.0))).clicked() {
                                self.app.goto_prev_match();
                            }
                            if ui.add(egui::Button::new(RichText::new("▶").font(small())).min_size(Vec2::new(30.0, 22.0))).clicked() {
                                self.app.goto_next_match();
                            }
                            ui.add_space(4.0);
                            if ui.add(egui::Button::new(RichText::new("Replace").font(small())).min_size(Vec2::new(60.0, 22.0))).clicked() {
                                self.app.replace_current();
                            }
                            if ui.add(egui::Button::new(RichText::new("All").font(small())).min_size(Vec2::new(40.0, 22.0))).clicked() {
                                self.app.replace_all();
                            }
                            let total = self.app.find_matches.len();
                            let label = if total > 0 {
                                format!("{}/{}", self.app.find_current + 1, total)
                            } else {
                                "No matches".to_string()
                            };
                            ui.label(RichText::new(label).font(small()).color(if total > 0 { self.tc.fg_dim } else { self.tc.red }));
                        });
                    });
                });
            }
            Focus::GoToLine => {
                let enter_pressed = ctx.input(|i| i.key_pressed(egui::Key::Enter));
                egui::Area::new(egui::Id::new("gl")).fixed_pos(egui::pos2(ctx.screen_rect().width() * 0.35, 50.0)).show(ctx, |ui| {
                    popup_frame(self.tc.accent).show(ui, |ui| {
                        ui.set_width(220.0);
                        ui.label(RichText::new("Go to Line").font(small()).color(self.tc.fg_dim));
                        ui.add_space(4.0);
                        let r = ui.add(egui::TextEdit::singleline(&mut self.app.goto_input)
                            .font(mono()).desired_width(200.0).text_color(self.tc.fg));
                        r.request_focus();
                        if enter_pressed {
                            if let Ok(l) = self.app.goto_input.parse::<usize>() {
                                self.app.active_editor_mut().go_to_line(l);
                                self.app.focus = Focus::Editor;
                            }
                        }
                    });
                });
            }
            Focus::NewFileDialog | Focus::NewFolderDialog => {
                let is_file = self.app.focus == Focus::NewFileDialog;
                let title = if is_file { "New File" } else { "New Folder" };
                let hint = if is_file { "filename.ext" } else { "folder-name" };
                let enter_pressed = ctx.input(|i| i.key_pressed(egui::Key::Enter));
                egui::Area::new(egui::Id::new("dialog")).fixed_pos(egui::pos2(ctx.screen_rect().width() * 0.3, 80.0)).show(ctx, |ui| {
                    popup_frame(self.tc.accent).show(ui, |ui| {
                        ui.set_width(320.0);
                        ui.label(RichText::new(title).font(mono()).color(self.tc.accent));
                        ui.add_space(6.0);
                        let r = ui.add(egui::TextEdit::singleline(&mut self.app.dialog_input)
                            .font(mono()).hint_text(hint).desired_width(300.0).text_color(self.tc.fg));
                        r.request_focus();
                        if enter_pressed && !self.app.dialog_input.is_empty() {
                            let path = format!("{}/{}", self.app.dialog_context_path, self.app.dialog_input);
                            if is_file {
                                match self.app.file_tree.create_file(&path) {
                                    Ok(()) => { self.app.open_file(&path); self.app.status_message = format!("Created: {}", self.app.dialog_input); }
                                    Err(e) => { self.app.status_message = format!("Error: {}", e); }
                                }
                            } else {
                                match self.app.file_tree.create_directory(&path) {
                                    Ok(()) => { self.app.status_message = format!("Created: {}", self.app.dialog_input); }
                                    Err(e) => { self.app.status_message = format!("Error: {}", e); }
                                }
                            }
                            self.app.focus = Focus::Editor;
                        }
                    });
                });
            }
            Focus::RenameDialog => {
                let enter_pressed = ctx.input(|i| i.key_pressed(egui::Key::Enter));
                egui::Area::new(egui::Id::new("rn")).fixed_pos(egui::pos2(ctx.screen_rect().width() * 0.3, 80.0)).show(ctx, |ui| {
                    popup_frame(self.tc.accent).show(ui, |ui| {
                        ui.set_width(320.0);
                        ui.label(RichText::new("Rename").font(mono()).color(self.tc.accent));
                        ui.add_space(6.0);
                        let r = ui.add(egui::TextEdit::singleline(&mut self.app.dialog_input)
                            .font(mono()).desired_width(300.0).text_color(self.tc.fg));
                        r.request_focus();
                        if enter_pressed && !self.app.dialog_input.is_empty() {
                            let old = self.app.dialog_context_path.clone();
                            let parent = std::path::Path::new(&old).parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
                            let new_path = format!("{}/{}", parent, self.app.dialog_input);
                            match self.app.file_tree.rename_entry(&old, &new_path) {
                                Ok(()) => { self.app.status_message = format!("Renamed to: {}", self.app.dialog_input); }
                                Err(e) => { self.app.status_message = format!("Error: {}", e); }
                            }
                            self.app.focus = Focus::Editor;
                        }
                    });
                });
            }
            Focus::CommitInput => {
                let enter_pressed = ctx.input(|i| i.key_pressed(egui::Key::Enter) && i.modifiers.command);
                egui::Area::new(egui::Id::new("commit")).fixed_pos(egui::pos2(ctx.screen_rect().width() * 0.25, 80.0)).show(ctx, |ui| {
                    popup_frame(self.tc.green).show(ui, |ui| {
                        ui.set_width(ctx.screen_rect().width() * 0.5);
                        ui.label(RichText::new("Commit").font(mono()).color(self.tc.green));
                        ui.add_space(4.0);
                        // Show staged files count
                        if let Some(ref status) = self.app.git_status {
                            let staged_count = status.files.iter().filter(|f| f.staged).count();
                            ui.label(RichText::new(format!("{} staged file(s)", staged_count)).font(small()).color(self.tc.fg_dim));
                        }
                        ui.add_space(6.0);
                        let r = ui.add(egui::TextEdit::multiline(&mut self.app.commit_message)
                            .font(mono())
                            .hint_text("Commit message...")
                            .desired_width(ui.available_width())
                            .desired_rows(3)
                            .text_color(self.tc.fg));
                        r.request_focus();
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            let can_commit = !self.app.commit_message.trim().is_empty();
                            if ui.add_enabled(can_commit,
                                egui::Button::new(RichText::new(" Commit (⌘+Enter) ").color(Color32::WHITE))
                                    .fill(if can_commit { Color32::from_rgb(40, 160, 80) } else { Color32::from_rgb(80, 80, 80) })
                                    .rounding(Rounding::same(4))
                            ).clicked() || (enter_pressed && can_commit) {
                                let msg = self.app.commit_message.clone();
                                if let Some(ref root) = self.app.file_tree.root_path.clone() {
                                    match self.app.git.commit(root, &msg) {
                                        Ok(_) => {
                                            self.app.status_message = format!("Committed: {}", msg.lines().next().unwrap_or(""));
                                            self.app.refresh_git_status();
                                        }
                                        Err(e) => self.app.status_message = format!("Commit error: {}", e),
                                    }
                                }
                                self.app.commit_message.clear();
                                self.app.focus = Focus::Editor;
                            }
                            ui.add_space(8.0);
                            if ui.add(egui::Button::new(RichText::new(" Cancel ").color(self.tc.fg))
                                .fill(self.tc.sidebar_bg).rounding(Rounding::same(4))
                                .stroke(Stroke::new(1.0, self.tc.border))).clicked()
                            {
                                self.app.focus = Focus::Editor;
                            }
                        });
                    });
                });
            }
            Focus::DeleteConfirm => {
                if let Some(entry) = self.app.file_tree.selected_entry().cloned() {
                    egui::Area::new(egui::Id::new("del")).fixed_pos(egui::pos2(ctx.screen_rect().width() * 0.3, 80.0)).show(ctx, |ui| {
                        popup_frame(self.tc.red).show(ui, |ui| {
                            ui.set_width(320.0);
                            ui.label(RichText::new(format!("Delete \"{}\"?", entry.name)).font(mono()).color(self.tc.red));
                            ui.add_space(4.0);
                            ui.label(RichText::new("This cannot be undone.").font(small()).color(self.tc.fg_dim));
                            ui.add_space(10.0);
                            ui.horizontal(|ui| {
                                if ui.add(egui::Button::new(RichText::new(" Delete ").color(Color32::WHITE))
                                    .fill(Color32::from_rgb(200, 60, 60)).rounding(Rounding::same(4))).clicked()
                                {
                                    let _ = self.app.file_tree.delete_entry(&entry.path);
                                    self.app.status_message = format!("Deleted: {}", entry.name);
                                    self.app.focus = Focus::Editor;
                                }
                                ui.add_space(8.0);
                                if ui.add(egui::Button::new(RichText::new(" Cancel ").color(self.tc.fg))
                                    .fill(self.tc.sidebar_bg).rounding(Rounding::same(4)).stroke(Stroke::new(1.0, self.tc.border))).clicked()
                                {
                                    self.app.focus = Focus::Editor;
                                }
                            });
                        });
                    });
                } else { self.app.focus = Focus::Editor; }
            }
            Focus::About => {
                egui::Area::new(egui::Id::new("about")).fixed_pos(egui::pos2(ctx.screen_rect().width() * 0.2, 50.0)).show(ctx, |ui| {
                    popup_frame(self.tc.accent).show(ui, |ui| {
                        ui.set_width(ctx.screen_rect().width() * 0.6);

                        // Title
                        ui.vertical_centered(|ui| {
                            ui.label(RichText::new("Code Editor").font(FontId::monospace(24.0)).color(self.tc.accent));
                            ui.add_space(4.0);
                            ui.label(RichText::new(format!("Version {}", env!("CARGO_PKG_VERSION"))).font(small()).color(self.tc.fg_dim));
                            ui.add_space(2.0);
                            ui.label(RichText::new("Lightweight native code editor built with Rust + egui").font(small()).color(self.tc.fg_dim));
                        });

                        ui.add_space(12.0);
                        ui.painter().line_segment(
                            [Pos2::new(ui.max_rect().min.x, ui.cursor().min.y), Pos2::new(ui.max_rect().max.x, ui.cursor().min.y)],
                            Stroke::new(1.0, self.tc.border),
                        );
                        ui.add_space(8.0);

                        // Keyboard shortcuts
                        ui.label(RichText::new("Keyboard Shortcuts").font(FontId::monospace(14.0)).color(self.tc.fg));
                        ui.add_space(6.0);

                        let shortcuts = [
                            ("General", vec![
                                ("⌘+Shift+P", "Command Palette"),
                                ("⌘+P", "Quick Open File"),
                                ("⌘+O", "Open Folder"),
                                ("⌘+N", "New File"),
                                ("⌘+S", "Save"),
                                ("⌘+W", "Close Tab"),
                                ("⌘+Q", "Quit"),
                            ]),
                            ("Navigation", vec![
                                ("⌘+G", "Go to Line"),
                                ("⌘+F", "Find in File"),
                                ("⌘+Shift+F", "Find in Project"),
                                ("⌘+B", "Toggle Sidebar"),
                            ]),
                            ("Editing", vec![
                                ("⌘+Z", "Undo"),
                                ("⌘+D", "Select Next Occurrence"),
                                ("⌘+Shift+D", "Duplicate Line"),
                                ("⌘+K", "Delete Line"),
                                ("⌘+/", "Toggle Comment"),
                                ("Alt+Up/Down", "Move Line Up/Down"),
                                ("Tab / Shift+Tab", "Indent / Outdent"),
                            ]),
                            ("View", vec![
                                ("⌘+Plus", "Zoom In"),
                                ("⌘+Minus", "Zoom Out"),
                                ("⌘+0", "Reset Zoom"),
                            ]),
                        ];

                        let col_w = (ui.available_width() - 16.0) / 2.0;
                        ui.horizontal(|ui| {
                            // Left column
                            ui.vertical(|ui| {
                                ui.set_width(col_w);
                                for (section, items) in &shortcuts[..2] {
                                    ui.label(RichText::new(*section).font(FontId::monospace(11.0)).color(self.tc.accent).strong());
                                    ui.add_space(2.0);
                                    for (key, desc) in items {
                                        ui.horizontal(|ui| {
                                            ui.label(RichText::new(format!("{:<18}", key)).font(FontId::monospace(10.0)).color(self.tc.fg_dim));
                                            ui.label(RichText::new(*desc).font(FontId::monospace(10.0)).color(self.tc.fg));
                                        });
                                    }
                                    ui.add_space(6.0);
                                }
                            });
                            // Right column
                            ui.vertical(|ui| {
                                ui.set_width(col_w);
                                for (section, items) in &shortcuts[2..] {
                                    ui.label(RichText::new(*section).font(FontId::monospace(11.0)).color(self.tc.accent).strong());
                                    ui.add_space(2.0);
                                    for (key, desc) in items {
                                        ui.horizontal(|ui| {
                                            ui.label(RichText::new(format!("{:<18}", key)).font(FontId::monospace(10.0)).color(self.tc.fg_dim));
                                            ui.label(RichText::new(*desc).font(FontId::monospace(10.0)).color(self.tc.fg));
                                        });
                                    }
                                    ui.add_space(6.0);
                                }
                            });
                        });

                        ui.add_space(8.0);
                        ui.painter().line_segment(
                            [Pos2::new(ui.max_rect().min.x, ui.cursor().min.y), Pos2::new(ui.max_rect().max.x, ui.cursor().min.y)],
                            Stroke::new(1.0, self.tc.border),
                        );
                        ui.add_space(8.0);

                        // Features
                        ui.label(RichText::new("Features").font(FontId::monospace(14.0)).color(self.tc.fg));
                        ui.add_space(4.0);
                        for feature in [
                            "Syntax highlighting for 20+ languages",
                            "Git integration (stage, unstage, commit, discard)",
                            "Project-wide search with regex support",
                            "9 color themes (Darcula, Dracula, Nord, Catppuccin...)",
                            "Code folding, bracket matching, rainbow brackets",
                            "Auto-save, auto-indent, smart home",
                            "Minimap, breadcrumbs, file tree with drag & drop",
                            "Autocomplete from file words",
                        ] {
                            ui.horizontal(|ui| {
                                ui.add_space(8.0);
                                ui.label(RichText::new(format!("  •  {}", feature)).font(FontId::monospace(10.0)).color(self.tc.fg_dim));
                            });
                        }

                        ui.add_space(12.0);
                        ui.vertical_centered(|ui| {
                            ui.label(RichText::new("Built with Rust + egui  •  MIT License").font(FontId::monospace(10.0)).color(self.tc.fg_dim));
                            ui.add_space(4.0);
                            if ui.add(egui::Button::new(RichText::new(" Close (Esc) ").color(self.tc.fg))
                                .fill(self.tc.sidebar_bg).rounding(Rounding::same(4))
                                .stroke(Stroke::new(1.0, self.tc.border))).clicked()
                            {
                                self.app.focus = Focus::Editor;
                            }
                        });
                    });
                });
            }
            _ => {}
        }
    }
}
