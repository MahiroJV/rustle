use std::path::PathBuf;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::kind::file_kind;

/// One item in a directory listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
    pub hidden: bool,
    /// Last modification time. `None` if the system couldn't tell us.
    pub modified: Option<SystemTime>,
}

/// What the file list is sorted by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortKey {
    #[default]
    Name,
    Size,
    Modified,
    Kind,
}

/// Folders first, then case-insensitive by name.
pub fn sort_entries(entries: &mut [Entry]) {
    sort_entries_by(entries, SortKey::Name, false);
}

/// Sort by `key`. Folders always stay on top, whatever the direction.
/// Ties (same size, same kind…) are broken by name, so the order is stable
/// and predictable.
pub fn sort_entries_by(entries: &mut [Entry], key: SortKey, descending: bool) {
    // Cached key: each name is lowercased once, not on every comparison.
    entries.sort_by_cached_key(|e| (!e.is_dir, e.name.to_lowercase()));
    if key == SortKey::Name && !descending {
        return;
    }
    // A stable sort keeps the name order from above for equal keys.
    entries.sort_by(|a, b| {
        let folders_first = b.is_dir.cmp(&a.is_dir);
        let by_key = match key {
            SortKey::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            // A folder's "size" from the disk is meaningless, so folders
            // sort among themselves by name when sorting by size.
            SortKey::Size if a.is_dir && b.is_dir => std::cmp::Ordering::Equal,
            SortKey::Size => a.size.cmp(&b.size),
            SortKey::Modified => a.modified.cmp(&b.modified),
            SortKey::Kind => {
                file_kind(&a.name, a.is_dir).cmp(&file_kind(&b.name, b.is_dir))
            }
        };
        folders_first.then(if descending { by_key.reverse() } else { by_key })
    });
}

/// Human-readable size, e.g. `512 B`, `1.5 KiB`, `3.2 MiB`.
pub fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["KiB", "MiB", "GiB", "TiB", "PiB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64 / 1024.0;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn e(name: &str, is_dir: bool) -> Entry {
        Entry {
            name: name.into(),
            path: name.into(),
            is_dir,
            size: 0,
            hidden: false,
            modified: None,
        }
    }

    fn names(v: &[Entry]) -> Vec<&str> {
        v.iter().map(|x| x.name.as_str()).collect()
    }

    #[test]
    fn folders_come_first_then_by_name() {
        let mut v = vec![
            e("b.txt", false),
            e("Zeta", true),
            e("a.txt", false),
            e("alpha", true),
        ];
        sort_entries(&mut v);
        assert_eq!(names(&v), ["alpha", "Zeta", "a.txt", "b.txt"]);
    }

    #[test]
    fn by_size_descending_keeps_folders_on_top() {
        let sized = |name, size| Entry { size, ..e(name, false) };
        let mut v = vec![sized("small", 1), e("dir", true), sized("big", 900), sized("mid", 50)];
        sort_entries_by(&mut v, SortKey::Size, true);
        assert_eq!(names(&v), ["dir", "big", "mid", "small"]);
    }

    #[test]
    fn by_modified_oldest_first() {
        let at = |name, secs| Entry {
            modified: Some(SystemTime::UNIX_EPOCH + Duration::from_secs(secs)),
            ..e(name, false)
        };
        let mut v = vec![at("new", 300), at("old", 100), at("mid", 200)];
        sort_entries_by(&mut v, SortKey::Modified, false);
        assert_eq!(names(&v), ["old", "mid", "new"]);
    }

    #[test]
    fn by_kind_then_name() {
        let mut v = vec![e("z.png", false), e("song.mp3", false), e("a.png", false)];
        sort_entries_by(&mut v, SortKey::Kind, false);
        // Image comes before Audio in FileKind's order; same kind → by name.
        assert_eq!(names(&v), ["a.png", "z.png", "song.mp3"]);
    }

    #[test]
    fn by_name_descending() {
        let mut v = vec![e("a", false), e("c", false), e("b", false), e("d", true)];
        sort_entries_by(&mut v, SortKey::Name, true);
        assert_eq!(names(&v), ["d", "c", "b", "a"]);
    }

    #[test]
    fn sizes_are_human_readable() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(1023), "1023 B");
        assert_eq!(format_size(1024), "1.0 KiB");
        assert_eq!(format_size(1536), "1.5 KiB");
        assert_eq!(format_size(5 * 1024 * 1024), "5.0 MiB");
    }
}
