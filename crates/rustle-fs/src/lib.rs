//! rustle-fs — the local-disk backend for rustle-core's `FileSystem` trait.

use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use rustle_core::{Entry, FileSystem, FsError};

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
                hidden: name.starts_with('.'),
                is_dir: meta.as_ref().is_some_and(|m| m.is_dir()),
                size: meta.as_ref().map_or(0, |m| m.len()),
                path: item.path(),
                name,
            });
        }
        Ok(entries)
    }

    fn home(&self) -> PathBuf {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"))
    }
}

fn map_err(e: std::io::Error, path: &Path) -> FsError {
    match e.kind() {
        ErrorKind::NotFound => FsError::NotFound(path.to_path_buf()),
        ErrorKind::PermissionDenied => FsError::PermissionDenied(path.to_path_buf()),
        _ => FsError::Other(e.to_string()),
    }
}