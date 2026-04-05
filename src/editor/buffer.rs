use ropey::Rope;
use std::fs;

#[derive(Debug)]
pub struct Buffer {
    pub rope: Rope,
}

impl Buffer {
    pub fn new() -> Self {
        Self {
            rope: Rope::new(),
        }
    }

    pub fn from_file(path: &str) -> std::io::Result<Self> {
        let content = fs::read_to_string(path)?;
        Ok(Self {
            rope: Rope::from_str(&content),
        })
    }

    pub fn save(&self, path: &str) -> std::io::Result<()> {
        let content = self.rope.to_string();
        fs::write(path, content)
    }

    pub fn line_count(&self) -> usize {
        self.rope.len_lines().max(1)
    }

    pub fn line_len(&self, line: usize) -> usize {
        if line >= self.rope.len_lines() {
            return 0;
        }
        let line_slice = self.rope.line(line);
        let len = line_slice.len_chars();
        // Subtract newline character if present
        if len > 0 {
            let last = line_slice.char(len - 1);
            if last == '\n' || last == '\r' {
                if len > 1 && line_slice.char(len - 2) == '\r' {
                    len - 2
                } else {
                    len - 1
                }
            } else {
                len
            }
        } else {
            0
        }
    }

    pub fn get_line(&self, line: usize) -> String {
        if line >= self.rope.len_lines() {
            return String::new();
        }
        let line_slice = self.rope.line(line);
        let s = line_slice.to_string();
        s.trim_end_matches(&['\n', '\r'][..]).to_string()
    }

    pub fn get_line_indent(&self, line: usize) -> String {
        let content = self.get_line(line);
        content
            .chars()
            .take_while(|c| c.is_whitespace())
            .collect()
    }

    pub fn insert_char(&mut self, line: usize, col: usize, c: char) {
        let idx = self.line_col_to_char(line, col);
        self.rope.insert_char(idx, c);
    }

    pub fn insert_newline(&mut self, line: usize, col: usize) {
        let idx = self.line_col_to_char(line, col);
        self.rope.insert_char(idx, '\n');
    }

    pub fn delete_char(&mut self, line: usize, col: usize) {
        let idx = self.line_col_to_char(line, col);
        if idx < self.rope.len_chars() {
            self.rope.remove(idx..idx + 1);
        }
    }

    pub fn delete_line(&mut self, line: usize) {
        if line >= self.rope.len_lines() {
            return;
        }
        let start = self.rope.line_to_char(line);
        let end = if line + 1 < self.rope.len_lines() {
            self.rope.line_to_char(line + 1)
        } else {
            self.rope.len_chars()
        };
        if start < end {
            self.rope.remove(start..end);
        }
    }

    pub fn join_lines(&mut self, line: usize) {
        if line + 1 >= self.rope.len_lines() {
            return;
        }
        // Find the newline at end of this line and remove it
        let next_line_start = self.rope.line_to_char(line + 1);
        let newline_pos = next_line_start - 1;
        if newline_pos < self.rope.len_chars() {
            // Check for \r\n
            if newline_pos > 0 && self.rope.char(newline_pos - 1) == '\r' {
                self.rope.remove(newline_pos - 1..newline_pos + 1);
            } else {
                self.rope.remove(newline_pos..newline_pos + 1);
            }
        }
    }

    pub fn duplicate_line(&mut self, line: usize) {
        let content = self.get_line(line);
        let insert_pos = if line + 1 < self.rope.len_lines() {
            self.rope.line_to_char(line + 1)
        } else {
            let pos = self.rope.len_chars();
            self.rope.insert_char(pos, '\n');
            pos + 1
        };
        self.rope.insert(insert_pos, &content);
        if line + 2 >= self.rope.len_lines() || insert_pos + content.len() >= self.rope.len_chars()
        {
            self.rope.insert_char(insert_pos + content.len(), '\n');
        }
    }

    pub fn swap_lines(&mut self, a: usize, b: usize) {
        if a == b || a >= self.rope.len_lines() || b >= self.rope.len_lines() {
            return;
        }
        let line_a = self.get_line(a);
        let line_b = self.get_line(b);
        self.replace_line(a, &line_b);
        self.replace_line(b, &line_a);
    }

    fn replace_line(&mut self, line: usize, content: &str) {
        let start = self.rope.line_to_char(line);
        let end_offset = self.line_len(line);
        let end = start + end_offset;
        if end > start {
            self.rope.remove(start..end);
        }
        self.rope.insert(start, content);
    }

    pub fn get_range(
        &self,
        start_line: usize,
        start_col: usize,
        end_line: usize,
        end_col: usize,
    ) -> String {
        let start = self.line_col_to_char(start_line, start_col);
        let end = self.line_col_to_char(end_line, end_col);
        self.rope.slice(start..end).to_string()
    }

    pub fn delete_range(
        &mut self,
        start_line: usize,
        start_col: usize,
        end_line: usize,
        end_col: usize,
    ) {
        let start = self.line_col_to_char(start_line, start_col);
        let end = self.line_col_to_char(end_line, end_col);
        if end > start {
            self.rope.remove(start..end);
        }
    }

    pub fn insert_text(&mut self, line: usize, col: usize, text: &str) {
        let idx = self.line_col_to_char(line, col);
        self.rope.insert(idx, text);
    }

    fn line_col_to_char(&self, line: usize, col: usize) -> usize {
        if line >= self.rope.len_lines() {
            return self.rope.len_chars();
        }
        let line_start = self.rope.line_to_char(line);
        let line_len = self.line_len(line);
        line_start + col.min(line_len)
    }

    pub fn text(&self) -> String {
        self.rope.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_buffer_is_empty() {
        let buf = Buffer::new();
        assert_eq!(buf.line_count(), 1);
        assert_eq!(buf.text(), "");
    }

    #[test]
    fn insert_char_and_read() {
        let mut buf = Buffer::new();
        buf.insert_char(0, 0, 'H');
        buf.insert_char(0, 1, 'i');
        assert_eq!(buf.get_line(0), "Hi");
        assert_eq!(buf.line_len(0), 2);
    }

    #[test]
    fn insert_newline_splits_line() {
        let mut buf = Buffer::new();
        buf.insert_char(0, 0, 'A');
        buf.insert_char(0, 1, 'B');
        buf.insert_newline(0, 1); // Split between A and B
        assert_eq!(buf.line_count(), 2);
        assert_eq!(buf.get_line(0), "A");
        assert_eq!(buf.get_line(1), "B");
    }

    #[test]
    fn delete_char_works() {
        let mut buf = Buffer::new();
        buf.insert_char(0, 0, 'A');
        buf.insert_char(0, 1, 'B');
        buf.insert_char(0, 2, 'C');
        buf.delete_char(0, 1); // Delete 'B'
        assert_eq!(buf.get_line(0), "AC");
    }

    #[test]
    fn join_lines_works() {
        let mut buf = Buffer::new();
        buf.insert_char(0, 0, 'A');
        buf.insert_newline(0, 1);
        buf.insert_char(1, 0, 'B');
        assert_eq!(buf.line_count(), 2);
        buf.join_lines(0);
        assert_eq!(buf.line_count(), 1);
        assert_eq!(buf.get_line(0), "AB");
    }

    #[test]
    fn delete_line_works() {
        let mut buf = Buffer::new();
        buf.insert_char(0, 0, 'A');
        buf.insert_newline(0, 1);
        buf.insert_char(1, 0, 'B');
        buf.insert_newline(1, 1);
        buf.insert_char(2, 0, 'C');
        assert_eq!(buf.line_count(), 3);
        buf.delete_line(1); // Delete line "B"
        assert_eq!(buf.line_count(), 2);
        assert_eq!(buf.get_line(0), "A");
        assert_eq!(buf.get_line(1), "C");
    }

    #[test]
    fn swap_lines_works() {
        let mut buf = Buffer::new();
        buf.insert_char(0, 0, 'A');
        buf.insert_newline(0, 1);
        buf.insert_char(1, 0, 'B');
        buf.swap_lines(0, 1);
        assert_eq!(buf.get_line(0), "B");
        assert_eq!(buf.get_line(1), "A");
    }

    #[test]
    fn duplicate_line_works() {
        let mut buf = Buffer::new();
        buf.insert_char(0, 0, 'A');
        buf.insert_newline(0, 1);
        buf.insert_char(1, 0, 'B');
        // Now we have "A\nB" — 2 lines
        assert_eq!(buf.line_count(), 2);
        buf.duplicate_line(0); // Duplicate "A" line
        assert_eq!(buf.line_count(), 3);
        assert_eq!(buf.get_line(0), "A");
        assert_eq!(buf.get_line(1), "A");
        assert_eq!(buf.get_line(2), "B");
    }

    #[test]
    fn get_range_works() {
        let mut buf = Buffer::new();
        for c in "Hello".chars() {
            let col = buf.line_len(0);
            buf.insert_char(0, col, c);
        }
        assert_eq!(buf.get_range(0, 1, 0, 4), "ell");
    }

    #[test]
    fn delete_range_works() {
        let mut buf = Buffer::new();
        for c in "Hello".chars() {
            let col = buf.line_len(0);
            buf.insert_char(0, col, c);
        }
        buf.delete_range(0, 1, 0, 4); // Delete "ell"
        assert_eq!(buf.get_line(0), "Ho");
    }

    #[test]
    fn insert_text_works() {
        let mut buf = Buffer::new();
        buf.insert_text(0, 0, "Hello World");
        assert_eq!(buf.get_line(0), "Hello World");
        assert_eq!(buf.line_len(0), 11);
    }

    #[test]
    fn get_line_indent_works() {
        let mut buf = Buffer::new();
        buf.insert_text(0, 0, "    indented");
        assert_eq!(buf.get_line_indent(0), "    ");
    }
}
