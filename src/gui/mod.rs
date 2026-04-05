mod keys;
mod menubar;
mod sidebar;
mod editor_view;
mod overlays;

use crate::app::{App, Focus, SidebarTab, PaletteAction};
use crate::settings::{Theme, ThemeColors};
use crate::syntax;
use egui::{self, Color32, FontId, RichText, Vec2, Rect, Pos2, Stroke, Rounding};

pub struct CodeEditorApp {
    pub app: App,
    drag_source: Option<(usize, String)>,
    drop_target: Option<(usize, String)>,
    tc: ThemeColors,
    clipboard: Option<arboard::Clipboard>,
}

pub(crate) const DEFAULT_FONT_SIZE: f32 = 14.0;
pub(crate) const LINE_SPACING: f32 = 4.0;

pub(crate) fn mono() -> FontId { FontId::monospace(DEFAULT_FONT_SIZE) }
pub(crate) fn mono_sized(size: f32) -> FontId { FontId::monospace(size) }
pub(crate) fn small_sized(size: f32) -> FontId { FontId::monospace((size - 1.5).max(8.0)) }
pub(crate) fn small() -> FontId { FontId::monospace(12.5) }

/// File type icon color for JetBrains-style colored dot/letter indicators
pub(crate) fn file_icon_color(name: &str, dark: bool) -> Color32 {
    // Check full filename first for dotfiles
    match name {
        ".env" | ".env.local" | ".env.production" | ".env.development" | ".env.test" | ".env.example"
            => return Color32::from_rgb(250, 189, 47), // yellow — secrets/config
        ".htaccess" | ".nginx.conf"
            => return Color32::from_rgb(106, 171, 115), // green — server config
        ".gitignore" | ".dockerignore" | ".hgignore"
            => return Color32::from_rgb(128, 128, 128), // gray
        ".editorconfig" | ".prettierrc" | ".eslintrc" | ".npmrc"
            => return Color32::from_rgb(152, 118, 170), // purple — config
        ".bashrc" | ".zshrc" | ".bash_profile" | ".profile"
            => return Color32::from_rgb(106, 171, 115), // green — shell
        "Dockerfile" | "dockerfile"
            => return Color32::from_rgb(55, 125, 207), // blue — docker
        "Makefile" | "makefile"
            => return Color32::from_rgb(204, 120, 50), // orange
        _ => {}
    }
    let ext = std::path::Path::new(name).extension().and_then(|e| e.to_str()).unwrap_or("");
    match ext {
        "rs" => Color32::from_rgb(204, 120, 50),
        "js" | "mjs" | "cjs" | "jsx" => Color32::from_rgb(220, 185, 0),
        "ts" | "mts" | "cts" | "tsx" => Color32::from_rgb(55, 125, 207),
        "py" | "pyw" => Color32::from_rgb(55, 125, 170),
        "go" => Color32::from_rgb(0, 173, 216),
        "php" => Color32::from_rgb(130, 100, 190),
        "rb" => Color32::from_rgb(200, 50, 50),
        "java" | "kt" | "kts" => Color32::from_rgb(204, 120, 50),
        "swift" => Color32::from_rgb(232, 131, 106),
        "c" | "h" => Color32::from_rgb(104, 151, 210),
        "cpp" | "cc" | "cxx" | "hpp" => Color32::from_rgb(104, 151, 210),
        "html" | "htm" => Color32::from_rgb(232, 131, 106),
        "css" | "scss" | "sass" | "less" => Color32::from_rgb(110, 76, 188),
        "vue" => Color32::from_rgb(65, 184, 131),
        "svelte" => Color32::from_rgb(255, 62, 0),
        "json" | "jsonc" => Color32::from_rgb(152, 118, 170),
        "yaml" | "yml" => Color32::from_rgb(152, 118, 170),
        "toml" => Color32::from_rgb(106, 171, 115),
        "xml" | "xsl" => Color32::from_rgb(204, 147, 89),
        "md" | "mdx" => Color32::from_rgb(104, 151, 210),
        "sql" | "sqlite" => Color32::from_rgb(204, 167, 89),
        "sh" | "bash" | "zsh" | "fish" => Color32::from_rgb(106, 171, 115),
        "svg" => Color32::from_rgb(204, 147, 89),
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "ico" => Color32::from_rgb(179, 131, 191),
        _ => if dark { Color32::from_rgb(128, 128, 128) } else { Color32::from_rgb(160, 160, 160) },
    }
}

impl CodeEditorApp {
    pub fn new(app: App) -> Self {
        let tc = app.settings.theme.colors();
        let clipboard = arboard::Clipboard::new().ok();
        Self { app, drag_source: None, drop_target: None, tc, clipboard }
    }
}

impl eframe::App for CodeEditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Refresh theme colors if changed
        let new_tc = self.app.settings.theme.colors();
        if self.tc.bg != new_tc.bg {
            self.tc = new_tc;
            let mut visuals = if self.app.settings.theme == Theme::Light {
                egui::Visuals::light()
            } else {
                egui::Visuals::dark()
            };
            visuals.panel_fill = self.tc.bg;
            visuals.window_fill = self.tc.bg;
            visuals.faint_bg_color = self.tc.sidebar_bg;
            ctx.set_visuals(visuals);
        }
        // Update window title with current file name
        {
            let ed = &self.app.editors[self.app.active_editor];
            let name = ed.file_name();
            let dirty = if ed.is_dirty { " ●" } else { "" };
            let project = self.app.file_tree.root_path.as_ref()
                .and_then(|p| std::path::Path::new(p).file_name())
                .map(|n| format!(" — {}", n.to_string_lossy()))
                .unwrap_or_default();
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(
                format!("{}{}{} — Code Editor", name, dirty, project)
            ));
        }
        self.handle_keys(ctx);
        self.render_menu_bar(ctx);
        self.render_tabs(ctx);
        self.render_status(ctx);
        if self.app.show_sidebar { self.render_sidebar(ctx); }
        self.render_editor(ctx);
        self.render_drag_overlay(ctx);
        self.render_overlays(ctx);
        // Auto-save tick
        self.app.tick();
        // Compute git diff for active editor periodically
        {
            let ed = &mut self.app.editors[self.app.active_editor];
            if ed.is_dirty && ed.original_content.is_some() {
                ed.compute_line_diff();
            }
        }
        // Execute deferred actions (file dialogs need to run after rendering)
        if let Some(action) = self.app.pending_action.take() {
            self.app.execute_palette_action(action);
        }
        // Poll async folder picker result
        if let Some(ref rx) = self.app.folder_picker_rx {
            if let Ok(path) = rx.try_recv() {
                self.app.open_folder(path);
                self.app.folder_picker_rx = None;
            }
        }
        // Poll async search results
        if let Some(ref rx) = self.app.search_rx {
            if let Ok((files, content)) = rx.try_recv() {
                self.app.file_search_results = files;
                self.app.global_search_results = content;
                self.app.global_search_selected = 0;
                self.app.search_rx = None;
            }
        }
        // Debounced global search trigger (150ms delay)
        if let Some(trigger_time) = self.app.last_search_trigger {
            if trigger_time.elapsed().as_millis() >= 150 {
                self.app.last_search_trigger = None;
                self.do_search();
            }
        }
        // Poll async git status
        if let Some(ref rx) = self.app.git_rx {
            if let Ok(status) = rx.try_recv() {
                self.app.git_status = Some(status);
                self.app.git_rx = None;
            }
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(500));
    }
}

impl CodeEditorApp {
    fn do_search(&mut self) {
        if let Some(ref root) = self.app.file_tree.root_path {
            let query = self.app.global_search_input.clone();
            let root = root.clone();
            let case_sensitive = self.app.find_case_sensitive;
            let use_regex = self.app.find_use_regex;
            let (tx, rx) = std::sync::mpsc::channel();
            self.app.search_rx = Some(rx);
            std::thread::spawn(move || {
                let files = crate::search::search_files_by_name(&root, &query);
                let content = crate::search::search_in_project(&root, &query, case_sensitive, use_regex);
                let _ = tx.send((files, content));
            });
        }
    }
}
