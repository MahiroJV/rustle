//! Every event the UI can produce. `update` in app.rs handles each one.

use std::path::PathBuf;

use iced::keyboard::Modifiers;
use rustle_core::{Entry, IconSize, Layout, SortKey, ThemeChoice};
use rustle_fs::places::{Drive, Place};

use crate::sidebar::Section;

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
    /// Escape: leave typing mode, close the View menu.
    Escape,

    // View menu & file list look
    ToggleViewMenu,
    CloseViewMenu,
    SetLayout(Layout),
    SetIconSize(IconSize),
    ZoomIn,
    ZoomOut,
    /// Chosen in the View menu.
    SetSort(SortKey),
    SetDescending(bool),
    /// A column header was clicked (same column again flips the direction).
    SortByColumn(SortKey),
    /// Item counts for the folders in a tab's listing, from the background.
    ItemCounts {
        tab_id: u64,
        request: u64,
        counts: Vec<(PathBuf, Option<u64>)>,
    },

    // Home page & sidebar
    ShowHomePage,
    /// User folders and drives, read in the background.
    PlacesLoaded {
        places: Vec<Place>,
        drives: Vec<Drive>,
    },
    /// ▸/▾ next to a sidebar folder.
    SidebarToggle(Section, PathBuf),
    /// Sub-folders of a sidebar folder, read in the background.
    SidebarChildren {
        section: Section,
        path: PathBuf,
        result: Result<Vec<Entry>, String>,
    },

    SetPreviews(bool),
    /// A background thumbnail job ended (`None` = couldn't make one).
    ThumbLoaded {
        path: PathBuf,
        handle: Option<iced::widget::image::Handle>,
    },

    // Raw input the app needs to remember
    /// Ctrl/Shift/Alt pressed or released (for Ctrl + mouse wheel).
    ModifiersChanged(Modifiers),
    /// Mouse wheel moved; positive = up/away from you.
    Wheel(f32),

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
