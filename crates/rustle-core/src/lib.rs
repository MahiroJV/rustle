//! rustle-core — the file-manager model. No UI, no OS-specific code.

use std::path::{Path, PathBuf};

/// One item in a directory listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
    pub hidden: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum FsError {
    #[error("not found: {0}")]
    NotFound(PathBuf),
    #[error("permission denied: {0}")]
    PermissionDenied(PathBuf),
    #[error("{0}")]
    Other(String),
}

/// What the app needs from a filesystem. Backends (Linux, Windows, a fake
/// one for tests…) implement this; the UI only ever talks to the trait.
pub trait FileSystem: Send + Sync {
    fn list(&self, dir: &Path) -> Result<Vec<Entry>, FsError>;
    fn home(&self) -> PathBuf;
}

/// Folders first, then case-insensitive by name.
pub fn sort_entries(entries: &mut [Entry]) {
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(name: &str, is_dir: bool) -> Entry {
        Entry {
            name: name.into(),
            path: name.into(),
            is_dir,
            size: 0,
            hidden: false,
        }
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
        let names: Vec<_> = v.iter().map(|x| x.name.as_str()).collect();
        assert_eq!(names, ["alpha", "Zeta", "a.txt", "b.txt"]);
    }
}
