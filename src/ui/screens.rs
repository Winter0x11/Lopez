//! Screen rendering. Pure display; state is read-only.

use crate::app::{AppState, Screen};
use crate::disk::BlockDevice;
use ratatui::prelude::*;
use ratatui::text::{Line, Span};
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
        .title(" Error ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);
    let text = "Must be run as root.\n\nUsage: sudo lopez\n\nPress q or Esc to quit.";
    let p = Paragraph::new(text)
        .block(block)
        .wrap(Wrap { trim: true });
    frame.render_widget(p, area);
}

fn render_drive_list(frame: &mut Frame, state: &AppState, area: Rect) {
    let title = " Removable drives (↑/↓ select, Enter confirm, q quit, Esc back) ";
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);

    let items: Vec<ListItem> = if state.drives.is_empty() {
        vec![ListItem::new("No removable drives found.")]
    } else {
        state
            .drives
            .iter()
            .enumerate()
            .map(|(i, d)| {
                let prefix = if i == state.selected_drive_index { "▸ " } else { "  " };
                let mount = d.mountpoint.as_deref().unwrap_or("—");
                let line = format!(
                    "{} {}  {}  {}  mount: {}",
                    prefix,
                    d.path(),
                    d.size,
                    d.device_type,
                    mount
                );
                ListItem::new(line)
            })
            .collect()
    };

    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}

fn render_iso_select(frame: &mut Frame, state: &AppState, area: Rect) {
    let block = Block::default()
        .title(" ISO path (.iso or .img) — Enter to confirm, Esc back ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);

    let mut lines = vec![
        Line::from("Path: ".to_string() + state.iso_input.as_str() + "▌"),
        Line::from(""),
    ];
    if let Some(ref e) = state.iso_error {
        lines.push(Line::from(Span::styled(
            format!("Error: {}", e),
            Style::default().fg(Color::Red),
        )));
    }
    let p = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: true });
    frame.render_widget(p, area);
}

fn render_confirm(frame: &mut Frame, state: &AppState, area: Rect) {
    let device_path = state
        .selected_drive()
        .map(|d: &BlockDevice| d.path())
        .unwrap_or_else(|| "?".to_string());
    let block = Block::default()
        .title(" Confirm — type device path to continue ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);

    let warning = vec![
        Line::from(""),
        Line::from(Span::styled(
            "WARNING: This will ERASE all data on the selected device.",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(format!("Device: {}", device_path)),
        Line::from(format!("ISO: {}", state.iso_path.display())),
        Line::from(""),
        Line::from("Type the full device path to confirm: ".to_string() + state.confirmation_input.as_str() + "▌"),
    ];
    let p = Paragraph::new(warning)
        .block(block)
        .wrap(Wrap { trim: true });
    frame.render_widget(p, area);
}

fn render_flash_progress(frame: &mut Frame, state: &AppState, area: Rect) {
    let block = Block::default()
        .title(" Flashing (do not remove device) ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);

    let text = if state.flash_progress.is_empty() {
        "Starting dd…"
    } else {
        state.flash_progress.as_str()
    };
    let p = Paragraph::new(text)
        .block(block)
        .wrap(Wrap { trim: true });
    frame.render_widget(p, area);
}

fn render_result(frame: &mut Frame, state: &AppState, area: Rect) {
    let block = Block::default()
        .title(" Result — Esc back to list, q quit ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);

    let (msg, color): (&str, Color) = match &state.flash_done {
        Some(Ok(())) => ("Flash completed successfully.", Color::Green),
        Some(Err(e)) => (e.as_str(), Color::Red),
        None => ("Unknown state.", Color::Gray),
    };
    let p = Paragraph::new(Line::from(Span::styled(
        msg,
        Style::default().fg(color),
    )))
    .block(block)
    .wrap(Wrap { trim: true });
    frame.render_widget(p, area);
}
