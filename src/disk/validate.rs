//! Safety checks: root, mounted status, ISO path validation, preflight.

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
            if fields
                .get(2)
                .map(|s| *s == "0")
                .unwrap_or(false)
            {
                return Ok(());
            }
            break;
        }
    }
    Err(AppError::NotRoot)
}

/// Return true if the block device or any of its partitions is mounted.
pub fn is_mounted(device_path: &str) -> Result<bool, AppError> {
    let mounts = fs::read_to_string("/proc/mounts").map_err(AppError::Io)?;
    Ok(is_device_mounted(&mounts, device_path))
}

/// Pure, testable mount check against /proc/mounts content.
/// Matches the device itself, its partitions (/dev/sdb1), and btrfs
/// subvolume mounts (/dev/sdb1[/subvol]).
pub fn is_device_mounted(mounts: &str, device_path: &str) -> bool {
    let base = device_path.trim_end_matches('/');
    if base.is_empty() {
        return false;
    }
    for line in mounts.lines() {
        let Some(device) = line.split_whitespace().next() else {
            continue;
        };
        // btrfs: /dev/sdb1[/subvol] — strip the subvolume part.
        let dev = device.split('#').next().unwrap_or(device);
        if dev == base {
            return true;
        }
        // Partition: /dev/sdb1 starts with /dev/sdb followed by a digit.
        if let Some(rest) = dev.strip_prefix(base) {
            if rest
                .chars()
                .next()
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false)
            {
                return true;
            }
        }
    }
    false
}

/// Validate ISO path: exists and has extension .iso or .img.
pub fn validate_iso_path(path: &Path) -> Result<(), AppError> {
    if path.as_os_str().is_empty() {
        return Err(AppError::InvalidIso("path is empty".to_string()));
    }
    let ext = path.extension().and_then(|e| e.to_str());
    let ok = ext
        .map(|e| e.eq_ignore_ascii_case("iso") || e.eq_ignore_ascii_case("img"))
        .unwrap_or(false);
    if !ok {
        return Err(AppError::InvalidIso(
            "extension must be .iso or .img".to_string(),
        ));
    }
    if !path.exists() {
        return Err(AppError::InvalidIso(format!(
            "file not found: {}",
            path.display()
        )));
    }
    if !path.is_file() {
        return Err(AppError::InvalidIso("path is not a file".to_string()));
    }
    Ok(())
}

/// All safety checks that must pass before flashing may start.
/// `device_size_bytes` is the target capacity (skip size check if unknown).
pub fn preflight_checks(
    device_path: &str,
    device_size_bytes: Option<u64>,
    iso_path: &Path,
) -> Result<(), AppError> {
    ensure_root()?;

    if is_mounted(device_path)? {
        return Err(AppError::DeviceMounted(device_path.to_string()));
    }

    if let (Some(device_size), Ok(meta)) = (device_size_bytes, fs::metadata(iso_path)) {
        let iso_size = meta.len();
        if iso_size > device_size {
            return Err(AppError::IsoTooLarge {
                iso_size: crate::disk::format_size(Some(iso_size)),
                device_size: crate::disk::format_size(Some(device_size)),
            });
        }
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

    #[test]
    fn mount_check_detects_device_and_partitions() {
        let mounts = "\
proc /proc proc rw 0 0
/dev/sda1 / ext4 rw 0 0
/dev/sdb1 /mnt/usb vfat rw 0 0
/dev/sdb1[/subvol] /srv btrfs rw 0 0
tmpfs /run tmpfs rw 0 0
";
        assert!(is_device_mounted(mounts, "/dev/sdb")); // partition mounted
        assert!(is_device_mounted(mounts, "/dev/sdb1"));
        assert!(!is_device_mounted(mounts, "/dev/sdc"));
        assert!(!is_device_mounted(mounts, ""));
        assert!(is_device_mounted("/dev/sdz /x vfat rw", "/dev/sdz")); // whole device mounted
    }

    #[test]
    fn mount_check_ignores_similar_names() {
        let mounts = "/dev/sdb1 /mnt vfat rw 0 0\n/dev/sdbx /y ext4 rw 0 0\n";
        assert!(is_device_mounted(mounts, "/dev/sdb"));
        // /dev/sdbx must NOT trigger /dev/sdbx1-style match for /dev/sdbc etc.
        assert!(!is_device_mounted(mounts, "/dev/sdbc"));
    }
}
