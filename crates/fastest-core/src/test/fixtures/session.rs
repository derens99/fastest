/*!
Session-scoped fixture management for Fastest (Core)

This module provides basic session-level fixture caching using standard library types.
Advanced session management is available in fastest-execution.
*/

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use super::FixtureScope;

/// Simple fixture value for core functionality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixtureValue {
    pub data: String,
    pub created_at: SystemTime,
}

/// Session fixture manager with basic functionality
#[derive(Debug)]
pub struct SessionFixtureManager {
    fixtures: Arc<Mutex<HashMap<String, FixtureValue>>>,
}

impl SessionFixtureManager {
    pub fn new() -> Self {
        Self {
            fixtures: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn get_fixture(&self, name: &str) -> Option<FixtureValue> {
        self.fixtures.lock().unwrap().get(name).cloned()
    }

    pub fn set_fixture(&self, name: String, value: FixtureValue) {
        self.fixtures.lock().unwrap().insert(name, value);
    }

    pub fn cleanup(&self) {
        self.fixtures.lock().unwrap().clear();
    }
}

impl Default for SessionFixtureManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Session fixture definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionFixture {
    pub name: String,
    pub scope: FixtureScope,
    pub setup_code: String,
    pub teardown_code: Option<String>,
}

/// Session statistics (simplified for core)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStats {
    pub fixtures_created: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub total_setup_time: Duration,
}

impl Default for SessionStats {
    fn default() -> Self {
        Self {
            fixtures_created: 0,
            cache_hits: 0,
            cache_misses: 0,
            total_setup_time: Duration::new(0, 0),
        }
    }
}
