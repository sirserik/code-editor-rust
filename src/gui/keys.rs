use super::*;

impl CodeEditorApp {
    pub(super) fn handle_keys(&mut self, ctx: &egui::Context) {
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
}
