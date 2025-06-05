//! pytest-cov compatibility plugin
//!
//! Provides code coverage collection and reporting.

use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule};

use crate::api::{Plugin, PluginMetadata, PluginBuilder, PluginResult};
use crate::hooks::{Hook, HookArgs, HookReturn, HookResult};
use crate::impl_plugin;

/// Coverage configuration
#[derive(Debug, Clone)]
pub struct CoverageConfig {
    /// Source directories to measure coverage for
    pub source: Vec<PathBuf>,
    
    /// Report types to generate
    pub report_types: Vec<CoverageReportType>,
    
    /// Minimum coverage percentage required
    pub min_coverage: Option<f64>,
    
    /// Coverage data file
    pub data_file: PathBuf,
    
    /// Whether to append to existing coverage data
    pub append: bool,
}

impl Default for CoverageConfig {
    fn default() -> Self {
        Self {
            source: vec![PathBuf::from(".")],
            report_types: vec![CoverageReportType::Terminal],
            min_coverage: None,
            data_file: PathBuf::from(".coverage"),
            append: false,
        }
    }
}

/// Coverage report types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoverageReportType {
    Terminal,
    Html,
    Xml,
    Json,
    Lcov,
}

/// pytest-cov compatibility plugin
pub struct CoveragePlugin {
    metadata: PluginMetadata,
    config: CoverageConfig,
    coverage: Arc<RwLock<Option<PyObject>>>,
}

impl CoveragePlugin {
    pub fn new() -> Self {
        Self::with_config(CoverageConfig::default())
    }
    
    pub fn with_config(config: CoverageConfig) -> Self {
        let metadata = PluginBuilder::new("fastest.pytest_cov")
            .version("0.1.0")
            .description("pytest-cov compatibility - code coverage collection")
            .priority(40)
            .build();
        
        Self {
            metadata,
            config,
            coverage: Arc::new(RwLock::new(None)),
        }
    }
    
    /// Initialize coverage module
    fn init_coverage(&self, py: Python) -> PyResult<PyObject> {
        // Import coverage module
        let coverage_module = PyModule::import(py, "coverage")
            .map_err(|_| PyErr::new::<pyo3::exceptions::PyImportError, _>(
                "coverage module not found. Install with: pip install coverage"
            ))?;
        
        // Create coverage instance
        let coverage_class = coverage_module.getattr("Coverage")?;
        
        let kwargs = PyDict::new(py);
        kwargs.set_item("data_file", self.config.data_file.to_str().unwrap())?;
        
        // Set source directories
        let source_list = pyo3::types::PyList::new(py, 
            self.config.source.iter()
                .filter_map(|p| p.to_str())
        );
        kwargs.set_item("source", source_list)?;
        
        let coverage = coverage_class.call((), Some(kwargs))?;
        
        // Start coverage if not appending
        if !self.config.append {
            coverage.call_method0("erase")?;
        }
        
        Ok(coverage.into())
    }
}

impl std::fmt::Debug for CoveragePlugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CoveragePlugin")
            .field("metadata", &self.metadata)
            .field("config", &self.config)
            .finish()
    }
}

impl_plugin!(CoveragePlugin);

/// Hook to start coverage collection
pub struct CoverageStartHook {
    plugin: Arc<CoveragePlugin>,
}

impl CoverageStartHook {
    pub fn new(plugin: Arc<CoveragePlugin>) -> Self {
        Self { plugin }
    }
}

impl Hook for CoverageStartHook {
    fn name(&self) -> &str {
        "pytest_sessionstart"
    }
    
    fn execute(&self, _args: HookArgs) -> HookResult<HookReturn> {
        Python::with_gil(|py| {
            if let Ok(coverage) = self.plugin.init_coverage(py) {
                // Start coverage collection
                if let Err(e) = coverage.call_method0(py, "start") {
                    eprintln!("Failed to start coverage: {}", e);
                } else {
                    *self.plugin.coverage.write() = Some(coverage);
                }
            }
            Ok(HookReturn::None)
        })
    }
}

impl std::fmt::Debug for CoverageStartHook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CoverageStartHook").finish()
    }
}

/// Hook to stop coverage and generate reports
pub struct CoverageFinishHook {
    plugin: Arc<CoveragePlugin>,
}

impl CoverageFinishHook {
    pub fn new(plugin: Arc<CoveragePlugin>) -> Self {
        Self { plugin }
    }
}

impl Hook for CoverageFinishHook {
    fn name(&self) -> &str {
        "pytest_sessionfinish"
    }
    
    fn execute(&self, _args: HookArgs) -> HookResult<HookReturn> {
        Python::with_gil(|py| {
            if let Some(coverage) = self.plugin.coverage.read().as_ref() {
                // Stop coverage collection
                if let Err(e) = coverage.call_method0(py, "stop") {
                    eprintln!("Failed to stop coverage: {}", e);
                }
                
                // Save coverage data
                if let Err(e) = coverage.call_method0(py, "save") {
                    eprintln!("Failed to save coverage data: {}", e);
                }
                
                // Generate reports
                for report_type in &self.plugin.config.report_types {
                    match report_type {
                        CoverageReportType::Terminal => {
                            if let Err(e) = coverage.call_method0(py, "report") {
                                eprintln!("Failed to generate terminal report: {}", e);
                            }
                        }
                        CoverageReportType::Html => {
                            if let Err(e) = coverage.call_method1(py, "html_report", 
                                (PyDict::new(py),)) {
                                eprintln!("Failed to generate HTML report: {}", e);
                            }
                        }
                        CoverageReportType::Xml => {
                            if let Err(e) = coverage.call_method0(py, "xml_report") {
                                eprintln!("Failed to generate XML report: {}", e);
                            }
                        }
                        CoverageReportType::Json => {
                            if let Err(e) = coverage.call_method0(py, "json_report") {
                                eprintln!("Failed to generate JSON report: {}", e);
                            }
                        }
                        CoverageReportType::Lcov => {
                            if let Err(e) = coverage.call_method0(py, "lcov_report") {
                                eprintln!("Failed to generate LCOV report: {}", e);
                            }
                        }
                    }
                }
                
                // Check minimum coverage if configured
                if let Some(min_coverage) = self.plugin.config.min_coverage {
                    if let Ok(percent) = coverage.call_method0(py, "report")
                        .and_then(|r| r.extract::<f64>(py)) {
                        if percent < min_coverage {
                            eprintln!("Coverage {:.1}% is below minimum {:.1}%", 
                                      percent, min_coverage);
                            // Could return a failure here
                        }
                    }
                }
            }
            Ok(HookReturn::None)
        })
    }
}

impl std::fmt::Debug for CoverageFinishHook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CoverageFinishHook").finish()
    }
}

/// Create coverage plugin with CLI arguments
pub fn create_from_args(args: &CoverageArgs) -> CoveragePlugin {
    let mut config = CoverageConfig::default();
    
    if !args.cov.is_empty() {
        config.source = args.cov.iter().map(PathBuf::from).collect();
    }
    
    if args.cov_report.contains(&"term".to_string()) {
        config.report_types.push(CoverageReportType::Terminal);
    }
    if args.cov_report.contains(&"html".to_string()) {
        config.report_types.push(CoverageReportType::Html);
    }
    if args.cov_report.contains(&"xml".to_string()) {
        config.report_types.push(CoverageReportType::Xml);
    }
    if args.cov_report.contains(&"json".to_string()) {
        config.report_types.push(CoverageReportType::Json);
    }
    if args.cov_report.contains(&"lcov".to_string()) {
        config.report_types.push(CoverageReportType::Lcov);
    }
    
    config.min_coverage = args.cov_min;
    config.append = args.cov_append;
    
    CoveragePlugin::with_config(config)
}

/// CLI arguments for coverage
#[derive(Debug, Clone)]
pub struct CoverageArgs {
    /// Source directories to measure coverage
    pub cov: Vec<String>,
    
    /// Report types
    pub cov_report: Vec<String>,
    
    /// Minimum coverage percentage
    pub cov_min: Option<f64>,
    
    /// Append to existing coverage data
    pub cov_append: bool,
}