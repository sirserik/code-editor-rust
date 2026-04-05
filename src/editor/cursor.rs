#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Cursor {
    pub line: usize,
    pub col: usize,
}

impl Cursor {
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_cursor_at_origin() {
        let c = Cursor::new();
        assert_eq!(c.line, 0);
        assert_eq!(c.col, 0);
    }

    #[test]
    fn cursor_equality() {
        let a = Cursor { line: 5, col: 10 };
        let b = Cursor { line: 5, col: 10 };
        let c = Cursor { line: 5, col: 11 };
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn cursor_clone() {
        let a = Cursor { line: 3, col: 7 };
        let b = a.clone();
        assert_eq!(a, b);
    }
}
