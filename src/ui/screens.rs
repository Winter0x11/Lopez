//! Screen rendering. Pure display; state is read-only.

use crate::app::{AppState, Screen};
use crate::disk::BlockDevice;
use ratatui::prelude::*;
use ratatui::text::{Line, Span};
use ratatui::layout::Alignment;
use ratatui::widgets::block::BorderType;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};

pub fn render(frame: &mut Frame, state: &AppState, area: Rect) {
    match state.screen {
        Screen::NotRoot => render_not_root(frame, area),
        Screen::DriveList => render_drive_list(frame, state, area),
        Screen::IsoSelect => render_iso_select(frame, state, area),
        Screen::Confirm => render_confirm(frame, state, area),
        Screen::FlashProgress => render_flash_progress(frame, state, area),
        Screen::Result => render_result(frame, state, area),
    }
}

fn render_not_root(frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .title(Span::styled(
            " ⚠️  Error ",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Red));

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Must be run as root.",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Usage: ",
            Style::default().fg(Color::White),
        )),
        Line::from(Span::styled(
            "  sudo lopez",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Press ",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            "q",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            " or ",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            "Esc",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            " to quit.",
            Style::default().fg(Color::DarkGray),
        )),
    ];
    let p = Paragraph::new(text)
        .block(block)
        .wrap(Wrap { trim: true })
        .alignment(Alignment::Center);
    frame.render_widget(p, area);
}

fn render_drive_list(frame: &mut Frame, state: &AppState, area: Rect) {
    let title = Span::styled(
        " 💾 Removable Drives ",
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    );
    let help = Span::styled(
        " ↑/↓ select • Enter confirm • q quit • Esc back ",
        Style::default().fg(Color::DarkGray),
    );
    let block = Block::default()
        .title(title)
        .title_bottom(help)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));

    let items: Vec<ListItem> = if state.drives.is_empty() {
        vec![ListItem::new(Span::styled(
            "  No removable drives found.",
            Style::default().fg(Color::DarkGray),
        ))]
    } else {
        state
            .drives
            .iter()
            .enumerate()
            .map(|(i, d)| {
                let is_selected = i == state.selected_drive_index;
                let prefix = if is_selected {
                    Span::styled("▶ ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                } else {
                    Span::styled("  ", Style::default())
                };
                let device = Span::styled(
                    d.path(),
                    if is_selected {
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    },
                );
                let size = Span::styled(
                    format!("  {}  ", d.size),
                    Style::default().fg(Color::Green),
                );
                let mount = d.mountpoint.as_deref().unwrap_or("—");
                let mount_style = if mount != "—" {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                let mount_span = Span::styled(format!("mount: {}", mount), mount_style);
                ListItem::new(Line::from(vec![prefix, device, size, mount_span]))
            })
            .collect()
    };

    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}

fn render_iso_select(frame: &mut Frame, state: &AppState, area: Rect) {
    // Split area: input section at top, file list below
    let chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Length(8),
            ratatui::layout::Constraint::Min(5),
        ])
        .split(area);

    // Input section
    let title = Span::styled(
        " 📁 ISO Path Selection ",
        Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD),
    );
    let help = Span::styled(
        " ↑/↓ navigate • Tab select • Enter confirm • Esc back • q quit ",
        Style::default().fg(Color::DarkGray),
    );
    let block = Block::default()
        .title(title)
        .title_bottom(help)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Blue));

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Path: ",
            Style::default().fg(Color::White),
        )),
    ];
    let input_line = Line::from(vec![
        Span::styled(
            state.iso_input.as_str(),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        Span::styled("▌", Style::default().fg(Color::Yellow)),
    ]);
    lines.push(input_line);
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Accepted extensions: ",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(Span::styled(
        "  .iso  .img",
        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
    )));
    if let Some(ref e) = state.iso_error {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("❌ Error: {}", e),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )));
    }
    let p = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: true });
    frame.render_widget(p, chunks[0]);

    // File list section
    if !state.iso_file_matches.is_empty() {
        let list_title = Span::styled(
            format!(" 📋 Matching Files ({}) ", state.iso_file_matches.len()),
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        );
        let list_block = Block::default()
            .title(list_title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Green));

        let items: Vec<ListItem> = state
            .iso_file_matches
            .iter()
            .enumerate()
            .map(|(i, path)| {
                let is_selected = i == state.iso_file_selected_index;
                let prefix = if is_selected {
                    Span::styled("▶ ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                } else {
                    Span::styled("  ", Style::default())
                };
                let path_str = path.to_string_lossy().into_owned();
                let path_span = Span::styled(
                    path_str,
                    if is_selected {
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    },
                );
                ListItem::new(Line::from(vec![prefix, path_span]))
            })
            .collect();

        let list = List::new(items).block(list_block);
        frame.render_widget(list, chunks[1]);
    } else if !state.iso_input.is_empty() {
        let empty_title = Span::styled(
            " 📋 No Matches ",
            Style::default().fg(Color::DarkGray),
        );
        let empty_block = Block::default()
            .title(empty_title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::DarkGray));
        let empty_text = vec![
            Line::from(""),
            Line::from(Span::styled(
                "No ISO/IMG files found matching your query.",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Try typing part of the filename or path.",
                Style::default().fg(Color::DarkGray),
            )),
        ];
        let empty_p = Paragraph::new(empty_text)
            .block(empty_block)
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Center);
        frame.render_widget(empty_p, chunks[1]);
    } else {
        let hint_title = Span::styled(
            " 💡 Start Typing ",
            Style::default().fg(Color::DarkGray),
        );
        let hint_block = Block::default()
            .title(hint_title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::DarkGray));
        let hint_text = vec![
            Line::from(""),
            Line::from(Span::styled(
                "Start typing to search for ISO/IMG files.",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Files will appear here as you type.",
                Style::default().fg(Color::DarkGray),
            )),
        ];
        let hint_p = Paragraph::new(hint_text)
            .block(hint_block)
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Center);
        frame.render_widget(hint_p, chunks[1]);
    }
}

fn render_confirm(frame: &mut Frame, state: &AppState, area: Rect) {
    let device_path = state
        .selected_drive()
        .map(|d: &BlockDevice| d.path())
        .unwrap_or_else(|| "?".to_string());
    let title = Span::styled(
        " ⚠️  Confirmation Required ",
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
    );
    let help = Span::styled(
        " Type device path to confirm • Esc back • q quit ",
        Style::default().fg(Color::DarkGray),
    );
    let block = Block::default()
        .title(title)
        .title_bottom(help)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Yellow));

    let warning = vec![
        Line::from(""),
        Line::from(Span::styled(
            "⚠️  WARNING: This will ERASE all data on the selected device!",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("Device: ", Style::default().fg(Color::White)),
            Span::styled(
                device_path,
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("ISO: ", Style::default().fg(Color::White)),
            Span::styled(
                state.iso_path.display().to_string(),
                Style::default().fg(Color::Green),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Type the full device path to confirm:",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(
                state.confirmation_input.as_str(),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
            Span::styled("▌", Style::default().fg(Color::Yellow)),
        ]),
    ];
    let p = Paragraph::new(warning)
        .block(block)
        .wrap(Wrap { trim: true });
    frame.render_widget(p, area);
}

fn render_flash_progress(frame: &mut Frame, state: &AppState, area: Rect) {
    let title = Span::styled(
        " ⚡ Flashing ",
        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
    );
    let help = Span::styled(
        " Do not remove device • q quit ",
        Style::default().fg(Color::DarkGray),
    );
    let block = Block::default()
        .title(title)
        .title_bottom(help)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Green));

    let text = if state.flash_progress.is_empty() {
        vec![
            Line::from(""),
            Line::from(Span::styled(
                "Starting dd…",
                Style::default().fg(Color::Cyan),
            )),
        ]
    } else {
        vec![
            Line::from(""),
            Line::from(Span::styled(
                state.flash_progress.as_str(),
                Style::default().fg(Color::Green),
            )),
        ]
    };
    let p = Paragraph::new(text)
        .block(block)
        .wrap(Wrap { trim: true });
    frame.render_widget(p, area);
}

fn render_result(frame: &mut Frame, state: &AppState, area: Rect) {
    let (title_text, title_color, icon) = match &state.flash_done {
        Some(Ok(())) => (
            " ✅ Success ",
            Color::Green,
            "✓",
        ),
        Some(Err(_)) => (
            " ❌ Error ",
            Color::Red,
            "✗",
        ),
        None => (
            " ⚠️  Result ",
            Color::Yellow,
            "?",
        ),
    };
    let title = Span::styled(
        title_text,
        Style::default().fg(title_color).add_modifier(Modifier::BOLD),
    );
    let help = Span::styled(
        " Esc back to list • q quit ",
        Style::default().fg(Color::DarkGray),
    );
    let block = Block::default()
        .title(title)
        .title_bottom(help)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(title_color));

    let (msg, color): (&str, Color) = match &state.flash_done {
        Some(Ok(())) => ("Flash completed successfully!", Color::Green),
        Some(Err(e)) => (e.as_str(), Color::Red),
        None => ("Unknown state.", Color::Gray),
    };
    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("{} {}", icon, msg),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )),
    ];
    let p = Paragraph::new(text)
        .block(block)
        .wrap(Wrap { trim: true })
        .alignment(Alignment::Center);
    frame.render_widget(p, area);
}
