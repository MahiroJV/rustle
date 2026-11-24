//! Every event the UI can produce. `update` in app.rs handles each one.

use std::path::PathBuf;

use rustle_core::{Entry, ThemeChoice};

#[derive(Debug, Clone)]
pub enum Message {
    // Browsing
    Navigate(PathBuf),
    OpenFile(PathBuf),
    Up,
    Loaded {
        request: u64,
        dir: PathBuf,
        result: Result<Vec<Entry>, String>,
    },

    // Settings page
    OpenSettings,
    CloseSettings,
    SetTheme(ThemeChoice),
    /// Sent while dragging the slider. Changes the look but doesn't save yet.
    SetOpacity(f32),
    SetBlur(bool),
    SetShowHidden(bool),
    /// Write settings to disk (after a change, or when the slider is released).
    SaveSettings,
    SettingsSaved(Result<(), String>),
}
