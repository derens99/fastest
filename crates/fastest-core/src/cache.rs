//! High-Performance Discovery Cache
//!
//! Features:
//! - RwLock for read-heavy workloads
//! - Arc<Vec<TestItem>> to avoid cloning
//! - Memory-mapped file hashing for large files
//! - Compression for cache storage
//! - Batch operations to reduce lock contention

use crate::error::Result;
use crate::test::discovery::TestItem;
use crate::utils::simd_json;
use memmap2::Mmap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

/// Shared test list to avoid cloning
type SharedTests = Arc<Vec<TestItem>>;

#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub tests: Option<SharedTests>,
    pub modified: SystemTime,
    pub content_hash: String,
    pub file_size: u64,
    pub cached_at: SystemTime,
}

// Separate struct for serialization
#[derive(Serialize, Deserialize)]
struct SerializedCacheEntry {
    tests: Vec<TestItem>,
    modified: SystemTime,
    content_hash: String,
    file_size: u64,
    cached_at: SystemTime,
}

impl CacheEntry {
    fn new(tests: Vec<TestItem>, modified: SystemTime, content_hash: String, file_size: u64) -> Self {
        Self {
            tests: Some(Arc::new(tests)),
            modified,
            content_hash,
            file_size,
            cached_at: SystemTime::now(),
        }
    }
    
    fn to_serialized(&self) -> SerializedCacheEntry {
        SerializedCacheEntry {
            tests: self.tests.as_ref().map(|arc| arc.as_ref().clone()).unwrap_or_default(),
            modified: self.modified,
            content_hash: self.content_hash.clone(),
            file_size: self.file_size,
            cached_at: self.cached_at,
        }
    }
    
    fn from_serialized(entry: SerializedCacheEntry) -> Self {
        Self {
            tests: Some(Arc::new(entry.tests)),
            modified: entry.modified,
            content_hash: entry.content_hash,
            file_size: entry.file_size,
            cached_at: entry.cached_at,
        }
    }
    
    fn get_tests(&self) -> Option<SharedTests> {
        self.tests.clone()
    }
}

/// Thread-safe discovery cache with RwLock
pub struct DiscoveryCache {
    inner: Arc<RwLock<CacheInner>>,
}

#[derive(Debug)]
struct CacheInner {
    entries: HashMap<PathBuf, CacheEntry>,
    version: u32,
    max_age: Duration,
    pending_updates: Vec<(PathBuf, Vec<TestItem>)>,
}

#[derive(Serialize, Deserialize)]
struct SerializedCacheInner {
    entries: HashMap<PathBuf, SerializedCacheEntry>,
    version: u32,
    max_age: Duration,
}

impl DiscoveryCache {
    const CURRENT_VERSION: u32 = 3; // Bumped for new format
    const DEFAULT_MAX_AGE: Duration = Duration::from_secs(7 * 24 * 60 * 60); // 7 days
    const COMPRESSION_THRESHOLD: usize = 10_240; // Compress cache files > 10KB
    
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(CacheInner {
                entries: HashMap::new(),
                version: Self::CURRENT_VERSION,
                max_age: Self::DEFAULT_MAX_AGE,
                pending_updates: Vec::new(),
            })),
        }
    }
    
    /// Load cache from disk with compression support
    pub fn load(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        let file_size = file.metadata()?.len();
        
        // Read and potentially decompress
        let data = if file_size > Self::COMPRESSION_THRESHOLD as u64 {
            // Try to decompress with zstd
            let mut decoder = zstd::Decoder::new(BufReader::new(file))?;
            let mut buffer = Vec::new();
            decoder.read_to_end(&mut buffer)?;
            buffer
        } else {
            // Read directly
            let mut reader = BufReader::new(file);
            let mut buffer = Vec::new();
            reader.read_to_end(&mut buffer)?;
            buffer
        };
        
        // Parse with SIMD JSON
        let serialized: SerializedCacheInner = simd_json::from_slice(&mut data.clone())
            .map_err(|e| crate::error::Error::Serialization(e.to_string()))?;
        
        // Version check
        if serialized.version != Self::CURRENT_VERSION {
            eprintln!("Cache version mismatch, creating new cache");
            return Ok(Self::new());
        }
        
        // Convert from serialized format
        let mut entries = HashMap::new();
        for (path, serialized_entry) in serialized.entries {
            if path.exists() {
                entries.insert(path, CacheEntry::from_serialized(serialized_entry));
            }
        }
        
        // Clean expired entries
        let now = SystemTime::now();
        entries.retain(|_, entry| {
            if let Ok(elapsed) = now.duration_since(entry.cached_at) {
                elapsed <= serialized.max_age
            } else {
                true
            }
        });
        
        let inner = CacheInner {
            entries,
            version: serialized.version,
            max_age: serialized.max_age,
            pending_updates: Vec::new(),
        };
        
        Ok(Self {
            inner: Arc::new(RwLock::new(inner)),
        })
    }
    
    /// Save cache with compression for large files
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        // Apply pending updates first
        self.flush_pending_updates()?;
        
        let temp_path = path.with_extension("tmp");
        
        {
            let inner = self.inner.read();
            
            // Convert to serialized format
            let mut serialized_entries = HashMap::new();
            for (path, entry) in &inner.entries {
                serialized_entries.insert(path.clone(), entry.to_serialized());
            }
            
            let serializable = SerializedCacheInner {
                entries: serialized_entries,
                version: inner.version,
                max_age: inner.max_age,
            };
            
            // Serialize to JSON
            let json_data = simd_json::to_vec(&serializable)?;
            
            // Compress if large
            if json_data.len() > Self::COMPRESSION_THRESHOLD {
                let file = File::create(&temp_path)?;
                let mut encoder = zstd::Encoder::new(BufWriter::new(file), 3)?;
                encoder.write_all(&json_data)?;
                encoder.finish()?;
            } else {
                fs::write(&temp_path, json_data)?;
            }
        }
        
        // Atomic rename
        fs::rename(temp_path, path)?;
        Ok(())
    }
    
    /// Get cached tests without cloning
    pub fn get(&self, path: &Path) -> Option<SharedTests> {
        let inner = self.inner.read();
        inner.entries.get(path).and_then(|entry| {
            // Quick metadata check
            if let Ok(metadata) = fs::metadata(path) {
                if metadata.len() == entry.file_size {
                    if let Ok(modified) = metadata.modified() {
                        if Self::is_same_time(&modified, &entry.modified) {
                            return entry.get_tests();
                        }
                    }
                }
            }
            None
        })
    }
    
    /// Batch update for better performance
    pub fn update_batch(&self, updates: Vec<(PathBuf, Vec<TestItem>)>) -> Result<()> {
        let mut inner = self.inner.write();
        inner.pending_updates.extend(updates);
        
        // Flush if too many pending
        if inner.pending_updates.len() > 100 {
            drop(inner);
            self.flush_pending_updates()?;
        }
        Ok(())
    }
    
    /// Update single entry
    pub fn update(&self, path: PathBuf, tests: Vec<TestItem>) -> Result<()> {
        self.update_batch(vec![(path, tests)])
    }
    
    /// Flush pending updates
    fn flush_pending_updates(&self) -> Result<()> {
        let mut inner = self.inner.write();
        let updates = std::mem::take(&mut inner.pending_updates);
        
        for (path, tests) in updates {
            if let Ok(metadata) = fs::metadata(&path) {
                let modified = metadata.modified()?;
                let file_size = metadata.len();
                let content_hash = Self::calculate_file_hash(&path)?;
                
                inner.entries.insert(
                    path,
                    CacheEntry::new(tests, modified, content_hash, file_size),
                );
            }
        }
        Ok(())
    }
    
    /// Calculate file hash using memory mapping for large files
    fn calculate_file_hash(path: &Path) -> Result<String> {
        use xxhash_rust::xxh3::Xxh3;
        
        let file_size = fs::metadata(path)?.len();
        
        // Use mmap for files > 1MB
        if file_size > 1_048_576 {
            let file = File::open(path)?;
            let mmap = unsafe { Mmap::map(&file)? };
            let hash = xxhash_rust::xxh3::xxh3_64(&mmap);
            Ok(format!("{:x}", hash))
        } else {
            // Read small files directly
            let data = fs::read(path)?;
            let hash = xxhash_rust::xxh3::xxh3_64(&data);
            Ok(format!("{:x}", hash))
        }
    }
    
    /// Compare times with tolerance
    fn is_same_time(t1: &SystemTime, t2: &SystemTime) -> bool {
        match (
            t1.duration_since(SystemTime::UNIX_EPOCH),
            t2.duration_since(SystemTime::UNIX_EPOCH),
        ) {
            (Ok(d1), Ok(d2)) => {
                let diff = if d1 > d2 { d1 - d2 } else { d2 - d1 };
                diff.as_secs() < 2
            }
            _ => false,
        }
    }
    
    /// Clear cache
    pub fn clear(&self) {
        let mut inner = self.inner.write();
        inner.entries.clear();
        inner.pending_updates.clear();
    }
    
    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let inner = self.inner.read();
        let now = SystemTime::now();
        let mut expired_count = 0;
        let mut total_tests = 0;
        
        for entry in inner.entries.values() {
            if let Ok(elapsed) = now.duration_since(entry.cached_at) {
                if elapsed > inner.max_age {
                    expired_count += 1;
                }
            }
            if let Some(tests) = &entry.tests {
                total_tests += tests.len();
            } else {
                total_tests += entry.tests_vec.len();
            }
        }
        
        CacheStats {
            total_entries: inner.entries.len(),
            total_tests,
            expired_entries: expired_count,
            pending_updates: inner.pending_updates.len(),
        }
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
    pub pending_updates: usize,
}

/// Get default cache path
pub fn default_cache_path() -> PathBuf {
    dirs::cache_dir()
        .or_else(dirs::data_local_dir)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| std::env::temp_dir())
        .join("fastest")
        .join("discovery_cache.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_cache_operations() {
        let cache = DiscoveryCache::new();
        let test_path = PathBuf::from("test.py");
        
        // Create dummy test items
        let tests = vec![
            TestItem {
                id: "test1".to_string(),
                path: test_path.clone(),
                function_name: "test_one".to_string(),
                line_number: Some(1),
                decorators: Default::default(),
                is_async: false,
                fixture_deps: Default::default(),
                class_name: None,
                is_xfail: false,
                name: "test_one".to_string(),
                indirect_params: Default::default(),
            },
        ];
        
        // Update should work
        cache.update(test_path.clone(), tests).unwrap();
        
        // Stats should reflect the update
        let stats = cache.stats();
        assert_eq!(stats.pending_updates, 1);
    }
    
    #[test]
    fn test_cache_compression() {
        let temp_dir = TempDir::new().unwrap();
        let cache_path = temp_dir.path().join("cache.json");
        
        let cache = DiscoveryCache::new();
        
        // Add many entries to trigger compression
        for i in 0..100 {
            let path = PathBuf::from(format!("test{}.py", i));
            let tests = vec![]; // Empty for simplicity
            cache.update(path, tests).unwrap();
        }
        
        cache.save(&cache_path).unwrap();
        
        // Should be able to load compressed cache
        let loaded = DiscoveryCache::load(&cache_path).unwrap();
        assert_eq!(loaded.stats().total_entries, 100);
    }
}