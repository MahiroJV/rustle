//! Icons. File and folder icons come from Material Icon Theme (colorful);
//! button icons come from Lucide (single color, tinted to the theme).
//! All SVGs are built into the program with `include_bytes!`, so Rustle
//! needs no icon files at runtime. Sources and licenses: assets/icons/README.md

use std::collections::HashMap;
use std::sync::LazyLock;

use iced::widget::{Svg, svg};
use iced::{Color, Theme};
use rustle_core::extension;

/// Builds a list of `("name", <bytes of assets/icons/<dir>/name.svg>)`.
/// A missing file is a compile error, so a typo can't slip through.
macro_rules! bundle {
    ($dir:literal: $($name:literal),* $(,)?) => {
        &[$(($name, include_bytes!(concat!("../assets/icons/", $dir, "/", $name, ".svg")))),*]
    };
}

static MATERIAL: &[(&str, &[u8])] = bundle!("material":
    "file", "folder",
    // files
    "3d", "android", "audio", "c", "certificate", "console", "cpp", "csharp", "css",
    "database", "disc", "docker", "document", "epub", "exe", "font", "git", "go", "html",
    "image", "java", "javascript", "json", "key", "kotlin", "lib", "lock", "log", "lua",
    "makefile", "markdown", "nix", "pdf", "php", "powerpoint", "python", "readme", "ruby",
    "rust", "settings", "svg", "swift", "table", "toml", "typescript", "video", "word",
    "xml", "yaml", "zip",
    // special folders
    "folder-app", "folder-audio", "folder-config", "folder-desktop", "folder-dist",
    "folder-docs", "folder-download", "folder-font", "folder-git", "folder-github",
    "folder-home", "folder-images", "folder-node", "folder-public", "folder-resource",
    "folder-src", "folder-target", "folder-temp", "folder-test", "folder-trash", "folder-video",
);

/// One `svg::Handle` per icon, created the first time it's needed.
/// A handle is cheap to clone (it's shared), and iced caches the rendering.
static HANDLES: LazyLock<HashMap<&'static str, svg::Handle>> = LazyLock::new(|| {
    MATERIAL
        .iter()
        .map(|(name, bytes)| (*name, svg::Handle::from_memory(*bytes)))
        .collect()
});

/// The icon for a file or folder name.
pub fn file_icon(name: &str, is_dir: bool) -> svg::Handle {
    HANDLES[icon_name(name, is_dir)].clone()
}

/// Which Material icon to use. Checks, in order: special folder names,
/// special file names, then the extension; falls back to plain folder/file.
pub fn icon_name(name: &str, is_dir: bool) -> &'static str {
    let lower = name.to_lowercase();
    if is_dir {
        return match lower.as_str() {
            "src" | "source" | "sources" => "folder-src",
            "docs" | "doc" | "documents" | "documentation" => "folder-docs",
            "downloads" | "download" => "folder-download",
            "pictures" | "images" | "img" | "photos" => "folder-images",
            "videos" | "video" | "movies" => "folder-video",
            "music" | "audio" | "sounds" => "folder-audio",
            "desktop" => "folder-desktop",
            ".git" => "folder-git",
            ".github" => "folder-github",
            ".config" | "config" | "configs" | "settings" => "folder-config",
            "public" => "folder-public",
            "test" | "tests" | "__tests__" | "spec" => "folder-test",
            "node_modules" => "folder-node",
            "target" => "folder-target",
            "dist" | "build" | "out" => "folder-dist",
            "assets" | "resources" | "res" => "folder-resource",
            "tmp" | "temp" => "folder-temp",
            "trash" | ".trash" => "folder-trash",
            "app" | "apps" | "applications" => "folder-app",
            "fonts" | ".fonts" => "folder-font",
            _ => "folder",
        };
    }

    match lower.as_str() {
        "cargo.toml" => return "rust",
        "cargo.lock" => return "lock",
        "readme" | "readme.md" | "readme.txt" => return "readme",
        "license" | "license.md" | "license.txt" | "licence" | "copying" => return "certificate",
        "dockerfile" | "docker-compose.yml" | "docker-compose.yaml" | "compose.yaml" => {
            return "docker";
        }
        "makefile" => return "makefile",
        ".gitignore" | ".gitattributes" | ".gitmodules" => return "git",
        _ => {}
    }

    let Some(ext) = extension(&lower) else {
        return "file";
    };
    match ext.as_str() {
        "rs" => "rust",
        "py" => "python",
        "js" | "mjs" | "cjs" | "jsx" => "javascript",
        "ts" | "mts" | "tsx" => "typescript",
        "html" | "htm" => "html",
        "css" | "scss" => "css",
        "json" => "json",
        "md" | "markdown" => "markdown",
        "svg" => "svg",
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "ico" | "tif" | "tiff" | "heic"
        | "avif" => "image",
        "mp3" | "wav" | "flac" | "ogg" | "m4a" | "aac" | "opus" | "wma" => "audio",
        "mp4" | "mkv" | "webm" | "avi" | "mov" | "wmv" | "flv" | "m4v" => "video",
        "zip" | "tar" | "gz" | "tgz" | "bz2" | "xz" | "7z" | "rar" | "zst" => "zip",
        "pdf" => "pdf",
        "doc" | "docx" | "odt" | "rtf" => "word",
        "xls" | "xlsx" | "ods" | "csv" => "table",
        "ppt" | "pptx" | "odp" => "powerpoint",
        "sh" | "bash" | "zsh" | "fish" => "console",
        "ini" | "conf" | "cfg" | "env" => "settings",
        "toml" => "toml",
        "yaml" | "yml" => "yaml",
        "lock" => "lock",
        "c" | "h" => "c",
        "cpp" | "hpp" | "cc" | "cxx" => "cpp",
        "go" => "go",
        "java" | "jar" => "java",
        "exe" | "msi" | "appimage" => "exe",
        "ttf" | "otf" | "woff" | "woff2" => "font",
        "db" | "sqlite" | "sql" => "database",
        "xml" => "xml",
        "pem" | "key" | "pub" | "gpg" => "key",
        "crt" | "cer" => "certificate",
        "log" => "log",
        "lua" => "lua",
        "php" => "php",
        "rb" => "ruby",
        "kt" | "kts" => "kotlin",
        "swift" => "swift",
        "cs" => "csharp",
        "so" | "dll" | "dylib" | "a" => "lib",
        "nix" => "nix",
        "iso" | "img" => "disc",
        "epub" => "epub",
        "obj" | "stl" | "fbx" | "gltf" | "glb" | "blend" => "3d",
        "apk" => "android",
        "txt" => "document",
        _ => "file",
    }
}

/// Button icons (Lucide). Single color, so they can be tinted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ui {
    Back,
    Forward,
    Settings,
    Close,
    Plus,
    Home,
}

/// Same order as the `Ui` enum: `Ui::Back as usize` is index 0, and so on.
static LUCIDE: &[(&str, &[u8])] = bundle!("lucide":
    "arrow-left", "arrow-right", "settings", "x", "plus", "house");

static UI_ICONS: LazyLock<Vec<svg::Handle>> = LazyLock::new(|| {
    LUCIDE
        .iter()
        .map(|(_, bytes)| svg::Handle::from_memory(*bytes))
        .collect()
});

/// A Lucide icon, `size` pixels square, painted in `color`.
pub fn ui<'a>(icon: Ui, color: Color, size: f32) -> Svg<'a, Theme> {
    let handle = UI_ICONS[icon as usize].clone();
    svg(handle)
        .width(size)
        .height(size)
        .style(move |_theme, _status| svg::Style { color: Some(color) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn special_folders() {
        assert_eq!(icon_name("src", true), "folder-src");
        assert_eq!(icon_name("Downloads", true), "folder-download");
        assert_eq!(icon_name(".git", true), "folder-git");
        assert_eq!(icon_name("holiday", true), "folder");
    }

    #[test]
    fn special_file_names_win_over_extension() {
        assert_eq!(icon_name("Cargo.toml", false), "rust");
        assert_eq!(icon_name("README.md", false), "readme");
        assert_eq!(icon_name("LICENSE", false), "certificate");
    }

    #[test]
    fn by_extension_any_case() {
        assert_eq!(icon_name("main.rs", false), "rust");
        assert_eq!(icon_name("Photo.JPG", false), "image");
        assert_eq!(icon_name("backup.tar.gz", false), "zip");
        assert_eq!(icon_name("strange.xyz", false), "file");
        assert_eq!(icon_name(".bashrc", false), "file");
    }

    #[test]
    fn a_folder_with_an_extension_is_still_a_folder() {
        assert_eq!(icon_name("photos.png", true), "folder");
    }

    /// Every name `icon_name` can return must be bundled, or `file_icon`
    /// would panic. Collect all string literals it returns by trying many inputs.
    #[test]
    fn every_returned_icon_is_bundled() {
        let samples = [
            "x.rs", "x.py", "x.js", "x.ts", "x.html", "x.css", "x.json", "x.md", "x.svg",
            "x.png", "x.mp3", "x.mp4", "x.zip", "x.pdf", "x.docx", "x.csv", "x.pptx", "x.sh",
            "x.ini", "x.toml", "x.yaml", "x.lock", "x.c", "x.cpp", "x.go", "x.java", "x.exe",
            "x.ttf", "x.db", "x.xml", "x.pem", "x.crt", "x.log", "x.lua", "x.php", "x.rb",
            "x.kt", "x.swift", "x.cs", "x.so", "x.nix", "x.iso", "x.epub", "x.stl", "x.apk",
            "x.txt", "x", "Cargo.toml", "Cargo.lock", "README", "LICENSE", "Dockerfile",
            "Makefile", ".gitignore",
        ];
        let folders = [
            "src", "docs", "downloads", "pictures", "videos", "music", "desktop", ".git",
            ".github", "config", "public", "tests", "node_modules", "target", "dist",
            "assets", "tmp", "trash", "apps", "fonts", "other",
        ];
        let bundled: Vec<&str> = MATERIAL.iter().map(|(n, _)| *n).collect();
        for name in samples {
            assert!(bundled.contains(&icon_name(name, false)), "{name}");
        }
        for name in folders {
            assert!(bundled.contains(&icon_name(name, true)), "{name}/");
        }
    }

    #[test]
    fn ui_enum_matches_lucide_list() {
        assert_eq!(LUCIDE.len(), Ui::Home as usize + 1);
        assert_eq!(LUCIDE[Ui::Home as usize].0, "house");
        assert_eq!(LUCIDE[Ui::Back as usize].0, "arrow-left");
        assert_eq!(LUCIDE[Ui::Close as usize].0, "x");
    }

    #[test]
    fn bundled_files_are_svgs() {
        for (name, bytes) in MATERIAL {
            assert!(String::from_utf8_lossy(bytes).contains("<svg"), "{name}");
        }
    }
}
