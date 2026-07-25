mod keys;
mod menubar;
mod sidebar;
mod editor_view;
mod icons;
mod overlays;
mod terminal_view;

use crate::app::{App, Focus, SidebarTab, PaletteAction};
use crate::settings::{Theme, ThemeColors};
use crate::syntax;
use egui::{self, Color32, CornerRadius, FontId, RichText, Vec2, Rect, Pos2, Stroke};

pub struct CodeEditorApp {
    pub app: App,
    drag_source: Option<(usize, String)>,
    drop_target: Option<(usize, String)>,
    tc: ThemeColors,
    clipboard: Option<arboard::Clipboard>,
    last_title: String,
    // Zed-style dirty tracking
    dirty: bool,
    last_input_time: std::time::Instant,
    last_frame_time: std::time::Instant,
    // Cached git blame. The blame itself (libgit2 blames the whole file — slow)
    // runs on a worker thread; `blame_rx` (Some = in flight) carries the result.
    blame_cache: Option<(usize, String)>, // (line, blame_text)
    blame_cache_line: usize,
    blame_rx: Option<std::sync::mpsc::Receiver<(usize, Option<String>)>>,
    // Perf HUD — rolling frame times
    pub(crate) frame_times_ms: std::collections::VecDeque<f32>,
    pub(crate) section_times_ms: [f32; 5], // [menubar, tabs, status, sidebar, editor]
    // Cache last applied zoom factor so we don't invalidate egui's glyph cache every frame
    last_zoom: f32,
    /// Light/dark polarity last pushed to the OS window chrome, so the (cheap but
    /// not free) viewport command only fires when it actually changes.
    window_chrome_light: Option<bool>,
    // Cached title-strip text — recomputed only when project root or branch changes.
    cached_title: String,
    cached_title_key: (Option<String>, Option<String>),
}

pub(crate) const DEFAULT_FONT_SIZE: f32 = 14.0;
pub(crate) const LINE_SPACING: f32 = 4.0;
pub(crate) const SCROLLBAR_WIDTH: f32 = 12.0;        // Zed: 15px, we use 12 for egui
pub(crate) const CURSOR_BLINK_INTERVAL_MS: u64 = 500; // Zed: 500ms
pub(crate) const MAX_LINE_LEN: usize = 1024;           // Zed: 1024 chars per line max render

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
        // Systems
        "rs" => Color32::from_rgb(204, 120, 50),
        "c" | "h" => Color32::from_rgb(104, 151, 210),
        "cpp" | "cc" | "cxx" | "hpp" | "hh" => Color32::from_rgb(104, 151, 210),
        "go" | "mod" => Color32::from_rgb(0, 173, 216),
        "swift" => Color32::from_rgb(232, 131, 106),
        "kt" | "kts" => Color32::from_rgb(204, 120, 50),
        "java" => Color32::from_rgb(204, 120, 50),
        "cs" => Color32::from_rgb(104, 151, 210),
        "zig" => Color32::from_rgb(244, 164, 32),
        "dart" => Color32::from_rgb(0, 173, 216),
        // Web
        "js" | "mjs" | "cjs" | "jsx" => Color32::from_rgb(220, 185, 0),
        "ts" | "mts" | "cts" | "tsx" => Color32::from_rgb(55, 125, 207),
        "html" | "htm" | "xhtml" => Color32::from_rgb(232, 131, 106),
        "css" | "scss" | "sass" | "less" => Color32::from_rgb(110, 76, 188),
        "vue" => Color32::from_rgb(65, 184, 131),
        "svelte" => Color32::from_rgb(255, 62, 0),
        "astro" => Color32::from_rgb(255, 90, 50),
        // Backend
        "php" | "phtml" => Color32::from_rgb(130, 100, 190),
        "rb" | "erb" | "rake" | "gemspec" => Color32::from_rgb(200, 50, 50),
        "py" | "pyi" | "pyw" => Color32::from_rgb(55, 125, 170),
        "ex" | "exs" => Color32::from_rgb(130, 90, 190),
        "lua" => Color32::from_rgb(0, 0, 200),
        "pl" | "pm" => Color32::from_rgb(55, 125, 170),
        // Data / config
        "json" | "jsonc" | "json5" => Color32::from_rgb(152, 118, 170),
        "yaml" | "yml" => Color32::from_rgb(152, 118, 170),
        "toml" | "ini" | "cfg" | "conf" => Color32::from_rgb(106, 171, 115),
        "xml" | "xsl" | "xsd" | "plist" => Color32::from_rgb(204, 147, 89),
        "svg" => Color32::from_rgb(204, 147, 89),
        "sql" | "psql" => Color32::from_rgb(204, 167, 89),
        "prisma" => Color32::from_rgb(55, 125, 207),
        "graphql" | "gql" => Color32::from_rgb(220, 50, 130),
        "proto" => Color32::from_rgb(104, 151, 210),
        // Shell
        "sh" | "bash" | "zsh" | "fish" | "bats" | "ksh" => Color32::from_rgb(106, 171, 115),
        "ps1" | "bat" | "cmd" => Color32::from_rgb(55, 125, 207),
        // Docs
        "md" | "mdx" | "markdown" => Color32::from_rgb(104, 151, 210),
        "tex" | "latex" => Color32::from_rgb(0, 128, 0),
        "diff" | "patch" => Color32::from_rgb(220, 185, 0),
        // DevOps
        "tf" | "hcl" => Color32::from_rgb(130, 90, 220),
        "nix" => Color32::from_rgb(104, 151, 210),
        // Images
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "ico" | "bmp" => Color32::from_rgb(179, 131, 191),
        // Fonts/binary
        "woff" | "woff2" | "ttf" | "otf" | "eot" => Color32::from_rgb(128, 128, 128),
        "zip" | "tar" | "gz" | "rar" | "7z" => Color32::from_rgb(128, 128, 128),
        _ => if dark { Color32::from_rgb(128, 128, 128) } else { Color32::from_rgb(160, 160, 160) },
    }
}

impl CodeEditorApp {
    pub fn new(app: App) -> Self {
        let tc = app.settings.theme.colors();
        let clipboard = arboard::Clipboard::new().ok();
        let now = std::time::Instant::now();
        Self {
            app, drag_source: None, drop_target: None, tc, clipboard,
            last_title: String::new(),
            dirty: true,
            last_input_time: now,
            last_frame_time: now,
            blame_cache: None,
            blame_cache_line: usize::MAX,
            blame_rx: None,
            frame_times_ms: std::collections::VecDeque::with_capacity(60),
            section_times_ms: [0.0; 5],
            last_zoom: 0.0,
            window_chrome_light: None,
            cached_title: String::new(),
            cached_title_key: (None, None),
        }
    }
}

impl eframe::App for CodeEditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let frame_start = std::time::Instant::now();
        let now = frame_start;

        // ── Zed-style input rate tracking ──
        // Detect if user is actively interacting
        let has_input = ctx.input(|i| {
            !i.events.is_empty() || i.pointer.any_down() || i.pointer.any_released()
                || i.smooth_scroll_delta.length() > 0.0
        });
        if has_input {
            self.last_input_time = now;
            self.dirty = true;
        }

        // ── UI zoom is independent of editor font size (JetBrains style) ──
        // Previously: set_zoom_factor was tied to editor font_size, which double-scaled the
        // editor text (mono_sized(font_size) × zoom_factor) and made UI scale unpredictably.
        // Now: editor font scales via mono_sized(font_size); UI stays at 1.0 (system DPI).
        if (1.0_f32 - self.last_zoom).abs() > 0.001 {
            ctx.set_zoom_factor(1.0);
            self.last_zoom = 1.0;
        }

        // ── Window chrome follows the editor theme ──
        // With `fullsize_content_view` the title bar overlays our content, but
        // macOS still paints it with the *system* appearance material. On a dark
        // system that put an opaque #282C2F band across the top of the light
        // theme — a black stripe over a white window. Handing the window an
        // explicit appearance makes the title bar (and the traffic lights) match
        // whichever theme is active, the way an IDE does.
        {
            let light = self.app.settings.theme.is_light();
            if self.window_chrome_light != Some(light) {
                self.window_chrome_light = Some(light);
                ctx.send_viewport_cmd(egui::ViewportCommand::SetTheme(if light {
                    egui::SystemTheme::Light
                } else {
                    egui::SystemTheme::Dark
                }));
            }
        }

        // ── Theme refresh (check system theme every ~5s for SystemDefault) ──
        let resolved_theme = self.app.settings.theme.resolved();
        let new_tc = resolved_theme.colors();
        if self.tc.bg != new_tc.bg {
            self.tc = new_tc;
            let mut visuals = if resolved_theme.is_light() {
                egui::Visuals::light()
            } else {
                egui::Visuals::dark()
            };
            visuals.panel_fill = self.tc.bg;
            visuals.window_fill = self.tc.bg;
            visuals.faint_bg_color = self.tc.sidebar_bg;
            ctx.set_visuals(visuals);
            self.dirty = true;
        }

        // ── Window title (only when changed) ──
        // Format: `<project> — <branch> · <path>`  e.g. "ferrite — main · ~/dev/ferrite"
        // Falls back to filename if no project is open.
        {
            let project = self
                .app
                .file_tree
                .root_path
                .as_ref()
                .and_then(|p| std::path::Path::new(p).file_name())
                .map(|n| n.to_string_lossy().to_string());
            let branch = self
                .app
                .git_status
                .as_ref()
                .filter(|g| g.is_repo)
                .map(|g| g.branch.clone());
            let path_display = self.app.file_tree.root_path.as_ref().map(|p| {
                // Replace $HOME prefix with "~" for readability
                let home = dirs::home_dir()
                    .map(|h| h.to_string_lossy().to_string())
                    .unwrap_or_default();
                if !home.is_empty() && p.starts_with(&home) {
                    format!("~{}", &p[home.len()..])
                } else {
                    p.clone()
                }
            });
            let title = match (project, branch, path_display) {
                (Some(proj), Some(br), Some(path)) => format!("{} — {} · {}", proj, br, path),
                (Some(proj), None, Some(path)) => format!("{} · {}", proj, path),
                (Some(proj), _, None) => proj,
                _ => {
                    let ed = &self.app.editors[self.app.active_editor];
                    let dirty = if ed.is_dirty { " ●" } else { "" };
                    format!("{}{} — Code Editor", ed.file_name(), dirty)
                }
            };
            if title != self.last_title {
                self.last_title = title.clone();
                ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
            }
        }

        // ── Render UI ──
        self.handle_keys(ctx);
        self.render_title_strip(ctx);
        let t0 = std::time::Instant::now();
        self.render_menu_bar(ctx);
        let mb_ms = t0.elapsed().as_secs_f32() * 1000.0;
        let t1 = std::time::Instant::now();
        self.render_tabs(ctx);
        let tabs_ms = t1.elapsed().as_secs_f32() * 1000.0;
        let t2 = std::time::Instant::now();
        self.render_status(ctx);
        let status_ms = t2.elapsed().as_secs_f32() * 1000.0;
        if self.app.show_bottom_panel { self.render_bottom_panel(ctx); }
        let t3 = std::time::Instant::now();
        self.render_activity_bar(ctx);
        if self.app.show_sidebar { self.render_sidebar(ctx); }
        let sb_ms = t3.elapsed().as_secs_f32() * 1000.0;
        let t4 = std::time::Instant::now();
        self.render_editor(ctx);
        let ed_ms = t4.elapsed().as_secs_f32() * 1000.0;
        self.section_times_ms = [mb_ms, tabs_ms, status_ms, sb_ms, ed_ms];
        self.render_drag_overlay(ctx);
        self.render_overlays(ctx);

        // ── Background tasks (throttled) ──
        self.app.tick();

        // File watcher — Zed: 100ms debounce, we poll every 2s
        if self.app.file_watcher_rx.is_some() {
            let secs = now.duration_since(self.last_frame_time).as_secs();
            if secs >= 2 {
                self.app.poll_file_watcher();
            }
        }

        // Git diff — only after 1s of inactivity
        {
            let idle_ms = now.duration_since(self.last_input_time).as_millis();
            let ed = &mut self.app.editors[self.app.active_editor];
            if ed.is_dirty && ed.original_content.is_some() && idle_ms > 1000 {
                ed.compute_line_diff();
            }
        }

        // Deferred actions
        if let Some(action) = self.app.pending_action.take() {
            self.app.execute_palette_action(action);
            self.dirty = true;
        }

        // Poll async file tree scan (Zed: BackgroundScanner result)
        if let Some(ref rx) = self.app.file_tree_rx {
            if let Ok(root) = rx.try_recv() {
                self.app.file_tree.root = Some(root);
                self.app.file_tree.flatten();
                let name = self.app.file_tree.root_path.as_ref()
                    .and_then(|p| std::path::Path::new(p).file_name())
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                self.app.status_message = format!("Opened: {}", name);
                self.app.file_tree_rx = None;
                self.dirty = true;
            }
        }

        // Poll async results
        if let Some(ref rx) = self.app.folder_picker_rx {
            if let Ok(path) = rx.try_recv() {
                self.app.open_folder(path);
                self.app.folder_picker_rx = None;
                self.dirty = true;
            }
        }
        if let Some(ref rx) = self.app.search_rx {
            if let Ok((files, content)) = rx.try_recv() {
                self.app.file_search_results = files;
                self.app.global_search_results = content;
                self.app.global_search_selected = 0;
                self.app.search_rx = None;
                self.dirty = true;
            }
        }
        if let Some(trigger_time) = self.app.last_search_trigger {
            if trigger_time.elapsed().as_millis() >= 150 {
                self.app.last_search_trigger = None;
                self.do_search();
            }
        }
        if let Some(ref rx) = self.app.git_rx {
            if let Ok(status) = rx.try_recv() {
                self.app.git_status = Some(status);
                self.app.git_rx = None;
                self.dirty = true;
            }
        }

        self.last_frame_time = now;

        // ── Perf HUD: record CPU-side frame time ──
        let elapsed_ms = frame_start.elapsed().as_secs_f32() * 1000.0;
        if self.frame_times_ms.len() == 60 { self.frame_times_ms.pop_front(); }
        self.frame_times_ms.push_back(elapsed_ms);

        // ── Zed-style repaint scheduling ──
        // Key insight from Zed: only request repaint when actually needed
        let idle_since = now.duration_since(self.last_input_time);
        let has_pending_async = self.app.search_rx.is_some()
            || self.app.git_rx.is_some()
            || self.app.folder_picker_rx.is_some()
            || self.app.file_tree_rx.is_some()
            || self.app.last_search_trigger.is_some()
            || self.drag_source.is_some();

        if self.dirty {
            // Something changed this frame — most importantly a deferred
            // `pending_action` (executed AFTER the panels render) or an async
            // result that arrived while otherwise idle. Its effect isn't on
            // screen yet, so draw the follow-up frame immediately. Cleared so we
            // don't spin once everything has settled.
            self.dirty = false;
            ctx.request_repaint();
        } else if idle_since.as_millis() < 1000 {
            // Sustain full-speed repaint for ~1s after any input. winit on macOS
            // coalesces/delays pointer events when the run loop is allowed to
            // sleep, so a 500ms blink cadence made mouse hover→press→release→click
            // feel laggy. Running at frame rate for a 1s tail keeps clicks (and
            // the cursor blink) snappy; continued input keeps resetting the tail.
            ctx.request_repaint();
        } else if has_pending_async {
            // Idle but background work is in flight — poll at 10fps.
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        // After 1s idle with nothing dirty/pending: NO repaint requested. egui
        // sleeps until the next user input — the key to ~0% CPU at idle, like Zed.
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
