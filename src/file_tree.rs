use std::cmp::Ordering;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: String,
    pub name: String,
    pub is_directory: bool,
    pub depth: usize,
    pub is_expanded: bool,
    pub children: Vec<FileEntry>,
}

impl FileEntry {
    pub fn new(path: String, name: String, is_directory: bool, depth: usize) -> Self {
        Self {
            path,
            name,
            is_directory,
            depth,
            is_expanded: false,
            children: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct FileTree {
    pub root: Option<FileEntry>,
    pub root_path: Option<String>,
    pub flat_entries: Vec<FlatEntry>,
    pub selected_index: usize,
    pub scroll_offset: usize,
    pub viewport_height: usize,
    pub show_hidden: bool,
}

#[derive(Debug, Clone)]
pub struct FlatEntry {
    pub path: String,
    pub name: String,
    pub is_directory: bool,
    pub depth: usize,
    pub is_expanded: bool,
}

/// Directories that are ALWAYS hidden (heavy/internal)
const ALWAYS_IGNORED: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    ".next",
    "__pycache__",
    ".venv",
    ".DS_Store",
    ".cache",
    "Thumbs.db",
];

impl FileTree {
    pub fn new() -> Self {
        Self {
            root: None,
            root_path: None,
            flat_entries: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
            viewport_height: 20,
            show_hidden: true, // Show hidden files by default
        }
    }

    pub fn load(&mut self, path: &str) {
        self.root_path = Some(path.to_string());
        let show_hidden = self.show_hidden;
        let root = Self::build_tree(path, 0, show_hidden);
        self.root = Some(root);
        self.flatten();
    }

    fn build_tree(path: &str, depth: usize, show_hidden: bool) -> FileEntry {
        let name = Path::new(path)
            .file_name()
            .map(|n: &std::ffi::OsStr| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string());

        let mut entry = FileEntry::new(path.to_string(), name, true, depth);

        if depth == 0 {
            entry.is_expanded = true;
        }

        // Only load immediate children (lazy loading — max 1 level deep)
        entry.children = Self::read_children(path, depth + 1, show_hidden);

        entry
    }

    fn read_children(path: &str, child_depth: usize, show_hidden: bool) -> Vec<FileEntry> {
        let Ok(read_dir) = std::fs::read_dir(path) else {
            return Vec::new();
        };

        let mut children: Vec<FileEntry> = read_dir
            .filter_map(|e| e.ok())
            .filter(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                if ALWAYS_IGNORED.contains(&name.as_str()) {
                    return false;
                }
                if !show_hidden && name.starts_with('.') {
                    return false;
                }
                true
            })
            .map(|e| {
                let p = e.path();
                let n = e.file_name().to_string_lossy().to_string();
                let is_dir = p.is_dir();
                // Don't recurse — just create the entry (children loaded on expand)
                FileEntry::new(p.to_string_lossy().to_string(), n, is_dir, child_depth)
            })
            .collect();

        children.sort_by(|a, b| match (a.is_directory, b.is_directory) {
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            _ => {
                let a_dot = a.name.starts_with('.');
                let b_dot = b.name.starts_with('.');
                match (a_dot, b_dot) {
                    (true, false) => Ordering::Greater,
                    (false, true) => Ordering::Less,
                    _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                }
            }
        });

        children
    }

    pub fn flatten(&mut self) {
        self.flat_entries.clear();
        if let Some(ref root) = self.root {
            Self::flatten_entry(root, &mut self.flat_entries);
        }
    }

    fn flatten_entry(entry: &FileEntry, flat: &mut Vec<FlatEntry>) {
        flat.push(FlatEntry {
            path: entry.path.clone(),
            name: entry.name.clone(),
            is_directory: entry.is_directory,
            depth: entry.depth,
            is_expanded: entry.is_expanded,
        });

        if entry.is_directory && entry.is_expanded {
            for child in &entry.children {
                Self::flatten_entry(child, flat);
            }
        }
    }

    pub fn toggle_expand(&mut self, index: usize) {
        if index >= self.flat_entries.len() {
            return;
        }
        let path = self.flat_entries[index].path.clone();
        if self.flat_entries[index].is_directory {
            let show_hidden = self.show_hidden;
            if let Some(ref mut root) = self.root {
                Self::toggle_node(root, &path, show_hidden);
            }
            self.flatten();
        }
    }

    fn toggle_node(entry: &mut FileEntry, path: &str, show_hidden: bool) -> bool {
        if entry.path == path {
            entry.is_expanded = !entry.is_expanded;
            // Lazy load: populate children when expanding for the first time
            if entry.is_expanded && entry.is_directory && entry.children.is_empty() {
                entry.children = Self::read_children(&entry.path, entry.depth + 1, show_hidden);
            }
            return true;
        }
        for child in &mut entry.children {
            if Self::toggle_node(child, path, show_hidden) {
                return true;
            }
        }
        false
    }

    pub fn reveal_path(&mut self, file_path: &str) {
        // Expand all parent directories leading to this file
        let path = std::path::Path::new(file_path);
        let root_str = self.root_path.clone().unwrap_or_default();
        // Collect all ancestor directories between root and file
        let mut ancestors: Vec<String> = Vec::new();
        let mut current = path.parent();
        while let Some(p) = current {
            let ps = p.to_string_lossy().to_string();
            if ps.len() < root_str.len() { break; }
            ancestors.push(ps.clone());
            if ps == root_str { break; }
            current = p.parent();
        }
        // Expand from root towards the file
        for anc in ancestors.iter().rev() {
            self.ensure_expanded(anc);
        }
        // Select the file in the tree
        for (i, entry) in self.flat_entries.iter().enumerate() {
            if entry.path == file_path {
                self.selected_index = i;
                self.scroll_into_view();
                break;
            }
        }
    }

    pub fn ensure_expanded(&mut self, path: &str) {
        let show_hidden = self.show_hidden;
        if let Some(ref mut root) = self.root {
            Self::expand_node(root, path, show_hidden);
        }
        self.flatten();
    }

    fn expand_node(entry: &mut FileEntry, path: &str, show_hidden: bool) -> bool {
        if entry.path == path {
            entry.is_expanded = true;
            if entry.is_directory && entry.children.is_empty() {
                entry.children = Self::read_children(&entry.path, entry.depth + 1, show_hidden);
            }
            return true;
        }
        for child in &mut entry.children {
            if Self::expand_node(child, path, show_hidden) {
                return true;
            }
        }
        false
    }

    pub fn move_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
        self.scroll_into_view();
    }

    pub fn move_down(&mut self) {
        if self.selected_index + 1 < self.flat_entries.len() {
            self.selected_index += 1;
        }
        self.scroll_into_view();
    }

    pub fn scroll_into_view(&mut self) {
        if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
        }
        if self.selected_index >= self.scroll_offset + self.viewport_height {
            self.scroll_offset = self.selected_index - self.viewport_height + 1;
        }
    }

    pub fn selected_entry(&self) -> Option<&FlatEntry> {
        self.flat_entries.get(self.selected_index)
    }

    /// Get the directory path of the currently selected item
    /// (if file is selected, returns its parent directory)
    pub fn selected_dir(&self) -> Option<String> {
        self.selected_entry().map(|e| {
            if e.is_directory {
                e.path.clone()
            } else {
                Path::new(&e.path)
                    .parent()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| {
                        self.root_path.clone().unwrap_or_else(|| ".".to_string())
                    })
            }
        })
    }

    pub fn refresh(&mut self) {
        if let Some(ref path) = self.root_path.clone() {
            self.load(path);
        }
    }

    pub fn toggle_hidden(&mut self) {
        self.show_hidden = !self.show_hidden;
        self.refresh();
    }

    pub fn create_file(&mut self, path: &str) -> Result<(), String> {
        // Create parent dirs if needed
        if let Some(parent) = Path::new(path).parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(path, "").map_err(|e| e.to_string())?;
        self.refresh();
        // Expand parent folder so the new file is visible
        if let Some(parent) = Path::new(path).parent() {
            self.ensure_expanded(&parent.to_string_lossy());
        }
        // Select the new file
        for (i, entry) in self.flat_entries.iter().enumerate() {
            if entry.path == path {
                self.selected_index = i;
                self.scroll_into_view();
                break;
            }
        }
        Ok(())
    }

    pub fn create_directory(&mut self, path: &str) -> Result<(), String> {
        std::fs::create_dir_all(path).map_err(|e| e.to_string())?;
        self.refresh();
        // Expand parent folder
        if let Some(parent) = Path::new(path).parent() {
            self.ensure_expanded(&parent.to_string_lossy());
        }
        for (i, entry) in self.flat_entries.iter().enumerate() {
            if entry.path == path {
                self.selected_index = i;
                self.scroll_into_view();
                break;
            }
        }
        Ok(())
    }

    pub fn delete_entry(&mut self, path: &str) -> Result<(), String> {
        let p = Path::new(path);
        if p.is_dir() {
            std::fs::remove_dir_all(p).map_err(|e| e.to_string())?;
        } else {
            std::fs::remove_file(p).map_err(|e| e.to_string())?;
        }
        self.refresh();
        if self.selected_index >= self.flat_entries.len() && !self.flat_entries.is_empty() {
            self.selected_index = self.flat_entries.len() - 1;
        }
        Ok(())
    }

    pub fn rename_entry(&mut self, old_path: &str, new_path: &str) -> Result<(), String> {
        std::fs::rename(old_path, new_path).map_err(|e| e.to_string())?;
        self.refresh();
        // Expand parent folder so renamed item is visible
        if let Some(parent) = Path::new(new_path).parent() {
            self.ensure_expanded(&parent.to_string_lossy());
        }
        for (i, entry) in self.flat_entries.iter().enumerate() {
            if entry.path == new_path {
                self.selected_index = i;
                self.scroll_into_view();
                break;
            }
        }
        Ok(())
    }

    pub fn duplicate_entry(&mut self, path: &str) -> Result<String, String> {
        let p = Path::new(path);
        let stem = p
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let ext = p
            .extension()
            .map(|e| format!(".{}", e.to_string_lossy()))
            .unwrap_or_default();
        let parent = p
            .parent()
            .map(|pp| pp.to_string_lossy().to_string())
            .unwrap_or_else(|| ".".to_string());

        // Find unique name
        let mut counter = 1;
        let new_path = loop {
            let candidate = format!("{}/{}_copy{}{}", parent, stem, counter, ext);
            if !Path::new(&candidate).exists() {
                break candidate;
            }
            counter += 1;
        };

        if p.is_dir() {
            // Copy directory recursively
            Self::copy_dir_recursive(p, Path::new(&new_path)).map_err(|e| e.to_string())?;
        } else {
            std::fs::copy(path, &new_path).map_err(|e| e.to_string())?;
        }

        self.refresh();
        // Expand parent folder so duplicate is visible
        if let Some(parent) = Path::new(&new_path).parent() {
            self.ensure_expanded(&parent.to_string_lossy());
        }
        Ok(new_path)
    }

    fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(dst)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());
            if src_path.is_dir() {
                Self::copy_dir_recursive(&src_path, &dst_path)?;
            } else {
                std::fs::copy(&src_path, &dst_path)?;
            }
        }
        Ok(())
    }

    pub fn fuzzy_search(&self, query: &str) -> Vec<FlatEntry> {
        if query.is_empty() {
            return Vec::new();
        }
        let query_lower = query.to_lowercase();
        let mut results: Vec<(i32, FlatEntry)> = Vec::new();

        // Also search in non-expanded directories
        if let Some(ref root) = self.root {
            Self::fuzzy_search_recursive(root, &query_lower, &mut results);
        }

        results.sort_by(|a, b| b.0.cmp(&a.0));
        results.into_iter().take(50).map(|(_, e)| e).collect()
    }

    fn fuzzy_search_recursive(
        entry: &FileEntry,
        query: &str,
        results: &mut Vec<(i32, FlatEntry)>,
    ) {
        if !entry.is_directory {
            let name_lower = entry.name.to_lowercase();
            if let Some(score) = fuzzy_score(&name_lower, query) {
                results.push((
                    score,
                    FlatEntry {
                        path: entry.path.clone(),
                        name: entry.name.clone(),
                        is_directory: false,
                        depth: entry.depth,
                        is_expanded: false,
                    },
                ));
            }
        }
        for child in &entry.children {
            Self::fuzzy_search_recursive(child, query, results);
        }
    }
}

fn fuzzy_score(text: &str, query: &str) -> Option<i32> {
    let text_chars: Vec<char> = text.chars().collect();
    let query_chars: Vec<char> = query.chars().collect();

    if query_chars.is_empty() {
        return Some(0);
    }

    let mut score = 0i32;
    let mut text_idx = 0;
    let mut prev_matched = false;

    for (qi, &qc) in query_chars.iter().enumerate() {
        let mut found = false;
        while text_idx < text_chars.len() {
            if text_chars[text_idx] == qc {
                score += 10;
                // Bonus for consecutive matches
                if prev_matched {
                    score += 8;
                }
                // Bonus for matching at word boundary
                if text_idx == 0
                    || text_chars[text_idx - 1] == '_'
                    || text_chars[text_idx - 1] == '-'
                    || text_chars[text_idx - 1] == '.'
                    || text_chars[text_idx - 1] == '/'
                {
                    score += 10;
                }
                // Bonus for first char match
                if qi == 0 && text_idx == 0 {
                    score += 15;
                }
                text_idx += 1;
                found = true;
                prev_matched = true;
                break;
            }
            text_idx += 1;
            score -= 1;
            prev_matched = false;
        }
        if !found {
            return None;
        }
    }

    // Bonus for shorter names (more specific match)
    score -= (text_chars.len() as i32 - query_chars.len() as i32).abs();

    // Exact match bonus
    if text == query {
        score += 100;
    }

    Some(score)
}
