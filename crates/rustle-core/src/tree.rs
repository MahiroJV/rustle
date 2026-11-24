//! Helper for the sidebar tree: which nodes to open so the current folder
//! becomes visible (like Explorer's "expand to open folder").

use std::path::{Path, PathBuf};

/// Of the given `roots`, find the **deepest** one that contains `target`,
/// and list every folder from just below that root down to `target`.
///
/// Example: roots `/` and `/home/me`, target `/home/me/src/rustle`
/// → root 1 (`/home/me`), chain `[/home/me/src, /home/me/src/rustle]`.
///
/// Returns `None` if no root contains `target`. If `target` *is* a root, the
/// chain is empty. Paths are compared by whole folder names, so
/// `/home/meow` is not inside `/home/me`.
pub fn expand_chain(roots: &[PathBuf], target: &Path) -> Option<(usize, Vec<PathBuf>)> {
    let (index, root) = roots
        .iter()
        .enumerate()
        .filter(|(_, root)| target.starts_with(root))
        .max_by_key(|(_, root)| root.components().count())?;

    let rest = target.strip_prefix(root).ok()?;
    let mut current = root.clone();
    let chain = rest
        .components()
        .map(|part| {
            current.push(part);
            current.clone()
        })
        .collect();
    Some((index, chain))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_deepest_root() {
        let roots = vec![PathBuf::from("/"), PathBuf::from("/home/me")];
        assert_eq!(
            expand_chain(&roots, Path::new("/home/me/src/rustle")),
            Some((1, vec!["/home/me/src".into(), "/home/me/src/rustle".into()]))
        );
    }

    #[test]
    fn root_itself_gives_empty_chain() {
        assert_eq!(
            expand_chain(&[PathBuf::from("/home/me")], Path::new("/home/me")),
            Some((0, vec![]))
        );
    }

    #[test]
    fn component_not_string_prefix() {
        assert_eq!(
            expand_chain(&[PathBuf::from("/home/me")], Path::new("/home/meow")),
            None
        );
    }

    #[test]
    fn outside_every_root() {
        assert_eq!(
            expand_chain(&[PathBuf::from("/home/me")], Path::new("/etc")),
            None
        );
    }
}
