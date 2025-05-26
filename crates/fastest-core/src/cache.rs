use crate::discovery::TestItem;
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Serialize, Deserialize)]
pub struct CacheEntry {
    pub tests: Vec<TestItem>,
    pub modified: SystemTime,
    pub content_hash: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiscoveryCache {
    entries: HashMap<PathBuf, CacheEntry>,
}

impl DiscoveryCache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Load cache from disk
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let cache: Self = serde_json::from_str(&content)?;
        Ok(cache)
    }

    /// Save cache to disk
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Get cached tests for a file if still valid
    pub fn get(&self, path: &Path) -> Option<Vec<TestItem>> {
        self.entries.get(path).and_then(|entry| {
            // Check if file has been modified
            match fs::metadata(path) {
                Ok(metadata) => match metadata.modified() {
                    Ok(modified) => {
                        if modified == entry.modified {
                            Some(entry.tests.clone())
                        } else {
                            None
                        }
                    }
                    Err(_) => None,
                },
                Err(_) => None,
            }
        })
    }

    /// Update cache entry for a file
    pub fn update(&mut self, path: PathBuf, tests: Vec<TestItem>) -> Result<()> {
        let metadata = fs::metadata(&path)?;
        let modified = metadata.modified()?;

        // Calculate content hash for more reliable caching
        let content_hash = self.calculate_content_hash(&path).ok();

        self.entries.insert(
            path,
            CacheEntry {
                tests,
                modified,
                content_hash,
            },
        );
        Ok(())
    }

    /// Clear all cache entries
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Remove stale entries (files that no longer exist)
    pub fn remove(&mut self, path: &Path) -> bool {
        self.entries.remove(path).is_some()
    }

    /// Calculate a fast hash of file content for cache invalidation
    fn calculate_content_hash(&self, path: &Path) -> Result<String> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let content = fs::read_to_string(path)?;
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        Ok(format!("{:x}", hasher.finish()))
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            total_entries: self.entries.len(),
            total_tests: self.entries.values().map(|e| e.tests.len()).sum(),
        }
    }
}

#[derive(Debug)]
pub struct CacheStats {
    pub total_entries: usize,
    pub total_tests: usize,
}

/// Get default cache path
pub fn default_cache_path() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("fastest")
        .join("discovery_cache.json")
}
