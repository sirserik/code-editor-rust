use regex::Regex;
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub file_path: String,
    pub line_number: usize,
    pub line_content: String,
    pub match_start: usize,
    pub match_end: usize,
}

#[derive(Debug, Clone)]
pub struct FindMatch {
    pub line: usize,
    pub col: usize,
    pub length: usize,
}

const IGNORED_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    ".next",
    "__pycache__",
    ".venv",
    "vendor",
    "dist",
    "build",
    ".cache",
];

const BINARY_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "bmp", "ico", "svg", "webp", "mp3", "mp4", "avi", "mov", "wav",
    "zip", "tar", "gz", "rar", "7z", "exe", "dll", "so", "dylib", "bin", "dat", "db", "sqlite",
    "pdf", "doc", "docx", "xls", "xlsx", "woff", "woff2", "ttf", "eot", "otf",
];

pub fn search_in_project(
    root_path: &str,
    query: &str,
    case_sensitive: bool,
    use_regex: bool,
) -> Vec<SearchResult> {
    let pattern = if use_regex {
        if case_sensitive {
            Regex::new(query).ok()
        } else {
            Regex::new(&format!("(?i){}", query)).ok()
        }
    } else {
        let escaped = regex::escape(query);
        if case_sensitive {
            Regex::new(&escaped).ok()
        } else {
            Regex::new(&format!("(?i){}", escaped)).ok()
        }
    };

    let pattern = match pattern {
        Some(p) => p,
        None => return Vec::new(),
    };

    let mut results = Vec::new();

    for entry in WalkDir::new(root_path)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !IGNORED_DIRS.contains(&name.as_ref())
        })
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();

        // Skip binary files
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if BINARY_EXTENSIONS.contains(&ext.to_lowercase().as_str()) {
                continue;
            }
        }

        // Read file
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let file_path = path.to_string_lossy().to_string();

        for (line_idx, line) in content.lines().enumerate() {
            for mat in pattern.find_iter(line) {
                results.push(SearchResult {
                    file_path: file_path.clone(),
                    line_number: line_idx + 1,
                    line_content: line.to_string(),
                    match_start: mat.start(),
                    match_end: mat.end(),
                });

                if results.len() >= 1000 {
                    return results;
                }
            }
        }
    }

    results
}

#[derive(Debug, Clone)]
pub struct FileMatch {
    pub file_path: String,
    pub file_name: String,
    pub rel_path: String,
}

pub fn search_files_by_name(root_path: &str, query: &str) -> Vec<FileMatch> {
    let query_lower = query.to_lowercase();
    let mut results = Vec::new();

    for entry in WalkDir::new(root_path)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !IGNORED_DIRS.contains(&name.as_ref())
        })
        .filter_map(|e| e.ok())
    {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.to_lowercase().contains(&query_lower) {
            let file_path = entry.path().to_string_lossy().to_string();
            let rel_path = file_path.strip_prefix(root_path)
                .unwrap_or(&file_path).trim_start_matches('/').to_string();
            results.push(FileMatch {
                file_path,
                file_name: name,
                rel_path,
            });
            if results.len() >= 200 {
                break;
            }
        }
    }

    results
}

pub fn find_in_content(
    content: &str,
    query: &str,
    case_sensitive: bool,
    use_regex: bool,
) -> Vec<FindMatch> {
    let pattern = if use_regex {
        if case_sensitive {
            Regex::new(query).ok()
        } else {
            Regex::new(&format!("(?i){}", query)).ok()
        }
    } else {
        let escaped = regex::escape(query);
        if case_sensitive {
            Regex::new(&escaped).ok()
        } else {
            Regex::new(&format!("(?i){}", escaped)).ok()
        }
    };

    let pattern = match pattern {
        Some(p) => p,
        None => return Vec::new(),
    };

    let mut matches = Vec::new();

    for (line_idx, line) in content.lines().enumerate() {
        for mat in pattern.find_iter(line) {
            matches.push(FindMatch {
                line: line_idx,
                col: mat.start(),
                length: mat.end() - mat.start(),
            });
        }
    }

    matches
}

pub fn replace_in_content(
    content: &str,
    query: &str,
    replacement: &str,
    case_sensitive: bool,
    use_regex: bool,
    replace_all: bool,
) -> String {
    let pattern = if use_regex {
        if case_sensitive {
            Regex::new(query).ok()
        } else {
            Regex::new(&format!("(?i){}", query)).ok()
        }
    } else {
        let escaped = regex::escape(query);
        if case_sensitive {
            Regex::new(&escaped).ok()
        } else {
            Regex::new(&format!("(?i){}", escaped)).ok()
        }
    };

    match pattern {
        Some(p) => {
            if replace_all {
                p.replace_all(content, replacement).to_string()
            } else {
                p.replace(content, replacement).to_string()
            }
        }
        None => content.to_string(),
    }
}

#[derive(Debug, Default, Clone)]
pub struct ReplaceStats {
    pub files_changed: usize,
    pub replacements: usize,
    pub errors: Vec<String>,
}

/// Build the regex used by both search and replace. Unicode case folding is on by default
/// in the `regex` crate, so `(?i)привет` matches "ПРИВЕТ" / "Қалай" → "ҚАЛАЙ" etc.
fn build_pattern(query: &str, case_sensitive: bool, use_regex: bool) -> Option<Regex> {
    let body = if use_regex { query.to_string() } else { regex::escape(query) };
    if case_sensitive {
        Regex::new(&body).ok()
    } else {
        Regex::new(&format!("(?i){}", body)).ok()
    }
}

/// Project-wide replace. Walks the same file set as `search_in_project`, rewrites any file
/// where the pattern matched, and returns a summary the UI can show in the status bar.
pub fn replace_in_project(
    root_path: &str,
    query: &str,
    replacement: &str,
    case_sensitive: bool,
    use_regex: bool,
) -> ReplaceStats {
    let mut stats = ReplaceStats::default();
    let pattern = match build_pattern(query, case_sensitive, use_regex) {
        Some(p) => p,
        None => {
            stats.errors.push("Invalid pattern".to_string());
            return stats;
        }
    };

    for entry in WalkDir::new(root_path)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !IGNORED_DIRS.contains(&name.as_ref())
        })
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if BINARY_EXTENSIONS.contains(&ext.to_lowercase().as_str()) {
                continue;
            }
        }
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue, // skip non-UTF8 / unreadable
        };
        let match_count = pattern.find_iter(&content).count();
        if match_count == 0 {
            continue;
        }
        let new_content = pattern.replace_all(&content, replacement).to_string();
        if new_content == content {
            continue; // pattern matched but expanded to itself — skip the disk write
        }
        match std::fs::write(path, &new_content) {
            Ok(_) => {
                stats.files_changed += 1;
                stats.replacements += match_count;
            }
            Err(e) => stats.errors.push(format!("{}: {}", path.display(), e)),
        }
    }

    stats
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_in_content_basic() {
        let matches = find_in_content("hello world\nhello again", "hello", false, false);
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].line, 0);
        assert_eq!(matches[0].col, 0);
        assert_eq!(matches[1].line, 1);
    }

    #[test]
    fn find_case_insensitive() {
        let matches = find_in_content("Hello HELLO hello", "hello", false, false);
        assert_eq!(matches.len(), 3);
    }

    #[test]
    fn find_case_sensitive() {
        let matches = find_in_content("Hello HELLO hello", "hello", true, false);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].col, 12);
    }

    #[test]
    fn find_with_regex() {
        let matches = find_in_content("foo123 bar456 baz", r"\w+\d+", false, true);
        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn find_no_matches() {
        let matches = find_in_content("hello world", "xyz", false, false);
        assert!(matches.is_empty());
    }

    #[test]
    fn replace_in_content_all() {
        let result = replace_in_content("foo bar foo", "foo", "baz", false, false, true);
        assert_eq!(result, "baz bar baz");
    }

    #[test]
    fn replace_in_content_single() {
        let result = replace_in_content("foo bar foo", "foo", "baz", false, false, false);
        assert_eq!(result, "baz bar foo");
    }

    #[test]
    fn replace_case_insensitive() {
        let result = replace_in_content("Foo FOO foo", "foo", "x", false, false, true);
        assert_eq!(result, "x x x");
    }

    #[test]
    fn invalid_regex_returns_empty() {
        let matches = find_in_content("hello", "[invalid", false, true);
        assert!(matches.is_empty());
    }

    // ─── Unicode coverage: Russian / Kazakh ───
    // The regex crate has Unicode case folding on by default, so case-insensitive search
    // must work on Cyrillic. The byte offsets we return must land on char boundaries
    // (regex always does this — these tests document the guarantee).

    #[test]
    fn find_russian_case_insensitive() {
        let m = find_in_content("Привет мир", "привет", false, false);
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].col, 0);
        // "Привет" = 6 chars × 2 bytes = 12 bytes
        assert_eq!(m[0].length, 12);
    }

    #[test]
    fn find_kazakh_specific_glyphs() {
        // Kazakh-only Cyrillic letters: Қ Ң Ұ Ү Ө Ғ І Ә Һ
        let m = find_in_content("Қалай жасайсыз? Қазақ тілі.", "қазақ", false, false);
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn find_russian_case_sensitive_misses() {
        let m = find_in_content("Привет МИР", "мир", true, false);
        assert!(m.is_empty());
    }

    #[test]
    fn replace_russian_preserves_case_with_regex_off() {
        let r = replace_in_content("Привет мир", "Привет", "Здравствуй", false, false, true);
        assert_eq!(r, "Здравствуй мир");
    }

    #[test]
    fn replace_kazakh_all() {
        let r = replace_in_content("Қазақстан, Қазақстан!", "Қазақстан", "Алматы", false, false, true);
        assert_eq!(r, "Алматы, Алматы!");
    }

    #[test]
    fn byte_offsets_land_on_char_boundaries() {
        // If the regex ever returned mid-codepoint offsets, slicing would panic. This test
        // succeeds only if `mat.start()` / `mat.end()` are char-aligned for multibyte text.
        let content = "Привет, world!";
        let m = find_in_content(content, "world", false, false);
        assert_eq!(m.len(), 1);
        let start = m[0].col;
        let end = start + m[0].length;
        // Slicing on the bytes must not panic.
        let slice = &content[start..end];
        assert_eq!(slice, "world");
    }
}
