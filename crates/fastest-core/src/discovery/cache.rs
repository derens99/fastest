//! Discovery cache for avoiding redundant parsing of unchanged test files.
//!
//! Uses xxhash (xxh3_64) for fast content hashing and persists the cache
//! to a JSON file on disk.

use crate::error::Result;
use crate::model::TestItem;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

const CACHE_FILENAME: &str = "discovery_cache.json";

/// Compute a fast 64-bit hash of file content using xxhash.
pub fn hash_content(content: &[u8]) -> u64 {
    xxhash_rust::xxh3::xxh3_64(content)
}

/// Cache entry storing the content hash and parsed test items for a file.
type CacheEntry = (u64, Vec<TestItem>);

/// A cache that maps file paths to their content hash and discovered test items.
///
/// This avoids re-parsing files that have not changed since the last discovery run.
pub struct DiscoveryCache {
    entries: HashMap<PathBuf, CacheEntry>,
    cache_dir: PathBuf,
}

impl DiscoveryCache {
    /// Load a discovery cache from the given directory.
    ///
    /// If no cache file exists, returns an empty cache.
    pub fn load(cache_dir: &Path) -> Self {
        let cache_path = cache_dir.join(CACHE_FILENAME);
        let entries = if cache_path.exists() {
            fs::read_to_string(&cache_path)
                .ok()
                .and_then(|content| serde_json::from_str(&content).ok())
                .unwrap_or_default()
        } else {
            HashMap::new()
        };

        DiscoveryCache {
            entries,
            cache_dir: cache_dir.to_path_buf(),
        }
    }

    /// Look up cached test items for a file path, returning them only if
    /// the content hash matches (i.e., the file has not changed).
    pub fn get(&self, path: &Path, hash: u64) -> Option<&Vec<TestItem>> {
        self.entries.get(path).and_then(|(cached_hash, items)| {
            if *cached_hash == hash {
                Some(items)
            } else {
                None
            }
        })
    }

    /// Insert or update the cache entry for a file path.
    pub fn insert(&mut self, path: PathBuf, hash: u64, items: Vec<TestItem>) {
        self.entries.insert(path, (hash, items));
    }

    /// Persist the cache to disk as a JSON file.
    ///
    /// Creates the cache directory if it does not exist.
    pub fn save(&self) -> Result<()> {
        fs::create_dir_all(&self.cache_dir)?;
        let cache_path = self.cache_dir.join(CACHE_FILENAME);
        let json = serde_json::to_string(&self.entries)?;
        fs::write(&cache_path, json)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_hash_deterministic() {
        let content = b"def test_example():\n    assert True\n";
        let hash1 = hash_content(content);
        let hash2 = hash_content(content);
        assert_eq!(hash1, hash2);

        // Different content should produce a different hash
        let other = b"def test_other():\n    assert False\n";
        let hash3 = hash_content(other);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_cache_hit_miss() {
        let dir = tempfile::tempdir().unwrap();
        let mut cache = DiscoveryCache::load(dir.path());

        let path = PathBuf::from("tests/test_example.py");
        let content = b"def test_one(): pass";
        let hash = hash_content(content);

        // Miss: nothing cached yet
        assert!(cache.get(&path, hash).is_none());

        // Insert a cached entry
        let items = vec![TestItem {
            id: "tests/test_example.py::test_one".to_string(),
            path: path.clone(),
            function_name: "test_one".to_string(),
            line_number: Some(1),
            decorators: vec![],
            is_async: false,
            fixture_deps: vec![],
            class_name: None,
            markers: vec![],
            parameters: None,
            name: "test_one".to_string(),
        }];
        cache.insert(path.clone(), hash, items);

        // Hit: same hash
        let cached = cache.get(&path, hash);
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().len(), 1);
        assert_eq!(cached.unwrap()[0].function_name, "test_one");

        // Miss: different hash (file changed)
        let new_hash = hash_content(b"def test_one(): assert True");
        assert!(cache.get(&path, new_hash).is_none());
    }

    #[test]
    fn test_cache_persistence() {
        let dir = tempfile::tempdir().unwrap();

        // Create and populate a cache
        {
            let mut cache = DiscoveryCache::load(dir.path());
            let path = PathBuf::from("tests/test_persist.py");
            let hash = hash_content(b"test content");
            let items = vec![TestItem {
                id: "tests/test_persist.py::test_persist".to_string(),
                path: path.clone(),
                function_name: "test_persist".to_string(),
                line_number: Some(1),
                decorators: vec![],
                is_async: false,
                fixture_deps: vec![],
                class_name: None,
                markers: vec![],
                parameters: None,
                name: "test_persist".to_string(),
            }];
            cache.insert(path, hash, items);
            cache.save().unwrap();
        }

        // Load the cache from disk and verify
        {
            let cache = DiscoveryCache::load(dir.path());
            let path = PathBuf::from("tests/test_persist.py");
            let hash = hash_content(b"test content");

            let cached = cache.get(&path, hash);
            assert!(cached.is_some());
            assert_eq!(cached.unwrap().len(), 1);
            assert_eq!(cached.unwrap()[0].function_name, "test_persist");
        }
    }
}
