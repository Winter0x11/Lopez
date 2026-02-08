//! TUI rendering only. No business logic; reads from AppState.

mod screens;

use crate::app::AppState;
use crate::errors::AppError;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;

pub fn draw(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &AppState,
) -> Result<(), AppError> {
    terminal.draw(|frame| {
        let area = frame.area();
        screens::render(frame, state, area);
    })?;
    Ok(())
}
