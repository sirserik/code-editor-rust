mod buffer;
mod cursor;

pub use buffer::Buffer;
pub use cursor::Cursor;

use std::collections::{HashMap, HashSet};

use crate::syntax::SyntaxError;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineDiffStatus {
    Added,
    Modified,
}

#[derive(Debug)]
pub struct Editor {
    pub buffer: Buffer,
    pub cursor: Cursor,
    pub scroll_offset: f32,  // pixel-level scroll (Zed-style)
    pub file_path: Option<String>,
    pub is_dirty: bool,
    pub viewport_height: usize,
    /// Soft-wrap width in columns, pushed in by the renderer each frame so that
    /// scrolling and cursor-visibility can reason in visual rows. `usize::MAX`
    /// means wrapping is off (every logical line is exactly one visual row).
    pub wrap_cols: usize,
    pub selection: Option<Selection>,
    // Undo/Redo: store snapshots
    undo_stack: Vec<(String, usize, usize)>, // (content, cursor_line, cursor_col)
    redo_stack: Vec<(String, usize, usize)>,
    undo_counter: usize, // track changes for periodic snapshots
    /// When the last undo snapshot was taken — used to coalesce a typing burst
    /// into one undo step instead of one per character.
    last_snapshot_time: Option<std::time::Instant>,
    // Code folding: maps fold start line -> fold end line
    pub fold_ranges: HashMap<usize, usize>,
    pub fold_ranges_computed: bool, // sticky flag to avoid re-running scan when ranges are empty
    pub folded: HashSet<usize>, // lines that are fold-start and currently folded
    // Syntax diagnostics
    pub diagnostics: Vec<SyntaxError>,
    pub diagnostics_dirty: bool,
    // Git diff per line
    pub line_diff: HashMap<usize, LineDiffStatus>,
    // Original content (for git diff computation)
    pub original_content: Option<String>,
    // Last edit timestamp for auto-save
    pub last_edit_time: Option<std::time::Instant>,
    // Multi-cursor: extra cursors beyond the main one
    pub extra_cursors: Vec<Cursor>,
    // Cached syntax highlights per line (Scintilla-style)
    pub highlight_cache: Vec<Vec<crate::syntax::HighlightSpan>>,
    pub highlight_cache_lang: String,
    /// Editor colour scheme the highlight cache was built under. Syntect bakes
    /// absolute RGB into each span, so switching themes must rebuild the cache —
    /// otherwise Darcula colours get painted on IntelliJ Light's white page.
    /// Keyed by the *scheme*, not a dark/light flag: two dark themes can have
    /// completely different code colours.
    pub highlight_cache_theme: Option<crate::settings::SyntaxTheme>,
    pub highlight_dirty_from: Option<usize>, // re-highlight from this line
    // Async analysis: the whole-buffer syntect pass + fold scan run on a worker
    // thread so opening/editing a file never blocks the UI. `highlight_gen` is
    // bumped whenever a re-analysis is needed; the worker tags its result with the
    // gen it ran for, and a stale result (older gen) is discarded. Coalesces bursts.
    // Payload: (gen, per-line highlight spans, fold-start→fold-end ranges,
    // syntax diagnostics). Diagnostics ride along because `check_syntax` is
    // another whole-buffer O(n) pass that has no business on the UI thread.
    #[allow(clippy::type_complexity)]
    pub highlight_rx: Option<std::sync::mpsc::Receiver<AnalysisResult>>,
    pub highlight_gen: u64,
    pub highlight_applied_gen: u64,
    /// How long the last analysis pass took. Small files re-analyse in a couple
    /// of milliseconds, so making *them* wait for a typing pause just leaves the
    /// text sitting in the default colour for a quarter of a second.
    pub last_analysis_ms: f32,
    /// Parser snapshots that let an edit resume near the change instead of
    /// re-parsing from line 0. Empty = the next pass is a full one; they are
    /// cleared whenever the line count or the colour scheme changes, since the
    /// snapshots are keyed by line number.
    pub highlight_checkpoints: Vec<crate::syntect_engine::Checkpoint>,
    /// Line the next analysis pass should resume from.
    pub highlight_request_from: usize,
}

/// One completed background analysis pass of a buffer.
pub struct AnalysisResult {
    pub generation: u64,
    /// First line the pass recomputed.
    pub start_line: usize,
    /// Line at which the parser state matched its previous value, so the cached
    /// spans from here down are still valid. `None` = recomputed to the end.
    pub converged_at: Option<usize>,
    /// Spans for `start_line .. converged_at`, spliced into the existing cache.
    pub highlights: Vec<Vec<crate::syntax::HighlightSpan>>,
    pub checkpoints: Vec<crate::syntect_engine::Checkpoint>,
    pub folds: HashMap<usize, usize>,
    pub diagnostics: Vec<SyntaxError>,
    /// Wall time the pass took, used to decide whether the next one is cheap
    /// enough to run immediately or needs to wait for a pause in typing.
    pub duration_ms: f32,
}

/// Compute brace/bracket fold ranges from raw text. Pure (no `&self`) so it can
/// run on a worker thread. Returns a map of fold-start line → fold-end line.
pub fn compute_folds(content: &str) -> HashMap<usize, usize> {
    let mut ranges = HashMap::new();
    let mut stack: Vec<usize> = Vec::new(); // lines where an opener appeared
    for (li, line) in content.lines().enumerate() {
        let mut in_string = false;
        let mut prev = '\0';
        for ch in line.chars() {
            if (ch == '"' || ch == '\'') && prev != '\\' {
                in_string = !in_string;
            }
            if !in_string {
                if ch == '{' || ch == '(' || ch == '[' {
                    stack.push(li);
                } else if ch == '}' || ch == ')' || ch == ']' {
                    if let Some(start) = stack.pop() {
                        if li > start {
                            ranges.insert(start, li);
                        }
                    }
                }
            }
            prev = ch;
        }
    }
    ranges
}

#[derive(Debug, Clone)]
pub struct Selection {
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
}

impl Editor {
    pub fn new() -> Self {
        Self {
            buffer: Buffer::new(),
            cursor: Cursor::new(),
            scroll_offset: 0.0,
            file_path: None,
            is_dirty: false,
            viewport_height: 24,
            wrap_cols: usize::MAX,
            selection: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            undo_counter: 0,
            last_snapshot_time: None,
            fold_ranges: HashMap::new(),
            fold_ranges_computed: false,
            folded: HashSet::new(),
            diagnostics: Vec::new(),
            diagnostics_dirty: true,
            line_diff: HashMap::new(),
            original_content: None,
            last_edit_time: None,
            extra_cursors: Vec::new(),
            highlight_cache: Vec::new(),
            highlight_cache_lang: String::new(),
            highlight_cache_theme: None,
            highlight_dirty_from: Some(0),
            highlight_rx: None,
            highlight_gen: 0,
            highlight_applied_gen: 0,
            last_analysis_ms: 0.0,
            highlight_checkpoints: Vec::new(),
            highlight_request_from: 0,
        }
    }

    pub fn from_file(path: &str) -> std::io::Result<Self> {
        // Read once and reuse the string for the rope, the undo baseline, and the
        // git-diff baseline — avoids rebuilding the whole file text from the rope.
        let initial_content = std::fs::read_to_string(path)?;
        let buffer = Buffer { rope: ropey::Rope::from_str(&initial_content) };
        Ok(Self {
            buffer,
            cursor: Cursor::new(),
            scroll_offset: 0.0,
            file_path: Some(path.to_string()),
            is_dirty: false,
            viewport_height: 24,
            wrap_cols: usize::MAX,
            selection: None,
            undo_stack: vec![(initial_content.clone(), 0, 0)],
            redo_stack: Vec::new(),
            undo_counter: 0,
            last_snapshot_time: None,
            fold_ranges: HashMap::new(),
            fold_ranges_computed: false,
            folded: HashSet::new(),
            diagnostics: Vec::new(),
            diagnostics_dirty: true,
            line_diff: HashMap::new(),
            original_content: Some(initial_content),
            last_edit_time: None,
            extra_cursors: Vec::new(),
            highlight_cache: Vec::new(),
            highlight_cache_lang: String::new(),
            highlight_cache_theme: None,
            highlight_dirty_from: Some(0),
            highlight_rx: None,
            highlight_gen: 0,
            highlight_applied_gen: 0,
            last_analysis_ms: 0.0,
            highlight_checkpoints: Vec::new(),
            highlight_request_from: 0,
        })
    }

    /// Mark highlights dirty from a specific line (for incremental re-lex)
    pub fn invalidate_highlights_from(&mut self, line: usize) {
        match self.highlight_dirty_from {
            Some(existing) => self.highlight_dirty_from = Some(existing.min(line)),
            None => self.highlight_dirty_from = Some(line),
        }
    }

    /// Detect indent size from file content (Zed-style): samples up to first 100 lines,
    /// returns most-common leading-space count (2..=8) or 4 if tabs dominate.
    pub fn detect_indent(&self) -> usize {
        let mut space_counts = [0u32; 9];
        let mut tab_count = 0u32;
        let sample = self.buffer.line_count().min(100);
        for li in 0..sample {
            // Only the leading whitespace matters — read it straight off the
            // rope rather than allocating a `String` per sampled line (this runs
            // on every Tab press).
            let mut chars = self.buffer.line_chars(li);
            match chars.next() {
                Some('\t') => tab_count += 1,
                Some(' ') => {
                    let spaces = 1 + chars.take_while(|c| *c == ' ').count();
                    if (2..=8).contains(&spaces) {
                        space_counts[spaces] += 1;
                    }
                }
                _ => continue,
            }
        }
        if tab_count > space_counts.iter().sum::<u32>() {
            return 4;
        }
        space_counts.iter().enumerate()
            .skip(2)
            .filter(|(_, &c)| c > 0)
            .max_by_key(|(_, &c)| c)
            .map(|(i, _)| i)
            .unwrap_or(4)
    }

    pub fn file_name(&self) -> String {
        self.file_path
            .as_ref()
            .map(|p| {
                std::path::Path::new(p)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| p.clone())
            })
            .unwrap_or_else(|| "untitled".to_string())
    }

    pub fn save(&mut self) -> std::io::Result<()> {
        if let Some(ref path) = self.file_path {
            self.buffer.save(path)?;
            self.is_dirty = false;
        }
        Ok(())
    }

    pub fn insert_char(&mut self, c: char) {
        self.maybe_save_undo();
        let line = self.cursor.line;
        let col = self.cursor.col;
        self.buffer.insert_char(line, col, c);
        self.cursor.col += 1;
        self.is_dirty = true; self.diagnostics_dirty = true;
        self.last_edit_time = Some(std::time::Instant::now());
    }

    pub fn insert_newline(&mut self) {
        self.save_undo_snapshot();
        let line = self.cursor.line;
        let col = self.cursor.col;

        // Get current line indentation for auto-indent
        let indent = self.buffer.get_line_indent(line);

        self.buffer.insert_newline(line, col);
        self.cursor.line += 1;
        self.cursor.col = 0;

        // Auto-indent
        if !indent.is_empty() {
            for c in indent.chars() {
                self.buffer.insert_char(self.cursor.line, self.cursor.col, c);
                self.cursor.col += 1;
            }
        }

        self.is_dirty = true; self.diagnostics_dirty = true;
        self.last_edit_time = Some(std::time::Instant::now());
    }

    pub fn insert_tab(&mut self) {
        let width = self.detect_indent();
        for _ in 0..width {
            self.insert_char(' ');
        }
    }

    pub fn delete_back(&mut self) {
        self.maybe_save_undo();
        if self.cursor.col > 0 {
            self.cursor.col -= 1;
            self.buffer.delete_char(self.cursor.line, self.cursor.col);
            self.is_dirty = true; self.diagnostics_dirty = true;
        } else if self.cursor.line > 0 {
            let prev_line_len = self.buffer.line_len(self.cursor.line - 1);
            self.buffer.join_lines(self.cursor.line - 1);
            self.cursor.line -= 1;
            self.cursor.col = prev_line_len;
            self.is_dirty = true; self.diagnostics_dirty = true;
        }
    }

    pub fn delete_forward(&mut self) {
        let line_len = self.buffer.line_len(self.cursor.line);
        if self.cursor.col < line_len {
            self.buffer.delete_char(self.cursor.line, self.cursor.col);
            self.is_dirty = true; self.diagnostics_dirty = true;
        } else if self.cursor.line < self.buffer.line_count() - 1 {
            self.buffer.join_lines(self.cursor.line);
            self.is_dirty = true; self.diagnostics_dirty = true;
        }
    }

    pub fn delete_line(&mut self) {
        self.save_undo_snapshot();
        if self.buffer.line_count() > 1 {
            self.buffer.delete_line(self.cursor.line);
            if self.cursor.line >= self.buffer.line_count() {
                self.cursor.line = self.buffer.line_count() - 1;
            }
            self.clamp_cursor();
            self.is_dirty = true; self.diagnostics_dirty = true;
        } else {
            // Clear the only line
            let len = self.buffer.line_len(0);
            for _ in 0..len {
                self.buffer.delete_char(0, 0);
            }
            self.cursor.col = 0;
            self.is_dirty = true; self.diagnostics_dirty = true;
        }
    }

    pub fn duplicate_line(&mut self) {
        self.buffer.duplicate_line(self.cursor.line);
        self.cursor.line += 1;
        self.is_dirty = true; self.diagnostics_dirty = true;
    }

    pub fn move_line_up(&mut self) {
        if self.cursor.line > 0 {
            self.buffer.swap_lines(self.cursor.line - 1, self.cursor.line);
            self.cursor.line -= 1;
            self.is_dirty = true; self.diagnostics_dirty = true;
        }
    }

    pub fn move_line_down(&mut self) {
        if self.cursor.line < self.buffer.line_count() - 1 {
            self.buffer.swap_lines(self.cursor.line, self.cursor.line + 1);
            self.cursor.line += 1;
            self.is_dirty = true; self.diagnostics_dirty = true;
        }
    }

    pub fn move_up(&mut self) {
        if self.cursor.line > 0 {
            self.cursor.line -= 1;
            self.clamp_cursor();
        }
    }

    pub fn move_down(&mut self) {
        if self.cursor.line < self.buffer.line_count() - 1 {
            self.cursor.line += 1;
            self.clamp_cursor();
        }
    }

    pub fn move_left(&mut self) {
        if self.cursor.col > 0 {
            self.cursor.col -= 1;
        } else if self.cursor.line > 0 {
            self.cursor.line -= 1;
            self.cursor.col = self.buffer.line_len(self.cursor.line);
        }
    }

    pub fn move_right(&mut self) {
        let line_len = self.buffer.line_len(self.cursor.line);
        if self.cursor.col < line_len {
            self.cursor.col += 1;
        } else if self.cursor.line < self.buffer.line_count() - 1 {
            self.cursor.line += 1;
            self.cursor.col = 0;
        }
    }

    pub fn move_word_left(&mut self) {
        if self.cursor.col == 0 {
            if self.cursor.line > 0 {
                self.cursor.line -= 1;
                self.cursor.col = self.buffer.line_len(self.cursor.line);
            }
            return;
        }
        let line = self.buffer.get_line(self.cursor.line);
        let chars: Vec<char> = line.chars().collect();
        let mut col = self.cursor.col;
        // Skip whitespace
        while col > 0 && chars.get(col - 1).map_or(false, |c| c.is_whitespace()) {
            col -= 1;
        }
        // Skip word chars
        while col > 0 && chars.get(col - 1).map_or(false, |c| c.is_alphanumeric() || *c == '_') {
            col -= 1;
        }
        self.cursor.col = col;
    }

    pub fn move_word_right(&mut self) {
        let line_len = self.buffer.line_len(self.cursor.line);
        if self.cursor.col >= line_len {
            if self.cursor.line < self.buffer.line_count() - 1 {
                self.cursor.line += 1;
                self.cursor.col = 0;
            }
            return;
        }
        let line = self.buffer.get_line(self.cursor.line);
        let chars: Vec<char> = line.chars().collect();
        let mut col = self.cursor.col;
        // Skip word chars
        while col < chars.len() && (chars[col].is_alphanumeric() || chars[col] == '_') {
            col += 1;
        }
        // Skip whitespace
        while col < chars.len() && chars[col].is_whitespace() {
            col += 1;
        }
        self.cursor.col = col;
    }

    pub fn move_home(&mut self) {
        // Smart home: first press goes to first non-whitespace, second to col 0
        let line = self.buffer.get_line(self.cursor.line);
        let first_non_ws = line.chars().take_while(|c| c.is_whitespace()).count();
        if self.cursor.col == first_non_ws {
            self.cursor.col = 0;
        } else {
            self.cursor.col = first_non_ws;
        }
    }

    pub fn move_end(&mut self) {
        self.cursor.col = self.buffer.line_len(self.cursor.line);
    }

    pub fn move_to_top(&mut self) {
        self.cursor.line = 0;
        self.cursor.col = 0;
        self.scroll_offset = 0.0;
    }

    pub fn move_to_bottom(&mut self) {
        self.cursor.line = self.buffer.line_count().saturating_sub(1);
        self.cursor.col = 0;
    }

    /// Visual rows a logical line occupies under the active wrap width (≥ 1).
    #[allow(dead_code)] // convenience wrapper; hot paths use `line_visual_rows_in`
    pub fn line_visual_rows(&self, line: usize) -> usize {
        if self.wrap_cols == usize::MAX {
            return 1;
        }
        let (mut chars, mut starts) = (Vec::new(), Vec::new());
        self.line_visual_rows_in(line, &mut chars, &mut starts)
    }

    /// `line_visual_rows` with caller-owned scratch buffers — the loop-friendly
    /// form. Wrap layout used to allocate a `String` *and* a `Vec<char>` per
    /// line, which is brutal inside the per-line loops below.
    fn line_visual_rows_in(&self, line: usize, chars: &mut Vec<char>, starts: &mut Vec<usize>) -> usize {
        if self.wrap_cols == usize::MAX {
            return 1;
        }
        self.buffer.line_chars_into(line, crate::wrap::MAX_LINE_LEN, chars);
        crate::wrap::visual_rows(chars, self.wrap_cols, starts)
    }

    /// Which visual row (0-based) the cursor column sits on within its line.
    fn cursor_visual_row(&self, line: usize, col: usize) -> usize {
        if self.wrap_cols == usize::MAX {
            return 0;
        }
        let mut chars = Vec::new();
        self.buffer.line_chars_into(line, crate::wrap::MAX_LINE_LEN, &mut chars);
        let mut scratch = Vec::new();
        crate::wrap::visual_row_of_col(&chars, self.wrap_cols, col, &mut scratch)
    }

    pub fn page_up(&mut self) {
        // Move the cursor up by roughly one viewport, measured in visual rows so
        // a screenful of wrapped text equals a screenful of logical lines.
        let budget0 = self.viewport_height.saturating_sub(2);
        if self.wrap_cols == usize::MAX {
            // No wrapping: one row per line, so the walk is plain arithmetic.
            self.cursor.line = self.cursor.line.saturating_sub(budget0);
            self.clamp_cursor();
            return;
        }
        let (mut chars, mut starts) = (Vec::new(), Vec::new());
        let mut budget = budget0;
        while budget > 0 && self.cursor.line > 0 {
            self.cursor.line -= 1;
            let rows = self.line_visual_rows_in(self.cursor.line, &mut chars, &mut starts);
            budget = budget.saturating_sub(rows);
        }
        self.clamp_cursor();
    }

    pub fn page_down(&mut self) {
        let last = self.buffer.line_count().saturating_sub(1);
        let budget0 = self.viewport_height.saturating_sub(2);
        if self.wrap_cols == usize::MAX {
            self.cursor.line = (self.cursor.line + budget0).min(last);
            self.clamp_cursor();
            return;
        }
        let (mut chars, mut starts) = (Vec::new(), Vec::new());
        let mut budget = budget0;
        while budget > 0 && self.cursor.line < last {
            let rows = self.line_visual_rows_in(self.cursor.line, &mut chars, &mut starts);
            budget = budget.saturating_sub(rows);
            self.cursor.line += 1;
        }
        self.cursor.line = self.cursor.line.min(last);
        self.clamp_cursor();
    }

    pub fn scroll_into_view(&mut self) {
        let vp_rows = self.viewport_height.max(1);
        let margin = 3.min(vp_rows / 3);
        let so = self.scroll_offset.floor().max(0.0) as usize;

        // Scroll up: cursor's logical line is above the top margin.
        if self.cursor.line < so + margin {
            self.scroll_offset = self.cursor.line.saturating_sub(margin) as f32;
            return;
        }

        if self.cursor.line < so {
            return;
        }

        // No wrapping: rows == lines, so both walks below collapse to arithmetic
        // instead of a per-line wrap layout pass.
        if self.wrap_cols == usize::MAX {
            let rows = self.cursor.line - so;
            if rows + margin >= vp_rows {
                let top = (self.cursor.line + margin + 1 - vp_rows).min(self.cursor.line);
                self.scroll_offset = top as f32;
            }
            return;
        }

        // Scroll down: count visual rows from the top of the viewport to the
        // cursor's row; if it falls past the bottom margin, pull the top line
        // forward until the cursor (plus margin) fits.
        let (mut chars, mut starts) = (Vec::new(), Vec::new());
        let mut rows = self.cursor_visual_row(self.cursor.line, self.cursor.col);
        for l in so..self.cursor.line {
            rows += self.line_visual_rows_in(l, &mut chars, &mut starts);
        }
        if rows + margin >= vp_rows {
            // Walk the top line downward, dropping its visual rows, until the
            // cursor's row sits within the viewport (leaving the margin).
            let target = rows + margin + 1 - vp_rows; // visual rows to drop off the top
            let mut dropped = 0;
            let mut top = so;
            while top < self.cursor.line && dropped < target {
                dropped += self.line_visual_rows_in(top, &mut chars, &mut starts);
                top += 1;
            }
            self.scroll_offset = top as f32;
        }
    }

    /// Scroll the viewport by `dy` pixels — positive scrolls *down* the document.
    ///
    /// `scroll_offset` is a logical line plus a fraction *through that line's
    /// wrapped block*, so the walk has to move by real pixel heights: a line
    /// wrapped into three rows is three times as tall as an unwrapped one.
    /// `row_h` is the pixel height of a single visual row.
    pub fn scroll_by_pixels(&mut self, dy: f32, row_h: f32) {
        let last = self.buffer.line_count().saturating_sub(1);
        let mut line = self.scroll_offset.floor().max(0.0) as usize;
        line = line.min(last);
        let frac = (self.scroll_offset - line as f32).clamp(0.0, 1.0);

        let (mut chars, mut starts) = (Vec::new(), Vec::new());
        macro_rules! h_of {
            ($l:expr) => {
                self.line_visual_rows_in($l, &mut chars, &mut starts) as f32 * row_h
            };
        }

        // Pixels into the top line's block, then move by `dy`.
        let mut within = frac * h_of!(line) + dy;

        // Carry downward through whole lines.
        loop {
            let h = h_of!(line);
            if within < h {
                break;
            }
            if line >= last {
                // Bottom of the document: swallow the overflow so the last line
                // stays pinned to the top of the viewport.
                //
                // This clamp used to also run for *upward* movement (the loop
                // condition was `within < h || line >= last`), which floored a
                // negative `within` to 0 before the upward carry below could
                // use it. The effect: once the last line reached the top of the
                // viewport, the wheel could never scroll back up again.
                within = within.min(h - 1.0).max(0.0);
                break;
            }
            within -= h;
            line += 1;
        }

        // Carry upward.
        while within < 0.0 && line > 0 {
            line -= 1;
            within += h_of!(line);
        }
        if within < 0.0 {
            within = 0.0; // already at the very top
        }

        let h = h_of!(line).max(row_h);
        self.scroll_offset = line as f32 + (within / h).clamp(0.0, 0.9999);
    }

    pub fn go_to_line(&mut self, line: usize) {
        let line = line.min(self.buffer.line_count()).saturating_sub(1);
        self.cursor.line = line;
        self.cursor.col = 0;
        self.scroll_into_view();
    }

    pub fn select_all(&mut self) {
        let last_line = self.buffer.line_count().saturating_sub(1);
        let last_col = self.buffer.line_len(last_line);
        self.selection = Some(Selection {
            start_line: 0,
            start_col: 0,
            end_line: last_line,
            end_col: last_col,
        });
    }

    /// Normalize selection so start <= end
    pub fn normalized_selection(&self) -> Option<Selection> {
        self.selection.as_ref().map(|sel| {
            if (sel.start_line, sel.start_col) <= (sel.end_line, sel.end_col) {
                sel.clone()
            } else {
                Selection {
                    start_line: sel.end_line, start_col: sel.end_col,
                    end_line: sel.start_line, end_col: sel.start_col,
                }
            }
        })
    }

    /// Start or extend selection from current cursor position
    pub fn ensure_selection_anchor(&mut self) {
        if self.selection.is_none() {
            self.selection = Some(Selection {
                start_line: self.cursor.line,
                start_col: self.cursor.col,
                end_line: self.cursor.line,
                end_col: self.cursor.col,
            });
        }
    }

    /// Update the selection end to current cursor position
    pub fn update_selection_end(&mut self) {
        if let Some(ref mut sel) = self.selection {
            sel.end_line = self.cursor.line;
            sel.end_col = self.cursor.col;
        }
    }

    /// Delete selection and return deleted text, or None if no selection
    pub fn delete_selection_text(&mut self) -> Option<String> {
        let sel = self.normalized_selection()?;
        let text = self.buffer.get_range(sel.start_line, sel.start_col, sel.end_line, sel.end_col);
        self.buffer.delete_range(sel.start_line, sel.start_col, sel.end_line, sel.end_col);
        self.cursor.line = sel.start_line;
        self.cursor.col = sel.start_col;
        self.selection = None;
        self.is_dirty = true;
        self.diagnostics_dirty = true;
        Some(text)
    }

    /// Select word at cursor position
    pub fn select_word_at_cursor(&mut self) {
        let line = self.buffer.get_line(self.cursor.line);
        let chars: Vec<char> = line.chars().collect();
        if chars.is_empty() { return; }
        let mut start = self.cursor.col.min(chars.len().saturating_sub(1));
        let mut end = start;
        while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_') {
            start -= 1;
        }
        while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
            end += 1;
        }
        if start < end {
            self.selection = Some(Selection {
                start_line: self.cursor.line, start_col: start,
                end_line: self.cursor.line, end_col: end,
            });
            self.cursor.col = end;
        }
    }

    /// Select entire current line
    pub fn select_line(&mut self) {
        let line_len = self.buffer.line_len(self.cursor.line);
        self.selection = Some(Selection {
            start_line: self.cursor.line, start_col: 0,
            end_line: self.cursor.line, end_col: line_len,
        });
    }

    pub fn get_selected_text(&self) -> Option<String> {
        let sel = self.selection.as_ref()?;
        Some(self.buffer.get_range(sel.start_line, sel.start_col, sel.end_line, sel.end_col))
    }

    pub fn toggle_comment(&mut self) {
        // Simple line comment toggle
        let line = self.buffer.get_line(self.cursor.line);
        let trimmed = line.trim_start();
        if trimmed.starts_with("// ") {
            let prefix_len = line.len() - trimmed.len();
            // Remove "// "
            for _ in 0..3 {
                self.buffer.delete_char(self.cursor.line, prefix_len);
            }
            if self.cursor.col >= prefix_len + 3 {
                self.cursor.col -= 3;
            }
        } else if trimmed.starts_with("//") {
            let prefix_len = line.len() - trimmed.len();
            for _ in 0..2 {
                self.buffer.delete_char(self.cursor.line, prefix_len);
            }
            if self.cursor.col >= prefix_len + 2 {
                self.cursor.col -= 2;
            }
        } else if trimmed.starts_with("# ") {
            let prefix_len = line.len() - trimmed.len();
            for _ in 0..2 {
                self.buffer.delete_char(self.cursor.line, prefix_len);
            }
            if self.cursor.col >= prefix_len + 2 {
                self.cursor.col -= 2;
            }
        } else {
            // Detect comment style from file extension
            let comment = if let Some(ref path) = self.file_path {
                match std::path::Path::new(path)
                    .extension()
                    .and_then(|e| e.to_str())
                {
                    Some("py" | "sh" | "bash" | "zsh" | "yml" | "yaml" | "toml" | "rb") => "# ",
                    Some("html" | "xml" | "svg") => "<!-- ",
                    Some("css" | "scss") => "/* ",
                    _ => "// ",
                }
            } else {
                "// "
            };
            let prefix_len = line.len() - trimmed.len();
            for (i, c) in comment.chars().enumerate() {
                self.buffer.insert_char(self.cursor.line, prefix_len + i, c);
            }
            self.cursor.col += comment.len();
        }
        self.is_dirty = true; self.diagnostics_dirty = true;
    }

    fn maybe_save_undo(&mut self) {
        self.undo_counter += 1;
        // Save snapshot every 10 edits
        if self.undo_counter % 10 == 0 {
            self.save_undo_snapshot();
        }
    }

    /// Snapshot for undo, but at most once per typing burst.
    ///
    /// The typing path called `save_undo_snapshot` for *every* character, and a
    /// snapshot is a full `buffer.text()` clone plus a full comparison against
    /// the previous one — O(file size) per keystroke, and up to 200 whole copies
    /// of the file resident in the undo stack. Coalescing gives editor-style
    /// undo granularity (one step per typing burst) at a fraction of the cost.
    pub fn save_undo_snapshot_coalesced(&mut self) {
        const COALESCE_MS: u128 = 400;
        if let Some(t) = self.last_snapshot_time {
            if t.elapsed().as_millis() < COALESCE_MS {
                return;
            }
        }
        self.save_undo_snapshot();
    }

    pub fn save_undo_snapshot(&mut self) {
        let content = self.buffer.text();
        self.last_snapshot_time = Some(std::time::Instant::now());
        // Don't save duplicate snapshots
        if let Some(last) = self.undo_stack.last() {
            if last.0 == content {
                return;
            }
        }
        let cursor_line = self.cursor.line;
        let cursor_col = self.cursor.col;
        // Bound *memory*, not just depth: each entry is a full copy of the file,
        // so 200 snapshots of a 5 MB file would be a gigabyte of history.
        let max_snapshots = match content.len() {
            0..=100_000 => 200,
            100_001..=1_000_000 => 60,
            _ => 20,
        };
        self.undo_stack.push((content, cursor_line, cursor_col));
        // Clear redo stack on new edit
        self.redo_stack.clear();
        if self.undo_stack.len() > max_snapshots {
            let excess = self.undo_stack.len() - max_snapshots;
            self.undo_stack.drain(..excess);
        }
    }

    pub fn undo(&mut self) {
        let current = self.buffer.text();
        // Save current state to redo stack
        if let Some(last) = self.undo_stack.last() {
            if last.0 != current {
                self.undo_stack.push((current.clone(), self.cursor.line, self.cursor.col));
            }
        }
        if self.undo_stack.len() > 1 {
            if let Some(popped) = self.undo_stack.pop() {
                self.redo_stack.push(popped);
            }
            if let Some((content, line, col)) = self.undo_stack.last().cloned() {
                self.buffer.rope = ropey::Rope::from_str(&content);
                self.cursor.line = line.min(self.buffer.line_count().saturating_sub(1));
                self.cursor.col = col;
                self.clamp_cursor();
                self.is_dirty = true; self.diagnostics_dirty = true;
            }
        }
    }

    pub fn redo(&mut self) {
        if let Some((content, line, col)) = self.redo_stack.pop() {
            // Save current to undo stack
            let current = self.buffer.text();
            self.undo_stack.push((current, self.cursor.line, self.cursor.col));
            // Restore redo state
            self.buffer.rope = ropey::Rope::from_str(&content);
            self.cursor.line = line.min(self.buffer.line_count().saturating_sub(1));
            self.cursor.col = col;
            self.clamp_cursor();
            self.is_dirty = true; self.diagnostics_dirty = true;
        }
    }

    pub fn outdent(&mut self) {
        let line = self.buffer.get_line(self.cursor.line);
        let spaces_to_remove = {
            let leading: usize = line.chars().take_while(|c| *c == ' ').count();
            leading.min(4)
        };
        if spaces_to_remove > 0 {
            for _ in 0..spaces_to_remove {
                self.buffer.delete_char(self.cursor.line, 0);
            }
            self.cursor.col = self.cursor.col.saturating_sub(spaces_to_remove);
            self.is_dirty = true; self.diagnostics_dirty = true;
        }
    }

    fn clamp_cursor(&mut self) {
        let line_len = self.buffer.line_len(self.cursor.line);
        if self.cursor.col > line_len {
            self.cursor.col = line_len;
        }
    }

    pub fn line_count(&self) -> usize {
        self.buffer.line_count()
    }

    /// Collect all words from the buffer for autocomplete.
    ///
    /// This runs on *every keystroke*, so it must not copy the file: the old
    /// version called `buffer.text()` first, i.e. a full-buffer allocation per
    /// character typed. Here the rope is scanned chunk by chunk with the fast
    /// `str::split` path, carrying words that straddle a chunk boundary. Past
    /// `FULL_SCAN_LIMIT` characters the scan is confined to a window around the
    /// cursor so typing in a multi-megabyte file stays responsive.
    pub fn collect_words(&self, prefix: &str) -> Vec<String> {
        const FULL_SCAN_LIMIT: usize = 200_000; // characters
        const WINDOW_LINES: usize = 2_000;

        let rope = &self.buffer.rope;
        let slice = if rope.len_chars() <= FULL_SCAN_LIMIT {
            rope.slice(..)
        } else {
            let lc = rope.len_lines();
            let first = self.cursor.line.saturating_sub(WINDOW_LINES);
            let last = self.cursor.line + WINDOW_LINES;
            let end = if last + 1 >= lc { rope.len_chars() } else { rope.line_to_char(last + 1) };
            rope.slice(rope.line_to_char(first.min(lc.saturating_sub(1)))..end)
        };

        let is_boundary = |c: char| !(c.is_alphanumeric() || c == '_');
        let mut words: HashSet<String> = HashSet::new();
        let keep = |w: &str, words: &mut HashSet<String>| {
            if w.len() >= 2 && w != prefix && w.starts_with(prefix) {
                words.insert(w.to_string());
            }
        };
        // A word can straddle a chunk boundary, so the trailing fragment of one
        // chunk is joined to the leading fragment of the next.
        let mut carry = String::new();
        for chunk in slice.chunks() {
            let mut parts = chunk.split(is_boundary).peekable();
            if let Some(first) = parts.next() {
                if parts.peek().is_none() {
                    carry.push_str(first); // whole chunk is one fragment
                    continue;
                }
                carry.push_str(first);
                keep(&carry, &mut words);
                carry.clear();
            }
            while let Some(part) = parts.next() {
                if parts.peek().is_none() {
                    carry.push_str(part); // may continue into the next chunk
                } else {
                    keep(part, &mut words);
                }
            }
        }
        keep(&carry, &mut words);

        let mut result: Vec<String> = words.into_iter().collect();
        result.sort();
        result.truncate(12);
        result
    }

    /// Get the word prefix at cursor position (for autocomplete)
    pub fn word_at_cursor(&self) -> String {
        let line = self.buffer.get_line(self.cursor.line);
        let chars: Vec<char> = line.chars().collect();
        let mut start = self.cursor.col;
        while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_') {
            start -= 1;
        }
        chars[start..self.cursor.col].iter().collect()
    }

    /// Compute line-by-line diff against original content
    pub fn compute_line_diff(&mut self) {
        self.line_diff.clear();
        let original = match &self.original_content {
            Some(c) => c,
            None => return,
        };
        let lc = self.buffer.line_count();
        // Only diff visible range + margin to avoid O(n) per frame
        let so = self.scroll_offset as usize;
        let start = so.saturating_sub(5);
        let end = (so + self.viewport_height + 10).min(lc);
        // Walk the baseline's lines lazily. Collecting them into a `Vec<&str>`
        // first allocated one entry per line of the whole file every time this
        // ran (once a second while the buffer is dirty).
        let mut orig = original.lines().skip(start);
        for li in start..end {
            match orig.next() {
                Some(orig_line) => {
                    if self.buffer.get_line(li) != orig_line {
                        self.line_diff.insert(li, LineDiffStatus::Modified);
                    }
                }
                None => {
                    self.line_diff.insert(li, LineDiffStatus::Added);
                }
            }
        }
    }

    /// Find next occurrence of word under cursor (for ⌘D)
    pub fn select_next_occurrence(&mut self) {
        let word = self.word_at_cursor();
        if word.is_empty() {
            // Select the word at cursor first
            let line = self.buffer.get_line(self.cursor.line);
            let chars: Vec<char> = line.chars().collect();
            let mut start = self.cursor.col;
            while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_') {
                start -= 1;
            }
            let mut end = self.cursor.col;
            while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
                end += 1;
            }
            if start < end {
                self.selection = Some(Selection {
                    start_line: self.cursor.line,
                    start_col: start,
                    end_line: self.cursor.line,
                    end_col: end,
                });
            }
            return;
        }
        // Search for next occurrence after current position
        let lc = self.buffer.line_count();
        let search_word = if let Some(ref sel) = self.selection {
            self.buffer.get_range(sel.start_line, sel.start_col, sel.end_line, sel.end_col)
        } else {
            word
        };
        if search_word.is_empty() { return; }

        // Cursor columns are *character* offsets while `str::find` works in
        // bytes — mixing them panicked on any line containing non-ASCII text
        // (Cyrillic, emoji, box-drawing…). Translate on both sides.
        let word_chars = search_word.chars().count();
        let find_from = |line: &str, from_char: usize| -> Option<usize> {
            let byte_start = if from_char == 0 {
                0
            } else {
                match line.char_indices().nth(from_char) {
                    Some((b, _)) => b,
                    None => return None,
                }
            };
            line[byte_start..]
                .find(search_word.as_str())
                .map(|pos| line[..byte_start + pos].chars().count())
        };

        // Search from cursor position
        for li in self.cursor.line..lc {
            let line = self.buffer.get_line(li);
            let start_col = if li == self.cursor.line { self.cursor.col + 1 } else { 0 };
            if let Some(col) = find_from(&line, start_col) {
                // Add current cursor as extra cursor
                self.extra_cursors.push(Cursor { line: self.cursor.line, col: self.cursor.col });
                self.cursor.line = li;
                self.cursor.col = col;
                self.selection = Some(Selection {
                    start_line: li,
                    start_col: col,
                    end_line: li,
                    end_col: col + word_chars,
                });
                self.scroll_into_view();
                return;
            }
        }
        // Wrap around from top
        for li in 0..self.cursor.line {
            let line = self.buffer.get_line(li);
            if let Some(col) = find_from(&line, 0) {
                self.extra_cursors.push(Cursor { line: self.cursor.line, col: self.cursor.col });
                self.cursor.line = li;
                self.cursor.col = col;
                self.selection = Some(Selection {
                    start_line: li,
                    start_col: col,
                    end_line: li,
                    end_col: col + word_chars,
                });
                self.scroll_into_view();
                return;
            }
        }
    }

    /// Compute fold ranges by matching { } brackets across lines
    /// Recompute fold ranges synchronously (tests / one-off callers). The editor
    /// normally gets folds from the async analysis worker; see `compute_folds`.
    #[cfg(test)]
    pub fn compute_fold_ranges(&mut self) {
        self.fold_ranges = compute_folds(&self.buffer.text());
        self.fold_ranges_computed = true;
    }

    /// Toggle fold at a given line
    pub fn toggle_fold(&mut self, line: usize) {
        if self.folded.contains(&line) {
            self.folded.remove(&line);
        } else if self.fold_ranges.contains_key(&line) {
            self.folded.insert(line);
        }
    }

    /// Check if a line is hidden by folding
    #[allow(dead_code)] // single-line query; `visible_lines` uses `hidden_intervals`
    pub fn is_line_folded(&self, line: usize) -> bool {
        if self.folded.is_empty() {
            return false;
        }
        for &start in &self.folded {
            if let Some(&end) = self.fold_ranges.get(&start) {
                if line > start && line <= end {
                    return true;
                }
            }
        }
        false
    }

    /// Folded regions as sorted, half-open-free `(first_hidden, last_hidden)`
    /// line intervals. Built once per query instead of re-scanning `fold_ranges`
    /// for every line.
    fn hidden_intervals(&self) -> Vec<(usize, usize)> {
        let mut hidden: Vec<(usize, usize)> = self
            .folded
            .iter()
            .filter_map(|s| self.fold_ranges.get(s).map(|&e| (s + 1, e)))
            .filter(|(s, e)| s <= e)
            .collect();
        hidden.sort_unstable();
        hidden
    }

    /// Get visible lines (skipping folded), returns Vec of actual line numbers
    pub fn visible_lines(&self, from: usize, count: usize) -> Vec<usize> {
        let lc = self.buffer.line_count();
        // Fast path: nothing is folded (the overwhelmingly common case), so the
        // visible window is simply the contiguous run starting at `from`. The
        // old code asked `is_line_folded` — itself O(fold_ranges) — for every
        // line from 0 up to the viewport, which made scrolling deep into a large
        // file quadratic *per frame*.
        if self.folded.is_empty() {
            let start = from.min(lc);
            return (start..lc.min(start + count)).collect();
        }

        let hidden = self.hidden_intervals();
        let mut result = Vec::with_capacity(count);
        let mut li = 0usize;
        let mut skipped = 0usize;
        let mut hi = 0usize;
        while li < lc && result.len() < count {
            while hi < hidden.len() && hidden[hi].1 < li {
                hi += 1;
            }
            if hi < hidden.len() && li >= hidden[hi].0 {
                // Jump over the whole folded block in one step.
                li = hidden[hi].1 + 1;
                continue;
            }
            if skipped < from {
                skipped += 1;
            } else {
                result.push(li);
            }
            li += 1;
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn editor_with_text(text: &str) -> Editor {
        let mut ed = Editor::new();
        for c in text.chars() {
            if c == '\n' {
                ed.insert_newline();
            } else {
                ed.insert_char(c);
            }
        }
        ed.cursor.line = 0;
        ed.cursor.col = 0;
        ed
    }

    #[test]
    fn new_editor_is_empty() {
        let ed = Editor::new();
        assert_eq!(ed.line_count(), 1);
        assert!(!ed.is_dirty);
        assert!(ed.file_path.is_none());
    }

    #[test]
    fn insert_char_marks_dirty() {
        let mut ed = Editor::new();
        ed.insert_char('a');
        assert!(ed.is_dirty);
        assert_eq!(ed.buffer.get_line(0), "a");
        assert_eq!(ed.cursor.col, 1);
    }

    #[test]
    fn insert_newline_moves_cursor() {
        let mut ed = Editor::new();
        ed.insert_char('A');
        ed.insert_newline();
        assert_eq!(ed.cursor.line, 1);
        assert_eq!(ed.cursor.col, 0);
        assert_eq!(ed.line_count(), 2);
    }

    #[test]
    fn move_up_down() {
        let mut ed = editor_with_text("line1\nline2\nline3");
        ed.cursor.line = 1;
        ed.cursor.col = 0;
        ed.move_up();
        assert_eq!(ed.cursor.line, 0);
        ed.move_down();
        assert_eq!(ed.cursor.line, 1);
        ed.move_down();
        assert_eq!(ed.cursor.line, 2);
        ed.move_down(); // Already at last line
        assert_eq!(ed.cursor.line, 2);
    }

    #[test]
    fn move_left_right() {
        let mut ed = editor_with_text("AB");
        ed.cursor.col = 0;
        ed.move_right();
        assert_eq!(ed.cursor.col, 1);
        ed.move_left();
        assert_eq!(ed.cursor.col, 0);
        ed.move_left(); // Already at start
        assert_eq!(ed.cursor.col, 0);
    }

    #[test]
    fn move_left_wraps_to_previous_line() {
        let mut ed = editor_with_text("AB\nCD");
        ed.cursor.line = 1;
        ed.cursor.col = 0;
        ed.move_left();
        assert_eq!(ed.cursor.line, 0);
        assert_eq!(ed.cursor.col, 2);
    }

    #[test]
    fn move_right_wraps_to_next_line() {
        let mut ed = editor_with_text("AB\nCD");
        ed.cursor.line = 0;
        ed.cursor.col = 2;
        ed.move_right();
        assert_eq!(ed.cursor.line, 1);
        assert_eq!(ed.cursor.col, 0);
    }

    #[test]
    fn delete_back_joins_lines() {
        let mut ed = editor_with_text("AB\nCD");
        ed.cursor.line = 1;
        ed.cursor.col = 0;
        ed.delete_back();
        assert_eq!(ed.line_count(), 1);
        assert_eq!(ed.buffer.get_line(0), "ABCD");
        assert_eq!(ed.cursor.col, 2);
    }

    #[test]
    fn select_all_and_get_text() {
        let mut ed = editor_with_text("Hello\nWorld");
        ed.select_all();
        assert!(ed.selection.is_some());
        let text = ed.get_selected_text().unwrap();
        assert_eq!(text, "Hello\nWorld");
    }

    #[test]
    fn undo_restores_state() {
        let mut ed = Editor::new();
        ed.save_undo_snapshot();
        ed.insert_char('A');
        ed.insert_char('B');
        ed.save_undo_snapshot();
        assert_eq!(ed.buffer.get_line(0), "AB");
        ed.undo();
        assert_eq!(ed.buffer.text(), "");
    }

    #[test]
    fn move_home_smart() {
        let mut ed = editor_with_text("    indented");
        ed.cursor.line = 0;
        ed.cursor.col = 8;
        ed.move_home(); // First press: go to first non-ws (col 4)
        assert_eq!(ed.cursor.col, 4);
        ed.move_home(); // Second press: go to col 0
        assert_eq!(ed.cursor.col, 0);
    }

    #[test]
    fn move_end() {
        let mut ed = editor_with_text("Hello");
        ed.cursor.col = 0;
        ed.move_end();
        assert_eq!(ed.cursor.col, 5);
    }

    #[test]
    fn file_name_returns_untitled_for_new() {
        let ed = Editor::new();
        assert_eq!(ed.file_name(), "untitled");
    }

    #[test]
    fn collect_words_basic() {
        let mut ed = editor_with_text("foo bar foo_bar baz foo");
        ed.cursor.col = 0;
        let words = ed.collect_words("fo");
        assert!(words.contains(&"foo".to_string()));
        assert!(words.contains(&"foo_bar".to_string()));
        assert!(!words.contains(&"bar".to_string()));
    }

    #[test]
    fn collect_words_windowed_large_buffer() {
        // Past the full-scan limit the scan is windowed around the cursor. It
        // must still surface nearby words and must not panic at either edge.
        let mut src = String::new();
        for i in 0..8_000 {
            src.push_str(&format!("let variable_{} = {};\n", i, i));
        }
        assert!(src.len() > 200_000, "fixture must exceed the full-scan limit");
        let mut ed = editor_with_raw(&src);
        ed.cursor.line = 0;
        assert!(ed.collect_words("variable_1").iter().any(|w| w == "variable_10"));
        ed.cursor.line = 7_999; // last line — window clamps at the end
        assert!(!ed.collect_words("variable_7").is_empty());
    }

    #[test]
    fn delete_line_works() {
        let mut ed = editor_with_text("A\nB\nC");
        ed.cursor.line = 1;
        ed.delete_line();
        assert_eq!(ed.line_count(), 2);
        assert_eq!(ed.buffer.get_line(0), "A");
        assert_eq!(ed.buffer.get_line(1), "C");
    }

    #[test]
    fn insert_tab_inserts_spaces() {
        let mut ed = Editor::new();
        ed.insert_tab();
        assert_eq!(ed.buffer.get_line(0), "    ");
        assert_eq!(ed.cursor.col, 4);
    }

    #[test]
    fn page_up_down() {
        let mut ed = editor_with_text(&"line\n".repeat(100));
        ed.viewport_height = 20;
        ed.cursor.line = 50;
        ed.page_up();
        assert!(ed.cursor.line < 50);
        let line_after_up = ed.cursor.line;
        ed.page_down();
        assert!(ed.cursor.line > line_after_up);
    }

    #[test]
    fn go_to_line() {
        let mut ed = editor_with_text(&"line\n".repeat(50));
        ed.go_to_line(25);
        assert_eq!(ed.cursor.line, 24); // 0-indexed
    }

    #[test]
    fn visual_rows_respects_wrap() {
        let mut ed = editor_with_text(&"x".repeat(25));
        assert_eq!(ed.line_visual_rows(0), 1); // wrap off by default
        ed.wrap_cols = 10;
        assert_eq!(ed.line_visual_rows(0), 3); // 25 chars / 10 cols → 3 rows
        assert_eq!(ed.cursor_visual_row(0, 0), 0);
        assert_eq!(ed.cursor_visual_row(0, 15), 1);
        assert_eq!(ed.cursor_visual_row(0, 24), 2);
    }

    #[test]
    fn scroll_into_view_counts_wrapped_rows() {
        // 10 logical lines, each wrapping into 3 visual rows = 30 visual rows.
        let mut ed = editor_with_text(&format!("{}\n", "x".repeat(25)).repeat(10));
        ed.viewport_height = 10; // 10 visual rows fit
        ed.wrap_cols = 10;       // each line is 3 rows tall
        ed.scroll_offset = 0.0;
        // Cursor on logical line 5 — only ~3 wrapped lines fit, so line 5 is well
        // below the fold and the view must scroll down (top line advances).
        ed.cursor.line = 5;
        ed.cursor.col = 0;
        ed.scroll_into_view();
        assert!(ed.scroll_offset.floor() as usize > 0, "expected scroll past the top");
        assert!((ed.scroll_offset.floor() as usize) <= 5);
    }

    // ── Selection tests (Zed-style) ──

    #[test]
    fn shift_selection_extend() {
        let mut ed = editor_with_text("Hello World");
        ed.cursor.col = 0;
        ed.ensure_selection_anchor();
        ed.move_right();
        ed.update_selection_end();
        let sel = ed.normalized_selection().unwrap();
        assert_eq!(sel.start_col, 0);
        assert_eq!(sel.end_col, 1);
    }

    #[test]
    fn selection_normalized_reversed() {
        let mut ed = editor_with_text("ABCDEF");
        ed.selection = Some(Selection {
            start_line: 0, start_col: 5,
            end_line: 0, end_col: 2,
        });
        let sel = ed.normalized_selection().unwrap();
        assert_eq!(sel.start_col, 2);
        assert_eq!(sel.end_col, 5);
    }

    #[test]
    fn delete_selection_text() {
        let mut ed = editor_with_text("Hello World");
        ed.selection = Some(Selection {
            start_line: 0, start_col: 5,
            end_line: 0, end_col: 11,
        });
        let deleted = ed.delete_selection_text();
        assert_eq!(deleted, Some(" World".to_string()));
        assert_eq!(ed.buffer.get_line(0), "Hello");
        assert!(ed.selection.is_none());
    }

    #[test]
    fn select_word_at_cursor() {
        let mut ed = editor_with_text("foo bar_baz qux");
        ed.cursor.col = 5; // middle of "bar_baz"
        ed.select_word_at_cursor();
        let sel = ed.normalized_selection().unwrap();
        assert_eq!(sel.start_col, 4);
        assert_eq!(sel.end_col, 11);
    }

    #[test]
    fn select_line() {
        let mut ed = editor_with_text("Hello\nWorld");
        ed.cursor.line = 0;
        ed.select_line();
        let sel = ed.normalized_selection().unwrap();
        assert_eq!(sel.start_col, 0);
        assert_eq!(sel.end_col, 5);
    }

    // ── Redo tests ──

    #[test]
    fn redo_works() {
        let mut ed = Editor::new();
        ed.save_undo_snapshot();
        ed.insert_char('A');
        ed.insert_char('B');
        ed.save_undo_snapshot();
        assert_eq!(ed.buffer.get_line(0), "AB");
        ed.undo();
        assert_eq!(ed.buffer.text(), "");
        ed.redo();
        assert_eq!(ed.buffer.get_line(0), "AB");
    }

    #[test]
    fn redo_cleared_on_new_edit() {
        let mut ed = Editor::new();
        ed.save_undo_snapshot();
        ed.insert_char('A');
        ed.save_undo_snapshot();
        ed.undo();
        // Now type something new — redo should be cleared
        ed.insert_char('X');
        ed.save_undo_snapshot();
        ed.undo();
        // Redo should go to "X", not "A"
        ed.redo();
        assert!(ed.buffer.text().contains('X'));
    }

    // ── Scroll margin test ──

    #[test]
    fn scroll_into_view_margin() {
        let mut ed = editor_with_text(&"line\n".repeat(100));
        ed.viewport_height = 20;
        ed.scroll_offset = 0.0;
        ed.cursor.line = 50;
        ed.scroll_into_view();
        let so = ed.scroll_offset as usize;
        assert!(so > 0);
        assert!(ed.cursor.line > so + 2);
        assert!(ed.cursor.line < so + ed.viewport_height - 2);
    }

    // ── Wheel scrolling ──

    const ROW_H: f32 = 22.0;

    #[test]
    fn scroll_up_from_the_bottom_works() {
        // Regression: once the last line reached the top of the viewport, the
        // bottom clamp zeroed out *upward* movement too, so the wheel could
        // never bring the earlier code back into view.
        let mut ed = editor_with_raw(&"line\n".repeat(500));
        let last = ed.line_count().saturating_sub(1);

        // Drive it to the very bottom the way repeated wheel events would.
        for _ in 0..200 {
            ed.scroll_by_pixels(2_000.0, ROW_H);
        }
        assert_eq!(ed.scroll_offset.floor() as usize, last, "should pin at the last line");

        // Now scroll back up.
        ed.scroll_by_pixels(-500.0, ROW_H);
        assert!(
            (ed.scroll_offset as usize) < last,
            "scrolling up from the bottom did not move (offset {})",
            ed.scroll_offset
        );

        // And all the way back to the top.
        for _ in 0..200 {
            ed.scroll_by_pixels(-2_000.0, ROW_H);
        }
        assert_eq!(ed.scroll_offset, 0.0);
    }

    #[test]
    fn scroll_round_trip_returns_to_start() {
        let mut ed = editor_with_raw(&"line\n".repeat(500));
        ed.scroll_by_pixels(1_000.0, ROW_H);
        let down = ed.scroll_offset;
        assert!(down > 0.0);
        ed.scroll_by_pixels(-1_000.0, ROW_H);
        assert!(ed.scroll_offset.abs() < 0.001, "expected 0, got {}", ed.scroll_offset);
    }

    #[test]
    fn scroll_round_trip_with_word_wrap() {
        // Wrapped lines are several rows tall; the walk must traverse them
        // symmetrically in both directions.
        let mut ed = editor_with_raw(&format!("{}\n", "x".repeat(25)).repeat(200));
        ed.wrap_cols = 10; // each line is 3 visual rows
        ed.scroll_by_pixels(1_000.0, ROW_H);
        let down = ed.scroll_offset;
        assert!(down > 0.0);
        ed.scroll_by_pixels(-1_000.0, ROW_H);
        assert!(ed.scroll_offset.abs() < 0.001, "expected 0, got {}", ed.scroll_offset);
    }

    #[test]
    fn scroll_never_passes_the_ends() {
        let mut ed = editor_with_raw(&"line\n".repeat(50));
        let last = ed.line_count().saturating_sub(1);
        ed.scroll_by_pixels(-10_000.0, ROW_H);
        assert_eq!(ed.scroll_offset, 0.0, "scrolled above the first line");
        ed.scroll_by_pixels(100_000.0, ROW_H);
        assert!(
            ed.scroll_offset >= last as f32 && ed.scroll_offset < last as f32 + 1.0,
            "scrolled past the last line: {}",
            ed.scroll_offset
        );
    }

    #[test]
    fn scroll_is_monotonic() {
        // Every downward step must move forward (or stay pinned at the end),
        // and every upward step backward — no oscillation between the carry
        // loops.
        let mut ed = editor_with_raw(&"line\n".repeat(300));
        let mut prev = ed.scroll_offset;
        for _ in 0..40 {
            ed.scroll_by_pixels(60.0, ROW_H);
            assert!(ed.scroll_offset >= prev, "went backwards while scrolling down");
            prev = ed.scroll_offset;
        }
        for _ in 0..40 {
            ed.scroll_by_pixels(-60.0, ROW_H);
            assert!(ed.scroll_offset <= prev, "went forwards while scrolling up");
            prev = ed.scroll_offset;
        }
        assert_eq!(ed.scroll_offset, 0.0);
    }

    // ── Fold tests ──

    #[test]
    fn compute_fold_ranges_basic() {
        let mut ed = editor_with_text("fn main() {\n    let x = 1;\n}");
        ed.compute_fold_ranges();
        assert!(ed.fold_ranges.contains_key(&0)); // line 0 has fold to line 2
        assert_eq!(*ed.fold_ranges.get(&0).unwrap(), 2);
    }

    #[test]
    fn toggle_fold_hides_lines() {
        let mut ed = editor_with_text("fn main() {\n    let x = 1;\n    let y = 2;\n}");
        ed.compute_fold_ranges();
        ed.toggle_fold(0);
        assert!(ed.folded.contains(&0));
        assert!(ed.is_line_folded(1));
        assert!(ed.is_line_folded(2));
        assert!(!ed.is_line_folded(0)); // fold start itself not hidden
        // Unfold
        ed.toggle_fold(0);
        assert!(!ed.folded.contains(&0));
        assert!(!ed.is_line_folded(1));
    }

    // ── Indent detection test ──

    fn editor_with_raw(text: &str) -> Editor {
        let mut ed = Editor::new();
        ed.buffer.rope = ropey::Rope::from_str(text);
        ed
    }

    // ── visible_lines / folding ──

    #[test]
    fn visible_lines_unfolded_is_contiguous() {
        let ed = editor_with_raw(&"line\n".repeat(100));
        assert_eq!(ed.visible_lines(10, 5), vec![10, 11, 12, 13, 14]);
        // Asking past the end yields only what exists.
        let tail = ed.visible_lines(99, 10);
        assert!(tail.len() <= 2, "got {:?}", tail);
    }

    #[test]
    fn visible_lines_skips_folded_block() {
        let mut ed = editor_with_text("a {\nb\nc\n}\nd");
        ed.compute_fold_ranges();
        ed.toggle_fold(0); // hides lines 1..=3
        assert_eq!(ed.visible_lines(0, 10), vec![0, 4]);
        // `from` counts *visible* lines, so 1 starts at the line after the fold.
        assert_eq!(ed.visible_lines(1, 10), vec![4]);
    }

    #[test]
    fn visible_lines_matches_is_line_folded() {
        let mut ed = editor_with_text("a {\nb\nc\n}\nx {\ny\n}\nz");
        ed.compute_fold_ranges();
        ed.toggle_fold(0);
        ed.toggle_fold(4);
        let expected: Vec<usize> = (0..ed.line_count()).filter(|&l| !ed.is_line_folded(l)).collect();
        assert_eq!(ed.visible_lines(0, 100), expected);
    }

    // ── Undo coalescing ──

    #[test]
    fn coalesced_undo_still_restores() {
        let mut ed = Editor::new();
        ed.save_undo_snapshot();
        // A typing burst: only the first call snapshots, the rest are coalesced.
        for c in "hello".chars() {
            ed.save_undo_snapshot_coalesced();
            ed.insert_char(c);
        }
        assert_eq!(ed.buffer.get_line(0), "hello");
        ed.undo();
        assert_eq!(ed.buffer.text(), "");
    }

    // ── UTF-8 safety ──

    #[test]
    fn select_next_occurrence_handles_cyrillic() {
        // Byte-vs-char confusion here used to panic on a multibyte line.
        let mut ed = editor_with_text("привет мир\nпривет снова");
        ed.cursor.line = 0;
        ed.cursor.col = 0;
        ed.select_word_at_cursor();
        ed.select_next_occurrence();
        assert_eq!(ed.cursor.line, 1);
        assert_eq!(ed.cursor.col, 0);
        let sel = ed.normalized_selection().unwrap();
        assert_eq!(sel.end_col, 6); // "привет" = 6 characters
    }

    #[test]
    fn duplicate_line_handles_cyrillic() {
        let mut ed = editor_with_text("привет\nмир");
        ed.cursor.line = 0;
        ed.duplicate_line();
        assert_eq!(ed.buffer.get_line(0), "привет");
        assert_eq!(ed.buffer.get_line(1), "привет");
        assert_eq!(ed.buffer.get_line(2), "мир");
    }

    #[test]
    fn line_chars_matches_get_line() {
        let ed = editor_with_raw("ascii\nкириллица\n\ttab\n");
        for li in 0..ed.line_count() {
            let via_iter: String = ed.buffer.line_chars(li).collect();
            assert_eq!(via_iter, ed.buffer.get_line(li), "line {}", li);
            let mut buf = Vec::new();
            ed.buffer.line_chars_into(li, usize::MAX, &mut buf);
            assert_eq!(buf.iter().collect::<String>(), ed.buffer.get_line(li));
        }
    }

    #[test]
    fn detect_indent_two_spaces() {
        let ed = editor_with_raw("  a\n  b\n  c");
        assert_eq!(ed.detect_indent(), 2);
    }

    #[test]
    fn detect_indent_four_spaces() {
        let ed = editor_with_raw("    a\n    b\n    c");
        assert_eq!(ed.detect_indent(), 4);
    }

    #[test]
    fn detect_indent_mixed() {
        let ed = editor_with_raw("    a\n    b\n  c\n    d\n    e");
        assert_eq!(ed.detect_indent(), 4);
    }
}
