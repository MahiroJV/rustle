//! rustle-core — the file-manager model. No UI, no OS-specific code.

mod crumbs;
mod entry;
mod fs;
mod history;
mod kind;
pub mod settings;
mod tree;
mod view;

pub use crumbs::{Crumb, crumbs};
pub use entry::{Entry, SortKey, format_size, sort_entries, sort_entries_by};
pub use fs::{FileSystem, FsError};
pub use history::History;
pub use kind::{FileKind, extension, file_kind};
pub use settings::{Settings, ThemeChoice, clamp_opacity};
pub use tree::expand_chain;
pub use view::{IconSize, Layout, ViewSettings};
