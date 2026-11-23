# Rustle

> A fast, modern file manager written in Rust — Linux first, everywhere later.

![Status](https://img.shields.io/badge/status-early%20development-orange)
![Rust](https://img.shields.io/badge/Made%20with-Rust-orange?logo=rust)
![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)
![Platform](https://img.shields.io/badge/platform-Linux-lightgrey?logo=linux)

Rustle lets you *rustle through* your files: browse, search, and organize them
without the UI ever waiting on the disk.

> ⚠️ **Rustle is in early development.** Nothing here is stable yet — expect
> breaking changes, missing features, and rough edges.

---

## Goals

- **Fast** — directory listing and file operations run off the UI thread, so
  huge folders never freeze the window.
- **Keyboard-friendly** — everything reachable without a mouse, but mouse
  users are first-class too.
- **Native feel on Linux** — respects XDG directories, `.desktop` file
  associations, and the system trash.
- **Cross-platform later** — the core is kept separate from the UI and from
  OS-specific code, so Windows, macOS, and mobile can be added without a
  rewrite.

## Roadmap

### MVP (Linux)
- [ ] Browse directories (list view)
- [ ] Open files with the default application
- [ ] Copy, move, rename, delete (to trash)
- [ ] Create files and folders
- [ ] Sidebar with home, XDG folders, and bookmarks
- [ ] Show / hide hidden files
- [ ] Sort by name, size, date, type

### Next
- [ ] Tabs
- [ ] Grid view with thumbnails
- [ ] Fast search inside the current folder
- [ ] File preview panel (text, images)
- [ ] Mounted drives and USB devices
- [ ] Themes (light / dark)
- [ ] Progress dialog for long copy/move operations, with cancel

### Later
- [ ] Dual-pane mode
- [ ] Archives (zip, tar) opened like folders
- [ ] Windows and macOS support
- [ ] Android support

## Building from source

Requirements: [Rust](https://rustup.rs) (stable).

```bash
git clone https://github.com/MahiroJV/rustle.git
cd rustle
cargo build --release
./target/release/rustle
```

System dependencies for the GUI will be listed here once the UI toolkit is
finalized.

## Project structure (planned)

```
rustle/
├── crates/
│   ├── rustle-core/   # file operations, directory model — no UI, no OS specifics
│   ├── rustle-fs/     # platform backends (Linux first: XDG, trash, mounts)
│   └── rustle-app/    # the GUI
└── README.md
```

Keeping the core independent of the UI and the platform is what makes
"all devices later" realistic.

## Contributing

The project is very young, so ideas and bug reports are the most useful thing
right now — open an [issue](https://github.com/MahiroJV/rustle/issues).
Pull requests are welcome too; for anything big, open an issue first so we can
talk about it.

## License

[MIT](LICENSE) © MahiroJV
