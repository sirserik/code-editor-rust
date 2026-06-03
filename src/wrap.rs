//! Soft word-wrap layout, shared by the renderer (`gui::editor_view`) and the
//! editor model (`editor`) so that wrapping, click mapping, and scrolling all
//! agree on how a logical line splits into visual rows.

/// Maximum characters considered per line when wrapping — mirrors the render
/// cap so wrap layout and painting never disagree on a pathological long line.
pub const MAX_LINE_LEN: usize = crate::gui::MAX_LINE_LEN;

/// Soft-wrap a single logical line into visual segments.
///
/// Fills `out` with the starting column of each wrapped segment; `out[0]` is
/// always 0, so a line that fits in `wrap_cols` yields `[0]` (one segment).
/// Wrapping packs greedily and breaks at word boundaries (whitespace); a single
/// word longer than `wrap_cols` is hard-broken at the column limit.
/// `wrap_cols == usize::MAX` disables wrapping. No allocation: `out` is reused.
pub fn compute_wrap_starts(chars: &[char], wrap_cols: usize, out: &mut Vec<usize>) {
    out.clear();
    out.push(0);
    if wrap_cols == usize::MAX || wrap_cols == 0 || chars.len() <= wrap_cols {
        return;
    }
    let n = chars.len();
    let mut seg_start = 0usize;
    // `last_ws_after` = index just past the most recent whitespace inside the
    // current row (a candidate word-boundary break); 0 means "none yet".
    let mut last_ws_after = 0usize;
    let mut i = 0usize;
    while i < n {
        if i - seg_start == wrap_cols {
            // Row is full. Pick the break that keeps words intact (greedy pack).
            let brk = if chars[i].is_whitespace() {
                // The boundary itself is whitespace — the row's last word fits;
                // break after the whitespace run so it stays on this row.
                let mut b = i;
                while b < n && chars[b].is_whitespace() {
                    b += 1;
                }
                b
            } else if last_ws_after > seg_start {
                last_ws_after // break at the last word boundary that fit
            } else {
                i // single word longer than the row — hard break
            };
            out.push(brk);
            seg_start = brk;
            last_ws_after = 0;
            i = brk;
            continue;
        }
        if chars[i].is_whitespace() {
            last_ws_after = i + 1;
        }
        i += 1;
    }
}

/// Number of visual rows the given characters occupy under `wrap_cols` (≥ 1).
pub fn visual_rows(chars: &[char], wrap_cols: usize, scratch: &mut Vec<usize>) -> usize {
    compute_wrap_starts(chars, wrap_cols, scratch);
    scratch.len()
}

/// Index of the visual row that column `col` falls on (0-based).
pub fn visual_row_of_col(chars: &[char], wrap_cols: usize, col: usize, scratch: &mut Vec<usize>) -> usize {
    compute_wrap_starts(chars, wrap_cols, scratch);
    let mut sidx = 0;
    while sidx + 1 < scratch.len() && col >= scratch[sidx + 1] {
        sidx += 1;
    }
    sidx
}

#[cfg(test)]
mod tests {
    use super::*;

    fn starts(s: &str, cols: usize) -> Vec<usize> {
        let chars: Vec<char> = s.chars().collect();
        let mut out = Vec::new();
        compute_wrap_starts(&chars, cols, &mut out);
        out
    }

    #[test]
    fn short_line_is_one_segment() {
        assert_eq!(starts("hello", 10), vec![0]);
        assert_eq!(starts("", 10), vec![0]);
        // exactly at the limit must not wrap
        assert_eq!(starts("0123456789", 10), vec![0]);
    }

    #[test]
    fn wrap_off_never_splits() {
        let long = "x".repeat(5000);
        assert_eq!(starts(&long, usize::MAX), vec![0]);
    }

    #[test]
    fn wraps_at_word_boundary() {
        // "the quick" (9 chars) fits exactly in cols=9, break after the space.
        assert_eq!(starts("the quick brown", 9), vec![0, 10]);
    }

    #[test]
    fn long_word_is_hard_broken() {
        // No whitespace → hard break exactly every `cols` chars.
        assert_eq!(starts("abcdefghijkl", 5), vec![0, 5, 10]);
    }

    #[test]
    fn row_of_col_and_count() {
        let chars: Vec<char> = "abcdefghijkl".chars().collect();
        let mut s = Vec::new();
        assert_eq!(visual_rows(&chars, 5, &mut s), 3); // [0,5,10]
        assert_eq!(visual_row_of_col(&chars, 5, 3, &mut s), 0);
        assert_eq!(visual_row_of_col(&chars, 5, 7, &mut s), 1);
        assert_eq!(visual_row_of_col(&chars, 5, 11, &mut s), 2);
    }
}
