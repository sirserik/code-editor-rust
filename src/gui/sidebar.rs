use super::*;

impl CodeEditorApp {
    pub(super) fn render_sidebar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("sidebar")
            .default_width(280.0).min_width(180.0).max_width(440.0)
            .frame(egui::Frame::NONE.fill(self.tc.sidebar_bg).inner_margin(0.0))
            .show(ctx, |ui| {
                let r = ui.max_rect();
                ui.painter().line_segment(
                    [Pos2::new(r.max.x, r.min.y), Pos2::new(r.max.x, r.max.y)],
                    Stroke::new(1.0, self.tc.border),
                );

                // Tool-window header: tab label on the left, action icons on the right.
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.add_space(14.0);
                    let title = match self.app.sidebar_tab {
                        SidebarTab::Files => "EXPLORER",
                        SidebarTab::Git => "GIT",
                        SidebarTab::Search => "FIND",
                    };
                    ui.label(
                        RichText::new(title)
                            .font(FontId::monospace(11.0))
                            .color(self.tc.fg_dim),
                    );

                    // Right-aligned action buttons (vector icons — no font dependency).
                    if self.app.sidebar_tab == SidebarTab::Files {
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                ui.spacing_mut().item_spacing.x = 6.0;
                                ui.add_space(10.0);
                                // Reverse visual order due to right_to_left layout.
                                if self.header_icon_button(ui, super::icons::refresh, "Refresh tree") {
                                    self.app.file_tree.refresh();
                                }
                                if self.header_icon_button(ui, super::icons::menu_bars, "Collapse all") {
                                    self.app.file_tree.collapse_all();
                                }
                                if self.header_icon_button(ui, super::icons::plus, "New file") {
                                    self.app.pending_action = Some(PaletteAction::NewFile);
                                }
                            },
                        );
                    }
                });
                ui.add_space(6.0);
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

    /// Vertical activity bar in JetBrains New UI style: thin strip of tool-window icons at
    /// the left edge. Clicking an active icon hides the sidebar; inactive switches tab.
    pub(super) fn render_activity_bar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("activity_bar")
            .exact_width(56.0)
            .resizable(false)
            .frame(egui::Frame::NONE.fill(self.tc.tab_bar_bg).inner_margin(egui::Margin { left: 0, right: 0, top: 10, bottom: 10 }))
            .show(ctx, |ui| {
                let r = ui.max_rect();
                // Right border separator
                ui.painter().line_segment(
                    [Pos2::new(r.max.x, r.min.y), Pos2::new(r.max.x, r.max.y)],
                    Stroke::new(1.0, self.tc.border),
                );

                // Top section: switchable tool windows. Icon kind selects which vector
                // routine paints the glyph — no font dependency.
                type IconFn = fn(&egui::Painter, Rect, Color32);
                let tabs: &[(IconFn, SidebarTab, &str)] = &[
                    (super::icons::document, SidebarTab::Files,  "Project"),
                    (super::icons::search,   SidebarTab::Search, "Search"),
                    (super::icons::branch,   SidebarTab::Git,    "Git"),
                ];
                for (draw, tab, tip) in tabs {
                    let active = self.app.show_sidebar && self.app.sidebar_tab == *tab;
                    let has_git_changes = *tip == "Git"
                        && self
                            .app
                            .git_status
                            .as_ref()
                            .map(|g| g.is_repo && !g.files.is_empty())
                            .unwrap_or(false);
                    if self.activity_button(ui, *draw, *tip, active, false, has_git_changes) {
                        if active {
                            self.app.show_sidebar = false;
                        } else {
                            self.app.sidebar_tab = *tab;
                            self.app.show_sidebar = true;
                        }
                    }
                }

                // Stubs mirroring the mockup. Disabled (no click handler).
                self.activity_button(ui, super::icons::gear, "Settings",   false, true, false);
                self.activity_button(ui, super::icons::grid, "Extensions", false, true, false);

                // Push the theme toggle to the very bottom.
                let avail = ui.available_size();
                if avail.y > 56.0 {
                    ui.add_space(avail.y - 56.0);
                }
                if self.activity_button(ui, super::icons::sun, "Toggle theme", false, false, false) {
                    let themes = Theme::ALL;
                    let idx = themes
                        .iter()
                        .position(|t| *t == self.app.settings.theme)
                        .unwrap_or(0);
                    self.app.settings.theme = themes[(idx + 1) % themes.len()];
                    self.app.settings.save();
                }
            });
    }

    /// Small 22×22 vector-icon button used in sidebar/header rows.
    fn header_icon_button(
        &self,
        ui: &mut egui::Ui,
        draw: fn(&egui::Painter, Rect, Color32),
        tooltip: &str,
    ) -> bool {
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(22.0, 22.0), egui::Sense::click());
        let resp = resp.on_hover_text(tooltip);
        if resp.hovered() {
            ui.painter().rect_filled(
                rect,
                CornerRadius::same(4),
                Color32::from_rgba_unmultiplied(255, 255, 255, 14),
            );
        }
        let icon_rect = Rect::from_center_size(rect.center(), Vec2::splat(14.0));
        draw(ui.painter(), icon_rect, self.tc.fg_dim);
        resp.clicked()
    }

    /// 40×40 activity-bar pill. Returns true when clicked.
    /// `disabled` greys the icon and skips both hover and click.
    /// `indicator_dot` paints a small accent dot in the upper-right (used on Git when dirty).
    fn activity_button(
        &self,
        ui: &mut egui::Ui,
        draw: fn(&egui::Painter, Rect, Color32),
        tooltip: &str,
        active: bool,
        disabled: bool,
        indicator_dot: bool,
    ) -> bool {
        let tc = self.tc;
        let mut clicked = false;
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            let btn_size = Vec2::new(40.0, 40.0);
            let (rect, resp) = ui.allocate_exact_size(btn_size, egui::Sense::click());
            let resp = resp.on_hover_text(tooltip);
            if active {
                ui.painter().rect_filled(
                    rect,
                    CornerRadius::same(10),
                    Color32::from_rgba_unmultiplied(tc.accent.r(), tc.accent.g(), tc.accent.b(), 46),
                );
            } else if !disabled && resp.hovered() {
                ui.painter().rect_filled(
                    rect,
                    CornerRadius::same(10),
                    Color32::from_rgba_unmultiplied(255, 255, 255, 10),
                );
            }
            let color = if active {
                Color32::WHITE
            } else if disabled {
                Color32::from_rgba_unmultiplied(tc.fg_dim.r(), tc.fg_dim.g(), tc.fg_dim.b(), 130)
            } else {
                tc.fg_dim
            };
            // 22×22 icon centered in the 40×40 pill
            let icon_rect = Rect::from_center_size(rect.center(), Vec2::splat(22.0));
            draw(ui.painter(), icon_rect, color);
            if indicator_dot {
                ui.painter().circle_filled(
                    Pos2::new(rect.max.x - 9.0, rect.min.y + 9.0),
                    3.0,
                    tc.accent,
                );
            }
            if !disabled && resp.clicked() {
                clicked = true;
            }
        });
        clicked
    }

    fn render_file_tree(&mut self, ui: &mut egui::Ui) {
        let dark = self.app.settings.theme.resolved() != Theme::Light;
        // The EXPLORER header (in render_sidebar) carries the +/collapse/refresh actions.
        // The "open another folder" and "close project" actions live in File menu + ⌘O / ⌘⇧W.
        let row_h = 28.0;
        let indent_px = 18.0;

        let total_entries = self.app.file_tree.flat_entries.len();
        let drag_active = self.drag_source.is_some();
        if drag_active { self.drop_target = None; }

        // Deferred mutations — collected during the row render pass, applied after.
        // This avoids per-frame clones of flat_entries and lets us use show_rows for
        // visible-row-only iteration (huge win on large projects).
        let mut new_drop_target: Option<(usize, String)> = None;
        let mut toggle_expand_idx: Option<usize> = None;
        let mut open_file_path: Option<String> = None;
        let mut select_idx: Option<usize> = None;
        let mut start_drag: Option<(usize, String)> = None;
        let mut start_new_file = false;
        let mut start_new_folder = false;
        let mut start_rename_for: Option<usize> = None;
        let mut duplicate_path: Option<String> = None;
        let mut want_delete_for: Option<usize> = None;
        let mut close_in_editor_path: Option<String> = None;
        let mut close_project_now = false;
        let mut pending_open_folder = false;

        egui::ScrollArea::vertical().auto_shrink([false, false]).show_rows(ui, row_h, total_entries, |ui, row_range| {
            ui.spacing_mut().item_spacing.y = 0.0;
            let drag_source_idx = self.drag_source.as_ref().map(|(i, _)| *i);
            let pointer_pos = ui.ctx().input(|i| i.pointer.hover_pos());

            for i in row_range {
                // Snapshot only the fields we need this iteration. Path/name clone only for
                // the visible slice (~30 rows), not for the entire tree.
                let (depth, name, path, is_directory, is_expanded) = {
                    let e = &self.app.file_tree.flat_entries[i];
                    (e.depth, e.name.clone(), e.path.clone(), e.is_directory, e.is_expanded)
                };
                let entry_name = name.as_str();
                let entry_path = path.as_str();
                let indent = indent_px * depth as f32 + 8.0;
                let sel = i == self.app.file_tree.selected_index;
                let is_dragged = drag_source_idx == Some(i);

                let (row_rect, row_resp) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), row_h),
                    egui::Sense::click_and_drag(),
                );

                let is_drop = if drag_active && is_directory {
                    pointer_pos.is_some_and(|p| row_rect.contains(p))
                } else {
                    false
                };

                let drop_bg = if dark { Color32::from_rgb(35, 50, 80) } else { Color32::from_rgb(225, 235, 248) };
                let hover_bg = if dark { Color32::from_rgb(54, 60, 70) } else { Color32::from_rgb(236, 238, 242) }; // Zed: #363c46
                let is_hovered = row_resp.hovered() && !is_drop && !sel;
                let bg = if is_drop { drop_bg } else if sel { self.tc.selection_bg } else if is_hovered { hover_bg } else { Color32::TRANSPARENT };

                if bg != Color32::TRANSPARENT {
                    ui.painter().rect_filled(row_rect, CornerRadius::ZERO, bg);
                }

                if is_drop {
                    ui.painter().rect_stroke(row_rect, CornerRadius::same(3), Stroke::new(2.0, self.tc.accent), egui::StrokeKind::Outside);
                    ui.painter().rect_filled(
                        Rect::from_min_size(row_rect.min, Vec2::new(3.0, row_rect.height())),
                        CornerRadius::ZERO,
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
                let text_color = if sel {
                    self.tc.fg
                } else if entry_name.starts_with('.') {
                    self.tc.fg_dim
                } else {
                    self.tc.fg
                }.linear_multiply(dim);
                let arrow_color = if sel {
                    self.tc.fg
                } else if dark {
                    Color32::from_rgb(160, 165, 175)
                } else {
                    Color32::from_rgb(95, 100, 110)
                }.linear_multiply(dim);

                if is_directory {
                    // Chevron (dim) + hollow folder outline (dim gray) — matches the
                    // Ferrite mockup where folders are subtle, not the icon's hero.
                    let arr = if is_expanded { "▾" } else { "▸" };
                    painter.text(
                        Pos2::new(row_rect.min.x + indent, row_y),
                        egui::Align2::LEFT_TOP, arr, FontId::monospace(12.0), arrow_color,
                    );
                    let folder_color = self.tc.fg_dim.linear_multiply(dim);
                    let folder_rect = Rect::from_min_size(
                        Pos2::new(row_rect.min.x + indent + 14.0, row_rect.min.y + (row_h - 14.0) / 2.0),
                        Vec2::splat(14.0),
                    );
                    super::icons::folder_outline(painter, folder_rect, folder_color);
                    painter.text(
                        Pos2::new(row_rect.min.x + indent + 34.0, row_y),
                        egui::Align2::LEFT_TOP, entry_name, FontId::monospace(13.0), text_color,
                    );
                } else {
                    // File row: colored dot + filename (kept simple — JBMono has no
                    // material-icon glyphs, and adding an icon font would be heavier).
                    let dot_color = file_icon_color(entry_name, dark).linear_multiply(dim);
                    let dot_y = row_rect.min.y + row_h / 2.0;
                    painter.circle_filled(
                        Pos2::new(row_rect.min.x + indent + 18.0, dot_y), 4.0, dot_color,
                    );
                    painter.text(
                        Pos2::new(row_rect.min.x + indent + 30.0, row_y),
                        egui::Align2::LEFT_TOP, entry_name, FontId::monospace(13.0), text_color,
                    );
                }

                if row_resp.drag_started() {
                    start_drag = Some((i, path.clone()));
                }

                if row_resp.clicked() && !drag_active {
                    select_idx = Some(i);
                    if is_directory {
                        toggle_expand_idx = Some(i);
                    } else {
                        open_file_path = Some(path.clone());
                    }
                }

                if drag_active {
                    if let Some(pp) = pointer_pos {
                        if row_rect.contains(pp) {
                            if is_directory {
                                new_drop_target = Some((i, path.clone()));
                            } else if let Some(parent) = std::path::Path::new(entry_path).parent() {
                                new_drop_target = Some((i, parent.to_string_lossy().to_string()));
                            }
                        }
                    }
                }

                let is_root = depth == 0;
                row_resp.context_menu(|ui| {
                    select_idx = Some(i);
                    if ui.button("New File Here").clicked() { start_new_file = true; ui.close_menu(); }
                    if ui.button("New Folder Here").clicked() { start_new_folder = true; ui.close_menu(); }
                    ui.separator();
                    if !is_directory {
                        if ui.button("Close in Editor").on_hover_text("Close this file's tabs (file stays on disk)").clicked() {
                            close_in_editor_path = Some(path.clone());
                            ui.close_menu();
                        }
                        ui.separator();
                    }
                    if !is_root {
                        if ui.button("Rename").clicked() { start_rename_for = Some(i); ui.close_menu(); }
                        if ui.button("Duplicate").clicked() {
                            duplicate_path = Some(path.clone());
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Delete from disk").clicked() { want_delete_for = Some(i); ui.close_menu(); }
                    } else {
                        if ui.button("Open Another Folder…  ⌘O").clicked() {
                            pending_open_folder = true;
                            ui.close_menu();
                        }
                        if ui.button("Close Project  ⌘⇧W").clicked() {
                            close_project_now = true;
                            ui.close_menu();
                        }
                    }
                });
            }
        });

        // Apply all deferred mutations after the render pass.
        if let Some(dt) = new_drop_target { self.drop_target = Some(dt); }
        if let Some(idx) = select_idx { self.app.file_tree.selected_index = idx; }
        if let Some(d) = start_drag { self.drag_source = Some(d); }
        if let Some(idx) = toggle_expand_idx { self.app.file_tree.toggle_expand(idx); }
        if let Some(p) = open_file_path { self.app.open_file(&p); }
        if start_new_file { self.app.start_new_file_dialog(); }
        if start_new_folder { self.app.start_new_folder_dialog(); }
        if let Some(idx) = start_rename_for { self.app.file_tree.selected_index = idx; self.app.start_rename_dialog(); }
        if let Some(p) = duplicate_path { let _ = self.app.file_tree.duplicate_entry(&p); }
        if let Some(idx) = want_delete_for { self.app.file_tree.selected_index = idx; self.app.focus = Focus::DeleteConfirm; }
        if close_project_now { self.app.close_project(); return; }
        if pending_open_folder { self.app.pending_action = Some(crate::app::PaletteAction::OpenFolder); }
        if let Some(p) = close_in_editor_path {
            // Close every tab pointing to that file. If only tab gets closed, leave an
            // untitled buffer so the editor never has zero tabs.
            let mut i = 0;
            while i < self.app.editors.len() {
                if self.app.editors[i].file_path.as_deref() == Some(p.as_str()) {
                    self.app.close_tab(i);
                } else {
                    i += 1;
                }
            }
        }

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

        let status = self.app.git_status.clone();
        let status = match status {
            Some(s) if s.is_repo => s,
            Some(_) => {
                ui.vertical_centered(|ui| {
                    ui.add_space(20.0);
                    ui.label(RichText::new("Not a git repository").font(small()).color(tc.fg_dim));
                });
                return;
            }
            None => {
                ui.vertical_centered(|ui| {
                    ui.add_space(20.0);
                    ui.label(RichText::new("Open a folder to see git status").font(small()).color(tc.fg_dim));
                });
                return;
            }
        };

        let has_staged = status.files.iter().any(|f| f.staged);
        let staged_count = status.files.iter().filter(|f| f.staged).count();
        let unstaged_count = status.files.len() - staged_count;
        let root = self.app.file_tree.root_path.clone();

        // ── Branch row + Pull / Push / Fetch / Refresh ──
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            // Branch is a button that opens the branch switcher popup.
            let branch_btn = ui.add(
                egui::Button::new(RichText::new(format!("⎇ {} ▾", status.branch))
                    .font(small()).color(tc.accent))
                .fill(Color32::TRANSPARENT)
                .stroke(Stroke::NONE)
                .min_size(Vec2::new(0.0, 20.0))
                .corner_radius(CornerRadius::same(3))
            ).on_hover_text("Switch branch · ⌥ to list");
            if branch_btn.clicked() {
                self.app.branch_popup_open = !self.app.branch_popup_open;
                self.app.branch_popup_anchor = Some(branch_btn.rect.left_bottom());
                self.app.branch_popup_query.clear();
                self.app.branch_popup_new_mode = false;
                self.app.branch_popup_new_name.clear();
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(4.0);
                ui.spacing_mut().item_spacing.x = 2.0;
                let icon_btn = |ui: &mut egui::Ui, txt: &str, tip: &str, color: Color32| -> bool {
                    ui.add(egui::Button::new(RichText::new(txt).font(FontId::monospace(13.0)).color(color))
                        .fill(Color32::TRANSPARENT).min_size(Vec2::new(22.0, 20.0))
                        .corner_radius(CornerRadius::same(3))
                    ).on_hover_text(tip).clicked()
                };
                if icon_btn(ui, "↻", "Refresh git status", tc.fg_dim) {
                    self.app.refresh_git_async();
                }
                if icon_btn(ui, "↑", "Push (git push)", tc.fg_dim) {
                    if let Some(ref r) = root { Self::spawn_git_cli(r.clone(), vec!["push".into()], "Push", self.app.status_message.clone().into()); self.app.status_message = "Pushing…".into(); }
                }
                if icon_btn(ui, "↓", "Pull (git pull)", tc.fg_dim) {
                    if let Some(ref r) = root { Self::spawn_git_cli(r.clone(), vec!["pull".into()], "Pull", self.app.status_message.clone().into()); self.app.status_message = "Pulling…".into(); }
                }
                if icon_btn(ui, "⇣", "Fetch (git fetch)", tc.fg_dim) {
                    if let Some(ref r) = root { Self::spawn_git_cli(r.clone(), vec!["fetch".into()], "Fetch", self.app.status_message.clone().into()); self.app.status_message = "Fetching…".into(); }
                }
            });
        });
        ui.add_space(4.0);
        ui.painter().line_segment(
            [Pos2::new(ui.max_rect().min.x, ui.cursor().min.y), Pos2::new(ui.max_rect().max.x, ui.cursor().min.y)],
            Stroke::new(1.0, tc.border),
        );
        ui.add_space(6.0);

        // ── Inline commit message + Commit / Commit&Push buttons ──
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.set_max_width(ui.available_width() - 16.0);
                ui.label(RichText::new("Commit message").font(FontId::monospace(10.0)).color(tc.fg_dim));
                ui.add(egui::TextEdit::multiline(&mut self.app.commit_message)
                    .font(FontId::monospace(12.0))
                    .hint_text("Describe your changes…")
                    .desired_rows(3)
                    .desired_width(ui.available_width())
                    .text_color(tc.fg));
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    let can_commit = has_staged && !self.app.commit_message.trim().is_empty();
                    let commit_color = if can_commit { tc.green } else { tc.fg_dim };
                    if ui.add_enabled(can_commit,
                        egui::Button::new(RichText::new(format!("Commit ({} staged)", staged_count))
                            .font(FontId::monospace(11.0)).color(commit_color))
                            .fill(Color32::TRANSPARENT).stroke(Stroke::new(1.0, commit_color))
                            .corner_radius(CornerRadius::same(3))
                            .min_size(Vec2::new(0.0, 22.0))
                    ).clicked() {
                        if let Some(ref r) = root {
                            match self.app.git.commit(r, &self.app.commit_message) {
                                Ok(_) => {
                                    self.app.status_message = "Commit created".into();
                                    self.app.commit_message.clear();
                                    self.app.refresh_git_status();
                                }
                                Err(e) => { self.app.status_message = format!("Commit failed: {}", e); }
                            }
                        }
                    }
                    if ui.add_enabled(can_commit,
                        egui::Button::new(RichText::new("Commit & Push")
                            .font(FontId::monospace(11.0)).color(commit_color))
                            .fill(Color32::TRANSPARENT).stroke(Stroke::new(1.0, commit_color))
                            .corner_radius(CornerRadius::same(3))
                            .min_size(Vec2::new(0.0, 22.0))
                    ).clicked() {
                        if let Some(ref r) = root {
                            match self.app.git.commit(r, &self.app.commit_message) {
                                Ok(_) => {
                                    self.app.commit_message.clear();
                                    Self::spawn_git_cli(r.clone(), vec!["push".into()], "Push", None);
                                    self.app.status_message = "Commit done, pushing…".into();
                                    self.app.refresh_git_status();
                                }
                                Err(e) => { self.app.status_message = format!("Commit failed: {}", e); }
                            }
                        }
                    }
                });
            });
        });
        ui.add_space(6.0);
        ui.painter().line_segment(
            [Pos2::new(ui.max_rect().min.x, ui.cursor().min.y), Pos2::new(ui.max_rect().max.x, ui.cursor().min.y)],
            Stroke::new(1.0, tc.border),
        );
        ui.add_space(4.0);

        // ── File lists with checkbox-style staging ──
        let mut action_stage: Option<String> = None;
        let mut action_unstage: Option<String> = None;
        let mut action_discard: Option<String> = None;
        let mut action_open: Option<String> = None;
        let mut stage_all = false;
        let mut unstage_all = false;

        // Build file lists before entering the scroll-area closure so they outlive it.
        let staged_files: Vec<(String, &'static str)> = status.files.iter()
            .filter(|f| f.staged)
            .map(|f| (f.path.clone(), f.status.symbol()))
            .collect();
        let unstaged_files: Vec<(String, &'static str)> = status.files.iter()
            .filter(|f| !f.staged)
            .map(|f| (f.path.clone(), f.status.symbol()))
            .collect();

        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            let render_section = |ui: &mut egui::Ui,
                title: &str,
                count: usize,
                title_color: Color32,
                files: Vec<(String, &'static str)>,
                section_action_caption: &str,
                section_action_flag: &mut bool,
                stage_checked: bool,
                tc: ThemeColors,
                stage_out: &mut Option<String>,
                unstage_out: &mut Option<String>,
                discard_out: &mut Option<String>,
                open_out: &mut Option<String>,
            | {
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    ui.label(RichText::new(format!("{} ({})", title, count))
                        .font(FontId::monospace(10.5)).color(title_color).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(8.0);
                        if count > 0 && ui.add(egui::Button::new(
                            RichText::new(section_action_caption).font(FontId::monospace(10.0)).color(tc.fg_dim)
                        ).fill(Color32::TRANSPARENT).min_size(Vec2::new(0.0, 18.0))
                        .corner_radius(CornerRadius::same(3))).clicked() {
                            *section_action_flag = true;
                        }
                    });
                });
                ui.add_space(2.0);

                for (path, status_sym) in files {
                    let (row_rect, row_resp) = ui.allocate_exact_size(
                        Vec2::new(ui.available_width(), 22.0),
                        egui::Sense::click(),
                    );
                    if row_resp.hovered() {
                        ui.painter().rect_filled(row_rect, CornerRadius::ZERO, tc.selection_bg);
                    }
                    // Checkbox at left (☑ if staged, ☐ if not)
                    let checkbox_x = row_rect.min.x + 8.0;
                    let cb_y = row_rect.min.y + 11.0;
                    let cb_rect = Rect::from_center_size(Pos2::new(checkbox_x + 6.0, cb_y), Vec2::new(14.0, 14.0));
                    let cb_resp = ui.allocate_rect(cb_rect, egui::Sense::click());
                    let cb_color = if stage_checked { tc.green } else { tc.fg_dim };
                    ui.painter().rect_stroke(cb_rect, CornerRadius::same(2), Stroke::new(1.0, cb_color), egui::StrokeKind::Outside);
                    if stage_checked {
                        ui.painter().text(cb_rect.center(), egui::Align2::CENTER_CENTER, "✓",
                            FontId::monospace(11.0), tc.green);
                    }
                    if cb_resp.clicked() {
                        if stage_checked { *unstage_out = Some(path.clone()); }
                        else { *stage_out = Some(path.clone()); }
                    }
                    // Status letter + path
                    let status_color = match status_sym {
                        "M" => tc.orange,
                        "A" => tc.green,
                        "D" => tc.red,
                        "?" => tc.fg_dim,
                        _ => tc.fg,
                    };
                    ui.painter().text(
                        Pos2::new(row_rect.min.x + 24.0, row_rect.min.y + 4.0),
                        egui::Align2::LEFT_TOP, status_sym, FontId::monospace(11.0), status_color,
                    );
                    ui.painter().text(
                        Pos2::new(row_rect.min.x + 38.0, row_rect.min.y + 4.0),
                        egui::Align2::LEFT_TOP, path.as_str(), FontId::monospace(11.0), tc.fg,
                    );
                    if row_resp.clicked() && !cb_resp.clicked() {
                        *open_out = Some(path.clone());
                    }
                    row_resp.context_menu(|ui| {
                        if stage_checked {
                            if ui.button("Unstage").clicked() { *unstage_out = Some(path.clone()); ui.close_menu(); }
                        } else {
                            if ui.button("Stage").clicked() { *stage_out = Some(path.clone()); ui.close_menu(); }
                            if ui.button("Discard Changes").clicked() { *discard_out = Some(path.clone()); ui.close_menu(); }
                        }
                        if ui.button("Open File").clicked() { *open_out = Some(path.clone()); ui.close_menu(); }
                    });
                }
                ui.add_space(6.0);
            };

            if staged_count > 0 {
                render_section(ui, "Staged", staged_count, tc.green, staged_files,
                    "Unstage All", &mut unstage_all, true, tc,
                    &mut action_stage, &mut action_unstage, &mut action_discard, &mut action_open);
            }
            if unstaged_count > 0 {
                render_section(ui, "Changes", unstaged_count, tc.orange, unstaged_files,
                    "Stage All", &mut stage_all, false, tc,
                    &mut action_stage, &mut action_unstage, &mut action_discard, &mut action_open);
            }
            if staged_count == 0 && unstaged_count == 0 {
                ui.add_space(20.0);
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new("Nothing to commit. Working tree clean.").font(small()).color(tc.fg_dim));
                });
            }
        });

        // ── Execute deferred actions ──
        let root = self.app.file_tree.root_path.clone();
        if let Some(ref r) = root {
            if let Some(path) = action_stage {
                match self.app.git.stage_file(r, &path) {
                    Ok(_) => { self.app.status_message = format!("Staged: {}", path); self.app.refresh_git_status(); }
                    Err(e) => self.app.status_message = format!("Error: {}", e),
                }
            }
            if let Some(path) = action_unstage {
                match self.app.git.unstage_file(r, &path) {
                    Ok(_) => { self.app.status_message = format!("Unstaged: {}", path); self.app.refresh_git_status(); }
                    Err(e) => self.app.status_message = format!("Error: {}", e),
                }
            }
            if let Some(path) = action_discard {
                match self.app.git.discard_file(r, &path) {
                    Ok(_) => { self.app.status_message = format!("Discarded: {}", path); self.app.refresh_git_status(); }
                    Err(e) => self.app.status_message = format!("Error: {}", e),
                }
            }
            if stage_all {
                match self.app.git.stage_all(r) {
                    Ok(_) => { self.app.status_message = "Staged all files".into(); self.app.refresh_git_status(); }
                    Err(e) => self.app.status_message = format!("Error: {}", e),
                }
            }
            if unstage_all {
                // Unstage all = `git reset` (reset index to HEAD)
                let r = r.clone();
                match crate::git::GitManager::run_cli(&r, &["reset"]) {
                    Ok(_) => { self.app.status_message = "Unstaged all files".into(); self.app.refresh_git_status(); }
                    Err(e) => self.app.status_message = format!("Error: {}", e),
                }
            }
            if let Some(path) = action_open {
                let full_path = format!("{}/{}", r, path);
                self.app.open_file(&full_path);
                self.app.focus = Focus::Editor;
            }
        }

        // ── Branch switcher popup ──
        if self.app.branch_popup_open {
            self.render_branch_popup(ui.ctx(), &status.branch);
        }
    }

    fn render_branch_popup(&mut self, ctx: &egui::Context, current_branch: &str) {
        let tc = self.tc;
        let root = match self.app.file_tree.root_path.clone() {
            Some(r) => r,
            None => return,
        };
        let anchor = self.app.branch_popup_anchor.unwrap_or(egui::pos2(60.0, 60.0));
        // Click outside the popup → close.
        let escape = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        if escape {
            self.app.branch_popup_open = false;
            return;
        }

        let dark = self.app.settings.theme.resolved() != Theme::Light;
        let popup_bg = if dark { Color32::from_rgb(30, 31, 42) } else { Color32::from_rgb(255, 255, 255) };

        let mut close_after = false;
        let mut checkout_target: Option<String> = None;
        let mut create_name: Option<String> = None;

        let popup_id = egui::Id::new("branch_popup");
        let inner = egui::Area::new(popup_id)
            .order(egui::Order::Foreground)
            .fixed_pos(anchor + egui::vec2(0.0, 4.0))
            .show(ctx, |ui| {
                egui::Frame::NONE
                    .fill(popup_bg)
                    .stroke(Stroke::new(1.0, tc.border))
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(8.0)
                    .shadow(egui::epaint::Shadow {
                        offset: [0, 8], blur: 24, spread: 2,
                        color: Color32::from_black_alpha(if dark { 80 } else { 30 }),
                    })
                    .show(ui, |ui| {
                        ui.set_min_width(280.0);
                        ui.set_max_width(360.0);

                        if self.app.branch_popup_new_mode {
                            // Create new branch UI
                            ui.label(RichText::new("New branch from current HEAD")
                                .font(FontId::monospace(11.0)).color(tc.fg_dim));
                            ui.add_space(4.0);
                            let resp = ui.add(egui::TextEdit::singleline(&mut self.app.branch_popup_new_name)
                                .font(FontId::monospace(13.0))
                                .hint_text("branch name…")
                                .desired_width(ui.available_width())
                                .text_color(tc.fg));
                            resp.request_focus();
                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                let trimmed = self.app.branch_popup_new_name.trim().to_string();
                                let valid = !trimmed.is_empty();
                                let create_color = if valid { tc.green } else { tc.fg_dim };
                                if ui.add_enabled(valid,
                                    egui::Button::new(RichText::new("Create").font(FontId::monospace(11.0)).color(create_color))
                                        .fill(Color32::TRANSPARENT).stroke(Stroke::new(1.0, create_color))
                                        .corner_radius(CornerRadius::same(3))
                                ).clicked() || (valid && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                                {
                                    create_name = Some(trimmed);
                                    close_after = true;
                                }
                                if ui.button("Cancel").clicked() {
                                    self.app.branch_popup_new_mode = false;
                                    self.app.branch_popup_new_name.clear();
                                }
                            });
                        } else {
                            // Switch existing branch
                            ui.label(RichText::new("Switch branch")
                                .font(FontId::monospace(11.0)).color(tc.fg_dim));
                            ui.add_space(4.0);
                            let resp = ui.add(egui::TextEdit::singleline(&mut self.app.branch_popup_query)
                                .font(FontId::monospace(13.0))
                                .hint_text("Search branches…")
                                .desired_width(ui.available_width())
                                .text_color(tc.fg));
                            resp.request_focus();
                            ui.add_space(6.0);

                            let branches = crate::git::GitManager::list_branches(&root);
                            let query = self.app.branch_popup_query.to_lowercase();
                            let filtered: Vec<&crate::git::BranchInfo> = branches.iter()
                                .filter(|b| query.is_empty() || b.name.to_lowercase().contains(&query))
                                .collect();

                            egui::ScrollArea::vertical()
                                .max_height(320.0)
                                .auto_shrink([false; 2])
                                .show(ui, |ui| {
                                    ui.spacing_mut().item_spacing.y = 0.0;
                                    let mut last_was_local = true;
                                    for b in &filtered {
                                        if b.is_remote && last_was_local && !filtered.iter().all(|x| x.is_remote) {
                                            ui.add_space(4.0);
                                            ui.label(RichText::new("Remote")
                                                .font(FontId::monospace(10.0)).color(tc.fg_dim));
                                            ui.add_space(2.0);
                                            last_was_local = false;
                                        }
                                        let prefix = if b.is_current { "● " } else { "  " };
                                        let color = if b.is_current { tc.accent } else if b.is_remote { tc.fg_dim } else { tc.fg };
                                        let btn = egui::Button::new(
                                            RichText::new(format!("{}{}", prefix, b.name))
                                                .font(FontId::monospace(12.0)).color(color)
                                        )
                                        .fill(Color32::TRANSPARENT)
                                        .stroke(Stroke::NONE)
                                        .min_size(Vec2::new(ui.available_width(), 22.0))
                                        .corner_radius(CornerRadius::same(3));
                                        if ui.add(btn).clicked() && !b.is_current {
                                            checkout_target = Some(b.name.clone());
                                            close_after = true;
                                        }
                                    }
                                });

                            ui.add_space(6.0);
                            ui.separator();
                            ui.add_space(4.0);
                            if ui.add(egui::Button::new(RichText::new("+ New Branch from current HEAD")
                                .font(FontId::monospace(11.0)).color(tc.accent))
                                .fill(Color32::TRANSPARENT).stroke(Stroke::NONE)
                                .min_size(Vec2::new(ui.available_width(), 22.0))
                                .corner_radius(CornerRadius::same(3))
                            ).clicked() {
                                self.app.branch_popup_new_mode = true;
                                self.app.branch_popup_new_name.clear();
                            }
                        }
                    });
            });

        // Click outside popup → close. We detect by checking if pointer was pressed and not over popup.
        let pointer_pressed = ctx.input(|i| i.pointer.any_pressed());
        if pointer_pressed {
            let pos = ctx.input(|i| i.pointer.interact_pos());
            if let Some(p) = pos {
                if !inner.response.rect.contains(p) {
                    self.app.branch_popup_open = false;
                }
            }
        }
        let _ = current_branch;

        if close_after {
            self.app.branch_popup_open = false;
        }

        // Apply git actions after the popup closure released its borrows.
        if let Some(target) = checkout_target {
            match crate::git::GitManager::checkout_branch(&root, &target) {
                Ok(_) => {
                    self.app.status_message = format!("Switched to '{}'", target);
                    self.app.refresh_git_status();
                }
                Err(e) => self.app.status_message = format!("Checkout failed: {}", e),
            }
        }
        if let Some(name) = create_name {
            match crate::git::GitManager::create_branch(&root, &name) {
                Ok(_) => {
                    self.app.status_message = format!("Created branch '{}'", name);
                    self.app.refresh_git_status();
                }
                Err(e) => self.app.status_message = format!("Create branch failed: {}", e),
            }
            self.app.branch_popup_new_mode = false;
            self.app.branch_popup_new_name.clear();
        }
    }

    /// Spawn `git <args>` in repo on a background thread. Result is logged but UI doesn't
    /// block waiting on it — the user gets a status_message update via the next refresh.
    fn spawn_git_cli(repo: String, args: Vec<String>, label: &'static str, _hint: Option<String>) {
        std::thread::spawn(move || {
            let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
            let _ = crate::git::GitManager::run_cli(&repo, &args_str);
            // The next refresh_git_async (triggered by user clicking ↻ or focusing) will
            // pick up the new branch/files state.
            let _ = label; // currently unused but kept for future telemetry
        });
    }

    fn render_search(&mut self, ui: &mut egui::Ui) {
        let tc = self.tc;
        let dark = self.app.settings.theme.resolved() != Theme::Light;
        ui.add_space(6.0);

        ui.horizontal(|ui| {
            ui.add_space(8.0);
            // ⌄/⌃ toggles the replace row, JetBrains-style.
            let toggle_label = if self.app.global_search_show_replace { "⌄" } else { "⌃" };
            if ui.add(egui::Button::new(RichText::new(toggle_label).font(small()).color(tc.fg_dim))
                .fill(Color32::TRANSPARENT).min_size(Vec2::new(16.0, 18.0))
                .corner_radius(CornerRadius::same(3)))
                .on_hover_text("Toggle Replace").clicked()
            {
                self.app.global_search_show_replace = !self.app.global_search_show_replace;
            }
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

        // ── Replace row (collapsible) ──
        if self.app.global_search_show_replace {
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                ui.add(egui::Button::new(RichText::new("↳").font(small()).color(tc.fg_dim))
                    .fill(Color32::TRANSPARENT).min_size(Vec2::new(16.0, 18.0))
                    .corner_radius(CornerRadius::same(3)))
                    .on_hover_text("Replace below");
                ui.add(
                    egui::TextEdit::singleline(&mut self.app.global_search_replace)
                        .font(small())
                        .desired_width(ui.available_width() - 12.0)
                        .hint_text("Replace with…")
                );
            });
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                let n_files = self.app.global_search_results.iter()
                    .map(|r| r.file_path.clone())
                    .collect::<std::collections::HashSet<_>>().len();
                let n_matches = self.app.global_search_results.len();
                let can_replace = !self.app.global_search_input.is_empty()
                    && self.app.file_tree.root_path.is_some()
                    && n_matches > 0;
                let color = if can_replace { tc.orange } else { tc.fg_dim };
                let label = format!("Replace All in {} file{}", n_files, if n_files == 1 { "" } else { "s" });
                if ui.add_enabled(can_replace,
                    egui::Button::new(RichText::new(label).font(FontId::monospace(10.5)).color(color))
                        .fill(Color32::TRANSPARENT).stroke(Stroke::new(1.0, color))
                        .corner_radius(CornerRadius::same(3))
                        .min_size(Vec2::new(0.0, 20.0))
                ).on_hover_text("Replace every match across the project (no undo — commit first!)").clicked() {
                    if let Some(root) = self.app.file_tree.root_path.clone() {
                        let stats = crate::search::replace_in_project(
                            &root,
                            &self.app.global_search_input,
                            &self.app.global_search_replace,
                            self.app.find_case_sensitive,
                            self.app.find_use_regex,
                        );
                        self.app.status_message = if stats.errors.is_empty() {
                            format!("Replaced {} matches in {} files", stats.replacements, stats.files_changed)
                        } else {
                            format!("Replaced {} in {} files ({} errors)",
                                stats.replacements, stats.files_changed, stats.errors.len())
                        };
                        // Reload open editors that may have been rewritten on disk.
                        for editor in &mut self.app.editors {
                            if let Some(ref p) = editor.file_path {
                                if let Ok(content) = std::fs::read_to_string(p) {
                                    editor.buffer.rope = ropey::Rope::from_str(&content);
                                    editor.is_dirty = false;
                                    editor.diagnostics_dirty = true;
                                    editor.highlight_cache.clear();
                                    editor.highlight_cache_lang.clear();
                                    editor.fold_ranges.clear();
                                    editor.fold_ranges_computed = false;
                                }
                            }
                        }
                        // Re-run search to refresh the results list with new state.
                        self.app.last_search_trigger = Some(std::time::Instant::now());
                    }
                }
            });
        }

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
                        ui.painter().rect_filled(row_rect, CornerRadius::ZERO, tc.selection_bg);
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
                        ui.painter().rect_filled(row_rect, CornerRadius::ZERO, tc.selection_bg);
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
                            CornerRadius::same(2), tc.accent.linear_multiply(0.15),
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
