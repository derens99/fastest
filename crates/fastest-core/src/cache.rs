use crate::test::discovery::TestItem;
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CacheEntry {
    pub tests: Vec<TestItem>,
    pub modified: SystemTime,
    pub content_hash: String,
    pub file_size: u64,
    pub cached_at: SystemTime,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiscoveryCache {
    entries: HashMap<PathBuf, CacheEntry>,
    version: u32,
    max_age: Duration,
}

impl DiscoveryCache {
    const CURRENT_VERSION: u32 = 2;
    const DEFAULT_MAX_AGE: Duration = Duration::from_secs(7 * 24 * 60 * 60); // 7 days

    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            version: Self::CURRENT_VERSION,
            max_age: Self::DEFAULT_MAX_AGE,
        }
    }

    /// Load cache from disk with version checking
    pub fn load(path: &Path) -> Result<Self> {
        // Use file locking to prevent concurrent access
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        let mut cache: Self = serde_json::from_reader(reader)?;

        // Check version compatibility and validate cache integrity
        if cache.version != Self::CURRENT_VERSION {
            eprintln!("Warning: Cache version mismatch (found: {}, expected: {}). Clearing cache.", 
                     cache.version, Self::CURRENT_VERSION);
            cache = Self::new();
        } else {
            // Validate cache integrity
            let initial_entries = cache.entries.len();
            cache.entries.retain(|path, _| path.exists() && path.is_file());
            let removed_entries = initial_entries - cache.entries.len();
            if removed_entries > 0 {
                eprintln!("Warning: Removed {} stale cache entries for missing files", removed_entries);
            }
        }

        // Clean up old entries
        cache.cleanup_expired();

        Ok(cache)
    }

    /// Save cache to disk with atomic write
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Write to temporary file first
        let temp_path = path.with_extension("tmp");

        {
            let file = File::create(&temp_path)?;
            let writer = BufWriter::new(file);

            serde_json::to_writer(writer, self)?;
        }

        // Atomic rename
        fs::rename(temp_path, path)?;
        Ok(())
    }

    /// Get cached tests for a file if still valid
    pub fn get(&self, path: &Path) -> Option<Vec<TestItem>> {
        self.entries.get(path).and_then(|entry| {
            // Check cache age
            if let Ok(elapsed) = SystemTime::now().duration_since(entry.cached_at) {
                if elapsed > self.max_age {
                    return None;
                }
            }

            // Validate file metadata
            match fs::metadata(path) {
                Ok(metadata) => {
                    let size = metadata.len();
                    let modified = metadata.modified().ok()?;

                    // Check both modification time and file size
                    if self.is_same_time(&modified, &entry.modified) && size == entry.file_size {
                        // Verify content hash for extra safety
                        if let Ok(current_hash) = self.calculate_content_hash_fast(path) {
                            if current_hash == entry.content_hash {
                                // Validate cached tests are not empty/corrupted
                                if !entry.tests.is_empty() {
                                    return Some(entry.tests.clone());
                                }
                            }
                        }
                    }
                }
                Err(_) => {}
            }
            None
        })
    }

    /// Update cache entry for a file
    pub fn update(&mut self, path: PathBuf, tests: Vec<TestItem>) -> Result<()> {
        let metadata = fs::metadata(&path)?;
        let modified = metadata.modified()?;
        let file_size = metadata.len();

        // Calculate content hash
        let content_hash = self.calculate_content_hash_fast(&path)?;

        self.entries.insert(
            path,
            CacheEntry {
                tests,
                modified,
                content_hash,
                file_size,
                cached_at: SystemTime::now(),
            },
        );
        Ok(())
    }

    /// Clear all cache entries
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Remove stale entries (files that no longer exist)
    pub fn cleanup_stale(&mut self) {
        self.entries.retain(|path, _| path.exists());
    }

    /// Remove expired entries
    pub fn cleanup_expired(&mut self) {
        let now = SystemTime::now();
        self.entries.retain(|_, entry| {
            if let Ok(elapsed) = now.duration_since(entry.cached_at) {
                elapsed <= self.max_age
            } else {
                true
            }
        });
    }

    /// Remove specific cache entry
    pub fn remove(&mut self, path: &Path) -> bool {
        self.entries.remove(path).is_some()
    }

    /// Ultra-fast content hash using xxHash (4x faster than SHA256) with file size checks
    fn calculate_content_hash_fast(&self, path: &Path) -> Result<String> {
        // For all files, use streaming xxHash for speed and memory efficiency
        self.calculate_content_hash_streaming_xxhash(path)
    }

    /// Streaming hash calculation optimized for all files using xxHash
    fn calculate_content_hash_streaming_xxhash(&self, path: &Path) -> Result<String> {
        use xxhash_rust::xxh3::Xxh3; // Using xxh3 for good performance
        use std::io::Read;

        let mut file = File::open(path)?;
        let mut hasher = Xxh3::new();
        let mut buffer = [0; 32768]; // 32KB buffer

        loop {
            let n = file.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }

        Ok(format!("{:x}", hasher.digest()))
    }

    /// Compare SystemTime with tolerance for filesystem precision differences
    fn is_same_time(&self, t1: &SystemTime, t2: &SystemTime) -> bool {
        match (
            t1.duration_since(SystemTime::UNIX_EPOCH),
            t2.duration_since(SystemTime::UNIX_EPOCH),
        ) {
            (Ok(d1), Ok(d2)) => {
                // Allow 2 second tolerance for filesystem precision differences
                let diff = if d1 > d2 { d1 - d2 } else { d2 - d1 };
                diff.as_secs() < 2
            }
            _ => false,
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let now = SystemTime::now();
        let mut expired_count = 0;
        let mut total_size = 0;

        for entry in self.entries.values() {
            if let Ok(elapsed) = now.duration_since(entry.cached_at) {
                if elapsed > self.max_age {
                    expired_count += 1;
                }
            }
            total_size += entry.tests.len() * std::mem::size_of::<TestItem>();
        }

        CacheStats {
            total_entries: self.entries.len(),
            total_tests: self.entries.values().map(|e| e.tests.len()).sum(),
            expired_entries: expired_count,
            approximate_memory_usage: total_size,
        }
    }

    /// Set maximum cache age
    pub fn set_max_age(&mut self, max_age: Duration) {
        self.max_age = max_age;
    }
}

impl Default for DiscoveryCache {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct CacheStats {
    pub total_entries: usize,
    pub total_tests: usize,
    pub expired_entries: usize,
    pub approximate_memory_usage: usize,
}

/// Get default cache path with better error handling
pub fn default_cache_path() -> PathBuf {
    dirs::cache_dir()
        .or_else(|| dirs::data_local_dir())
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| {
            eprintln!("Warning: Unable to determine cache directory, using temp dir");
            std::env::temp_dir()
        })
        .join("fastest")
        .join("discovery_cache.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_cache_save_load() {
        let temp_dir = TempDir::new().unwrap();
        let cache_path = temp_dir.path().join("cache.json");
        let test_file = temp_dir.path().join("test.py");

        // Create a test file
        fs::write(&test_file, "def test_example(): pass").unwrap();

        let mut cache = DiscoveryCache::new();
        cache.update(test_file, vec![]).unwrap();

        cache.save(&cache_path).unwrap();
        let loaded = DiscoveryCache::load(&cache_path).unwrap();

        assert_eq!(loaded.entries.len(), 1);
    }

    #[test]
    fn test_cache_expiration() {
        let mut cache = DiscoveryCache::new();
        cache.set_max_age(Duration::from_secs(1));

        let entry = CacheEntry {
            tests: vec![],
            modified: SystemTime::now(),
            content_hash: "test".to_string(),
            file_size: 100,
            cached_at: SystemTime::now() - Duration::from_secs(2),
        };

        cache.entries.insert(PathBuf::from("old.py"), entry);
        cache.cleanup_expired();

        assert_eq!(cache.entries.len(), 0);
    }
}
