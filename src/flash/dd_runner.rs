//! Run dd to write ISO to device; stream progress via channel.

use crate::errors::AppError;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::mpsc::Sender;

#[derive(Debug)]
pub enum FlashMessage {
    Progress(String),
    Done(Result<(), AppError>),
}

/// Run dd in the current thread. Call from a spawned thread; send progress lines and final result.
pub fn run_dd(iso_path: &str, device_path: &str, tx: Sender<FlashMessage>) {
    let args = [
        format!("if={}", iso_path),
        format!("of={}", device_path),
        "bs=4M".to_string(),
        "status=progress".to_string(),
        "conv=fsync".to_string(),
    ];

    let mut child = match Command::new("dd")
        .args(&args)
        .stderr(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            let _ = tx.send(FlashMessage::Done(Err(AppError::FlashFailed(e.to_string()))));
            return;
        }
    };

    let stderr = match child.stderr.take() {
        Some(s) => s,
        None => {
            let _ = tx.send(FlashMessage::Done(Err(AppError::FlashFailed(
                "dd stderr not captured".to_string(),
            ))));
            return;
        }
    };

    let reader = BufReader::new(stderr);
    for line in reader.lines() {
        let line = line.unwrap_or_default();
        if tx.send(FlashMessage::Progress(line)).is_err() {
            break;
        }
    }

    let result = match child.wait() {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => Err(AppError::FlashFailed(format!(
            "dd exited with: {}",
            status
        ))),
        Err(e) => Err(AppError::FlashFailed(e.to_string())),
    };
    let _ = tx.send(FlashMessage::Done(result));
}
