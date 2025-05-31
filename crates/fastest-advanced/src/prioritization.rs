//! Smart Test Prioritization
//!
//! Fast test ordering using priority queues and machine learning

use anyhow::Result;
use priority_queue::PriorityQueue;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use super::{incremental::TestStatus, AdvancedConfig};

/// Smart test prioritizer using multiple strategies
pub struct TestPrioritizer {
    config: AdvancedConfig,
    test_history: HashMap<String, TestHistory>,
    priority_weights: PriorityWeights,
    cache_file: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestHistory {
    pub test_id: String,
    pub recent_results: Vec<TestExecution>,
    pub average_duration_ms: f64,
    pub failure_rate: f64,
    pub last_execution: chrono::DateTime<chrono::Utc>,
    pub priority_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestExecution {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub status: TestStatus,
    pub duration_ms: u64,
    pub file_modified_time: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorityWeights {
    pub failure_rate_weight: f64,
    pub duration_weight: f64,
    pub recency_weight: f64,
    pub modification_weight: f64,
    pub dependency_weight: f64,
}

impl Default for PriorityWeights {
    fn default() -> Self {
        Self {
            failure_rate_weight: 0.4,  // Prioritize frequently failing tests
            duration_weight: 0.1,      // Slightly prefer faster tests
            recency_weight: 0.2,       // Prefer recently modified files
            modification_weight: 0.2,   // Prefer tests for recently changed files
            dependency_weight: 0.1,     // Consider dependency relationships
        }
    }
}

#[derive(Debug)]
pub struct PrioritizedTest {
    pub test_id: String,
    pub priority_score: f64,
    pub reasons: Vec<String>,
}

impl TestPrioritizer {
    pub fn new(config: &AdvancedConfig) -> Result<Self> {
        let cache_file = config.cache_dir.join("prioritization_cache.json");
        
        Ok(Self {
            config: config.clone(),
            test_history: HashMap::new(),
            priority_weights: PriorityWeights::default(),
            cache_file,
        })
    }

    pub async fn initialize(&mut self) -> Result<()> {
        // Load cached test history
        self.load_cache().await?;
        
        // Apply machine learning to adjust weights
        self.optimize_weights().await?;
        
        tracing::info!("Test prioritizer initialized with {} test histories", 
                      self.test_history.len());
        Ok(())
    }

    /// Prioritize tests using multiple smart strategies
    pub async fn prioritize_tests(&self, test_ids: &[String]) -> Result<Vec<String>> {
        let mut priority_queue = PriorityQueue::new();
        
        for test_id in test_ids {
            let score = self.calculate_priority_score(test_id).await?;
            priority_queue.push(test_id.clone(), (score * 1000.0) as i64); // Scale for integer priority
        }

        // Extract tests in priority order
        let mut prioritized = Vec::new();
        while let Some((test_id, _)) = priority_queue.pop() {
            prioritized.push(test_id);
        }

        tracing::info!("Prioritized {} tests", prioritized.len());
        Ok(prioritized)
    }

    /// Calculate smart priority score for a test
    async fn calculate_priority_score(&self, test_id: &str) -> Result<f64> {
        let mut score = 0.0;
        let mut reasons = Vec::new();

        // Get test history
        let history = self.test_history.get(test_id);

        // 1. Failure rate score (higher for frequently failing tests)
        if let Some(h) = history {
            let failure_score = h.failure_rate * self.priority_weights.failure_rate_weight;
            score += failure_score;
            if h.failure_rate > 0.1 {
                reasons.push(format!("High failure rate: {:.1}%", h.failure_rate * 100.0));
            }
        }

        // 2. Duration score (slightly prefer faster tests)
        if let Some(h) = history {
            let duration_score = (1.0 / (h.average_duration_ms / 1000.0 + 1.0)) 
                * self.priority_weights.duration_weight;
            score += duration_score;
        }

        // 3. Recency score (prefer recently executed tests)
        if let Some(h) = history {
            let hours_since_last = chrono::Utc::now()
                .signed_duration_since(h.last_execution)
                .num_hours() as f64;
            let recency_score = (1.0 / (hours_since_last / 24.0 + 1.0)) 
                * self.priority_weights.recency_weight;
            score += recency_score;
        }

        // 4. File modification score
        let modification_score = self.calculate_modification_score(test_id).await?;
        score += modification_score * self.priority_weights.modification_weight;
        if modification_score > 0.5 {
            reasons.push("Recently modified file".to_string());
        }

        // 5. Critical path score (high-impact tests)
        let critical_score = self.calculate_critical_path_score(test_id).await?;
        score += critical_score * self.priority_weights.dependency_weight;

        Ok(score)
    }

    /// Calculate file modification score
    async fn calculate_modification_score(&self, test_id: &str) -> Result<f64> {
        // Extract file path from test ID
        let file_path = if let Some(pos) = test_id.find("::") {
            &test_id[..pos]
        } else {
            test_id
        };

        if let Ok(metadata) = std::fs::metadata(file_path) {
            if let Ok(modified) = metadata.modified() {
                let modified_time = chrono::DateTime::<chrono::Utc>::from(modified);
                let hours_since_modified = chrono::Utc::now()
                    .signed_duration_since(modified_time)
                    .num_hours() as f64;
                
                // Higher score for recently modified files
                return Ok(1.0 / (hours_since_modified / 24.0 + 1.0));
            }
        }

        Ok(0.0)
    }

    /// Calculate critical path score based on dependencies
    async fn calculate_critical_path_score(&self, test_id: &str) -> Result<f64> {
        // Tests that are dependencies of many other tests get higher priority
        let dependent_count = self.count_dependent_tests(test_id).await?;
        Ok((dependent_count as f64).sqrt() / 10.0) // Normalize
    }

    /// Count how many tests depend on this test
    async fn count_dependent_tests(&self, _test_id: &str) -> Result<usize> {
        // This would integrate with the dependency tracker
        // For now, return a simple heuristic
        Ok(0)
    }

    /// Record test execution for learning
    pub async fn record_execution(&mut self, test_id: &str, execution: TestExecution) -> Result<()> {
        let history = self.test_history.entry(test_id.to_string())
            .or_insert_with(|| TestHistory {
                test_id: test_id.to_string(),
                recent_results: Vec::new(),
                average_duration_ms: 0.0,
                failure_rate: 0.0,
                last_execution: execution.timestamp,
                priority_score: 0.0,
            });

        // Keep only recent executions (last 100)
        history.recent_results.push(execution.clone());
        if history.recent_results.len() > 100 {
            history.recent_results.remove(0);
        }

        // Update statistics
        history.last_execution = execution.timestamp;
        Self::update_statistics(history);

        Ok(())
    }

    /// Update test statistics
    fn update_statistics(history: &mut TestHistory) {
        if history.recent_results.is_empty() {
            return;
        }

        // Calculate average duration
        let total_duration: u64 = history.recent_results.iter()
            .map(|e| e.duration_ms)
            .sum();
        history.average_duration_ms = total_duration as f64 / history.recent_results.len() as f64;

        // Calculate failure rate
        let failures = history.recent_results.iter()
            .filter(|e| matches!(e.status, TestStatus::Failed | TestStatus::Error))
            .count();
        history.failure_rate = failures as f64 / history.recent_results.len() as f64;
    }

    /// Optimize priority weights using machine learning
    async fn optimize_weights(&mut self) -> Result<()> {
        // Simple optimization: analyze historical data to improve weights
        if self.test_history.len() < 10 {
            return Ok(()); // Not enough data
        }

        let mut total_failures = 0;
        let mut total_executions = 0;

        for history in self.test_history.values() {
            for execution in &history.recent_results {
                total_executions += 1;
                if matches!(execution.status, TestStatus::Failed | TestStatus::Error) {
                    total_failures += 1;
                }
            }
        }

        let overall_failure_rate = total_failures as f64 / total_executions as f64;

        // Adjust weights based on failure patterns
        if overall_failure_rate > 0.1 {
            // High failure rate: increase failure weight
            self.priority_weights.failure_rate_weight = 0.5;
            self.priority_weights.recency_weight = 0.3;
        } else {
            // Low failure rate: focus more on recent changes
            self.priority_weights.modification_weight = 0.3;
            self.priority_weights.recency_weight = 0.3;
        }

        tracing::debug!("Optimized priority weights based on {} executions", total_executions);
        Ok(())
    }

    /// Get failed tests for high priority
    pub async fn get_failed_tests(&self) -> Vec<String> {
        self.test_history
            .values()
            .filter(|h| h.failure_rate > 0.0)
            .map(|h| h.test_id.clone())
            .collect()
    }

    /// Get recently modified tests
    pub async fn get_recently_modified_tests(&self, hours: u64) -> Vec<String> {
        let cutoff = chrono::Utc::now() - chrono::Duration::hours(hours as i64);
        
        self.test_history
            .values()
            .filter(|h| h.last_execution > cutoff)
            .map(|h| h.test_id.clone())
            .collect()
    }

    /// Get slow tests for deprioritization
    pub async fn get_slow_tests(&self, threshold_ms: u64) -> Vec<String> {
        self.test_history
            .values()
            .filter(|h| h.average_duration_ms > threshold_ms as f64)
            .map(|h| h.test_id.clone())
            .collect()
    }

    /// Load cached test history
    async fn load_cache(&mut self) -> Result<()> {
        if !self.cache_file.exists() {
            return Ok(());
        }

        let content = std::fs::read_to_string(&self.cache_file)?;
        let data: serde_json::Value = serde_json::from_str(&content)?;

        if let Some(history) = data.get("test_history") {
            if let Ok(history_map) = serde_json::from_value::<HashMap<String, TestHistory>>(history.clone()) {
                self.test_history = history_map;
            }
        }

        if let Some(weights) = data.get("priority_weights") {
            if let Ok(weights_obj) = serde_json::from_value::<PriorityWeights>(weights.clone()) {
                self.priority_weights = weights_obj;
            }
        }

        tracing::debug!("Loaded {} test histories from cache", self.test_history.len());
        Ok(())
    }

    /// Save test history and weights to cache
    async fn save_cache(&self) -> Result<()> {
        let data = serde_json::json!({
            "test_history": self.test_history,
            "priority_weights": self.priority_weights,
            "timestamp": chrono::Utc::now()
        });

        std::fs::write(&self.cache_file, serde_json::to_string_pretty(&data)?)?;
        
        tracing::debug!("Saved {} test histories to cache", self.test_history.len());
        Ok(())
    }

    /// Get prioritization statistics
    pub async fn get_stats(&self) -> HashMap<String, serde_json::Value> {
        let mut stats = HashMap::new();
        
        stats.insert("tracked_tests".to_string(), 
                    serde_json::Value::Number(self.test_history.len().into()));

        let failed_tests = self.get_failed_tests().await;
        stats.insert("failed_tests".to_string(), 
                    serde_json::Value::Number(failed_tests.len().into()));

        let slow_tests = self.get_slow_tests(5000).await; // 5 second threshold
        stats.insert("slow_tests".to_string(), 
                    serde_json::Value::Number(slow_tests.len().into()));

        if !self.test_history.is_empty() {
            let avg_duration: f64 = self.test_history.values()
                .map(|h| h.average_duration_ms)
                .sum::<f64>() / self.test_history.len() as f64;
            stats.insert("average_duration_ms".to_string(), 
                        serde_json::Value::Number(serde_json::Number::from_f64(avg_duration).unwrap()));

            let avg_failure_rate: f64 = self.test_history.values()
                .map(|h| h.failure_rate)
                .sum::<f64>() / self.test_history.len() as f64;
            stats.insert("average_failure_rate".to_string(), 
                        serde_json::Value::Number(serde_json::Number::from_f64(avg_failure_rate).unwrap()));
        }

        stats.insert("priority_weights".to_string(), 
                    serde_json::to_value(&self.priority_weights).unwrap());

        stats
    }

    /// Cleanup old test history
    pub async fn cleanup_history(&mut self, max_age_days: u64) -> Result<()> {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(max_age_days as i64);
        
        self.test_history.retain(|_, history| {
            history.last_execution > cutoff
        });

        self.save_cache().await?;
        
        tracing::info!("Cleaned up test history older than {} days", max_age_days);
        Ok(())
    }
}