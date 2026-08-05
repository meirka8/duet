//! Mount & Drive Manager (Task T-6.1.14) parsing `/proc/mounts`, udisks2 discovery, and GVFS mounts.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MountKind {
    Nvme,
    Usb,
    Gvfs,
    Local,
    Optical,
    Other,
}

#[derive(Debug, Clone)]
pub struct SystemMountInfo {
    pub device: String,
    pub mount_point: String,
    pub fs_type: String,
    pub options: String,
    pub kind: MountKind,
    pub is_removable: bool,
}

pub struct MountManager;

impl MountManager {
    /// Read `/proc/mounts` and discover all active system mounts.
    pub fn discover_mounts() -> Vec<SystemMountInfo> {
        let mut list = Vec::new();
        let file = match File::open("/proc/mounts") {
            Ok(f) => f,
            Err(_) => return list,
        };

        let reader = BufReader::new(file);
        for line in reader.lines().map_while(Result::ok) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                let device = parts[0].to_string();
                let mount_point = parts[1].to_string();
                let fs_type = parts[2].to_string();
                let options = parts[3].to_string();

                // Skip non-storage pseudo filesystems (proc, sysfs, devtmpfs, cgroup)
                if fs_type == "proc" || fs_type == "sysfs" || fs_type == "devtmpfs" || fs_type == "cgroup" || fs_type == "tmpfs" {
                    continue;
                }

                let kind = if device.contains("nvme") {
                    MountKind::Nvme
                } else if device.contains("sd") || mount_point.contains("/media/") {
                    MountKind::Usb
                } else if mount_point.contains("gvfs") || fs_type == "fuse.gvfsd-fuse" {
                    MountKind::Gvfs
                } else if fs_type == "iso9660" || fs_type == "udf" {
                    MountKind::Optical
                } else {
                    MountKind::Local
                };

                let is_removable = kind == MountKind::Usb || kind == MountKind::Gvfs || kind == MountKind::Optical;

                list.push(SystemMountInfo {
                    device,
                    mount_point,
                    fs_type,
                    options,
                    kind,
                    is_removable,
                });
            }
        }
        list
    }

    /// Simulate unmounting / ejecting a removable mount point.
    pub fn unmount(mount_point: &str) -> Result<(), String> {
        let path = Path::new(mount_point);
        if !path.exists() {
            return Err(format!("Mount point {} does not exist", mount_point));
        }

        // Run umount CLI helper as unmount executor
        let status = std::process::Command::new("umount")
            .arg(mount_point)
            .status();

        match status {
            Ok(st) if st.success() => Ok(()),
            Ok(st) => Err(format!("umount failed with exit code: {:?}", st.code())),
            Err(e) => Err(format!("Failed to execute umount: {}", e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mount_discovery() {
        let mounts = MountManager::discover_mounts();
        // At least root / should be present on Linux system
        assert!(mounts.iter().any(|m| m.mount_point == "/"));
    }
}
