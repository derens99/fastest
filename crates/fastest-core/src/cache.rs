use crate::discovery::TestItem;
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Serialize, Deserialize)]
pub struct CacheEntry {
    pub tests: Vec<TestItem>,
    pub last_modified: SystemTime,
    pub file_hash: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiscoveryCache {
    entries: HashMap<PathBuf, CacheEntry>,
    cache_version: String,
}

impl DiscoveryCache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            cache_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// Load cache from disk
    pub fn load(cache_path: &Path) -> Result<Self> {
        if !cache_path.exists() {
            return Ok(Self::new());
        }

        let content = std::fs::read_to_string(cache_path)?;
        let cache: Self = serde_json::from_str(&content)?;

        // Check version compatibility
        if cache.cache_version != env!("CARGO_PKG_VERSION") {
            return Ok(Self::new());
        }

        Ok(cache)
    }

    /// Save cache to disk
    pub fn save(&self, cache_path: &Path) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        if let Some(parent) = cache_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(cache_path, content)?;
        Ok(())
    }

    /// Get cached tests for a file if still valid
    pub fn get(&self, path: &Path) -> Option<&Vec<TestItem>> {
        let entry = self.entries.get(path)?;

        // Check if file has been modified
        if let Ok(metadata) = path.metadata() {
            if let Ok(modified) = metadata.modified() {
                if modified <= entry.last_modified {
                    return Some(&entry.tests);
                }
            }
        }

        None
    }

    /// Update cache entry for a file
    pub fn update(&mut self, path: PathBuf, tests: Vec<TestItem>) -> Result<()> {
        let metadata = path.metadata()?;
        let last_modified = metadata.modified()?;

        self.entries.insert(
            path,
            CacheEntry {
                tests,
                last_modified,
                file_hash: None, // Could add content hashing for extra safety
            },
        );

        Ok(())
    }

    /// Clear all cache entries
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Remove stale entries (files that no longer exist)
    pub fn cleanup(&mut self) {
        self.entries.retain(|path, _| path.exists());
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
