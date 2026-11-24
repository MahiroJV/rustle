//! rustle-fs — the local-disk backend for rustle-core's `FileSystem` trait.

use std::fs::{DirEntry, Metadata};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use rustle_core::{Entry, FileSystem, FsError};

mod settings_store;
pub use settings_store::{load_settings, save_settings, settings_path};

#[derive(Debug, Default, Clone, Copy)]
pub struct LocalFs;

impl FileSystem for LocalFs {
    fn list(&self, dir: &Path) -> Result<Vec<Entry>, FsError> {
        let read = std::fs::read_dir(dir).map_err(|e| map_err(e, dir))?;

        let mut entries = Vec::new();
        for item in read.flatten() {
            let name = item.file_name().to_string_lossy().into_owned();
            // metadata() follows symlinks, so a link to a folder counts as a folder.
            let meta = std::fs::metadata(item.path()).ok();
            entries.push(Entry {
                hidden: is_hidden(&name, &item),
                is_dir: meta.as_ref().is_some_and(Metadata::is_dir),
                size: meta.as_ref().map_or(0, Metadata::len),
                modified: meta.as_ref().and_then(|m| m.modified().ok()),
                path: item.path(),
                name,
            });
        }
        Ok(entries)
    }

    fn home(&self) -> PathBuf {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"))
    }

    fn open(&self, path: &Path) -> Result<(), FsError> {
        // Detached so closing rustle doesn't take the opened app with it.
        open::that_detached(path).map_err(|e| map_err(e, path))
    }
}

/// How many items are directly inside `dir` (not counting deeper levels).
/// Used for the "Items" column. Hidden items count too, like `ls -A`.
pub fn count_items(dir: &Path) -> Result<u64, FsError> {
    let read = std::fs::read_dir(dir).map_err(|e| map_err(e, dir))?;
    Ok(read.count() as u64)
}

/// Dot-files are hidden everywhere; on Windows the "hidden" attribute counts too.
fn is_hidden(name: &str, item: &DirEntry) -> bool {
    if name.starts_with('.') {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
        if let Ok(meta) = item.metadata() {
            return meta.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0;
        }
    }
    #[cfg(not(windows))]
    let _ = item;
    false
}

fn map_err(e: std::io::Error, path: &Path) -> FsError {
    match e.kind() {
        ErrorKind::NotFound => FsError::NotFound(path.to_path_buf()),
        ErrorKind::PermissionDenied => FsError::PermissionDenied(path.to_path_buf()),
        _ => FsError::Other(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_items_counts_direct_children_only() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), "x").unwrap();
        std::fs::write(dir.path().join(".hidden"), "x").unwrap();
        std::fs::create_dir_all(dir.path().join("sub/deeper")).unwrap();
        assert_eq!(count_items(dir.path()).unwrap(), 3); // a.txt, .hidden, sub
    }

    #[test]
    fn count_items_of_missing_dir_is_an_error() {
        assert!(count_items(Path::new("/definitely/not/here")).is_err());
    }

    #[test]
    fn list_reports_modified_time() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("f"), "x").unwrap();
        let entries = LocalFs.list(dir.path()).unwrap();
        assert!(entries[0].modified.is_some());
    }
}
