# Rustle Explorer UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give Rustle a custom look with a settings page (opacity and blur), tabs, an Explorer-style sidebar tree, a right-click menu with file operations, and cloud accounts through rclone.

**Architecture:** Pure logic (settings model, history, tree path expansion, paste naming, rclone output parsing) lives in library crates and gets unit tests. `rustle-app` is split into small UI modules that call those crates. Every disk, network or process call runs in `Task::perform` + `tokio::task::spawn_blocking`, never in `update`/`view`. Cloud accounts are rclone FUSE mounts, so the UI treats them as normal folders.

**Tech Stack:** Rust 2024, iced 0.14 (`tokio`, `image`, `svg`), serde + toml, sysinfo, trash, open, dirs, serde_json, tempfile (dev), rclone ≥ 1.69 (external).

**Spec:** `docs/superpowers/specs/2026-11-23-rustle-explorer-ui-design.md`

## Global Constraints

- Never call the filesystem, network or a child process synchronously in `update`/`view`. Use `Task::perform` with `spawn_blocking`, and guard results with a request counter like the current `request: u64`.
- Dark palette: background `#16181D`, surface `#1E2128`, raised `#262A33`, text `#E6E8EE`, muted `#8B91A0`, accent `#E07A3F`, danger `#E5534B`.
- Light palette: background `#F6F7F9`, surface `#FFFFFF`, raised `#ECEEF2`, text `#1C1F26`, muted `#6B7280`, accent `#E07A3F`, danger `#E5534B`.
- Corner radius 6 px, row height ~28 px, selection = accent at 20 % alpha, sidebar width 220 px.
- Opacity range 0.60–1.00. Blur applies at startup only, and its label says "(restart to apply)". The blur toggle is hidden on Windows.
- Settings file: `dirs::config_dir()/rustle/settings.toml`.
- Cloud mounts: `dirs::data_dir()/rustle/mounts/<remote>`, started with `--vfs-cache-mode full`, detached, never stopped when Rustle quits.
- Rustle never asks for, stores or logs cloud passwords or tokens.
- Do not change the existing layering: `rustle-core` has no UI and no OS code, and `rustle-app` only talks to the `FileSystem` trait for file operations.
- Work happens on branch `feature/explorer-ui`, with one commit per task.

## Review Focus

1. **Pasting into the same folder you copied from** must create `name (2).ext` and never overwrite. Test: `paste_into_same_dir_never_overwrites` (Task 9).
2. **Moving a folder into itself or one of its children** must fail with an error and leave the source intact. Test: `move_into_own_child_is_rejected` (Task 9).
3. **A corrupt `settings.toml`** must not stop Rustle from starting; it falls back to defaults. Test: `corrupt_file_gives_defaults` (Task 2).
4. **Expanding a folder you can't read** in the sidebar must show the node as empty with the error in the status line, with no crash or hang. Test: `list_unreadable_dir_is_permission_denied` (Task 9), plus the error path in Task 8.
5. **A broken symlink** must still be listed, as a non-folder, and opening it must report an error instead of panicking. Test: `broken_symlink_is_listed_as_file` (Task 9).

---

## File Structure

```
crates/rustle-core/src/
  lib.rs        re-exports
  entry.rs      Entry, sort_entries, format_size        (moved from lib.rs)
  fs.rs         FsError, FileSystem trait               (moved + extended)
  settings.rs   Settings, ThemeChoice                   Task 1
  history.rs    History                                 Task 4
  tree.rs       expand_chain                            Task 6
  naming.rs     unique_name                             Task 9
crates/rustle-fs/src/
  lib.rs        LocalFs                                 (extended Task 9)
  settings_store.rs  load/save                          Task 2
  places.rs     user_folders, drives, is_real_fs        Task 7
crates/rustle-cloud/                                    NEW crate, Tasks 11–12
  src/lib.rs    Rclone, Provider, UploadStats
  src/parse.rs  parse_remotes, parse_upload_stats, is_mounted_in, mount_args
  src/terminal.rs find_terminal
crates/rustle-app/src/
  main.rs       boot: load settings → window settings → application(...)
  app.rs        App state, update, view composition
  message.rs    Message enum
  theme.rs      palettes → iced::Theme, style helpers
  settings_page.rs
  tab.rs        Tab (folder, entries, history, selection, loading)
  tabs_bar.rs
  sidebar.rs    Node tree, lazy expansion, follow current folder
  file_list.rs  rows, selection, inline edit field
  context_menu.rs
  cloud.rs      app-side cloud state + tasks
```

---

## Phase 1 — Look & settings

### Task 1: Split core into modules and add the `Settings` model

**Files:**
- Modify: `crates/rustle-core/src/lib.rs`, `crates/rustle-core/Cargo.toml`
- Create: `crates/rustle-core/src/{entry.rs,fs.rs,settings.rs}`

**Interfaces:**
- Produces:
  - `pub enum ThemeChoice { Dark, Light }`, which derives `Serialize, Deserialize, Default (Dark), Clone, Copy, PartialEq, Eq, Debug`.
  - `pub struct Settings { pub theme: ThemeChoice, pub opacity: f32, pub blur: bool, pub show_hidden: bool, pub favorites: Vec<PathBuf> }` with `#[serde(default)]`, deriving `Serialize, Deserialize, Clone, Debug, PartialEq`.
  - `impl Default for Settings`: Dark, `1.0`, `false`, `false`, `[]`.
  - `pub fn clamp_opacity(v: f32) -> f32`, which clamps to `0.60..=1.00` and maps NaN to `1.0`.
  - `Settings::normalized(self) -> Self`, which applies `clamp_opacity` and removes duplicate favorites while keeping their order.
  - `lib.rs` re-exports everything at the crate root, so existing `use rustle_core::{Entry, ...}` lines keep working.

- [ ] **Step 1:** Create branch: `git switch -c feature/explorer-ui`
- [ ] **Step 2:** Move `Entry`, `sort_entries`, `format_size` and their tests to `entry.rs`, and `FsError`/`FileSystem` to `fs.rs`. Re-export them from `lib.rs`. Add `serde = { version = "1", features = ["derive"] }` to the workspace and to core.
- [ ] **Step 3: Write failing tests in `settings.rs`**

```rust
#[test] fn defaults() {
    let s = Settings::default();
    assert_eq!((s.theme, s.opacity, s.blur, s.show_hidden), (ThemeChoice::Dark, 1.0, false, false));
    assert!(s.favorites.is_empty());
}
#[test] fn opacity_is_clamped() {
    assert_eq!(clamp_opacity(0.2), 0.60);
    assert_eq!(clamp_opacity(1.5), 1.00);
    assert_eq!(clamp_opacity(f32::NAN), 1.0);
    assert_eq!(clamp_opacity(0.8), 0.8);
}
#[test] fn normalized_dedups_favorites_keeping_order() {
    let s = Settings { favorites: vec!["/a".into(), "/b".into(), "/a".into()], opacity: 9.0, ..Default::default() }.normalized();
    assert_eq!(s.favorites, vec![PathBuf::from("/a"), PathBuf::from("/b")]);
    assert_eq!(s.opacity, 1.0);
}
```

- [ ] **Step 4:** Run `cargo test -p rustle-core`. Expected: the settings tests fail to compile.
- [ ] **Step 5:** Implement `settings.rs` as described in Interfaces.
- [ ] **Step 6:** Run `cargo test -p rustle-core && cargo build`. Expected: all tests pass, including the old sort and size tests, and the app still builds.
- [ ] **Step 7:** Commit `refactor(core): split modules, add Settings model`

### Task 2: Settings file load/save

**Files:**
- Create: `crates/rustle-fs/src/settings_store.rs`
- Modify: `crates/rustle-fs/src/lib.rs`, `crates/rustle-fs/Cargo.toml` (add `toml = "0.9"`, `serde`, dev `tempfile = "3"`)

**Interfaces:**
- Consumes: `Settings`, `Settings::normalized` (Task 1)
- Produces:
  - `pub fn settings_path() -> PathBuf`, which returns `dirs::config_dir().unwrap_or(home)/rustle/settings.toml`.
  - `pub fn load_settings(path: &Path) -> Settings`. It never fails: a missing or unparsable file gives `Settings::default()`. The result is always `normalized()`.
  - `pub fn save_settings(path: &Path, s: &Settings) -> Result<(), FsError>`. It creates the parent dirs and writes atomically: write to `settings.toml.tmp`, then rename.

- [ ] **Step 1: Write failing tests**

```rust
#[test] fn round_trip() {
    let dir = tempfile::tempdir().unwrap(); let p = dir.path().join("x/settings.toml");
    let s = Settings { theme: ThemeChoice::Light, opacity: 0.8, blur: true, show_hidden: true, favorites: vec!["/tmp".into()] };
    save_settings(&p, &s).unwrap();
    assert_eq!(load_settings(&p), s);   // Settings derives PartialEq
}
#[test] fn missing_file_gives_defaults() { assert_eq!(load_settings(Path::new("/nonexistent/s.toml")), Settings::default()); }
#[test] fn corrupt_file_gives_defaults() {
    let dir = tempfile::tempdir().unwrap(); let p = dir.path().join("s.toml");
    std::fs::write(&p, "theme = [[[ not toml").unwrap();
    assert_eq!(load_settings(&p), Settings::default());
}
#[test] fn partial_file_fills_defaults() {
    let dir = tempfile::tempdir().unwrap(); let p = dir.path().join("s.toml");
    std::fs::write(&p, "opacity = 0.7\n").unwrap();
    let s = load_settings(&p); assert_eq!(s.opacity, 0.7); assert_eq!(s.theme, ThemeChoice::Dark);
}
```

- [ ] **Step 2:** Run `cargo test -p rustle-fs`. Expected: FAIL (missing functions).
- [ ] **Step 3:** Implement the three functions.
- [ ] **Step 4:** Run `cargo test -p rustle-fs`. Expected: PASS.
- [ ] **Step 5:** Commit `feat(fs): settings load/save`

### Task 3: Theme, layout shell, settings page

**Files:**
- Create: `crates/rustle-app/src/{app.rs,message.rs,theme.rs,settings_page.rs,file_list.rs}`
- Modify: `crates/rustle-app/src/main.rs`

**Interfaces:**
- Consumes: `load_settings`, `save_settings`, `settings_path` (Task 2); `Settings`, `ThemeChoice`, `clamp_opacity` (Task 1)
- Produces:
  - `theme::palette(choice: ThemeChoice) -> Colors`, where `pub struct Colors { bg, surface, raised, text, muted, accent, danger: iced::Color }` holds the hex values from Global Constraints.
  - `theme::iced_theme(choice) -> iced::Theme`, built with `Theme::custom("Rustle Dark"/"Rustle Light", Palette { background: bg, text, primary: accent, success: accent, warning: accent, danger })`.
  - `theme::row_button(colors) -> impl Fn(&Theme, button::Status) -> button::Style`: transparent background, `raised` on hover.
  - `theme::selected_row(colors)`: the same, but with accent at alpha 0.2.
  - `theme::panel(color) -> impl Fn(&Theme) -> container::Style`: radius 6.
  - `App { settings: Settings, page: Page, … }` with `enum Page { Files, Settings }`.
  - `Message::{OpenSettings, CloseSettings, SetTheme(ThemeChoice), SetOpacity(f32), SetBlur(bool), SetShowHidden(bool), SettingsSaved(Result<(), String>)}`. Every `Set*` updates `self.settings` and returns a save task (spawn_blocking `save_settings`).
  - `main.rs` reads the settings before `iced::application(...)` and sets `.window(window::Settings { transparent: s.opacity < 1.0 || s.blur, blur: s.blur, ..Default::default() })`, `.theme(|app| iced_theme(app.settings.theme))` and `.style(|app, theme| Style { background_color: bg.scale_alpha(opacity), text_color })`.
  - `transparent` is fixed at startup. If the user drops opacity below 1.0 while the window started opaque, the settings page shows "(restart to apply)" next to the slider too.

- [ ] **Step 1:** Move the current App, Message and the `view` code into `app.rs`, `message.rs` and `file_list.rs` unchanged. Run `cargo build`. Expected: OK.
- [ ] **Step 2:** Add the `theme.rs` palette and styles, and a unit test `dark_accent_is_copper` asserting `palette(ThemeChoice::Dark).accent == Color::from_rgb8(0xE0, 0x7A, 0x3F)` and that the Light `bg` is `0xF6F7F9`.
- [ ] **Step 3:** Run `cargo test -p rustle-app`. Expected: PASS.
- [ ] **Step 4:** Lay out the shell as in the spec sketch: toolbar row (`←` `→` `↑` disabled for now, path text, `⚙`), a 220 px sidebar placeholder panel, the file list with a Size column, and a status bar `"{n} items · {h} hidden"`. Remove the "Show hidden" header button, since the setting replaces it.
- [ ] **Step 5:** Build `settings_page.rs`: theme radio (Dark/Light), opacity slider 0.60–1.00 step 0.01, blur toggler with "(restart to apply)" (`#[cfg(not(windows))]` only), show-hidden toggler, and a Back button.
- [ ] **Step 6:** Run `cargo clippy --all-targets -- -D warnings && cargo run`. Expected: the dark theme shows. Change opacity to 0.8 and theme to Light, quit, and run again. Expected: both settings persisted (check `~/.config/rustle/settings.toml`). With blur on and restarted on KDE/Hyprland, the background is blurred.
- [ ] **Step 7:** Commit `feat(app): theme, layout shell, settings page`

---

## Phase 2 — Tabs

### Task 4: Navigation history

**Files:** Create `crates/rustle-core/src/history.rs`

**Interfaces:**
- Produces `pub struct History`, which derives `Default, Clone, Debug`, with:
  - `visit(&mut self, from: PathBuf)`: pushes onto back and clears forward.
  - `back(&mut self, current: PathBuf) -> Option<PathBuf>`
  - `forward(&mut self, current: PathBuf) -> Option<PathBuf>`
  - `can_back(&self) -> bool`, `can_forward(&self) -> bool`

- [ ] **Step 1: Failing test**

```rust
#[test] fn back_and_forward() {
    let mut h = History::default();
    h.visit("/a".into());                 // now at /b
    h.visit("/b".into());                 // now at /c
    assert_eq!(h.back("/c".into()), Some("/b".into()));
    assert_eq!(h.back("/b".into()), Some("/a".into()));
    assert_eq!(h.back("/a".into()), None);
    assert_eq!(h.forward("/a".into()), Some("/b".into()));
    h.visit("/b".into());                 // new branch clears forward
    assert!(!h.can_forward());
}
```

- [ ] **Step 2:** Run `cargo test -p rustle-core history`. Expected: FAIL.
- [ ] **Step 3:** Implement it.
- [ ] **Step 4:** Run the tests. Expected: PASS.
- [ ] **Step 5:** Commit `feat(core): navigation history`

### Task 5: Tabs in the app

**Files:** Create `crates/rustle-app/src/{tab.rs,tabs_bar.rs}`; modify `app.rs`, `message.rs`, `main.rs`

**Interfaces:**
- Consumes: `History` (Task 4)
- Produces:
  - `pub struct Tab { pub id: u64, pub cwd: PathBuf, pub entries: Vec<Entry>, pub loading: bool, pub request: u64, pub history: History, pub selected: Option<PathBuf>, pub location_input: String }`
  - `Tab::title(&self) -> String`: the folder name, or `"/"` for root.
  - `App { tabs: Vec<Tab>, active: usize, next_tab_id: u64, … }`
  - `App::load(&mut self, tab_id: u64, dir: PathBuf, push_history: bool) -> Task<Message>`. This replaces the current `load`. `Loaded` now carries `tab_id` and is dropped if the tab is gone or the request is stale.
  - Messages: `NewTab(Option<PathBuf>)` (`None` opens Home), `CloseTab(u64)`, `SelectTab(usize)`, `NextTab`, `PrevTab`, `Back`, `Forward`, `Up`, `Navigate(PathBuf)` (active tab), `LocationChanged(String)`, `LocationSubmit`.
  - Shortcuts live in one place: `crates/rustle-app/src/keybinds.rs` with a pure `pub fn shortcut(key: &keyboard::Key, mods: keyboard::Modifiers) -> Option<Message>` (unit-tested) and `pub fn mouse_button(b: mouse::Button) -> Option<Message>`, fed by one `iced::event::listen_with` subscription. Fixed defaults (not user-editable this round):
    - Ctrl+T new tab · Ctrl+W close tab · Ctrl+Tab / Ctrl+Shift+Tab next/prev tab
    - Alt+Left / **mouse Back button** → Back · Alt+Right / **mouse Forward button** → Forward
    - Alt+Up / Backspace → Up · Ctrl+L focus location bar · F5 refresh · Ctrl+H show/hide hidden · Ctrl+, settings
    - Keys that need a selection (F2 rename, Delete → trash, Ctrl+C/X/V, Ctrl+Shift+N new folder, Enter open) are added to `shortcut` in Task 10.
    - Shortcuts are ignored while a text field has focus, so typing a path with Backspace doesn't navigate up (`listen_with` passes `event::Status::Captured` for those → return `None`).
  - Closing the last tab does nothing. Middle-click on a tab closes it (`mouse_area::on_middle_press`).
  - A `LocationSubmit` to a path that fails to load keeps the old folder and shows the error in the status bar.

- [ ] **Step 1:** Write unit tests in `tab.rs`: `title_of_root_is_slash` (`Tab` at `/` → `"/"`) and `title_is_folder_name` (`/home/me/src` → `"src"`).
- [ ] **Step 2:** Run `cargo test -p rustle-app tab`. Expected: FAIL.
- [ ] **Step 3:** Implement `Tab` and the tab bar, then rewire `update`/`view` to the active tab.
- [ ] **Step 4:** Run `cargo test -p rustle-app && cargo clippy --all-targets -- -D warnings`. Expected: PASS.
- [ ] **Step 5:** Manual check with `cargo run`. Open 3 tabs, navigate each separately, use Back/Forward, type `/etc` in the location bar, try Ctrl+W on the last tab, and type a bad path. Expected: everything behaves as the Interfaces block says.
- [ ] **Step 6:** Commit `feat(app): tabs, history, location bar`

---

## Phase 3 — Sidebar

### Task 6: Path chain for "follow current folder"

**Files:** Create `crates/rustle-core/src/tree.rs`

**Interfaces:**
- Produces `pub fn expand_chain(roots: &[PathBuf], target: &Path) -> Option<(usize, Vec<PathBuf>)>`. It returns the index of the **deepest** root that contains `target`, plus every folder from just below that root down to `target` inclusive. If `target == root`, the chain is empty. If no root matches, it returns `None`. It compares path components, not string prefixes.

- [ ] **Step 1: Failing tests**

```rust
#[test] fn picks_deepest_root() {
    let roots = vec![PathBuf::from("/"), PathBuf::from("/home/me")];
    assert_eq!(expand_chain(&roots, Path::new("/home/me/src/rustle")),
        Some((1, vec!["/home/me/src".into(), "/home/me/src/rustle".into()])));
}
#[test] fn root_itself_gives_empty_chain() {
    assert_eq!(expand_chain(&[PathBuf::from("/home/me")], Path::new("/home/me")), Some((0, vec![])));
}
#[test] fn component_not_string_prefix() {
    assert_eq!(expand_chain(&[PathBuf::from("/home/me")], Path::new("/home/meow")), None);
}
```

- [ ] **Step 2:** Run the tests and confirm they FAIL. Then implement, run again and confirm PASS. Commit `feat(core): tree path chain`.

### Task 7: Places — user folders and drives

**Files:** Create `crates/rustle-fs/src/places.rs`; add `sysinfo` with `cargo add -p rustle-fs sysinfo`

**Interfaces:**
- Produces:
  - `pub struct Place { pub label: String, pub path: PathBuf }`
  - `pub fn user_folders() -> Vec<Place>`: Home first, then Desktop, Downloads, Documents, Pictures, Music, Videos from `dirs::*_dir()`, keeping only those that exist.
  - `pub struct Drive { pub label: String, pub mount: PathBuf, pub free: u64, pub total: u64 }`
  - `pub fn drives() -> Vec<Drive>`: from `sysinfo::Disks::new_with_refreshed_list()`, filtered by `is_real_fs`, sorted by mount, deduplicated by mount.
  - `pub fn is_real_fs(fs_type: &str, mount: &Path) -> bool`: false for `tmpfs`, `devtmpfs`, `proc`, `sysfs`, `overlay`, `squashfs`, `efivarfs`, `fuse.portal`, and for mounts under `/boot`, `/snap`, `/proc`, `/sys`, `/run` except `/run/media`. It is also false under the Rustle mounts dir, because cloud accounts are shown in their own section.

- [ ] **Step 1: Failing tests**

```rust
#[test] fn filters_pseudo() {
    assert!(is_real_fs("ext4", Path::new("/")));
    assert!(is_real_fs("vfat", Path::new("/run/media/me/USB")));
    assert!(!is_real_fs("tmpfs", Path::new("/tmp")));
    assert!(!is_real_fs("vfat", Path::new("/boot")));
    assert!(!is_real_fs("squashfs", Path::new("/snap/x")));
}
#[test] fn home_is_first_user_folder() { assert_eq!(user_folders()[0].path, dirs::home_dir().unwrap()); }
```

- [ ] **Step 2:** Run the tests and confirm they FAIL. Then implement, run again and confirm PASS. Commit `feat(fs): user folders and drives`.

### Task 8: Sidebar tree in the app

**Files:** Create `crates/rustle-app/src/sidebar.rs`; modify `app.rs`, `message.rs`

**Interfaces:**
- Consumes: `expand_chain` (Task 6), `user_folders`, `drives` (Task 7), `Settings.favorites`
- Produces:
  - `pub enum Section { Favorites, Home, Drives, Cloud }`
  - `pub struct Node { pub path: PathBuf, pub label: String, pub expanded: bool, pub children: Option<Vec<Node>>, pub loading: bool }` (`None` = not loaded yet)
  - `pub struct Sidebar { pub favorites: Vec<Node>, pub home: Vec<Node>, pub drives: Vec<Node>, pub cloud: Vec<Node> }`
  - `Sidebar::find_mut(&mut self, path: &Path) -> Option<&mut Node>`
  - `Sidebar::reveal(&mut self, cwd: &Path) -> Vec<PathBuf>`: uses `expand_chain` over all section roots, marks the chain expanded, and returns the paths whose children still need loading.
  - Messages: `ToggleNode(PathBuf)`, `NodeChildren { path: PathBuf, result: Result<Vec<Entry>, String> }`, `SidebarOpen(PathBuf)`, and `NewTab(Some(path))` on middle-click.
  - Children come from `FileSystem::list`, keeping only `is_dir` entries, sorted with `sort_entries`; hidden folders follow `settings.show_hidden`.
  - Drive labels show free space: `"{label}  {format_size(free)} free"`. Loading uses the same `spawn_blocking` pattern as tabs.
  - A load error sets `children = Some(vec![])` and shows the error in the status bar.
  - Every successful `Loaded` in the active tab, and every tab switch, calls `reveal`. The node equal to the active tab's cwd uses the `selected_row` style.

- [ ] **Step 1:** Unit tests in `sidebar.rs` (pure, using a hand-built `Sidebar`):
  - `reveal_expands_chain_and_reports_unloaded`: home root `/h` with child `/h/a` unloaded, `reveal("/h/a/b")`. Expect `/h` and `/h/a` to have `expanded == true`, and the result to contain `/h/a`.
  - `toggle_collapses`
- [ ] **Step 2:** Run them and confirm they FAIL. Then implement, and confirm `cargo test -p rustle-app` passes.
- [ ] **Step 3:** Manual check with `cargo run`. Expand Drives → `/` → `usr`. Navigate in the file list into `~/Documents/x`. Expected: the tree opens down to it and highlights it. Expand `/root` (no permission). Expected: an empty node plus an error in the status bar, and the UI stays responsive.
- [ ] **Step 4:** Commit `feat(app): sidebar tree`

---

## Phase 4 — Right-click menu & file operations

### Task 9: File operations on the trait

**Files:** Create `crates/rustle-core/src/naming.rs`; modify `crates/rustle-core/src/fs.rs`, `crates/rustle-fs/src/lib.rs`; `cargo add -p rustle-fs trash`

**Interfaces:**
- Produces:
  - `pub fn unique_name(dir: &Path, name: &str, exists: impl Fn(&Path) -> bool) -> PathBuf`
    - `report.txt` → `report (2).txt` → `report (3).txt`
    - Extension = after the last dot, except a leading dot: `.bashrc` → `.bashrc (2)`, `a.tar.gz` → `a.tar (2).gz`
    - If the name is free, it returns `dir/name` unchanged.
  - New `FileSystem` methods (all `-> Result<…, FsError>`):
    - `trash(&self, path: &Path) -> Result<(), FsError>`
    - `copy(&self, src: &Path, dest_dir: &Path) -> Result<PathBuf, FsError>`: recursive for folders, with the target name from `unique_name`. It never overwrites.
    - `move_to(&self, src: &Path, dest_dir: &Path) -> Result<PathBuf, FsError>`: tries `rename` first, and falls back to copy + delete when it crosses devices (`ErrorKind::CrossesDevices`). It returns `FsError::Other("can't move a folder into itself")` when `dest_dir` starts with `src`. Moving into the folder it's already in does nothing and returns `src`.
    - `rename(&self, path: &Path, new_name: &str) -> Result<PathBuf, FsError>`: rejects an empty name, a name with `/` and an existing target, using a new `FsError::AlreadyExists(PathBuf)` variant.
    - `create_dir(&self, parent: &Path, name: &str) -> Result<PathBuf, FsError>`, with the same name rules.

- [ ] **Step 1: Failing tests**, with `unique_name` tests in core and tempdir tests in `rustle-fs`:

```rust
// core
#[test] fn unique_names() {
    let taken = |p: &Path| ["/d/report.txt", "/d/report (2).txt", "/d/.bashrc", "/d/a.tar.gz"].iter().any(|t| Path::new(t) == p);
    assert_eq!(unique_name(Path::new("/d"), "report.txt", taken), PathBuf::from("/d/report (3).txt"));
    assert_eq!(unique_name(Path::new("/d"), ".bashrc", taken), PathBuf::from("/d/.bashrc (2)"));
    assert_eq!(unique_name(Path::new("/d"), "a.tar.gz", taken), PathBuf::from("/d/a.tar (2).gz"));
    assert_eq!(unique_name(Path::new("/d"), "new.txt", taken), PathBuf::from("/d/new.txt"));
}
// rustle-fs
#[test] fn paste_into_same_dir_never_overwrites()   // copy d/f.txt into d → d/f (2).txt, original unchanged
#[test] fn copy_folder_is_recursive()               // d/x/y/z.txt copied into e → e/x/y/z.txt
#[test] fn move_into_own_child_is_rejected()        // move d/x into d/x/sub → Err, d/x still exists
#[test] fn rename_rejects_existing_and_slash()      // AlreadyExists, and Err for "a/b"
#[test] fn create_dir_and_list()                    // create_dir(d, "New folder") → listed as is_dir
#[test] fn list_unreadable_dir_is_permission_denied()   // chmod 000 (#[cfg(unix)], skip if running as root)
#[test] fn broken_symlink_is_listed_as_file()       // symlink to missing target → Entry { is_dir: false }; open() → Err, no panic
```

  Each test asserts exactly what its comment says.
- [ ] **Step 2:** Run `cargo test -p rustle-core -p rustle-fs`. Expected: FAIL.
- [ ] **Step 3:** Implement `unique_name` and the five `LocalFs` methods.
- [ ] **Step 4:** Run the tests. Expected: PASS. Note: `trash` isn't unit-tested, because it would touch the real trash; Task 10 checks it by hand.
- [ ] **Step 5:** Commit `feat: file operations (copy, move, rename, mkdir, trash)`

### Task 10: Selection, context menu, clipboard, favorites

**Files:** Create `crates/rustle-app/src/context_menu.rs`; modify `file_list.rs`, `sidebar.rs`, `app.rs`, `message.rs`

**Interfaces:**
- Consumes: Task 9's trait methods, `Settings.favorites`, `save_settings`
- Produces:
  - `pub enum Target { File(PathBuf), Folder(PathBuf), Empty, Favorite(PathBuf), CloudAccount(String) }`
  - `pub enum MenuAction { Open, OpenInNewTab, Cut, Copy, Paste, Rename, Trash, CopyPath, NewFolder, Refresh, ToggleHidden, AddFavorite, RemoveFavorite, Disconnect }`
  - `pub fn actions_for(target: &Target, can_paste: bool) -> Vec<MenuAction>`: exactly the lists in spec §5. `Paste` appears only when `can_paste`.
  - `pub struct Clipboard { pub paths: Vec<PathBuf>, pub cut: bool }`, stored as `Option<Clipboard>` on `App`.
  - `App.cursor: Point`, tracked by a root `mouse_area(...).on_move(Message::CursorMoved)`.
  - `App.menu: Option<(Target, Point)>`
  - `App.editing: Option<Edit>`, where `enum Edit { Rename { path, text }, NewFolder { parent, text } }`
  - Messages:
    - `Select(PathBuf)`
    - `OpenMenu(Target)` (uses the current `cursor`)
    - `CloseMenu`
    - `Menu(MenuAction)`
    - `EditChanged(String)`, `EditSubmit`, `EditCancel`
    - `OpDone(Result<(), String>)`, which refreshes the active tab and any expanded sidebar parent.
  - The menu is drawn with `stack![content, pin(menu).x(cursor.x).y(cursor.y)]`, nudged left or up if it would overflow the window. Esc or a click outside sends `CloseMenu`.
  - Copy path puts text on the clipboard via `iced::clipboard::write`.
  - After a successful cut + paste, `Clipboard` is cleared.
  - Double-click opens (`mouse_area::on_double_click`). A single click only selects. This replaces today's single-click-to-open.
  - The settings page gets a Favorites list with a remove (×) button per entry, sending `Menu(RemoveFavorite)` for that path, so favorites are editable in both places as the spec says.

- [ ] **Step 1: Failing tests** for `actions_for`:

```rust
#[test] fn file_menu() { assert_eq!(actions_for(&Target::File("/f".into()), false),
    vec![Open, Cut, Copy, Rename, Trash, CopyPath]); }
#[test] fn folder_menu() { assert_eq!(actions_for(&Target::Folder("/d".into()), false),
    vec![Open, OpenInNewTab, AddFavorite, Cut, Copy, Rename, Trash, CopyPath]); }
#[test] fn empty_menu_paste_only_when_clipboard() {
    assert!(!actions_for(&Target::Empty, false).contains(&Paste));
    assert_eq!(actions_for(&Target::Empty, true), vec![NewFolder, Paste, Refresh, ToggleHidden]);
}
```

- [ ] **Step 2:** Run the tests and confirm they FAIL. Then implement and confirm `cargo test -p rustle-app` passes, along with `cargo clippy --all-targets -- -D warnings`.
- [ ] **Step 3:** Manual check with `cargo run`, testing every menu item in a scratch folder under `/tmp`:
  - Copy and paste in the same folder gives `(2)`.
  - Cut and paste into another tab moves the file.
  - Renaming to an existing name shows the error.
  - Trash puts the file in the system trash.
  - Add to Favorites is still there after a restart.
  - Right-clicking near the bottom-right corner keeps the menu on screen.
- [ ] **Step 4:** Commit `feat(app): right-click menu, selection, clipboard, favorites`

---

## Phase 5 — Cloud (rclone)

### Task 11: rclone output parsing (pure)

**Files:** Create crate `crates/rustle-cloud` (`Cargo.toml` with `serde`, `serde_json`, `dirs`, `thiserror`; add to workspace `members` and `[workspace.dependencies]`), `src/lib.rs`, `src/parse.rs`, `src/terminal.rs`

**Interfaces:**
- Produces:
  - `pub enum Provider { GoogleDrive, Dropbox, OneDrive, ICloud }` with `rclone_type(&self) -> &'static str` = `"drive"`, `"dropbox"`, `"onedrive"`, `"iclouddrive"`, and `label()`.
  - `pub fn parse_remotes(stdout: &str) -> Vec<String>`: from `rclone listremotes` lines like `gdrive:`. It strips the trailing `:` and skips blank lines.
  - `pub struct UploadStats { pub in_progress: u64, pub queued: u64 }` with `pending() = in_progress + queued`.
  - `pub fn parse_upload_stats(json: &str) -> Result<UploadStats, CloudError>`: reads `diskCache.uploadsInProgress` and `diskCache.uploadsQueued` from `rclone rc vfs/stats` output. A missing `diskCache` gives 0/0.
  - `pub fn is_mounted_in(mountinfo: &str, mountpoint: &Path) -> bool`: checks field 5 of each `/proc/self/mountinfo` line, which uses octal escapes such as `\040` for a space, so decode those.
  - `pub fn mount_args(remote: &str, mountpoint: &Path, rc_port: u16) -> Vec<String>`, which produces exactly:
    `["mount", "<remote>:", "<mountpoint>", "--vfs-cache-mode", "full", "--rc", "--rc-addr", "127.0.0.1:<port>", "--rc-no-auth"]`
  - `pub fn find_terminal(env_terminal: Option<&str>, exists: impl Fn(&str) -> bool) -> Option<String>`: `$TERMINAL` first, then the first that exists of `kitty`, `alacritty`, `foot`, `konsole`, `gnome-terminal`, `xterm`.
  - `#[derive(thiserror::Error)] pub enum CloudError { NotInstalled, Command(String), Parse(String) }`

- [ ] **Step 1: Failing tests**

```rust
#[test] fn remotes() { assert_eq!(parse_remotes("gdrive:\n\nmy box:\n"), vec!["gdrive", "my box"]); }
#[test] fn stats() {
    let s = parse_upload_stats(r#"{"diskCache":{"uploadsInProgress":1,"uploadsQueued":2}}"#).unwrap();
    assert_eq!(s.pending(), 3);
    assert_eq!(parse_upload_stats("{}").unwrap().pending(), 0);
}
#[test] fn mountinfo_with_space() {
    let mi = "36 35 0:42 / /home/me/My\\040Drive rw,nosuid - fuse.rclone gdrive: rw\n";
    assert!(is_mounted_in(mi, Path::new("/home/me/My Drive")));
    assert!(!is_mounted_in(mi, Path::new("/home/me/My")));
}
#[test] fn args() { assert_eq!(mount_args("gdrive", Path::new("/m/gdrive"), 5600)[..5],
    ["mount", "gdrive:", "/m/gdrive", "--vfs-cache-mode", "full"]); }
#[test] fn terminal_order() {
    assert_eq!(find_terminal(Some("wezterm"), |_| false), Some("wezterm".into()));
    assert_eq!(find_terminal(None, |t| t == "foot" || t == "xterm"), Some("foot".into()));
    assert_eq!(find_terminal(None, |_| false), None);
}
```

- [ ] **Step 2:** Run `cargo test -p rustle-cloud`. Expected: FAIL. Then implement, run again and confirm PASS.
- [ ] **Step 3:** Commit `feat(cloud): rclone output parsing`

### Task 12: rclone process control

**Files:** Modify `crates/rustle-cloud/src/lib.rs`

**Interfaces:**
- Consumes: Task 11's functions
- Produces `pub struct Rclone { bin: PathBuf }` with:
  - `Rclone::detect() -> Option<Rclone>`: runs `rclone version`; `None` if it can't be started.
  - `list_remotes(&self) -> Result<Vec<String>, CloudError>`
  - `add_remote(&self, name: &str, provider: Provider) -> Result<(), CloudError>`
    - Google Drive, Dropbox and OneDrive run `rclone config create <name> <type>`, which opens the browser, and wait for it to finish.
    - ICloud runs `<terminal> -e rclone config create <name> iclouddrive --all` via `find_terminal`, detached. Without a terminal it returns `CloudError::Command("Run in a terminal: rclone config create <name> iclouddrive --all")`.
  - `pub fn mounts_dir() -> PathBuf`: `dirs::data_dir()/rustle/mounts`.
  - `mount(&self, remote: &str) -> Result<PathBuf, CloudError>`
    - If it's already mounted (`is_mounted_in` on `/proc/self/mountinfo`), it just returns the path.
    - Otherwise it creates the dir and picks a free port (bind `127.0.0.1:0`, read the port, drop the listener).
    - It writes the port to `mounts_dir()/<remote>.port` and spawns `rclone` with `mount_args` detached: `Stdio::null()`, own process group via `CommandExt::process_group(0)` on unix, not waited on.
    - It then polls for up to 10 s until mounted. On timeout it returns `Command` with the last stderr from `mounts_dir()/<remote>.log` (pass `--log-file`).
  - `unmount(&self, remote: &str) -> Result<(), CloudError>`: `fusermount3 -u`, falling back to `fusermount -u` (Linux); `umount` on macOS.
  - `upload_stats(&self, remote: &str) -> Result<UploadStats, CloudError>`: `rclone rc --url http://127.0.0.1:<port from .port file> vfs/stats` → `parse_upload_stats`.

- [ ] **Step 1:** Add an integration test `tests/rclone_local.rs` with every test `#[ignore]` unless rclone is present. It creates a remote of type `alias` pointing at a tempdir (`rclone config create rustle-test alias remote=<tmp> --config <tmp>/rclone.conf`), passing a `--config` override via a `Rclone::with_config(path)` constructor kept for tests. It then checks:
  - mounting → a file written into the tempdir appears in the mount
  - `upload_stats` returns `Ok`
  - unmounting → `is_mounted_in` is false
- [ ] **Step 2:** Implement it. Run `cargo test -p rustle-cloud -- --include-ignored` on a machine with rclone + fuse3. Expected: PASS. Without rclone, the ignored tests are skipped and the rest pass.
- [ ] **Step 3:** Commit `feat(cloud): rclone mount/unmount/stats`

### Task 13: Cloud in the app

**Files:** Create `crates/rustle-app/src/cloud.rs`; modify `sidebar.rs`, `settings_page.rs`, `context_menu.rs`, `app.rs`, `message.rs`; add `rustle-cloud` to app deps

**Interfaces:**
- Consumes: Task 12's `Rclone`, Task 8's `Sidebar.cloud`, Task 10's `Target::CloudAccount`, `MenuAction::Disconnect`
- Produces:
  - `pub enum CloudState { Checking, Missing, Ready { rclone: Arc<Rclone>, accounts: Vec<Account> } }`
  - `pub struct Account { pub name: String, pub status: AccountStatus }`, with `enum AccountStatus { Idle, Mounting, Mounted(PathBuf), Failed(String) }`
  - Messages:
    - `CloudDetected(Option<Arc<Rclone>>)`, `CloudAccounts(Result<Vec<String>, String>)`
    - `Mount(String)`, `Mounted { name, result: Result<PathBuf, String> }`
    - `Disconnect(String)`, `DisconnectConfirmed(String)`
    - `AddAccount { name: String, provider: Provider }`, `AccountAdded(Result<(), String>)`
    - `UploadTick`, `UploadStats(u64)`
  - The sidebar Cloud section shows:
    - `Missing`: "Install rclone and fuse3 to add cloud accounts".
    - `Ready`: one node per account. Expanding an `Idle` node sends `Mount`. `Mounted(path)` makes the node's path the mountpoint, and children load like any folder. `Failed` shows the message plus a "Retry" row that sends `Mount`.
    - An "Add…" row that opens the add-account form on the settings page: name input + provider pick list.
  - The settings page Cloud block shows the rclone status (Missing, or the version found), the account list with a Disconnect button each, and the add form.
  - The status bar shows `⟳ uploading N` while the sum of `pending()` over mounted accounts is > 0. Polling runs as a `Subscription` via `iced::time::every(Duration::from_secs(3))`, only while there is at least one mounted account.
  - `Disconnect` with pending > 0 shows an inline warning with a "Disconnect anyway" button that sends `DisconnectConfirmed`. Otherwise it unmounts at once.
  - On quit nothing is unmounted (constraint).

- [ ] **Step 1:** Write a unit test `cloud.rs::pending_total_sums_mounted_only`. Make `fn pending_total(accounts: &[Account], stats: &HashMap<String, UploadStats>) -> u64` a pure function so it can be tested, and assert that `Idle` and `Failed` accounts are ignored.
- [ ] **Step 2:** Run it and confirm it FAILS. Then implement, and confirm `cargo test --workspace && cargo clippy --all-targets -- -D warnings` passes.
- [ ] **Step 3:** Manual check (needs `rclone`, `fuse3` and a Google account):
  - Add a Google Drive account; the browser sign-in completes.
  - Expand the account; the tree shows My Drive.
  - Open a `.txt`, edit it and save; the status shows `⟳ uploading 1` and then clears, and the file has changed in drive.google.com.
  - Quit Rustle; the mount is still there (`mount | grep rclone`). Restart; the mount is reused with no second rclone process.
  - Disconnect; it unmounts.
  - Uninstall or rename rclone; the "Install rclone" hint shows and local browsing works.
- [ ] **Step 4:** Commit `feat(app): cloud accounts via rclone`

---

## Done when

- `cargo test --workspace` and `cargo clippy --all-targets -- -D warnings` are clean.
- Every manual check in Tasks 3, 5, 8, 10 and 13 passes on the user's Arch machine.
- Branch `feature/explorer-ui` is ready to merge (finish with superpowers:finishing-a-development-branch).
