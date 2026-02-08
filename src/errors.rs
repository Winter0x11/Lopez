//! Application error types. No unwrap in production paths.

use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("must be run as root")]
    NotRoot,

    #[error("device is mounted: {0}")]
    DeviceMounted(String),

    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("lsblk failed: {0}")]
    Lsblk(String),

    #[error("invalid ISO path: {0}")]
    InvalidIso(String),

    #[error("flash failed: {0}")]
    FlashFailed(String),
}
