//! Lopez: terminal-based ISO flasher for Linux (dd-powered).
//! Linux only, keyboard-only, safety first.

mod app;
mod disk;
mod errors;
mod flash;
mod ui;

use crate::app::{run_event_loop, AppState};
use crate::disk::{ensure_root, lsblk_removable_disks};
use crate::errors::AppError;
use clap::Parser;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "lopez", about = "Terminal-based ISO flasher for Linux (dd-powered)")]
struct Args {
    /// ISO or IMG file to flash (can also be set in TUI)
    #[arg(short, long)]
    iso: Option<PathBuf>,
}

fn main() -> Result<(), AppError> {
    let args = Args::parse();
    let drives = lsblk_removable_disks().unwrap_or_default();
    let mut state = AppState::new(drives);

    if ensure_root().is_err() {
        state = state.with_not_root();
    } else if let Some(iso) = args.iso {
        state = state.with_iso_from_cli(iso);
    }

    let mut stdout = io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen)?;
    enable_raw_mode().map_err(|e| AppError::Io(e.into()))?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).map_err(|e| AppError::Io(e.into()))?;

    let result = run_event_loop(&mut terminal, &mut state);

    disable_raw_mode().map_err(|e| AppError::Io(e.into()))?;
    crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor().map_err(|e| AppError::Io(e.into()))?;

    result
}
