mod app;
mod editor;
mod file_tree;
mod git;
mod gui;
mod output;
mod search;
mod settings;
mod snippets;
mod syntax;
mod syntect_engine;
mod templates;
mod terminal;
mod wrap;

use app::App;

fn main() -> eframe::Result<()> {
    // Warm syntect's grammar + theme sets on a background thread so the first
    // file open doesn't pay the ~100-200ms one-time load on the UI thread.
    std::thread::spawn(syntect_engine::prewarm);

    let mut app = App::new();

    // Parse CLI args
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        let path = std::path::Path::new(&args[1])
            .canonicalize()
            .unwrap_or_else(|_| std::path::PathBuf::from(&args[1]));
        if path.is_dir() {
            app.open_folder(path.to_string_lossy().to_string());
        } else if path.is_file() {
            if let Some(parent) = path.parent() {
                app.open_folder(parent.to_string_lossy().to_string());
            }
            app.open_file(&path.to_string_lossy());
        }
    }
    // No else — if no args, start with empty editor (user opens folder via menu)

    // Bake the Dock icon into the binary so it survives terminal launches (where there
    // is no .app bundle context for macOS to discover Info.plist + AppIcon.icns).
    let icon = egui::IconData {
        rgba: include_bytes!("../assets/icon-256.rgba").to_vec(),
        width: 256,
        height: 256,
    };

    // Frameless-style macOS window: content extends under the title bar so our
    // dark theme fills the whole window. The traffic-light buttons stay visible
    // (we just hide the system title text — we render our own). On macOS this
    // requires `with_fullsize_content_view(true)` + `with_title_shown(false)`.
    let viewport = egui::ViewportBuilder::default()
        .with_inner_size([1200.0, 800.0])
        .with_min_inner_size([600.0, 400.0])
        .with_title("Ferrite")
        .with_icon(icon)
        .with_fullsize_content_view(true)
        .with_title_shown(false)
        .with_titlebar_buttons_shown(true);
    let options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Wgpu,
        vsync: true,
        ..Default::default()
    };

    eframe::run_native("Code Editor", options, Box::new(|cc| {
        // Load JetBrains Mono font
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "JetBrainsMono".to_owned(),
            std::sync::Arc::new(egui::FontData::from_static(
                include_bytes!("../assets/JetBrainsMono-Regular.ttf"),
            )),
        );
        fonts.families.entry(egui::FontFamily::Monospace).or_default()
            .insert(0, "JetBrainsMono".to_owned());
        fonts.families.entry(egui::FontFamily::Proportional).or_default()
            .insert(0, "JetBrainsMono".to_owned());
        cc.egui_ctx.set_fonts(fonts);

        // Set initial visuals from loaded theme (resolve SystemDefault)
        let resolved = app.settings.theme.resolved();
        let tc = resolved.colors();
        let mut visuals = if resolved == crate::settings::Theme::Light {
            egui::Visuals::light()
        } else {
            egui::Visuals::dark()
        };
        visuals.panel_fill = tc.bg;
        visuals.window_fill = tc.bg;
        visuals.faint_bg_color = tc.sidebar_bg;
        cc.egui_ctx.set_visuals(visuals);
        Ok(Box::new(gui::CodeEditorApp::new(app)))
    }))
}
