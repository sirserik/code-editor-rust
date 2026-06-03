use super::*;
use std::fmt::Write;

/// Compute bracket depth at each bracket in the file.
fn compute_bracket_depths(app: &crate::app::App, vis_lines: &[usize]) -> std::collections::HashMap<(usize, usize), usize> {
    let ed = &app.editors[app.active_editor];
    let mut depths = std::collections::HashMap::new();
    if vis_lines.is_empty() { return depths; }
    let mut depth: i32 = 0;
    let mut in_string = false;
    let mut string_char = '"';

    // Only scan visible lines (not from line 0!)
    for &li in vis_lines {
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

use crate::wrap::compute_wrap_starts;

/// Number of visual rows a logical line occupies under the given wrap width.
/// Always ≥ 1. Reuses `scratch` to avoid a persistent allocation.
fn seg_count(ed: &crate::editor::Editor, line: usize, wrap_cols: usize, scratch: &mut Vec<usize>) -> usize {
    if wrap_cols == usize::MAX {
        return 1;
    }
    let chars: Vec<char> = ed.buffer.get_line(line).chars().take(MAX_LINE_LEN).collect();
    compute_wrap_starts(&chars, wrap_cols, scratch);
    scratch.len()
}

/// Screen position of column `col` on a wrapped line whose segment starts are
/// `starts`, given the line's top-left origin (`xs`, `base_y`). Returns the
/// pixel `(x, y)` of that column, picking the correct wrapped row.
fn seg_xy(starts: &[usize], col: usize, xs: f32, base_y: f32, cw: f32, lh: f32) -> (f32, f32) {
    let mut sidx = 0;
    while sidx + 1 < starts.len() && col >= starts[sidx + 1] {
        sidx += 1;
    }
    (xs + (col - starts[sidx]) as f32 * cw, base_y + sidx as f32 * lh)
}

/// Map a click at visual `row_idx` (counted from `start_line`'s first row) and
/// horizontal pixel `rel_x` back to a logical `(line, col)`, accounting for soft
/// wrap. `scratch` is reused for wrap computation to avoid per-call allocation.
fn point_to_line_col(
    ed: &crate::editor::Editor,
    start_line: usize,
    row_idx: usize,
    rel_x: f32,
    gw: f32,
    cw: f32,
    wrap_cols: usize,
    lc: usize,
    scratch: &mut Vec<usize>,
) -> (usize, usize) {
    let within = ((rel_x - gw).max(0.0) / cw) as usize;
    let mut acc = 0usize;
    let mut line = start_line.min(lc.saturating_sub(1));
    loop {
        let text = ed.buffer.get_line(line);
        let chars: Vec<char> = text.chars().take(MAX_LINE_LEN).collect();
        compute_wrap_starts(&chars, wrap_cols, scratch);
        let n_seg = scratch.len();
        if row_idx < acc + n_seg {
            let sidx = row_idx - acc;
            let seg_start = scratch[sidx];
            let seg_end = if sidx + 1 < n_seg { scratch[sidx + 1] } else { chars.len() };
            return (line, (seg_start + within).min(seg_end));
        }
        acc += n_seg;
        if line + 1 >= lc {
            return (line, chars.len());
        }
        line += 1;
    }
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

/// Coarse-grained "just now / 3m ago / yesterday / 2d ago / 3w ago" formatter
/// for the welcome-screen Recent Projects list.
fn humanize_time(timestamp: u64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let diff = now.saturating_sub(timestamp);
    match diff {
        0..=59 => "just now".to_string(),
        60..=3599 => format!("{}m ago", diff / 60),
        3600..=86399 => format!("{}h ago", diff / 3600),
        86400..=172799 => "yesterday".to_string(),
        _ => {
            let days = diff / 86400;
            if days < 7 {
                format!("{} days ago", days)
            } else if days < 30 {
                format!("{}w ago", days / 7)
            } else if days < 365 {
                format!("{}mo ago", days / 30)
            } else {
                format!("{}y ago", days / 365)
            }
        }
    }
}

impl CodeEditorApp {
    pub(super) fn render_editor(&mut self, ctx: &egui::Context) {
        // Split pane — render second editor on the right
        if self.app.split_active {
            let split_idx = self.app.split_editor.min(self.app.editors.len().saturating_sub(1));
            let split_name = self.app.editors[split_idx].file_name();
            let tc = self.tc;
            egui::SidePanel::right("split_editor")
                .default_width(ctx.screen_rect().width() * 0.4)
                .min_width(200.0)
                .frame(egui::Frame::NONE.fill(tc.bg).inner_margin(0.0))
                .show(ctx, |ui| {
                    // Split border
                    let r = ui.max_rect();
                    ui.painter().line_segment(
                        [Pos2::new(r.min.x, r.min.y), Pos2::new(r.min.x, r.max.y)],
                        Stroke::new(1.0, tc.border),
                    );
                    // Header with filename
                    let (hdr, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 26.0), egui::Sense::hover());
                    let hdr_bg = tc.tab_bar_bg;
                    ui.painter().rect_filled(hdr, CornerRadius::ZERO, hdr_bg);
                    ui.painter().text(
                        Pos2::new(hdr.min.x + 12.0, hdr.min.y + 6.0),
                        egui::Align2::LEFT_TOP, &split_name,
                        FontId::monospace(11.5), tc.fg_dim,
                    );
                    // Close split button
                    let close_rect = Rect::from_min_size(
                        Pos2::new(hdr.max.x - 28.0, hdr.min.y + 3.0),
                        Vec2::new(20.0, 20.0),
                    );
                    let close_resp = ui.allocate_rect(close_rect, egui::Sense::click());
                    ui.painter().text(close_rect.center(), egui::Align2::CENTER_CENTER,
                        "×", FontId::monospace(14.0), tc.fg_dim);
                    if close_resp.clicked() {
                        self.app.split_active = false;
                    }

                    // Render split editor content (read-only view)
                    let ed = &self.app.editors[split_idx];
                    let fs = self.app.settings.font_size;
                    let font = mono_sized(fs);
                    let cw = ui.fonts(|f| f.glyph_width(&font, ' '));
                    let lh = ui.fonts(|f| f.row_height(&font)) + LINE_SPACING;
                    let avail = ui.available_size();
                    let (rect, _) = ui.allocate_exact_size(avail, egui::Sense::click());
                    let painter = ui.painter_at(rect);
                    let vis = (rect.height() / lh) as usize;
                    let vis_lines = ed.visible_lines(ed.scroll_offset as usize, vis + 2);
                    let gutter_digits = format!("{}", ed.line_count()).len().max(3);
                    let gw = cw * (gutter_digits as f32 + 2.0);
                    let dark = self.app.settings.theme.resolved() != Theme::Light;

                    for (row, &li) in vis_lines.iter().enumerate() {
                        let y = rect.min.y + row as f32 * lh;
                        if y > rect.max.y { break; }
                        // Line number
                        let nc = tc.gutter_fg;
                        painter.text(
                            Pos2::new(rect.min.x + gw - cw * 0.8, y + LINE_SPACING / 2.0),
                            egui::Align2::RIGHT_TOP,
                            format!("{}", li + 1),
                            small_sized(fs), nc,
                        );
                        // Text
                        let line = ed.buffer.get_line(li);
                        let hls = if li < ed.highlight_cache.len() { &ed.highlight_cache[li] } else { &[] as &[syntax::HighlightSpan] };
                        let chars: Vec<char> = line.chars().take(MAX_LINE_LEN).collect();
                        let xs = rect.min.x + gw;
                        let seg_end = chars.len();
                        self.render_line_text(&painter, &chars, hls, &std::collections::HashMap::new(), li, 0, seg_end, xs, y + LINE_SPACING / 2.0, &font, dark);
                    }
                });
        }

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

            let dark = self.app.settings.theme.resolved() != Theme::Light;
            let fs = self.app.settings.font_size;
            let font = mono_sized(fs);
            let sfont = small_sized(fs);
            let cw = ui.fonts(|f| f.glyph_width(&font, ' '));
            let lh = ui.fonts(|f| f.row_height(&font)) + LINE_SPACING;
            let show_ln = self.app.settings.show_line_numbers;

            // Diagnostics (bracket/string checking) — deferred until 1s after the
            // last user input so it never runs on the frame a file is opened or
            // while the user is typing/clicking. Folds are no longer computed here;
            // they come from the async analysis worker below. `check_syntax` is
            // O(n) but only fires once the user has paused.
            {
                let idle_enough = self.last_input_time.elapsed().as_millis() > 1000;
                let ed = &mut self.app.editors[self.app.active_editor];
                if ed.diagnostics_dirty && idle_enough {
                    let content = ed.buffer.text();
                    let first_line = content.lines().next().unwrap_or("");
                    let lang_for_diag = ed.file_path.as_ref()
                        .map(|p| syntax::detect_language_with_first_line(p, first_line).to_string())
                        .unwrap_or("text".into());
                    ed.diagnostics = syntax::check_syntax(&content, &lang_for_diag);
                    ed.diagnostics_dirty = false;
                }
            }

            // Update syntax highlight cache ASYNCHRONOUSLY. Syntect is stateful
            // (multi-line strings etc.) so the whole buffer is parsed in one pass,
            // which can take 100s of ms on big files — too slow for the UI thread.
            // Instead a worker thread produces the cache and we install it when
            // ready. Until then the editor paints as plain text (no freeze on open
            // or while typing). Bursts are coalesced via the generation counter.
            {
                let dark = self.app.settings.theme.resolved() != Theme::Light;
                let ed = &mut self.app.editors[self.app.active_editor];
                let lc = ed.line_count();

                let first_load = ed.highlight_cache_lang.is_empty();
                let edited = ed.highlight_dirty_from.is_some();
                if first_load {
                    let first_line = ed.buffer.get_line(0);
                    let lang = ed.file_path.as_ref()
                        .map(|p| crate::syntax::detect_language_with_first_line(p, &first_line).to_string())
                        .unwrap_or("text".into());
                    ed.highlight_cache_lang = lang;
                }
                // Theme switches must rebuild the cache: syntect bakes absolute RGB
                // into each span, so stale colors would be painted on the new bg.
                // The length mismatch is the safety net for edits that change the
                // line count without invalidating (paste, undo/redo).
                let theme_changed = ed.highlight_cache_dark != Some(dark);
                let stale = first_load || edited || theme_changed || ed.highlight_cache.len() != lc;
                // Bump only when we currently believe we're up to date — otherwise a
                // persistent (level-triggered) mismatch would bump every frame and
                // every in-flight result would be discarded as stale forever.
                if stale && ed.highlight_applied_gen == ed.highlight_gen {
                    ed.highlight_gen += 1; // a newer cache is wanted
                    ed.highlight_cache_dark = Some(dark);
                    // Consume the dirty marker only now that we're acting on it, so
                    // edits during an in-flight pass aren't lost — they keep the
                    // marker set and trigger a fresh pass once this one settles.
                    ed.highlight_dirty_from = None;
                }

                // Install a finished worker result (newest gen only; drop stale).
                if let Some(rx) = &ed.highlight_rx {
                    if let Ok((g, cache, folds)) = rx.try_recv() {
                        if g == ed.highlight_gen {
                            ed.highlight_cache = cache;
                            if ed.highlight_cache.len() != lc {
                                ed.highlight_cache.resize(lc, Vec::new());
                            }
                            ed.fold_ranges = folds;
                            ed.fold_ranges_computed = true;
                            ed.highlight_applied_gen = g;
                        }
                        ed.highlight_rx = None;
                    }
                }

                // No worker running and the shown cache is out of date → spawn one.
                // It produces both the highlight cache and the fold ranges so neither
                // O(n) pass runs on the UI thread on open.
                if ed.highlight_rx.is_none() && ed.highlight_applied_gen != ed.highlight_gen {
                    let content = ed.buffer.text();
                    let lang = ed.highlight_cache_lang.clone();
                    let gen = ed.highlight_gen;
                    let (tx, rx) = std::sync::mpsc::channel();
                    std::thread::spawn(move || {
                        let cache = crate::syntect_engine::highlight_buffer(&content, &lang, dark)
                            .unwrap_or_else(|| {
                                // Fallback: per-line keyword highlighter for languages
                                // syntect doesn't bundle (or when its parser bails out).
                                content.lines()
                                    .map(|line| crate::syntax::highlight_line(line, &lang))
                                    .collect()
                            });
                        let folds = crate::editor::compute_folds(&content);
                        let _ = tx.send((gen, cache, folds));
                    });
                    ed.highlight_rx = Some(rx);
                }

                // Keep repainting while a highlight is in flight so the result is
                // polled and shown promptly (egui otherwise idles between events).
                if ed.highlight_rx.is_some() {
                    ctx.request_repaint();
                }
            }

            let ed = &self.app.editors[self.app.active_editor];
            let lc = ed.line_count();
            let gutter_digits = format!("{}", lc).len().max(3);
            let fold_w = cw * 1.8;
            let gw = if show_ln { cw * (gutter_digits as f32 + 1.5) + fold_w } else { fold_w };

            // Bracket match — only compute if cursor is on a bracket char (avoid O(n) scan)
            let bracket_match = {
                let line = ed.buffer.get_line(ed.cursor.line);
                let ch = line.chars().nth(ed.cursor.col);
                if ch.map(|c| "()[]{}".contains(c)).unwrap_or(false) {
                    find_matching_bracket(&self.app, ed.cursor.line, ed.cursor.col)
                } else {
                    None
                }
            };

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
                            ed.save_undo_snapshot();
                            // Auto-surround selection (Zed: wrap selection with brackets)
                            if ed.selection.is_some() {
                                let wrap_pair = match text.as_str() {
                                    "(" => Some(("(", ")")),
                                    "[" => Some(("[", "]")),
                                    "{" => Some(("{", "}")),
                                    "\"" => Some(("\"", "\"")),
                                    "'" => Some(("'", "'")),
                                    "`" => Some(("`", "`")),
                                    _ => None,
                                };
                                if let Some((open, close)) = wrap_pair {
                                    if let Some(sel) = ed.normalized_selection() {
                                        let selected = ed.buffer.get_range(sel.start_line, sel.start_col, sel.end_line, sel.end_col);
                                        ed.buffer.delete_range(sel.start_line, sel.start_col, sel.end_line, sel.end_col);
                                        let wrapped = format!("{}{}{}", open, selected, close);
                                        ed.buffer.insert_text(sel.start_line, sel.start_col, &wrapped);
                                        ed.cursor.line = sel.start_line;
                                        ed.cursor.col = sel.start_col + wrapped.len();
                                        ed.selection = None;
                                        ed.is_dirty = true;
                                        ed.diagnostics_dirty = true;
                                        let edit_line = ed.cursor.line;
                                        ed.scroll_into_view();
                                        ed.invalidate_highlights_from(edit_line);
                                        self.app.trigger_autocomplete();
                                        continue;
                                    }
                                }
                            }
                            // Typing replaces selection
                            ed.delete_selection_text();
                            let edit_line = ed.cursor.line;
                            for c in text.chars() {
                                // Zed skip-over: if next char is the closing bracket, just move past it
                                let line = ed.buffer.get_line(ed.cursor.line);
                                let next_char = line.chars().nth(ed.cursor.col);
                                let is_closing = matches!(c, ')' | ']' | '}' | '"' | '\'' | '`');
                                if is_closing && next_char == Some(c) {
                                    ed.cursor.col += 1; // skip over
                                    continue;
                                }
                                ed.insert_char(c);
                                // Auto-close brackets (Zed: BracketPair auto-close)
                                if let Some(cc) = match c {
                                    '(' => Some(')'), '[' => Some(']'), '{' => Some('}'),
                                    _ => None
                                } {
                                    let (l, co) = (ed.cursor.line, ed.cursor.col);
                                    ed.buffer.insert_char(l, co, cc);
                                }
                                // Auto-close quotes (Zed: only if count is even)
                                if matches!(c, '"' | '\'' | '`') {
                                    let line = ed.buffer.get_line(ed.cursor.line);
                                    let quote_count = line.chars().filter(|&ch| ch == c).count();
                                    // If odd count after insert, we just inserted opening — add closing
                                    if quote_count % 2 == 1 {
                                        let (l, co) = (ed.cursor.line, ed.cursor.col);
                                        ed.buffer.insert_char(l, co, c);
                                    }
                                }
                            }
                            ed.scroll_into_view();
                            ed.invalidate_highlights_from(edit_line);
                            self.app.trigger_autocomplete();
                        }
                        egui::Event::Key { key, pressed: true, modifiers, .. } => {
                            if self.app.show_autocomplete && matches!(key,
                                egui::Key::ArrowUp | egui::Key::ArrowDown |
                                egui::Key::Tab | egui::Key::Enter | egui::Key::Escape
                            ) {
                                continue;
                            }
                            let shift = modifiers.shift;
                            let ed = &mut self.app.editors[self.app.active_editor];
                            match key {
                                // Movement with Shift = extend selection
                                egui::Key::ArrowUp if modifiers.command => { ed.selection = None; ed.move_to_top(); },
                                egui::Key::ArrowDown if modifiers.command => { ed.selection = None; ed.move_to_bottom(); },
                                egui::Key::ArrowUp if modifiers.alt => ed.move_line_up(),
                                egui::Key::ArrowDown if modifiers.alt => ed.move_line_down(),
                                egui::Key::ArrowUp => {
                                    if shift { ed.ensure_selection_anchor(); }
                                    ed.move_up();
                                    if shift { ed.update_selection_end(); } else { ed.selection = None; }
                                    self.app.show_autocomplete = false;
                                },
                                egui::Key::ArrowDown => {
                                    if shift { ed.ensure_selection_anchor(); }
                                    ed.move_down();
                                    if shift { ed.update_selection_end(); } else { ed.selection = None; }
                                    self.app.show_autocomplete = false;
                                },
                                egui::Key::ArrowLeft if modifiers.alt && shift => {
                                    ed.ensure_selection_anchor();
                                    ed.move_word_left();
                                    ed.update_selection_end();
                                },
                                egui::Key::ArrowRight if modifiers.alt && shift => {
                                    ed.ensure_selection_anchor();
                                    ed.move_word_right();
                                    ed.update_selection_end();
                                },
                                egui::Key::ArrowLeft if modifiers.alt => { ed.selection = None; ed.move_word_left(); },
                                egui::Key::ArrowRight if modifiers.alt => { ed.selection = None; ed.move_word_right(); },
                                egui::Key::ArrowLeft if shift => {
                                    ed.ensure_selection_anchor();
                                    ed.move_left();
                                    ed.update_selection_end();
                                    self.app.show_autocomplete = false;
                                },
                                egui::Key::ArrowRight if shift => {
                                    ed.ensure_selection_anchor();
                                    ed.move_right();
                                    ed.update_selection_end();
                                    self.app.show_autocomplete = false;
                                },
                                egui::Key::ArrowLeft => { ed.selection = None; ed.move_left(); self.app.show_autocomplete = false; },
                                egui::Key::ArrowRight => { ed.selection = None; ed.move_right(); self.app.show_autocomplete = false; },
                                egui::Key::Home if shift => {
                                    ed.ensure_selection_anchor();
                                    ed.move_home();
                                    ed.update_selection_end();
                                },
                                egui::Key::End if shift => {
                                    ed.ensure_selection_anchor();
                                    ed.move_end();
                                    ed.update_selection_end();
                                },
                                egui::Key::Home => { ed.selection = None; ed.move_home(); },
                                egui::Key::End => { ed.selection = None; ed.move_end(); },
                                egui::Key::Backspace => {
                                    if ed.selection.is_some() {
                                        ed.save_undo_snapshot();
                                        ed.delete_selection_text();
                                    } else {
                                        ed.delete_back();
                                    }
                                    self.app.show_autocomplete = false;
                                },
                                egui::Key::Delete => {
                                    if ed.selection.is_some() {
                                        ed.save_undo_snapshot();
                                        ed.delete_selection_text();
                                    } else {
                                        ed.delete_forward();
                                    }
                                },
                                egui::Key::Enter => {
                                    ed.delete_selection_text();
                                    ed.insert_newline();
                                },
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

            // Click handling — single click, double-click, drag
            let minimap_w_check = if self.app.show_minimap { 80.0 } else { 0.0 };

            // Soft-wrap width in columns. The text column ends before the minimap
            // (or scrollbar) so wrapped lines never paint underneath it. Disabled
            // (usize::MAX) when Word Wrap is off, which collapses every line to a
            // single segment and reproduces the old one-line-per-row behaviour.
            let xs_for_wrap = rect.min.x + gw;
            let content_right = if self.app.show_minimap {
                rect.max.x - 80.0
            } else {
                rect.max.x - SCROLLBAR_WIDTH
            };
            let wrap_cols = if self.app.settings.word_wrap {
                (((content_right - xs_for_wrap - cw) / cw).floor() as i64)
                    .clamp(8, MAX_LINE_LEN as i64) as usize
            } else {
                usize::MAX
            };
            // Scratch buffer reused by click→(line,col) mapping (no per-event alloc).
            let mut click_scratch: Vec<usize> = Vec::new();

            // Sub-pixel-aware row mapping for click positions. The fractional
            // scroll is a fraction *through the top line's wrapped block*, so the
            // pixels already scrolled off the top span that line's visual rows.
            let ed_scroll = self.app.editors[self.app.active_editor].scroll_offset;
            let click_so = ed_scroll.floor().max(0.0) as usize;
            let click_top_segs = seg_count(
                &self.app.editors[self.app.active_editor], click_so, wrap_cols, &mut click_scratch,
            );
            let click_pixel_offset = (ed_scroll - click_so as f32) * click_top_segs as f32 * lh;
            let rel_to_row = |rel_y: f32| -> usize {
                ((rel_y + click_pixel_offset).max(0.0) / lh) as usize
            };

            // Double-click to select word
            if response.double_clicked() {
                self.app.focus = Focus::Editor;
                if let Some(pos) = response.interact_pointer_pos() {
                    if !(self.app.show_minimap && pos.x > rect.max.x - minimap_w_check) {
                        let rel = pos - rect.min;
                        let row_idx = rel_to_row(rel.y);
                        let (actual_line, col) = point_to_line_col(
                            &self.app.editors[self.app.active_editor],
                            click_so, row_idx, rel.x, gw, cw, wrap_cols, lc, &mut click_scratch,
                        );
                        let ed = &mut self.app.editors[self.app.active_editor];
                        ed.cursor.line = actual_line;
                        ed.cursor.col = col;
                        ed.select_word_at_cursor();
                    }
                }
            } else if response.clicked() {
                self.app.focus = Focus::Editor;
                if let Some(pos) = response.interact_pointer_pos() {
                    if !(self.app.show_minimap && pos.x > rect.max.x - minimap_w_check) {
                        let rel = pos - rect.min;
                        let row_idx = rel_to_row(rel.y);
                        let (actual_line, col) = point_to_line_col(
                            &self.app.editors[self.app.active_editor],
                            click_so, row_idx, rel.x, gw, cw, wrap_cols, lc, &mut click_scratch,
                        );
                        let ed = &mut self.app.editors[self.app.active_editor];

                        if rel.x < gw {
                            if ed.fold_ranges.contains_key(&actual_line) {
                                ed.toggle_fold(actual_line);
                            } else {
                                ed.cursor.line = actual_line;
                                ed.cursor.col = 0;
                            }
                        } else {
                            ed.cursor.line = actual_line;
                            ed.cursor.col = col;
                        }
                        ed.selection = None;
                        ed.extra_cursors.clear();
                    }
                }
            }

            // Drag to select
            if response.drag_started() {
                if let Some(pos) = response.interact_pointer_pos() {
                    if !(self.app.show_minimap && pos.x > rect.max.x - minimap_w_check) {
                        let rel = pos - rect.min;
                        let row_idx = rel_to_row(rel.y);
                        let (actual_line, col) = point_to_line_col(
                            &self.app.editors[self.app.active_editor],
                            click_so, row_idx, rel.x, gw, cw, wrap_cols, lc, &mut click_scratch,
                        );
                        let ed = &mut self.app.editors[self.app.active_editor];
                        ed.cursor.line = actual_line;
                        ed.cursor.col = col;
                        ed.selection = Some(crate::editor::Selection {
                            start_line: actual_line, start_col: col,
                            end_line: actual_line, end_col: col,
                        });
                    }
                }
            }
            if response.dragged() {
                if let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) {
                    if !(self.app.show_minimap && pos.x > rect.max.x - minimap_w_check) {
                        let rel = pos - rect.min;
                        let row_idx = rel_to_row(rel.y);
                        let (actual_line, col) = point_to_line_col(
                            &self.app.editors[self.app.active_editor],
                            click_so, row_idx, rel.x, gw, cw, wrap_cols, lc, &mut click_scratch,
                        );
                        let ed = &mut self.app.editors[self.app.active_editor];
                        ed.cursor.line = actual_line;
                        ed.cursor.col = col;
                        ed.update_selection_end();
                    }
                }
            }

            // Right-click context menu (Zed-style)
            response.context_menu(|ui| {
                if ui.button("Cut              ⌘X").clicked() {
                    let ed = self.app.active_editor();
                    if ed.selection.is_some() {
                        let sel = ed.normalized_selection();
                        if let Some(sel) = sel {
                            let text = ed.buffer.get_range(sel.start_line, sel.start_col, sel.end_line, sel.end_col);
                            if let Some(ref mut cb) = self.clipboard { let _ = cb.set_text(&text); }
                        }
                        self.app.active_editor_mut().save_undo_snapshot();
                        self.app.active_editor_mut().delete_selection_text();
                    }
                    ui.close_menu();
                }
                if ui.button("Copy             ⌘C").clicked() {
                    let ed = self.app.active_editor();
                    if let Some(text) = ed.get_selected_text() {
                        if let Some(ref mut cb) = self.clipboard { let _ = cb.set_text(&text); }
                    } else {
                        let line = ed.buffer.get_line(ed.cursor.line);
                        if let Some(ref mut cb) = self.clipboard { let _ = cb.set_text(format!("{}\n", line)); }
                    }
                    ui.close_menu();
                }
                if ui.button("Paste            ⌘V").clicked() {
                    if let Some(ref mut cb) = self.clipboard {
                        if let Ok(text) = cb.get_text() {
                            let ed = self.app.active_editor_mut();
                            ed.save_undo_snapshot();
                            ed.delete_selection_text();
                            for c in text.chars() {
                                if c == '\n' { ed.insert_newline(); }
                                else if c != '\r' { ed.insert_char(c); }
                            }
                        }
                    }
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Select All       ⌘A").clicked() {
                    self.app.active_editor_mut().select_all();
                    ui.close_menu();
                }
                if ui.button("Select Line      ⌘L").clicked() {
                    self.app.active_editor_mut().select_line();
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Toggle Comment   ⌘/").clicked() {
                    self.app.active_editor_mut().toggle_comment();
                    ui.close_menu();
                }
                if ui.button("Fold / Unfold").clicked() {
                    let line = self.app.active_editor().cursor.line;
                    self.app.active_editor_mut().toggle_fold(line);
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Command Palette  ⇧⌘P").clicked() {
                    self.app.focus = Focus::CommandPalette;
                    self.app.palette_input.clear();
                    self.app.palette_selected = 0;
                    ui.close_menu();
                }
            });

            // Scroll — Zed-style sub-pixel smooth scrolling
            let cmd_held = ui.input(|i| i.modifiers.command);
            let sd = ui.input(|i| i.smooth_scroll_delta.y);
            if sd != 0.0 {
                if cmd_held {
                    let delta = if sd > 0.0 { 1.0 } else { -1.0 };
                    self.app.settings.font_size = (self.app.settings.font_size + delta).clamp(8.0, 48.0);
                    self.app.settings.save();
                    self.app.status_message = format!("Zoom: {}px", self.app.settings.font_size as u32);
                } else {
                    let ed = &mut self.app.editors[self.app.active_editor];
                    // Smooth scroll measured in pixels, carried across lines of
                    // *variable* visual height (wrapped lines are taller). The
                    // fractional scroll is a fraction through the top line's block,
                    // so a tall wrapped line is traversed row-by-row, never skipped.
                    let last = lc.saturating_sub(1);
                    let mut line = ed.scroll_offset.floor().max(0.0) as usize;
                    let frac = (ed.scroll_offset - line as f32).clamp(0.0, 1.0);
                    let line_h = |l: usize, s: &mut Vec<usize>| seg_count(ed, l, wrap_cols, s) as f32 * lh;
                    let mut scratch: Vec<usize> = Vec::new();
                    // Pixels into the top line's block, then move by -sd (down = +).
                    let mut within = frac * line_h(line, &mut scratch) - sd;
                    // Carry downward through full lines.
                    loop {
                        let h = line_h(line, &mut scratch);
                        if within < h || line >= last {
                            if line >= last { within = within.min(h - 1.0).max(0.0); }
                            break;
                        }
                        within -= h;
                        line += 1;
                    }
                    // Carry upward.
                    while within < 0.0 && line > 0 {
                        line -= 1;
                        within += line_h(line, &mut scratch);
                    }
                    let h = line_h(line, &mut scratch).max(lh);
                    let new_frac = (within / h).clamp(0.0, 0.9999);
                    ed.scroll_offset = line as f32 + new_frac;
                }
                // Force continuous repaint during scroll inertia — without this, egui only
                // repaints on raw input events, dropping FPS while smooth_scroll_delta decays.
                ctx.request_repaint();
            }

            let painter = ui.painter_at(rect);
            let vis = (rect.height() / lh).ceil() as usize;
            {
                let ed = &mut self.app.editors[self.app.active_editor];
                ed.viewport_height = vis.max(1);
                ed.wrap_cols = wrap_cols; // keep scroll/cursor math wrap-aware
            }
            let ed = &self.app.editors[self.app.active_editor];
            // Sub-pixel scroll math: floor → top logical line; the fractional part
            // is a fraction *through that line's wrapped block*, so a tall wrapped
            // top line scrolls smoothly row-by-row instead of jumping.
            let so = ed.scroll_offset.floor().max(0.0) as usize;
            let mut so_scratch: Vec<usize> = Vec::new();
            let top_segs = seg_count(ed, so, wrap_cols, &mut so_scratch);
            let pixel_offset = (ed.scroll_offset - so as f32) * top_segs as f32 * lh;
            let text_y_offset = LINE_SPACING / 2.0;

            // +2 ensures partial top + bottom rows are present
            let vis_lines = ed.visible_lines(so, vis + 2);
            let bracket_depths = compute_bracket_depths(&self.app, &vis_lines);

            // Gutter separator
            if show_ln {
                painter.line_segment(
                    [Pos2::new(rect.min.x + gw - fold_w - 2.0, rect.min.y), Pos2::new(rect.min.x + gw - fold_w - 2.0, rect.max.y)],
                    Stroke::new(1.0, self.tc.border),
                );
            }

            // Pre-compute selection once (not per line)
            let norm_sel = ed.normalized_selection();
            let cursor_line = ed.cursor.line;
            let cursor_col = ed.cursor.col;
            let xs = rect.min.x + gw;
            let fold_x = rect.min.x + gw - fold_w + 2.0;
            let ln_x = rect.min.x + gw - fold_w - cw * 0.8;
            // Pre-format line numbers to avoid alloc per line
            let mut ln_buf = String::with_capacity(8);

            // Soft-wrap rendering. `y` accumulates per visual row instead of
            // per logical line, so a wrapped line occupies several rows. Scroll
            // is still anchored to logical line `so`; the fractional `pixel_offset`
            // scrolls within the first row. `wrap_starts` is reused every line.
            let mut y = rect.min.y - pixel_offset;
            let mut wrap_starts: Vec<usize> = Vec::with_capacity(8);
            let error_color = Color32::from_rgb(247, 118, 142);

            // Captured during the loop for post-loop drawing (caret/blame/extras).
            let mut caret_xy: Option<(f32, f32)> = None;
            let mut blame_anchor: Option<(f32, f32)> = None;
            let mut extra_caret_xy: Vec<(f32, f32)> = Vec::new();

            for &li in vis_lines.iter() {
                if y > rect.max.y { break; }

                // Line content — single alloc per line.
                let line = ed.buffer.get_line(li);
                let hls = if li < ed.highlight_cache.len() { &ed.highlight_cache[li] } else { &[] as &[syntax::HighlightSpan] };
                let chars: Vec<char> = if line.len() > MAX_LINE_LEN {
                    line.chars().take(MAX_LINE_LEN).collect()
                } else {
                    line.chars().collect()
                };
                let line_len = chars.len();

                compute_wrap_starts(&chars, wrap_cols, &mut wrap_starts);
                let n_seg = wrap_starts.len();
                let block_h = n_seg as f32 * lh;

                // Whole wrapped block sits above the viewport — skip it.
                if y + block_h < rect.min.y {
                    y += block_h;
                    continue;
                }

                // Current line highlight — covers every wrapped row.
                if li == cursor_line {
                    painter.rect_filled(
                        Rect::from_min_size(Pos2::new(rect.min.x, y), Vec2::new(rect.width(), block_h)),
                        CornerRadius::ZERO, self.tc.current_line_bg,
                    );
                }

                // Git diff gutter (only if we have diffs)
                if !ed.line_diff.is_empty() {
                    if let Some(diff_status) = ed.line_diff.get(&li) {
                        let diff_color = match diff_status {
                            crate::editor::LineDiffStatus::Added => self.tc.green,
                            crate::editor::LineDiffStatus::Modified => Color32::from_rgb(70, 140, 220),
                        };
                        painter.rect_filled(
                            Rect::from_min_size(Pos2::new(rect.min.x + 1.0, y), Vec2::new(3.0, block_h)),
                            CornerRadius::ZERO, diff_color,
                        );
                    }
                }

                // Line numbers — first visual row only.
                if show_ln {
                    ln_buf.clear();
                    use std::fmt::Write;
                    let _ = write!(ln_buf, "{}", li + 1);
                    let nc = if li == cursor_line { self.tc.fg } else { self.tc.gutter_fg };
                    painter.text(Pos2::new(ln_x, y + text_y_offset), egui::Align2::RIGHT_TOP,
                        &ln_buf, sfont.clone(), nc);
                }

                // Fold indicators — only if this line has a fold
                if !ed.fold_ranges.is_empty() {
                    if let Some(&fold_end) = ed.fold_ranges.get(&li) {
                        let is_folded = ed.folded.contains(&li);
                        let arrow = if is_folded { "▸" } else { "▾" };
                        let fold_color = if li == cursor_line { self.tc.fg_dim } else { self.tc.fold_fg };
                        painter.text(Pos2::new(fold_x, y + text_y_offset), egui::Align2::LEFT_TOP,
                            arrow, sfont.clone(), fold_color);
                        if is_folded {
                            let (lex, ley) = seg_xy(&wrap_starts, line_len, xs, y, cw, lh);
                            let line_end_x = lex + cw;
                            let folded_count = fold_end - li;
                            painter.rect_filled(
                                Rect::from_min_size(Pos2::new(line_end_x, ley + 2.0),
                                    Vec2::new(cw * 6.0, lh - 4.0)),
                                CornerRadius::same(3), if dark { Color32::from_rgb(40, 44, 65) } else { Color32::from_rgb(228, 228, 228) });
                            ln_buf.clear();
                            let _ = write!(ln_buf, "⋯ {}", folded_count);
                            painter.text(Pos2::new(line_end_x + cw * 0.5, ley + text_y_offset), egui::Align2::LEFT_TOP,
                                &ln_buf, FontId::monospace((fs - 2.5).max(8.0)), self.tc.fg_dim);
                        }
                    }
                }

                // Selection highlight — drawn per wrapped row.
                if let Some(ref sel) = norm_sel {
                    if li >= sel.start_line && li <= sel.end_line {
                        let sel_start = if li == sel.start_line { sel.start_col } else { 0 };
                        let sel_end = if li == sel.end_line { sel.end_col } else { line_len };
                        let full_line = li < sel.end_line; // selection continues past this line
                        if sel_start < sel_end || full_line {
                            for sidx in 0..n_seg {
                                let s = wrap_starts[sidx];
                                let e = if sidx + 1 < n_seg { wrap_starts[sidx + 1] } else { line_len };
                                let row_end = if full_line { e } else { sel_end.min(e) };
                                let a = sel_start.max(s);
                                let b = row_end.max(a);
                                if b > a || full_line {
                                    let sx = xs + (a - s) as f32 * cw;
                                    let mut sw = (b - a) as f32 * cw;
                                    if full_line && sidx + 1 == n_seg { sw += cw; } // trailing newline hint
                                    let row_y = y + sidx as f32 * lh;
                                    painter.rect_filled(
                                        Rect::from_min_size(Pos2::new(sx, row_y), Vec2::new(sw.max(cw * 0.5), lh)),
                                        CornerRadius::ZERO, self.tc.selection_bg,
                                    );
                                }
                            }
                        }
                    }
                }

                // Find match highlights
                for m in &self.app.find_matches {
                    if m.line == li {
                        let (mx, my) = seg_xy(&wrap_starts, m.col, xs, y, cw, lh);
                        let mw = m.length as f32 * cw;
                        painter.rect_filled(
                            Rect::from_min_size(Pos2::new(mx - 1.0, my), Vec2::new(mw + 2.0, lh)),
                            CornerRadius::same(2),
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
                    if li == ml && mc < line_len {
                        let (bx, by) = seg_xy(&wrap_starts, mc, xs, y, cw, lh);
                        painter.rect_filled(
                            Rect::from_min_size(Pos2::new(bx - 1.0, by), Vec2::new(cw + 2.0, lh)),
                            CornerRadius::same(2), self.tc.bracket_match_bg,
                        );
                    }
                }
                if li == cursor_line && cursor_col < line_len {
                    let ch = chars[cursor_col];
                    if "()[]{}".contains(ch) && bracket_match.is_some() {
                        let (bx, by) = seg_xy(&wrap_starts, cursor_col, xs, y, cw, lh);
                        painter.rect_filled(
                            Rect::from_min_size(Pos2::new(bx - 1.0, by), Vec2::new(cw + 2.0, lh)),
                            CornerRadius::same(2), self.tc.bracket_match_bg,
                        );
                    }
                }

                // Indent guides — extend down every wrapped row.
                if !chars.is_empty() {
                    let indent_spaces = chars.iter().take_while(|c| **c == ' ').count();
                    for g in (4..indent_spaces + 1).step_by(4) {
                        let gx = xs + g as f32 * cw;
                        painter.line_segment(
                            [Pos2::new(gx, y), Pos2::new(gx, y + block_h)],
                            Stroke::new(1.0, if dark { Color32::from_rgb(57, 57, 57) } else { Color32::from_rgb(228, 228, 228) }),
                        );
                    }
                }

                // Render text with syntax highlighting + rainbow brackets — one
                // paint per wrapped segment, each starting back at the left margin.
                for sidx in 0..n_seg {
                    let s = wrap_starts[sidx];
                    let e = if sidx + 1 < n_seg { wrap_starts[sidx + 1] } else { line_len };
                    let seg_y = y + sidx as f32 * lh + text_y_offset;
                    self.render_line_text(&painter, &chars, &hls, &bracket_depths, li, s, e, xs, seg_y, &font, dark);
                }

                // Error underlines (drawn on the row that holds the diagnostic start)
                for diag in &ed.diagnostics {
                    if diag.line == li {
                        let err_len = if diag.length > 0 { diag.length } else { 1 };
                        let (err_x_start, err_y) = seg_xy(&wrap_starts, diag.col, xs, y, cw, lh);
                        let err_x_end = err_x_start + err_len as f32 * cw;
                        let wave_y = err_y + lh - 2.0;

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

                // Capture caret / blame anchor / extra-cursor screen positions
                // while we still have this line's wrap layout.
                if li == cursor_line {
                    caret_xy = Some(seg_xy(&wrap_starts, cursor_col, xs, y, cw, lh));
                    blame_anchor = Some(seg_xy(&wrap_starts, line_len, xs, y, cw, lh));
                }
                for ec in &ed.extra_cursors {
                    if ec.line == li {
                        extra_caret_xy.push(seg_xy(&wrap_starts, ec.col, xs, y, cw, lh));
                    }
                }

                y += block_h;
            }

            let mut minimap_new_scroll: Option<usize> = None;

            // Primary caret
            if self.app.focus == Focus::Editor {
                if let Some((cx, cy)) = caret_xy {
                    let blink = (ui.input(|i| i.time) * 1000.0) as u64 % (CURSOR_BLINK_INTERVAL_MS * 2) < CURSOR_BLINK_INTERVAL_MS;
                    if blink && cy < rect.max.y && cy + lh > rect.min.y {
                        painter.rect_filled(
                            Rect::from_min_size(Pos2::new(cx, cy), Vec2::new(2.0, lh)),
                            CornerRadius::ZERO, self.tc.cursor_color,
                        );
                    }
                }
            }

            // Git blame for the current line. libgit2 blames the WHOLE file (slow,
            // O(history)) so it runs on a worker thread — never on the UI thread.
            // One blame in flight at a time (coalesced); the result is shown only
            // while it still matches the cursor line.
            if let Some((bx, by)) = blame_anchor {
                let cur_line = ed.cursor.line;

                // Install a finished blame result.
                if let Some(rx) = &self.blame_rx {
                    if let Ok((line, text)) = rx.try_recv() {
                        self.blame_cache = text.map(|t| (line, t));
                        self.blame_cache_line = line;
                        self.blame_rx = None;
                    }
                }

                // Cursor moved to a new line and no blame running → kick one off.
                if self.blame_cache_line != cur_line && self.blame_rx.is_none() {
                    if let (Some(root), Some(fp)) =
                        (self.app.file_tree.root_path.clone(), ed.file_path.clone())
                    {
                        let rel_path = fp.strip_prefix(&root).unwrap_or(&fp)
                            .trim_start_matches('/').to_string();
                        let (tx, rx) = std::sync::mpsc::channel();
                        self.blame_rx = Some(rx);
                        std::thread::spawn(move || {
                            let git = crate::git::GitManager::new();
                            let res = git.blame_line(&root, &rel_path, cur_line);
                            let _ = tx.send((cur_line, res));
                        });
                    } else {
                        self.blame_cache = None;
                        self.blame_cache_line = cur_line;
                    }
                }

                if self.blame_rx.is_some() {
                    ctx.request_repaint();
                }

                // Only paint blame that belongs to the line the cursor is on now.
                if let Some((line, ref blame_text)) = self.blame_cache {
                    if line == cur_line {
                        let blame_x = bx + 4.0 * cw;
                        painter.text(
                            Pos2::new(blame_x, by + LINE_SPACING / 2.0),
                            egui::Align2::LEFT_TOP,
                            blame_text,
                            FontId::monospace(11.0),
                            self.tc.fg_dim.linear_multiply(0.5),
                        );
                    }
                }
            }

            // Minimap
            if self.app.show_minimap && lc > 1 {
                self.render_minimap(&painter, rect, dark, lc, vis, so, ed, &mut minimap_new_scroll, ctx);
            } else if lc > vis {
                // Scrollbar (wider, more visible)
                let sb_h = (vis as f32 / lc as f32 * rect.height()).max(24.0);
                let sb_y = rect.min.y + (ed.scroll_offset / lc as f32 * rect.height());
                // Track background
                painter.rect_filled(
                    Rect::from_min_size(Pos2::new(rect.max.x - SCROLLBAR_WIDTH, rect.min.y), Vec2::new(SCROLLBAR_WIDTH, rect.height())),
                    CornerRadius::ZERO, if dark { Color32::from_rgba_premultiplied(30, 33, 40, 80) } else { Color32::from_rgba_premultiplied(0, 0, 0, 10) },
                );
                // Thumb
                painter.rect_filled(
                    Rect::from_min_size(Pos2::new(rect.max.x - SCROLLBAR_WIDTH + 2.0, sb_y), Vec2::new(SCROLLBAR_WIDTH - 4.0, sb_h)),
                    CornerRadius::same(4), if dark { Color32::from_rgba_premultiplied(150, 160, 180, 100) } else { Color32::from_rgba_premultiplied(0, 0, 0, 60) },
                );
            }

            // Autocomplete popup — anchored at the (wrap-aware) caret position.
            if self.app.show_autocomplete && self.app.focus == Focus::Editor {
                self.render_autocomplete(&painter, caret_xy, lh, dark);
            }

            // Extra cursors
            if self.app.focus == Focus::Editor {
                let blink = (ui.input(|i| i.time) * 1000.0) as u64 % (CURSOR_BLINK_INTERVAL_MS * 2) < CURSOR_BLINK_INTERVAL_MS;
                if blink {
                    for &(ecx, ecy) in &extra_caret_xy {
                        if ecy < rect.max.y && ecy + lh > rect.min.y {
                            painter.rect_filled(
                                Rect::from_min_size(Pos2::new(ecx, ecy), Vec2::new(2.0, lh)),
                                CornerRadius::ZERO, self.tc.cursor_color,
                            );
                        }
                    }
                }
            }

            if let Some(new_scroll) = minimap_new_scroll {
                self.app.editors[self.app.active_editor].scroll_offset = new_scroll as f32;
            }
        });
    }

    /// Render the wrapped segment `chars[seg_start..seg_end]` of a logical line.
    /// Highlight colours and rainbow-bracket depths are computed over absolute
    /// column indices, but the segment is drawn starting at the left margin `xs`
    /// (column `seg_start` maps to `xs`). With wrap off the caller passes
    /// `seg_start = 0`, `seg_end = chars.len()`, reproducing the old full-line draw.
    fn render_line_text(
        &self,
        painter: &egui::Painter,
        chars: &[char],
        hls: &[syntax::HighlightSpan],
        bracket_depths: &std::collections::HashMap<(usize, usize), usize>,
        li: usize,
        seg_start: usize,
        seg_end: usize,
        xs: f32,
        y: f32,
        font: &FontId,
        dark: bool,
    ) {
        if chars.is_empty() { return; }
        let cw = painter.fonts(|f| f.glyph_width(font, ' '));

        // Build per-char color map
        let len = chars.len();
        let mut colors = vec![self.tc.fg; len];

        // Apply syntax highlight colors — syntect populates override_color from its theme,
        // the legacy keyword highlighter leaves it None and we fall back to kind.color(dark).
        for hl in hls {
            let start = hl.start.min(len);
            let end = hl.end.min(len);
            let c = hl.override_color.unwrap_or_else(|| hl.kind.color(dark));
            // colors[] is indexed by char position, but spans are byte ranges. Translate.
            let mut byte = 0usize;
            for (ci, ch) in chars.iter().enumerate() {
                if byte >= end { break; }
                if byte >= start { colors[ci] = c; }
                byte += ch.len_utf8();
            }
        }

        // Override bracket colors (rainbow)
        for (ci, &ch) in chars.iter().enumerate() {
            if "()[]{}".contains(ch) {
                if let Some(&depth) = bracket_depths.get(&(li, ci)) {
                    colors[ci] = self.tc.bracket_colors[depth % self.tc.bracket_colors.len()];
                }
            }
        }

        // Render in batched spans of same color — much faster than per-char.
        // Only the requested segment is drawn, shifted so it starts at `xs`.
        let lo = seg_start.min(len);
        let hi = seg_end.min(len);
        let mut span_start = lo;
        while span_start < hi {
            let color = colors[span_start];
            let mut span_end = span_start + 1;
            while span_end < hi && colors[span_end] == color {
                span_end += 1;
            }
            let text: String = chars[span_start..span_end].iter().collect();
            painter.text(
                Pos2::new(xs + (span_start - seg_start) as f32 * cw, y),
                egui::Align2::LEFT_TOP,
                text,
                font.clone(),
                color,
            );
            span_start = span_end;
        }
    }

    fn render_welcome(&mut self, ui: &mut egui::Ui) {
        let resolved = self.app.settings.theme.resolved();
        let dark = resolved != Theme::Light;
        let tc = self.tc;
        let mut open_project: Option<String> = None;
        let mut remove_project: Option<String> = None;
        let mut clear_all_projects = false;

        // Visual constants (Ferrite welcome).
        let card_bg = if dark {
            Color32::from_rgb(0x11, 0x14, 0x1d)
        } else {
            Color32::from_rgb(0xee, 0xf0, 0xf4)
        };
        let card_bg_hover = if dark {
            Color32::from_rgb(0x18, 0x1c, 0x28)
        } else {
            Color32::from_rgb(0xe3, 0xe6, 0xed)
        };
        let card_border = if dark {
            Color32::from_rgb(0x1c, 0x22, 0x33)
        } else {
            Color32::from_rgb(0xd6, 0xda, 0xe2)
        };
        let badge_bg = if dark {
            Color32::from_rgb(0x13, 0x17, 0x22)
        } else {
            Color32::from_rgb(0xe9, 0xeb, 0xf0)
        };

        let avail_w = ui.available_width();
        let content_w = 920.0_f32.min(avail_w - 80.0);
        let side_pad = ((avail_w - content_w) / 2.0).max(0.0);

        // Wrap everything in a scroll area so smaller windows still work.
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(48.0);

                // ── Hero title + soft accent glow ──
                ui.vertical_centered(|ui| {
                    let title_font = FontId::monospace(96.0);
                    let title_galley = ui.fonts(|f| {
                        f.layout_no_wrap("Ferrite".into(), title_font.clone(), tc.accent)
                    });
                    let title_size = title_galley.size();
                    let (title_rect, _) = ui.allocate_exact_size(title_size, egui::Sense::hover());
                    ui.painter().galley(title_rect.min, title_galley, tc.accent);

                    ui.add_space(8.0);
                    // Subtitle: "v0.4.2  •  ⚡ Built with Rust"
                    ui.horizontal(|ui| {
                        // Center the line manually by computing widths.
                        let s_font = FontId::monospace(13.0);
                        let v = format!("v{}", env!("CARGO_PKG_VERSION"));
                        let mid = "  •  ".to_string();
                        let rest = "Built with Rust".to_string();
                        let g1 = ui.fonts(|f| f.layout_no_wrap(v.clone(), s_font.clone(), tc.fg_dim));
                        let g2 = ui.fonts(|f| f.layout_no_wrap(mid.clone(), s_font.clone(), tc.fg_dim));
                        let g3 = ui.fonts(|f| f.layout_no_wrap(rest.clone(), s_font.clone(), tc.accent));
                        let total = g1.size().x + g2.size().x + g3.size().x + 22.0; // 22px for bolt
                        let pad = (ui.available_width() - total) / 2.0;
                        if pad > 0.0 { ui.add_space(pad); }
                        ui.label(RichText::new(v).font(s_font.clone()).color(tc.fg_dim));
                        ui.label(RichText::new(mid).font(s_font.clone()).color(tc.fg_dim));
                        // Bolt glyph + label in accent color
                        ui.label(RichText::new("⚡ ").font(s_font.clone()).color(tc.accent));
                        ui.label(RichText::new(rest).font(s_font).color(tc.accent));
                    });
                });

                ui.add_space(40.0);

                // ── Two action cards: Open Folder · Open File ──
                ui.horizontal(|ui| {
                    ui.add_space(side_pad);
                    let gap = 18.0;
                    let card_w = (content_w - gap) / 2.0;
                    let card_h = 88.0;

                    if self.welcome_action_card(
                        ui, card_w, card_h, card_bg, card_bg_hover, card_border,
                        super::icons::folder_filled, "Open Folder", "\u{2318} O", tc,
                    ) {
                        self.app.pending_action = Some(PaletteAction::OpenFolder);
                    }
                    ui.add_space(gap);
                    if self.welcome_action_card(
                        ui, card_w, card_h, card_bg, card_bg_hover, card_border,
                        super::icons::document, "Open File", "\u{2318} \u{21E7} O", tc,
                    ) {
                        self.app.pending_action = Some(PaletteAction::OpenFile);
                    }
                });

                // ── Recent Projects ──
                let recent = self.app.settings.recent_projects.clone();
                let home_dir = dirs::home_dir()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();

                if !recent.is_empty() {
                    ui.add_space(40.0);
                    ui.horizontal(|ui| {
                        ui.add_space(side_pad);
                        ui.label(
                            RichText::new("Recent Projects")
                                .font(FontId::monospace(15.0))
                                .color(tc.fg),
                        );
                        ui.add_space(14.0);
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("Clear All")
                                        .font(FontId::monospace(11.0))
                                        .color(tc.fg_dim),
                                )
                                .fill(Color32::TRANSPARENT)
                                .stroke(Stroke::NONE),
                            )
                            .clicked()
                        {
                            clear_all_projects = true;
                        }
                    });
                    ui.add_space(10.0);

                    for project in recent.iter().take(6) {
                        let short_path =
                            if !home_dir.is_empty() && project.path.starts_with(&home_dir) {
                                format!("~{}", &project.path[home_dir.len()..])
                            } else {
                                project.path.clone()
                            };
                        let time_label = humanize_time(project.timestamp);

                        ui.horizontal(|ui| {
                            ui.add_space(side_pad);
                            let row_h = 36.0;
                            let (row_rect, row_resp) = ui.allocate_exact_size(
                                Vec2::new(content_w, row_h),
                                egui::Sense::click(),
                            );
                            if row_resp.hovered() {
                                ui.painter().rect_filled(
                                    row_rect,
                                    CornerRadius::same(6),
                                    Color32::from_rgba_unmultiplied(255, 255, 255, 10),
                                );
                            }
                            if row_resp.clicked() {
                                open_project = Some(project.path.clone());
                            }
                            row_resp.context_menu(|ui| {
                                if ui.button("Remove").clicked() {
                                    remove_project = Some(project.path.clone());
                                    ui.close_menu();
                                }
                            });

                            // Folder icon
                            let icon_rect = Rect::from_min_size(
                                Pos2::new(row_rect.min.x + 8.0, row_rect.center().y - 9.0),
                                Vec2::new(18.0, 18.0),
                            );
                            super::icons::folder_outline(ui.painter(), icon_rect, tc.fg_dim);

                            // Name (bold-ish via white) + path (dim) on the left
                            let name_pos =
                                Pos2::new(icon_rect.right() + 12.0, row_rect.center().y);
                            ui.painter().text(
                                name_pos,
                                egui::Align2::LEFT_CENTER,
                                &project.name,
                                FontId::monospace(13.0),
                                tc.fg,
                            );
                            let name_w = ui.fonts(|f| {
                                f.layout_no_wrap(
                                    project.name.clone(),
                                    FontId::monospace(13.0),
                                    tc.fg,
                                )
                                .size()
                                .x
                            });
                            ui.painter().text(
                                Pos2::new(name_pos.x + name_w + 16.0, row_rect.center().y),
                                egui::Align2::LEFT_CENTER,
                                short_path,
                                FontId::monospace(12.0),
                                tc.fg_dim,
                            );

                            // Time on the right
                            ui.painter().text(
                                Pos2::new(row_rect.right() - 12.0, row_rect.center().y),
                                egui::Align2::RIGHT_CENTER,
                                time_label,
                                FontId::monospace(11.5),
                                tc.fg_dim,
                            );
                        });
                    }
                }

                // ── Quick Start (two columns of keyboard shortcuts) ──
                ui.add_space(44.0);
                ui.horizontal(|ui| {
                    ui.add_space(side_pad);
                    ui.label(
                        RichText::new("Quick Start")
                            .font(FontId::monospace(15.0))
                            .color(tc.fg),
                    );
                });
                ui.add_space(14.0);

                ui.horizontal(|ui| {
                    ui.add_space(side_pad);
                    let col_w = (content_w - 32.0) / 2.0;
                    let pairs_left: &[(&str, &str)] = &[
                        ("\u{2318} \u{21E7} P", "Command Palette"),
                        ("\u{2318} P",          "Quick Open"),
                        ("\u{2318} F",          "Find in File"),
                        ("\u{2318} \u{21E7} F", "Find in Project"),
                    ];
                    let pairs_right: &[(&str, &str)] = &[
                        ("\u{2318} B", "Toggle Sidebar"),
                        ("\u{2318} G", "Go to Line"),
                        ("\u{2318} S", "Save"),
                        ("\u{2318} Z", "Undo / Redo"),
                    ];

                    ui.vertical(|ui| {
                        ui.set_width(col_w);
                        for (k, d) in pairs_left {
                            self.welcome_shortcut_row(ui, k, d, badge_bg, card_border, tc);
                        }
                    });
                    ui.add_space(32.0);
                    ui.vertical(|ui| {
                        ui.set_width(col_w);
                        for (k, d) in pairs_right {
                            self.welcome_shortcut_row(ui, k, d, badge_bg, card_border, tc);
                        }
                    });
                });

                ui.add_space(40.0);
            });

        // Deferred actions
        if let Some(path) = open_project {
            self.app.open_folder(path);
        }
        if let Some(path) = remove_project {
            self.app.settings.remove_recent_project(&path);
        }
        if clear_all_projects {
            self.app.settings.recent_projects.clear();
            self.app.settings.save();
        }
    }

    /// Big rounded "card" button for the welcome screen.
    /// Returns true when clicked.
    #[allow(clippy::too_many_arguments)]
    fn welcome_action_card(
        &self,
        ui: &mut egui::Ui,
        w: f32,
        h: f32,
        bg: Color32,
        bg_hover: Color32,
        border: Color32,
        icon: fn(&egui::Painter, Rect, Color32),
        label: &str,
        shortcut: &str,
        tc: crate::settings::ThemeColors,
    ) -> bool {
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, h), egui::Sense::click());
        let hovered = resp.hovered();
        let painter = ui.painter();
        painter.rect_filled(rect, CornerRadius::same(12), if hovered { bg_hover } else { bg });
        painter.rect_stroke(
            rect,
            CornerRadius::same(12),
            Stroke::new(1.0, border),
            egui::StrokeKind::Inside,
        );
        // Icon: tinted square on the left
        let icon_box = Rect::from_min_size(
            Pos2::new(rect.min.x + 20.0, rect.center().y - 22.0),
            Vec2::splat(44.0),
        );
        painter.rect_filled(
            icon_box,
            CornerRadius::same(10),
            Color32::from_rgba_unmultiplied(tc.accent.r(), tc.accent.g(), tc.accent.b(), 36),
        );
        let icon_rect = Rect::from_center_size(icon_box.center(), Vec2::splat(22.0));
        icon(painter, icon_rect, if hovered { tc.accent } else { tc.fg });
        // Label + shortcut on the right
        let text_x = icon_box.right() + 18.0;
        painter.text(
            Pos2::new(text_x, rect.center().y - 12.0),
            egui::Align2::LEFT_CENTER,
            label,
            FontId::monospace(17.0),
            tc.fg,
        );
        painter.text(
            Pos2::new(text_x, rect.center().y + 14.0),
            egui::Align2::LEFT_CENTER,
            shortcut,
            FontId::monospace(11.5),
            tc.fg_dim,
        );
        resp.clicked()
    }

    fn welcome_shortcut_row(
        &self,
        ui: &mut egui::Ui,
        key: &str,
        desc: &str,
        badge_bg: Color32,
        border: Color32,
        tc: crate::settings::ThemeColors,
    ) {
        ui.horizontal(|ui| {
            let badge_w = 92.0;
            let (rect, _) = ui.allocate_exact_size(Vec2::new(badge_w, 26.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, CornerRadius::same(6), badge_bg);
            ui.painter().rect_stroke(
                rect,
                CornerRadius::same(6),
                Stroke::new(1.0, border),
                egui::StrokeKind::Inside,
            );
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                key,
                FontId::monospace(11.5),
                tc.fg,
            );
            ui.add_space(14.0);
            ui.label(
                RichText::new(desc)
                    .font(FontId::monospace(13.0))
                    .color(tc.fg_dim),
            );
        });
        ui.add_space(8.0);
    }

    fn render_breadcrumbs(&self, ui: &mut egui::Ui) {
        let ed = &self.app.editors[self.app.active_editor];
        if let Some(ref fp) = ed.file_path {
            let root = self.app.file_tree.root_path.as_deref().unwrap_or("");
            let rel = fp.strip_prefix(root).unwrap_or(fp).trim_start_matches('/');
            let parts: Vec<&str> = rel.split('/').collect();
            let dark = self.app.settings.theme.resolved() != Theme::Light;
            let bc_bg = if dark { Color32::from_rgb(32, 33, 40) } else { Color32::from_rgb(240, 240, 242) };
            let bc_h = 26.0;
            let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), bc_h), egui::Sense::hover());
            ui.painter().rect_filled(rect, CornerRadius::ZERO, bc_bg);
            let text_y = rect.min.y + (bc_h - 12.0) / 2.0;
            let mut x = rect.min.x + 14.0;
            for (i, part) in parts.iter().enumerate() {
                if i > 0 {
                    let sep_r = ui.painter().text(
                        Pos2::new(x, text_y), egui::Align2::LEFT_TOP,
                        " › ", FontId::monospace(11.5), self.tc.fg_dim,
                    );
                    x += sep_r.width();
                }
                let is_last = i == parts.len() - 1;
                let color = if is_last { self.tc.fg } else { self.tc.fg_dim };
                let tr = ui.painter().text(
                    Pos2::new(x, text_y), egui::Align2::LEFT_TOP,
                    *part, FontId::monospace(11.5), color,
                );
                x += tr.width();
            }
            // Subtle bottom border
            ui.painter().line_segment(
                [Pos2::new(rect.min.x, rect.max.y - 0.5), Pos2::new(rect.max.x, rect.max.y - 0.5)],
                Stroke::new(0.5, self.tc.border),
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
        painter.rect_filled(minimap_rect, CornerRadius::ZERO, minimap_bg);
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

        // Cap draw calls to ~(viewport_height / mini_line_h) — anything denser is wasted pixels.
        // For a 800px viewport that's 400 lines max regardless of file size, dropping a 2000-line
        // file from 2000 to 400 rect_filled calls per frame.
        let max_draws = ((rect.height() / mini_line_h) as usize).max(50);
        let step = (lc / max_draws).max(1);
        for li in (0..lc).step_by(step) {
            let my = rect.min.y + li as f32 * mini_line_h - minimap_scroll_offset;
            if my < rect.min.y - 2.0 { continue; }
            if my > rect.max.y { break; }

            // Use rope line length directly to avoid String allocation
            let line_char_len = ed.buffer.line_len(li);
            if line_char_len == 0 { continue; }
            let rope_line = ed.buffer.rope.line(li);
            let indent = rope_line.chars().take_while(|c| c.is_whitespace()).count();
            let content_len = line_char_len.saturating_sub(indent).min(60);
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
                CornerRadius::ZERO, line_color,
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
            CornerRadius::ZERO, vp_color,
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
                        CornerRadius::ZERO,
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
        caret_xy: Option<(f32, f32)>,
        lh: f32,
        dark: bool,
    ) {
        if let Some((caret_x, caret_y)) = caret_xy {
            let ac_x = caret_x;
            let ac_y = caret_y + lh;
            let ac_w = 220.0;
            let ac_item_h = 24.0;
            let ac_count = self.app.autocomplete_suggestions.len().min(8);
            let ac_h = ac_count as f32 * ac_item_h + 4.0;

            let popup_bg = if dark { Color32::from_rgb(43, 43, 43) } else { Color32::from_rgb(255, 255, 255) };
            let ac_rect = Rect::from_min_size(Pos2::new(ac_x, ac_y), Vec2::new(ac_w, ac_h));

            painter.rect_filled(
                ac_rect.translate(Vec2::new(2.0, 2.0)),
                CornerRadius::same(4), Color32::from_black_alpha(if dark { 80 } else { 30 }),
            );
            painter.rect_filled(ac_rect, CornerRadius::same(4), popup_bg);
            painter.rect_stroke(ac_rect, CornerRadius::same(4), Stroke::new(1.0, self.tc.border), egui::StrokeKind::Outside);

            for (i, suggestion) in self.app.autocomplete_suggestions.iter().enumerate().take(ac_count) {
                let item_y = ac_y + 2.0 + i as f32 * ac_item_h;
                let selected = i == self.app.autocomplete_selected;
                if selected {
                    painter.rect_filled(
                        Rect::from_min_size(Pos2::new(ac_x + 2.0, item_y), Vec2::new(ac_w - 4.0, ac_item_h)),
                        CornerRadius::same(3), self.tc.selection_bg,
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

#[cfg(test)]
mod wrap_tests {
    use super::seg_xy;

    #[test]
    fn seg_xy_picks_correct_row_and_column() {
        let starts = vec![0usize, 10, 20];
        // (xs, base_y) = (100, 50), cw = 8, lh = 16
        // col 3 → row 0
        assert_eq!(seg_xy(&starts, 3, 100.0, 50.0, 8.0, 16.0), (100.0 + 24.0, 50.0));
        // col 12 → row 1, offset 2 within the row
        assert_eq!(seg_xy(&starts, 12, 100.0, 50.0, 8.0, 16.0), (100.0 + 16.0, 66.0));
        // col 25 → row 2, offset 5
        assert_eq!(seg_xy(&starts, 25, 100.0, 50.0, 8.0, 16.0), (100.0 + 40.0, 82.0));
    }
}
