//! Persistent cache for last-failed test tracking.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

const CACHE_DIR: &str = ".fastest_cache";
const LASTFAILED_FILE: &str = "lastfailed";

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
}
