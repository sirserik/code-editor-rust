use super::*;

impl CodeEditorApp {
    pub(super) fn handle_keys(&mut self, ctx: &egui::Context) {
        let find = ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::F));
        let save = ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::S));
        let save_as = ctx.input_mut(|i| i.consume_key(egui::Modifiers { command: true, shift: true, ..Default::default() }, egui::Key::S));
        let quit = ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Q));
        let new_file = ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::N));
        let close_tab = ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::W));
        let toggle_sb = ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::B));
        let palette = ctx.input_mut(|i| i.consume_key(egui::Modifiers { command: true, shift: true, ..Default::default() }, egui::Key::P));
        let quick_open = ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::P));
        let goto = ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::G));
        let undo = ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Z));
        let redo = ctx.input_mut(|i| i.consume_key(egui::Modifiers { command: true, shift: true, ..Default::default() }, egui::Key::Z));
        let open_folder = ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::O));
        let copy = ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::C));
        let paste = ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::V));
        let cut = ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::X));
        let select_line = ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::L));
        let zoom_in = ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Equals))
            || ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Plus));
        let zoom_out = ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Minus));
        let zoom_reset = ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Num0));
        let find_in_project = ctx.input_mut(|i| i.consume_key(egui::Modifiers { command: true, shift: true, ..Default::default() }, egui::Key::F));
        let esc = ctx.input(|i| i.key_pressed(egui::Key::Escape));

        if quit { ctx.send_viewport_cmd(egui::ViewportCommand::Close); }
        if esc && self.app.focus != Focus::Editor { self.app.focus = Focus::Editor; return; }
        if save_as {
            self.app.save_as_input = self.app.file_tree.root_path.as_ref()
                .map(|p| format!("{}/", p)).unwrap_or_default();
            self.app.focus = Focus::SaveAsDialog;
        } else if save {
            self.app.save_current();
        }
        if open_folder {
            self.app.pending_action = Some(PaletteAction::OpenFolder);
        }
        if find_in_project {
            self.app.focus = Focus::GlobalSearch;
            self.app.sidebar_tab = SidebarTab::Search;
            self.app.show_sidebar = true;
        } else if palette {
            self.app.focus = Focus::CommandPalette;
            self.app.palette_input.clear();
            self.app.palette_selected = 0;
        } else if quick_open {
            self.app.focus = Focus::QuickOpen;
            self.app.quick_open_input.clear();
            self.app.quick_open_results.clear();
        }
        if new_file { self.app.editors.push(crate::editor::Editor::new()); self.app.active_editor = self.app.editors.len() - 1; self.app.focus = Focus::Editor; }
        if close_tab { let i = self.app.active_editor; self.app.close_tab(i); }
        if toggle_sb { self.app.show_sidebar = !self.app.show_sidebar; }
        if find {
            self.app.focus = Focus::FindReplace;
            self.app.find_input.clear();
            self.app.find_matches.clear();
        }
        if goto { self.app.focus = Focus::GoToLine; self.app.goto_input.clear(); }
        if redo && self.app.focus == Focus::Editor { self.app.active_editor_mut().redo(); }
        else if undo && self.app.focus == Focus::Editor { self.app.active_editor_mut().undo(); }

        // Clipboard operations
        if self.app.focus == Focus::Editor {
            if copy {
                let ed = self.app.active_editor();
                if let Some(text) = ed.get_selected_text() {
                    if let Some(ref mut cb) = self.clipboard {
                        let _ = cb.set_text(&text);
                    }
                } else {
                    // No selection: copy current line (Sublime behavior)
                    let line = ed.buffer.get_line(ed.cursor.line);
                    if let Some(ref mut cb) = self.clipboard {
                        let _ = cb.set_text(format!("{}\n", line));
                    }
                }
            }
            if cut {
                let ed = self.app.active_editor();
                if ed.selection.is_some() {
                    let sel = ed.normalized_selection();
                    if let Some(sel) = sel {
                        let text = ed.buffer.get_range(sel.start_line, sel.start_col, sel.end_line, sel.end_col);
                        if let Some(ref mut cb) = self.clipboard {
                            let _ = cb.set_text(&text);
                        }
                    }
                    self.app.active_editor_mut().save_undo_snapshot();
                    self.app.active_editor_mut().delete_selection_text();
                } else {
                    // No selection: cut current line (Sublime behavior)
                    let ed = self.app.active_editor();
                    let line = ed.buffer.get_line(ed.cursor.line);
                    if let Some(ref mut cb) = self.clipboard {
                        let _ = cb.set_text(format!("{}\n", line));
                    }
                    self.app.active_editor_mut().save_undo_snapshot();
                    self.app.active_editor_mut().delete_line();
                }
            }
            if paste {
                if let Some(ref mut cb) = self.clipboard {
                    if let Ok(text) = cb.get_text() {
                        let ed = self.app.active_editor_mut();
                        ed.save_undo_snapshot();
                        // Delete selection first if any
                        ed.delete_selection_text();
                        // Insert pasted text
                        for c in text.chars() {
                            if c == '\n' {
                                ed.insert_newline();
                            } else if c != '\r' {
                                ed.insert_char(c);
                            }
                        }
                        ed.scroll_into_view();
                    }
                }
            }
            if select_line {
                self.app.active_editor_mut().select_line();
            }
        }

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
