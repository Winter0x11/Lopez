//! Event loop: crossterm input, delegate to app state and UI.

use crate::app::state::{AppState, Screen};
use crate::disk::{find_iso_files, lsblk_removable_disks, preflight_checks, validate_iso_path};
use crate::errors::AppError;
use crate::flash::{parse_progress_bytes, start_flash, FlashMessage};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

pub fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut AppState,
) -> Result<(), AppError> {
    const DRIVE_REFRESH_INTERVAL: Duration = Duration::from_secs(1);

    loop {
        if event::poll(Duration::from_millis(100)).map_err(AppError::Io)? {
            if let Event::Key(key) = event::read().map_err(AppError::Io)? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                if handle_key(state, key) {
                    break;
                }
            }
        }

        // Refresh drives periodically when on DriveList screen
        if state.screen == Screen::DriveList
            && state.last_drive_refresh.elapsed() >= DRIVE_REFRESH_INTERVAL
        {
            if let Ok(new_drives) = lsblk_removable_disks() {
                state.refresh_drives(new_drives);
            }
        }

        // Drain flash messages
        while let Some(rx) = state.flash_rx.as_ref() {
            match rx.try_recv() {
                Ok(FlashMessage::Progress(line)) => {
                    if let Some(n) = parse_progress_bytes(&line) {
                        state.flash_bytes = Some(n);
                    }
                    state.flash_progress = line;
                }
                Ok(FlashMessage::Done(result)) => {
                    state.flash_done = Some(result.map_err(|e| e.to_string()));
                    state.flash_rx = None;
                    state.flash_handle = None;
                    state.screen = Screen::Result;
                    break;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    state.flash_rx = None;
                    break;
                }
            }
        }

        crate::ui::draw(terminal, state)?;
    }
    Ok(())
}

fn ctrl_q(key: &KeyEvent) -> bool {
    key.code == KeyCode::Char('q') && key.modifiers.contains(KeyModifiers::CONTROL)
}

fn fn_cancel_flash(state: &mut AppState) {
    if let Some(handle) = state.flash_handle.as_ref() {
        if handle.is_cancel_requested() {
            return;
        }
        handle.cancel();
    }
    state.flash_cancelled = true;
}

fn handle_key(state: &mut AppState, key: KeyEvent) -> bool {
    match state.screen {
        Screen::NotRoot => {
            if key.code == KeyCode::Char('q') || key.code == KeyCode::Esc {
                return true;
            }
        }
        Screen::DriveList => {
            if key.code == KeyCode::Char('q') || key.code == KeyCode::Esc {
                return true;
            }
            if key.code == KeyCode::Up {
                state.select_prev_drive();
            }
            if key.code == KeyCode::Down {
                state.select_next_drive();
            }
            if key.code == KeyCode::Enter && state.selected_drive().is_some() {
                state.screen = Screen::IsoSelect;
                state.iso_error = None;
                // Initialize file search
                state.iso_file_matches = find_iso_files(&state.iso_input);
                state.iso_file_selected_index = 0;
            }
        }
        Screen::IsoSelect => {
            if ctrl_q(&key) {
                return true;
            }
            if key.code == KeyCode::Esc {
                state.screen = Screen::DriveList;
                state.iso_error = None;
                state.iso_file_matches.clear();
                return false;
            }
            if key.code == KeyCode::Up {
                state.select_prev_iso_file();
                // Update input when navigating
                if let Some(selected) = state.selected_iso_file() {
                    state.iso_input = selected.to_string_lossy().into_owned();
                    state.iso_error = None;
                }
                return false;
            }
            if key.code == KeyCode::Down {
                state.select_next_iso_file();
                // Update input when navigating
                if let Some(selected) = state.selected_iso_file() {
                    state.iso_input = selected.to_string_lossy().into_owned();
                    state.iso_error = None;
                }
                return false;
            }
            if key.code == KeyCode::Enter {
                // Try selected file first, then typed input
                let path = if let Some(selected) = state.selected_iso_file() {
                    selected.clone()
                } else {
                    PathBuf::from(state.iso_input.trim())
                };
                match validate_iso_path(&path) {
                    Ok(()) => {
                        state.iso_path = path;
                        state.iso_size = std::fs::metadata(&state.iso_path).ok().map(|m| m.len());
                        state.iso_input = state.iso_path.to_string_lossy().into_owned();
                        state.iso_error = None;
                        state.iso_file_matches.clear();
                        state.screen = Screen::Confirm;
                        state.confirmation_input.clear();
                        state.confirm_error = None;
                    }
                    Err(e) => {
                        state.iso_error = Some(e.to_string());
                    }
                }
                return false;
            }
            if key.code == KeyCode::Tab {
                // Tab to select from list
                if let Some(selected) = state.selected_iso_file() {
                    state.iso_input = selected.to_string_lossy().into_owned();
                    state.iso_error = None;
                }
                return false;
            }
            if key.code == KeyCode::Backspace {
                state.iso_input.pop();
                state.iso_error = None;
                state.iso_file_selected_index = 0;
                // Update file matches
                state.iso_file_matches = find_iso_files(&state.iso_input);
            }
            if let KeyCode::Char(c) = key.code {
                state.iso_input.push(c);
                state.iso_error = None;
                state.iso_file_selected_index = 0;
                // Update file matches
                state.iso_file_matches = find_iso_files(&state.iso_input);
            }
        }
        Screen::Confirm => {
            if ctrl_q(&key) {
                return true;
            }
            if key.code == KeyCode::Esc {
                state.screen = Screen::IsoSelect;
                state.confirmation_input.clear();
                state.confirm_error = None;
                return false;
            }
            if key.code == KeyCode::Enter {
                if let Some(drive) = state.selected_drive() {
                    let device_path = drive.path();
                    let typed = state.confirmation_input.trim();

                    if typed != device_path {
                        state.confirm_error = Some(format!(
                            "Type the device path exactly to confirm: {}",
                            device_path
                        ));
                        state.confirmation_input.clear();
                        return false;
                    }

                    let iso_path = state.iso_path.clone();
                    if let Err(e) = preflight_checks(
                        &device_path,
                        drive.size_bytes,
                        &iso_path,
                    ) {
                        state.confirm_error = Some(e.to_string());
                        return false;
                    }

                    let (tx, rx) = mpsc::channel();
                    state.flash_rx = Some(rx);
                    state.flash_progress.clear();
                    state.flash_bytes = None;
                    state.flash_cancelled = false;
                    state.flash_done = None;
                    state.confirm_error = None;
                    state.screen = Screen::FlashProgress;
                    let handle = start_flash(
                        iso_path.to_string_lossy().as_ref(),
                        &device_path,
                        tx,
                    );
                    state.flash_handle = Some(handle);                }
                return false;
            }
            if key.code == KeyCode::Backspace {
                state.confirmation_input.pop();
                state.confirm_error = None;
            }
            if let KeyCode::Char(c) = key.code {
                state.confirmation_input.push(c);
                state.confirm_error = None;
            }
        }
        Screen::FlashProgress => {
            // Do not allow quitting mid-flash; require explicit cancellation.
            if key.code == KeyCode::Esc {
                fn_cancel_flash(state);
            }
        }
        Screen::Result => {
            if key.code == KeyCode::Char('q') {
                return true;
            }
            if key.code == KeyCode::Esc {
                state.screen = Screen::DriveList;
                state.flash_done = None;
                state.flash_progress.clear();
                state.flash_bytes = None;
                state.flash_cancelled = false;
            }
        }
    }
    false
}
