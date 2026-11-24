//! Back/forward history for one tab, like a web browser.
//!
//! The tab's *current* folder is not stored here, only the folders behind
//! it (`back`) and ahead of it (`forward`).

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct History {
    back: Vec<PathBuf>,
    forward: Vec<PathBuf>,
}

impl History {
    /// We went somewhere new; `from` is the folder we left.
    /// Going somewhere new always clears the forward list.
    pub fn visit(&mut self, from: PathBuf) {
        self.back.push(from);
        self.forward.clear();
    }

    /// Step back. `current` moves onto the forward list.
    pub fn back(&mut self, current: PathBuf) -> Option<PathBuf> {
        let target = self.back.pop()?;
        self.forward.push(current);
        Some(target)
    }

    /// Step forward. `current` moves onto the back list.
    pub fn forward(&mut self, current: PathBuf) -> Option<PathBuf> {
        let target = self.forward.pop()?;
        self.back.push(current);
        Some(target)
    }

    /// Where `back` would go, without changing anything.
    pub fn peek_back(&self) -> Option<&Path> {
        self.back.last().map(PathBuf::as_path)
    }

    /// Where `forward` would go, without changing anything.
    pub fn peek_forward(&self) -> Option<&Path> {
        self.forward.last().map(PathBuf::as_path)
    }

    pub fn can_back(&self) -> bool {
        !self.back.is_empty()
    }

    pub fn can_forward(&self) -> bool {
        !self.forward.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn back_and_forward() {
        let mut h = History::default();
        h.visit("/a".into()); // now at /b
        h.visit("/b".into()); // now at /c
        assert_eq!(h.back("/c".into()), Some("/b".into()));
        assert_eq!(h.back("/b".into()), Some("/a".into()));
        assert_eq!(h.back("/a".into()), None);
        assert_eq!(h.forward("/a".into()), Some("/b".into()));
        h.visit("/b".into()); // a new branch clears forward
        assert!(!h.can_forward());
    }

    #[test]
    fn peek_does_not_move() {
        let mut h = History::default();
        h.visit("/a".into());
        assert_eq!(h.peek_back(), Some(Path::new("/a")));
        assert_eq!(h.peek_back(), Some(Path::new("/a")));
        assert_eq!(h.peek_forward(), None);
    }
}
