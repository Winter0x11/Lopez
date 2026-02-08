//! Safety checks: root, mounted status, ISO path validation.

use crate::errors::AppError;
use std::fs;
use std::path::Path;

/// Ensure the process is running as root (Linux: effective UID 0).
pub fn ensure_root() -> Result<(), AppError> {
    let status = fs::read_to_string("/proc/self/status").map_err(AppError::Io)?;
    for line in status.lines() {
        if line.starts_with("Uid:") {
            let fields: Vec<&str> = line.split_whitespace().collect();
            // Uid: real effective saved setuid
            if fields.get(2).map(|s| *s == "0").unwrap_or(false) {
                return Ok(());
            }
            break;
        }
    }
    Err(AppError::NotRoot)
}

/// Return true if the block device is currently mounted.
pub fn is_mounted(device_path: &str) -> Result<bool, AppError> {
    let path = Path::new(device_path);
    if !path.exists() {
        return Ok(false);
    }
    let mounts = fs::read_to_string("/proc/mounts").map_err(AppError::Io)?;
    let canonical = path.canonicalize().map_err(AppError::Io)?;
    let dev = canonical.to_string_lossy();
    for line in mounts.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if let Some(device) = parts.first() {
            if *device == dev || *device == device_path {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

/// Validate ISO path: exists and has extension .iso or .img.
pub fn validate_iso_path(path: &Path) -> Result<(), AppError> {
    if path.as_os_str().is_empty() {
        return Err(AppError::InvalidIso("path is empty".to_string()));
    }
    let ext = path.extension().and_then(|e| e.to_str());
    let ok = ext.map(|e| e.eq_ignore_ascii_case("iso") || e.eq_ignore_ascii_case("img")).unwrap_or(false);
    if !ok {
        return Err(AppError::InvalidIso("extension must be .iso or .img".to_string()));
    }
    if !path.exists() {
        return Err(AppError::InvalidIso(format!("file not found: {}", path.display())));
    }
    if !path.is_file() {
        return Err(AppError::InvalidIso("path is not a file".to_string()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn validate_iso_rejects_empty_path() {
        assert!(validate_iso_path(Path::new("")).is_err());
    }

    #[test]
    fn validate_iso_rejects_wrong_extension() {
        assert!(validate_iso_path(Path::new("/nonexistent/file.txt")).is_err());
        assert!(validate_iso_path(Path::new("/nonexistent/file")).is_err());
    }

    #[test]
    fn validate_iso_accepts_valid_extension_when_file_exists() {
        let tmp = std::env::temp_dir();
        let iso = tmp.join("lopez_test_iso.iso");
        let img = tmp.join("lopez_test_img.img");
        std::fs::write(&iso, b"").unwrap();
        std::fs::write(&img, b"").unwrap();
        assert!(validate_iso_path(&iso).is_ok());
        assert!(validate_iso_path(&img).is_ok());
        let _ = std::fs::remove_file(&iso);
        let _ = std::fs::remove_file(&img);
    }
}
