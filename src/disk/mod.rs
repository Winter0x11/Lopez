//! Disk detection and validation. System operations live here, not in UI.

mod detect;
mod validate;

pub use detect::{lsblk_removable_disks, BlockDevice};
pub use validate::{ensure_root, is_mounted, validate_iso_path};
