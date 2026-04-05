use crate::editor::Editor;
use crate::file_tree::FileTree;
use crate::git::{GitManager, GitStatus};
use crate::search;
use crate::settings::Settings;
use crate::terminal::TerminalManager;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Focus {
    Editor,
    FileTree,
    Terminal,
    CommandPalette,
    QuickOpen,
    FindReplace,
    GoToLine,
    GlobalSearch,
    GitPanel,
    NewFileDialog,
    NewFolderDialog,
    RenameDialog,
    DeleteConfirm,
    CommitInput,
    SaveAsDialog,
    Autocomplete,
    About,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SidebarTab {
    Files,
    Git,
    Search,
}

pub struct App {
    pub editors: Vec<Editor>,
    pub active_editor: usize,
    pub file_tree: FileTree,
    pub git: GitManager,
    pub git_status: Option<GitStatus>,
    pub terminal: TerminalManager,
    pub settings: Settings,
    pub focus: Focus,
    pub show_sidebar: bool,
    pub show_terminal: bool,
    pub sidebar_width: u16,
    pub terminal_height: u16,
    pub sidebar_tab: SidebarTab,
    pub status_message: String,
    pub width: u16,
    pub height: u16,

    // Command palette
    pub palette_input: String,
    pub palette_items: Vec<PaletteItem>,
    pub palette_selected: usize,

    // Quick open
    pub quick_open_input: String,
    pub quick_open_results: Vec<crate::file_tree::FlatEntry>,
    pub quick_open_selected: usize,

    // Find & Replace
    pub find_input: String,
    pub replace_input: String,
    pub find_matches: Vec<search::FindMatch>,
    pub find_current: usize,
    pub find_case_sensitive: bool,
    pub find_use_regex: bool,
    pub find_focus_replace: bool,

    // Go to line
    pub goto_input: String,

    // Global search
    pub global_search_input: String,
    pub global_search_results: Vec<search::SearchResult>,
    pub global_search_selected: usize,
    pub file_search_results: Vec<search::FileMatch>,

    // Git commit
    pub commit_message: String,
    pub git_selected: usize,

    // Terminal
    pub active_terminal: Option<u32>,

    // File dialogs
    pub dialog_input: String,
    pub dialog_context_path: String, // parent dir for new file/folder, or path for rename

    // Save-as dialog
    pub save_as_input: String,

    // Autocomplete
    pub autocomplete_suggestions: Vec<String>,
    pub autocomplete_selected: usize,
    pub show_autocomplete: bool,

    // Auto-save
    pub auto_save_enabled: bool,

    // Minimap
    pub show_minimap: bool,

    // Breadcrumbs
    pub show_breadcrumbs: bool,

    // Deferred action (for file dialogs from keyboard shortcuts)
    pub pending_action: Option<PaletteAction>,

    // Async folder picker result
    pub folder_picker_rx: Option<std::sync::mpsc::Receiver<String>>,

    // Async search
    pub search_rx: Option<std::sync::mpsc::Receiver<(Vec<search::FileMatch>, Vec<search::SearchResult>)>>,

    // Async git status
    pub git_rx: Option<std::sync::mpsc::Receiver<crate::git::GitStatus>>,

    // Debounce: last search trigger time
    pub last_search_trigger: Option<std::time::Instant>,
}

#[derive(Debug, Clone)]
pub struct PaletteItem {
    pub name: String,
    pub shortcut: String,
    pub action: PaletteAction,
}

#[derive(Debug, Clone)]
pub enum PaletteAction {
    NewFile,
    OpenFile,
    OpenFolder,
    Save,
    SaveAll,
    CloseTab,
    Find,
    FindInProject,
    GoToLine,
    ToggleSidebar,
    ToggleTerminal,
    ToggleTheme,
    ToggleWordWrap,
    ToggleLineNumbers,
    ToggleHiddenFiles,
    ToggleMinimap,
    ToggleBreadcrumbs,
    ToggleAutoSave,
    QuickOpen,
    Quit,
}

impl App {
    pub fn new() -> Self {
        let settings = Settings::load();
        Self {
            editors: vec![Editor::new()],
            active_editor: 0,
            file_tree: FileTree::new(),
            git: GitManager::new(),
            git_status: None,
            terminal: TerminalManager::new(),
            settings,
            focus: Focus::Editor,
            show_sidebar: true,
            show_terminal: false,
            sidebar_width: 32,
            terminal_height: 12,
            sidebar_tab: SidebarTab::Files,
            status_message: String::from("Ready  |  Ctrl+Shift+P: commands  |  Ctrl+P: quick open"),
            width: 80,
            height: 24,
            palette_input: String::new(),
            palette_items: Self::build_palette_items(),
            palette_selected: 0,
            quick_open_input: String::new(),
            quick_open_results: Vec::new(),
            quick_open_selected: 0,
            find_input: String::new(),
            replace_input: String::new(),
            find_matches: Vec::new(),
            find_current: 0,
            find_case_sensitive: false,
            find_use_regex: false,
            find_focus_replace: false,
            goto_input: String::new(),
            global_search_input: String::new(),
            global_search_results: Vec::new(),
            global_search_selected: 0,
            file_search_results: Vec::new(),
            commit_message: String::new(),
            git_selected: 0,
            active_terminal: None,
            dialog_input: String::new(),
            dialog_context_path: String::new(),
            save_as_input: String::new(),
            autocomplete_suggestions: Vec::new(),
            autocomplete_selected: 0,
            show_autocomplete: false,
            auto_save_enabled: true,
            show_minimap: true,
            show_breadcrumbs: true,
            pending_action: None,
            folder_picker_rx: None,
            search_rx: None,
            git_rx: None,
            last_search_trigger: None,
        }
    }

    fn build_palette_items() -> Vec<PaletteItem> {
        vec![
            PaletteItem { name: "New File".into(), shortcut: "Ctrl+N".into(), action: PaletteAction::NewFile },
            PaletteItem { name: "Save".into(), shortcut: "Ctrl+S".into(), action: PaletteAction::Save },
            PaletteItem { name: "Save All".into(), shortcut: "Ctrl+Shift+S".into(), action: PaletteAction::SaveAll },
            PaletteItem { name: "Close Tab".into(), shortcut: "Ctrl+W".into(), action: PaletteAction::CloseTab },
            PaletteItem { name: "Quick Open".into(), shortcut: "Ctrl+P".into(), action: PaletteAction::QuickOpen },
            PaletteItem { name: "Find in File".into(), shortcut: "Ctrl+F".into(), action: PaletteAction::Find },
            PaletteItem { name: "Find in Project".into(), shortcut: "Ctrl+Shift+F".into(), action: PaletteAction::FindInProject },
            PaletteItem { name: "Go to Line".into(), shortcut: "Ctrl+G".into(), action: PaletteAction::GoToLine },
            PaletteItem { name: "Toggle Sidebar".into(), shortcut: "Ctrl+B".into(), action: PaletteAction::ToggleSidebar },
            PaletteItem { name: "Toggle Terminal".into(), shortcut: "Ctrl+`".into(), action: PaletteAction::ToggleTerminal },
            PaletteItem { name: "Toggle Theme (Dark/Light)".into(), shortcut: "".into(), action: PaletteAction::ToggleTheme },
            PaletteItem { name: "Toggle Word Wrap".into(), shortcut: "".into(), action: PaletteAction::ToggleWordWrap },
            PaletteItem { name: "Toggle Line Numbers".into(), shortcut: "".into(), action: PaletteAction::ToggleLineNumbers },
            PaletteItem { name: "Toggle Hidden Files (.env, .git...)".into(), shortcut: "".into(), action: PaletteAction::ToggleHiddenFiles },
            PaletteItem { name: "Toggle Minimap".into(), shortcut: "".into(), action: PaletteAction::ToggleMinimap },
            PaletteItem { name: "Toggle Breadcrumbs".into(), shortcut: "".into(), action: PaletteAction::ToggleBreadcrumbs },
            PaletteItem { name: "Toggle Auto-Save".into(), shortcut: "".into(), action: PaletteAction::ToggleAutoSave },
            PaletteItem { name: "Quit".into(), shortcut: "Ctrl+Q".into(), action: PaletteAction::Quit },
        ]
    }

    pub fn active_editor(&self) -> &Editor {
        &self.editors[self.active_editor]
    }

    pub fn active_editor_mut(&mut self) -> &mut Editor {
        &mut self.editors[self.active_editor]
    }

    pub fn open_folder(&mut self, path: String) {
        let name = std::path::Path::new(&path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or(path.clone());
        self.status_message = format!("Opening: {}...", name);
        self.file_tree.load(&path);
        self.status_message = format!("Opened: {}", name);
        // Save to recent projects
        self.settings.add_recent_project(&path);
        // Git status in background
        self.refresh_git_async();
    }

    pub fn open_file(&mut self, path: &str) {
        // Check if already open
        for (i, editor) in self.editors.iter().enumerate() {
            if editor.file_path.as_deref() == Some(path) {
                self.active_editor = i;
                self.focus = Focus::Editor;
                return;
            }
        }

        match Editor::from_file(path) {
            Ok(editor) => {
                // Replace empty untitled buffer
                if self.editors.len() == 1
                    && self.editors[0].file_path.is_none()
                    && !self.editors[0].is_dirty
                {
                    self.editors[0] = editor;
                } else {
                    self.editors.push(editor);
                    self.active_editor = self.editors.len() - 1;
                }
                self.focus = Focus::Editor;
                let name = std::path::Path::new(path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.to_string());
                self.status_message = format!("Opened: {}", name);
            }
            Err(e) => {
                self.status_message = format!("Error: {}", e);
            }
        }
    }

    pub fn close_tab(&mut self, index: usize) {
        if self.editors.len() <= 1 {
            self.editors[0] = Editor::new();
            self.active_editor = 0;
            return;
        }
        self.editors.remove(index);
        if self.active_editor >= self.editors.len() {
            self.active_editor = self.editors.len() - 1;
        }
    }

    pub fn save_current(&mut self) {
        // If no file path, open save-as dialog
        if self.editors[self.active_editor].file_path.is_none() {
            self.save_as_input = self
                .file_tree
                .root_path
                .as_ref()
                .map(|p| format!("{}/", p))
                .unwrap_or_default();
            self.focus = Focus::SaveAsDialog;
            return;
        }
        let editor = &mut self.editors[self.active_editor];
        match editor.save() {
            Ok(_) => {
                let name = editor.file_name();
                self.status_message = format!("Saved: {}", name);
                self.refresh_git_status();
            }
            Err(e) => {
                self.status_message = format!("Save error: {}", e);
            }
        }
    }

    pub fn save_all(&mut self) {
        let mut saved = 0;
        let mut errors = 0;
        for editor in &mut self.editors {
            if editor.is_dirty && editor.file_path.is_some() {
                match editor.save() {
                    Ok(_) => saved += 1,
                    Err(_) => errors += 1,
                }
            }
        }
        self.status_message = if errors > 0 {
            format!("Saved {} files, {} errors", saved, errors)
        } else {
            format!("Saved {} files", saved)
        };
        self.refresh_git_status();
    }

    pub fn refresh_git_status(&mut self) {
        if let Some(ref root) = self.file_tree.root_path {
            self.git_status = Some(self.git.get_status(root));
        }
    }

    pub fn refresh_git_async(&mut self) {
        if let Some(ref root) = self.file_tree.root_path {
            let root = root.clone();
            let (tx, rx) = std::sync::mpsc::channel();
            self.git_rx = Some(rx);
            std::thread::spawn(move || {
                let mut git = crate::git::GitManager::new();
                let status = git.get_status(&root);
                let _ = tx.send(status);
            });
        }
    }

    pub fn execute_palette_action(&mut self, action: PaletteAction) {
        match action {
            PaletteAction::NewFile => {
                self.editors.push(Editor::new());
                self.active_editor = self.editors.len() - 1;
                self.focus = Focus::Editor;
            }
            PaletteAction::Save => self.save_current(),
            PaletteAction::SaveAll => self.save_all(),
            PaletteAction::CloseTab => self.close_tab(self.active_editor),
            PaletteAction::Find => {
                self.focus = Focus::FindReplace;
                self.find_focus_replace = false;
            }
            PaletteAction::FindInProject => {
                self.focus = Focus::GlobalSearch;
                self.sidebar_tab = SidebarTab::Search;
                self.show_sidebar = true;
            }
            PaletteAction::GoToLine => {
                self.focus = Focus::GoToLine;
                self.goto_input.clear();
            }
            PaletteAction::ToggleSidebar => self.show_sidebar = !self.show_sidebar,
            PaletteAction::ToggleTerminal => {
                self.show_terminal = !self.show_terminal;
                if self.show_terminal && self.active_terminal.is_none() {
                    self.spawn_terminal();
                }
                if self.show_terminal {
                    self.focus = Focus::Terminal;
                } else {
                    self.focus = Focus::Editor;
                }
            }
            PaletteAction::ToggleTheme => {
                let themes = crate::settings::Theme::ALL;
                let idx = themes.iter().position(|t| *t == self.settings.theme).unwrap_or(0);
                self.settings.theme = themes[(idx + 1) % themes.len()];
                self.settings.save();
            }
            PaletteAction::ToggleWordWrap => {
                self.settings.word_wrap = !self.settings.word_wrap;
                self.settings.save();
            }
            PaletteAction::ToggleLineNumbers => {
                self.settings.show_line_numbers = !self.settings.show_line_numbers;
                self.settings.save();
            }
            PaletteAction::ToggleHiddenFiles => {
                self.file_tree.toggle_hidden();
                let state = if self.file_tree.show_hidden { "shown" } else { "hidden" };
                self.status_message = format!("Hidden files: {}", state);
            }
            PaletteAction::ToggleMinimap => {
                self.show_minimap = !self.show_minimap;
                self.status_message = format!("Minimap: {}", if self.show_minimap { "on" } else { "off" });
            }
            PaletteAction::ToggleBreadcrumbs => {
                self.show_breadcrumbs = !self.show_breadcrumbs;
                self.status_message = format!("Breadcrumbs: {}", if self.show_breadcrumbs { "on" } else { "off" });
            }
            PaletteAction::ToggleAutoSave => {
                self.auto_save_enabled = !self.auto_save_enabled;
                self.status_message = format!("Auto-save: {}", if self.auto_save_enabled { "on" } else { "off" });
            }
            PaletteAction::QuickOpen => {
                self.focus = Focus::QuickOpen;
                self.quick_open_input.clear();
                self.quick_open_results.clear();
                self.quick_open_selected = 0;
            }
            PaletteAction::Quit => {}
            PaletteAction::OpenFile => {
                let start_dir = self.file_tree.root_path.clone()
                    .or_else(|| dirs::home_dir().map(|p| p.to_string_lossy().to_string()))
                    .unwrap_or_else(|| ".".to_string());
                if let Some(path) = rfd::FileDialog::new().set_directory(&start_dir).pick_file() {
                    let path_str = path.to_string_lossy().to_string();
                    if let Some(parent) = path.parent() {
                        if self.file_tree.root_path.is_none() {
                            self.open_folder(parent.to_string_lossy().to_string());
                        }
                    }
                    self.open_file(&path_str);
                }
            }
            PaletteAction::OpenFolder => {
                // Spawn native macOS folder picker in background thread
                let start_dir = self.file_tree.root_path.clone()
                    .or_else(|| dirs::home_dir().map(|p| p.to_string_lossy().to_string()))
                    .unwrap_or_else(|| ".".to_string());
                let (tx, rx) = std::sync::mpsc::channel();
                self.folder_picker_rx = Some(rx);
                std::thread::spawn(move || {
                    if let Ok(output) = std::process::Command::new("osascript")
                        .arg("-e")
                        .arg(format!("POSIX path of (choose folder with prompt \"Open Folder\" default location POSIX file \"{}\")", start_dir))
                        .output()
                    {
                        if output.status.success() {
                            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                            if !path.is_empty() {
                                let _ = tx.send(path);
                            }
                        }
                    }
                });
            }
        }
    }

    /// Get the working directory: project root, or cwd
    fn working_dir(&self) -> String {
        self.file_tree
            .root_path
            .clone()
            .unwrap_or_else(|| {
                std::env::current_dir()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| ".".to_string())
            })
    }

    pub fn start_new_file_dialog(&mut self) {
        let dir = self
            .file_tree
            .selected_dir()
            .unwrap_or_else(|| self.working_dir());
        self.dialog_context_path = dir;
        self.dialog_input.clear();
        self.focus = Focus::NewFileDialog;
        self.show_sidebar = true;
        self.sidebar_tab = SidebarTab::Files;
    }

    pub fn start_new_folder_dialog(&mut self) {
        let dir = self
            .file_tree
            .selected_dir()
            .unwrap_or_else(|| self.working_dir());
        self.dialog_context_path = dir;
        self.dialog_input.clear();
        self.focus = Focus::NewFolderDialog;
        self.show_sidebar = true;
        self.sidebar_tab = SidebarTab::Files;
    }

    pub fn start_rename_dialog(&mut self) {
        if let Some(entry) = self.file_tree.selected_entry() {
            self.dialog_context_path = entry.path.clone();
            self.dialog_input = entry.name.clone();
            self.focus = Focus::RenameDialog;
        }
    }

    fn spawn_terminal(&mut self) {
        let dir = self.file_tree.root_path.as_deref();
        match self.terminal.spawn(dir) {
            Ok(id) => {
                self.active_terminal = Some(id);
                self.status_message = format!("Terminal #{} started", id);
            }
            Err(e) => {
                self.status_message = format!("Terminal error: {}", e);
            }
        }
    }

    pub fn update_find_matches(&mut self) {
        if self.find_input.is_empty() {
            self.find_matches.clear();
            return;
        }
        let content = self.active_editor().buffer.text();
        self.find_matches = search::find_in_content(
            &content,
            &self.find_input,
            self.find_case_sensitive,
            self.find_use_regex,
        );
        self.find_current = 0;
    }

    pub fn goto_next_match(&mut self) { self.find_next(); }
    pub fn goto_prev_match(&mut self) { self.find_prev(); }

    fn find_next(&mut self) {
        if !self.find_matches.is_empty() {
            self.find_current = (self.find_current + 1) % self.find_matches.len();
            let line = self.find_matches[self.find_current].line;
            let col = self.find_matches[self.find_current].col;
            self.editors[self.active_editor].cursor.line = line;
            self.editors[self.active_editor].cursor.col = col;
            self.editors[self.active_editor].scroll_into_view();
        }
    }

    fn find_prev(&mut self) {
        if !self.find_matches.is_empty() {
            if self.find_current == 0 {
                self.find_current = self.find_matches.len() - 1;
            } else {
                self.find_current -= 1;
            }
            let line = self.find_matches[self.find_current].line;
            let col = self.find_matches[self.find_current].col;
            self.editors[self.active_editor].cursor.line = line;
            self.editors[self.active_editor].cursor.col = col;
            self.editors[self.active_editor].scroll_into_view();
        }
    }

    pub fn replace_current(&mut self) {
        if self.find_matches.is_empty() {
            return;
        }
        let m = self.find_matches[self.find_current].clone();
        let replacement = self.replace_input.clone();
        let editor = &mut self.editors[self.active_editor];
        for _ in 0..m.length {
            editor.buffer.delete_char(m.line, m.col);
        }
        editor.buffer.insert_text(m.line, m.col, &replacement);
        editor.is_dirty = true;
        self.update_find_matches();
    }

    pub fn replace_all(&mut self) {
        if self.find_input.is_empty() {
            return;
        }
        let content = self.active_editor().buffer.text();
        let new_content = search::replace_in_content(
            &content,
            &self.find_input,
            &self.replace_input,
            self.find_case_sensitive,
            self.find_use_regex,
            true,
        );
        let editor = self.active_editor_mut();
        editor.buffer.rope = ropey::Rope::from_str(&new_content);
        editor.is_dirty = true;
        self.update_find_matches();
    }

    /// Auto-save: save dirty files after 2 seconds of inactivity
    pub fn auto_save_tick(&mut self) {
        if !self.auto_save_enabled { return; }
        for editor in &mut self.editors {
            if editor.is_dirty && editor.file_path.is_some() {
                if let Some(last_edit) = editor.last_edit_time {
                    if last_edit.elapsed().as_secs() >= 2 {
                        let _ = editor.save();
                        editor.last_edit_time = None;
                    }
                }
            }
        }
    }

    /// Trigger autocomplete
    pub fn trigger_autocomplete(&mut self) {
        let prefix = self.active_editor().word_at_cursor();
        if prefix.len() < 2 {
            self.show_autocomplete = false;
            self.autocomplete_suggestions.clear();
            return;
        }
        let suggestions = self.active_editor().collect_words(&prefix);
        if suggestions.is_empty() {
            self.show_autocomplete = false;
            self.autocomplete_suggestions.clear();
        } else {
            self.autocomplete_suggestions = suggestions;
            self.autocomplete_selected = 0;
            self.show_autocomplete = true;
        }
    }

    /// Accept current autocomplete suggestion
    pub fn accept_autocomplete(&mut self) {
        if !self.show_autocomplete || self.autocomplete_suggestions.is_empty() {
            return;
        }
        let suggestion = self.autocomplete_suggestions[self.autocomplete_selected].clone();
        let prefix = self.active_editor().word_at_cursor();
        let suffix = &suggestion[prefix.len()..];
        let ed = self.active_editor_mut();
        for c in suffix.chars() {
            ed.buffer.insert_char(ed.cursor.line, ed.cursor.col, c);
            ed.cursor.col += 1;
        }
        ed.is_dirty = true;
        ed.diagnostics_dirty = true;
        ed.last_edit_time = Some(std::time::Instant::now());
        self.show_autocomplete = false;
        self.autocomplete_suggestions.clear();
    }

    pub fn tick(&mut self) {
        self.auto_save_tick();
    }
}

// Remaining code after GUI migration removed
// Git panel helpers for GUI
impl App {
    pub fn git_stage_selected(&mut self) {
        let (file_path, root) = match (&self.git_status, &self.file_tree.root_path) {
            (Some(status), Some(root)) => match status.files.get(self.git_selected) {
                Some(file) => (file.path.clone(), root.clone()),
                None => return,
            },
            _ => return,
        };
        match self.git.stage_file(&root, &file_path) {
            Ok(_) => {
                self.status_message = format!("Staged: {}", file_path);
                self.refresh_git_status();
            }
            Err(e) => self.status_message = format!("Error: {}", e),
        }
    }

    pub fn git_unstage_selected(&mut self) {
        let (file_path, root) = match (&self.git_status, &self.file_tree.root_path) {
            (Some(status), Some(root)) => match status.files.get(self.git_selected) {
                Some(file) => (file.path.clone(), root.clone()),
                None => return,
            },
            _ => return,
        };
        match self.git.unstage_file(&root, &file_path) {
            Ok(_) => {
                self.status_message = format!("Unstaged: {}", file_path);
                self.refresh_git_status();
            }
            Err(e) => self.status_message = format!("Error: {}", e),
        }
    }

    pub fn git_stage_all(&mut self) {
        let root = match &self.file_tree.root_path {
            Some(r) => r.clone(),
            None => return,
        };
        match self.git.stage_all(&root) {
            Ok(_) => {
                self.status_message = "Staged all files".into();
                self.refresh_git_status();
            }
            Err(e) => self.status_message = format!("Error: {}", e),
        }
    }

    pub fn filtered_palette_items(&self) -> Vec<PaletteItem> {
        if self.palette_input.is_empty() {
            return self.palette_items.clone();
        }
        let query = self.palette_input.to_lowercase();
        self.palette_items
            .iter()
            .filter(|item| item.name.to_lowercase().contains(&query))
            .cloned()
            .collect()
    }
}
