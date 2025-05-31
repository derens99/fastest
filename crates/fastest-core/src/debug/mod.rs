//! Basic debugging support for Fastest (Core)
//!
//! This module provides basic debugging configuration and types.
//! Advanced debugging with test results is available in fastest-execution.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::TestItem;

/// Debug manager for basic debugging support
pub struct DebugManager {
    config: DebugConfig,
}

/// Debug configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugConfig {
    pub enabled: bool,
    pub enhanced_errors: bool,
    pub pdb_enabled: bool,
    pub pdb_on_first_failure: bool,
    pub breakpoints: Vec<Breakpoint>,
    pub debug_output_dir: Option<PathBuf>,
}

impl Default for DebugConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            enhanced_errors: true,
            pdb_enabled: false,
            pdb_on_first_failure: true,
            breakpoints: Vec::new(),
            debug_output_dir: None,
        }
    }
}

/// Breakpoint definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Breakpoint {
    pub file: PathBuf,
    pub line: usize,
    pub condition: Option<String>,
    pub temporary: bool,
}

/// Enhanced error information (simplified for core)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedError {
    pub test_id: String,
    pub error_type: ErrorType,
    pub message: String,
    pub context: HashMap<String, String>,
}

/// Error classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorType {
    AssertionError,
    ImportError,
    SyntaxError,
    RuntimeError,
    TimeoutError,
    Unknown,
}

impl DebugManager {
    pub fn new(config: DebugConfig) -> Self {
        Self { config }
    }

    /// Check if debugging is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Get debug configuration
    pub fn config(&self) -> &DebugConfig {
        &self.config
    }

    /// Create enhanced error from basic information
    pub fn create_enhanced_error(
        &self,
        test: &TestItem,
        error_message: &str,
    ) -> EnhancedError {
        EnhancedError {
            test_id: test.id.clone(),
            error_type: self.classify_error(error_message),
            message: error_message.to_string(),
            context: HashMap::new(),
        }
    }

    /// Classify error type from message
    fn classify_error(&self, error: &str) -> ErrorType {
        if error.contains("AssertionError") {
            ErrorType::AssertionError
        } else if error.contains("ImportError") || error.contains("ModuleNotFoundError") {
            ErrorType::ImportError
        } else if error.contains("SyntaxError") {
            ErrorType::SyntaxError
        } else if error.contains("timeout") || error.contains("TimeoutError") {
            ErrorType::TimeoutError
        } else if error.contains("RuntimeError") || error.contains("Exception") {
            ErrorType::RuntimeError
        } else {
            ErrorType::Unknown
        }
    }
}