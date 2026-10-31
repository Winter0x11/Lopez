//! Detect removable block devices via lsblk -J -b.

use crate::errors::AppError;
use serde::Deserialize;
use std::process::Command;

/// One block device as returned by lsblk (top-level disks only).
#[derive(Debug, Clone)]
pub struct BlockDevice {
    pub name: String,
    /// Size in bytes (lsblk -b), if reported.
    pub size_bytes: Option<u64>,
    pub model: Option<String>,
    /// Mountpoints of the device and its partitions.
    pub mountpoints: Vec<String>,
}

impl BlockDevice {
    /// Device path e.g. /dev/sdb.
    pub fn path(&self) -> String {
        format!("/dev/{}", self.name)
    }
}

/// Raw lsblk JSON node.
#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct LsblkNode {
    name: String,
    size: Option<SizeValue>,
    #[serde(rename = "type")]
    device_type: Option<String>,
    mountpoint: Option<Mountpoints>,
    #[serde(rename = "rm")]
    rm: Option<BoolValue>,
    model: Option<String>,
    children: Vec<LsblkNode>,
}

/// lsblk emits RM as 0/1 (number) or true/false (bool) depending on version.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum BoolValue {
    Bool(bool),
    Int(u8),
    Str(String),
}

impl BoolValue {
    fn as_bool(&self) -> bool {
        match self {
            BoolValue::Bool(b) => *b,
            BoolValue::Int(n) => *n != 0,
            BoolValue::Str(s) => s == "1" || s.eq_ignore_ascii_case("true"),
        }
    }
}

/// lsblk emits SIZE as a JSON number (with -b) or a human string otherwise.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum SizeValue {
    Num(u64),
    Str(String),
}

impl SizeValue {
    fn as_bytes(&self) -> Option<u64> {
        match self {
            SizeValue::Num(n) => Some(*n),
            SizeValue::Str(s) => s.trim().parse::<u64>().ok(),
        }
    }
}

/// MOUNTPOINT may be a string, null, or an array of strings (multiple mounts).
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Mountpoints {
    One(String),
    Many(Vec<String>),
}

impl Mountpoints {
    fn collect_into(&self, out: &mut Vec<String>) {
        match self {
            Mountpoints::One(s) => out.push(s.clone()),
            Mountpoints::Many(v) => out.extend(v.iter().cloned()),
        }
    }
}

#[derive(Debug, Deserialize)]
struct LsblkOutput {
    #[serde(default)]
    blockdevices: Vec<LsblkNode>,
}

/// Run lsblk -J -b and return only removable disks.
pub fn lsblk_removable_disks() -> Result<Vec<BlockDevice>, AppError> {
    let out = Command::new("lsblk")
        .args(["-J", "-b", "-o", "NAME,SIZE,TYPE,MOUNTPOINT,RM,MODEL"])
        .output()
        .map_err(|e| AppError::Lsblk(e.to_string()))?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(AppError::Lsblk(stderr.into_owned()));
    }

    parse_lsblk_output(&out.stdout)
}

/// Parse lsblk JSON output into a list of removable disks. Public for tests.
pub fn parse_lsblk_output(data: &[u8]) -> Result<Vec<BlockDevice>, AppError> {
    let parsed: LsblkOutput =
        serde_json::from_slice(data).map_err(|e| AppError::Lsblk(e.to_string()))?;

    let devices: Vec<BlockDevice> = parsed
        .blockdevices
        .into_iter()
        .filter_map(node_to_removable_disk)
        .collect();

    Ok(devices)
}

fn node_to_removable_disk(node: LsblkNode) -> Option<BlockDevice> {
    let removable = node.rm.as_ref().map(|r| r.as_bool()).unwrap_or(false);
    let device_type = node
        .device_type
        .clone()
        .unwrap_or_else(|| "disk".to_string());

    if !removable || device_type != "disk" {
        return None;
    }

    // Collect mountpoints from the disk itself and all children (partitions).
    let mut mountpoints = Vec::new();
    collect_mountpoints(&node, &mut mountpoints);
    mountpoints.retain(|s| !s.is_empty());

    Some(BlockDevice {
        name: node.name,
        size_bytes: node.size.as_ref().and_then(|s| s.as_bytes()),
        model: node.model.map(|m| m.trim().to_string()).filter(|m| !m.is_empty()),
        mountpoints,
    })
}

fn collect_mountpoints(node: &LsblkNode, out: &mut Vec<String>) {
    if let Some(m) = &node.mountpoint {
        m.collect_into(out);
    }
    for child in &node.children {
        collect_mountpoints(child, out);
    }
}

/// Human-readable size, e.g. 15_000_000_000 -> "15.00 GB".
pub fn format_size(bytes: Option<u64>) -> String {
    match bytes {
        None => "—".to_string(),
        Some(b) => {
            const UNITS: [&str; 6] = ["B", "KiB", "MiB", "GiB", "TiB", "PiB"];
            let mut value = b as f64;
            let mut unit = 0;
            while value >= 1024.0 && unit < UNITS.len() - 1 {
                value /= 1024.0;
                unit += 1;
            }
            if unit == 0 {
                format!("{} B", b)
            } else {
                format!("{:.2} {}", value, UNITS[unit])
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_removable_disk_with_partition_mount() {
        let json = r#"{
            "blockdevices": [
                {"name": "sda", "size": 1000204886016, "type": "disk", "mountpoint": null, "rm": false,
                 "children": [{"name": "sda1", "size": 512, "type": "part", "mountpoint": "/boot/efi", "rm": false}]},
                {"name": "sdb", "size": 15000000000000, "type": "disk", "mountpoint": null, "rm": true, "model": "SanDisk Ultra",
                 "children": [
                    {"name": "sdb1", "size": 15000, "type": "part", "mountpoint": ["/mnt/usb", "/media/usb2"], "rm": true},
                    {"name": "sdb2", "size": 1500, "type": "part", "mountpoint": null, "rm": true}
                 ]},
                {"name": "sr0", "size": 1073741312, "type": "rom", "mountpoint": null, "rm": true}
            ]
        }"#;
        let disks = parse_lsblk_output(json.as_bytes()).expect("parse");
        assert_eq!(disks.len(), 1);
        let d = &disks[0];
        assert_eq!(d.name, "sdb");
        assert_eq!(d.size_bytes, Some(15000000000000));
        assert_eq!(d.model.as_deref(), Some("SanDisk Ultra"));
        assert_eq!(d.mountpoints, vec!["/mnt/usb", "/media/usb2"]);
        assert_eq!(d.path(), "/dev/sdb");
    }

    #[test]
    fn parses_string_rm_flag() {
        let json = r#"{
            "blockdevices": [
                {"name": "mmcblk0", "size": null, "type": "disk", "rm": "1"}
            ]
        }"#;
        let disks = parse_lsblk_output(json.as_bytes()).expect("parse");
        assert_eq!(disks.len(), 1);
        assert_eq!(disks[0].size_bytes, None);
    }

    #[test]
    fn empty_and_malformed_inputs() {
        assert!(parse_lsblk_output(br#"{"blockdevices": []}"#).unwrap().is_empty());
        assert!(parse_lsblk_output(b"not json").is_err());
    }

    #[test]
    fn format_size_units() {
        assert_eq!(format_size(None), "—");
        assert_eq!(format_size(Some(0)), "0 B");
        assert_eq!(format_size(Some(512)), "512 B");
        assert_eq!(format_size(Some(5 * 1024 * 1024)), "5.00 MiB");
        assert_eq!(format_size(Some(1024)), "1.00 KiB");
    }
}
