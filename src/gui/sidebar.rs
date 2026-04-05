use super::*;

impl CodeEditorApp {
    pub(super) fn render_sidebar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("sidebar")
            .default_width(240.0).min_width(150.0).max_width(400.0)
            .frame(egui::Frame::NONE.fill(self.tc.sidebar_bg).inner_margin(0.0))
            .show(ctx, |ui| {
                let r = ui.max_rect();
                ui.painter().line_segment(
                    [Pos2::new(r.max.x, r.min.y), Pos2::new(r.max.x, r.max.y)],
                    Stroke::new(1.0, self.tc.border),
                );

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    let dark = self.app.settings.theme != Theme::Light;
                    for (label, tab) in [("  Files  ", SidebarTab::Files), ("  Git  ", SidebarTab::Git), ("  Search  ", SidebarTab::Search)] {
                        let active = self.app.sidebar_tab == tab;
                        let c = if active { self.tc.accent } else { self.tc.fg_dim };
                        let btn = egui::Button::new(RichText::new(label).font(small()).color(c))
                            .fill(Color32::TRANSPARENT).rounding(Rounding::ZERO).stroke(Stroke::NONE);
                        let resp = ui.add(btn);
                        // Hover background
                        if resp.hovered() && !active {
                            ui.painter().rect_filled(resp.rect, Rounding::same(3),
                                if dark { Color32::from_white_alpha(8) } else { Color32::from_black_alpha(8) });
                        }
                        if resp.clicked() { self.app.sidebar_tab = tab; }
                        if active {
                            let rect = resp.rect;
                            ui.painter().rect_filled(
                                Rect::from_min_size(
                                    Pos2::new(rect.min.x + 4.0, rect.max.y - 2.0),
                                    Vec2::new(rect.width() - 8.0, 2.0),
                                ),
                                Rounding::same(1), self.tc.accent,
                            );
                        }
                    }
                });
                ui.add_space(2.0);
                ui.painter().line_segment(
                    [Pos2::new(ui.max_rect().min.x, ui.cursor().min.y), Pos2::new(ui.max_rect().max.x, ui.cursor().min.y)],
                    Stroke::new(1.0, self.tc.border),
                );
                ui.add_space(4.0);

                match self.app.sidebar_tab {
                    SidebarTab::Files => self.render_file_tree(ui),
                    SidebarTab::Git => self.render_git(ui),
                    SidebarTab::Search => self.render_search(ui),
                }
            });
    }

    fn render_file_tree(&mut self, ui: &mut egui::Ui) {
        let dark = self.app.settings.theme != Theme::Light;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.add_space(8.0);
            for (icon, tip, action) in [
                ("+", "New File", 0u8),
                ("□", "New Folder", 1),
                ("↻", "Refresh", 2),
            ] {
                if ui.add(egui::Button::new(RichText::new(icon).font(small()).color(self.tc.fg_dim))
                    .fill(Color32::TRANSPARENT).min_size(Vec2::new(22.0, 18.0))
                    .rounding(Rounding::same(3)))
                    .on_hover_text(tip).clicked()
                {
                    match action {
                        0 => self.app.start_new_file_dialog(),
                        1 => self.app.start_new_folder_dialog(),
                        _ => self.app.file_tree.refresh(),
                    }
                }
            }
        });
        ui.add_space(2.0);

        let row_h = 28.0;
        let indent_px = 18.0;

        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            let entries = self.app.file_tree.flat_entries.clone();
            let drag_source_idx = self.drag_source.as_ref().map(|(i, _)| *i);
            if self.drag_source.is_some() {
                self.drop_target = None;
            }
            let mut new_drop_target: Option<(usize, String)> = None;
            let pointer_pos = ui.ctx().input(|i| i.pointer.hover_pos());

            for (i, entry) in entries.iter().enumerate() {
                let depth = entry.depth;
                let indent = indent_px * depth as f32 + 8.0;
                let sel = i == self.app.file_tree.selected_index;
                let is_dragged = drag_source_idx == Some(i);

                let (row_rect, row_resp) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), row_h),
                    egui::Sense::click_and_drag(),
                );

                let is_drop = if self.drag_source.is_some() && entry.is_directory {
                    pointer_pos.is_some_and(|p| row_rect.contains(p))
                } else {
                    false
                };

                let drop_bg = if dark { Color32::from_rgb(35, 50, 80) } else { Color32::from_rgb(210, 225, 245) };
                let bg = if is_drop { drop_bg } else if sel { self.tc.selection_bg } else { Color32::TRANSPARENT };

                if bg != Color32::TRANSPARENT {
                    ui.painter().rect_filled(row_rect, Rounding::ZERO, bg);
                }

                if is_drop {
                    ui.painter().rect_stroke(row_rect, Rounding::same(3), Stroke::new(2.0, self.tc.accent), egui::StrokeKind::Outside);
                    ui.painter().rect_filled(
                        Rect::from_min_size(row_rect.min, Vec2::new(3.0, row_rect.height())),
                        Rounding::ZERO,
                        self.tc.accent,
                    );
                }

                // Indent guide lines
                let painter = ui.painter();
                let guide_c = if dark { Color32::from_rgb(57, 57, 57) } else { Color32::from_rgb(228, 228, 228) };
                for d in 1..depth + 1 {
                    let gx = row_rect.min.x + indent_px * d as f32 + 4.0;
                    painter.line_segment(
                        [Pos2::new(gx, row_rect.min.y), Pos2::new(gx, row_rect.max.y)],
                        Stroke::new(1.0, guide_c),
                    );
                }

                let dim = if is_dragged { 0.35 } else { 1.0 };
                let row_y = row_rect.min.y + (row_h - 13.0) / 2.0;
                let text_color = if entry.name.starts_with('.') {
                    self.tc.fg_dim
                } else {
                    self.tc.fg
                }.linear_multiply(dim);
                let arrow_color = if dark {
                    Color32::from_rgb(140, 140, 140)
                } else {
                    Color32::from_rgb(130, 130, 130)
                }.linear_multiply(dim);

                if entry.is_directory {
                    let arr = if entry.is_expanded { "▾" } else { "▸" };
                    painter.text(
                        Pos2::new(row_rect.min.x + indent, row_y),
                        egui::Align2::LEFT_TOP, arr, FontId::monospace(12.0), arrow_color,
                    );
                    painter.text(
                        Pos2::new(row_rect.min.x + indent + 14.0, row_y),
                        egui::Align2::LEFT_TOP, &entry.name, FontId::monospace(13.0), text_color,
                    );
                } else {
                    let dot_color = file_icon_color(&entry.name, dark).linear_multiply(dim);
                    let dot_y = row_rect.min.y + row_h / 2.0;
                    painter.circle_filled(
                        Pos2::new(row_rect.min.x + indent + 5.0, dot_y), 3.5, dot_color,
                    );
                    painter.text(
                        Pos2::new(row_rect.min.x + indent + 14.0, row_y),
                        egui::Align2::LEFT_TOP, &entry.name, FontId::monospace(13.0), text_color,
                    );
                }

                if row_resp.drag_started() {
                    self.drag_source = Some((i, entry.path.clone()));
                }

                if row_resp.clicked() && self.drag_source.is_none() {
                    self.app.file_tree.selected_index = i;
                    if entry.is_directory {
                        self.app.file_tree.toggle_expand(i);
                    } else {
                        let p = entry.path.clone();
                        self.app.open_file(&p);
                    }
                }

                if self.drag_source.is_some() {
                    if let Some(pp) = pointer_pos {
                        if row_rect.contains(pp) {
                            if entry.is_directory {
                                new_drop_target = Some((i, entry.path.clone()));
                            } else if let Some(parent) = std::path::Path::new(&entry.path).parent() {
                                new_drop_target = Some((i, parent.to_string_lossy().to_string()));
                            }
                        }
                    }
                }

                row_resp.context_menu(|ui| {
                    self.app.file_tree.selected_index = i;
                    if ui.button("New File Here").clicked() { self.app.start_new_file_dialog(); ui.close_menu(); }
                    if ui.button("New Folder Here").clicked() { self.app.start_new_folder_dialog(); ui.close_menu(); }
                    ui.separator();
                    if ui.button("Rename").clicked() { self.app.start_rename_dialog(); ui.close_menu(); }
                    if ui.button("Duplicate").clicked() {
                        let p = entry.path.clone();
                        let _ = self.app.file_tree.duplicate_entry(&p);
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Delete").clicked() { self.app.focus = Focus::DeleteConfirm; ui.close_menu(); }
                });
            }

            if let Some(dt) = new_drop_target {
                self.drop_target = Some(dt);
            }
        });

        // Handle drop
        if ui.input(|i| i.pointer.any_released()) {
            if let (Some((_, src_path)), Some((_, dst_dir))) = (self.drag_source.take(), self.drop_target.take()) {
                let src = std::path::Path::new(&src_path);
                let dst = std::path::Path::new(&dst_dir);
                let src_parent = src.parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
                if src_path != dst_dir && src_parent != dst_dir && !dst_dir.starts_with(&src_path) {
                    if let Some(name) = src.file_name() {
                        let new_path = dst.join(name);
                        if !new_path.exists() {
                            if let Err(e) = std::fs::rename(&src_path, &new_path) {
                                self.app.status_message = format!("Move error: {}", e);
                            } else {
                                self.app.status_message = format!("Moved to {}", dst_dir);
                                self.app.file_tree.refresh();
                                self.app.file_tree.ensure_expanded(&dst_dir);
                            }
                        } else {
                            self.app.status_message = format!("Already exists: {}", new_path.display());
                        }
                    }
                }
            }
            self.drag_source = None;
            self.drop_target = None;
        }
    }

    fn render_git(&mut self, ui: &mut egui::Ui) {
        let tc = self.tc;
        ui.add_space(4.0);

        // Clone status to avoid borrow issues
        let status = self.app.git_status.clone();
        if let Some(status) = status {
            if status.is_repo {
                // Branch + toolbar
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    ui.label(RichText::new(format!("⎇ {}", status.branch)).font(small()).color(tc.accent));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(8.0);
                        // Refresh button
                        if ui.add(egui::Button::new(RichText::new("↻").font(small()).color(tc.fg_dim))
                            .fill(Color32::TRANSPARENT).min_size(Vec2::new(22.0, 18.0)))
                            .on_hover_text("Refresh git status").clicked()
                        {
                            self.app.refresh_git_async();
                        }
                    });
                });
                ui.add_space(4.0);
                ui.painter().line_segment(
                    [Pos2::new(ui.max_rect().min.x, ui.cursor().min.y), Pos2::new(ui.max_rect().max.x, ui.cursor().min.y)],
                    Stroke::new(1.0, tc.border),
                );
                ui.add_space(2.0);

                // Action buttons
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    ui.spacing_mut().item_spacing.x = 4.0;
                    let has_staged = status.files.iter().any(|f| f.staged);
                    // Stage All
                    if ui.add(egui::Button::new(RichText::new("Stage All").font(FontId::monospace(10.0)).color(tc.fg_dim))
                        .fill(Color32::TRANSPARENT).min_size(Vec2::new(0.0, 18.0))).clicked()
                    {
                        self.app.git_stage_all();
                    }
                    // Commit
                    if has_staged {
                        if ui.add(egui::Button::new(RichText::new("Commit").font(FontId::monospace(10.0)).color(tc.green))
                            .fill(Color32::TRANSPARENT).min_size(Vec2::new(0.0, 18.0))).clicked()
                        {
                            self.app.focus = Focus::CommitInput;
                            self.app.commit_message.clear();
                        }
                    }
                });

                ui.add_space(2.0);
                ui.painter().line_segment(
                    [Pos2::new(ui.max_rect().min.x, ui.cursor().min.y), Pos2::new(ui.max_rect().max.x, ui.cursor().min.y)],
                    Stroke::new(1.0, tc.border),
                );
                ui.add_space(4.0);

                // Staged files section
                let staged: Vec<_> = status.files.iter().filter(|f| f.staged).collect();
                let unstaged: Vec<_> = status.files.iter().filter(|f| !f.staged).collect();

                // Deferred actions
                let mut action_stage: Option<String> = None;
                let mut action_unstage: Option<String> = None;
                let mut action_discard: Option<String> = None;
                let mut action_open: Option<String> = None;

                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;

                    if !staged.is_empty() {
                        ui.horizontal(|ui| {
                            ui.add_space(8.0);
                            ui.label(RichText::new(format!("Staged ({})", staged.len()))
                                .font(FontId::monospace(10.0)).color(tc.green).strong());
                        });
                        ui.add_space(2.0);

                        for f in &staged {
                            let (row_rect, row_resp) = ui.allocate_exact_size(
                                Vec2::new(ui.available_width(), 24.0),
                                egui::Sense::click(),
                            );
                            if row_resp.hovered() {
                                ui.painter().rect_filled(row_rect, Rounding::ZERO, tc.selection_bg);
                            }
                            ui.painter().text(
                                Pos2::new(row_rect.min.x + 12.0, row_rect.min.y + 5.0),
                                egui::Align2::LEFT_TOP,
                                format!("✓ {} {}", f.status.symbol(), f.path),
                                FontId::monospace(11.0), tc.green,
                            );
                            // Click to open file
                            if row_resp.clicked() {
                                action_open = Some(f.path.clone());
                            }
                            // Context menu
                            row_resp.context_menu(|ui| {
                                if ui.button("Unstage").clicked() {
                                    action_unstage = Some(f.path.clone());
                                    ui.close_menu();
                                }
                                if ui.button("Open File").clicked() {
                                    action_open = Some(f.path.clone());
                                    ui.close_menu();
                                }
                            });
                        }
                        ui.add_space(6.0);
                    }

                    if !unstaged.is_empty() {
                        ui.horizontal(|ui| {
                            ui.add_space(8.0);
                            ui.label(RichText::new(format!("Changes ({})", unstaged.len()))
                                .font(FontId::monospace(10.0)).color(tc.orange).strong());
                        });
                        ui.add_space(2.0);

                        for f in &unstaged {
                            let (row_rect, row_resp) = ui.allocate_exact_size(
                                Vec2::new(ui.available_width(), 24.0),
                                egui::Sense::click(),
                            );
                            if row_resp.hovered() {
                                ui.painter().rect_filled(row_rect, Rounding::ZERO, tc.selection_bg);
                            }
                            ui.painter().text(
                                Pos2::new(row_rect.min.x + 12.0, row_rect.min.y + 5.0),
                                egui::Align2::LEFT_TOP,
                                format!("● {} {}", f.status.symbol(), f.path),
                                FontId::monospace(11.0), tc.orange,
                            );
                            if row_resp.clicked() {
                                action_open = Some(f.path.clone());
                            }
                            row_resp.context_menu(|ui| {
                                if ui.button("Stage").clicked() {
                                    action_stage = Some(f.path.clone());
                                    ui.close_menu();
                                }
                                if ui.button("Discard Changes").clicked() {
                                    action_discard = Some(f.path.clone());
                                    ui.close_menu();
                                }
                                if ui.button("Open File").clicked() {
                                    action_open = Some(f.path.clone());
                                    ui.close_menu();
                                }
                            });
                        }
                    }

                    if staged.is_empty() && unstaged.is_empty() {
                        ui.add_space(20.0);
                        ui.vertical_centered(|ui| {
                            ui.label(RichText::new("No changes").font(small()).color(tc.fg_dim));
                        });
                    }
                });

                // Execute deferred actions
                if let Some(path) = action_stage {
                    if let Some(ref root) = self.app.file_tree.root_path.clone() {
                        match self.app.git.stage_file(root, &path) {
                            Ok(_) => { self.app.status_message = format!("Staged: {}", path); self.app.refresh_git_status(); }
                            Err(e) => self.app.status_message = format!("Error: {}", e),
                        }
                    }
                }
                if let Some(path) = action_unstage {
                    if let Some(ref root) = self.app.file_tree.root_path.clone() {
                        match self.app.git.unstage_file(root, &path) {
                            Ok(_) => { self.app.status_message = format!("Unstaged: {}", path); self.app.refresh_git_status(); }
                            Err(e) => self.app.status_message = format!("Error: {}", e),
                        }
                    }
                }
                if let Some(path) = action_discard {
                    if let Some(ref root) = self.app.file_tree.root_path.clone() {
                        match self.app.git.discard_file(root, &path) {
                            Ok(_) => { self.app.status_message = format!("Discarded: {}", path); self.app.refresh_git_status(); }
                            Err(e) => self.app.status_message = format!("Error: {}", e),
                        }
                    }
                }
                if let Some(path) = action_open {
                    if let Some(ref root) = self.app.file_tree.root_path {
                        let full_path = format!("{}/{}", root, path);
                        self.app.open_file(&full_path);
                        self.app.focus = Focus::Editor;
                    }
                }
            } else {
                ui.vertical_centered(|ui| {
                    ui.add_space(20.0);
                    ui.label(RichText::new("Not a git repository").font(small()).color(tc.fg_dim));
                });
            }
        } else {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                ui.label(RichText::new("Open a folder to see git status").font(small()).color(tc.fg_dim));
            });
        }
    }

    fn render_search(&mut self, ui: &mut egui::Ui) {
        let tc = self.tc;
        let dark = self.app.settings.theme != Theme::Light;
        ui.add_space(6.0);

        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.label(RichText::new("🔍").font(small()));
            let resp = ui.add(
                egui::TextEdit::singleline(&mut self.app.global_search_input)
                    .font(small())
                    .desired_width(ui.available_width() - 12.0)
                    .hint_text("Search in files...")
            );
            if resp.changed() && self.app.global_search_input.len() >= 2 {
                self.app.last_search_trigger = Some(std::time::Instant::now());
            }
            if resp.changed() && self.app.global_search_input.is_empty() {
                self.app.global_search_results.clear();
                self.app.file_search_results.clear();
            }
        });

        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.add_space(8.0);
            let case_btn = if self.app.find_case_sensitive { "Aa ●" } else { "Aa" };
            if ui.add(egui::Button::new(RichText::new(case_btn).font(FontId::monospace(10.0)).color(tc.fg_dim))
                .fill(if self.app.find_case_sensitive { tc.selection_bg } else { Color32::TRANSPARENT })
                .min_size(Vec2::new(28.0, 16.0))).on_hover_text("Case sensitive").clicked()
            {
                self.app.find_case_sensitive = !self.app.find_case_sensitive;
                if self.app.global_search_input.len() >= 2 { self.do_search(); }
            }
            let regex_btn = if self.app.find_use_regex { ".* ●" } else { ".*" };
            if ui.add(egui::Button::new(RichText::new(regex_btn).font(FontId::monospace(10.0)).color(tc.fg_dim))
                .fill(if self.app.find_use_regex { tc.selection_bg } else { Color32::TRANSPARENT })
                .min_size(Vec2::new(28.0, 16.0))).on_hover_text("Use regex").clicked()
            {
                self.app.find_use_regex = !self.app.find_use_regex;
                if self.app.global_search_input.len() >= 2 { self.do_search(); }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(8.0);
                let fc = self.app.file_search_results.len();
                let cc = self.app.global_search_results.len();
                let label = format!("{} files, {} lines", fc, if cc >= 1000 { "1000+".to_string() } else { cc.to_string() });
                ui.label(RichText::new(label).font(FontId::monospace(10.0)).color(tc.fg_dim));
            });
        });

        ui.add_space(2.0);
        ui.painter().line_segment(
            [Pos2::new(ui.max_rect().min.x, ui.cursor().min.y), Pos2::new(ui.max_rect().max.x, ui.cursor().min.y)],
            Stroke::new(1.0, tc.border),
        );
        ui.add_space(2.0);

        let has_file_results = !self.app.file_search_results.is_empty();
        let has_content_results = !self.app.global_search_results.is_empty();
        if !has_file_results && !has_content_results && !self.app.global_search_input.is_empty() && self.app.global_search_input.len() >= 2 {
            ui.add_space(20.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("No results found").font(small()).color(tc.fg_dim));
            });
            return;
        }

        let file_results = self.app.file_search_results.clone();
        let results = self.app.global_search_results.clone();
        let root = self.app.file_tree.root_path.clone().unwrap_or_default();

        // Group results by file
        let mut grouped: Vec<(String, String, Vec<(usize, &crate::search::SearchResult)>)> = Vec::new();
        for (idx, r) in results.iter().enumerate() {
            let rel_path = r.file_path.strip_prefix(&root).unwrap_or(&r.file_path)
                .trim_start_matches('/').to_string();
            if let Some(group) = grouped.last_mut() {
                if group.0 == r.file_path {
                    group.2.push((idx, r));
                    continue;
                }
            }
            grouped.push((r.file_path.clone(), rel_path, vec![(idx, r)]));
        }

        let mut open_file_path: Option<String> = None;
        let mut open_file_line: Option<(String, usize, usize)> = None;

        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;

            if !file_results.is_empty() {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    ui.label(RichText::new(format!("📂 Files ({})", file_results.len()))
                        .font(FontId::monospace(11.0)).color(tc.accent).strong());
                });
                ui.add_space(2.0);

                for fm in &file_results {
                    let (row_rect, row_resp) = ui.allocate_exact_size(
                        Vec2::new(ui.available_width(), 22.0),
                        egui::Sense::click(),
                    );
                    if row_resp.hovered() {
                        ui.painter().rect_filled(row_rect, Rounding::ZERO, tc.selection_bg);
                    }
                    let dot_c = file_icon_color(&fm.file_name, dark);
                    let painter = ui.painter();
                    painter.circle_filled(
                        Pos2::new(row_rect.min.x + 16.0, row_rect.min.y + 11.0), 3.0, dot_c,
                    );
                    painter.text(
                        Pos2::new(row_rect.min.x + 24.0, row_rect.min.y + 4.0),
                        egui::Align2::LEFT_TOP, &fm.file_name, FontId::monospace(11.0), tc.fg,
                    );
                    let name_w = painter.text(
                        Pos2::new(row_rect.min.x + 24.0, row_rect.min.y + 4.0),
                        egui::Align2::LEFT_TOP, &fm.file_name, FontId::monospace(11.0), tc.fg,
                    ).width();
                    painter.text(
                        Pos2::new(row_rect.min.x + 32.0 + name_w, row_rect.min.y + 4.0),
                        egui::Align2::LEFT_TOP, &fm.rel_path, FontId::monospace(10.0), tc.fg_dim,
                    );

                    if row_resp.clicked() {
                        open_file_path = Some(fm.file_path.clone());
                    }
                }

                if !results.is_empty() {
                    ui.add_space(6.0);
                    ui.painter().line_segment(
                        [Pos2::new(ui.max_rect().min.x + 8.0, ui.cursor().min.y),
                         Pos2::new(ui.max_rect().max.x - 8.0, ui.cursor().min.y)],
                        Stroke::new(1.0, tc.border),
                    );
                    ui.add_space(4.0);
                }
            }

            if !results.is_empty() {
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    ui.label(RichText::new(format!("📄 Content ({})", results.len()))
                        .font(FontId::monospace(11.0)).color(tc.accent).strong());
                });
                ui.add_space(2.0);
            }

            for (file_path, rel_path, matches) in &grouped {
                let file_name = std::path::Path::new(file_path)
                    .file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                let dot_c = file_icon_color(&file_name, dark);

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    let (r, _) = ui.allocate_exact_size(Vec2::new(8.0, 14.0), egui::Sense::hover());
                    ui.painter().circle_filled(Pos2::new(r.min.x + 4.0, r.center().y), 3.0, dot_c);
                    ui.label(RichText::new(&file_name).font(FontId::monospace(11.0)).color(tc.fg).strong());
                    ui.label(RichText::new(format!("  {}", rel_path)).font(FontId::monospace(10.0)).color(tc.fg_dim));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(8.0);
                        ui.label(RichText::new(format!("{}", matches.len())).font(FontId::monospace(10.0)).color(tc.accent));
                    });
                });

                for (_idx, m) in matches {
                    let (row_rect, row_resp) = ui.allocate_exact_size(
                        Vec2::new(ui.available_width(), 20.0),
                        egui::Sense::click(),
                    );

                    if row_resp.hovered() {
                        ui.painter().rect_filled(row_rect, Rounding::ZERO, tc.selection_bg);
                    }

                    let painter = ui.painter();
                    painter.text(
                        Pos2::new(row_rect.min.x + 16.0, row_rect.min.y + 3.0),
                        egui::Align2::LEFT_TOP,
                        format!("{}", m.line_number),
                        FontId::monospace(10.0), tc.gutter_fg,
                    );

                    let line = &m.line_content;
                    let trimmed = line.trim_start();
                    let trim_offset = line.len() - trimmed.len();
                    let display_line: String = if trimmed.len() > 80 {
                        trimmed.chars().take(80).collect::<String>() + "…"
                    } else {
                        trimmed.to_string()
                    };

                    let text_x = row_rect.min.x + 52.0;
                    let text_y = row_rect.min.y + 3.0;

                    let hl_start_byte = m.match_start.saturating_sub(trim_offset);
                    let hl_end_byte = m.match_end.saturating_sub(trim_offset);

                    let safe_start = display_line.char_indices()
                        .map(|(i, _)| i)
                        .find(|&i| i >= hl_start_byte)
                        .unwrap_or(display_line.len());
                    let safe_end = display_line.char_indices()
                        .map(|(i, _)| i)
                        .find(|&i| i >= hl_end_byte)
                        .unwrap_or(display_line.len());

                    if safe_start < display_line.len() && safe_end <= display_line.len() && safe_start < safe_end {
                        let pre = &display_line[..safe_start];
                        let match_text = &display_line[safe_start..safe_end];
                        let post = &display_line[safe_end..];

                        let pre_w = painter.text(
                            Pos2::new(text_x, text_y), egui::Align2::LEFT_TOP,
                            pre, FontId::monospace(10.0), tc.fg_dim,
                        ).width();
                        let match_w = painter.text(
                            Pos2::new(text_x + pre_w, text_y), egui::Align2::LEFT_TOP,
                            match_text, FontId::monospace(10.0), tc.accent,
                        ).width();
                        painter.rect_filled(
                            Rect::from_min_size(
                                Pos2::new(text_x + pre_w - 1.0, text_y - 1.0),
                                Vec2::new(match_w + 2.0, 13.0),
                            ),
                            Rounding::same(2), tc.accent.linear_multiply(0.15),
                        );
                        painter.text(
                            Pos2::new(text_x + pre_w, text_y), egui::Align2::LEFT_TOP,
                            match_text, FontId::monospace(10.0), tc.accent,
                        );
                        if !post.is_empty() {
                            painter.text(
                                Pos2::new(text_x + pre_w + match_w, text_y), egui::Align2::LEFT_TOP,
                                post, FontId::monospace(10.0), tc.fg_dim,
                            );
                        }
                    } else {
                        painter.text(
                            Pos2::new(text_x, text_y), egui::Align2::LEFT_TOP,
                            &display_line, FontId::monospace(10.0), tc.fg_dim,
                        );
                    }

                    if row_resp.clicked() {
                        open_file_line = Some((m.file_path.clone(), m.line_number.saturating_sub(1), m.match_start));
                    }
                }
            }
        });

        if let Some(fp) = open_file_path {
            if std::path::Path::new(&fp).is_dir() {
                self.app.open_folder(fp);
            } else {
                self.app.open_file(&fp);
                self.app.file_tree.reveal_path(&fp);
                self.app.sidebar_tab = SidebarTab::Files;
                self.app.focus = Focus::Editor;
            }
        }
        if let Some((fp, line, col)) = open_file_line {
            self.app.open_file(&fp);
            self.app.file_tree.reveal_path(&fp);
            self.app.sidebar_tab = SidebarTab::Files;
            self.app.active_editor_mut().cursor.line = line;
            self.app.active_editor_mut().cursor.col = col;
            self.app.active_editor_mut().scroll_into_view();
            // Highlight search term in editor
            if !self.app.global_search_input.is_empty() {
                self.app.find_input = self.app.global_search_input.clone();
                self.app.update_find_matches();
            }
            self.app.focus = Focus::Editor;
        }
    }
}
