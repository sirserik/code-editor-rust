use regex::Regex;
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub file_path: String,
    pub file_name: String,
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
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        for (line_idx, line) in content.lines().enumerate() {
            for mat in pattern.find_iter(line) {
                results.push(SearchResult {
                    file_path: file_path.clone(),
                    file_name: file_name.clone(),
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
