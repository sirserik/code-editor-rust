mod app;
mod editor;
mod file_tree;
mod git;
mod gui;
mod search;
mod settings;
mod syntax;
mod terminal;

use app::App;

fn main() -> eframe::Result<()> {
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

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([600.0, 400.0])
            .with_title("Code Editor"),
        renderer: eframe::Renderer::Glow,
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

        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = egui::Color32::from_rgb(26, 27, 38);
        visuals.window_fill = egui::Color32::from_rgb(26, 27, 38);
        visuals.faint_bg_color = egui::Color32::from_rgb(22, 22, 30);
        cc.egui_ctx.set_visuals(visuals);
        Ok(Box::new(gui::CodeEditorApp::new(app)))
    }))
}
