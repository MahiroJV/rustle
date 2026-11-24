//! One tab: its own folder, file list and back/forward history.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rustle_core::{Entry, History};

pub struct Tab {
    /// Stays the same for the tab's whole life (its index can change when
    /// other tabs close), so background results find the right tab.
    pub id: u64,
    pub cwd: PathBuf,
    pub entries: Vec<Entry>,
    pub loading: bool,
    /// Bumped on every load; results from older requests are dropped, so a
    /// slow folder can't overwrite one the user clicked afterwards.
    pub request: u64,
    pub history: History,
    /// What's typed in the location bar (may differ from `cwd` while editing).
    pub location_input: String,
    /// How many items each sub-folder holds (the "Items" column).
    /// Missing = still counting; `None` = couldn't be read.
    pub item_counts: HashMap<PathBuf, Option<u64>>,
    /// Showing the Home page instead of `cwd`'s files.
    pub home_page: bool,
    /// How far the file list is scrolled, in pixels. The list only builds
    /// the rows around this position, so it must match the real scroll.
    pub scroll_y: f32,
}

/// How the Home page is written in a tab's history: an empty path, which is
/// never a real folder. Back/Forward check for it.
pub fn home_page_marker() -> PathBuf {
    PathBuf::new()
}

impl Tab {
    pub fn new(id: u64, cwd: PathBuf) -> Self {
        Self {
            id,
            location_input: cwd.display().to_string(),
            cwd,
            entries: Vec::new(),
            loading: false,
            request: 0,
            history: History::default(),
            item_counts: HashMap::new(),
            home_page: false,
            scroll_y: 0.0,
        }
    }

    /// A new tab that opens on the Home page (its `cwd` is the home folder,
    /// loaded in the background, so the path bar works right away).
    pub fn on_home_page(id: u64, home: PathBuf) -> Self {
        Self {
            home_page: true,
            ..Self::new(id, home)
        }
    }

    /// Where this tab "is", as stored in the history.
    pub fn location(&self) -> PathBuf {
        if self.home_page {
            home_page_marker()
        } else {
            self.cwd.clone()
        }
    }

    /// Folder name, or the whole path for a root like `/` or `C:\`.
    pub fn title(&self) -> String {
        if self.home_page {
            return "Home page".into();
        }
        title_of(&self.cwd)
    }
}

fn title_of(path: &Path) -> String {
    match path.file_name() {
        Some(name) => name.to_string_lossy().into_owned(),
        None => path.display().to_string(),
    }
}

/// Shorten long tab titles: at most `max` characters, ending with "…".
/// Counts characters, not bytes, so names in any language are cut safely.
pub fn shorten(title: &str, max: usize) -> String {
    if title.chars().count() <= max {
        title.to_string()
    } else {
        let cut: String = title.chars().take(max - 1).collect();
        format!("{cut}…")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_of_root_is_slash() {
        assert_eq!(Tab::new(0, "/".into()).title(), "/");
    }

    #[test]
    fn title_is_folder_name() {
        assert_eq!(Tab::new(0, "/home/me/src".into()).title(), "src");
    }

    #[test]
    fn shorten_counts_characters() {
        assert_eq!(shorten("short", 10), "short");
        assert_eq!(shorten("abcdefghij", 5), "abcd…");
        assert_eq!(shorten("şəkillərim", 5), "şəki…");
    }
}
