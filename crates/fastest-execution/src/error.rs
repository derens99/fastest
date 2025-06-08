//! Error types for the fastest-execution crate

use thiserror::Error;

/// Errors that can occur during test execution
#[derive(Error, Debug)]
pub enum ExecutionError {
    /// Python runtime initialization or execution errors
    #[error("Python runtime error: {0}")]
    PythonRuntime(String),
    
    /// Strategy selection errors
    #[error("Strategy selection error: {0}")]
    StrategySelection(String),
    
    /// Fixture-related errors
    #[error("Fixture error: {0}")]
    Fixture(String),
    
    /// Test execution failures
    #[error("Test execution failed: {0}")]
    TestExecution(String),
    
    /// Timeout errors
    #[error("Test timed out after {0} seconds")]
    Timeout(u64),
    
    /// Worker pool errors
    #[error("Worker pool error: {0}")]
    WorkerPool(String),
    
    /// Output capture errors
    #[error("Output capture error: {0}")]
    OutputCapture(String),
    
    /// Core crate errors
    #[error("Core error: {0}")]
    Core(#[from] fastest_core::Error),
    
    /// Plugin errors
    #[error("Plugin error: {0}")]
    Plugin(#[from] fastest_plugins::PluginError),
    
    /// IO errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    /// Python errors from PyO3
    #[cfg(feature = "python")]
    #[error("Python error: {0}")]
    Python(#[from] pyo3::PyErr),
    
    /// Generic errors for flexibility
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Result type alias for the execution crate
pub type Result<T> = std::result::Result<T, ExecutionError>;