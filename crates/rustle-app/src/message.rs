//! Every event the UI can produce. `update` in app.rs handles each one.

use std::path::PathBuf;

use rustle_core::{Entry, ThemeChoice};

/// Why a folder is being loaded. It decides what happens to the history
/// once the load succeeds (a failed load never touches the history).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Nav {
    /// Went somewhere new: the old folder goes onto the back list.
    Fresh,
    Back,
    Forward,
    /// Same folder again (F5, first load): history unchanged.
    Reload,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    // Browsing (always in the active tab)
    Navigate(PathBuf),
    OpenFile(PathBuf),
    Up,
    Back,
    Forward,
    Refresh,
    Loaded {
        tab_id: u64,
        request: u64,
        dir: PathBuf,
        nav: Nav,
        result: Result<Vec<Entry>, String>,
    },

    // Path bar
    /// Switch the path bar from buttons to a text field (click or Ctrl+L).
    StartEditLocation,
    LocationChanged(String),
    LocationSubmit,
    /// Escape: leave typing mode (later also closes menus).
    Escape,

    // Tabs
    /// `None` opens the new tab in the home folder.
    NewTab(Option<PathBuf>),
    CloseTab(u64),
    CloseActiveTab,
    SelectTab(usize),
    NextTab,
    PrevTab,

    // Settings page
    OpenSettings,
    CloseSettings,
    SetTheme(ThemeChoice),
    /// Sent while dragging the slider. Changes the look but doesn't save yet.
    SetOpacity(f32),
    SetBlur(bool),
    SetShowHidden(bool),
    ToggleHidden,
    /// Write settings to disk (after a change, or when the slider is released).
    SaveSettings,
    SettingsSaved(Result<(), String>),
}
