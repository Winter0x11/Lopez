//! Event loop: crossterm input, delegate to app state and UI.

use crate::app::state::{AppState, Screen};
use crate::disk::{ensure_root, is_mounted, validate_iso_path};
use crate::errors::AppError;
use crate::flash::{run_dd, FlashMessage};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use std::sync::mpsc;
use std::thread;

pub fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut AppState,
) -> Result<(), AppError> {
    loop {
        if event::poll(std::time::Duration::from_millis(100))
            .map_err(|e| AppError::Io(e.into()))?
        {
            if let Event::Key(key) = event::read().map_err(|e| AppError::Io(e.into()))? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                if handle_key(state, key.code) {
                    break;
                }
            }
        }

        while let Some(rx) = state.flash_rx.as_ref() {
            match rx.try_recv() {
                Ok(FlashMessage::Progress(line)) => state.flash_progress = line,
                Ok(FlashMessage::Done(result)) => {
                    state.flash_done = Some(result.map_err(|e| e.to_string()));
                    state.flash_rx = None;
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

fn handle_key(state: &mut AppState, code: KeyCode) -> bool {
    match state.screen {
        Screen::NotRoot => {
            if code == KeyCode::Char('q') || code == KeyCode::Esc {
                return true;
            }
        }
        Screen::DriveList => {
            if code == KeyCode::Char('q') {
                return true;
            }
            if code == KeyCode::Esc {
                return true;
            }
            if code == KeyCode::Up {
                state.select_prev_drive();
            }
            if code == KeyCode::Down {
                state.select_next_drive();
            }
            if code == KeyCode::Enter && state.selected_drive().is_some() {
                state.screen = Screen::IsoSelect;
                state.iso_error = None;
            }
        }
        Screen::IsoSelect => {
            if code == KeyCode::Char('q') {
                return true;
            }
            if code == KeyCode::Esc {
                state.screen = Screen::DriveList;
                state.iso_error = None;
                return false;
            }
            if code == KeyCode::Enter {
                let path = std::path::PathBuf::from(state.iso_input.trim());
                match validate_iso_path(&path) {
                    Ok(()) => {
                        state.iso_path = path;
                        state.iso_error = None;
                        state.screen = Screen::Confirm;
                        state.confirmation_input.clear();
                    }
                    Err(e) => {
                        state.iso_error = Some(e.to_string());
                    }
                }
                return false;
            }
            if code == KeyCode::Backspace {
                state.iso_input.pop();
                state.iso_error = None;
            }
            if let KeyCode::Char(c) = code {
                state.iso_input.push(c);
                state.iso_error = None;
            }
        }
        Screen::Confirm => {
            if code == KeyCode::Char('q') {
                return true;
            }
            if code == KeyCode::Esc {
                state.screen = Screen::IsoSelect;
                state.confirmation_input.clear();
                return false;
            }
            if code == KeyCode::Enter {
                if let Some(drive) = state.selected_drive() {
                    let device_path = drive.path();
                    let typed = state.confirmation_input.trim();
                    if typed != device_path {
                        state.flash_done = Some(Err(format!(
                            "Type the full device path to confirm: {}",
                            device_path
                        )));
                        state.screen = Screen::Result;
                        return false;
                    }
                    if let Err(_) = ensure_root() {
                        state.flash_done = Some(Err("Must be run as root to flash.".to_string()));
                        state.screen = Screen::Result;
                        return false;
                    }
                    if let Ok(true) = is_mounted(&device_path) {
                        state.flash_done = Some(Err(format!(
                            "Device {} is mounted. Unmount it first.",
                            device_path
                        )));
                        state.screen = Screen::Result;
                        return false;
                    }
                    let iso_path = state.iso_path.clone();
                    let (tx, rx) = mpsc::channel();
                    state.flash_rx = Some(rx);
                    state.flash_progress.clear();
                    state.flash_done = None;
                    state.screen = Screen::FlashProgress;
                    thread::spawn(move || {
                        run_dd(
                            iso_path.to_string_lossy().as_ref(),
                            &device_path,
                            tx,
                        );
                    });
                }
                return false;
            }
            if code == KeyCode::Backspace {
                state.confirmation_input.pop();
            }
            if let KeyCode::Char(c) = code {
                state.confirmation_input.push(c);
            }
        }
        Screen::FlashProgress => {
            if code == KeyCode::Char('q') {
                return true;
            }
        }
        Screen::Result => {
            if code == KeyCode::Char('q') {
                return true;
            }
            if code == KeyCode::Esc {
                state.screen = Screen::DriveList;
                state.flash_done = None;
                state.flash_progress.clear();
            }
        }
    }
    false
}
