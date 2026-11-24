//! Breadcrumbs: split a path into clickable pieces, like GNOME Files' path
//! bar. `/home/me/src/rustle` becomes  [🏠 Home] / [src] / [rustle].

use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Crumb {
    pub label: String,
    /// Where clicking this crumb goes.
    pub path: PathBuf,
    /// The first crumb is "Home" when the path is inside the home folder.
    pub is_home: bool,
}

pub fn crumbs(path: &Path, home: &Path) -> Vec<Crumb> {
    let mut out = Vec::new();
    let mut acc = PathBuf::new();
    let mut rest = path;

    // Inside home: start with one "Home" crumb instead of / › home › me.
    // `starts_with` compares whole components, so /home/meow is not inside /home/me.
    if !home.as_os_str().is_empty()
        && let Ok(after_home) = path.strip_prefix(home)
    {
        out.push(Crumb {
            label: "Home".into(),
            path: home.to_path_buf(),
            is_home: true,
        });
        acc = home.to_path_buf();
        rest = after_home;
    }

    for component in rest.components() {
        acc.push(component);
        match component {
            // Windows: "C:" is followed by a RootDir; they become one "C:\" crumb.
            Component::Prefix(_) => {}
            Component::RootDir => out.push(Crumb {
                label: acc.display().to_string(),
                path: acc.clone(),
                is_home: false,
            }),
            Component::Normal(name) => out.push(Crumb {
                label: name.to_string_lossy().into_owned(),
                path: acc.clone(),
                is_home: false,
            }),
            // "." and ".." don't appear in the paths we navigate to.
            Component::CurDir | Component::ParentDir => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(path: &str, home: &str) -> Vec<String> {
        crumbs(Path::new(path), Path::new(home))
            .into_iter()
            .map(|c| c.label)
            .collect()
    }

    #[test]
    fn inside_home_starts_with_home() {
        assert_eq!(labels("/home/me/src/rustle", "/home/me"), ["Home", "src", "rustle"]);
        let c = crumbs(Path::new("/home/me/src"), Path::new("/home/me"));
        assert!(c[0].is_home);
        assert_eq!(c[1].path, PathBuf::from("/home/me/src"));
    }

    #[test]
    fn home_itself_is_one_crumb() {
        assert_eq!(labels("/home/me", "/home/me"), ["Home"]);
    }

    #[test]
    fn outside_home_starts_at_root() {
        assert_eq!(labels("/etc/ssh", "/home/me"), ["/", "etc", "ssh"]);
        assert_eq!(labels("/", "/home/me"), ["/"]);
    }

    #[test]
    fn similar_name_is_not_inside_home() {
        assert_eq!(labels("/home/meow", "/home/me"), ["/", "home", "meow"]);
    }

    #[test]
    fn each_crumb_path_is_the_path_up_to_it() {
        let c = crumbs(Path::new("/etc/ssh"), Path::new("/home/me"));
        let paths: Vec<_> = c.iter().map(|c| c.path.clone()).collect();
        assert_eq!(paths, [PathBuf::from("/"), PathBuf::from("/etc"), PathBuf::from("/etc/ssh")]);
    }
}
