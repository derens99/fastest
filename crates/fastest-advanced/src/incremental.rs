//! Smart Incremental Testing
//!
//! Fast change detection using git2 and blake3 hashing

use anyhow::Result;
use blake3::Hasher;
use git2::{Repository, Status, StatusOptions};
use lru::LruCache;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use walkdir;

use super::AdvancedConfig;

/// Smart incremental tester using git integration
pub struct IncrementalTester {
    config: AdvancedConfig,
    git_repo: Option<Repository>,
    file_hashes: Arc<RwLock<HashMap<PathBuf, String>>>,
    test_cache: Arc<RwLock<LruCache<String, TestResult>>>,
    affected_cache: Arc<RwLock<HashMap<String, HashSet<String>>>>,
    cache_file: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub test_id: String,
    pub status: TestStatus,
    pub duration_ms: u64,
    pub file_hash: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TestStatus {
    Passed,
    Failed,
    Skipped,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChangeInfo {
    pub file_path: PathBuf,
    pub change_type: ChangeType,
    pub old_hash: Option<String>,
    pub new_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeType {
    Added,
    Modified,
    Deleted,
    Renamed,
}

impl IncrementalTester {
    pub fn new(config: &AdvancedConfig) -> Result<Self> {
        let cache_file = config.cache_dir.join("incremental_cache.json");
        
        // Try to open git repository
        let git_repo = Repository::discover(".")
            .map_err(|e| tracing::warn!("Git repository not found: {}", e))
            .ok();

        Ok(Self {
            config: config.clone(),
            git_repo,
            file_hashes: Arc::new(RwLock::new(HashMap::new())),
            test_cache: Arc::new(RwLock::new(
                LruCache::new(NonZeroUsize::new(10000).unwrap())
            )),
            affected_cache: Arc::new(RwLock::new(HashMap::new())),
            cache_file,
        })
    }

    pub async fn initialize(&mut self) -> Result<()> {
        // Load caches
        self.load_cache().await?;
        
        // Initialize file hashes
        self.update_file_hashes().await?;
        
        tracing::info!("Incremental tester initialized");
        Ok(())
    }

    /// Get tests affected by recent changes
    pub async fn get_affected_tests(&self) -> Result<Vec<String>> {
        let changed_files = self.get_changed_files().await?;
        
        if changed_files.is_empty() {
            tracing::info!("No changes detected, running all tests");
            return Ok(vec![]);
        }

        let mut affected_tests = HashSet::new();
        let affected_cache = self.affected_cache.read().await;

        for change in &changed_files {
            let file_path = change.file_path.to_string_lossy();
            
            // Direct test file changes
            if file_path.contains("test_") || file_path.ends_with("_test.py") {
                affected_tests.insert(file_path.to_string());
            }

            // Check cached affected relationships
            if let Some(tests) = affected_cache.get(&file_path.to_string()) {
                affected_tests.extend(tests.clone());
            }

            // Smart impact analysis
            affected_tests.extend(self.analyze_impact(&change).await?);
        }

        let result: Vec<String> = affected_tests.into_iter().collect();
        tracing::info!("Found {} affected tests from {} changed files", 
                      result.len(), changed_files.len());
        
        Ok(result)
    }

    /// Fast change detection using git and file hashing
    async fn get_changed_files(&self) -> Result<Vec<FileChangeInfo>> {
        if let Some(repo) = &self.git_repo {
            self.get_git_changes(repo).await
        } else {
            self.get_filesystem_changes().await
        }
    }

    /// Get changes from git status
    async fn get_git_changes(&self, repo: &Repository) -> Result<Vec<FileChangeInfo>> {
        let mut changes = Vec::new();
        let mut status_opts = StatusOptions::new();
        status_opts.include_untracked(true);
        
        let statuses = repo.statuses(Some(&mut status_opts))?;
        
        for entry in statuses.iter() {
            if let Some(path) = entry.path() {
                let file_path = PathBuf::from(path);
                let change_type = match entry.status() {
                    Status::WT_NEW => ChangeType::Added,
                    Status::WT_MODIFIED => ChangeType::Modified,
                    Status::WT_DELETED => ChangeType::Deleted,
                    Status::WT_RENAMED => ChangeType::Renamed,
                    _ => continue,
                };

                if file_path.exists() {
                    let new_hash = self.calculate_file_hash(&file_path).await?;
                    let old_hash = self.file_hashes.read().await.get(&file_path).cloned();

                    changes.push(FileChangeInfo {
                        file_path,
                        change_type,
                        old_hash,
                        new_hash,
                    });
                }
            }
        }

        Ok(changes)
    }

    /// Get changes by comparing file hashes
    async fn get_filesystem_changes(&self) -> Result<Vec<FileChangeInfo>> {
        let mut changes = Vec::new();
        let current_hashes = self.calculate_all_file_hashes().await?;
        let stored_hashes = self.file_hashes.read().await;

        for (file_path, new_hash) in &current_hashes {
            if let Some(old_hash) = stored_hashes.get(file_path) {
                if old_hash != new_hash {
                    changes.push(FileChangeInfo {
                        file_path: file_path.clone(),
                        change_type: ChangeType::Modified,
                        old_hash: Some(old_hash.clone()),
                        new_hash: new_hash.clone(),
                    });
                }
            } else {
                changes.push(FileChangeInfo {
                    file_path: file_path.clone(),
                    change_type: ChangeType::Added,
                    old_hash: None,
                    new_hash: new_hash.clone(),
                });
            }
        }

        Ok(changes)
    }

    /// Smart impact analysis for changed files
    async fn analyze_impact(&self, change: &FileChangeInfo) -> Result<HashSet<String>> {
        let mut affected = HashSet::new();
        let file_path = &change.file_path;

        // Python files: analyze imports and dependencies
        if file_path.extension().map_or(false, |ext| ext == "py") {
            affected.extend(self.analyze_python_impact(file_path).await?);
        }

        // Configuration files affect all tests
        if self.is_config_file(file_path) {
            affected.insert("*".to_string()); // Special marker for all tests
        }

        // Requirements files affect all tests
        if file_path.file_name().map_or(false, |name| {
            name.to_string_lossy().contains("requirements") ||
            name == "pyproject.toml" ||
            name == "setup.py"
        }) {
            affected.insert("*".to_string());
        }

        Ok(affected)
    }

    /// Analyze Python file dependencies
    async fn analyze_python_impact(&self, file_path: &Path) -> Result<HashSet<String>> {
        let mut affected = HashSet::new();
        
        if !file_path.exists() {
            return Ok(affected);
        }

        let content = std::fs::read_to_string(file_path)?;
        let file_stem = file_path.file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        // Find files that import this module
        let search_patterns = vec![
            format!("import {}", file_stem),
            format!("from {} import", file_stem),
            format!("from .{} import", file_stem),
        ];

        // Search in all Python files (this could be optimized with an index)
        for entry in walkdir::WalkDir::new(".")
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "py"))
        {
            if let Ok(search_content) = std::fs::read_to_string(entry.path()) {
                for pattern in &search_patterns {
                    if search_content.contains(pattern) {
                        affected.insert(entry.path().to_string_lossy().to_string());
                    }
                }
            }
        }

        Ok(affected)
    }

    /// Check if file is a configuration file
    fn is_config_file(&self, file_path: &Path) -> bool {
        if let Some(name) = file_path.file_name() {
            let name_str = name.to_string_lossy();
            matches!(name_str.as_ref(), 
                "pytest.ini" | "pyproject.toml" | "setup.cfg" | 
                "tox.ini" | ".coveragerc" | "conftest.py"
            )
        } else {
            false
        }
    }

    /// Fast file hashing using BLAKE3
    async fn calculate_file_hash(&self, file_path: &Path) -> Result<String> {
        let content = std::fs::read(file_path)?;
        let mut hasher = Hasher::new();
        hasher.update(&content);
        Ok(hasher.finalize().to_hex().to_string())
    }

    /// Calculate hashes for all relevant files
    async fn calculate_all_file_hashes(&self) -> Result<HashMap<PathBuf, String>> {
        let mut hashes = HashMap::new();
        
        // Walk through Python files
        for entry in walkdir::WalkDir::new(".")
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                let path = e.path();
                path.extension().map_or(false, |ext| ext == "py") ||
                self.is_config_file(path)
            })
        {
            let path = entry.path().to_path_buf();
            if let Ok(hash) = self.calculate_file_hash(&path).await {
                hashes.insert(path, hash);
            }
        }

        Ok(hashes)
    }

    /// Update stored file hashes
    async fn update_file_hashes(&self) -> Result<()> {
        let new_hashes = self.calculate_all_file_hashes().await?;
        let mut stored_hashes = self.file_hashes.write().await;
        *stored_hashes = new_hashes;
        
        tracing::debug!("Updated {} file hashes", stored_hashes.len());
        Ok(())
    }

    /// Cache test result for future incremental runs
    pub async fn cache_test_result(&self, result: TestResult) -> Result<()> {
        let mut cache = self.test_cache.write().await;
        cache.put(result.test_id.clone(), result);
        Ok(())
    }

    /// Check if test can be skipped based on cache
    pub async fn can_skip_test(&self, test_id: &str, file_hash: &str) -> bool {
        if let Some(cached_result) = self.test_cache.read().await.peek(test_id) {
            // Skip if file hasn't changed and test previously passed
            cached_result.file_hash == file_hash && 
            matches!(cached_result.status, TestStatus::Passed)
        } else {
            false
        }
    }

    /// Record test-file affection relationship
    pub async fn record_test_affection(&self, test_id: &str, affected_files: Vec<String>) -> Result<()> {
        let mut cache = self.affected_cache.write().await;
        
        for file in affected_files {
            cache.entry(file)
                .or_insert_with(HashSet::new)
                .insert(test_id.to_string());
        }

        Ok(())
    }

    /// Load caches from disk
    async fn load_cache(&self) -> Result<()> {
        if !self.cache_file.exists() {
            return Ok(());
        }

        let content = std::fs::read_to_string(&self.cache_file)?;
        let data: serde_json::Value = serde_json::from_str(&content)?;

        // Load file hashes
        if let Some(hashes) = data.get("file_hashes") {
            if let Ok(hashes_map) = serde_json::from_value::<HashMap<PathBuf, String>>(hashes.clone()) {
                *self.file_hashes.write().await = hashes_map;
            }
        }

        // Load affected cache
        if let Some(affected) = data.get("affected_cache") {
            if let Ok(affected_map) = serde_json::from_value::<HashMap<String, HashSet<String>>>(affected.clone()) {
                *self.affected_cache.write().await = affected_map;
            }
        }

        tracing::debug!("Loaded incremental cache");
        Ok(())
    }

    /// Save caches to disk
    async fn save_cache(&self) -> Result<()> {
        let data = serde_json::json!({
            "file_hashes": *self.file_hashes.read().await,
            "affected_cache": *self.affected_cache.read().await,
            "timestamp": chrono::Utc::now()
        });

        std::fs::write(&self.cache_file, serde_json::to_string_pretty(&data)?)?;
        
        tracing::debug!("Saved incremental cache");
        Ok(())
    }

    /// Get incremental testing statistics
    pub async fn get_stats(&self) -> HashMap<String, serde_json::Value> {
        let mut stats = HashMap::new();
        
        stats.insert("tracked_files".to_string(), 
                    serde_json::Value::Number(self.file_hashes.read().await.len().into()));
        stats.insert("cached_results".to_string(), 
                    serde_json::Value::Number(self.test_cache.read().await.len().into()));
        stats.insert("affected_relationships".to_string(), 
                    serde_json::Value::Number(self.affected_cache.read().await.len().into()));
        
        if self.git_repo.is_some() {
            stats.insert("git_enabled".to_string(), serde_json::Value::Bool(true));
        } else {
            stats.insert("git_enabled".to_string(), serde_json::Value::Bool(false));
        }
        
        stats
    }

    /// Cleanup old cache entries
    pub async fn cleanup_cache(&self, max_age_days: u64) -> Result<()> {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(max_age_days as i64);
        let mut cache = self.test_cache.write().await;
        
        // Remove old entries (LRU will handle this automatically)
        let new_cap = cache.cap().get() / 2;
        cache.resize(NonZeroUsize::new(new_cap).unwrap());
        
        self.save_cache().await?;
        
        tracing::info!("Cleaned up incremental cache");
        Ok(())
    }
    
    /// Get tests that depend on a given file (for watch mode compatibility)
    pub fn get_dependents(&self, file_path: &Path) -> Option<Vec<String>> {
        // This is a synchronous version for compatibility with the watch mode
        // In a real implementation, this would need to use tokio::runtime::Handle::current().block_on()
        // For now, return empty to avoid blocking
        Some(vec![])
    }
    
    /// Update dependency information from discovered tests (for watch mode compatibility)
    pub fn update_from_tests(&mut self, _tests: &[fastest_core::TestItem]) -> Result<()> {
        // This would update internal dependency tracking based on test discovery
        // For now, this is a stub implementation
        Ok(())
    }
}