//! rustle-core — the file-manager model. No UI, no OS-specific code.

mod entry;
mod fs;
mod history;
mod kind;
pub mod settings;

pub use entry::{Entry, format_size, sort_entries};
pub use fs::{FileSystem, FsError};
pub use history::History;
pub use kind::{FileKind, extension, file_kind};
pub use settings::{Settings, ThemeChoice, clamp_opacity};
