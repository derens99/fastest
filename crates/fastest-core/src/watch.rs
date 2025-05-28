use crate::discovery::{discover_tests, TestItem};
use crate::error::Result;
use crate::incremental::DependencyTracker;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// File watcher for test watch mode
pub struct TestWatcher {
    watcher: RecommendedWatcher,
    receiver: Receiver<notify::Result<Event>>,
    watched_paths: Arc<Mutex<HashSet<PathBuf>>>,
    dependency_tracker: Arc<Mutex<DependencyTracker>>,
}

impl TestWatcher {
    /// Create a new test watcher
    pub fn new() -> Result<Self> {
        let (tx, rx) = channel();

        let watcher = RecommendedWatcher::new(
            move |res| {
                let _ = tx.send(res);
            },
            Config::default().with_poll_interval(Duration::from_millis(100)),
        )
        .map_err(|e| {
            crate::error::Error::Io(std::io::Error::other(format!(
                "Failed to create watcher: {}",
                e
            )))
        })?;

        Ok(Self {
            watcher,
            receiver: rx,
            watched_paths: Arc::new(Mutex::new(HashSet::new())),
            dependency_tracker: Arc::new(Mutex::new(DependencyTracker::new())),
        })
    }

    /// Watch a directory or file
    pub fn watch(&mut self, path: &Path) -> Result<()> {
        self.watcher
            .watch(path, RecursiveMode::Recursive)
            .map_err(|e| {
                crate::error::Error::Io(std::io::Error::other(format!(
                    "Failed to watch path: {}",
                    e
                )))
            })?;

        self.watched_paths
            .lock()
            .unwrap()
            .insert(path.to_path_buf());
        Ok(())
    }

    /// Stop watching a path
    pub fn unwatch(&mut self, path: &Path) -> Result<()> {
        self.watcher.unwatch(path).map_err(|e| {
            crate::error::Error::Io(std::io::Error::other(format!(
                "Failed to unwatch path: {}",
                e
            )))
        })?;

        self.watched_paths.lock().unwrap().remove(path);
        Ok(())
    }

    /// Get changed files since last check
    pub fn get_changed_files(&self) -> Vec<PathBuf> {
        let mut changed_files = Vec::new();

        // Drain all pending events
        while let Ok(event_result) = self.receiver.try_recv() {
            if let Ok(event) = event_result {
                match event.kind {
                    EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                        for path in event.paths {
                            if Self::is_python_file(&path) || Self::is_test_file(&path) {
                                changed_files.push(path);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        // Deduplicate
        changed_files.sort();
        changed_files.dedup();
        changed_files
    }

    /// Get tests affected by changed files
    pub fn get_affected_tests(
        &self,
        changed_files: &[PathBuf],
        all_tests: &[TestItem],
    ) -> Vec<TestItem> {
        let tracker = self.dependency_tracker.lock().unwrap();
        let mut affected_tests = HashSet::new();

        for file in changed_files {
            // If it's a test file, include all tests from that file
            if Self::is_test_file(file) {
                for test in all_tests {
                    if test.path == *file {
                        affected_tests.insert(test.id.clone());
                    }
                }
            }

            // Get tests that depend on this file
            if let Some(dependents) = tracker.get_dependents(file) {
                for test_id in dependents {
                    affected_tests.insert(test_id.clone());
                }
            }
        }

        // Filter tests
        all_tests
            .iter()
            .filter(|test| affected_tests.contains(&test.id))
            .cloned()
            .collect()
    }

    /// Update dependency information after test discovery
    pub fn update_dependencies(&self, tests: &[TestItem]) -> Result<()> {
        let mut tracker = self.dependency_tracker.lock().unwrap();
        tracker.update_from_tests(tests)?;
        Ok(())
    }

    /// Check if a file is a Python file
    fn is_python_file(path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext == "py")
            .unwrap_or(false)
    }

    /// Check if a file is likely a test file
    fn is_test_file(path: &Path) -> bool {
        if !Self::is_python_file(path) {
            return false;
        }

        path.file_name()
            .and_then(|name| name.to_str())
            .map(|name| {
                name.starts_with("test_") || name.ends_with("_test.py") || name == "conftest.py"
            })
            .unwrap_or(false)
    }
}

/// Watch mode controller
pub struct WatchMode {
    watcher: TestWatcher,
    _filter: Option<String>,
    _markers: Option<String>,
}

impl WatchMode {
    /// Create a new watch mode controller
    pub fn new(filter: Option<String>, markers: Option<String>) -> Result<Self> {
        Ok(Self {
            watcher: TestWatcher::new()?,
            _filter: filter,
            _markers: markers,
        })
    }

    /// Start watching directories
    pub fn watch_directories(&mut self, paths: &[PathBuf]) -> Result<()> {
        for path in paths {
            self.watcher.watch(path)?;
        }
        Ok(())
    }

    /// Run watch loop
    pub fn run<F>(&mut self, mut run_tests: F) -> Result<()>
    where
        F: FnMut(&[TestItem]) -> Result<()>,
    {
        println!("🔍 Watch mode started. Press Ctrl+C to exit.");

        // Initial test discovery and run
        let mut all_tests = Vec::new();
        for path in self.watcher.watched_paths.lock().unwrap().iter() {
            let tests = discover_tests(path)?;
            all_tests.extend(tests);
        }

        // Update dependencies
        self.watcher.update_dependencies(&all_tests)?;

        // Run all tests initially
        run_tests(&all_tests)?;

        // Watch loop
        loop {
            std::thread::sleep(Duration::from_millis(100));

            let changed_files = self.watcher.get_changed_files();
            if !changed_files.is_empty() {
                println!("\n📝 Files changed: {:?}", changed_files);

                // Re-discover tests from changed files
                let mut updated_tests = Vec::new();
                for file in &changed_files {
                    if TestWatcher::is_test_file(file) {
                        if let Ok(tests) = discover_tests(file) {
                            updated_tests.extend(tests);
                        }
                    }
                }

                // Get affected tests
                let affected_tests = self.watcher.get_affected_tests(&changed_files, &all_tests);

                if !affected_tests.is_empty() {
                    println!("🧪 Running {} affected tests...", affected_tests.len());
                    run_tests(&affected_tests)?;
                } else if !updated_tests.is_empty() {
                    println!("🧪 Running {} new/updated tests...", updated_tests.len());
                    run_tests(&updated_tests)?;
                } else {
                    println!("ℹ️  No tests affected by changes");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_python_file() {
        assert!(TestWatcher::is_python_file(Path::new("test.py")));
        assert!(TestWatcher::is_python_file(Path::new("module/file.py")));
        assert!(!TestWatcher::is_python_file(Path::new("test.rs")));
        assert!(!TestWatcher::is_python_file(Path::new("README.md")));
    }

    #[test]
    fn test_is_test_file() {
        assert!(TestWatcher::is_test_file(Path::new("test_foo.py")));
        assert!(TestWatcher::is_test_file(Path::new("foo_test.py")));
        assert!(TestWatcher::is_test_file(Path::new("conftest.py")));
        assert!(!TestWatcher::is_test_file(Path::new("foo.py")));
        assert!(!TestWatcher::is_test_file(Path::new("test.txt")));
    }
}
