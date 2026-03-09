//! Persistent cache for last-failed test tracking.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

const CACHE_DIR: &str = ".fastest_cache";
const LASTFAILED_FILE: &str = "lastfailed";
const STEPWISE_FILE: &str = "stepwise";

/// Load the set of test IDs that failed in the last run.
pub fn load_lastfailed(root: &Path) -> HashSet<String> {
    let path = root.join(CACHE_DIR).join(LASTFAILED_FILE);
    match fs::read_to_string(&path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
        Err(_) => HashSet::new(),
    }
}

/// Save the set of failed test IDs to the cache file.
pub fn save_lastfailed(root: &Path, failed: &HashSet<String>) {
    let cache_dir = root.join(CACHE_DIR);
    let _ = fs::create_dir_all(&cache_dir);
    let path = cache_dir.join(LASTFAILED_FILE);
    if let Ok(json) = serde_json::to_string_pretty(failed) {
        let _ = fs::write(&path, json);
    }
}

/// Load the stepwise marker: the test ID where we stopped last time.
pub fn load_stepwise(root: &Path) -> Option<String> {
    let path = root.join(CACHE_DIR).join(STEPWISE_FILE);
    fs::read_to_string(&path).ok().and_then(|s| {
        let trimmed = s.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

/// Save the stepwise marker: the test ID where we stopped.
pub fn save_stepwise(root: &Path, test_id: &str) {
    let cache_dir = root.join(CACHE_DIR);
    let _ = fs::create_dir_all(&cache_dir);
    let path = cache_dir.join(STEPWISE_FILE);
    let _ = fs::write(&path, test_id);
}

/// Clear the stepwise marker (called when all tests pass).
pub fn clear_stepwise(root: &Path) {
    let path = root.join(CACHE_DIR).join(STEPWISE_FILE);
    let _ = fs::remove_file(&path);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_load_lastfailed_no_file() {
        let dir = tempfile::tempdir().unwrap();
        let result = load_lastfailed(dir.path());
        assert!(result.is_empty());
    }

    #[test]
    fn test_save_and_load_lastfailed() {
        let dir = tempfile::tempdir().unwrap();
        let mut failed = HashSet::new();
        failed.insert("tests/test_a.py::test_one".to_string());
        failed.insert("tests/test_b.py::test_two".to_string());

        save_lastfailed(dir.path(), &failed);
        let loaded = load_lastfailed(dir.path());

        assert_eq!(loaded, failed);
    }

    #[test]
    fn test_save_empty_set() {
        let dir = tempfile::tempdir().unwrap();
        let failed = HashSet::new();
        save_lastfailed(dir.path(), &failed);
        let loaded = load_lastfailed(dir.path());
        assert!(loaded.is_empty());
    }

    #[test]
    fn test_load_corrupted_file() {
        let dir = tempfile::tempdir().unwrap();
        let cache_dir = dir.path().join(CACHE_DIR);
        fs::create_dir_all(&cache_dir).unwrap();
        let path = cache_dir.join(LASTFAILED_FILE);
        fs::write(&path, "not valid json!!!").unwrap();

        let result = load_lastfailed(dir.path());
        assert!(result.is_empty());
    }

    #[test]
    fn test_stepwise_save_and_load() {
        let dir = tempfile::tempdir().unwrap();

        // Initially no stepwise marker
        assert!(load_stepwise(dir.path()).is_none());

        // Save a marker
        save_stepwise(dir.path(), "tests/test_a.py::test_fail");
        let loaded = load_stepwise(dir.path());
        assert_eq!(loaded.as_deref(), Some("tests/test_a.py::test_fail"));

        // Clear the marker
        clear_stepwise(dir.path());
        assert!(load_stepwise(dir.path()).is_none());
    }

    #[test]
    fn test_stepwise_overwrite() {
        let dir = tempfile::tempdir().unwrap();

        save_stepwise(dir.path(), "tests/test_a.py::test_one");
        save_stepwise(dir.path(), "tests/test_b.py::test_two");

        let loaded = load_stepwise(dir.path());
        assert_eq!(loaded.as_deref(), Some("tests/test_b.py::test_two"));
    }
}
