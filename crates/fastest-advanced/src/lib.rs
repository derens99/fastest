//! Advanced features for Fastest test runner
//!
//! This crate provides advanced testing features including coverage collection,
//! incremental testing, file watching, and update checking.

pub mod coverage;
pub mod dependencies;
pub mod error;
pub mod incremental;
pub mod phase3;
pub mod prioritization;
pub mod updates;
pub mod watch;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// Re-export main types
pub use coverage::{CoverageReport, SmartCoverage};
pub use incremental::IncrementalTester;
pub use phase3::{Phase3Config, Phase3Manager};
pub use updates::{check_for_updates, UpdateChecker};
pub use watch::TestWatcher;

/// Advanced features configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedConfig {
    /// Enable coverage collection
    pub coverage_enabled: bool,
    /// Coverage report formats
    pub coverage_formats: Vec<CoverageFormat>,
    /// Enable incremental testing
    pub incremental_enabled: bool,
    /// Enable test prioritization
    pub prioritization_enabled: bool,
    /// Enable dependency tracking
    pub dependency_tracking: bool,
    /// Cache directory for advanced features
    pub cache_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoverageFormat {
    Terminal,
    Html,
    Xml,
    Json,
    Lcov,
}

impl Default for AdvancedConfig {
    fn default() -> Self {
        Self {
            coverage_enabled: false,
            coverage_formats: vec![CoverageFormat::Terminal],
            incremental_enabled: false,
            prioritization_enabled: true,
            dependency_tracking: true,
            cache_dir: dirs::cache_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("fastest"),
        }
    }
}

/// Smart advanced features manager
pub struct AdvancedManager {
    #[allow(dead_code)]
    config: AdvancedConfig,
}

impl AdvancedManager {
    pub fn new(config: AdvancedConfig) -> Result<Self> {
        // Ensure cache directory exists
        std::fs::create_dir_all(&config.cache_dir)?;
        Ok(Self { config })
    }

    /// Initialize advanced features
    pub async fn initialize(&mut self) -> Result<()> {
        tracing::info!("Advanced features initialized");
        Ok(())
    }

    /// Get smart test selection
    pub async fn get_smart_test_selection(
        &self,
        all_tests: &[String],
    ) -> Result<SmartTestSelection> {
        Ok(SmartTestSelection {
            incremental_tests: all_tests.to_vec(),
            prioritized_order: all_tests.to_vec(),
            dependency_order: all_tests.to_vec(),
        })
    }
}

/// Smart test selection result
#[derive(Debug, Default)]
pub struct SmartTestSelection {
    pub incremental_tests: Vec<String>,
    pub prioritized_order: Vec<String>,
    pub dependency_order: Vec<String>,
}
