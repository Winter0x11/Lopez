//! Central app state. Structs for state, enums for screens.

use crate::disk::BlockDevice;
use crate::flash::FlashMessage;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    NotRoot,
    DriveList,
    IsoSelect,
    Confirm,
    FlashProgress,
    Result,
}

pub struct AppState {
    pub screen: Screen,
    pub drives: Vec<BlockDevice>,
    pub selected_drive_index: usize,
    /// Resolved ISO path (validated).
    pub iso_path: PathBuf,
    /// Current line being typed on ISO screen.
    pub iso_input: String,
    /// Validation error to show on ISO screen.
    pub iso_error: Option<String>,
    /// Confirmation input: user must type device path.
    pub confirmation_input: String,
    /// Last progress line from dd (stderr).
    pub flash_progress: String,
    /// Set when flash thread finishes.
    pub flash_done: Option<Result<(), String>>,
    /// Receiver for flash thread messages; consumed in event loop.
    pub flash_rx: Option<Receiver<FlashMessage>>,
}

impl AppState {
    pub fn new(drives: Vec<BlockDevice>) -> Self {
        let selected_drive_index = if drives.is_empty() { 0 } else { 0 };
        Self {
            screen: Screen::DriveList,
            drives,
            selected_drive_index,
            iso_path: PathBuf::new(),
            iso_input: String::new(),
            iso_error: None,
            confirmation_input: String::new(),
            flash_progress: String::new(),
            flash_done: None,
            flash_rx: None,
        }
    }

    pub fn with_not_root(mut self) -> Self {
        self.screen = Screen::NotRoot;
        self
    }

    pub fn with_iso_from_cli(mut self, path: PathBuf) -> Self {
        self.iso_path = path.clone();
        self.iso_input = path.to_string_lossy().into_owned();
        self
    }

    pub fn selected_drive(&self) -> Option<&BlockDevice> {
        self.drives.get(self.selected_drive_index)
    }

    pub fn select_prev_drive(&mut self) {
        if self.drives.is_empty() {
            return;
        }
        self.selected_drive_index = self.selected_drive_index.saturating_sub(1);
    }

    pub fn select_next_drive(&mut self) {
        if self.drives.is_empty() {
            return;
        }
        self.selected_drive_index = (self.selected_drive_index + 1).min(self.drives.len() - 1);
    }
}
