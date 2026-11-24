//! Thumbnails kept in memory, plus the queue of thumbnails still to make.
//!
//! - At most `MAX_JOBS` thumbnails are made at the same time, so opening a
//!   folder with 2,000 photos doesn't start 2,000 threads.
//! - At most `CAPACITY` thumbnails stay in memory; when full, the oldest
//!   one is dropped (it can be loaded again quickly from the disk cache).

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};

use iced::widget::image;

pub const MAX_JOBS: usize = 4;
pub const CAPACITY: usize = 500;

#[derive(Debug, Clone)]
enum State {
    /// A job is making it right now.
    Loading,
    Ready(image::Handle),
    /// Couldn't be made (broken file…): don't try again, show the icon.
    Failed,
}

#[derive(Debug, Default)]
pub struct Thumbs {
    states: HashMap<PathBuf, State>,
    /// Finished thumbnails, oldest first: the front is dropped when full.
    finished: VecDeque<PathBuf>,
    /// Waiting for a free job slot.
    queue: VecDeque<PathBuf>,
    running: usize,
}

impl Thumbs {
    /// The thumbnail for `path`, if it's ready.
    pub fn get(&self, path: &Path) -> Option<&image::Handle> {
        match self.states.get(path) {
            Some(State::Ready(handle)) => Some(handle),
            _ => None,
        }
    }

    /// These are the pictures we want now (e.g. the open folder). Replaces the
    /// waiting queue, so pictures of a folder the user left aren't made.
    /// Pictures already done or being made are skipped.
    pub fn want(&mut self, paths: impl IntoIterator<Item = PathBuf>) {
        self.queue = paths
            .into_iter()
            .filter(|p| !self.states.contains_key(p))
            .collect();
    }

    /// Take paths from the queue while job slots are free. The caller starts
    /// one background job per returned path.
    pub fn next_jobs(&mut self) -> Vec<PathBuf> {
        let mut jobs = Vec::new();
        while self.running < MAX_JOBS {
            let Some(path) = self.queue.pop_front() else { break };
            self.states.insert(path.clone(), State::Loading);
            self.running += 1;
            jobs.push(path);
        }
        jobs
    }

    /// A job ended: `None` means the thumbnail couldn't be made.
    pub fn finished(&mut self, path: PathBuf, handle: Option<image::Handle>) {
        self.running = self.running.saturating_sub(1);
        let state = match handle {
            Some(h) => State::Ready(h),
            None => State::Failed,
        };
        self.states.insert(path.clone(), state);
        self.finished.push_back(path);
        while self.finished.len() > CAPACITY {
            if let Some(oldest) = self.finished.pop_front() {
                self.states.remove(&oldest);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn handle() -> image::Handle {
        image::Handle::from_rgba(1, 1, vec![0u8; 4])
    }

    fn paths(n: usize) -> Vec<PathBuf> {
        (0..n).map(|i| PathBuf::from(format!("/p/{i}.png"))).collect()
    }

    #[test]
    fn at_most_max_jobs_run_together() {
        let mut t = Thumbs::default();
        t.want(paths(10));
        assert_eq!(t.next_jobs().len(), MAX_JOBS);
        assert!(t.next_jobs().is_empty()); // all slots busy
        t.finished("/p/0.png".into(), Some(handle()));
        assert_eq!(t.next_jobs(), vec![PathBuf::from("/p/4.png")]); // one slot freed
    }

    #[test]
    fn ready_and_failed() {
        let mut t = Thumbs::default();
        t.want(paths(2));
        t.next_jobs();
        t.finished("/p/0.png".into(), Some(handle()));
        t.finished("/p/1.png".into(), None);
        assert!(t.get(Path::new("/p/0.png")).is_some());
        assert!(t.get(Path::new("/p/1.png")).is_none());
        // Neither is queued again.
        t.want(paths(2));
        assert!(t.next_jobs().is_empty());
    }

    #[test]
    fn new_wish_replaces_the_old_queue() {
        let mut t = Thumbs::default();
        t.want(paths(10)); // folder A
        t.next_jobs(); // 0..3 running
        t.want(vec![PathBuf::from("/b/x.png")]); // user moved to folder B
        for i in 0..MAX_JOBS {
            t.finished(format!("/p/{i}.png").into(), Some(handle()));
        }
        // Folder A's waiting pictures (4..9) were dropped; only B's remains.
        assert_eq!(t.next_jobs(), vec![PathBuf::from("/b/x.png")]);
    }

    #[test]
    fn oldest_is_dropped_when_full() {
        let mut t = Thumbs::default();
        for p in paths(CAPACITY + 1) {
            t.finished(p, Some(handle()));
        }
        assert!(t.get(Path::new("/p/0.png")).is_none()); // the oldest is gone
        assert!(t.get(Path::new(&format!("/p/{CAPACITY}.png"))).is_some());
    }
}
