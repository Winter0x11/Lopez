//! Application error types. No unwrap in production paths.

use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("must be run as root (sudo lopez)")]
    NotRoot,

    #[error("device {0} is mounted — unmount it and its partitions first")]
    DeviceMounted(String),

    #[error("ISO is larger than the target device ({iso_size} > {device_size})")]
    IsoTooLarge {
        iso_size: String,
        device_size: String,
    },

    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("lsblk failed: {0}")]
    Lsblk(String),

    #[error("invalid ISO path: {0}")]
    InvalidIso(String),

    #[error("flash failed: {0}")]
    FlashFailed(String),
}
