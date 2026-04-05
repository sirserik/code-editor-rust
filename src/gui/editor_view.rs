use super::*;

/// Compute bracket depth at each bracket in the file.
fn compute_bracket_depths(app: &crate::app::App, vis_lines: &[usize]) -> std::collections::HashMap<(usize, usize), usize> {
    let ed = &app.editors[app.active_editor];
    let max_line = vis_lines.last().copied().unwrap_or(0) + 1;
    let lc = ed.line_count().min(max_line);
    let mut depths = std::collections::HashMap::new();
    let mut depth: i32 = 0;
    let mut in_string = false;
    let mut string_char = '"';

    for li in 0..lc {
        let line = ed.buffer.get_line(li);
        let mut prev = '\0';
        for (ci, ch) in line.chars().enumerate() {
            if in_string {
                if ch == string_char && prev != '\\' { in_string = false; }
            } else {
                if (ch == '"' || ch == '\'' || ch == '`') && prev != '\\' {
                    in_string = true;
                    string_char = ch;
                } else {
                    match ch {
                        '(' | '[' | '{' => {
                            depths.insert((li, ci), depth as usize);
                            depth += 1;
                        }
                        ')' | ']' | '}' => {
                            depth = (depth - 1).max(0);
                            depths.insert((li, ci), depth as usize);
                        }
                        _ => {}
                    }
                }
            }
            prev = ch;
        }
        in_string = false;
    }
    depths
}

/// Find matching bracket position for bracket at (line, col)
fn find_matching_bracket(app: &crate::app::App, line: usize, col: usize) -> Option<(usize, usize)> {
    let ed = &app.editors[app.active_editor];
    let text = ed.buffer.get_line(line);
    let chars: Vec<char> = text.chars().collect();
    if col >= chars.len() { return None; }
    let ch = chars[col];
    let (target, forward) = match ch {
        '(' => (')', true), ')' => ('(', false),
        '[' => (']', true), ']' => ('[', false),
        '{' => ('}', true), '}' => ('{', false),
        _ => return None,
    };
    let mut depth = 0i32;
    let lc = ed.line_count();
    if forward {
        let mut l = line;
        let mut c = col;
        loop {
            let ln = ed.buffer.get_line(l);
            let lchars: Vec<char> = ln.chars().collect();
            while c < lchars.len() {
                if lchars[c] == ch { depth += 1; }
                else if lchars[c] == target { depth -= 1; if depth == 0 { return Some((l, c)); } }
                c += 1;
            }
            l += 1; c = 0;
            if l >= lc { break; }
        }
    } else {
        let mut l = line as isize;
        let mut c = col as isize;
        loop {
            let ln = ed.buffer.get_line(l as usize);
            let lchars: Vec<char> = ln.chars().collect();
            while c >= 0 {
                let cu = c as usize;
                if cu < lchars.len() {
                    if lchars[cu] == ch { depth += 1; }
                    else if lchars[cu] == target { depth -= 1; if depth == 0 { return Some((l as usize, cu)); } }
                }
                c -= 1;
            }
            l -= 1;
            if l < 0 { break; }
            let prev_len = ed.buffer.get_line(l as usize).chars().count();
            c = prev_len as isize - 1;
        }
    }
    None
}

impl CodeEditorApp {
    pub(super) fn render_editor(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().frame(egui::Frame::NONE.fill(self.tc.bg)).show(ctx, |ui| {
            // Welcome screen
            let has_project = self.app.file_tree.root_path.is_some();
            let ed_empty = self.app.editors.len() == 1
                && self.app.editors[0].file_path.is_none()
                && self.app.editors[0].line_count() <= 1;
            if !has_project && ed_empty {
                self.render_welcome(ui);
                return;
            }

            // Breadcrumbs bar
            if self.app.show_breadcrumbs {
                self.render_breadcrumbs(ui);
            }

            let dark = self.app.settings.theme != Theme::Light;
            let fs = self.app.settings.font_size;
            let font = mono_sized(fs);
            let sfont = small_sized(fs);
            let cw = ui.fonts(|f| f.glyph_width(&font, ' '));
            let lh = ui.fonts(|f| f.row_height(&font)) + LINE_SPACING;
            let show_ln = self.app.settings.show_line_numbers;

            // Recompute fold ranges when file is opened or after edits
            {
                let ed = &mut self.app.editors[self.app.active_editor];
                if ed.fold_ranges.is_empty() || ed.is_dirty {
                    ed.compute_fold_ranges();
                }
            }

            // Recompute diagnostics when content changes
            {
                let ed = &mut self.app.editors[self.app.active_editor];
                if ed.diagnostics_dirty {
                    let lang_for_diag = ed.file_path.as_ref().map(|p| syntax::detect_language(p).to_string()).unwrap_or("text".into());
                    let content = ed.buffer.text();
                    ed.diagnostics = syntax::check_syntax(&content, &lang_for_diag);
                    ed.diagnostics_dirty = false;
                }
            }

            let ed = &self.app.editors[self.app.active_editor];
            let lc = ed.line_count();
            let lang = ed.file_path.as_ref().map(|p| syntax::detect_language(p).to_string()).unwrap_or("text".into());
            let gutter_digits = format!("{}", lc).len().max(3);
            let fold_w = cw * 1.8;
            let gw = if show_ln { cw * (gutter_digits as f32 + 1.5) + fold_w } else { fold_w };

            let bracket_match = find_matching_bracket(&self.app, ed.cursor.line, ed.cursor.col);

            // Autocomplete key handling
            if self.app.show_autocomplete && self.app.focus == Focus::Editor {
                let ac_up = ctx.input(|i| i.key_pressed(egui::Key::ArrowUp));
                let ac_down = ctx.input(|i| i.key_pressed(egui::Key::ArrowDown));
                let ac_accept = ctx.input(|i| i.key_pressed(egui::Key::Tab))
                    || ctx.input(|i| i.key_pressed(egui::Key::Enter));
                let ac_dismiss = ctx.input(|i| i.key_pressed(egui::Key::Escape));
                if ac_up && self.app.autocomplete_selected > 0 {
                    self.app.autocomplete_selected -= 1;
                }
                if ac_down && self.app.autocomplete_selected + 1 < self.app.autocomplete_suggestions.len() {
                    self.app.autocomplete_selected += 1;
                }
                if ac_accept {
                    self.app.accept_autocomplete();
                }
                if ac_dismiss || ac_accept {
                    self.app.show_autocomplete = false;
                }
                if ac_up || ac_down || ac_accept || ac_dismiss {
                    // skip normal key processing
                }
            }

            // Input handling
            if self.app.focus == Focus::Editor {
                let events = ctx.input(|i| i.events.clone());
                for event in &events {
                    match event {
                        egui::Event::Text(text) => {
                            let ed = &mut self.app.editors[self.app.active_editor];
                            for c in text.chars() {
                                ed.selection = None;
                                ed.insert_char(c);
                                if let Some(cc) = match c {
                                    '(' => Some(')'), '[' => Some(']'), '{' => Some('}'),
                                    '"' => Some('"'), '\'' => Some('\''),
                                    _ => None
                                } {
                                    let (l, co) = (ed.cursor.line, ed.cursor.col);
                                    ed.buffer.insert_char(l, co, cc);
                                }
                            }
                            ed.scroll_into_view();
                            self.app.trigger_autocomplete();
                        }
                        egui::Event::Key { key, pressed: true, modifiers, .. } => {
                            if self.app.show_autocomplete && matches!(key,
                                egui::Key::ArrowUp | egui::Key::ArrowDown |
                                egui::Key::Tab | egui::Key::Enter | egui::Key::Escape
                            ) {
                                continue;
                            }
                            let ed = &mut self.app.editors[self.app.active_editor];
                            match key {
                                egui::Key::ArrowUp if modifiers.alt => ed.move_line_up(),
                                egui::Key::ArrowDown if modifiers.alt => ed.move_line_down(),
                                egui::Key::ArrowUp => { ed.move_up(); self.app.show_autocomplete = false; },
                                egui::Key::ArrowDown => { ed.move_down(); self.app.show_autocomplete = false; },
                                egui::Key::ArrowLeft if modifiers.alt => ed.move_word_left(),
                                egui::Key::ArrowRight if modifiers.alt => ed.move_word_right(),
                                egui::Key::ArrowLeft => { ed.move_left(); self.app.show_autocomplete = false; },
                                egui::Key::ArrowRight => { ed.move_right(); self.app.show_autocomplete = false; },
                                egui::Key::Home => ed.move_home(),
                                egui::Key::End => ed.move_end(),
                                egui::Key::Backspace => { ed.delete_back(); self.app.show_autocomplete = false; },
                                egui::Key::Delete => ed.delete_forward(),
                                egui::Key::Enter => ed.insert_newline(),
                                egui::Key::Tab if modifiers.shift => ed.outdent(),
                                egui::Key::Tab => ed.insert_tab(),
                                egui::Key::D if modifiers.command && modifiers.shift => ed.duplicate_line(),
                                egui::Key::D if modifiers.command => ed.select_next_occurrence(),
                                egui::Key::K if modifiers.command => ed.delete_line(),
                                egui::Key::Slash if modifiers.command => ed.toggle_comment(),
                                egui::Key::PageUp => ed.page_up(),
                                egui::Key::PageDown => ed.page_down(),
                                egui::Key::A if modifiers.command => ed.select_all(),
                                _ => {}
                            }
                            ed.scroll_into_view();
                        }
                        _ => {}
                    }
                }
            }

            // Paint editor
            let avail = ui.available_size();
            let (rect, response) = ui.allocate_exact_size(avail, egui::Sense::click_and_drag());

            // Click handling
            let minimap_w_check = if self.app.show_minimap { 80.0 } else { 0.0 };
            if response.clicked() {
                self.app.focus = Focus::Editor;
                if let Some(pos) = response.interact_pointer_pos() {
                    if !(self.app.show_minimap && pos.x > rect.max.x - minimap_w_check) {
                        let rel = pos - rect.min;
                        let row_idx = (rel.y / lh) as usize;
                        let ed = &mut self.app.editors[self.app.active_editor];
                        let vis_lines = ed.visible_lines(ed.scroll_offset, ed.viewport_height + 2);
                        let actual_line = vis_lines.get(row_idx).copied().unwrap_or(lc.saturating_sub(1)).min(lc.saturating_sub(1));

                        if rel.x < gw - fold_w + fold_w {
                            if ed.fold_ranges.contains_key(&actual_line) {
                                ed.toggle_fold(actual_line);
                            } else {
                                ed.cursor.line = actual_line;
                                ed.cursor.col = 0;
                            }
                        } else {
                            let cc = ((rel.x - gw).max(0.0) / cw) as usize;
                            ed.cursor.line = actual_line;
                            ed.cursor.col = cc.min(ed.buffer.line_len(actual_line));
                        }
                        ed.selection = None;
                        ed.extra_cursors.clear();
                    }
                }
            }

            // Scroll
            let cmd_held = ui.input(|i| i.modifiers.command);
            let sd = ui.input(|i| i.smooth_scroll_delta.y);
            if sd != 0.0 {
                if cmd_held {
                    let delta = if sd > 0.0 { 1.0 } else { -1.0 };
                    self.app.settings.font_size = (self.app.settings.font_size + delta).clamp(8.0, 32.0);
                    self.app.settings.save();
                } else {
                    let ed = &mut self.app.editors[self.app.active_editor];
                    let lines = (-sd / lh) as isize;
                    if lines > 0 { ed.scroll_offset = (ed.scroll_offset + lines as usize).min(lc.saturating_sub(1)); }
                    else if lines < 0 { ed.scroll_offset = ed.scroll_offset.saturating_sub((-lines) as usize); }
                }
            }

            let painter = ui.painter_at(rect);
            let vis = (rect.height() / lh) as usize;
            self.app.editors[self.app.active_editor].viewport_height = vis.max(1);
            let ed = &self.app.editors[self.app.active_editor];
            let so = ed.scroll_offset;
            let text_y_offset = LINE_SPACING / 2.0;

            let vis_lines = ed.visible_lines(so, vis + 2);
            let bracket_depths = compute_bracket_depths(&self.app, &vis_lines);

            // Gutter separator
            if show_ln {
                painter.line_segment(
                    [Pos2::new(rect.min.x + gw - fold_w - 2.0, rect.min.y), Pos2::new(rect.min.x + gw - fold_w - 2.0, rect.max.y)],
                    Stroke::new(1.0, self.tc.border),
                );
            }

            for (row, &li) in vis_lines.iter().enumerate() {
                let y = rect.min.y + row as f32 * lh;
                if y > rect.max.y { break; }

                // Current line highlight
                if li == ed.cursor.line {
                    painter.rect_filled(
                        Rect::from_min_size(Pos2::new(rect.min.x, y), Vec2::new(rect.width(), lh)),
                        Rounding::ZERO, self.tc.current_line_bg,
                    );
                }

                // Git diff gutter
                if let Some(diff_status) = ed.line_diff.get(&li) {
                    let diff_color = match diff_status {
                        crate::editor::LineDiffStatus::Added => self.tc.green,
                        crate::editor::LineDiffStatus::Modified => Color32::from_rgb(70, 140, 220),
                    };
                    painter.rect_filled(
                        Rect::from_min_size(Pos2::new(rect.min.x + 1.0, y), Vec2::new(3.0, lh)),
                        Rounding::ZERO, diff_color,
                    );
                }

                // Line numbers
                if show_ln {
                    let nc = if li == ed.cursor.line { self.tc.fg } else { self.tc.gutter_fg };
                    painter.text(
                        Pos2::new(rect.min.x + gw - fold_w - cw * 0.8, y + text_y_offset),
                        egui::Align2::RIGHT_TOP,
                        format!("{}", li + 1),
                        sfont.clone(), nc,
                    );
                }

                // Fold indicators
                let fold_x = rect.min.x + gw - fold_w + 2.0;
                if let Some(&fold_end) = ed.fold_ranges.get(&li) {
                    let is_folded = ed.folded.contains(&li);
                    let arrow = if is_folded { "▸" } else { "▾" };
                    let fold_color = if li == ed.cursor.line { self.tc.fg_dim } else { self.tc.fold_fg };
                    painter.text(
                        Pos2::new(fold_x, y + text_y_offset),
                        egui::Align2::LEFT_TOP, arrow, sfont.clone(), fold_color,
                    );
                    if is_folded {
                        let line = ed.buffer.get_line(li);
                        let line_end_x = rect.min.x + gw + line.chars().count() as f32 * cw + cw;
                        let folded_count = fold_end - li;
                        painter.rect_filled(
                            Rect::from_min_size(Pos2::new(line_end_x, y + 2.0), Vec2::new(cw * (folded_count.to_string().len() as f32 + 4.0), lh - 4.0)),
                            Rounding::same(3), if dark { Color32::from_rgb(40, 44, 65) } else { Color32::from_rgb(228, 228, 228) },
                        );
                        painter.text(
                            Pos2::new(line_end_x + cw * 0.5, y + text_y_offset),
                            egui::Align2::LEFT_TOP,
                            format!("⋯ {} lines", folded_count),
                            FontId::monospace((fs - 2.5).max(8.0)),
                            self.tc.fg_dim,
                        );
                    }
                }

                let line = ed.buffer.get_line(li);
                let hls = syntax::highlight_line(&line, &lang);
                let chars: Vec<char> = line.chars().collect();
                let xs = rect.min.x + gw;

                // Find match highlights
                for m in &self.app.find_matches {
                    if m.line == li {
                        let mx = xs + m.col as f32 * cw;
                        let mw = m.length as f32 * cw;
                        painter.rect_filled(
                            Rect::from_min_size(Pos2::new(mx - 1.0, y), Vec2::new(mw + 2.0, lh)),
                            Rounding::same(2),
                            if li == ed.cursor.line && m.col == ed.cursor.col {
                                self.tc.accent.linear_multiply(0.35)
                            } else {
                                self.tc.accent.linear_multiply(0.15)
                            },
                        );
                    }
                }

                // Bracket match highlight
                if let Some((ml, mc)) = bracket_match {
                    if li == ml && mc < chars.len() {
                        painter.rect_filled(
                            Rect::from_min_size(Pos2::new(xs + mc as f32 * cw - 1.0, y), Vec2::new(cw + 2.0, lh)),
                            Rounding::same(2), self.tc.bracket_match_bg,
                        );
                    }
                }
                if li == ed.cursor.line && ed.cursor.col < chars.len() {
                    let ch = chars[ed.cursor.col];
                    if "()[]{}".contains(ch) && bracket_match.is_some() {
                        painter.rect_filled(
                            Rect::from_min_size(Pos2::new(xs + ed.cursor.col as f32 * cw - 1.0, y), Vec2::new(cw + 2.0, lh)),
                            Rounding::same(2), self.tc.bracket_match_bg,
                        );
                    }
                }

                // Indent guides
                if !chars.is_empty() {
                    let indent_spaces = chars.iter().take_while(|c| **c == ' ').count();
                    for g in (4..indent_spaces + 1).step_by(4) {
                        let gx = xs + g as f32 * cw;
                        painter.line_segment(
                            [Pos2::new(gx, y), Pos2::new(gx, y + lh)],
                            Stroke::new(1.0, if dark { Color32::from_rgb(57, 57, 57) } else { Color32::from_rgb(228, 228, 228) }),
                        );
                    }
                }

                // Render text with syntax highlighting + rainbow brackets
                self.render_line_text(&painter, &chars, &hls, &bracket_depths, li, xs, y + text_y_offset, &font, dark);

                // Error underlines
                let error_color = Color32::from_rgb(247, 118, 142);
                for diag in &ed.diagnostics {
                    if diag.line == li {
                        let err_x_start = xs + diag.col as f32 * cw;
                        let err_len = if diag.length > 0 { diag.length } else { 1 };
                        let err_x_end = xs + (diag.col + err_len) as f32 * cw;
                        let wave_y = y + lh - 2.0;

                        let mut points = Vec::new();
                        let mut wx = err_x_start;
                        let mut wave_up = true;
                        while wx < err_x_end {
                            let wy = if wave_up { wave_y - 1.5 } else { wave_y + 1.5 };
                            points.push(Pos2::new(wx, wy));
                            wx += 2.0;
                            wave_up = !wave_up;
                        }
                        if points.len() >= 2 {
                            for pair in points.windows(2) {
                                painter.line_segment([pair[0], pair[1]], Stroke::new(1.2, error_color));
                            }
                        }

                        if show_ln {
                            painter.circle_filled(
                                Pos2::new(rect.min.x + 6.0, y + lh / 2.0), 3.0, error_color,
                            );
                        }
                    }
                }
            }

            let mut minimap_new_scroll: Option<usize> = None;

            // Cursor
            let cursor_row = vis_lines.iter().position(|&l| l == ed.cursor.line);
            if self.app.focus == Focus::Editor {
                if let Some(row) = cursor_row {
                    let cy = rect.min.y + row as f32 * lh;
                    let cx = rect.min.x + gw + ed.cursor.col as f32 * cw;
                    let blink = (ui.input(|i| i.time) * 2.0) as u32 % 2 == 0;
                    if blink && cy < rect.max.y {
                        painter.rect_filled(
                            Rect::from_min_size(Pos2::new(cx, cy), Vec2::new(2.0, lh)),
                            Rounding::ZERO, self.tc.cursor_color,
                        );
                    }
                }
            }

            // Minimap
            if self.app.show_minimap && lc > 1 {
                self.render_minimap(&painter, rect, dark, lc, vis, so, ed, &mut minimap_new_scroll, ctx);
            } else if lc > vis {
                // Simple scrollbar
                let sb_h = (vis as f32 / lc as f32 * rect.height()).max(20.0);
                let sb_y = rect.min.y + (so as f32 / lc as f32 * rect.height());
                painter.rect_filled(
                    Rect::from_min_size(Pos2::new(rect.max.x - 6.0, sb_y), Vec2::new(4.0, sb_h)),
                    Rounding::same(2), if dark { Color32::from_rgba_premultiplied(122, 162, 247, 60) } else { Color32::from_rgba_premultiplied(0, 0, 0, 40) },
                );
            }

            // Autocomplete popup
            if self.app.show_autocomplete && self.app.focus == Focus::Editor {
                self.render_autocomplete(&painter, rect, cursor_row, gw, cw, lh, dark, ed);
            }

            // Extra cursors
            if self.app.focus == Focus::Editor {
                let blink = (ui.input(|i| i.time) * 2.0) as u32 % 2 == 0;
                if blink {
                    for ec in &ed.extra_cursors {
                        if let Some(row) = vis_lines.iter().position(|&l| l == ec.line) {
                            let ecy = rect.min.y + row as f32 * lh;
                            let ecx = rect.min.x + gw + ec.col as f32 * cw;
                            if ecy < rect.max.y {
                                painter.rect_filled(
                                    Rect::from_min_size(Pos2::new(ecx, ecy), Vec2::new(2.0, lh)),
                                    Rounding::ZERO, self.tc.cursor_color,
                                );
                            }
                        }
                    }
                }
            }

            if let Some(new_scroll) = minimap_new_scroll {
                self.app.editors[self.app.active_editor].scroll_offset = new_scroll;
            }
        });
    }

    fn render_line_text(
        &self,
        painter: &egui::Painter,
        chars: &[char],
        hls: &[syntax::HighlightSpan],
        bracket_depths: &std::collections::HashMap<(usize, usize), usize>,
        li: usize,
        xs: f32,
        y: f32,
        font: &FontId,
        dark: bool,
    ) {
        let cw = painter.fonts(|f| f.glyph_width(font, ' '));

        if hls.is_empty() {
            let mut cx = xs;
            for (ci, &ch) in chars.iter().enumerate() {
                let color = if "()[]{}".contains(ch) {
                    if let Some(&depth) = bracket_depths.get(&(li, ci)) {
                        self.tc.bracket_colors[depth % self.tc.bracket_colors.len()]
                    } else { self.tc.fg }
                } else { self.tc.fg };
                painter.text(Pos2::new(cx, y), egui::Align2::LEFT_TOP, String::from(ch), font.clone(), color);
                cx += cw;
            }
        } else {
            let mut pos = 0;
            for hl in hls {
                if pos < hl.start && pos < chars.len() {
                    for ci in pos..hl.start.min(chars.len()) {
                        let ch = chars[ci];
                        let color = if "()[]{}".contains(ch) {
                            if let Some(&depth) = bracket_depths.get(&(li, ci)) {
                                self.tc.bracket_colors[depth % self.tc.bracket_colors.len()]
                            } else { self.tc.fg }
                        } else { self.tc.fg };
                        painter.text(Pos2::new(xs + ci as f32 * cw, y), egui::Align2::LEFT_TOP, String::from(ch), font.clone(), color);
                    }
                }
                if hl.start < chars.len() {
                    let end = hl.end.min(chars.len());
                    for ci in hl.start..end {
                        let ch = chars[ci];
                        let color = if "()[]{}".contains(ch) {
                            if let Some(&depth) = bracket_depths.get(&(li, ci)) {
                                self.tc.bracket_colors[depth % self.tc.bracket_colors.len()]
                            } else { hl.kind.color(dark) }
                        } else { hl.kind.color(dark) };
                        painter.text(Pos2::new(xs + ci as f32 * cw, y), egui::Align2::LEFT_TOP, String::from(ch), font.clone(), color);
                    }
                }
                pos = hl.end;
            }
            if pos < chars.len() {
                for ci in pos..chars.len() {
                    let ch = chars[ci];
                    let color = if "()[]{}".contains(ch) {
                        if let Some(&depth) = bracket_depths.get(&(li, ci)) {
                            self.tc.bracket_colors[depth % self.tc.bracket_colors.len()]
                        } else { self.tc.fg }
                    } else { self.tc.fg };
                    painter.text(Pos2::new(xs + ci as f32 * cw, y), egui::Align2::LEFT_TOP, String::from(ch), font.clone(), color);
                }
            }
        }
    }

    fn render_welcome(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() * 0.2);
            ui.label(RichText::new("Code Editor").font(FontId::monospace(28.0)).color(self.tc.accent));
            ui.add_space(8.0);
            ui.label(RichText::new("Lightweight • Fast • Native").font(FontId::monospace(14.0)).color(self.tc.gutter_fg));
            ui.add_space(32.0);

            let btn_width = 220.0;
            let btn_height = 36.0;
            let btn_bg = self.tc.sidebar_bg;
            let btn_text = self.tc.accent;
            if ui.add_sized([btn_width, btn_height],
                egui::Button::new(RichText::new("  Open Folder…  ").font(FontId::monospace(14.0)).color(btn_text))
                    .fill(btn_bg).rounding(Rounding::same(6))
                    .stroke(Stroke::new(1.0, self.tc.border))
            ).clicked() {
                self.app.pending_action = Some(PaletteAction::OpenFolder);
            }
            ui.add_space(8.0);
            if ui.add_sized([btn_width, btn_height],
                egui::Button::new(RichText::new("  Open File…  ").font(FontId::monospace(14.0)).color(btn_text))
                    .fill(btn_bg).rounding(Rounding::same(6))
                    .stroke(Stroke::new(1.0, self.tc.border))
            ).clicked() {
                self.app.pending_action = Some(PaletteAction::OpenFile);
            }

            ui.add_space(40.0);
            ui.label(RichText::new("Keyboard Shortcuts").font(FontId::monospace(13.0)).color(self.tc.fg_dim));
            ui.add_space(8.0);
            for (keys, desc) in [
                ("⌘+Shift+P", "Command Palette"),
                ("⌘+P", "Quick Open File"),
                ("⌘+O", "Open File"),
                ("⌘+S", "Save"),
                ("⌘+B", "Toggle Sidebar"),
            ] {
                ui.horizontal(|ui| {
                    let total = 260.0;
                    let offset = (ui.available_width() - total) / 2.0;
                    ui.add_space(offset);
                    ui.label(RichText::new(format!("{:<16}", keys)).font(small()).color(self.tc.accent));
                    ui.label(RichText::new(desc).font(small()).color(self.tc.fg_dim));
                });
            }
        });
    }

    fn render_breadcrumbs(&self, ui: &mut egui::Ui) {
        let ed = &self.app.editors[self.app.active_editor];
        if let Some(ref fp) = ed.file_path {
            let root = self.app.file_tree.root_path.as_deref().unwrap_or("");
            let rel = fp.strip_prefix(root).unwrap_or(fp).trim_start_matches('/');
            let parts: Vec<&str> = rel.split('/').collect();
            let dark = self.app.settings.theme != Theme::Light;
            let bc_bg = if dark { Color32::from_rgb(37, 37, 37) } else { Color32::from_rgb(245, 245, 245) };
            let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 22.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, Rounding::ZERO, bc_bg);
            let mut x = rect.min.x + 12.0;
            for (i, part) in parts.iter().enumerate() {
                if i > 0 {
                    let sep_r = ui.painter().text(
                        Pos2::new(x, rect.min.y + 4.0), egui::Align2::LEFT_TOP,
                        " › ", FontId::monospace(11.0), self.tc.fg_dim,
                    );
                    x += sep_r.width();
                }
                let is_last = i == parts.len() - 1;
                let color = if is_last { self.tc.fg } else { self.tc.fg_dim };
                let tr = ui.painter().text(
                    Pos2::new(x, rect.min.y + 4.0), egui::Align2::LEFT_TOP,
                    *part, FontId::monospace(11.0), color,
                );
                x += tr.width();
            }
            ui.painter().line_segment(
                [Pos2::new(rect.min.x, rect.max.y), Pos2::new(rect.max.x, rect.max.y)],
                Stroke::new(1.0, self.tc.border),
            );
        }
    }

    fn render_minimap(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        dark: bool,
        lc: usize,
        vis: usize,
        so: usize,
        ed: &crate::editor::Editor,
        minimap_new_scroll: &mut Option<usize>,
        ctx: &egui::Context,
    ) {
        let minimap_w = 80.0;
        let minimap_x = rect.max.x - minimap_w;
        let minimap_rect = Rect::from_min_size(Pos2::new(minimap_x, rect.min.y), Vec2::new(minimap_w, rect.height()));
        let minimap_bg = if dark { Color32::from_rgb(30, 30, 30) } else { Color32::from_rgb(240, 240, 240) };
        painter.rect_filled(minimap_rect, Rounding::ZERO, minimap_bg);
        painter.line_segment(
            [Pos2::new(minimap_x, rect.min.y), Pos2::new(minimap_x, rect.max.y)],
            Stroke::new(1.0, self.tc.border),
        );

        let mini_line_h: f32 = 2.0;
        let total_mini_h = mini_line_h * lc as f32;
        let minimap_scroll_offset: f32 = if total_mini_h > rect.height() {
            let scroll_ratio = so as f32 / (lc as f32 - vis as f32).max(1.0);
            scroll_ratio * (total_mini_h - rect.height())
        } else {
            0.0
        };

        let step = if lc > 3000 { (lc / 1500).max(1) } else { 1 };
        for li in (0..lc).step_by(step) {
            let my = rect.min.y + li as f32 * mini_line_h - minimap_scroll_offset;
            if my < rect.min.y - 2.0 { continue; }
            if my > rect.max.y { break; }

            let line = ed.buffer.get_line(li);
            let indent = line.chars().take_while(|c| c.is_whitespace()).count();
            let content_len = line.trim().len().min(60);
            if content_len == 0 { continue; }

            let mx = minimap_x + 4.0 + (indent as f32 * 0.6).min(20.0);
            let mw = (content_len as f32 * 0.8).min(minimap_w - 8.0);

            let line_color = if let Some(diff_st) = ed.line_diff.get(&li) {
                match diff_st {
                    crate::editor::LineDiffStatus::Added => if dark {
                        Color32::from_rgba_premultiplied(106, 171, 115, 100)
                    } else {
                        Color32::from_rgba_premultiplied(10, 132, 57, 60)
                    },
                    crate::editor::LineDiffStatus::Modified => if dark {
                        Color32::from_rgba_premultiplied(70, 140, 220, 100)
                    } else {
                        Color32::from_rgba_premultiplied(55, 125, 207, 60)
                    },
                }
            } else if dark {
                Color32::from_rgba_premultiplied(187, 187, 187, 50)
            } else {
                Color32::from_rgba_premultiplied(0, 0, 0, 35)
            };

            painter.rect_filled(
                Rect::from_min_size(Pos2::new(mx, my), Vec2::new(mw, mini_line_h.max(1.0))),
                Rounding::ZERO, line_color,
            );
        }

        // Viewport indicator
        let vp_y = rect.min.y + so as f32 * mini_line_h - minimap_scroll_offset;
        let vp_h = (vis as f32 * mini_line_h).max(10.0);
        let vp_color = if dark {
            Color32::from_rgba_premultiplied(122, 162, 247, 30)
        } else {
            Color32::from_rgba_premultiplied(55, 125, 207, 25)
        };
        painter.rect_filled(
            Rect::from_min_size(Pos2::new(minimap_x, vp_y), Vec2::new(minimap_w, vp_h)),
            Rounding::ZERO, vp_color,
        );
        let vp_border = if dark {
            Color32::from_rgba_premultiplied(122, 162, 247, 70)
        } else {
            Color32::from_rgba_premultiplied(55, 125, 207, 50)
        };
        painter.line_segment(
            [Pos2::new(minimap_x, vp_y), Pos2::new(minimap_x + minimap_w, vp_y)],
            Stroke::new(1.0, vp_border),
        );
        painter.line_segment(
            [Pos2::new(minimap_x, vp_y + vp_h), Pos2::new(minimap_x + minimap_w, vp_y + vp_h)],
            Stroke::new(1.0, vp_border),
        );

        // Minimap click & drag
        if let Some(pointer_pos) = ctx.input(|i| i.pointer.hover_pos()) {
            if minimap_rect.contains(pointer_pos) {
                let clicking = ctx.input(|i| i.pointer.primary_down());
                if clicking {
                    let click_y = pointer_pos.y - rect.min.y + minimap_scroll_offset;
                    let target_line = (click_y / mini_line_h) as usize;
                    let target_scroll = target_line.saturating_sub(vis / 2).min(lc.saturating_sub(vis));
                    *minimap_new_scroll = Some(target_scroll);
                }
                if !clicking {
                    let hover_y = pointer_pos.y - rect.min.y + minimap_scroll_offset;
                    let hover_line = (hover_y / mini_line_h) as usize;
                    let hover_vp_y = rect.min.y + hover_line.saturating_sub(vis / 2) as f32 * mini_line_h - minimap_scroll_offset;
                    let hover_vp_h = vis as f32 * mini_line_h;
                    painter.rect_filled(
                        Rect::from_min_size(Pos2::new(minimap_x, hover_vp_y), Vec2::new(minimap_w, hover_vp_h)),
                        Rounding::ZERO,
                        if dark {
                            Color32::from_rgba_premultiplied(255, 255, 255, 10)
                        } else {
                            Color32::from_rgba_premultiplied(0, 0, 0, 8)
                        },
                    );
                }
            }
        }
    }

    fn render_autocomplete(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        cursor_row: Option<usize>,
        gw: f32,
        cw: f32,
        lh: f32,
        dark: bool,
        ed: &crate::editor::Editor,
    ) {
        if let Some(cursor_row) = cursor_row {
            let ac_x = rect.min.x + gw + ed.cursor.col as f32 * cw;
            let ac_y = rect.min.y + (cursor_row + 1) as f32 * lh;
            let ac_w = 220.0;
            let ac_item_h = 24.0;
            let ac_count = self.app.autocomplete_suggestions.len().min(8);
            let ac_h = ac_count as f32 * ac_item_h + 4.0;

            let popup_bg = if dark { Color32::from_rgb(43, 43, 43) } else { Color32::from_rgb(255, 255, 255) };
            let ac_rect = Rect::from_min_size(Pos2::new(ac_x, ac_y), Vec2::new(ac_w, ac_h));

            painter.rect_filled(
                ac_rect.translate(Vec2::new(2.0, 2.0)),
                Rounding::same(4), Color32::from_black_alpha(if dark { 80 } else { 30 }),
            );
            painter.rect_filled(ac_rect, Rounding::same(4), popup_bg);
            painter.rect_stroke(ac_rect, Rounding::same(4), Stroke::new(1.0, self.tc.border), egui::StrokeKind::Outside);

            for (i, suggestion) in self.app.autocomplete_suggestions.iter().enumerate().take(ac_count) {
                let item_y = ac_y + 2.0 + i as f32 * ac_item_h;
                let selected = i == self.app.autocomplete_selected;
                if selected {
                    painter.rect_filled(
                        Rect::from_min_size(Pos2::new(ac_x + 2.0, item_y), Vec2::new(ac_w - 4.0, ac_item_h)),
                        Rounding::same(3), self.tc.selection_bg,
                    );
                }
                painter.text(
                    Pos2::new(ac_x + 8.0, item_y + 4.0), egui::Align2::LEFT_TOP,
                    suggestion, FontId::monospace(12.0),
                    if selected { self.tc.fg } else { self.tc.fg_dim },
                );
            }
        }
    }
}
