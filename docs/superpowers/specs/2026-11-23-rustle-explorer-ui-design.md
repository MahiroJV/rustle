# Rustle — Explorer-style UI, tabs, context menu, cloud

Date: 2026-11-23 · Status: draft for approval

## Intent

Turn Rustle from a single folder list into a lightweight, good-looking file
manager that works like Windows Explorer: a navigation tree on the left, tabs,
a right-click menu, and cloud accounts (Google Drive, Dropbox, OneDrive,
iCloud, …) that you sign in to and browse like any other folder, with edits
syncing back automatically.

"Lightweight" is the guiding word: fast start, no heavy chrome, nothing that
blocks the window.

## Goals

1. **Look & settings** — a custom theme and a Settings page with
   transparency and blur.
2. **Sidebar tree** in three sections — Favorites; user folders (Home,
   Desktop, Downloads, Documents, Pictures, Music, Videos); drives — plus a
   Cloud section. Every folder expands with ▸ like Explorer's left pane. The
   tree follows the current folder.
3. **Tabs.**
4. **Right-click menu** on files, empty space and sidebar items.
5. **Cloud** via rclone: sign in, browse as a tree, open/edit files, changes
   upload automatically.

## Non-goals (this round)

- Writing our own cloud API clients (decided: rclone does it).
- File preview pane. (Grid view and picture thumbnails were added on 2026-11-24, see §7.)
- "Open with…" app picker (default app only).
- Search.
- Drag and drop (between tabs, sidebar or other apps).
- Multi-select.
- Copy/move progress dialogs (operations run in the background; errors show
  in the status line).

## Anti-goals

- The window must never freeze — no filesystem or network call on the UI
  thread (cloud folders can be slow).
- A cloud edit must never be lost: quitting Rustle must not cancel pending
  uploads.
- No feature that only works with network access for local browsing.

## Constraints

- iced 0.14, Rust 2024 edition, existing crate split
  (`rustle-core` model/trait, `rustle-fs` local backend, `rustle-app` UI).
- Primary target: Linux (the user's Arch system). Code stays
  cross-platform; Windows is best-effort. *(my call)*
- iced's `window::Settings::blur` works on Linux (compositors that support it,
  e.g. KDE, Hyprland) and macOS, not Windows, and only takes effect when the
  window is created → toggling blur needs a restart. Transparency (opacity)
  can change live.
- iCloud through rclone relies on an unofficial protocol and may break when
  Apple changes things. Accepted.

## Approaches considered (cloud)

| | Approach | Trade-off |
|---|---|---|
| **A ✔** | **rclone in the background**: rclone handles sign-in; each account is *mounted* with a local cache, so it is just a folder to Rustle | Needs rclone (+ FUSE on Linux / WinFsp on Windows) installed. Covers ~40 providers incl. iCloud. Uses rclone's already-verified Google client, so no Google security review. Every other feature works on cloud for free. |
| B | Native Rust client per provider (own OAuth, REST, file-watch + upload) | No external tool, full control; but app registration + Google's paid review, a module per provider, our own sync/conflict logic, and no iCloud. |
| C | Hybrid: rclone now, native later | Most total work; only worth it if A disappoints. |

**Picked: A** (user's choice), because it is the only path to sign-in +
tree + auto-sync + iCloud without months of work.

Other parts have one sensible approach in iced and are described below.

## Design

### Architecture

```
rustle-core   Entry, FsError, FileSystem trait (+ new ops), sort/format helpers,
              Settings model (serde)
rustle-fs     LocalFs (list/open/trash/copy/move/rename/mkdir),
              places: user dirs + drives
rustle-cloud  NEW — rclone wrapper: detect, list accounts, sign in, mount,
              unmount, status
rustle-app    UI modules: theme, settings, tabs, sidebar, file_list,
              context_menu
```

Cloud accounts appear as mounted folders, so they go through `LocalFs` like
everything else. `rustle-cloud` only manages rclone processes.

### 1. Look

- Custom `iced::Theme` built from a palette *(my call — "pick what you want")*:
  - **Dark (default):** background `#16181D`, surface `#1E2128`,
    raised `#262A33`, text `#E6E8EE`, muted text `#8B91A0`,
    **accent copper `#E07A3F`** (a nod to "Rust"), danger `#E5534B`.
  - **Light:** background `#F6F7F9`, surface `#FFFFFF`, raised `#ECEEF2`,
    text `#1C1F26`, muted `#6B7280`, same accent.
- Flat style: 6 px corner radius, no heavy borders, hover = raised surface,
  selection = accent at ~20 % opacity. Row height ~28 px. Emoji icons stay
  for now (📁 📄 ☁ 💽 ★).
- Layout:

```
┌──────────────────────────────────────────────────────────────┐
│ [Home] [Downloads ×] [~/src/rustle ×] [+]                    │  tabs
├──────────────────────────────────────────────────────────────┤
│ ← → ↑  │ /home/me/src/rustle                       │ ⚙       │  toolbar
├────────────┬─────────────────────────────────────────────────┤
│ ★ Favorites│ Name                                 Size       │
│   ▸ Projects│ 📁 crates                                      │
│ 🏠 Home    │ 📁 docs                                          │
│   ▸ Desktop│ 📄 Cargo.toml                        412 B      │
│   ▾ src    │ 📄 README.md                         1.2 KiB    │
│     ▾ rustle← current folder highlighted                     │
│ 💽 Drives  │                                                 │
│   ▸ /       │                                                │
│ ☁ Cloud    │                                                 │
│   ▸ gdrive │                                                 │
│   + Add…   │                                                 │
├────────────┴─────────────────────────────────────────────────┤
│ 24 items · 3 hidden                          ⟳ uploading 2   │  status
└──────────────────────────────────────────────────────────────┘
```

### 2. Settings

Opened with ⚙; shown as a page replacing the file area (not a separate
window). Stored at `dirs::config_dir()/rustle/settings.toml` (serde + toml).

- Theme: Dark / Light *(no "follow system" this round — my call, avoids
  relying on desktop detection)*.
- Window opacity: slider 60–100 % (live; applies the background alpha via
  `application(...).style`, with `transparent(true)` when < 100 %).
- Blur behind window: toggle (Linux/macOS; needs restart — the label says so;
  hidden on Windows).
- Show hidden files (default off) — replaces the current header button.
- Favorites list (also editable via right-click).
- Cloud: rclone status, list of accounts, Add / Disconnect.

### 3. Sidebar

- Sections in order: **Favorites**, **Home** (user folders from `dirs`:
  Desktop, Downloads, Documents, Pictures, Music, Videos — only those that
  exist), **Drives** (mounted filesystems via the `sysinfo` crate, showing
  name + free space; pseudo filesystems filtered out), **Cloud**.
- Every folder node has ▸/▾. Expanding loads its **subfolders only**, in the
  background, the first time; results are cached until refreshed.
- **Follows the current folder** *(my reading of "current folder destination
  in the sidebar")*: when the active tab navigates, the tree expands the
  path to that folder (from the closest root it lives under) and highlights
  it. If the folder lives under no root, nothing is highlighted.
- Click = navigate active tab. Middle-click = open in new tab.
- Fixed width ~220 px *(resizable later)*.

### 4. Tabs

- Each tab owns: current folder, entries, loading flag, request counter
  (current stale-result guard), selection, back/forward history.
- Tab title = folder name (`/` for root, account name for cloud root).
- Ctrl+T new tab (at Home), Ctrl+W close (last tab can't be closed),
  Ctrl+Tab / Ctrl+Shift+Tab switch, middle-click tab to close, `+` button.
- Toolbar: Back / Forward / Up *(back/forward is my addition; cheap with
  per-tab history)*, location bar showing the path (editable: type a path +
  Enter).
- Open tabs are not restored on restart *(my call; easy later)*.

### 5. Right-click menu

An overlay drawn at the cursor (`mouse_area::on_right_press` + `stack!`),
closed by Esc, clicking elsewhere, or choosing an item.

- **On a file:** Open · Cut · Copy · Rename · Move to Trash · Copy path
- **On a folder:** Open · Open in new tab · Add to Favorites · Cut · Copy ·
  Rename · Move to Trash · Copy path
- **On empty space:** New folder · Paste · Refresh · Show/Hide hidden files
- **On a sidebar favorite:** Open in new tab · Remove from Favorites
- **On a cloud account:** Open in new tab · Disconnect

Clicking a row selects it (single selection this round). Rename and New
folder use an inline text field in the row. Cut/Copy store paths in an
app-wide clipboard; Paste copies/moves in the background (recursive for
folders; a name clash gets a " (2)" suffix). Trash uses the `trash` crate.
All ops are on the `FileSystem` trait so cloud mounts get them too.

### 6. Cloud (rclone)

- **Detection:** on start, run `rclone version`. If missing, the Cloud
  section shows "Install rclone to add cloud accounts" with a link; nothing
  else breaks.
- **Accounts** = rclone remotes (`rclone listremotes`). Rustle uses the
  user's normal rclone config, so remotes made in a terminal show up too.
- **Add account:** a small dialog — pick provider (Google Drive, Dropbox,
  OneDrive, iCloud Drive, "Other (configure in terminal)"), enter a name →
  Rustle runs `rclone config create <name> <type>`, which opens the browser
  for sign-in. iCloud needs Apple ID + password + 2FA code — those are typed
  into **rclone's own prompt in a terminal** that Rustle opens, never into
  Rustle. Terminal choice *(my call)*: `$TERMINAL` if set, else the first
  found of kitty, alacritty, foot, konsole, gnome-terminal, xterm; if none,
  show the exact command to run.
- Rustle never stores or sees cloud passwords or tokens — rclone's config
  holds them.
- **Mount:** expanding/clicking an account mounts it at
  `dirs::data_dir()/rustle/mounts/<name>` with
  `rclone mount <name>: <dir> --vfs-cache-mode full`. Edits in any app go to
  the local cache and rclone uploads them automatically.
- **Mounts outlive Rustle:** mount processes are started detached and are
  **not** stopped when Rustle quits, so pending uploads finish. On start
  Rustle reuses an existing mount (detected by the mountpoint being in use).
  "Disconnect" in the menu unmounts (`fusermount3 -u` on Linux) — and warns
  if uploads are still pending.
- **Status:** each mount is started with rclone's remote-control API
  enabled (`--rc --rc-addr 127.0.0.1:<port> --rc-no-auth`, a port picked per
  mount); Rustle polls `vfs/stats` every few seconds to show
  "⟳ uploading N" in the status bar.
- Linux needs FUSE (`fuse3`); the "rclone missing" hint mentions it.
- A mount that fails (offline, expired sign-in) shows the rclone error on
  the account node with a "Retry" item; it never blocks local browsing.

### 7. View menu & icons (added 2026-11-24)

- **File and folder icons:** from **Material Icon Theme** (MIT, github.com/material-extensions/vscode-material-icon-theme). They're colorful, one per file type, and special folders (src, docs, Downloads, .git…) get their own icons. Only the icons we use are bundled, in `crates/rustle-app/assets/icons/material/`, with the theme's LICENSE.
  - The theme generates its plain `folder.svg` and `file.svg` at build time, so Rustle ships its own two in the same style. Plain folders are copper `#E07A3F` *(my call)*; plain files are `#90A4AE`, as in the theme.
  - The user may draw their own icons later and swap them in.
- **Button icons:** **Lucide** (ISC), single color and tinted to the theme, in `assets/icons/lucide/`.
- **File kind** (core): Folder, Image, Audio, Video, Archive, Code, Document, Text, App, Other, chosen by extension. It's used for the Type column, "sort by type" and the fallback icon.
- **View button** in the toolbar opens a menu:
  - **Layout:** Details (Name · Size · Date modified · Type) / List / Icons (fluid grid).
  - **Icon size:** Small / Medium / Large / Extra large. Ctrl + mouse wheel zooms, and Ctrl+1/2/3 switches the layout.
  - **Sort by:** Name / Size / Date modified / Type, ascending or descending. Clicking a column header sorts too, and folders always stay on top.
  - **Picture previews:** thumbnails made in the background with the `image` crate and cached in `~/.cache/rustle/thumbs`, keyed by path + size + modified time. Only images under 50 MB (PNG, JPEG, GIF, WebP, BMP) get one.
- The view is global, saved in Settings *(my call; per-folder views can come later)*.

## Build order

Each step is usable on its own and gets merged before the next:

1. Theme + layout shell + settings file/page (incl. opacity, blur).
2. Tabs (moves current per-window state into `Tab`).
3. Sidebar tree (places, drives, lazy expansion, follow current folder).
4. Selection + right-click menu + file ops (re-adds `trash`).
5. Cloud via rclone.

## Left to the builder

- Exact widget composition, module boundaries inside `rustle-app`.
- Keyboard shortcuts beyond those listed.
- How drives are filtered (which filesystem types count as "pseudo").
- Polling interval for upload status (a few seconds).
- Tests: model/logic (sorting, history, clipboard paste naming, tree path
  expansion, settings round-trip) get unit tests; UI is checked by running it.
