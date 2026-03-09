use crate::app::{App, Focus, SidebarTab};
use crate::settings::Theme;
use crate::syntax;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};
use ratatui::Frame;

fn is_dark(app: &App) -> bool {
    app.settings.theme == Theme::Dark
}

fn bg(app: &App) -> Color {
    if is_dark(app) {
        Color::Rgb(26, 27, 38)
    } else {
        Color::Rgb(250, 250, 252)
    }
}

fn fg(app: &App) -> Color {
    if is_dark(app) {
        Color::Rgb(192, 202, 245)
    } else {
        Color::Rgb(40, 42, 54)
    }
}

fn sidebar_bg(app: &App) -> Color {
    if is_dark(app) {
        Color::Rgb(22, 22, 30)
    } else {
        Color::Rgb(240, 240, 245)
    }
}

fn gutter_fg(app: &App) -> Color {
    if is_dark(app) {
        Color::Rgb(60, 65, 90)
    } else {
        Color::Rgb(170, 175, 190)
    }
}

fn selection_bg(app: &App) -> Color {
    if is_dark(app) {
        Color::Rgb(40, 44, 65)
    } else {
        Color::Rgb(200, 210, 235)
    }
}

fn accent(app: &App) -> Color {
    if is_dark(app) {
        Color::Rgb(122, 162, 247)
    } else {
        Color::Rgb(40, 80, 180)
    }
}

fn status_bg(app: &App) -> Color {
    if is_dark(app) {
        Color::Rgb(30, 32, 48)
    } else {
        Color::Rgb(230, 232, 240)
    }
}

pub fn draw(f: &mut Frame, app: &mut App) {
    let size = f.area();
    app.width = size.width;
    app.height = size.height;

    // Main layout: [tab_bar] [main_content] [status_bar]
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Tab bar
            Constraint::Min(1),   // Content
            Constraint::Length(1), // Status bar
        ])
        .split(size);

    draw_tab_bar(f, app, main_chunks[0]);

    // Content: split into [sidebar | editor+terminal]
    let content_area = main_chunks[1];

    if app.show_sidebar {
        let h_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(app.sidebar_width),
                Constraint::Min(1),
            ])
            .split(content_area);

        draw_sidebar(f, app, h_chunks[0]);
        draw_editor_area(f, app, h_chunks[1]);
    } else {
        draw_editor_area(f, app, content_area);
    }

    draw_status_bar(f, app, main_chunks[2]);

    // Overlays
    match app.focus {
        Focus::CommandPalette => draw_command_palette(f, app, size),
        Focus::QuickOpen => draw_quick_open(f, app, size),
        Focus::FindReplace => draw_find_replace(f, app, size),
        Focus::GoToLine => draw_goto_line(f, app, size),
        Focus::NewFileDialog => draw_input_dialog(f, app, size, "New File", "File name:"),
        Focus::NewFolderDialog => draw_input_dialog(f, app, size, "New Folder", "Folder name:"),
        Focus::RenameDialog => draw_input_dialog(f, app, size, "Rename", "New name:"),
        Focus::DeleteConfirm => draw_confirm_dialog(f, app, size),
        Focus::CommitInput => draw_commit_dialog(f, app, size),
        Focus::SaveAsDialog => draw_save_as_dialog(f, app, size),
        _ => {}
    }
}

fn draw_tab_bar(f: &mut Frame, app: &App, area: Rect) {
    let mut spans = Vec::new();

    for (i, editor) in app.editors.iter().enumerate() {
        let name = editor.file_name();
        let dirty = if editor.is_dirty { " *" } else { "" };
        let is_active = i == app.active_editor;

        let style = if is_active {
            Style::default().fg(fg(app)).bg(bg(app)).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(gutter_fg(app))
                .bg(sidebar_bg(app))
        };

        spans.push(Span::styled(format!(" {}{} ", name, dirty), style));
        spans.push(Span::styled("│", Style::default().fg(gutter_fg(app)).bg(sidebar_bg(app))));
    }

    let line = Line::from(spans);
    let tabs = Paragraph::new(line).style(Style::default().bg(sidebar_bg(app)));
    f.render_widget(tabs, area);
}

fn draw_sidebar(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(area);

    // Sidebar tabs
    let tab_names = [("1:Files", SidebarTab::Files), ("2:Git", SidebarTab::Git), ("3:Search", SidebarTab::Search)];
    let tab_spans: Vec<Span> = tab_names
        .iter()
        .map(|(name, tab)| {
            let style = if app.sidebar_tab == *tab {
                Style::default().fg(accent(app)).bg(sidebar_bg(app)).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(gutter_fg(app)).bg(sidebar_bg(app))
            };
            Span::styled(format!(" {} ", name), style)
        })
        .collect();
    let tab_bar = Paragraph::new(Line::from(tab_spans)).style(Style::default().bg(sidebar_bg(app)));
    f.render_widget(tab_bar, chunks[0]);

    match app.sidebar_tab {
        SidebarTab::Files => draw_file_tree(f, app, chunks[1]),
        SidebarTab::Git => draw_git_panel(f, app, chunks[1]),
        SidebarTab::Search => draw_search_panel(f, app, chunks[1]),
    }
}

fn draw_file_tree(f: &mut Frame, app: &mut App, area: Rect) {
    // Split into tree area + hints bar
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(2)])
        .split(area);

    let tree_area = chunks[0];
    let hints_area = chunks[1];

    app.file_tree.viewport_height = tree_area.height as usize;

    let items: Vec<ListItem> = app
        .file_tree
        .flat_entries
        .iter()
        .enumerate()
        .skip(app.file_tree.scroll_offset)
        .take(tree_area.height as usize)
        .map(|(i, entry)| {
            let indent = "  ".repeat(entry.depth);
            let icon = if entry.is_directory {
                if entry.is_expanded { "▼ " } else { "▶ " }
            } else {
                file_icon(&entry.name)
            };

            let is_hidden = entry.name.starts_with('.');
            let style = if i == app.file_tree.selected_index {
                Style::default().fg(fg(app)).bg(selection_bg(app))
            } else if entry.is_directory {
                Style::default().fg(accent(app))
            } else if is_hidden {
                Style::default().fg(gutter_fg(app)) // Dim hidden files
            } else {
                Style::default().fg(fg(app))
            };

            ListItem::new(Line::from(vec![
                Span::raw(indent),
                Span::styled(format!("{}{}", icon, entry.name), style),
            ]))
        })
        .collect();

    let list = List::new(items).style(Style::default().bg(sidebar_bg(app)));
    f.render_widget(list, tree_area);

    // Hints
    let hidden_indicator = if app.file_tree.show_hidden { "●" } else { "○" };
    let hints = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("n", Style::default().fg(accent(app))),
            Span::styled("ew ", Style::default().fg(gutter_fg(app))),
            Span::styled("N", Style::default().fg(accent(app))),
            Span::styled("dir ", Style::default().fg(gutter_fg(app))),
            Span::styled("d", Style::default().fg(accent(app))),
            Span::styled("el ", Style::default().fg(gutter_fg(app))),
            Span::styled("R", Style::default().fg(accent(app))),
            Span::styled("en", Style::default().fg(gutter_fg(app))),
        ]),
        Line::from(vec![
            Span::styled(".", Style::default().fg(accent(app))),
            Span::styled(format!("hidden{} ", hidden_indicator), Style::default().fg(gutter_fg(app))),
            Span::styled("r", Style::default().fg(accent(app))),
            Span::styled("efresh", Style::default().fg(gutter_fg(app))),
        ]),
    ])
    .style(Style::default().bg(sidebar_bg(app)));
    f.render_widget(hints, hints_area);
}

fn draw_git_panel(f: &mut Frame, app: &App, area: Rect) {
    let mut lines = Vec::new();

    if let Some(ref status) = app.git_status {
        if !status.is_repo {
            lines.push(Line::from(Span::styled(
                " Not a git repository",
                Style::default().fg(gutter_fg(app)),
            )));
        } else {
            lines.push(Line::from(vec![
                Span::styled(" Branch: ", Style::default().fg(gutter_fg(app))),
                Span::styled(&status.branch, Style::default().fg(accent(app)).add_modifier(Modifier::BOLD)),
            ]));
            lines.push(Line::from(""));

            if status.files.is_empty() {
                lines.push(Line::from(Span::styled(
                    " No changes",
                    Style::default().fg(gutter_fg(app)),
                )));
            } else {
                // Staged files
                let staged: Vec<_> = status.files.iter().filter(|f| f.staged).collect();
                if !staged.is_empty() {
                    lines.push(Line::from(Span::styled(
                        " Staged Changes",
                        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                    )));
                    for file in &staged {
                        lines.push(Line::from(vec![
                            Span::styled(
                                format!("  {} ", file.status.symbol()),
                                Style::default().fg(Color::Green),
                            ),
                            Span::styled(&file.path, Style::default().fg(fg(app))),
                        ]));
                    }
                    lines.push(Line::from(""));
                }

                // Unstaged files
                let unstaged: Vec<_> = status.files.iter().filter(|f| !f.staged).collect();
                if !unstaged.is_empty() {
                    lines.push(Line::from(Span::styled(
                        " Changes",
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                    )));
                    for (i, file) in unstaged.iter().enumerate() {
                        let is_selected = app.focus == Focus::GitPanel && i + staged.len() == app.git_selected;
                        let style = if is_selected {
                            Style::default().fg(fg(app)).bg(selection_bg(app))
                        } else {
                            Style::default().fg(fg(app))
                        };
                        lines.push(Line::from(vec![
                            Span::styled(
                                format!("  {} ", file.status.symbol()),
                                Style::default().fg(Color::Yellow),
                            ),
                            Span::styled(&file.path, style),
                        ]));
                    }
                }
            }

            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                " [s]tage [u]nstage [a]ll [c]ommit",
                Style::default().fg(gutter_fg(app)),
            )));
        }
    } else {
        lines.push(Line::from(Span::styled(
            " No project open",
            Style::default().fg(gutter_fg(app)),
        )));
    }

    let paragraph = Paragraph::new(lines).style(Style::default().bg(sidebar_bg(app)));
    f.render_widget(paragraph, area);
}

fn draw_search_panel(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(area);

    // Search input
    let input_style = if app.focus == Focus::GlobalSearch {
        Style::default().fg(fg(app)).bg(selection_bg(app))
    } else {
        Style::default().fg(fg(app)).bg(sidebar_bg(app))
    };
    let input = Paragraph::new(format!(" > {}", app.global_search_input)).style(input_style);
    f.render_widget(input, chunks[0]);

    // Results
    let mut lines = Vec::new();
    for (i, result) in app.global_search_results.iter().enumerate().take(chunks[1].height as usize) {
        let is_selected = i == app.global_search_selected;
        let style = if is_selected {
            Style::default().fg(fg(app)).bg(selection_bg(app))
        } else {
            Style::default().fg(fg(app))
        };

        lines.push(Line::from(vec![
            Span::styled(
                format!(" {}:{} ", result.file_name, result.line_number),
                Style::default().fg(accent(app)),
            ),
            Span::styled(result.line_content.trim(), style),
        ]));
    }

    let results = Paragraph::new(lines).style(Style::default().bg(sidebar_bg(app)));
    f.render_widget(results, chunks[1]);
}

fn draw_editor_area(f: &mut Frame, app: &mut App, area: Rect) {
    if app.show_terminal {
        let v_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(app.terminal_height),
            ])
            .split(area);

        draw_editor(f, app, v_chunks[0]);
        draw_terminal(f, app, v_chunks[1]);
    } else {
        draw_editor(f, app, area);
    }
}

fn draw_editor(f: &mut Frame, app: &mut App, area: Rect) {
    // Update editor viewport and scroll
    {
        let editor = &mut app.editors[app.active_editor];
        editor.viewport_height = area.height as usize;
        editor.viewport_width = area.width as usize;
        editor.scroll_into_view();
    }

    // Extract all data we need from the editor (to avoid borrow conflicts)
    let dark = is_dark(app);
    let fg_color = fg(app);
    let gutter_color = gutter_fg(app);
    let bg_color = bg(app);
    let show_line_numbers = app.settings.show_line_numbers;

    let editor = &app.editors[app.active_editor];
    let scroll_offset = editor.scroll_offset;
    let cursor_line = editor.cursor.line;
    let cursor_col = editor.cursor.col;
    let line_count = editor.line_count();
    let language = editor
        .file_path
        .as_ref()
        .map(|p| syntax::detect_language(p).to_string())
        .unwrap_or_else(|| "text".to_string());

    let gutter_width = if show_line_numbers {
        let digits = format!("{}", line_count).len();
        digits.max(3) + 2
    } else {
        0
    };

    // Collect line contents
    let mut line_data: Vec<(usize, String)> = Vec::new();
    for row in 0..area.height as usize {
        let line_idx = scroll_offset + row;
        if line_idx >= line_count {
            break;
        }
        line_data.push((line_idx, editor.buffer.get_line(line_idx)));
    }

    let current_line_bg = if dark {
        Color::Rgb(30, 32, 48)
    } else {
        Color::Rgb(240, 242, 248)
    };

    // Build display lines
    let mut lines = Vec::new();

    for row in 0..area.height as usize {
        let line_idx = scroll_offset + row;
        if line_idx >= line_count {
            let mut spans = Vec::new();
            if show_line_numbers {
                spans.push(Span::styled(
                    format!("{:>width$} ", "~", width = gutter_width - 1),
                    Style::default().fg(gutter_color),
                ));
            }
            lines.push(Line::from(spans));
            continue;
        }

        let line_content = &line_data[row].1;
        let mut spans = Vec::new();

        // Line number
        if show_line_numbers {
            let num_style = if line_idx == cursor_line {
                Style::default().fg(fg_color).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(gutter_color)
            };
            spans.push(Span::styled(
                format!("{:>width$} ", line_idx + 1, width = gutter_width - 1),
                num_style,
            ));
        }

        // Syntax highlighting
        let highlights = syntax::highlight_line(line_content, &language);

        if highlights.is_empty() {
            let style = if line_idx == cursor_line {
                Style::default().fg(fg_color).bg(current_line_bg)
            } else {
                Style::default().fg(fg_color)
            };
            spans.push(Span::styled(line_content.clone(), style));
        } else {
            let chars: Vec<char> = line_content.chars().collect();
            let mut pos = 0;

            for hl in &highlights {
                if pos < hl.start && pos < chars.len() {
                    let gap: String = chars[pos..hl.start.min(chars.len())].iter().collect();
                    spans.push(Span::styled(gap, Style::default().fg(fg_color)));
                }
                if hl.start < chars.len() {
                    let text: String = chars[hl.start..hl.end.min(chars.len())].iter().collect();
                    spans.push(Span::styled(
                        text,
                        Style::default().fg(hl.kind.color(dark)),
                    ));
                }
                pos = hl.end;
            }
            if pos < chars.len() {
                let rest: String = chars[pos..].iter().collect();
                spans.push(Span::styled(rest, Style::default().fg(fg_color)));
            }
        }

        lines.push(Line::from(spans));
    }

    let paragraph = Paragraph::new(lines).style(Style::default().bg(bg_color));
    f.render_widget(paragraph, area);

    // Cursor
    if app.focus == Focus::Editor && cursor_line >= scroll_offset {
        let cursor_x = area.x + gutter_width as u16 + cursor_col as u16;
        let cursor_y = area.y + (cursor_line - scroll_offset) as u16;
        if cursor_x < area.x + area.width && cursor_y < area.y + area.height {
            f.set_cursor_position((cursor_x, cursor_y));
        }
    }
}

fn draw_terminal(f: &mut Frame, app: &App, area: Rect) {
    let border_style = if app.focus == Focus::Terminal {
        Style::default().fg(accent(app))
    } else {
        Style::default().fg(gutter_fg(app))
    };

    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(border_style)
        .title(" Terminal ")
        .title_style(Style::default().fg(fg(app)));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Render terminal output
    if let Some(id) = app.active_terminal {
        let output = app.terminal.read_output(id);
        if !output.is_empty() {
            let text = String::from_utf8_lossy(&output);
            // Simple rendering - strip ANSI codes for now
            let clean = strip_ansi(&text);
            let lines: Vec<Line> = clean
                .lines()
                .rev()
                .take(inner.height as usize)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .map(|l| Line::from(Span::styled(l, Style::default().fg(fg(app)))))
                .collect();

            let paragraph = Paragraph::new(lines).style(Style::default().bg(bg(app)));
            f.render_widget(paragraph, inner);
        }
    }
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let editor = &app.editors[app.active_editor];
    let language = editor
        .file_path
        .as_ref()
        .map(|p| syntax::detect_language(p))
        .unwrap_or("Text");

    let branch = app
        .git_status
        .as_ref()
        .filter(|s| s.is_repo)
        .map(|s| format!(" {} ", s.branch))
        .unwrap_or_default();

    let left = format!(
        " {} {} ",
        app.status_message,
        if app.focus == Focus::Terminal { "[TERMINAL]" } else { "" }
    );

    let right = format!(
        "{}  Ln {}, Col {}  {}  {}  ",
        branch,
        editor.cursor.line + 1,
        editor.cursor.col + 1,
        language,
        if editor.is_dirty { "Modified" } else { "" }
    );

    let padding = area
        .width
        .saturating_sub(left.len() as u16 + right.len() as u16);

    let line = Line::from(vec![
        Span::styled(&left, Style::default().fg(fg(app))),
        Span::styled(" ".repeat(padding as usize), Style::default()),
        Span::styled(&right, Style::default().fg(fg(app))),
    ]);

    let bar = Paragraph::new(line).style(Style::default().bg(status_bg(app)));
    f.render_widget(bar, area);
}

fn draw_command_palette(f: &mut Frame, app: &App, area: Rect) {
    let width = 60u16.min(area.width.saturating_sub(4));
    let height = 15u16.min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(width)) / 2;
    let y = area.height / 6;

    let popup_area = Rect::new(x, y, width, height);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(accent(app)))
        .title(" Command Palette ")
        .title_style(Style::default().fg(accent(app)).add_modifier(Modifier::BOLD))
        .style(Style::default().bg(sidebar_bg(app)));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(inner);

    // Input
    let input = Paragraph::new(format!("> {}", app.palette_input))
        .style(Style::default().fg(fg(app)).bg(selection_bg(app)));
    f.render_widget(input, chunks[0]);

    // Items
    let filtered = app.filtered_palette_items();
    let items: Vec<ListItem> = filtered
        .iter()
        .enumerate()
        .take(chunks[1].height as usize)
        .map(|(i, item)| {
            let style = if i == app.palette_selected {
                Style::default().fg(fg(app)).bg(selection_bg(app))
            } else {
                Style::default().fg(fg(app))
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!(" {} ", item.name), style),
                Span::styled(
                    format!(" {}", item.shortcut),
                    Style::default().fg(gutter_fg(app)),
                ),
            ]))
        })
        .collect();

    let list = List::new(items).style(Style::default().bg(sidebar_bg(app)));
    f.render_widget(list, chunks[1]);
}

fn draw_quick_open(f: &mut Frame, app: &App, area: Rect) {
    let width = 60u16.min(area.width.saturating_sub(4));
    let height = 15u16.min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(width)) / 2;
    let y = area.height / 6;

    let popup_area = Rect::new(x, y, width, height);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(accent(app)))
        .title(" Quick Open ")
        .title_style(Style::default().fg(accent(app)).add_modifier(Modifier::BOLD))
        .style(Style::default().bg(sidebar_bg(app)));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(inner);

    let input = Paragraph::new(format!("> {}", app.quick_open_input))
        .style(Style::default().fg(fg(app)).bg(selection_bg(app)));
    f.render_widget(input, chunks[0]);

    let items: Vec<ListItem> = app
        .quick_open_results
        .iter()
        .enumerate()
        .take(chunks[1].height as usize)
        .map(|(i, entry)| {
            let style = if i == app.quick_open_selected {
                Style::default().fg(fg(app)).bg(selection_bg(app))
            } else {
                Style::default().fg(fg(app))
            };
            let path_display = entry
                .path
                .trim_start_matches(
                    app.file_tree
                        .root_path
                        .as_deref()
                        .unwrap_or(""),
                )
                .trim_start_matches('/');
            ListItem::new(Line::from(vec![
                Span::styled(format!(" {} ", file_icon(&entry.name)), Style::default().fg(accent(app))),
                Span::styled(&entry.name, style),
                Span::styled(
                    format!("  {}", path_display),
                    Style::default().fg(gutter_fg(app)),
                ),
            ]))
        })
        .collect();

    let list = List::new(items).style(Style::default().bg(sidebar_bg(app)));
    f.render_widget(list, chunks[1]);
}

fn draw_find_replace(f: &mut Frame, app: &App, area: Rect) {
    let sidebar_offset = if app.show_sidebar { app.sidebar_width } else { 0 };
    let width = (area.width - sidebar_offset).min(50);
    let height = 4;
    let x = area.width - width - 1;
    let y = 1;

    let popup_area = Rect::new(x, y, width, height);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(accent(app)))
        .title(" Find & Replace ")
        .style(Style::default().bg(sidebar_bg(app)));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(inner);

    // Find input
    let find_style = if !app.find_focus_replace {
        Style::default().fg(fg(app)).bg(selection_bg(app))
    } else {
        Style::default().fg(fg(app))
    };
    let match_info = if !app.find_matches.is_empty() {
        format!(
            " {}/{}",
            app.find_current + 1,
            app.find_matches.len()
        )
    } else if !app.find_input.is_empty() {
        " 0".to_string()
    } else {
        String::new()
    };
    let find = Paragraph::new(format!(
        "Find: {}{}  [{}] [{}]",
        app.find_input,
        match_info,
        if app.find_case_sensitive { "Cc" } else { "cc" },
        if app.find_use_regex { ".*" } else { ".." },
    ))
    .style(find_style);
    f.render_widget(find, chunks[0]);

    // Replace input
    let replace_style = if app.find_focus_replace {
        Style::default().fg(fg(app)).bg(selection_bg(app))
    } else {
        Style::default().fg(fg(app))
    };
    let replace = Paragraph::new(format!("Repl: {}", app.replace_input)).style(replace_style);
    f.render_widget(replace, chunks[1]);
}

fn draw_goto_line(f: &mut Frame, app: &App, area: Rect) {
    let width = 30u16.min(area.width.saturating_sub(4));
    let height = 3;
    let x = (area.width.saturating_sub(width)) / 2;
    let y = area.height / 4;

    let popup_area = Rect::new(x, y, width, height);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(accent(app)))
        .title(" Go to Line ")
        .style(Style::default().bg(sidebar_bg(app)));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let editor = &app.editors[app.active_editor];
    let input = Paragraph::new(format!(
        "Line (1-{}): {}",
        editor.line_count(),
        app.goto_input
    ))
    .style(Style::default().fg(fg(app)));
    f.render_widget(input, inner);
}

fn file_icon(name: &str) -> &'static str {
    let ext = std::path::Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    match ext {
        "rs" => "🦀 ",
        "js" | "mjs" | "cjs" => "⬡ ",
        "ts" | "mts" | "cts" => "⬡ ",
        "jsx" | "tsx" => "⚛ ",
        "py" => "🐍 ",
        "go" => "🔵 ",
        "html" | "htm" => "🌐 ",
        "css" | "scss" | "sass" => "🎨 ",
        "json" => "📋 ",
        "md" | "markdown" => "📝 ",
        "toml" | "yaml" | "yml" => "⚙ ",
        "sh" | "bash" | "zsh" => "🔧 ",
        "sql" => "🗃 ",
        "php" => "🐘 ",
        "java" => "☕ ",
        "svg" | "png" | "jpg" | "jpeg" | "gif" => "🖼 ",
        "lock" => "🔒 ",
        "env" => "🔐 ",
        _ => "📄 ",
    }
}

fn draw_input_dialog(f: &mut Frame, app: &App, area: Rect, title: &str, label: &str) {
    let width = 50u16.min(area.width.saturating_sub(4));
    let height = 5;
    let x = (area.width.saturating_sub(width)) / 2;
    let y = area.height / 3;

    let popup_area = Rect::new(x, y, width, height);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(accent(app)))
        .title(format!(" {} ", title))
        .title_style(Style::default().fg(accent(app)).add_modifier(Modifier::BOLD))
        .style(Style::default().bg(sidebar_bg(app)));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1), Constraint::Length(1)])
        .split(inner);

    // Context path
    let ctx = &app.dialog_context_path;
    let short_ctx = if ctx.len() > (width as usize - 4) {
        format!("...{}", &ctx[ctx.len().saturating_sub(width as usize - 7)..])
    } else {
        ctx.clone()
    };
    let ctx_line = Paragraph::new(format!(" in: {}", short_ctx))
        .style(Style::default().fg(gutter_fg(app)));
    f.render_widget(ctx_line, chunks[0]);

    // Label + input
    let input = Paragraph::new(format!(" {} {}_", label, app.dialog_input))
        .style(Style::default().fg(fg(app)).bg(selection_bg(app)));
    f.render_widget(input, chunks[1]);

    // Hint
    let hint = Paragraph::new(" Enter: confirm  Esc: cancel")
        .style(Style::default().fg(gutter_fg(app)));
    f.render_widget(hint, chunks[2]);
}

fn draw_confirm_dialog(f: &mut Frame, app: &App, area: Rect) {
    let width = 50u16.min(area.width.saturating_sub(4));
    let height = 5;
    let x = (area.width.saturating_sub(width)) / 2;
    let y = area.height / 3;

    let popup_area = Rect::new(x, y, width, height);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red))
        .title(" Delete ")
        .title_style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
        .style(Style::default().bg(sidebar_bg(app)));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let name = std::path::Path::new(&app.dialog_context_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1), Constraint::Length(1)])
        .split(inner);

    let msg = Paragraph::new(format!(" Delete \"{}\"?", name))
        .style(Style::default().fg(Color::Red));
    f.render_widget(msg, chunks[0]);

    let warn = Paragraph::new(" This cannot be undone!")
        .style(Style::default().fg(Color::Yellow));
    f.render_widget(warn, chunks[1]);

    let hint = Paragraph::new(" y: yes  n/Esc: cancel")
        .style(Style::default().fg(gutter_fg(app)));
    f.render_widget(hint, chunks[2]);
}

fn draw_commit_dialog(f: &mut Frame, app: &App, area: Rect) {
    let width = 60u16.min(area.width.saturating_sub(4));
    let height = 4;
    let x = (area.width.saturating_sub(width)) / 2;
    let y = area.height / 3;

    let popup_area = Rect::new(x, y, width, height);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green))
        .title(" Git Commit ")
        .title_style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
        .style(Style::default().bg(sidebar_bg(app)));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(inner);

    let input = Paragraph::new(format!(" Message: {}_", app.commit_message))
        .style(Style::default().fg(fg(app)).bg(selection_bg(app)));
    f.render_widget(input, chunks[0]);

    let hint = Paragraph::new(" Enter: commit  Esc: cancel")
        .style(Style::default().fg(gutter_fg(app)));
    f.render_widget(hint, chunks[1]);
}

fn draw_save_as_dialog(f: &mut Frame, app: &App, area: Rect) {
    let width = 60u16.min(area.width.saturating_sub(4));
    let height = 4;
    let x = (area.width.saturating_sub(width)) / 2;
    let y = area.height / 3;

    let popup_area = Rect::new(x, y, width, height);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(accent(app)))
        .title(" Save As ")
        .title_style(Style::default().fg(accent(app)).add_modifier(Modifier::BOLD))
        .style(Style::default().bg(sidebar_bg(app)));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(inner);

    let input = Paragraph::new(format!(" Path: {}_", app.save_as_input))
        .style(Style::default().fg(fg(app)).bg(selection_bg(app)));
    f.render_widget(input, chunks[0]);

    let hint = Paragraph::new(" Enter: save  Esc: cancel")
        .style(Style::default().fg(gutter_fg(app)));
    f.render_widget(hint, chunks[1]);
}

fn strip_ansi(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next.is_alphabetic() || next == 'H' || next == 'J' || next == 'K' {
                        break;
                    }
                }
            }
        } else {
            result.push(c);
        }
    }

    result
}
