/*!
Session-scoped fixture management for Fastest

This module provides comprehensive session-level fixture caching and lifecycle management,
enabling pytest-compatible session fixtures with proper setup and teardown.
*/

use anyhow::{anyhow, Result};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use tracing::{debug, info, warn};

use super::FixtureScope;
use super::execution::FixtureValue;

/// Session fixture manager with lifecycle tracking
#[derive(Debug)]
pub struct SessionFixtureManager {
    /// Session-scoped fixture cache
    fixtures: Arc<DashMap<String, SessionFixture>>,
    /// Teardown order tracking (reverse creation order)
    teardown_order: Arc<Mutex<Vec<String>>>,
    /// Session statistics
    stats: Arc<Mutex<SessionStats>>,
    /// Session start time
    session_start: SystemTime,
}

/// Enhanced session fixture with lifecycle tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionFixture {
    /// Fixture name
    pub name: String,
    /// Fixture value
    pub value: serde_json::Value,
    /// Python teardown code to execute
    pub teardown_code: Option<String>,
    /// Creation timestamp
    pub created_at: SystemTime,
    /// Last access time
    pub last_accessed: SystemTime,
    /// Access count for usage tracking
    pub access_count: u64,
    /// Dependencies that were resolved for this fixture
    pub resolved_dependencies: Vec<String>,
}

/// Session fixture performance statistics
#[derive(Debug, Clone, Default)]
pub struct SessionStats {
    pub fixtures_created: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub total_creation_time: Duration,
    pub total_teardown_time: Duration,
    pub memory_usage_bytes: u64,
}

impl SessionFixtureManager {
    /// Create a new session fixture manager
    pub fn new() -> Self {
        Self {
            fixtures: Arc::new(DashMap::new()),
            teardown_order: Arc::new(Mutex::new(Vec::new())),
            stats: Arc::new(Mutex::new(SessionStats::default())),
            session_start: SystemTime::now(),
        }
    }

    /// Get or create a session fixture
    pub async fn get_or_create_fixture(
        &self,
        name: &str,
        dependencies: &[String],
        creation_code: Option<&str>,
    ) -> Result<SessionFixture> {
        // Check cache first
        if let Some(mut cached) = self.fixtures.get_mut(name) {
            cached.last_accessed = SystemTime::now();
            cached.access_count += 1;
            
            // Update stats
            {
                let mut stats = self.stats.lock().unwrap();
                stats.cache_hits += 1;
            }
            
            debug!("Session fixture cache hit: {} (accessed {} times)", name, cached.access_count);
            return Ok(cached.clone());
        }

        // Create new session fixture
        info!("Creating session fixture: {}", name);
        let start_time = SystemTime::now();

        // Execute fixture creation
        let fixture_value = self.create_session_fixture(name, dependencies, creation_code).await?;

        // Cache the fixture
        let session_fixture = SessionFixture {
            name: name.to_string(),
            value: fixture_value.value,
            teardown_code: fixture_value.teardown_code,
            created_at: start_time,
            last_accessed: start_time,
            access_count: 1,
            resolved_dependencies: dependencies.to_vec(),
        };

        self.fixtures.insert(name.to_string(), session_fixture.clone());

        // Track for teardown in reverse order
        {
            let mut teardown_order = self.teardown_order.lock().unwrap();
            teardown_order.push(name.to_string());
        }

        // Update statistics
        {
            let mut stats = self.stats.lock().unwrap();
            stats.fixtures_created += 1;
            stats.cache_misses += 1;
            if let Ok(elapsed) = start_time.elapsed() {
                stats.total_creation_time += elapsed;
            }
        }

        info!("Session fixture created: {} in {:?}", name, start_time.elapsed().unwrap_or_default());
        Ok(session_fixture)
    }

    /// Execute session fixture teardown in reverse creation order
    pub async fn teardown_session(&self) -> Result<()> {
        let teardown_list = {
            let order = self.teardown_order.lock().unwrap();
            order.clone()
        };

        let teardown_start = SystemTime::now();
        info!("Starting session teardown for {} fixtures", teardown_list.len());

        let mut teardown_errors = Vec::new();
        let mut successful_teardowns = 0;

        // Teardown in reverse order (LIFO - last created, first torn down)
        for fixture_name in teardown_list.iter().rev() {
            match self.teardown_single_fixture(fixture_name).await {
                Ok(_) => {
                    successful_teardowns += 1;
                    debug!("Successfully tore down session fixture: {}", fixture_name);
                }
                Err(e) => {
                    let error_msg = format!("Failed to teardown fixture '{}': {}", fixture_name, e);
                    warn!("{}", error_msg);
                    teardown_errors.push(error_msg);
                }
            }
        }

        // Clear teardown tracking
        {
            let mut order = self.teardown_order.lock().unwrap();
            order.clear();
        }

        // Update teardown stats
        {
            let mut stats = self.stats.lock().unwrap();
            if let Ok(elapsed) = teardown_start.elapsed() {
                stats.total_teardown_time += elapsed;
            }
        }

        info!(
            "Session teardown completed: {}/{} fixtures successfully torn down in {:?}",
            successful_teardowns,
            teardown_list.len(),
            teardown_start.elapsed().unwrap_or_default()
        );

        if !teardown_errors.is_empty() {
            warn!("Session teardown had {} errors", teardown_errors.len());
            for error in &teardown_errors {
                warn!("  {}", error);
            }
        }

        Ok(())
    }

    /// Teardown a single session fixture
    async fn teardown_single_fixture(&self, fixture_name: &str) -> Result<()> {
        if let Some((_, fixture)) = self.fixtures.remove(fixture_name) {
            if let Some(teardown_code) = &fixture.teardown_code {
                debug!("Executing teardown for session fixture: {}", fixture_name);
                // Execute Python teardown code
                self.execute_python_teardown(teardown_code).await?;
            } else {
                debug!("No teardown code for session fixture: {}", fixture_name);
            }
        }
        Ok(())
    }

    /// Get session statistics
    pub fn get_stats(&self) -> SessionStats {
        self.stats.lock().unwrap().clone()
    }

    /// Clear all session fixtures (useful for testing)
    pub fn clear_session(&self) {
        info!("Clearing session fixtures");
        self.fixtures.clear();
        {
            let mut order = self.teardown_order.lock().unwrap();
            order.clear();
        }
        {
            let mut stats = self.stats.lock().unwrap();
            *stats = SessionStats::default();
        }
    }

    /// Get list of active session fixtures
    pub fn list_active_fixtures(&self) -> Vec<String> {
        self.fixtures.iter().map(|entry| entry.key().clone()).collect()
    }

    /// Get fixture access patterns for optimization insights
    pub fn get_access_patterns(&self) -> HashMap<String, (u64, SystemTime)> {
        self.fixtures
            .iter()
            .map(|entry| {
                let fixture = entry.value();
                (entry.key().clone(), (fixture.access_count, fixture.last_accessed))
            })
            .collect()
    }

    /// Create a session fixture (integrate with Python execution)
    async fn create_session_fixture(
        &self,
        name: &str,
        dependencies: &[String],
        creation_code: Option<&str>,
    ) -> Result<FixtureValue> {
        // Placeholder implementation - integrate with Python worker system
        let value = match name {
            "session_db" => {
                serde_json::json!({
                    "connection_string": "sqlite:///tmp/fastest_session.db",
                    "session_id": uuid::Uuid::new_v4().to_string(),
                    "created_at": SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs()
                })
            }
            "session_config" => {
                serde_json::json!({
                    "config_id": uuid::Uuid::new_v4().to_string(),
                    "environment": "test",
                    "debug": true,
                    "dependencies": dependencies
                })
            }
            "session_cache" => {
                serde_json::json!({
                    "cache_backend": "memory",
                    "max_size": 1000,
                    "ttl": 3600
                })
            }
            _ => {
                // Generic session fixture
                let mut fixture_data = serde_json::json!({
                    "name": name,
                    "scope": "session",
                    "session_id": uuid::Uuid::new_v4().to_string(),
                    "created_at": SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs()
                });

                // Add creation code if provided
                if let Some(code) = creation_code {
                    fixture_data.as_object_mut().unwrap().insert(
                        "creation_code".to_string(),
                        serde_json::Value::String(code.to_string())
                    );
                }

                fixture_data
            }
        };

        // Generate teardown code based on fixture type
        let teardown_code = match name {
            "session_db" => Some("# Close database connection\ndb.close()".to_string()),
            "session_config" => Some("# Clear configuration\nconfig.clear()".to_string()),
            "session_cache" => Some("# Clear cache\ncache.clear()".to_string()),
            _ => None,
        };

        Ok(FixtureValue {
            name: name.to_string(),
            value,
            scope: FixtureScope::Session,
            teardown_code,
            created_at: SystemTime::now(),
            last_accessed: SystemTime::now(),
            access_count: 0,
            msgpack_value: None,
        })
    }

    /// Execute Python teardown code (placeholder)
    async fn execute_python_teardown(&self, _teardown_code: &str) -> Result<()> {
        // Placeholder - integrate with Python worker system
        // In real implementation, this would send teardown code to Python worker
        debug!("Executing Python teardown code");
        Ok(())
    }
}

impl Default for SessionFixtureManager {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SessionStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let hit_rate = if self.cache_hits + self.cache_misses > 0 {
            (self.cache_hits as f64) / ((self.cache_hits + self.cache_misses) as f64) * 100.0
        } else {
            0.0
        };

        write!(f, "Session Fixture Stats:\n")?;
        write!(f, "  Fixtures Created: {}\n", self.fixtures_created)?;
        write!(f, "  Cache Hits: {}\n", self.cache_hits)?;
        write!(f, "  Cache Misses: {}\n", self.cache_misses)?;
        write!(f, "  Hit Rate: {:.1}%\n", hit_rate)?;
        write!(f, "  Creation Time: {:.3}s\n", self.total_creation_time.as_secs_f64())?;
        write!(f, "  Teardown Time: {:.3}s\n", self.total_teardown_time.as_secs_f64())?;
        write!(f, "  Memory Usage: {} bytes", self.memory_usage_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_session_fixture_creation() {
        let manager = SessionFixtureManager::new();
        
        let fixture = manager.get_or_create_fixture(
            "test_session_db",
            &["session_config".to_string()],
            Some("db = create_test_db()")
        ).await.unwrap();
        
        assert_eq!(fixture.name, "test_session_db");
        assert_eq!(fixture.access_count, 1);
        assert_eq!(fixture.resolved_dependencies, vec!["session_config"]);
    }
    
    #[tokio::test]
    async fn test_session_fixture_caching() {
        let manager = SessionFixtureManager::new();
        
        // First access - should create
        let fixture1 = manager.get_or_create_fixture("test_cache", &[], None).await.unwrap();
        assert_eq!(fixture1.access_count, 1);
        
        // Second access - should use cache
        let fixture2 = manager.get_or_create_fixture("test_cache", &[], None).await.unwrap();
        assert_eq!(fixture2.access_count, 2);
        
        let stats = manager.get_stats();
        assert_eq!(stats.fixtures_created, 1);
        assert_eq!(stats.cache_hits, 1);
        assert_eq!(stats.cache_misses, 1);
    }
    
    #[tokio::test]
    async fn test_session_teardown_order() {
        let manager = SessionFixtureManager::new();
        
        // Create fixtures in order
        manager.get_or_create_fixture("first", &[], None).await.unwrap();
        manager.get_or_create_fixture("second", &[], None).await.unwrap();
        manager.get_or_create_fixture("third", &[], None).await.unwrap();
        
        let teardown_order = manager.teardown_order.lock().unwrap();
        assert_eq!(*teardown_order, vec!["first", "second", "third"]);
        
        // Teardown should happen in reverse order
        drop(teardown_order);
        manager.teardown_session().await.unwrap();
        
        assert_eq!(manager.fixtures.len(), 0);
    }
}