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
    /// Dark/light mode the highlight cache was built under. Syntect bakes
    /// absolute RGB into each span, so a theme switch must rebuild the cache —
    /// otherwise dark colors get painted on the light background (unreadable).
    pub highlight_cache_dark: Option<bool>,
    pub highlight_dirty_from: Option<usize>, // re-highlight from this line
    // Async analysis: the whole-buffer syntect pass + fold scan run on a worker
    // thread so opening/editing a file never blocks the UI. `highlight_gen` is
    // bumped whenever a re-analysis is needed; the worker tags its result with the
    // gen it ran for, and a stale result (older gen) is discarded. Coalesces bursts.
    // Payload: (gen, per-line highlight spans, fold-start→fold-end ranges).
    #[allow(clippy::type_complexity)]
    pub highlight_rx: Option<std::sync::mpsc::Receiver<(u64, Vec<Vec<crate::syntax::HighlightSpan>>, HashMap<usize, usize>)>>,
    pub highlight_gen: u64,
    pub highlight_applied_gen: u64,
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
            highlight_cache_dark: None,
            highlight_dirty_from: Some(0),
            highlight_rx: None,
            highlight_gen: 0,
            highlight_applied_gen: 0,
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
            highlight_cache_dark: None,
            highlight_dirty_from: Some(0),
            highlight_rx: None,
            highlight_gen: 0,
            highlight_applied_gen: 0,
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
            let line = self.buffer.get_line(li);
            if line.is_empty() { continue; }
            let first_char = line.chars().next().unwrap_or(' ');
            if first_char == '\t' {
                tab_count += 1;
            } else if first_char == ' ' {
                let spaces = line.chars().take_while(|c| *c == ' ').count();
                if (2..=8).contains(&spaces) {
                    space_counts[spaces] += 1;
                }
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
    pub fn line_visual_rows(&self, line: usize) -> usize {
        if self.wrap_cols == usize::MAX {
            return 1;
        }
        let chars: Vec<char> = self.buffer.get_line(line).chars().take(crate::wrap::MAX_LINE_LEN).collect();
        let mut scratch = Vec::new();
        crate::wrap::visual_rows(&chars, self.wrap_cols, &mut scratch)
    }

    /// Which visual row (0-based) the cursor column sits on within its line.
    fn cursor_visual_row(&self, line: usize, col: usize) -> usize {
        if self.wrap_cols == usize::MAX {
            return 0;
        }
        let chars: Vec<char> = self.buffer.get_line(line).chars().take(crate::wrap::MAX_LINE_LEN).collect();
        let mut scratch = Vec::new();
        crate::wrap::visual_row_of_col(&chars, self.wrap_cols, col, &mut scratch)
    }

    pub fn page_up(&mut self) {
        // Move the cursor up by roughly one viewport, measured in visual rows so
        // a screenful of wrapped text equals a screenful of logical lines.
        let mut budget = self.viewport_height.saturating_sub(2);
        while budget > 0 && self.cursor.line > 0 {
            self.cursor.line -= 1;
            budget = budget.saturating_sub(self.line_visual_rows(self.cursor.line));
        }
        self.clamp_cursor();
    }

    pub fn page_down(&mut self) {
        let last = self.buffer.line_count().saturating_sub(1);
        let mut budget = self.viewport_height.saturating_sub(2);
        while budget > 0 && self.cursor.line < last {
            budget = budget.saturating_sub(self.line_visual_rows(self.cursor.line));
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

        // Scroll down: count visual rows from the top of the viewport to the
        // cursor's row; if it falls past the bottom margin, pull the top line
        // forward until the cursor (plus margin) fits.
        if self.cursor.line >= so {
            let mut rows = self.cursor_visual_row(self.cursor.line, self.cursor.col);
            for l in so..self.cursor.line {
                rows += self.line_visual_rows(l);
            }
            if rows + margin >= vp_rows {
                // Walk the top line downward, dropping its visual rows, until the
                // cursor's row sits within the viewport (leaving the margin).
                let target = rows + margin + 1 - vp_rows; // visual rows to drop off the top
                let mut dropped = 0;
                let mut top = so;
                while top < self.cursor.line && dropped < target {
                    dropped += self.line_visual_rows(top);
                    top += 1;
                }
                self.scroll_offset = top as f32;
            }
        }
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

    pub fn save_undo_snapshot(&mut self) {
        let content = self.buffer.text();
        // Don't save duplicate snapshots
        if let Some(last) = self.undo_stack.last() {
            if last.0 == content {
                return;
            }
        }
        let cursor_line = self.cursor.line;
        let cursor_col = self.cursor.col;
        self.undo_stack.push((content, cursor_line, cursor_col));
        // Clear redo stack on new edit
        self.redo_stack.clear();
        // Limit stack size
        if self.undo_stack.len() > 200 {
            self.undo_stack.drain(..50);
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

    /// Collect all words from the buffer for autocomplete
    pub fn collect_words(&self, prefix: &str) -> Vec<String> {
        let mut words = HashSet::new();
        let text = self.buffer.text();
        for word in text.split(|c: char| !c.is_alphanumeric() && c != '_') {
            if word.len() >= 2 && word != prefix && word.starts_with(prefix) {
                words.insert(word.to_string());
            }
        }
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
        let orig_lines: Vec<&str> = original.lines().collect();
        let lc = self.buffer.line_count();
        // Only diff visible range + margin to avoid O(n) per frame
        let so = self.scroll_offset as usize;
        let start = so.saturating_sub(5);
        let end = (so + self.viewport_height + 10).min(lc);
        for li in start..end {
            if li >= orig_lines.len() {
                self.line_diff.insert(li, LineDiffStatus::Added);
            } else {
                let current = self.buffer.get_line(li);
                if current != orig_lines[li] {
                    self.line_diff.insert(li, LineDiffStatus::Modified);
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

        // Search from cursor position
        for li in self.cursor.line..lc {
            let line = self.buffer.get_line(li);
            let start_col = if li == self.cursor.line { self.cursor.col + 1 } else { 0 };
            if let Some(pos) = line[start_col..].find(&search_word) {
                let col = start_col + pos;
                // Add current cursor as extra cursor
                self.extra_cursors.push(Cursor { line: self.cursor.line, col: self.cursor.col });
                self.cursor.line = li;
                self.cursor.col = col;
                self.selection = Some(Selection {
                    start_line: li,
                    start_col: col,
                    end_line: li,
                    end_col: col + search_word.len(),
                });
                self.scroll_into_view();
                return;
            }
        }
        // Wrap around from top
        for li in 0..self.cursor.line {
            let line = self.buffer.get_line(li);
            if let Some(pos) = line.find(&search_word) {
                self.extra_cursors.push(Cursor { line: self.cursor.line, col: self.cursor.col });
                self.cursor.line = li;
                self.cursor.col = pos;
                self.selection = Some(Selection {
                    start_line: li,
                    start_col: pos,
                    end_line: li,
                    end_col: pos + search_word.len(),
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
    pub fn is_line_folded(&self, line: usize) -> bool {
        for (&start, &end) in &self.fold_ranges {
            if self.folded.contains(&start) && line > start && line <= end {
                return true;
            }
        }
        false
    }

    /// Get visible lines (skipping folded), returns Vec of actual line numbers
    pub fn visible_lines(&self, from: usize, count: usize) -> Vec<usize> {
        let lc = self.buffer.line_count();
        let mut result = Vec::with_capacity(count);
        let mut li = 0;
        let mut skipped = 0;

        // First skip to `from` visible lines
        while li < lc && skipped < from {
            if !self.is_line_folded(li) {
                skipped += 1;
            }
            li += 1;
        }

        // Then collect `count` visible lines
        while li < lc && result.len() < count {
            if !self.is_line_folded(li) {
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
