use crate::app::{App, Focus, SidebarTab, PaletteAction};
use crate::settings::{Theme, ThemeColors};
use crate::syntax;
use egui::{self, Color32, FontId, RichText, Vec2, Rect, Pos2, Stroke, Rounding};

pub struct CodeEditorApp {
    pub app: App,
    drag_source: Option<(usize, String)>,  // (index, path) of dragged item
    drop_target: Option<(usize, String)>,  // (index, path) of drop target
    tc: ThemeColors,                        // cached theme colors
}

const DEFAULT_FONT_SIZE: f32 = 14.0;
const LINE_SPACING: f32 = 4.0; // extra pixels between lines

fn mono() -> FontId { FontId::monospace(DEFAULT_FONT_SIZE) }
fn mono_sized(size: f32) -> FontId { FontId::monospace(size) }
fn small_sized(size: f32) -> FontId { FontId::monospace((size - 1.5).max(8.0)) }
fn small() -> FontId { FontId::monospace(12.5) }

/// File type icon color for JetBrains-style colored dot/letter indicators
fn file_icon_color(name: &str, dark: bool) -> Color32 {
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

/// Compute bracket depth at each bracket in the file.
/// For performance, we scan up to and including visible lines.
fn compute_bracket_depths(app: &App, vis_lines: &[usize]) -> std::collections::HashMap<(usize, usize), usize> {
    let ed = &app.editors[app.active_editor];
    let max_line = vis_lines.last().copied().unwrap_or(0) + 1;
    let lc = ed.line_count().min(max_line);
    let mut depths = std::collections::HashMap::new();
    let mut depth: i32 = 0;
    let mut in_string = false;
    let mut string_char = '"';

    for li in 0..lc {
        let line = ed.buffer.get_line(li);
        let mut prev = '\0';
        for (ci, ch) in line.chars().enumerate() {
            if in_string {
                if ch == string_char && prev != '\\' { in_string = false; }
            } else {
                if (ch == '"' || ch == '\'' || ch == '`') && prev != '\\' {
                    in_string = true;
                    string_char = ch;
                } else {
                    match ch {
                        '(' | '[' | '{' => {
                            depths.insert((li, ci), depth as usize);
                            depth += 1;
                        }
                        ')' | ']' | '}' => {
                            depth = (depth - 1).max(0);
                            depths.insert((li, ci), depth as usize);
                        }
                        _ => {}
                    }
                }
            }
            prev = ch;
        }
        in_string = false; // reset per-line for simplicity
    }
    depths
}

/// Find matching bracket position for bracket at (line, col)
fn find_matching_bracket(app: &App, line: usize, col: usize) -> Option<(usize, usize)> {
    let ed = &app.editors[app.active_editor];
    let text = ed.buffer.get_line(line);
    let chars: Vec<char> = text.chars().collect();
    if col >= chars.len() { return None; }
    let ch = chars[col];
    let (target, forward) = match ch {
        '(' => (')', true), ')' => ('(', false),
        '[' => (']', true), ']' => ('[', false),
        '{' => ('}', true), '}' => ('{', false),
        _ => return None,
    };
    let mut depth = 0i32;
    let lc = ed.line_count();
    if forward {
        let mut l = line;
        let mut c = col;
        loop {
            let ln = ed.buffer.get_line(l);
            let lchars: Vec<char> = ln.chars().collect();
            while c < lchars.len() {
                if lchars[c] == ch { depth += 1; }
                else if lchars[c] == target { depth -= 1; if depth == 0 { return Some((l, c)); } }
                c += 1;
            }
            l += 1; c = 0;
            if l >= lc { break; }
        }
    } else {
        let mut l = line as isize;
        let mut c = col as isize;
        loop {
            let ln = ed.buffer.get_line(l as usize);
            let lchars: Vec<char> = ln.chars().collect();
            while c >= 0 {
                let cu = c as usize;
                if cu < lchars.len() {
                    if lchars[cu] == ch { depth += 1; }
                    else if lchars[cu] == target { depth -= 1; if depth == 0 { return Some((l as usize, cu)); } }
                }
                c -= 1;
            }
            l -= 1;
            if l < 0 { break; }
            let prev_len = ed.buffer.get_line(l as usize).chars().count();
            c = prev_len as isize - 1;
        }
    }
    None
}

impl CodeEditorApp {
    pub fn new(app: App) -> Self {
        let tc = app.settings.theme.colors();
        Self { app, drag_source: None, drop_target: None, tc }
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
    fn handle_keys(&mut self, ctx: &egui::Context) {
        // Use consume_key to grab shortcuts before widgets can eat them
        let find = ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::F));
        let save = ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::S));
        let quit = ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Q));
        let new_file = ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::N));
        let close_tab = ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::W));
        let toggle_sb = ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::B));
        let palette = ctx.input_mut(|i| i.consume_key(egui::Modifiers { command: true, shift: true, ..Default::default() }, egui::Key::P));
        let quick_open = ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::P));
        let goto = ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::G));
        let undo = ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Z));
        let open_folder = ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::O));
        let zoom_in = ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Equals))
            || ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Plus));
        let zoom_out = ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Minus));
        let zoom_reset = ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Num0));
        let esc = ctx.input(|i| i.key_pressed(egui::Key::Escape));

        if quit { ctx.send_viewport_cmd(egui::ViewportCommand::Close); }
        if esc && self.app.focus != Focus::Editor { self.app.focus = Focus::Editor; return; }
        if save { self.app.save_current(); }
        if open_folder {
            self.app.pending_action = Some(PaletteAction::OpenFolder);
        }
        if palette { self.app.focus = Focus::CommandPalette; self.app.palette_input.clear(); self.app.palette_selected = 0; }
        else if quick_open { self.app.focus = Focus::QuickOpen; self.app.quick_open_input.clear(); self.app.quick_open_results.clear(); }
        if new_file { self.app.editors.push(crate::editor::Editor::new()); self.app.active_editor = self.app.editors.len() - 1; self.app.focus = Focus::Editor; }
        if close_tab { let i = self.app.active_editor; self.app.close_tab(i); }
        if toggle_sb { self.app.show_sidebar = !self.app.show_sidebar; }
        if find {
            self.app.focus = Focus::FindReplace;
            self.app.find_input.clear();
            self.app.find_matches.clear();
        }
        if goto { self.app.focus = Focus::GoToLine; self.app.goto_input.clear(); }
        if undo && self.app.focus == Focus::Editor { self.app.active_editor_mut().undo(); }
        if zoom_in {
            self.app.settings.font_size = (self.app.settings.font_size + 1.0).min(32.0);
            self.app.settings.save();
        }
        if zoom_out {
            self.app.settings.font_size = (self.app.settings.font_size - 1.0).max(8.0);
            self.app.settings.save();
        }
        if zoom_reset {
            self.app.settings.font_size = DEFAULT_FONT_SIZE;
            self.app.settings.save();
        }
    }

    fn render_menu_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menubar")
            .exact_height(26.0)
            .frame(egui::Frame::NONE.fill(self.tc.tab_bar_bg).inner_margin(egui::Margin { left: 8, right: 8, top: 3, bottom: 3 }))
            .show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    ui.menu_button(RichText::new("File").font(small()).color(self.tc.fg), |ui| {
                        if ui.button("New File          ⌘N").clicked() {
                            self.app.editors.push(crate::editor::Editor::new());
                            self.app.active_editor = self.app.editors.len() - 1;
                            self.app.focus = Focus::Editor;
                            ui.close_menu();
                        }
                        if ui.button("Open Folder...    ⌘O").clicked() {
                            self.app.pending_action = Some(PaletteAction::OpenFolder);
                            ui.close_menu();
                        }
                        if ui.button("Open File...").clicked() {
                            self.app.pending_action = Some(PaletteAction::OpenFile);
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Save              ⌘S").clicked() {
                            self.app.save_current();
                            ui.close_menu();
                        }
                        if ui.button("Close Tab         ⌘W").clicked() {
                            let i = self.app.active_editor;
                            self.app.close_tab(i);
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Quit              ⌘Q").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.menu_button(RichText::new("Edit").font(small()).color(self.tc.fg), |ui| {
                        if ui.button("Undo              ⌘Z").clicked() {
                            self.app.active_editor_mut().undo();
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Find              ⌘F").clicked() {
                            self.app.focus = Focus::FindReplace;
                            ui.close_menu();
                        }
                        if ui.button("Go to Line        ⌘G").clicked() {
                            self.app.focus = Focus::GoToLine;
                            self.app.goto_input.clear();
                            ui.close_menu();
                        }
                    });
                    ui.menu_button(RichText::new("View").font(small()).color(self.tc.fg), |ui| {
                        if ui.button("Command Palette   ⇧⌘P").clicked() {
                            self.app.focus = Focus::CommandPalette;
                            self.app.palette_input.clear();
                            self.app.palette_selected = 0;
                            ui.close_menu();
                        }
                        if ui.button("Quick Open        ⌘P").clicked() {
                            self.app.focus = Focus::QuickOpen;
                            self.app.quick_open_input.clear();
                            self.app.quick_open_results.clear();
                            ui.close_menu();
                        }
                        ui.separator();
                        let sidebar_label = if self.app.show_sidebar { "Hide Sidebar      ⌘B" } else { "Show Sidebar      ⌘B" };
                        if ui.button(sidebar_label).clicked() {
                            self.app.show_sidebar = !self.app.show_sidebar;
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Toggle Word Wrap").clicked() {
                            self.app.settings.word_wrap = !self.app.settings.word_wrap;
                            self.app.settings.save();
                            ui.close_menu();
                        }
                        if ui.button("Toggle Line Numbers").clicked() {
                            self.app.settings.show_line_numbers = !self.app.settings.show_line_numbers;
                            self.app.settings.save();
                            ui.close_menu();
                        }
                        let minimap_label = if self.app.show_minimap { "Hide Minimap" } else { "Show Minimap" };
                        if ui.button(minimap_label).clicked() {
                            self.app.show_minimap = !self.app.show_minimap;
                            ui.close_menu();
                        }
                        let bc_label = if self.app.show_breadcrumbs { "Hide Breadcrumbs" } else { "Show Breadcrumbs" };
                        if ui.button(bc_label).clicked() {
                            self.app.show_breadcrumbs = !self.app.show_breadcrumbs;
                            ui.close_menu();
                        }
                        let as_label = if self.app.auto_save_enabled { "Disable Auto-Save" } else { "Enable Auto-Save" };
                        if ui.button(as_label).clicked() {
                            self.app.auto_save_enabled = !self.app.auto_save_enabled;
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Zoom In            ⌘+").clicked() {
                            self.app.settings.font_size = (self.app.settings.font_size + 1.0).min(32.0);
                            self.app.settings.save();
                            ui.close_menu();
                        }
                        if ui.button("Zoom Out           ⌘-").clicked() {
                            self.app.settings.font_size = (self.app.settings.font_size - 1.0).max(8.0);
                            self.app.settings.save();
                            ui.close_menu();
                        }
                        if ui.button("Reset Zoom         ⌘0").clicked() {
                            self.app.settings.font_size = DEFAULT_FONT_SIZE;
                            self.app.settings.save();
                            ui.close_menu();
                        }
                    });
                    ui.menu_button(RichText::new("Theme").font(small()).color(self.tc.fg), |ui| {
                        for &theme in Theme::ALL {
                            let active = self.app.settings.theme == theme;
                            let label = if active { format!("● {}", theme.name()) } else { format!("  {}", theme.name()) };
                            if ui.button(label).clicked() {
                                self.app.settings.theme = theme;
                                self.app.settings.save();
                                ui.close_menu();
                            }
                        }
                    });
                });
            });
    }

    fn render_tabs(&mut self, ctx: &egui::Context) {
        let mut tab_to_close: Option<usize> = None;
        egui::TopBottomPanel::top("tabs")
            .exact_height(34.0)
            .frame(egui::Frame::NONE.fill(self.tc.tab_bar_bg).inner_margin(egui::Margin { left: 4, right: 4, top: 4, bottom: 0 }))
            .show(ctx, |ui| {
                // Bottom border
                let r = ui.max_rect();
                ui.painter().line_segment(
                    [Pos2::new(r.min.x, r.max.y), Pos2::new(r.max.x, r.max.y)],
                    Stroke::new(1.0, self.tc.border),
                );
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing.x = 1.0;
                    for i in 0..self.app.editors.len() {
                        let name = self.app.editors[i].file_name();
                        let dirty = self.app.editors[i].is_dirty;
                        let active = i == self.app.active_editor;
                        let tc = if active { self.tc.fg } else { self.tc.fg_dim };
                        let bg = if active { self.tc.bg } else { self.tc.tab_bar_bg };
                        let rounding = Rounding { nw: 6, ne: 6, sw: 0, se: 0 };

                        // Tab group: name + close button
                        let frame = egui::Frame::NONE.fill(bg).rounding(rounding)
                            .stroke(if active { Stroke::new(1.0, self.tc.border) } else { Stroke::NONE })
                            .inner_margin(egui::Margin { left: 8, right: 2, top: 2, bottom: 2 });

                        frame.show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 2.0;
                                // Tab name — clickable
                                let dot = if dirty { " ●" } else { "" };
                                let label_resp = ui.add(egui::Label::new(
                                    RichText::new(format!("{}{}", name, dot)).font(small()).color(tc)
                                ).sense(egui::Sense::click()));
                                if label_resp.clicked() {
                                    self.app.active_editor = i;
                                    self.app.focus = Focus::Editor;
                                }
                                // Close button ×
                                let close_resp = ui.add(
                                    egui::Button::new(
                                        RichText::new(" × ").font(small()).color(self.tc.fg_dim)
                                    )
                                    .frame(false)
                                    .min_size(egui::vec2(20.0, 20.0))
                                );
                                if close_resp.clicked() {
                                    tab_to_close = Some(i);
                                }
                            });
                        });
                    }
                });
            });
        // Close tab outside the borrow
        if let Some(i) = tab_to_close {
            self.app.close_tab(i);
        }
    }

    fn render_status(&mut self, ctx: &egui::Context) {
        let tc = self.tc;
        egui::TopBottomPanel::bottom("status")
            .exact_height(24.0)
            .frame(egui::Frame::NONE.fill(tc.accent.linear_multiply(0.15)).inner_margin(egui::Margin::symmetric(8, 2)))
            .show(ctx, |ui| {
                let ed = &self.app.editors[self.app.active_editor];
                let lang = ed.file_path.as_ref().map(|p| syntax::detect_language(p)).unwrap_or("Text");
                let status_msg = self.app.status_message.clone();
                let line = ed.cursor.line + 1;
                let col = ed.cursor.col + 1;
                let dirty = ed.is_dirty;
                let theme_name = self.app.settings.theme.name();
                ui.horizontal_centered(|ui| {
                    ui.label(RichText::new(&status_msg).font(small()).color(tc.fg));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 12.0;
                        // Theme switcher button
                        let theme_btn = ui.add(egui::Button::new(
                            RichText::new(format!("🎨 {}", theme_name)).font(small()).color(tc.fg)
                        ).frame(false));
                        if theme_btn.clicked() {
                            let themes = Theme::ALL;
                            let idx = themes.iter().position(|t| *t == self.app.settings.theme).unwrap_or(0);
                            self.app.settings.theme = themes[(idx + 1) % themes.len()];
                            self.app.settings.save();
                        }
                        theme_btn.on_hover_text("Click to switch theme");
                        if dirty {
                            ui.label(RichText::new("● Modified").font(small()).color(tc.orange));
                        }
                        let err_count = ed.diagnostics.len();
                        if err_count > 0 {
                            ui.label(RichText::new(format!("⚠ {}", err_count)).font(small()).color(Color32::from_rgb(247, 118, 142)));
                        }
                        if self.app.auto_save_enabled {
                            ui.label(RichText::new("Auto-Save").font(FontId::monospace(10.0)).color(tc.green));
                        }
                        let font_size = self.app.settings.font_size;
                        ui.label(RichText::new(format!("{}px", font_size as u32)).font(small()).color(tc.fg_dim));
                        ui.label(RichText::new(lang).font(small()).color(tc.fg_dim));
                        ui.label(RichText::new(format!("Ln {}, Col {}", line, col)).font(small()).color(tc.fg));
                        ui.label(RichText::new("UTF-8").font(small()).color(tc.fg_dim));
                    });
                });
            });
    }

    fn render_sidebar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("sidebar")
            .default_width(240.0).min_width(150.0).max_width(400.0)
            .frame(egui::Frame::NONE.fill(self.tc.sidebar_bg).inner_margin(0.0))
            .show(ctx, |ui| {
                // Right border
                let r = ui.max_rect();
                ui.painter().line_segment(
                    [Pos2::new(r.max.x, r.min.y), Pos2::new(r.max.x, r.max.y)],
                    Stroke::new(1.0, self.tc.border),
                );

                // Tab buttons
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    for (label, tab) in [("  Files  ", SidebarTab::Files), ("  Git  ", SidebarTab::Git), ("  Search  ", SidebarTab::Search)] {
                        let active = self.app.sidebar_tab == tab;
                        let c = if active { self.tc.accent } else { self.tc.fg_dim };
                        let btn = egui::Button::new(RichText::new(label).font(small()).color(c))
                            .fill(Color32::TRANSPARENT).rounding(Rounding::ZERO).stroke(Stroke::NONE);
                        let resp = ui.add(btn);
                        if resp.clicked() { self.app.sidebar_tab = tab; }
                        // Active indicator line
                        if active {
                            let rect = resp.rect;
                            ui.painter().line_segment(
                                [Pos2::new(rect.min.x + 4.0, rect.max.y), Pos2::new(rect.max.x - 4.0, rect.max.y)],
                                Stroke::new(2.0, self.tc.accent),
                            );
                        }
                    }
                });
                ui.add_space(2.0);
                ui.painter().line_segment(
                    [Pos2::new(ui.max_rect().min.x, ui.cursor().min.y), Pos2::new(ui.max_rect().max.x, ui.cursor().min.y)],
                    Stroke::new(1.0, self.tc.border),
                );
                ui.add_space(4.0);

                match self.app.sidebar_tab {
                    SidebarTab::Files => self.render_file_tree(ui),
                    SidebarTab::Git => self.render_git(ui),
                    SidebarTab::Search => self.render_search(ui),
                }
            });
    }

    fn render_file_tree(&mut self, ui: &mut egui::Ui) {
        let dark = self.app.settings.theme != Theme::Light;
        // Toolbar
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.add_space(8.0);
            for (icon, tip, action) in [
                ("+", "New File", 0u8),
                ("□", "New Folder", 1),
                ("↻", "Refresh", 2),
            ] {
                if ui.add(egui::Button::new(RichText::new(icon).font(small()).color(self.tc.fg_dim))
                    .fill(Color32::TRANSPARENT).min_size(Vec2::new(22.0, 18.0))
                    .rounding(Rounding::same(3)))
                    .on_hover_text(tip).clicked()
                {
                    match action {
                        0 => self.app.start_new_file_dialog(),
                        1 => self.app.start_new_folder_dialog(),
                        _ => self.app.file_tree.refresh(),
                    }
                }
            }
        });
        ui.add_space(2.0);

        let row_h = 28.0;
        let indent_px = 18.0;
        let guide_color = Color32::from_rgba_premultiplied(60, 65, 90, 50);

        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            let entries = self.app.file_tree.flat_entries.clone();
            let drag_source_idx = self.drag_source.as_ref().map(|(i, _)| *i);
            // Reset drop target each frame — will be re-set below if pointer is over a valid target
            if self.drag_source.is_some() {
                self.drop_target = None;
            }
            let mut new_drop_target: Option<(usize, String)> = None;

            let pointer_pos = ui.ctx().input(|i| i.pointer.hover_pos());

            for (i, entry) in entries.iter().enumerate() {
                let depth = entry.depth;
                let indent = indent_px * depth as f32 + 8.0;
                let sel = i == self.app.file_tree.selected_index;
                let is_dragged = drag_source_idx == Some(i);

                // Allocate row rect
                let (row_rect, row_resp) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), row_h),
                    egui::Sense::click_and_drag(),
                );

                // Determine if this row is the drop target (by pointer position)
                let is_drop = if self.drag_source.is_some() && entry.is_directory {
                    pointer_pos.is_some_and(|p| row_rect.contains(p))
                } else {
                    false
                };

                let drop_bg = if dark { Color32::from_rgb(35, 50, 80) } else { Color32::from_rgb(210, 225, 245) };
                let bg = if is_drop { drop_bg } else if sel { self.tc.selection_bg } else { Color32::TRANSPARENT };

                // Background
                if bg != Color32::TRANSPARENT {
                    ui.painter().rect_filled(row_rect, Rounding::ZERO, bg);
                }

                // Drop target highlight — prominent border + glow
                if is_drop {
                    ui.painter().rect_stroke(row_rect, Rounding::same(3), Stroke::new(2.0, self.tc.accent), egui::StrokeKind::Outside);
                    // Left accent bar
                    ui.painter().rect_filled(
                        Rect::from_min_size(row_rect.min, Vec2::new(3.0, row_rect.height())),
                        Rounding::ZERO,
                        self.tc.accent,
                    );
                }

                // Indent guide lines — subtle gray like JetBrains
                let painter = ui.painter();
                let guide_c = if dark { Color32::from_rgb(57, 57, 57) } else { Color32::from_rgb(228, 228, 228) };
                for d in 1..depth + 1 {
                    let gx = row_rect.min.x + indent_px * d as f32 + 4.0;
                    painter.line_segment(
                        [Pos2::new(gx, row_rect.min.y), Pos2::new(gx, row_rect.max.y)],
                        Stroke::new(1.0, guide_c),
                    );
                }

                let dim = if is_dragged { 0.35 } else { 1.0 };
                let row_y = row_rect.min.y + (row_h - 13.0) / 2.0;
                let text_color = if entry.name.starts_with('.') {
                    self.tc.fg_dim
                } else {
                    self.tc.fg
                }.linear_multiply(dim);
                let arrow_color = if dark {
                    Color32::from_rgb(140, 140, 140)
                } else {
                    Color32::from_rgb(130, 130, 130)
                }.linear_multiply(dim);

                if entry.is_directory {
                    // Arrow
                    let arr = if entry.is_expanded { "▾" } else { "▸" };
                    painter.text(
                        Pos2::new(row_rect.min.x + indent, row_y),
                        egui::Align2::LEFT_TOP,
                        arr,
                        FontId::monospace(12.0),
                        arrow_color,
                    );
                    // Folder name (bold-ish via slightly larger font)
                    painter.text(
                        Pos2::new(row_rect.min.x + indent + 14.0, row_y),
                        egui::Align2::LEFT_TOP,
                        &entry.name,
                        FontId::monospace(13.0),
                        text_color,
                    );
                } else {
                    // Colored dot indicator for file type
                    let dot_color = file_icon_color(&entry.name, dark).linear_multiply(dim);
                    let dot_y = row_rect.min.y + row_h / 2.0;
                    painter.circle_filled(
                        Pos2::new(row_rect.min.x + indent + 5.0, dot_y),
                        3.5,
                        dot_color,
                    );
                    // File name
                    painter.text(
                        Pos2::new(row_rect.min.x + indent + 14.0, row_y),
                        egui::Align2::LEFT_TOP,
                        &entry.name,
                        FontId::monospace(13.0),
                        text_color,
                    );
                }

                // Drag start
                if row_resp.drag_started() {
                    self.drag_source = Some((i, entry.path.clone()));
                }

                // Click — only if NOT dragging
                if row_resp.clicked() && self.drag_source.is_none() {
                    self.app.file_tree.selected_index = i;
                    if entry.is_directory {
                        self.app.file_tree.toggle_expand(i);
                    } else {
                        let p = entry.path.clone();
                        self.app.open_file(&p);
                    }
                }

                // Drag hover — detect drop target by pointer position
                if self.drag_source.is_some() {
                    if let Some(pp) = pointer_pos {
                        if row_rect.contains(pp) {
                            if entry.is_directory {
                                new_drop_target = Some((i, entry.path.clone()));
                            } else if let Some(parent) = std::path::Path::new(&entry.path).parent() {
                                new_drop_target = Some((i, parent.to_string_lossy().to_string()));
                            }
                        }
                    }
                }

                // Context menu
                row_resp.context_menu(|ui| {
                    self.app.file_tree.selected_index = i;
                    if ui.button("New File Here").clicked() { self.app.start_new_file_dialog(); ui.close_menu(); }
                    if ui.button("New Folder Here").clicked() { self.app.start_new_folder_dialog(); ui.close_menu(); }
                    ui.separator();
                    if ui.button("Rename").clicked() { self.app.start_rename_dialog(); ui.close_menu(); }
                    if ui.button("Duplicate").clicked() {
                        let p = entry.path.clone();
                        let _ = self.app.file_tree.duplicate_entry(&p);
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Delete").clicked() { self.app.focus = Focus::DeleteConfirm; ui.close_menu(); }
                });
            }

            // Apply detected drop target
            if let Some(dt) = new_drop_target {
                self.drop_target = Some(dt);
            }
        });

        // Handle drop (drag released)
        if ui.input(|i| i.pointer.any_released()) {
            if let (Some((_, src_path)), Some((_, dst_dir))) = (self.drag_source.take(), self.drop_target.take()) {
                // Don't drop onto itself or its own parent
                let src = std::path::Path::new(&src_path);
                let dst = std::path::Path::new(&dst_dir);
                let src_parent = src.parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
                if src_path != dst_dir && src_parent != dst_dir && !dst_dir.starts_with(&src_path) {
                    if let Some(name) = src.file_name() {
                        let new_path = dst.join(name);
                        if !new_path.exists() {
                            if let Err(e) = std::fs::rename(&src_path, &new_path) {
                                self.app.status_message = format!("Move error: {}", e);
                            } else {
                                self.app.status_message = format!("Moved to {}", dst_dir);
                                self.app.file_tree.refresh();
                                self.app.file_tree.ensure_expanded(&dst_dir);
                            }
                        } else {
                            self.app.status_message = format!("Already exists: {}", new_path.display());
                        }
                    }
                }
            }
            self.drag_source = None;
            self.drop_target = None;
        }
    }

    fn render_git(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        if let Some(ref status) = self.app.git_status.clone() {
            if status.is_repo {
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    ui.label(RichText::new(format!("⎇ {}", status.branch)).font(small()).color(self.tc.accent));
                });
                ui.add_space(4.0);
                ui.painter().line_segment(
                    [Pos2::new(ui.max_rect().min.x, ui.cursor().min.y), Pos2::new(ui.max_rect().max.x, ui.cursor().min.y)],
                    Stroke::new(1.0, self.tc.border),
                );
                ui.add_space(4.0);
                for f in &status.files {
                    let c = if f.staged { self.tc.green } else { self.tc.orange };
                    let prefix = if f.staged { "✓" } else { "●" };
                    ui.horizontal(|ui| {
                        ui.add_space(12.0);
                        ui.label(RichText::new(format!("{} {} {}", prefix, f.status.symbol(), f.path)).font(small()).color(c));
                    });
                }
                if status.files.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(20.0);
                        ui.label(RichText::new("No changes").font(small()).color(self.tc.fg_dim));
                    });
                }
            } else {
                ui.vertical_centered(|ui| {
                    ui.add_space(20.0);
                    ui.label(RichText::new("Not a git repository").font(small()).color(self.tc.fg_dim));
                });
            }
        }
    }

    fn render_search(&mut self, ui: &mut egui::Ui) {
        let tc = self.tc;
        let dark = self.app.settings.theme != Theme::Light;
        ui.add_space(6.0);

        // Search input
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.label(RichText::new("🔍").font(small()));
            let resp = ui.add(
                egui::TextEdit::singleline(&mut self.app.global_search_input)
                    .font(small())
                    .desired_width(ui.available_width() - 12.0)
                    .hint_text("Search in files...")
            );
            if resp.changed() && self.app.global_search_input.len() >= 2 {
                self.do_search();
            }
            if resp.changed() && self.app.global_search_input.is_empty() {
                self.app.global_search_results.clear();
                self.app.file_search_results.clear();
            }
        });

        ui.add_space(4.0);

        // Options row
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            let case_btn = if self.app.find_case_sensitive { "Aa ●" } else { "Aa" };
            if ui.add(egui::Button::new(RichText::new(case_btn).font(FontId::monospace(10.0)).color(tc.fg_dim))
                .fill(if self.app.find_case_sensitive { tc.selection_bg } else { Color32::TRANSPARENT })
                .min_size(Vec2::new(28.0, 16.0))).on_hover_text("Case sensitive").clicked()
            {
                self.app.find_case_sensitive = !self.app.find_case_sensitive;
                if self.app.global_search_input.len() >= 2 { self.do_search(); }
            }
            let regex_btn = if self.app.find_use_regex { ".* ●" } else { ".*" };
            if ui.add(egui::Button::new(RichText::new(regex_btn).font(FontId::monospace(10.0)).color(tc.fg_dim))
                .fill(if self.app.find_use_regex { tc.selection_bg } else { Color32::TRANSPARENT })
                .min_size(Vec2::new(28.0, 16.0))).on_hover_text("Use regex").clicked()
            {
                self.app.find_use_regex = !self.app.find_use_regex;
                if self.app.global_search_input.len() >= 2 { self.do_search(); }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(8.0);
                let fc = self.app.file_search_results.len();
                let cc = self.app.global_search_results.len();
                let label = format!("{} files, {} lines", fc, if cc >= 1000 { "1000+".to_string() } else { cc.to_string() });
                ui.label(RichText::new(label).font(FontId::monospace(10.0)).color(tc.fg_dim));
            });
        });

        ui.add_space(2.0);
        ui.painter().line_segment(
            [Pos2::new(ui.max_rect().min.x, ui.cursor().min.y), Pos2::new(ui.max_rect().max.x, ui.cursor().min.y)],
            Stroke::new(1.0, tc.border),
        );
        ui.add_space(2.0);

        // Results
        let has_file_results = !self.app.file_search_results.is_empty();
        let has_content_results = !self.app.global_search_results.is_empty();
        if !has_file_results && !has_content_results && !self.app.global_search_input.is_empty() && self.app.global_search_input.len() >= 2 {
            ui.add_space(20.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("No results found").font(small()).color(tc.fg_dim));
            });
            return;
        }

        let file_results = self.app.file_search_results.clone();
        let results = self.app.global_search_results.clone();
        let root = self.app.file_tree.root_path.clone().unwrap_or_default();

        // Group results by file
        let mut grouped: Vec<(String, String, Vec<(usize, &crate::search::SearchResult)>)> = Vec::new();
        for (idx, r) in results.iter().enumerate() {
            let rel_path = r.file_path.strip_prefix(&root).unwrap_or(&r.file_path)
                .trim_start_matches('/').to_string();
            if let Some(group) = grouped.last_mut() {
                if group.0 == r.file_path {
                    group.2.push((idx, r));
                    continue;
                }
            }
            grouped.push((r.file_path.clone(), rel_path, vec![(idx, r)]));
        }

        let mut open_file_path: Option<String> = None;
        let mut open_file_line: Option<(String, usize, usize)> = None;

        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;

            // === File name matches ===
            if !file_results.is_empty() {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    ui.label(RichText::new(format!("📂 Files ({})", file_results.len()))
                        .font(FontId::monospace(11.0)).color(tc.accent).strong());
                });
                ui.add_space(2.0);

                for fm in &file_results {
                    let (row_rect, row_resp) = ui.allocate_exact_size(
                        Vec2::new(ui.available_width(), 22.0),
                        egui::Sense::click(),
                    );
                    if row_resp.hovered() {
                        ui.painter().rect_filled(row_rect, Rounding::ZERO, tc.selection_bg);
                    }
                    let dot_c = file_icon_color(&fm.file_name, dark);
                    let painter = ui.painter();
                    painter.circle_filled(
                        Pos2::new(row_rect.min.x + 16.0, row_rect.min.y + 11.0),
                        3.0, dot_c,
                    );
                    painter.text(
                        Pos2::new(row_rect.min.x + 24.0, row_rect.min.y + 4.0),
                        egui::Align2::LEFT_TOP, &fm.file_name, FontId::monospace(11.0), tc.fg,
                    );
                    let name_w = painter.text(
                        Pos2::new(row_rect.min.x + 24.0, row_rect.min.y + 4.0),
                        egui::Align2::LEFT_TOP, &fm.file_name, FontId::monospace(11.0), tc.fg,
                    ).width();
                    painter.text(
                        Pos2::new(row_rect.min.x + 32.0 + name_w, row_rect.min.y + 4.0),
                        egui::Align2::LEFT_TOP,
                        &fm.rel_path, FontId::monospace(10.0), tc.fg_dim,
                    );

                    if row_resp.clicked() {
                        open_file_path = Some(fm.file_path.clone());
                    }
                }

                // Separator between file results and content results
                if !results.is_empty() {
                    ui.add_space(6.0);
                    ui.painter().line_segment(
                        [Pos2::new(ui.max_rect().min.x + 8.0, ui.cursor().min.y),
                         Pos2::new(ui.max_rect().max.x - 8.0, ui.cursor().min.y)],
                        Stroke::new(1.0, tc.border),
                    );
                    ui.add_space(4.0);
                }
            }

            // === Content matches ===
            if !results.is_empty() {
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    ui.label(RichText::new(format!("📄 Content ({})", results.len()))
                        .font(FontId::monospace(11.0)).color(tc.accent).strong());
                });
                ui.add_space(2.0);
            }

            for (file_path, rel_path, matches) in &grouped {
                // File header
                let file_name = std::path::Path::new(file_path)
                    .file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                let dot_c = file_icon_color(&file_name, dark);

                ui.add_space(4.0);
                let header_resp = ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    // Colored dot
                    let (r, _) = ui.allocate_exact_size(Vec2::new(8.0, 14.0), egui::Sense::hover());
                    ui.painter().circle_filled(Pos2::new(r.min.x + 4.0, r.center().y), 3.0, dot_c);
                    ui.label(RichText::new(&file_name).font(FontId::monospace(11.0)).color(tc.fg).strong());
                    ui.label(RichText::new(format!("  {}", rel_path)).font(FontId::monospace(10.0)).color(tc.fg_dim));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(8.0);
                        ui.label(RichText::new(format!("{}", matches.len())).font(FontId::monospace(10.0)).color(tc.accent));
                    });
                }).response;

                // Match lines
                for (_idx, m) in matches {
                    let (row_rect, row_resp) = ui.allocate_exact_size(
                        Vec2::new(ui.available_width(), 20.0),
                        egui::Sense::click(),
                    );

                    let hovered = row_resp.hovered();
                    if hovered {
                        ui.painter().rect_filled(row_rect, Rounding::ZERO, tc.selection_bg);
                    }

                    let painter = ui.painter();
                    // Line number
                    painter.text(
                        Pos2::new(row_rect.min.x + 16.0, row_rect.min.y + 3.0),
                        egui::Align2::LEFT_TOP,
                        format!("{}", m.line_number),
                        FontId::monospace(10.0),
                        tc.gutter_fg,
                    );

                    // Line content with match highlight
                    let line = &m.line_content;
                    let trimmed = line.trim_start();
                    let trim_offset = line.len() - trimmed.len();
                    let display_line: String = if trimmed.len() > 80 {
                        trimmed.chars().take(80).collect::<String>() + "…"
                    } else {
                        trimmed.to_string()
                    };

                    // Draw the line text with safe char-boundary slicing
                    let text_x = row_rect.min.x + 52.0;
                    let text_y = row_rect.min.y + 3.0;

                    // Convert byte offsets to safe char boundaries on display_line
                    let hl_start_byte = m.match_start.saturating_sub(trim_offset);
                    let hl_end_byte = m.match_end.saturating_sub(trim_offset);

                    // Find nearest char boundaries
                    let safe_start = display_line.char_indices()
                        .map(|(i, _)| i)
                        .find(|&i| i >= hl_start_byte)
                        .unwrap_or(display_line.len());
                    let safe_end = display_line.char_indices()
                        .map(|(i, _)| i)
                        .find(|&i| i >= hl_end_byte)
                        .unwrap_or(display_line.len());

                    if safe_start < display_line.len() && safe_end <= display_line.len() && safe_start < safe_end {
                        let pre = &display_line[..safe_start];
                        let match_text = &display_line[safe_start..safe_end];
                        let post = &display_line[safe_end..];

                        let pre_w = painter.text(
                            Pos2::new(text_x, text_y), egui::Align2::LEFT_TOP,
                            pre, FontId::monospace(10.0), tc.fg_dim,
                        ).width();
                        // Background behind match
                        let match_w = painter.text(
                            Pos2::new(text_x + pre_w, text_y), egui::Align2::LEFT_TOP,
                            match_text, FontId::monospace(10.0), tc.accent,
                        ).width();
                        painter.rect_filled(
                            Rect::from_min_size(
                                Pos2::new(text_x + pre_w - 1.0, text_y - 1.0),
                                Vec2::new(match_w + 2.0, 13.0),
                            ),
                            Rounding::same(2),
                            tc.accent.linear_multiply(0.15),
                        );
                        painter.text(
                            Pos2::new(text_x + pre_w, text_y), egui::Align2::LEFT_TOP,
                            match_text, FontId::monospace(10.0), tc.accent,
                        );
                        if !post.is_empty() {
                            painter.text(
                                Pos2::new(text_x + pre_w + match_w, text_y), egui::Align2::LEFT_TOP,
                                post, FontId::monospace(10.0), tc.fg_dim,
                            );
                        }
                    } else {
                        painter.text(
                            Pos2::new(text_x, text_y), egui::Align2::LEFT_TOP,
                            &display_line, FontId::monospace(10.0), tc.fg_dim,
                        );
                    }

                    // Click to open file at line
                    if row_resp.clicked() {
                        open_file_line = Some((m.file_path.clone(), m.line_number.saturating_sub(1), m.match_start));
                    }
                }
            }
        });

        // Handle deferred clicks from search results
        if let Some(fp) = open_file_path {
            if std::path::Path::new(&fp).is_dir() {
                self.app.open_folder(fp);
            } else {
                self.app.open_file(&fp);
                self.app.file_tree.reveal_path(&fp);
                self.app.sidebar_tab = SidebarTab::Files;
                self.app.focus = Focus::Editor;
            }
        }
        if let Some((fp, line, col)) = open_file_line {
            self.app.open_file(&fp);
            self.app.file_tree.reveal_path(&fp);
            self.app.sidebar_tab = SidebarTab::Files;
            self.app.active_editor_mut().cursor.line = line;
            self.app.active_editor_mut().cursor.col = col;
            self.app.active_editor_mut().scroll_into_view();
            self.app.focus = Focus::Editor;
        }
    }

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

    fn render_editor(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().frame(egui::Frame::NONE.fill(self.tc.bg)).show(ctx, |ui| {
            // Welcome screen
            let has_project = self.app.file_tree.root_path.is_some();
            let ed_empty = self.app.editors.len() == 1
                && self.app.editors[0].file_path.is_none()
                && self.app.editors[0].line_count() <= 1;
            if !has_project && ed_empty {
                ui.vertical_centered(|ui| {
                    ui.add_space(ui.available_height() * 0.2);
                    ui.label(RichText::new("Code Editor").font(FontId::monospace(28.0)).color(self.tc.accent));
                    ui.add_space(8.0);
                    ui.label(RichText::new("Lightweight • Fast • Native").font(FontId::monospace(14.0)).color(self.tc.gutter_fg));
                    ui.add_space(32.0);

                    let btn_width = 220.0;
                    let btn_height = 36.0;
                    let btn_bg = self.tc.sidebar_bg;
                    let btn_text = self.tc.accent;
                    if ui.add_sized([btn_width, btn_height],
                        egui::Button::new(RichText::new("  Open Folder…  ").font(FontId::monospace(14.0)).color(btn_text))
                            .fill(btn_bg).rounding(Rounding::same(6))
                            .stroke(Stroke::new(1.0, self.tc.border))
                    ).clicked() {
                        self.app.pending_action = Some(PaletteAction::OpenFolder);
                    }
                    ui.add_space(8.0);
                    if ui.add_sized([btn_width, btn_height],
                        egui::Button::new(RichText::new("  Open File…  ").font(FontId::monospace(14.0)).color(btn_text))
                            .fill(btn_bg).rounding(Rounding::same(6))
                            .stroke(Stroke::new(1.0, self.tc.border))
                    ).clicked() {
                        self.app.pending_action = Some(PaletteAction::OpenFile);
                    }

                    ui.add_space(40.0);
                    ui.label(RichText::new("Keyboard Shortcuts").font(FontId::monospace(13.0)).color(self.tc.fg_dim));
                    ui.add_space(8.0);
                    for (keys, desc) in [
                        ("⌘+Shift+P", "Command Palette"),
                        ("⌘+P", "Quick Open File"),
                        ("⌘+O", "Open File"),
                        ("⌘+S", "Save"),
                        ("⌘+B", "Toggle Sidebar"),
                    ] {
                        ui.horizontal(|ui| {
                            let total = 260.0;
                            let offset = (ui.available_width() - total) / 2.0;
                            ui.add_space(offset);
                            ui.label(RichText::new(format!("{:<16}", keys)).font(small()).color(self.tc.accent));
                            ui.label(RichText::new(desc).font(small()).color(self.tc.fg_dim));
                        });
                    }
                });
                return;
            }

            // Breadcrumbs bar
            if self.app.show_breadcrumbs {
                let ed = &self.app.editors[self.app.active_editor];
                if let Some(ref fp) = ed.file_path {
                    let root = self.app.file_tree.root_path.as_deref().unwrap_or("");
                    let rel = fp.strip_prefix(root).unwrap_or(fp).trim_start_matches('/');
                    let parts: Vec<&str> = rel.split('/').collect();
                    let dark = self.app.settings.theme != Theme::Light;
                    let bc_bg = if dark { Color32::from_rgb(37, 37, 37) } else { Color32::from_rgb(245, 245, 245) };
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 22.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, Rounding::ZERO, bc_bg);
                    let mut x = rect.min.x + 12.0;
                    for (i, part) in parts.iter().enumerate() {
                        if i > 0 {
                            let sep_r = ui.painter().text(
                                Pos2::new(x, rect.min.y + 4.0), egui::Align2::LEFT_TOP,
                                " › ", FontId::monospace(11.0), self.tc.fg_dim,
                            );
                            x += sep_r.width();
                        }
                        let is_last = i == parts.len() - 1;
                        let color = if is_last { self.tc.fg } else { self.tc.fg_dim };
                        let tr = ui.painter().text(
                            Pos2::new(x, rect.min.y + 4.0), egui::Align2::LEFT_TOP,
                            *part, FontId::monospace(11.0), color,
                        );
                        x += tr.width();
                    }
                    // Bottom border
                    ui.painter().line_segment(
                        [Pos2::new(rect.min.x, rect.max.y), Pos2::new(rect.max.x, rect.max.y)],
                        Stroke::new(1.0, self.tc.border),
                    );
                }
            }

            let dark = self.app.settings.theme != Theme::Light;
            let fs = self.app.settings.font_size;
            let font = mono_sized(fs);
            let sfont = small_sized(fs);
            let cw = ui.fonts(|f| f.glyph_width(&font, ' '));
            let lh = ui.fonts(|f| f.row_height(&font)) + LINE_SPACING;
            let show_ln = self.app.settings.show_line_numbers;

            // Recompute fold ranges when file is opened or after edits
            {
                let ed = &mut self.app.editors[self.app.active_editor];
                if ed.fold_ranges.is_empty() || ed.is_dirty {
                    ed.compute_fold_ranges();
                }
            }

            // Recompute diagnostics when content changes
            {
                let ed = &mut self.app.editors[self.app.active_editor];
                if ed.diagnostics_dirty {
                    let lang_for_diag = ed.file_path.as_ref().map(|p| syntax::detect_language(p).to_string()).unwrap_or("text".into());
                    let content = ed.buffer.text();
                    ed.diagnostics = syntax::check_syntax(&content, &lang_for_diag);
                    ed.diagnostics_dirty = false;
                }
            }

            let ed = &self.app.editors[self.app.active_editor];
            let lc = ed.line_count();
            let lang = ed.file_path.as_ref().map(|p| syntax::detect_language(p).to_string()).unwrap_or("text".into());
            let gutter_digits = format!("{}", lc).len().max(3);
            // Extra space in gutter for fold arrows
            let fold_w = cw * 1.8;
            let gw = if show_ln { cw * (gutter_digits as f32 + 1.5) + fold_w } else { fold_w };

            // Find matching bracket for cursor
            let bracket_match = find_matching_bracket(&self.app, ed.cursor.line, ed.cursor.col);
            // bracket_depths computed after vis_lines below

            // Autocomplete key handling (before regular input)
            if self.app.show_autocomplete && self.app.focus == Focus::Editor {
                let ac_up = ctx.input(|i| i.key_pressed(egui::Key::ArrowUp));
                let ac_down = ctx.input(|i| i.key_pressed(egui::Key::ArrowDown));
                let ac_accept = ctx.input(|i| i.key_pressed(egui::Key::Tab))
                    || ctx.input(|i| i.key_pressed(egui::Key::Enter));
                let ac_dismiss = ctx.input(|i| i.key_pressed(egui::Key::Escape));
                if ac_up && self.app.autocomplete_selected > 0 {
                    self.app.autocomplete_selected -= 1;
                }
                if ac_down && self.app.autocomplete_selected + 1 < self.app.autocomplete_suggestions.len() {
                    self.app.autocomplete_selected += 1;
                }
                if ac_accept {
                    self.app.accept_autocomplete();
                }
                if ac_dismiss || ac_accept {
                    self.app.show_autocomplete = false;
                }
                // Skip normal key processing when autocomplete consumed the key
                if ac_up || ac_down || ac_accept || ac_dismiss {
                    // still need to continue to paint, just skip input
                }
            }

            // Input handling
            if self.app.focus == Focus::Editor {
                for event in &ctx.input(|i| i.events.clone()) {
                    match event {
                        egui::Event::Text(text) => {
                            let ed = &mut self.app.editors[self.app.active_editor];
                            for c in text.chars() {
                                ed.selection = None;
                                ed.insert_char(c);
                                if let Some(cc) = match c {
                                    '(' => Some(')'), '[' => Some(']'), '{' => Some('}'),
                                    '"' => Some('"'), '\'' => Some('\''),
                                    _ => None
                                } {
                                    let (l, co) = (ed.cursor.line, ed.cursor.col);
                                    ed.buffer.insert_char(l, co, cc);
                                }
                            }
                            ed.scroll_into_view();
                            // Trigger autocomplete
                            self.app.trigger_autocomplete();
                        }
                        egui::Event::Key { key, pressed: true, modifiers, .. } => {
                            // Skip if autocomplete already handled this key
                            if self.app.show_autocomplete && matches!(key,
                                egui::Key::ArrowUp | egui::Key::ArrowDown |
                                egui::Key::Tab | egui::Key::Enter | egui::Key::Escape
                            ) {
                                continue;
                            }
                            let ed = &mut self.app.editors[self.app.active_editor];
                            match key {
                                egui::Key::ArrowUp if modifiers.alt => ed.move_line_up(),
                                egui::Key::ArrowDown if modifiers.alt => ed.move_line_down(),
                                egui::Key::ArrowUp => { ed.move_up(); self.app.show_autocomplete = false; },
                                egui::Key::ArrowDown => { ed.move_down(); self.app.show_autocomplete = false; },
                                egui::Key::ArrowLeft if modifiers.alt => ed.move_word_left(),
                                egui::Key::ArrowRight if modifiers.alt => ed.move_word_right(),
                                egui::Key::ArrowLeft => { ed.move_left(); self.app.show_autocomplete = false; },
                                egui::Key::ArrowRight => { ed.move_right(); self.app.show_autocomplete = false; },
                                egui::Key::Home => ed.move_home(),
                                egui::Key::End => ed.move_end(),
                                egui::Key::Backspace => { ed.delete_back(); self.app.show_autocomplete = false; },
                                egui::Key::Delete => ed.delete_forward(),
                                egui::Key::Enter => ed.insert_newline(),
                                egui::Key::Tab if modifiers.shift => ed.outdent(),
                                egui::Key::Tab => ed.insert_tab(),
                                egui::Key::D if modifiers.command && modifiers.shift => ed.duplicate_line(),
                                egui::Key::D if modifiers.command => ed.select_next_occurrence(),
                                egui::Key::K if modifiers.command => ed.delete_line(),
                                egui::Key::Slash if modifiers.command => ed.toggle_comment(),
                                egui::Key::PageUp => ed.page_up(),
                                egui::Key::PageDown => ed.page_down(),
                                egui::Key::A if modifiers.command => ed.select_all(),
                                _ => {}
                            }
                            ed.scroll_into_view();
                        }
                        _ => {}
                    }
                }
            }

            // Paint editor
            let avail = ui.available_size();
            let (rect, response) = ui.allocate_exact_size(avail, egui::Sense::click_and_drag());

            // Click handling (skip if click is in minimap area)
            let minimap_w_check = if self.app.show_minimap { 80.0 } else { 0.0 };
            if response.clicked() {
                self.app.focus = Focus::Editor;
                if let Some(pos) = response.interact_pointer_pos() {
                    // Skip click if it's on the minimap
                    if !(self.app.show_minimap && pos.x > rect.max.x - minimap_w_check) {
                        let rel = pos - rect.min;
                        let row_idx = (rel.y / lh) as usize;
                        let ed = &mut self.app.editors[self.app.active_editor];
                        let vis_lines = ed.visible_lines(ed.scroll_offset, ed.viewport_height + 2);
                        let actual_line = vis_lines.get(row_idx).copied().unwrap_or(lc.saturating_sub(1)).min(lc.saturating_sub(1));

                        // Check if click is in the fold gutter area
                        if rel.x < gw - fold_w + fold_w {
                            // Check if this line has a fold range
                            if ed.fold_ranges.contains_key(&actual_line) {
                                ed.toggle_fold(actual_line);
                            } else {
                                ed.cursor.line = actual_line;
                                ed.cursor.col = 0;
                            }
                        } else {
                            let cc = ((rel.x - gw).max(0.0) / cw) as usize;
                            ed.cursor.line = actual_line;
                            ed.cursor.col = cc.min(ed.buffer.line_len(actual_line));
                        }
                        ed.selection = None;
                        // Clear extra cursors on single click
                        ed.extra_cursors.clear();
                    }
                }
            }

            // Scroll (⌘+scroll = zoom, normal scroll = scroll)
            let cmd_held = ui.input(|i| i.modifiers.command);
            let sd = ui.input(|i| i.smooth_scroll_delta.y);
            if sd != 0.0 {
                if cmd_held {
                    let delta = if sd > 0.0 { 1.0 } else { -1.0 };
                    self.app.settings.font_size = (self.app.settings.font_size + delta).clamp(8.0, 32.0);
                    self.app.settings.save();
                } else {
                    let ed = &mut self.app.editors[self.app.active_editor];
                    let lines = (-sd / lh) as isize;
                    if lines > 0 { ed.scroll_offset = (ed.scroll_offset + lines as usize).min(lc.saturating_sub(1)); }
                    else if lines < 0 { ed.scroll_offset = ed.scroll_offset.saturating_sub((-lines) as usize); }
                }
            }

            let painter = ui.painter_at(rect);
            let vis = (rect.height() / lh) as usize;
            self.app.editors[self.app.active_editor].viewport_height = vis.max(1);
            let ed = &self.app.editors[self.app.active_editor];
            let so = ed.scroll_offset;
            let text_y_offset = LINE_SPACING / 2.0;

            // Get visible lines (accounting for folds)
            let vis_lines = ed.visible_lines(so, vis + 2);
            let bracket_depths = compute_bracket_depths(&self.app, &vis_lines);

            // Gutter separator
            if show_ln {
                painter.line_segment(
                    [Pos2::new(rect.min.x + gw - fold_w - 2.0, rect.min.y), Pos2::new(rect.min.x + gw - fold_w - 2.0, rect.max.y)],
                    Stroke::new(1.0, self.tc.border),
                );
            }

            for (row, &li) in vis_lines.iter().enumerate() {
                let y = rect.min.y + row as f32 * lh;
                if y > rect.max.y { break; }

                // Current line highlight
                if li == ed.cursor.line {
                    painter.rect_filled(
                        Rect::from_min_size(Pos2::new(rect.min.x, y), Vec2::new(rect.width(), lh)),
                        Rounding::ZERO, self.tc.current_line_bg,
                    );
                }

                // Git diff gutter indicator
                if let Some(diff_status) = ed.line_diff.get(&li) {
                    let diff_color = match diff_status {
                        crate::editor::LineDiffStatus::Added => self.tc.green,
                        crate::editor::LineDiffStatus::Modified => Color32::from_rgb(70, 140, 220),
                    };
                    painter.rect_filled(
                        Rect::from_min_size(Pos2::new(rect.min.x + 1.0, y), Vec2::new(3.0, lh)),
                        Rounding::ZERO, diff_color,
                    );
                }

                // Line numbers
                if show_ln {
                    let nc = if li == ed.cursor.line { self.tc.fg } else { self.tc.gutter_fg };
                    painter.text(
                        Pos2::new(rect.min.x + gw - fold_w - cw * 0.8, y + text_y_offset),
                        egui::Align2::RIGHT_TOP,
                        format!("{}", li + 1),
                        sfont.clone(),
                        nc,
                    );
                }

                // Fold indicators in gutter
                let fold_x = rect.min.x + gw - fold_w + 2.0;
                if let Some(&fold_end) = ed.fold_ranges.get(&li) {
                    let is_folded = ed.folded.contains(&li);
                    let arrow = if is_folded { "▸" } else { "▾" };
                    let fold_color = if li == ed.cursor.line { self.tc.fg_dim } else { self.tc.fold_fg };
                    painter.text(
                        Pos2::new(fold_x, y + text_y_offset),
                        egui::Align2::LEFT_TOP,
                        arrow,
                        sfont.clone(),
                        fold_color,
                    );
                    // If folded, show indicator after the line
                    if is_folded {
                        let line = ed.buffer.get_line(li);
                        let line_end_x = rect.min.x + gw + line.chars().count() as f32 * cw + cw;
                        let folded_count = fold_end - li;
                        painter.rect_filled(
                            Rect::from_min_size(Pos2::new(line_end_x, y + 2.0), Vec2::new(cw * (folded_count.to_string().len() as f32 + 4.0), lh - 4.0)),
                            Rounding::same(3), if dark { Color32::from_rgb(40, 44, 65) } else { Color32::from_rgb(228, 228, 228) },
                        );
                        painter.text(
                            Pos2::new(line_end_x + cw * 0.5, y + text_y_offset),
                            egui::Align2::LEFT_TOP,
                            format!("⋯ {} lines", folded_count),
                            FontId::monospace((fs - 2.5).max(8.0)),
                            self.tc.fg_dim,
                        );
                    }
                }

                let line = ed.buffer.get_line(li);
                let hls = syntax::highlight_line(&line, &lang);
                let chars: Vec<char> = line.chars().collect();
                let xs = rect.min.x + gw;

                // Bracket match highlight
                if let Some((ml, mc)) = bracket_match {
                    if li == ml && mc < chars.len() {
                        painter.rect_filled(
                            Rect::from_min_size(Pos2::new(xs + mc as f32 * cw - 1.0, y), Vec2::new(cw + 2.0, lh)),
                            Rounding::same(2), self.tc.bracket_match_bg,
                        );
                    }
                }
                if li == ed.cursor.line && ed.cursor.col < chars.len() {
                    let ch = chars[ed.cursor.col];
                    if "()[]{}".contains(ch) && bracket_match.is_some() {
                        painter.rect_filled(
                            Rect::from_min_size(Pos2::new(xs + ed.cursor.col as f32 * cw - 1.0, y), Vec2::new(cw + 2.0, lh)),
                            Rounding::same(2), self.tc.bracket_match_bg,
                        );
                    }
                }

                // Indent guides
                if !chars.is_empty() {
                    let indent_spaces = chars.iter().take_while(|c| **c == ' ').count();
                    for g in (4..indent_spaces + 1).step_by(4) {
                        let gx = xs + g as f32 * cw;
                        painter.line_segment(
                            [Pos2::new(gx, y), Pos2::new(gx, y + lh)],
                            Stroke::new(1.0, if dark { Color32::from_rgb(57, 57, 57) } else { Color32::from_rgb(228, 228, 228) }),
                        );
                    }
                }

                // Render text with syntax highlighting + rainbow brackets
                if hls.is_empty() {
                    // No syntax highlights — still do rainbow brackets
                    let mut cx = xs;
                    for (ci, &ch) in chars.iter().enumerate() {
                        let color = if "()[]{}".contains(ch) {
                            if let Some(&depth) = bracket_depths.get(&(li, ci)) {
                                self.tc.bracket_colors[depth % self.tc.bracket_colors.len()]
                            } else { self.tc.fg }
                        } else { self.tc.fg };
                        painter.text(Pos2::new(cx, y + text_y_offset), egui::Align2::LEFT_TOP, String::from(ch), font.clone(), color);
                        cx += cw;
                    }
                } else {
                    // With syntax highlighting — render spans, but override bracket colors
                    let mut pos = 0;
                    for hl in &hls {
                        // Gap before this highlight span
                        if pos < hl.start && pos < chars.len() {
                            for ci in pos..hl.start.min(chars.len()) {
                                let ch = chars[ci];
                                let color = if "()[]{}".contains(ch) {
                                    if let Some(&depth) = bracket_depths.get(&(li, ci)) {
                                        self.tc.bracket_colors[depth % self.tc.bracket_colors.len()]
                                    } else { self.tc.fg }
                                } else { self.tc.fg };
                                painter.text(Pos2::new(xs + ci as f32 * cw, y + text_y_offset), egui::Align2::LEFT_TOP, String::from(ch), font.clone(), color);
                            }
                        }
                        // The highlight span itself
                        if hl.start < chars.len() {
                            let end = hl.end.min(chars.len());
                            for ci in hl.start..end {
                                let ch = chars[ci];
                                let color = if "()[]{}".contains(ch) {
                                    if let Some(&depth) = bracket_depths.get(&(li, ci)) {
                                        self.tc.bracket_colors[depth % self.tc.bracket_colors.len()]
                                    } else { hl.kind.color(dark) }
                                } else { hl.kind.color(dark) };
                                painter.text(Pos2::new(xs + ci as f32 * cw, y + text_y_offset), egui::Align2::LEFT_TOP, String::from(ch), font.clone(), color);
                            }
                        }
                        pos = hl.end;
                    }
                    // Remaining text after last highlight
                    if pos < chars.len() {
                        for ci in pos..chars.len() {
                            let ch = chars[ci];
                            let color = if "()[]{}".contains(ch) {
                                if let Some(&depth) = bracket_depths.get(&(li, ci)) {
                                    self.tc.bracket_colors[depth % self.tc.bracket_colors.len()]
                                } else { self.tc.fg }
                            } else { self.tc.fg };
                            painter.text(Pos2::new(xs + ci as f32 * cw, y + text_y_offset), egui::Align2::LEFT_TOP, String::from(ch), font.clone(), color);
                        }
                    }
                }

                // Draw error underlines for this line
                let error_color = Color32::from_rgb(247, 118, 142); // Red
                for diag in &ed.diagnostics {
                    if diag.line == li {
                        let err_x_start = xs + diag.col as f32 * cw;
                        let err_len = if diag.length > 0 { diag.length } else { 1 };
                        let err_x_end = xs + (diag.col + err_len) as f32 * cw;
                        let wave_y = y + lh - 2.0;

                        // Draw wavy underline
                        let mut points = Vec::new();
                        let mut wx = err_x_start;
                        let mut wave_up = true;
                        while wx < err_x_end {
                            let wy = if wave_up { wave_y - 1.5 } else { wave_y + 1.5 };
                            points.push(Pos2::new(wx, wy));
                            wx += 2.0;
                            wave_up = !wave_up;
                        }
                        if points.len() >= 2 {
                            for pair in points.windows(2) {
                                painter.line_segment(
                                    [pair[0], pair[1]],
                                    Stroke::new(1.2, error_color),
                                );
                            }
                        }

                        // Error dot in gutter
                        if show_ln {
                            painter.circle_filled(
                                Pos2::new(rect.min.x + 6.0, y + lh / 2.0),
                                3.0,
                                error_color,
                            );
                        }
                    }
                }
            }

            // Deferred minimap scroll (set by minimap click, applied after ed borrow ends)
            let mut minimap_new_scroll: Option<usize> = None;

            // Cursor (blinking)
            let cursor_row = vis_lines.iter().position(|&l| l == ed.cursor.line);
            if self.app.focus == Focus::Editor {
                if let Some(row) = cursor_row {
                let cy = rect.min.y + row as f32 * lh;
                let cx = rect.min.x + gw + ed.cursor.col as f32 * cw;
                let blink = (ui.input(|i| i.time) * 2.0) as u32 % 2 == 0;
                if blink && cy < rect.max.y {
                    painter.rect_filled(
                        Rect::from_min_size(Pos2::new(cx, cy), Vec2::new(2.0, lh)),
                        Rounding::ZERO, self.tc.cursor_color,
                    );
                }
            }}  // close if let Some(row) and if focus

            // Minimap (code overview on right side)
            if self.app.show_minimap && lc > 1 {
                let minimap_w = 80.0;
                let minimap_x = rect.max.x - minimap_w;
                let minimap_rect = Rect::from_min_size(Pos2::new(minimap_x, rect.min.y), Vec2::new(minimap_w, rect.height()));
                let minimap_bg = if dark {
                    Color32::from_rgb(30, 30, 30)
                } else {
                    Color32::from_rgb(240, 240, 240)
                };
                painter.rect_filled(minimap_rect, Rounding::ZERO, minimap_bg);
                // Left border
                painter.line_segment(
                    [Pos2::new(minimap_x, rect.min.y), Pos2::new(minimap_x, rect.max.y)],
                    Stroke::new(1.0, self.tc.border),
                );

                // Calculate minimap scaling
                let mini_line_h: f32 = 2.0;
                let total_mini_h = mini_line_h * lc as f32;
                // If all lines fit in the rect, no scrolling needed for minimap
                // If they don't fit, we scroll the minimap proportionally
                let minimap_scroll_offset: f32 = if total_mini_h > rect.height() {
                    let scroll_ratio = so as f32 / (lc as f32 - vis as f32).max(1.0);
                    scroll_ratio * (total_mini_h - rect.height())
                } else {
                    0.0
                };

                // Draw minimap lines
                let step = if lc > 3000 { (lc / 1500).max(1) } else { 1 };
                for li in (0..lc).step_by(step) {
                    let my = rect.min.y + li as f32 * mini_line_h - minimap_scroll_offset;
                    if my < rect.min.y - 2.0 { continue; }
                    if my > rect.max.y { break; }

                    let line = ed.buffer.get_line(li);
                    let indent = line.chars().take_while(|c| c.is_whitespace()).count();
                    let content_len = line.trim().len().min(60);
                    if content_len == 0 { continue; }

                    let mx = minimap_x + 4.0 + (indent as f32 * 0.6).min(20.0);
                    let mw = (content_len as f32 * 0.8).min(minimap_w - 8.0);

                    // Color based on syntax — keywords brighter
                    let line_color = if let Some(diff_st) = ed.line_diff.get(&li) {
                        match diff_st {
                            crate::editor::LineDiffStatus::Added => if dark {
                                Color32::from_rgba_premultiplied(106, 171, 115, 100)
                            } else {
                                Color32::from_rgba_premultiplied(10, 132, 57, 60)
                            },
                            crate::editor::LineDiffStatus::Modified => if dark {
                                Color32::from_rgba_premultiplied(70, 140, 220, 100)
                            } else {
                                Color32::from_rgba_premultiplied(55, 125, 207, 60)
                            },
                        }
                    } else if dark {
                        Color32::from_rgba_premultiplied(187, 187, 187, 50)
                    } else {
                        Color32::from_rgba_premultiplied(0, 0, 0, 35)
                    };

                    painter.rect_filled(
                        Rect::from_min_size(Pos2::new(mx, my), Vec2::new(mw, mini_line_h.max(1.0))),
                        Rounding::ZERO, line_color,
                    );
                }

                // Viewport indicator (highlighted area showing what's visible)
                let vp_y = rect.min.y + so as f32 * mini_line_h - minimap_scroll_offset;
                let vp_h = (vis as f32 * mini_line_h).max(10.0);
                let vp_color = if dark {
                    Color32::from_rgba_premultiplied(122, 162, 247, 30)
                } else {
                    Color32::from_rgba_premultiplied(55, 125, 207, 25)
                };
                painter.rect_filled(
                    Rect::from_min_size(Pos2::new(minimap_x, vp_y), Vec2::new(minimap_w, vp_h)),
                    Rounding::ZERO, vp_color,
                );
                // Border on viewport indicator
                let vp_border = if dark {
                    Color32::from_rgba_premultiplied(122, 162, 247, 70)
                } else {
                    Color32::from_rgba_premultiplied(55, 125, 207, 50)
                };
                painter.line_segment(
                    [Pos2::new(minimap_x, vp_y), Pos2::new(minimap_x + minimap_w, vp_y)],
                    Stroke::new(1.0, vp_border),
                );
                painter.line_segment(
                    [Pos2::new(minimap_x, vp_y + vp_h), Pos2::new(minimap_x + minimap_w, vp_y + vp_h)],
                    Stroke::new(1.0, vp_border),
                );

                // Handle minimap click & drag to scroll
                if let Some(pointer_pos) = ctx.input(|i| i.pointer.hover_pos()) {
                    if minimap_rect.contains(pointer_pos) {
                        let clicking = ctx.input(|i| i.pointer.primary_down());
                        if clicking {
                            let click_y = pointer_pos.y - rect.min.y + minimap_scroll_offset;
                            let target_line = (click_y / mini_line_h) as usize;
                            let target_scroll = target_line.saturating_sub(vis / 2).min(lc.saturating_sub(vis));
                            minimap_new_scroll = Some(target_scroll);
                        }

                        // Hover highlight
                        if !clicking {
                            let hover_y = pointer_pos.y - rect.min.y + minimap_scroll_offset;
                            let hover_line = (hover_y / mini_line_h) as usize;
                            let hover_vp_y = rect.min.y + hover_line.saturating_sub(vis / 2) as f32 * mini_line_h - minimap_scroll_offset;
                            let hover_vp_h = vis as f32 * mini_line_h;
                            painter.rect_filled(
                                Rect::from_min_size(Pos2::new(minimap_x, hover_vp_y), Vec2::new(minimap_w, hover_vp_h)),
                                Rounding::ZERO,
                                if dark {
                                    Color32::from_rgba_premultiplied(255, 255, 255, 10)
                                } else {
                                    Color32::from_rgba_premultiplied(0, 0, 0, 8)
                                },
                            );
                        }
                    }
                }
            } else if lc > vis {
                // Simple scrollbar when minimap is off
                let sb_h = (vis as f32 / lc as f32 * rect.height()).max(20.0);
                let sb_y = rect.min.y + (so as f32 / lc as f32 * rect.height());
                painter.rect_filled(
                    Rect::from_min_size(Pos2::new(rect.max.x - 6.0, sb_y), Vec2::new(4.0, sb_h)),
                    Rounding::same(2), if dark { Color32::from_rgba_premultiplied(122, 162, 247, 60) } else { Color32::from_rgba_premultiplied(0, 0, 0, 40) },
                );
            }

            // Autocomplete popup
            if self.app.show_autocomplete && self.app.focus == Focus::Editor {
                if let Some(cursor_row) = cursor_row {
                    let ac_x = rect.min.x + gw + ed.cursor.col as f32 * cw;
                    let ac_y = rect.min.y + (cursor_row + 1) as f32 * lh;
                    let ac_w = 220.0;
                    let ac_item_h = 24.0;
                    let ac_count = self.app.autocomplete_suggestions.len().min(8);
                    let ac_h = ac_count as f32 * ac_item_h + 4.0;

                    let popup_bg = if dark { Color32::from_rgb(43, 43, 43) } else { Color32::from_rgb(255, 255, 255) };
                    let ac_rect = Rect::from_min_size(Pos2::new(ac_x, ac_y), Vec2::new(ac_w, ac_h));

                    // Shadow
                    painter.rect_filled(
                        ac_rect.translate(Vec2::new(2.0, 2.0)),
                        Rounding::same(4), Color32::from_black_alpha(if dark { 80 } else { 30 }),
                    );
                    painter.rect_filled(ac_rect, Rounding::same(4), popup_bg);
                    painter.rect_stroke(ac_rect, Rounding::same(4), Stroke::new(1.0, self.tc.border), egui::StrokeKind::Outside);

                    for (i, suggestion) in self.app.autocomplete_suggestions.iter().enumerate().take(ac_count) {
                        let item_y = ac_y + 2.0 + i as f32 * ac_item_h;
                        let selected = i == self.app.autocomplete_selected;
                        if selected {
                            painter.rect_filled(
                                Rect::from_min_size(Pos2::new(ac_x + 2.0, item_y), Vec2::new(ac_w - 4.0, ac_item_h)),
                                Rounding::same(3), self.tc.selection_bg,
                            );
                        }
                        painter.text(
                            Pos2::new(ac_x + 8.0, item_y + 4.0), egui::Align2::LEFT_TOP,
                            suggestion, FontId::monospace(12.0),
                            if selected { self.tc.fg } else { self.tc.fg_dim },
                        );
                    }
                }
            }

            // Extra cursors (multi-cursor)
            if self.app.focus == Focus::Editor {
                let blink = (ui.input(|i| i.time) * 2.0) as u32 % 2 == 0;
                if blink {
                    for ec in &ed.extra_cursors {
                        if let Some(row) = vis_lines.iter().position(|&l| l == ec.line) {
                            let ecy = rect.min.y + row as f32 * lh;
                            let ecx = rect.min.x + gw + ec.col as f32 * cw;
                            if ecy < rect.max.y {
                                painter.rect_filled(
                                    Rect::from_min_size(Pos2::new(ecx, ecy), Vec2::new(2.0, lh)),
                                    Rounding::ZERO, self.tc.cursor_color,
                                );
                            }
                        }
                    }
                }
            }

            // Apply deferred minimap scroll (after `ed` borrow ends)
            if let Some(new_scroll) = minimap_new_scroll {
                self.app.editors[self.app.active_editor].scroll_offset = new_scroll;
            }
        });
    }

    fn render_drag_overlay(&self, ctx: &egui::Context) {
        if let Some((_, ref src_path)) = self.drag_source {
            let file_name = std::path::Path::new(src_path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| src_path.clone());

            let is_dir = std::path::Path::new(src_path).is_dir();
            let icon = if is_dir { "📁 " } else { "📄 " };

            // Build label text
            let label = if let Some((_, ref dst_path)) = self.drop_target {
                let dst_name = std::path::Path::new(dst_path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| dst_path.clone());
                format!("{}{} → 📂 {}", icon, file_name, dst_name)
            } else {
                format!("{}{}", icon, file_name)
            };

            if let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) {
                let painter = ctx.layer_painter(egui::LayerId::new(
                    egui::Order::Tooltip,
                    egui::Id::new("drag_overlay"),
                ));

                let font = FontId::monospace(12.0);
                let galley = painter.layout_no_wrap(label, font, self.tc.fg);
                let text_size = galley.size();
                let padding = Vec2::new(8.0, 4.0);
                let offset = Vec2::new(12.0, -8.0);

                let bg_rect = Rect::from_min_size(
                    pos + offset - Vec2::new(0.0, 0.0),
                    text_size + padding * 2.0,
                );

                // Shadow
                painter.rect_filled(
                    bg_rect.translate(Vec2::new(2.0, 2.0)),
                    Rounding::same(4),
                    Color32::from_black_alpha(80),
                );
                // Background
                painter.rect_filled(bg_rect, Rounding::same(4), self.tc.sidebar_bg);
                painter.rect_stroke(bg_rect, Rounding::same(4), Stroke::new(1.0, self.tc.accent), egui::StrokeKind::Outside);
                // Text
                painter.galley(bg_rect.min + padding, galley, Color32::PLACEHOLDER);
            }

            ctx.request_repaint();
        }
    }

    fn render_overlays(&mut self, ctx: &egui::Context) {
        let dark = self.app.settings.theme != Theme::Light;
        let popup_frame = |accent: Color32| -> egui::Frame {
            let popup_bg = if dark { Color32::from_rgb(25, 26, 36) } else { Color32::from_rgb(255, 255, 255) };
            let shadow_alpha = if dark { 80 } else { 30 };
            egui::Frame::NONE
                .fill(popup_bg)
                .stroke(Stroke::new(1.0, accent))
                .rounding(Rounding::same(8))
                .inner_margin(12.0)
                .shadow(egui::epaint::Shadow {
                    offset: [0, 4],
                    blur: 16,
                    spread: 0,
                    color: Color32::from_black_alpha(shadow_alpha),
                })
        };

        match self.app.focus {
            Focus::CommandPalette => {
                egui::Area::new(egui::Id::new("pal")).fixed_pos(egui::pos2(ctx.screen_rect().width() * 0.25, 50.0)).show(ctx, |ui| {
                    popup_frame(self.tc.accent).show(ui, |ui| {
                        ui.set_width(ctx.screen_rect().width() * 0.5);
                        let r = ui.add(egui::TextEdit::singleline(&mut self.app.palette_input)
                            .font(mono()).hint_text("> Command...").desired_width(ui.available_width()).text_color(self.tc.fg));
                        r.request_focus();
                        let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                        let up = ui.input(|i| i.key_pressed(egui::Key::ArrowUp));
                        let down = ui.input(|i| i.key_pressed(egui::Key::ArrowDown));
                        if up && self.app.palette_selected > 0 { self.app.palette_selected -= 1; }
                        let items = self.app.filtered_palette_items();
                        if down && self.app.palette_selected + 1 < items.len() { self.app.palette_selected += 1; }
                        ui.add_space(4.0);
                        ui.painter().line_segment(
                            [Pos2::new(ui.max_rect().min.x, ui.cursor().min.y), Pos2::new(ui.max_rect().max.x, ui.cursor().min.y)],
                            Stroke::new(1.0, self.tc.border),
                        );
                        ui.add_space(4.0);
                        for (i, item) in items.iter().enumerate().take(12) {
                            let s = i == self.app.palette_selected;
                            let bg = if s { self.tc.selection_bg } else { Color32::TRANSPARENT };
                            let rr = ui.add(egui::Button::new(
                                RichText::new(format!("  {}", item.name)).font(small()).color(if s { self.tc.fg } else { self.tc.fg_dim })
                            ).fill(bg).min_size(Vec2::new(ui.available_width(), 26.0)).rounding(Rounding::same(4)).stroke(Stroke::NONE));
                            if rr.clicked() || (enter && s) {
                                let a = item.action.clone();
                                self.app.focus = Focus::Editor;
                                self.app.execute_palette_action(a);
                                return;
                            }
                        }
                        if enter && !items.is_empty() {
                            let a = items[self.app.palette_selected].action.clone();
                            self.app.focus = Focus::Editor;
                            self.app.execute_palette_action(a);
                        }
                    });
                });
            }
            Focus::QuickOpen => {
                egui::Area::new(egui::Id::new("qo")).fixed_pos(egui::pos2(ctx.screen_rect().width() * 0.25, 50.0)).show(ctx, |ui| {
                    popup_frame(self.tc.accent).show(ui, |ui| {
                        ui.set_width(ctx.screen_rect().width() * 0.5);
                        let old = self.app.quick_open_input.clone();
                        let r = ui.add(egui::TextEdit::singleline(&mut self.app.quick_open_input)
                            .font(mono()).hint_text("File name...").desired_width(ui.available_width()).text_color(self.tc.fg));
                        r.request_focus();
                        if self.app.quick_open_input != old {
                            self.app.quick_open_results = self.app.file_tree.fuzzy_search(&self.app.quick_open_input);
                            self.app.quick_open_selected = 0;
                        }
                        let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                        let up = ui.input(|i| i.key_pressed(egui::Key::ArrowUp));
                        let down = ui.input(|i| i.key_pressed(egui::Key::ArrowDown));
                        if up && self.app.quick_open_selected > 0 { self.app.quick_open_selected -= 1; }
                        if down && self.app.quick_open_selected + 1 < self.app.quick_open_results.len() { self.app.quick_open_selected += 1; }
                        ui.add_space(4.0);
                        let res = self.app.quick_open_results.clone();
                        for (i, e) in res.iter().enumerate().take(12) {
                            let s = i == self.app.quick_open_selected;
                            let rr = ui.add(egui::Button::new(
                                RichText::new(format!("  {}", e.name)).font(small()).color(if s { self.tc.fg } else { self.tc.fg_dim })
                            ).fill(if s { self.tc.selection_bg } else { Color32::TRANSPARENT }).min_size(Vec2::new(ui.available_width(), 26.0)).rounding(Rounding::same(4)).stroke(Stroke::NONE));
                            if rr.clicked() || (enter && s) { let p = e.path.clone(); self.app.open_file(&p); self.app.focus = Focus::Editor; return; }
                        }
                        if enter && !res.is_empty() { let p = res[self.app.quick_open_selected].path.clone(); self.app.open_file(&p); self.app.focus = Focus::Editor; }
                    });
                });
            }
            Focus::FindReplace => {
                let enter_find = ctx.input(|i| i.key_pressed(egui::Key::Enter));
                egui::Area::new(egui::Id::new("fr")).anchor(egui::Align2::RIGHT_TOP, [-20.0, 50.0]).show(ctx, |ui| {
                    popup_frame(self.tc.accent).show(ui, |ui| {
                        ui.set_width(350.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Find").font(small()).color(self.tc.fg_dim));
                            let r = ui.add(egui::TextEdit::singleline(&mut self.app.find_input)
                                .font(mono()).desired_width(280.0).text_color(self.tc.fg));
                            if r.changed() {
                                self.app.update_find_matches();
                            }
                            if enter_find && r.has_focus() {
                                self.app.goto_next_match();
                            }
                            r.request_focus();
                        });
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Replace").font(small()).color(self.tc.fg_dim));
                            ui.add(egui::TextEdit::singleline(&mut self.app.replace_input)
                                .font(mono()).desired_width(260.0).text_color(self.tc.fg));
                        });
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            if ui.add(egui::Button::new(RichText::new("◀").font(small())).min_size(Vec2::new(30.0, 22.0))).clicked() {
                                self.app.goto_prev_match();
                            }
                            if ui.add(egui::Button::new(RichText::new("▶").font(small())).min_size(Vec2::new(30.0, 22.0))).clicked() {
                                self.app.goto_next_match();
                            }
                            ui.add_space(4.0);
                            if ui.add(egui::Button::new(RichText::new("Replace").font(small())).min_size(Vec2::new(60.0, 22.0))).clicked() {
                                self.app.replace_current();
                            }
                            if ui.add(egui::Button::new(RichText::new("All").font(small())).min_size(Vec2::new(40.0, 22.0))).clicked() {
                                self.app.replace_all();
                            }
                            let total = self.app.find_matches.len();
                            let label = if total > 0 {
                                format!("{}/{}", self.app.find_current + 1, total)
                            } else {
                                "No matches".to_string()
                            };
                            ui.label(RichText::new(label).font(small()).color(if total > 0 { self.tc.fg_dim } else { self.tc.red }));
                        });
                    });
                });
            }
            Focus::GoToLine => {
                let enter_pressed = ctx.input(|i| i.key_pressed(egui::Key::Enter));
                egui::Area::new(egui::Id::new("gl")).fixed_pos(egui::pos2(ctx.screen_rect().width() * 0.35, 50.0)).show(ctx, |ui| {
                    popup_frame(self.tc.accent).show(ui, |ui| {
                        ui.set_width(220.0);
                        ui.label(RichText::new("Go to Line").font(small()).color(self.tc.fg_dim));
                        ui.add_space(4.0);
                        let r = ui.add(egui::TextEdit::singleline(&mut self.app.goto_input)
                            .font(mono()).desired_width(200.0).text_color(self.tc.fg));
                        r.request_focus();
                        if enter_pressed {
                            if let Ok(l) = self.app.goto_input.parse::<usize>() {
                                self.app.active_editor_mut().go_to_line(l);
                                self.app.focus = Focus::Editor;
                            }
                        }
                    });
                });
            }
            Focus::NewFileDialog | Focus::NewFolderDialog => {
                let is_file = self.app.focus == Focus::NewFileDialog;
                let title = if is_file { "New File" } else { "New Folder" };
                let hint = if is_file { "filename.ext" } else { "folder-name" };
                let enter_pressed = ctx.input(|i| i.key_pressed(egui::Key::Enter));
                egui::Area::new(egui::Id::new("dialog")).fixed_pos(egui::pos2(ctx.screen_rect().width() * 0.3, 80.0)).show(ctx, |ui| {
                    popup_frame(self.tc.accent).show(ui, |ui| {
                        ui.set_width(320.0);
                        ui.label(RichText::new(title).font(mono()).color(self.tc.accent));
                        ui.add_space(6.0);
                        let r = ui.add(egui::TextEdit::singleline(&mut self.app.dialog_input)
                            .font(mono()).hint_text(hint).desired_width(300.0).text_color(self.tc.fg));
                        r.request_focus();
                        if enter_pressed && !self.app.dialog_input.is_empty() {
                            let path = format!("{}/{}", self.app.dialog_context_path, self.app.dialog_input);
                            if is_file {
                                match self.app.file_tree.create_file(&path) {
                                    Ok(()) => { self.app.open_file(&path); self.app.status_message = format!("Created: {}", self.app.dialog_input); }
                                    Err(e) => { self.app.status_message = format!("Error: {}", e); }
                                }
                            } else {
                                match self.app.file_tree.create_directory(&path) {
                                    Ok(()) => { self.app.status_message = format!("Created: {}", self.app.dialog_input); }
                                    Err(e) => { self.app.status_message = format!("Error: {}", e); }
                                }
                            }
                            self.app.focus = Focus::Editor;
                        }
                    });
                });
            }
            Focus::RenameDialog => {
                let enter_pressed = ctx.input(|i| i.key_pressed(egui::Key::Enter));
                egui::Area::new(egui::Id::new("rn")).fixed_pos(egui::pos2(ctx.screen_rect().width() * 0.3, 80.0)).show(ctx, |ui| {
                    popup_frame(self.tc.accent).show(ui, |ui| {
                        ui.set_width(320.0);
                        ui.label(RichText::new("Rename").font(mono()).color(self.tc.accent));
                        ui.add_space(6.0);
                        let r = ui.add(egui::TextEdit::singleline(&mut self.app.dialog_input)
                            .font(mono()).desired_width(300.0).text_color(self.tc.fg));
                        r.request_focus();
                        if enter_pressed && !self.app.dialog_input.is_empty() {
                            let old = self.app.dialog_context_path.clone();
                            let parent = std::path::Path::new(&old).parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
                            let new_path = format!("{}/{}", parent, self.app.dialog_input);
                            match self.app.file_tree.rename_entry(&old, &new_path) {
                                Ok(()) => { self.app.status_message = format!("Renamed to: {}", self.app.dialog_input); }
                                Err(e) => { self.app.status_message = format!("Error: {}", e); }
                            }
                            self.app.focus = Focus::Editor;
                        }
                    });
                });
            }
            Focus::DeleteConfirm => {
                if let Some(entry) = self.app.file_tree.selected_entry().cloned() {
                    egui::Area::new(egui::Id::new("del")).fixed_pos(egui::pos2(ctx.screen_rect().width() * 0.3, 80.0)).show(ctx, |ui| {
                        popup_frame(self.tc.red).show(ui, |ui| {
                            ui.set_width(320.0);
                            ui.label(RichText::new(format!("Delete \"{}\"?", entry.name)).font(mono()).color(self.tc.red));
                            ui.add_space(4.0);
                            ui.label(RichText::new("This cannot be undone.").font(small()).color(self.tc.fg_dim));
                            ui.add_space(10.0);
                            ui.horizontal(|ui| {
                                if ui.add(egui::Button::new(RichText::new(" Delete ").color(Color32::WHITE))
                                    .fill(Color32::from_rgb(200, 60, 60)).rounding(Rounding::same(4))).clicked()
                                {
                                    let _ = self.app.file_tree.delete_entry(&entry.path);
                                    self.app.status_message = format!("Deleted: {}", entry.name);
                                    self.app.focus = Focus::Editor;
                                }
                                ui.add_space(8.0);
                                if ui.add(egui::Button::new(RichText::new(" Cancel ").color(self.tc.fg))
                                    .fill(self.tc.sidebar_bg).rounding(Rounding::same(4)).stroke(Stroke::new(1.0, self.tc.border))).clicked()
                                {
                                    self.app.focus = Focus::Editor;
                                }
                            });
                        });
                    });
                } else { self.app.focus = Focus::Editor; }
            }
            _ => {}
        }
    }
}
