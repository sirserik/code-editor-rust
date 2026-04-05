mod buffer;
mod cursor;

pub use buffer::Buffer;
pub use cursor::Cursor;

use ropey::Rope;
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
    pub scroll_offset: usize,
    pub file_path: Option<String>,
    pub is_dirty: bool,
    pub viewport_height: usize,
    pub viewport_width: usize,
    pub selection: Option<Selection>,
    // Simple undo: store snapshots
    undo_stack: Vec<(String, usize, usize)>, // (content, cursor_line, cursor_col)
    undo_counter: usize, // track changes for periodic snapshots
    // Code folding: maps fold start line -> fold end line
    pub fold_ranges: HashMap<usize, usize>,
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
            scroll_offset: 0,
            file_path: None,
            is_dirty: false,
            viewport_height: 24,
            viewport_width: 80,
            selection: None,
            undo_stack: Vec::new(),
            undo_counter: 0,
            fold_ranges: HashMap::new(),
            folded: HashSet::new(),
            diagnostics: Vec::new(),
            diagnostics_dirty: true,
            line_diff: HashMap::new(),
            original_content: None,
            last_edit_time: None,
            extra_cursors: Vec::new(),
        }
    }

    pub fn from_file(path: &str) -> std::io::Result<Self> {
        let buffer = Buffer::from_file(path)?;
        let initial_content = buffer.text();
        Ok(Self {
            buffer,
            cursor: Cursor::new(),
            scroll_offset: 0,
            file_path: Some(path.to_string()),
            is_dirty: false,
            viewport_height: 24,
            viewport_width: 80,
            selection: None,
            undo_stack: vec![(initial_content.clone(), 0, 0)],
            undo_counter: 0,
            fold_ranges: HashMap::new(),
            folded: HashSet::new(),
            diagnostics: Vec::new(),
            diagnostics_dirty: true,
            line_diff: HashMap::new(),
            original_content: Some(initial_content),
            last_edit_time: None,
            extra_cursors: Vec::new(),
        })
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
        for _ in 0..4 {
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
        self.scroll_offset = 0;
    }

    pub fn move_to_bottom(&mut self) {
        self.cursor.line = self.buffer.line_count().saturating_sub(1);
        self.cursor.col = 0;
    }

    pub fn page_up(&mut self) {
        let jump = self.viewport_height.saturating_sub(2);
        self.cursor.line = self.cursor.line.saturating_sub(jump);
        self.clamp_cursor();
    }

    pub fn page_down(&mut self) {
        let jump = self.viewport_height.saturating_sub(2);
        self.cursor.line = (self.cursor.line + jump).min(self.buffer.line_count().saturating_sub(1));
        self.clamp_cursor();
    }

    pub fn scroll_into_view(&mut self) {
        if self.cursor.line < self.scroll_offset {
            self.scroll_offset = self.cursor.line;
        }
        if self.cursor.line >= self.scroll_offset + self.viewport_height {
            self.scroll_offset = self.cursor.line - self.viewport_height + 1;
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

    pub fn get_selected_text(&self) -> Option<String> {
        let sel = self.selection.as_ref()?;
        Some(self.buffer.get_range(sel.start_line, sel.start_col, sel.end_line, sel.end_col))
    }

    pub fn delete_selection(&mut self) {
        if let Some(sel) = self.selection.take() {
            self.buffer.delete_range(sel.start_line, sel.start_col, sel.end_line, sel.end_col);
            self.cursor.line = sel.start_line;
            self.cursor.col = sel.start_col;
            self.is_dirty = true; self.diagnostics_dirty = true;
        }
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
        // Limit stack size — use swap_remove-like drain for efficiency
        if self.undo_stack.len() > 200 {
            self.undo_stack.drain(..50);
        }
    }

    pub fn undo(&mut self) {
        // Save current state first if it differs
        let current = self.buffer.text();
        if let Some(last) = self.undo_stack.last() {
            if last.0 != current {
                self.undo_stack.push((current, self.cursor.line, self.cursor.col));
            }
        }
        // Pop current, restore previous
        if self.undo_stack.len() > 1 {
            self.undo_stack.pop(); // remove current
            if let Some((content, line, col)) = self.undo_stack.last().cloned() {
                self.buffer.rope = ropey::Rope::from_str(&content);
                self.cursor.line = line.min(self.buffer.line_count().saturating_sub(1));
                self.cursor.col = col;
                self.clamp_cursor();
                self.is_dirty = true; self.diagnostics_dirty = true;
            }
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

    pub fn rope(&self) -> &Rope {
        &self.buffer.rope
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
        for li in 0..lc {
            let current = self.buffer.get_line(li);
            if li >= orig_lines.len() {
                self.line_diff.insert(li, LineDiffStatus::Added);
            } else if current != orig_lines[li] {
                self.line_diff.insert(li, LineDiffStatus::Modified);
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
    pub fn compute_fold_ranges(&mut self) {
        self.fold_ranges.clear();
        let lc = self.buffer.line_count();
        let mut stack: Vec<usize> = Vec::new(); // stack of line numbers where '{' opened

        for li in 0..lc {
            let line = self.buffer.get_line(li);
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
                                self.fold_ranges.insert(start, li);
                            }
                        }
                    }
                }
                prev = ch;
            }
        }
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
}
