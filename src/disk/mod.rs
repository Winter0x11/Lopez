//! Disk detection and validation. System operations live here, not in UI.

mod detect;
mod file_finder;
mod validate;

pub use detect::{format_size, lsblk_removable_disks, BlockDevice};
pub use file_finder::find_iso_files;
pub use validate::{ensure_root, preflight_checks, validate_iso_path};
