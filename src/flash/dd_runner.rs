//! Run dd to write ISO to device; stream progress via channel; cancellable.

use crate::errors::AppError;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub enum FlashMessage {
    Progress(String),
    Done(Result<(), AppError>),
}

/// Handle used to cancel a running flash job (kills the dd child).
#[derive(Clone)]
pub struct FlashHandle {
    cancel_flag: Arc<AtomicBool>,
    child: Arc<Mutex<Option<Child>>>,
}

impl FlashHandle {
    /// Request cancellation: sets the flag and SIGKILLs the dd process if running.
    pub fn cancel(&self) {
        self.cancel_flag.store(true, Ordering::SeqCst);
        if let Ok(mut guard) = self.child.lock() {
            if let Some(ref mut child) = *guard {
                let _ = child.kill();
            }
        }
    }

    pub fn is_cancel_requested(&self) -> bool {
        self.cancel_flag.load(Ordering::SeqCst)
    }
}

/// Extract bytes-copied from a dd progress line, e.g.
/// "1234567 bytes (1.2 MB, 1.2 MiB) copied, 0.5 s, 2.5 MB/s" -> Some(1234567).
/// Returns None for records/summary lines.
pub fn parse_progress_bytes(line: &str) -> Option<u64> {
    let first = line.split_whitespace().next()?;
    first.parse::<u64>().ok()
}

/// Spawn dd in a background thread and return a cancel handle.
/// Progress lines (and the final result) are sent through `tx`.
pub fn start_flash(iso_path: &str, device_path: &str, tx: Sender<FlashMessage>) -> FlashHandle {
    let cancel_flag = Arc::new(AtomicBool::new(false));
    let child_slot: Arc<Mutex<Option<Child>>> = Arc::new(Mutex::new(None));
    let handle = FlashHandle {
        cancel_flag: Arc::clone(&cancel_flag),
        child: Arc::clone(&child_slot),
    };

    let iso = iso_path.to_string();
    let device = device_path.to_string();

    std::thread::spawn(move || {
        run_dd(&iso, &device, tx, cancel_flag, child_slot);
    });

    handle
}

fn run_dd(
    iso_path: &str,
    device_path: &str,
    tx: Sender<FlashMessage>,
    cancel_flag: Arc<AtomicBool>,
    child_slot: Arc<Mutex<Option<Child>>>,
) {
    // If cancellation was requested before the child even started, abort.
    if cancel_flag.load(Ordering::SeqCst) {
        let _ = tx.send(FlashMessage::Done(Err(AppError::FlashFailed(
            "flash cancelled by user".to_string(),
        ))));
        return;
    }

    let args = [
        format!("if={}", iso_path),
        format!("of={}", device_path),
        "bs=4M".to_string(),
        "status=progress".to_string(),
        "conv=fsync".to_string(),
    ];

    let spawned = Command::new("dd")
        .args(&args)
        .stderr(Stdio::piped())
        .stdout(Stdio::null())
        .spawn();

    let mut child = match spawned {
        Ok(c) => c,
        Err(e) => {
            let _ = tx.send(FlashMessage::Done(Err(AppError::FlashFailed(e.to_string()))));
            return;
        }
    };

    // Take the stderr pipe first, then store the child so cancel() can always
    // reach it through the slot while we stream progress.
    let stderr_pipe = child.stderr.take();

    *child_slot.lock().expect("child slot poisoned") = Some(child);

    // Cancellation raced with startup: kill immediately.
    if cancel_flag.load(Ordering::SeqCst) {
        if let Ok(mut guard) = child_slot.lock() {
            if let Some(ref mut c) = *guard {
                let _ = c.kill();
            }
        }
    }

    // dd writes progress with \r line endings; read until \r or split on \n.
    if let Some(pipe) = stderr_pipe {
        let mut reader = BufReader::new(pipe);
        let mut buf: Vec<u8> = Vec::new();
        loop {
            buf.clear();
            let n = reader.read_until(b'\r', &mut buf).unwrap_or(0);
            if n == 0 {
                break;
            }
            let chunk = String::from_utf8_lossy(&buf);
            for part in chunk.split('\n') {
                let line = part.trim_matches(|c| c == '\r' || c == '\n');
                if !line.is_empty() && tx.send(FlashMessage::Progress(line.to_string())).is_err() {
                    break;
                }
            }
        }
    }

    // Reap: the child may still be in the slot (normal flow) or was killed via
    // the slot by cancel(); take it so cancel() can no longer touch it.
    let wait_result = match child_slot.lock().expect("child slot poisoned").take() {
        Some(mut child) => child.wait(),
        None => Err(std::io::Error::other("child already reaped")),
    };
    let cancelled = cancel_flag.load(Ordering::SeqCst);

    let result = match wait_result {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => {
            if cancelled {
                Err(AppError::FlashFailed("flash cancelled by user".to_string()))
            } else {
                Err(AppError::FlashFailed(format!("dd exited with: {}", status)))
            }
        }
        Err(e) => {
            if cancelled {
                Err(AppError::FlashFailed("flash cancelled by user".to_string()))
            } else {
                Err(AppError::FlashFailed(e.to_string()))
            }
        }
    };

    let _ = tx.send(FlashMessage::Done(result));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bytes_copied_line() {
        let line = "1234567 bytes (1.2 MB, 1.2 MiB) copied, 0.5 s, 2.5 MB/s";
        assert_eq!(parse_progress_bytes(line), Some(1234567));
    }

    #[test]
    fn rejects_record_lines() {
        assert_eq!(parse_progress_bytes("64+1 records in"), None);
        assert_eq!(parse_progress_bytes(""), None);
    }

    /// Serializes tests that mutate PATH (process-global, tests run concurrently).
    static PATH_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn fake_dd(dir: &std::path::Path, script: &str) -> String {
        use std::fs;
        let bin = dir.join("bin");
        fs::create_dir_all(&bin).unwrap();
        let fake = bin.join("dd");
        fs::write(&fake, script).unwrap();
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).unwrap();
        }
        let old_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{}", bin.display(), old_path));
        old_path
    }

    /// Integration test: flashes through a fake `dd` on PATH (sleeps and emits
    /// progress), then verifies the cancel path kills it and reports result.
    #[test]
    fn flash_cancel_reports_cancelled() {
        let _guard = PATH_LOCK.lock().unwrap();
        use std::fs;
        let dir = std::env::temp_dir().join("lopez_flash_test");
        let old_path = fake_dd(
            &dir,
            "#!/bin/sh\nfor i in 1 2 3 4 5; do printf '%s bytes (100 KB) copied, 1 s, 1.0 MB/s\\r' $((i*100000)) >&2; sleep 0.3; done\n",
        );

        let (tx, rx) = std::sync::mpsc::channel();
        let handle = start_flash("/dev/null", "/dev/null", tx);

        // Wait for the first progress line to know dd is streaming.
        let mut got_progress = false;
        for _ in 0..20 {
            match rx.recv_timeout(std::time::Duration::from_millis(500)) {
                Ok(FlashMessage::Progress(_)) => {
                    got_progress = true;
                    break;
                }
                Ok(FlashMessage::Done(r)) => panic!("dd finished before cancel: {:?}", r),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        assert!(got_progress, "no progress from fake dd");
        handle.cancel();

        let done = (|| {
            for _ in 0..40 {
                if let Ok(m) = rx.recv_timeout(std::time::Duration::from_millis(200)) {
                    if let FlashMessage::Done(r) = m {
                        return r;
                    }
                }
            }
            panic!("no Done message after cancel");
        })();
        match done {
            Err(AppError::FlashFailed(msg)) => assert!(msg.contains("cancelled")),
            other => panic!("expected cancelled error, got {:?}", other),
        }

        std::env::set_var("PATH", old_path);
        let _ = fs::remove_dir_all(&dir);
    }

    /// Integration test: successful flash through the fake dd reports Ok.
    #[test]
    fn flash_success_reports_ok() {
        let _guard = PATH_LOCK.lock().unwrap();
        use std::fs;
        let dir = std::env::temp_dir().join("lopez_flash_test_ok");
        let old_path = fake_dd(
            &dir,
            "#!/bin/sh\nprintf '1000000 bytes (1.0 MB) copied, 0 s, 999 MB/s\\r' >&2\necho \"1+1 records in\" >&2\necho \"1+1 records out\" >&2\n",
        );

        let (tx, rx) = std::sync::mpsc::channel();
        start_flash("/dev/null", "/dev/null", tx);

        let done = (|| {
            for _ in 0..20 {
                if let Ok(m) = rx.recv_timeout(std::time::Duration::from_millis(200)) {
                    if let FlashMessage::Done(r) = m {
                        return r;
                    }
                }
            }
            panic!("no Done message");
        })();
        assert!(done.is_ok(), "expected success, got {:?}", done);

        std::env::set_var("PATH", old_path);
        let _ = fs::remove_dir_all(&dir);
    }
}
