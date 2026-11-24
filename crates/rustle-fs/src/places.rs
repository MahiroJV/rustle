//! "Places": the user's standard folders and the drives on this computer.
//! Used by the Home page and the sidebar.

use std::path::{Path, PathBuf};

/// One of the user's standard folders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    pub label: String,
    pub path: PathBuf,
}

/// A mounted drive / partition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Drive {
    pub label: String,
    pub mount: PathBuf,
    pub free: u64,
    pub total: u64,
    pub removable: bool,
}

/// Home first, then Desktop, Downloads, Documents, Pictures, Music, Videos —
/// only the ones that exist on this computer.
pub fn user_folders() -> Vec<Place> {
    let candidates = [
        ("Home", dirs::home_dir()),
        ("Desktop", dirs::desktop_dir()),
        ("Downloads", dirs::download_dir()),
        ("Documents", dirs::document_dir()),
        ("Pictures", dirs::picture_dir()),
        ("Music", dirs::audio_dir()),
        ("Videos", dirs::video_dir()),
    ];
    let home = dirs::home_dir();
    candidates
        .into_iter()
        .filter_map(|(label, path)| Some((label, path?)))
        // Some systems set e.g. Desktop = Home when it isn't configured.
        .filter(|(label, path)| *label == "Home" || Some(path) != home.as_ref())
        .filter(|(_, path)| path.is_dir())
        .map(|(label, path)| Place {
            label: label.into(),
            path,
        })
        .collect()
}

/// A mount as the system reports it, before filtering.
#[derive(Debug, Clone)]
pub struct RawMount {
    pub device: String,
    pub fs_type: String,
    pub mount: PathBuf,
    pub total: u64,
    pub free: u64,
    pub removable: bool,
}

/// The drives worth showing. Reading the system's list can be a little slow,
/// so call this from a background thread.
pub fn drives() -> Vec<Drive> {
    let Ok(disks) = sysinfo::Disks::new_with_refreshed_list() else {
        return Vec::new();
    };
    let raw = disks
        .iter()
        .map(|d| RawMount {
            device: d.name().to_string_lossy().into_owned(),
            fs_type: d.file_system().to_string_lossy().into_owned(),
            mount: d.mount_point().to_path_buf(),
            total: d.total_space(),
            free: d.available_space(),
            removable: d.is_removable(),
        })
        .collect();
    pick_drives(raw, &cloud_mounts_dir())
}

/// Where cloud accounts get mounted later (Phase 5). They have their own
/// section, so they're not listed as drives.
pub fn cloud_mounts_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("rustle")
        .join("mounts")
}

/// Filter and tidy the raw mount list:
/// - drop system/virtual mounts (`is_real_fs`) and empty ones,
/// - drop anything under `skip_under` (cloud mounts),
/// - one entry per device: a partition mounted in several places (common
///   with btrfs subvolumes) is shown once, at its shortest mount path.
pub fn pick_drives(raw: Vec<RawMount>, skip_under: &Path) -> Vec<Drive> {
    let mut kept: Vec<RawMount> = Vec::new();
    for m in raw {
        if !is_real_fs(&m.fs_type, &m.mount) || m.total == 0 || m.mount.starts_with(skip_under) {
            continue;
        }
        match kept.iter_mut().find(|k| k.device == m.device) {
            Some(existing) if m.mount.as_os_str().len() < existing.mount.as_os_str().len() => {
                *existing = m;
            }
            Some(_) => {}
            None => kept.push(m),
        }
    }
    kept.sort_by(|a, b| a.mount.cmp(&b.mount));
    kept.into_iter()
        .map(|m| Drive {
            label: drive_label(&m.mount),
            mount: m.mount,
            free: m.free,
            total: m.total,
            removable: m.removable,
        })
        .collect()
}

/// Is this a real, user-facing filesystem (not a virtual/system one)?
pub fn is_real_fs(fs_type: &str, mount: &Path) -> bool {
    const VIRTUAL: [&str; 11] = [
        "tmpfs", "devtmpfs", "proc", "sysfs", "overlay", "squashfs", "efivarfs", "autofs", "ramfs",
        "cgroup2", "devpts",
    ];
    if VIRTUAL.contains(&fs_type) {
        return false;
    }
    // FUSE mounts of programs (AppImages, portals…) are "fuse.<name>";
    // "fuseblk" is a real disk (e.g. NTFS) and is kept.
    if fs_type.starts_with("fuse.") {
        return false;
    }
    const SYSTEM_DIRS: [&str; 9] = [
        "/boot", "/efi", "/snap", "/proc", "/sys", "/dev", "/tmp", "/var", "/etc",
    ];
    if SYSTEM_DIRS.iter().any(|d| mount.starts_with(d)) {
        return false;
    }
    // /run is runtime stuff, except /run/media where USB sticks appear.
    !(mount.starts_with("/run") && !mount.starts_with("/run/media"))
}

/// "/" → "System"; other mounts → their folder name ("/mnt/universal" → "universal").
fn drive_label(mount: &Path) -> String {
    match mount.file_name() {
        Some(name) => name.to_string_lossy().into_owned(),
        None if mount == Path::new("/") => "System".into(),
        None => mount.display().to_string(), // e.g. "C:\"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(device: &str, fs: &str, mount: &str) -> RawMount {
        RawMount {
            device: device.into(),
            fs_type: fs.into(),
            mount: mount.into(),
            total: 100,
            free: 40,
            removable: false,
        }
    }

    #[test]
    fn filters_pseudo() {
        assert!(is_real_fs("ext4", Path::new("/")));
        assert!(is_real_fs("vfat", Path::new("/run/media/me/USB")));
        assert!(is_real_fs("fuseblk", Path::new("/mnt/universal")));
        assert!(!is_real_fs("tmpfs", Path::new("/tmp")));
        assert!(!is_real_fs("vfat", Path::new("/boot/efi")));
        assert!(!is_real_fs("squashfs", Path::new("/snap/x")));
        assert!(!is_real_fs(
            "fuse.SomeApp.AppImage",
            Path::new("/tmp/.mount_x")
        ));
        assert!(!is_real_fs("ext4", Path::new("/run/credentials/x")));
    }

    /// The mount list of the machine Rustle is developed on (Nov 2026).
    #[test]
    fn real_world_mount_list() {
        let list = vec![
            raw("run", "tmpfs", "/run"),
            raw("/dev/nvme0n1p6", "ext4", "/"),
            raw("systemd-1", "autofs", "/mnt/universal"),
            raw("tmpfs", "tmpfs", "/etc/pacman.d/gnupg"),
            raw("/dev/nvme0n1p1", "vfat", "/boot/efi"),
            raw("tmpfs", "tmpfs", "/tmp"),
            raw("/dev/nvme0n1p5", "fuseblk", "/mnt/universal"),
            raw("App.AppImage", "fuse.App.AppImage", "/tmp/.mount_App"),
        ];
        let d = pick_drives(list, Path::new("/home/me/.local/share/rustle/mounts"));
        let labels: Vec<_> = d.iter().map(|d| d.label.as_str()).collect();
        assert_eq!(labels, ["System", "universal"]);
    }

    #[test]
    fn btrfs_subvolumes_show_once() {
        let list = vec![
            raw("/dev/sda2", "btrfs", "/home"),
            raw("/dev/sda2", "btrfs", "/"),
            raw("/dev/sda2", "btrfs", "/srv"),
        ];
        let d = pick_drives(list, Path::new("/nowhere"));
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].mount, PathBuf::from("/"));
    }

    #[test]
    fn cloud_mounts_and_empty_ones_are_skipped() {
        let mut empty = raw("/dev/sdb1", "ext4", "/mnt/empty");
        empty.total = 0;
        let list = vec![raw("gdrive:", "ext4", "/data/rustle/mounts/gdrive"), empty];
        assert!(pick_drives(list, Path::new("/data/rustle/mounts")).is_empty());
    }

    /// Prints what this computer reports. Not run by default; run with
    /// `cargo test -p rustle-fs print_places -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn print_places() {
        for p in user_folders() {
            println!("folder {:10} {}", p.label, p.path.display());
        }
        for d in drives() {
            println!(
                "drive  {:10} {} free {} of {}",
                d.label,
                d.mount.display(),
                d.free,
                d.total
            );
        }
    }

    #[test]
    fn home_is_first_user_folder() {
        assert_eq!(user_folders()[0].path, dirs::home_dir().unwrap());
    }
}
