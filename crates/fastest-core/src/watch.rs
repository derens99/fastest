//! Watch mode for automatic test re-runs
//!
//! Monitors Python source files for changes and triggers callbacks when
//! modifications are detected.  Uses debouncing to coalesce rapid edits
//! into a single notification.

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::error::Result;

/// Watches a directory tree for Python file changes and invokes a callback.
pub struct TestWatcher {
    debounce_ms: u64,
}

impl TestWatcher {
    /// Create a new watcher with the given debounce interval in milliseconds.
    pub fn new(debounce_ms: u64) -> Self {
        Self { debounce_ms }
    }

    /// Start watching `path` recursively for `.py` file changes.
    ///
    /// When one or more Python files are modified or created, `on_change` is
    /// called with the list of affected paths.  Changes are debounced: after
    /// the first event, the watcher waits `debounce_ms` before collecting all
    /// accumulated events and invoking the callback.
    ///
    /// This function blocks the calling thread until the underlying watcher
    /// channel is closed (e.g. if the watcher is dropped from another thread).
    pub fn watch<F>(&self, path: &Path, on_change: F) -> Result<()>
    where
        F: FnMut(&[PathBuf]) + Send + 'static,
    {
        self.watch_paths(&[path.to_path_buf()], on_change)
    }

    /// Start watching multiple paths recursively for `.py` file changes.
    pub fn watch_paths<F>(&self, paths: &[PathBuf], mut on_change: F) -> Result<()>
    where
        F: FnMut(&[PathBuf]) + Send + 'static,
    {
        let (tx, rx) = mpsc::channel::<notify::Result<Event>>();

        let mut watcher = RecommendedWatcher::new(tx, Config::default())?;
        for path in paths {
            watcher.watch(path, RecursiveMode::Recursive)?;
        }

        let debounce = Duration::from_millis(self.debounce_ms);

        while let Ok(first) = rx.recv() {
            let mut changed: Vec<PathBuf> = Vec::new();
            Self::collect_py_paths(&first, &mut changed);

            // Drain additional events within the debounce window
            while let Ok(event) = rx.recv_timeout(debounce) {
                Self::collect_py_paths(&event, &mut changed);
            }

            if !changed.is_empty() {
                // Deduplicate
                changed.sort();
                changed.dedup();
                on_change(&changed);
            }
        }

        drop(watcher); // prevent early drop — watcher must outlive the loop
        Ok(())
    }

    /// If `event` is a Modify or Create on `.py` files, append the paths.
    fn collect_py_paths(event: &notify::Result<Event>, out: &mut Vec<PathBuf>) {
        if let Ok(ev) = event {
            match ev.kind {
                EventKind::Modify(_) | EventKind::Create(_) => {
                    for p in &ev.paths {
                        if p.extension().and_then(|e| e.to_str()) == Some("py") {
                            out.push(p.clone());
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_watcher_creation() {
        let watcher = TestWatcher::new(200);
        assert_eq!(watcher.debounce_ms, 200);
    }

    #[test]
    fn test_collect_py_paths_filters_non_python() {
        let event = Ok(Event {
            kind: EventKind::Modify(notify::event::ModifyKind::Data(
                notify::event::DataChange::Content,
            )),
            paths: vec![
                PathBuf::from("tests/test_a.py"),
                PathBuf::from("README.md"),
                PathBuf::from("src/lib.rs"),
                PathBuf::from("tests/test_b.py"),
            ],
            attrs: Default::default(),
        });

        let mut paths = Vec::new();
        TestWatcher::collect_py_paths(&event, &mut paths);

        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0], PathBuf::from("tests/test_a.py"));
        assert_eq!(paths[1], PathBuf::from("tests/test_b.py"));
    }

    #[test]
    fn test_collect_py_paths_ignores_delete_events() {
        let event = Ok(Event {
            kind: EventKind::Remove(notify::event::RemoveKind::File),
            paths: vec![PathBuf::from("tests/test_deleted.py")],
            attrs: Default::default(),
        });

        let mut paths = Vec::new();
        TestWatcher::collect_py_paths(&event, &mut paths);

        assert!(paths.is_empty(), "delete events should be ignored");
    }
}
