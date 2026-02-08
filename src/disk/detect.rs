//! Detect removable block devices via lsblk -J.

use crate::errors::AppError;
use serde::Deserialize;
use std::process::Command;

/// One block device as returned by lsblk (top-level only for listing).
#[derive(Debug, Clone)]
pub struct BlockDevice {
    pub name: String,
    pub size: String,
    pub device_type: String,
    pub mountpoint: Option<String>,
    pub removable: bool,
}

impl BlockDevice {
    /// Device path e.g. /dev/sdb.
    pub fn path(&self) -> String {
        format!("/dev/{}", self.name)
    }
}

/// Raw lsblk JSON node. RM can be 0/1 or boolean depending on lsblk version.
#[derive(Debug, Deserialize)]
struct LsblkNode {
    name: String,
    size: Option<String>,
    #[serde(rename = "type")]
    device_type: Option<String>,
    mountpoint: Option<String>,
    #[serde(rename = "rm")]
    rm: Option<Removable>,
    #[serde(default)]
    children: Option<Vec<LsblkNode>>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Removable {
    Bool(bool),
    Int(u8),
    Str(String),
}

impl Removable {
    fn as_bool(&self) -> bool {
        match self {
            Removable::Bool(b) => *b,
            Removable::Int(n) => *n != 0,
            Removable::Str(s) => s == "1" || s.eq_ignore_ascii_case("true"),
        }
    }
}

#[derive(Debug, Deserialize)]
struct LsblkOutput {
    blockdevices: Vec<LsblkNode>,
}

/// Run lsblk -J -o NAME,SIZE,TYPE,MOUNTPOINT,RM and return only removable disks.
pub fn lsblk_removable_disks() -> Result<Vec<BlockDevice>, AppError> {
    let out = Command::new("lsblk")
        .args(["-J", "-o", "NAME,SIZE,TYPE,MOUNTPOINT,RM"])
        .output()
        .map_err(|e| AppError::Lsblk(e.to_string()))?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(AppError::Lsblk(stderr.into_owned()));
    }

    let parsed: LsblkOutput = serde_json::from_slice(&out.stdout)
        .map_err(|e| AppError::Lsblk(e.to_string()))?;

    let devices: Vec<BlockDevice> = parsed
        .blockdevices
        .into_iter()
        .filter_map(|n| node_to_removable_disk(n))
        .collect();

    Ok(devices)
}

fn node_to_removable_disk(node: LsblkNode) -> Option<BlockDevice> {
    let removable = node.rm.as_ref().map(|r| r.as_bool()).unwrap_or(false);
    let device_type = node.device_type.unwrap_or_else(|| "disk".to_string());

    if !removable || device_type != "disk" {
        return None;
    }

    Some(BlockDevice {
        name: node.name,
        size: node.size.unwrap_or_else(|| "—".to_string()),
        device_type,
        mountpoint: node.mountpoint.filter(|s| !s.is_empty()),
        removable,
    })
}
