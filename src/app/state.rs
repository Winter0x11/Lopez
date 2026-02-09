//! Central app state. Structs for state, enums for screens.

use crate::disk::BlockDevice;
use crate::flash::FlashMessage;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::time::Instant;

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
    /// Last time drives were refreshed.
    pub last_drive_refresh: Instant,
    /// Resolved ISO path (validated).
    pub iso_path: PathBuf,
    /// Current line being typed on ISO screen.
    pub iso_input: String,
    /// Filtered ISO/IMG files matching the input.
    pub iso_file_matches: Vec<PathBuf>,
    /// Selected index in the file matches list.
    pub iso_file_selected_index: usize,
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
            last_drive_refresh: Instant::now(),
            iso_path: PathBuf::new(),
            iso_input: String::new(),
            iso_file_matches: Vec::new(),
            iso_file_selected_index: 0,
            iso_error: None,
            confirmation_input: String::new(),
            flash_progress: String::new(),
            flash_done: None,
            flash_rx: None,
        }
    }

    /// Refresh the drive list and preserve selection if possible.
    pub fn refresh_drives(&mut self, new_drives: Vec<BlockDevice>) {
        let old_selected_path = self
            .selected_drive()
            .map(|d| d.path());
        self.drives = new_drives;
        self.last_drive_refresh = Instant::now();

        // Try to preserve selection by device path
        if let Some(ref path) = old_selected_path {
            if let Some(new_idx) = self
                .drives
                .iter()
                .position(|d| d.path() == *path)
            {
                self.selected_drive_index = new_idx;
                return;
            }
        }

        // If old selection not found, clamp index to valid range
        if !self.drives.is_empty() {
            self.selected_drive_index = self.selected_drive_index.min(self.drives.len() - 1);
        } else {
            self.selected_drive_index = 0;
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

    pub fn select_prev_iso_file(&mut self) {
        if self.iso_file_matches.is_empty() {
            return;
        }
        self.iso_file_selected_index = self.iso_file_selected_index.saturating_sub(1);
    }

    pub fn select_next_iso_file(&mut self) {
        if self.iso_file_matches.is_empty() {
            return;
        }
        self.iso_file_selected_index = (self.iso_file_selected_index + 1).min(self.iso_file_matches.len() - 1);
    }

    pub fn selected_iso_file(&self) -> Option<&PathBuf> {
        self.iso_file_matches.get(self.iso_file_selected_index)
    }
}
