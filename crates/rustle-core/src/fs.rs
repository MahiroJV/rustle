use std::path::{Path, PathBuf};

use crate::Entry;

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
    /// Open a file with the system's default application.
    fn open(&self, path: &Path) -> Result<(), FsError>;
}
