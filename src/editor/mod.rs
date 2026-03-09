mod buffer;
mod cursor;

pub use buffer::Buffer;
pub use cursor::Cursor;

use ropey::Rope;
use std::collections::{HashMap, HashSet};

use crate::syntax::SyntaxError;

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
            undo_stack: vec![(initial_content, 0, 0)],
            undo_counter: 0,
            fold_ranges: HashMap::new(),
            folded: HashSet::new(),
            diagnostics: Vec::new(),
            diagnostics_dirty: true,
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
        let cursor_line = self.cursor.line;
        let cursor_col = self.cursor.col;
        // Don't save duplicate snapshots
        if let Some(last) = self.undo_stack.last() {
            if last.0 == content {
                return;
            }
        }
        self.undo_stack.push((content, cursor_line, cursor_col));
        // Limit stack size
        if self.undo_stack.len() > 200 {
            self.undo_stack.remove(0);
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
