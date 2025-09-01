use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, AppMode, EditMode};
use crate::config::{Config, KeyBinding};
use secrecy::ExposeSecret;
use crate::note::Note;

fn calculate_help_height(help_text: &str, available_width: u16) -> u16 {
    if help_text.is_empty() {
        return 3;
    }
    
    let usable_width = available_width.saturating_sub(4) as usize;
    if usable_width == 0 {
        return 3;
    }
    
    let words: Vec<&str> = help_text.split_whitespace().collect();
    let mut lines_needed = 1u16;
    let mut current_line_len = 0usize;
    
    for word in words {
        let word_len = word.len();
        
        if current_line_len + word_len > usable_width {
            if word_len > usable_width {
                lines_needed += (word_len + usable_width - 1) as u16 / usable_width as u16;
                current_line_len = word_len % usable_width;
            } else {
                lines_needed += 1;
                current_line_len = word_len;
            }
        } else {
            if current_line_len > 0 {
                current_line_len += 1;
            }
            current_line_len += word_len;
        }
    }
    
    (lines_needed + 2).max(3)
}

fn generate_help_text(app: &App, config: &Config) -> String {
    let kb = &config.keybindings;
    match app.mode {
        AppMode::PasswordPrompt => {
            "Enter password to unlock encrypted notes | Esc: Quit".to_string()
        }
        AppMode::PasswordSetup => {
            "Create a password for your new encrypted notes vault | Esc: Quit".to_string()
        }
        AppMode::NoteList => {
            let base_help = format!("{}: Navigate | {}: View | {}: Edit | {}: New Note | {}: Search | {}: Pin | {}: Delete | {}: Quit",
                format!("{}/{}", format_keybinding(&kb.move_up), format_keybinding(&kb.move_down)),
                format_keybinding(&kb.view_note),
                format_keybinding(&kb.edit_note),
                format_keybinding(&kb.create_note),
                format_keybinding(&kb.search_notes),
                format_keybinding(&kb.toggle_pin),
                format_keybinding(&kb.delete_note),
                format_keybinding(&kb.quit)
            );
            if config.behavior.encryption_enabled {
                format!("{} | {}: Export Backup", base_help, format_keybinding(&kb.export_plaintext))
            } else {
                base_help
            }
        }
        AppMode::Searching => {
            format!("Type to search | {}: Navigate Results | {}/{}: View Selected | {}: Exit Search | {}: Quit",
                format!("{}/{}", format_keybinding(&kb.move_up), format_keybinding(&kb.move_down)),
                format_keybinding(&kb.search_select),
                format_keybinding(&kb.search_view),
                format_keybinding(&kb.exit_search),
                format_keybinding(&kb.quit)
            )
        }
        AppMode::ViewingNote => {
            format!("{}: Return to List | {}: Edit Note | {}: Scroll | {}: Page | {}: Quit",
                format_keybinding(&kb.return_to_list),
                format_keybinding(&kb.edit_from_view),
                format!("{}/{}", format_keybinding(&kb.move_up), format_keybinding(&kb.move_down)),
                format!("{}/{}", format_keybinding(&kb.page_up), format_keybinding(&kb.page_down)),
                format_keybinding(&kb.quit)
            )
        }
        AppMode::EditingNote => {
            let save_text = if config.behavior.auto_save {
                format!("{}: Return | {}: Save Now", 
                    format_keybinding(&kb.save_and_exit),
                    format_keybinding(&kb.manual_save))
            } else {
                format!("{}: Save & Return | {}: Save", 
                    format_keybinding(&kb.save_and_exit),
                    format_keybinding(&kb.manual_save))
            };
            format!("{} | {}: Switch | {}: Toggle Selection | ←/→/↑/↓: Move | Ctrl+↑/↓: Scroll | {}: Page",
                save_text,
                format_keybinding(&kb.switch_field),
                format_keybinding(&kb.toggle_highlighting),
                format!("{}/{}", format_keybinding(&kb.page_up), format_keybinding(&kb.page_down))
            )
        }
        AppMode::CreatingNote => {
            format!("{}: Save & Return | {}: Save Now | {}: Switch | {}: Toggle Selection | ←/→/↑/↓: Move | Ctrl+↑/↓: Scroll | {}: Page",
                format_keybinding(&kb.save_and_exit),
                format_keybinding(&kb.manual_save),
                format_keybinding(&kb.switch_field),
                format_keybinding(&kb.toggle_highlighting),
                format!("{}/{}", format_keybinding(&kb.page_up), format_keybinding(&kb.page_down))
            )
        }
        AppMode::ConfirmingDelete => {
            format!("{}: Confirm Deletion | {}: Cancel",
                format_keybinding_vec(&kb.confirm_delete),
                format_keybinding_vec(&kb.cancel_delete)
            )
        }
        AppMode::ConfirmingUnsavedExit => {
            format!("{}: Save & Exit | {}: Discard & Exit | {}: Cancel",
                format_keybinding_vec(&kb.save_and_exit_unsaved),
                format_keybinding_vec(&kb.discard_and_exit),
                format_keybinding_vec(&kb.cancel_exit)
            )
        }
        AppMode::ConfirmingExport => {
            "Y/y: Confirm Export (opens file dialog) | N/n/Esc: Cancel".to_string()
        }
        AppMode::EncryptedFileWarning => {
            "Your notes file is encrypted, but encryption is disabled in config | Esc/q: Quit".to_string()
        }
    }
}

pub fn draw(f: &mut Frame, app: &mut App, config: &Config) {
    const MIN_WIDTH: u16 = 50;
    const MIN_HEIGHT: u16 = 10;
    
    if f.area().width < MIN_WIDTH || f.area().height < MIN_HEIGHT {
        let warning_text = format!(
            "Terminal too small!\nMinimum size: {}x{}\nCurrent size: {}x{}",
            MIN_WIDTH, MIN_HEIGHT, f.area().width, f.area().height
        );
        
        let warning = Paragraph::new(warning_text)
            .style(Style::default().fg(config.colors.text.to_color()))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .title("Warning")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(config.colors.delete_dialog_border.to_color())),
            )
            .wrap(Wrap { trim: true });
        
        f.render_widget(warning, f.area());
        return;
    }

    let constraints = if app.help_visible {
        let help_text = generate_help_text(app, config);
        let help_height = calculate_help_height(&help_text, f.area().width);
        
        vec![
            Constraint::Length(3),           // title
            Constraint::Min(0),              // main content
            Constraint::Length(help_height), // help (with dynamic height)
        ]
    } else {
        vec![
            Constraint::Length(3),    // title
            Constraint::Min(0),       // main content, takes all remaining space
        ]
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(f.area());

    draw_title(f, chunks[0], config);
    
    match app.mode {
        AppMode::PasswordPrompt => {
            draw_password_prompt(f, chunks[1], app, config);
        }
        AppMode::PasswordSetup => {
            draw_password_setup(f, chunks[1], app, config);
        }
        AppMode::NoteList => {
            draw_note_list(f, chunks[1], app, config);
        }
        AppMode::Searching => {
            draw_search_mode(f, chunks[1], app, config);
        }
        AppMode::ViewingNote => {
            draw_viewer(f, chunks[1], app, config);
        }
        AppMode::EditingNote | AppMode::CreatingNote => {
            draw_editor(f, chunks[1], app, config);
        }
        AppMode::ConfirmingDelete => {
            draw_note_list(f, chunks[1], app, config);
            draw_delete_confirmation(f, f.area(), app, config);
        }
        AppMode::ConfirmingUnsavedExit => {
            draw_editor(f, chunks[1], app, config);
            draw_unsaved_changes_confirmation(f, f.area(), app, config);
        }
        AppMode::ConfirmingExport => {
            draw_note_list(f, chunks[1], app, config);
            draw_export_confirmation(f, f.area(), app, config);
        }
        AppMode::EncryptedFileWarning => {
            draw_encrypted_file_warning(f, chunks[1], app, config);
        }
    }
    
    if app.help_visible {
        draw_help(f, chunks[2], app, config);
    }
}

fn draw_title(f: &mut Frame, area: Rect, config: &Config) {
    let title = Paragraph::new("Notes")
        .style(Style::default().fg(config.colors.title_bar.to_color()).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(config.colors.border_inactive.to_color())),
        );
    f.render_widget(title, area);
}

fn draw_note_list(f: &mut Frame, area: Rect, app: &mut App, config: &Config) {
    let selected_index = app.selected_note_index;
    let notes = app.get_notes();
    let notes_len = notes.len();
    draw_note_list_generic(f, area, &notes, selected_index, "Notes", notes_len, config);
}

fn draw_search_mode(f: &mut Frame, area: Rect, app: &mut App, config: &Config) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(area);

    let search_paragraph = Paragraph::new(app.search_query.as_str())
        .style(Style::default().fg(config.colors.text.to_color()))
        .block(
            Block::default()
                .title(format!("Search ({})", app.search_results.len()))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(config.colors.search_border.to_color())),
        );

    f.render_widget(search_paragraph, chunks[0]);

    let cursor_x = chunks[0].x + 1 + app.search_cursor_position.min(chunks[0].width.saturating_sub(2) as usize) as u16;
    let cursor_y = chunks[0].y + 1;
    f.set_cursor_position((cursor_x, cursor_y));

    let selected_index = app.selected_note_index;
    let search_results_len = app.search_results.len();
    let search_notes = app.get_search_results();
    draw_note_list_generic(f, chunks[1], &search_notes, selected_index, "Search Results", search_results_len, config);
}

fn draw_note_list_generic(f: &mut Frame, area: Rect, notes: &[&Note], selected_index: usize, title: &str, total_count: usize, config: &Config) {
    if notes.is_empty() {
        let empty_msg = if title == "Search Results" {
            if total_count == 0 {
                "Start typing to search notes..."
            } else {
                "No notes match your search."
            }
        } else {
            &format!("No notes available. Press '{}' to create a new note.", format_keybinding(&config.keybindings.create_note))
        };
        
        let empty_paragraph = Paragraph::new(empty_msg)
            .style(Style::default().fg(config.colors.text_secondary.to_color()))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(config.colors.border_inactive.to_color())),
            );
        f.render_widget(empty_paragraph, area);
        return;
    }

    let items: Vec<ListItem> = notes
        .iter()
        .enumerate()
        .map(|(i, note)| {
            let preview = note.content
                .lines()
                .next()
                .unwrap_or("")
                .chars()
                .take(50)
                .collect::<String>();
            
            let preview = if preview.len() < note.content.len() {
                format!("{}...", preview)
            } else {
                preview
            };

            let content = vec![
                Line::from({
                    let mut spans = vec![];
                    if note.pinned {
                        spans.push(Span::styled("* ", Style::default().add_modifier(Modifier::BOLD)));
                    }
                    spans.push(Span::styled(&note.title, Style::default().add_modifier(Modifier::BOLD)));
                    spans
                }),
                Line::from(vec![
                    Span::styled(preview, Style::default().fg(config.colors.text_secondary.to_color())),
                ]),
                Line::from(vec![
                    Span::styled(
                        format!("Updated: {}", note.updated_at.format("%Y-%m-%d %H:%M")),
                        Style::default().fg(config.colors.text_secondary.to_color()),
                    ),
                ]),
            ];

            ListItem::new(content).style(
                if i == selected_index {
                    Style::default().bg(config.colors.background_selected.to_bg_color())
                } else {
                    Style::default()
                }
            )
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(config.colors.border_inactive.to_color())),
        );

    f.render_widget(list, area);
}

fn draw_viewer(f: &mut Frame, area: Rect, app: &App, config: &Config) {
    if let Some(note) = &app.viewing_note {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
            ])
            .split(area);

        let title_paragraph = Paragraph::new(note.title.as_str())
            .style(Style::default().fg(config.colors.text.to_color()).add_modifier(Modifier::BOLD))
            .block(
                Block::default()
                    .title("Title (Read-Only)")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(config.colors.border_active.to_color())),
            );

        f.render_widget(title_paragraph, chunks[0]);

        let content_lines: Vec<&str> = note.content.lines().collect();
        let visible_height = chunks[1].height.saturating_sub(2) as usize;
        let start_line = app.scroll_offset.min(content_lines.len().saturating_sub(1));
        let end_line = (start_line + visible_height).min(content_lines.len());
        
        let visible_content = if start_line < content_lines.len() {
            content_lines[start_line..end_line].join("\n")
        } else {
            String::new()
        };

        let scroll_indicator = if content_lines.len() > visible_height {
            format!(" (Line {}/{}) ↑/↓ Scroll, PgUp/PgDn", start_line + 1, content_lines.len())
        } else {
            " (Read-Only)".to_string()
        };

        let content_paragraph = Paragraph::new(visible_content)
            .style(Style::default().fg(config.colors.text.to_color()))
            .block(
                Block::default()
                    .title(format!("Content{}", scroll_indicator))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(config.colors.border_active.to_color())),
            )
            .wrap(Wrap { trim: false });

        f.render_widget(content_paragraph, chunks[1]);
    }
}

fn draw_editor(f: &mut Frame, area: Rect, app: &mut App, config: &Config) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(area);

    app.title_textarea.set_block(
        Block::default()
            .title("Title")
            .borders(Borders::ALL)
            .border_style(if app.edit_mode == EditMode::Title {
                Style::default().fg(config.colors.border_active.to_color())
            } else {
                Style::default().fg(config.colors.border_inactive.to_color())
            }),
    );

    if app.edit_mode == EditMode::Title {
        app.title_textarea.set_cursor_style(Style::default().bg(config.colors.text_highlight.to_color()));
        app.content_textarea.set_cursor_style(Style::default());
    } else {
        app.title_textarea.set_cursor_style(Style::default());
        app.content_textarea.set_cursor_style(Style::default().bg(config.colors.text_highlight.to_color()));
    }

    if app.highlighting_enabled {
        if app.edit_mode == EditMode::Title {
            app.title_textarea.set_cursor_line_style(Style::default().add_modifier(Modifier::UNDERLINED));
            app.content_textarea.set_cursor_line_style(Style::default());
        } else {
            app.title_textarea.set_cursor_line_style(Style::default());
            app.content_textarea.set_cursor_line_style(Style::default().add_modifier(Modifier::UNDERLINED));
        }
    } else {
        app.title_textarea.set_cursor_line_style(Style::default());
        app.content_textarea.set_cursor_line_style(Style::default());
    }

    let title_text = match app.mode {
        AppMode::CreatingNote => "Creating New Note",
        AppMode::EditingNote => "Editing Note",
        _ => "Content",
    };
    
    app.content_textarea.set_block(
        Block::default()
            .title(title_text)
            .borders(Borders::ALL)
            .border_style(if app.edit_mode == EditMode::Content {
                Style::default().fg(config.colors.border_active.to_color())
            } else {
                Style::default().fg(config.colors.border_inactive.to_color())
            }),
    );

    f.render_widget(&app.title_textarea, chunks[0]);
    f.render_widget(&app.content_textarea, chunks[1]);
}

fn format_keybinding(kb: &KeyBinding) -> String {
    let mut parts = Vec::new();
    
    if kb.ctrl {
        parts.push("Ctrl");
    }
    if kb.alt {
        parts.push("Alt");
    }
    if kb.shift {
        parts.push("Shift");
    }
    
    parts.push(&kb.key);
    parts.join("+")
}

fn format_keybinding_vec(kbs: &[KeyBinding]) -> String {
    kbs.iter()
        .map(|kb| format_keybinding(kb))
        .collect::<Vec<_>>()
        .join("/")
}

fn draw_help(f: &mut Frame, area: Rect, app: &App, config: &Config) {
    let help_text = generate_help_text(app, config);
    
    let available_width = area.width.saturating_sub(2) as usize; // minus borders
    let text_lines = wrap_text_lines(&help_text, available_width.saturating_sub(2));
    
    let centered_text = center_text_lines(text_lines, available_width.saturating_sub(2));
    
    let help = Paragraph::new(centered_text)
        .style(Style::default().fg(config.colors.help_text.to_color()))
        .alignment(Alignment::Left)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(config.colors.border_inactive.to_color()))
                .padding(ratatui::widgets::Padding::horizontal(1)),
        );

    f.render_widget(help, area);
}

fn wrap_text_lines(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }
    
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut lines = Vec::new();
    let mut current_line = String::new();
    
    for word in words {
        if current_line.is_empty() {
            current_line = word.to_string();
        } else if current_line.len() + 1 + word.len() <= width {
            current_line.push(' ');
            current_line.push_str(word);
        } else {
            lines.push(current_line);
            current_line = word.to_string();
        }
    }
    
    if !current_line.is_empty() {
        lines.push(current_line);
    }
    
    lines
}

fn center_text_lines(lines: Vec<String>, width: usize) -> String {
    lines
        .into_iter()
        .map(|line| {
            if line.len() >= width {
                line
            } else {
                let padding = (width - line.len()) / 2;
                format!("{}{}", " ".repeat(padding), line)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn draw_delete_confirmation(f: &mut Frame, area: Rect, app: &App, config: &Config) {
    let dialog_width = 60.min(area.width - 4);
    let dialog_height = 7;
    let dialog_x = (area.width.saturating_sub(dialog_width)) / 2;
    let dialog_y = (area.height.saturating_sub(dialog_height)) / 2;
    
    let dialog_area = Rect {
        x: dialog_x,
        y: dialog_y,
        width: dialog_width,
        height: dialog_height,
    };

    f.render_widget(Clear, dialog_area);

    let note_title = &app.delete_note_title;
    let truncated_title = if note_title.len() > 40 {
        format!("{}...", &note_title[..37])
    } else {
        note_title.clone()
    };

    let confirmation_text = format!(
        "Delete note: '{}'\n\nThis action cannot be undone.\n\nPress '{}' to confirm, '{}' to cancel.",
        truncated_title,
        format_keybinding_vec(&config.keybindings.confirm_delete),
        format_keybinding_vec(&config.keybindings.cancel_delete)
    );

    let dialog = Paragraph::new(confirmation_text)
        .style(Style::default().fg(config.colors.text.to_color()))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .title("Confirm Deletion")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(config.colors.delete_dialog_border.to_color()).add_modifier(Modifier::BOLD))
                .style(Style::default().bg(config.colors.delete_dialog_border.to_bg_color())),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(dialog, dialog_area);
}

fn draw_unsaved_changes_confirmation(f: &mut Frame, area: Rect, _app: &App, config: &Config) {
    let dialog_width = 60.min(area.width - 4);
    let dialog_height = 8;
    let dialog_x = (area.width.saturating_sub(dialog_width)) / 2;
    let dialog_y = (area.height.saturating_sub(dialog_height)) / 2;
    
    let dialog_area = Rect {
        x: dialog_x,
        y: dialog_y,
        width: dialog_width,
        height: dialog_height,
    };

    f.render_widget(Clear, dialog_area);

    let confirmation_text = format!(
        "You have unsaved changes.\n\nWhat would you like to do?\n\nPress '{}' to save and exit\nPress '{}' to discard changes and exit\nPress '{}' to cancel and continue editing",
        format_keybinding_vec(&config.keybindings.save_and_exit_unsaved),
        format_keybinding_vec(&config.keybindings.discard_and_exit),
        format_keybinding_vec(&config.keybindings.cancel_exit)
    );

    let dialog = Paragraph::new(confirmation_text)
        .style(Style::default().fg(config.colors.text.to_color()))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .title("Unsaved Changes")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(config.colors.delete_dialog_border.to_color()).add_modifier(Modifier::BOLD))
                .style(Style::default().bg(config.colors.delete_dialog_border.to_bg_color())),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(dialog, dialog_area);
}

fn draw_password_prompt(f: &mut Frame, area: Rect, app: &App, config: &Config) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(8),
            Constraint::Min(0),
        ])
        .split(area);

    let password_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(70),
            Constraint::Min(0),
        ])
        .split(chunks[1]);

    let password_display = "*".repeat(app.password_input.expose_secret().len());
    
    let title = if app.password_error.is_some() {
        "🔒 Password Required (Error)"
    } else {
        "🔒 Password Required"
    };

    let mut content = vec![
        Line::from("Enter your password to unlock encrypted notes:"),
        Line::from(""),
        Line::from(vec![
            Span::styled("> ", Style::default().fg(config.colors.text.to_color())),
            Span::styled(password_display, Style::default().fg(config.colors.text.to_color())),
        ]),
    ];

    if let Some(error) = &app.password_error {
        content.push(Line::from(""));
        content.push(Line::from(vec![
            Span::styled("Error: ", Style::default().fg(config.colors.delete_dialog_border.to_color())),
            Span::styled(error, Style::default().fg(config.colors.delete_dialog_border.to_color())),
        ]).alignment(Alignment::Center));
    } else if app.password_limit_reached {
        content.push(Line::from(""));
        content.push(Line::from(vec![
            Span::styled("Maximum password length reached (64 characters)", 
                Style::default().fg(config.colors.delete_dialog_border.to_color())),
        ]).alignment(Alignment::Center));
    }

    let password_block = Paragraph::new(content)
        .style(Style::default().fg(config.colors.text.to_color()))
        .alignment(Alignment::Left)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(if app.password_error.is_some() {
                    Style::default().fg(config.colors.delete_dialog_border.to_color())
                } else {
                    Style::default().fg(config.colors.border_active.to_color())
                }),
        );

    f.render_widget(password_block, password_area[1]);

    let cursor_x = password_area[1].x + 3 + app.password_input.expose_secret().len() as u16;
    let cursor_y = password_area[1].y + 3;
    f.set_cursor_position((cursor_x, cursor_y));
}

fn draw_password_setup(f: &mut Frame, area: Rect, app: &App, config: &Config) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(8),
            Constraint::Min(0),
        ])
        .split(area);

    let password_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(70),
            Constraint::Min(0),
        ])
        .split(chunks[1]);

    let password_display = "*".repeat(app.password_input.expose_secret().len());
    
    let title = if app.password_error.is_some() {
        "🔐 Set Up Encryption (Error)"
    } else {
        "🔐 Set Up Encryption"
    };

    let mut content = vec![
        Line::from("Create a password for your new encrypted notes vault.").alignment(Alignment::Center),
        Line::from("The password must be 8-64 characters long.").alignment(Alignment::Center),
        Line::from(""),
        Line::from(vec![
            Span::styled("> ", Style::default().fg(config.colors.text.to_color())),
            Span::styled(password_display, Style::default().fg(config.colors.text.to_color())),
        ]),
    ];

    if let Some(error) = &app.password_error {
        content.push(Line::from(""));
        content.push(Line::from(vec![
            Span::styled("Error: ", Style::default().fg(config.colors.delete_dialog_border.to_color())),
            Span::styled(error, Style::default().fg(config.colors.delete_dialog_border.to_color())),
        ]).alignment(Alignment::Center));
    } else if app.password_limit_reached {
        content.push(Line::from(""));
        content.push(Line::from(vec![
            Span::styled("Maximum password length reached (64 characters)", 
                Style::default().fg(config.colors.delete_dialog_border.to_color())),
        ]).alignment(Alignment::Center));
    }

    let password_block = Paragraph::new(content)
        .style(Style::default().fg(config.colors.text.to_color()))
        .alignment(Alignment::Left)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(if app.password_error.is_some() {
                    Style::default().fg(config.colors.delete_dialog_border.to_color())
                } else {
                    Style::default().fg(config.colors.border_active.to_color())
                }),
        );

    f.render_widget(password_block, password_area[1]);

    let cursor_x = password_area[1].x + 3 + app.password_input.expose_secret().len() as u16;
    let cursor_y = password_area[1].y + 4;
    f.set_cursor_position((cursor_x, cursor_y));
}

fn draw_encrypted_file_warning(f: &mut Frame, area: Rect, _app: &App, config: &Config) {
    let dialog_width = 80.min(area.width - 4);
    let dialog_height = 12;
    let dialog_x = (area.width.saturating_sub(dialog_width)) / 2;
    let dialog_y = (area.height.saturating_sub(dialog_height)) / 2;
    
    let dialog_area = Rect {
        x: dialog_x,
        y: dialog_y,
        width: dialog_width,
        height: dialog_height,
    };

    f.render_widget(Clear, dialog_area);

    let warning_text = "⚠️  ENCRYPTED FILE DETECTED  ⚠️\n\n\
        Your notes file appears to be encrypted, but encryption is disabled in your configuration.\n\n\
        To access your encrypted notes:\n\
        1. Enable encryption in your config file (~/.config/tui-notes/config.toml)\n\
        2. Set 'encryption_enabled = true' in the [behavior] section\n\
        3. Restart the application\n\n\
        Or use a different notes file by changing 'default_notes_file' in config.\n\n\
        Press 'Esc' or 'q' to quit.";

    let dialog = Paragraph::new(warning_text)
        .block(
            Block::default()
                .title("Configuration Error")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(config.colors.delete_dialog_border.to_color()).add_modifier(Modifier::BOLD))
                .style(Style::default().bg(config.colors.delete_dialog_border.to_bg_color())),
        )
        .wrap(Wrap { trim: true })
        .style(Style::default().fg(config.colors.text.to_color()));

    f.render_widget(dialog, dialog_area);
}

fn draw_export_confirmation(f: &mut Frame, area: Rect, _app: &App, config: &Config) {
    let dialog_width = 70.min(area.width - 4);
    let dialog_height = 9;
    let dialog_x = (area.width.saturating_sub(dialog_width)) / 2;
    let dialog_y = (area.height.saturating_sub(dialog_height)) / 2;
    
    let dialog_area = Rect {
        x: dialog_x,
        y: dialog_y,
        width: dialog_width,
        height: dialog_height,
    };

    f.render_widget(Clear, dialog_area);

    let warning_text = "⚠️  PLAINTEXT EXPORT WARNING  ⚠️\n\n\
        You are about to export your notes in PLAINTEXT format.\n\
        This will create an unencrypted backup file that anyone can read.\n\n\
        Are you sure you want to continue?\n\n\
        Press 'Y' to open file dialog and choose location\n\
        Press 'N' to cancel";

    let dialog = Paragraph::new(warning_text)
        .style(Style::default().fg(config.colors.text.to_color()))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .title("Export Confirmation")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(config.colors.delete_dialog_border.to_color()).add_modifier(Modifier::BOLD))
                .style(Style::default().bg(config.colors.delete_dialog_border.to_bg_color())),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(dialog, dialog_area);
}

