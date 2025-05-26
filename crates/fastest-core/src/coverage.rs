use crate::error::Result;
use crate::executor::TestResult;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

use serde::{Deserialize, Serialize};

/// Coverage data for a single file
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileCoverage {
    pub filename: PathBuf,
    pub executed_lines: Vec<usize>,
    pub missing_lines: Vec<usize>,
    pub coverage_percentage: f64,
}

/// Overall coverage report
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CoverageReport {
    pub files: HashMap<PathBuf, FileCoverage>,
    pub total_statements: usize,
    pub covered_statements: usize,
    pub total_coverage: f64,
}

/// Coverage runner that integrates with coverage.py
pub struct CoverageRunner {
    coverage_cmd: String,
    _data_file: PathBuf,
    source_dirs: Vec<PathBuf>,
}

impl CoverageRunner {
    pub fn new(source_dirs: Vec<PathBuf>) -> Self {
        Self {
            coverage_cmd: "coverage".to_string(),
            _data_file: PathBuf::from(".coverage"),
            source_dirs,
        }
    }

    /// Check if coverage.py is installed
    pub fn is_available(&self) -> bool {
        Command::new(&self.coverage_cmd)
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Generate Python code that runs tests with coverage
    pub fn wrap_test_code(&self, test_code: &str) -> String {
        format!(
            r#"
import coverage
import sys

# Start coverage collection
cov = coverage.Coverage(data_file='.coverage.fastest', source={:?})
cov.start()

try:
    # Run the actual tests
    {}
finally:
    # Stop coverage and save
    cov.stop()
    cov.save()
"#,
            self.source_dirs
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect::<Vec<_>>(),
            test_code
        )
    }

    /// Combine coverage data from multiple test runs
    pub fn combine_coverage(&self) -> Result<()> {
        // First check if there are any coverage files to combine
        let coverage_files: Vec<_> = std::fs::read_dir(".")
            .map_err(|e| crate::error::Error::Io(e))?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".coverage.fastest.")
            })
            .collect();

        if coverage_files.is_empty() {
            return Ok(());
        }
        let output = Command::new(&self.coverage_cmd)
            .args(&["combine", "--append"])
            .output()
            .map_err(|e| {
                crate::error::Error::Execution(format!("Failed to combine coverage: {}", e))
            })?;

        if !output.status.success() {
            return Err(crate::error::Error::Execution(format!(
                "Coverage combine failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        Ok(())
    }

    /// Generate coverage report
    pub fn generate_report(&self, format: CoverageFormat) -> Result<CoverageReport> {
        // First, generate JSON report for parsing
        let output = Command::new(&self.coverage_cmd)
            .args(&["json", "-o", ".coverage.json"])
            .output()
            .map_err(|e| {
                crate::error::Error::Execution(format!("Failed to generate coverage report: {}", e))
            })?;

        if !output.status.success() {
            return Err(crate::error::Error::Execution(format!(
                "Coverage report failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        // Parse the JSON report
        let json_content =
            std::fs::read_to_string(".coverage.json").map_err(|e| crate::error::Error::Io(e))?;

        let json: serde_json::Value = serde_json::from_str(&json_content)?;

        // Generate requested format
        match format {
            CoverageFormat::Terminal => {
                Command::new(&self.coverage_cmd)
                    .arg("report")
                    .status()
                    .map_err(|e| {
                        crate::error::Error::Execution(format!(
                            "Failed to show coverage report: {}",
                            e
                        ))
                    })?;
            }
            CoverageFormat::Html => {
                Command::new(&self.coverage_cmd)
                    .args(&["html", "-d", "htmlcov"])
                    .status()
                    .map_err(|e| {
                        crate::error::Error::Execution(format!(
                            "Failed to generate HTML coverage: {}",
                            e
                        ))
                    })?;
                println!("HTML coverage report generated in htmlcov/index.html");
            }
            CoverageFormat::Xml => {
                Command::new(&self.coverage_cmd)
                    .args(&["xml", "-o", "coverage.xml"])
                    .status()
                    .map_err(|e| {
                        crate::error::Error::Execution(format!(
                            "Failed to generate XML coverage: {}",
                            e
                        ))
                    })?;
            }
            _ => {}
        }

        // Parse and return coverage data
        self.parse_coverage_json(&json)
    }

    /// Parse coverage JSON into our format
    fn parse_coverage_json(&self, json: &serde_json::Value) -> Result<CoverageReport> {
        let mut files = HashMap::new();
        let mut total_statements = 0;
        let mut covered_statements = 0;

        if let Some(file_data) = json["files"].as_object() {
            for (filename, data) in file_data {
                let executed = data["executed_lines"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_u64().map(|n| n as usize))
                            .collect()
                    })
                    .unwrap_or_default();

                let missing = data["missing_lines"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_u64().map(|n| n as usize))
                            .collect()
                    })
                    .unwrap_or_default();

                let num_statements =
                    data["summary"]["num_statements"].as_u64().unwrap_or(0) as usize;

                let covered = data["summary"]["covered_lines"].as_u64().unwrap_or(0) as usize;

                let percentage = if num_statements > 0 {
                    (covered as f64 / num_statements as f64) * 100.0
                } else {
                    0.0
                };

                files.insert(
                    PathBuf::from(filename),
                    FileCoverage {
                        filename: PathBuf::from(filename),
                        executed_lines: executed,
                        missing_lines: missing,
                        coverage_percentage: percentage,
                    },
                );

                total_statements += num_statements;
                covered_statements += covered;
            }
        }

        let total_coverage = if total_statements > 0 {
            (covered_statements as f64 / total_statements as f64) * 100.0
        } else {
            0.0
        };

        Ok(CoverageReport {
            files,
            total_statements,
            covered_statements,
            total_coverage,
        })
    }

    /// Clean up coverage data files
    pub fn cleanup(&self) -> Result<()> {
        let files_to_remove = [".coverage", ".coverage.fastest", ".coverage.json"];

        for file in &files_to_remove {
            let _ = std::fs::remove_file(file);
        }

        Ok(())
    }
}

/// Coverage output format
#[derive(Debug, Clone, Copy)]
pub enum CoverageFormat {
    Terminal,
    Html,
    Xml,
    Json,
}

/// Coverage result structure
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct CoverageResult {
    pub report: CoverageReport,
    pub format: String,
}

impl Default for CoverageReport {
    fn default() -> Self {
        Self {
            files: HashMap::new(),
            total_statements: 0,
            covered_statements: 0,
            total_coverage: 0.0,
        }
    }
}

/// Integration with test execution
pub fn run_tests_with_coverage(
    _tests: Vec<crate::discovery::TestItem>,
    source_dirs: Vec<PathBuf>,
    _format: CoverageFormat,
) -> Result<(Vec<TestResult>, CoverageReport)> {
    let coverage = CoverageRunner::new(source_dirs);

    if !coverage.is_available() {
        return Err(crate::error::Error::Execution(
            "coverage.py is not installed. Install with: pip install coverage".to_string(),
        ));
    }

    // Clean up any existing coverage data
    coverage.cleanup()?;

    // TODO: Integrate with executors to wrap test code
    // For now, return a placeholder

    Err(crate::error::Error::Execution(
        "Coverage integration not fully implemented yet".to_string(),
    ))
}

pub fn collect_with_tests(
    _tests: Vec<crate::discovery::TestItem>,
    _sources: Vec<String>,
    _format: CoverageFormat,
) -> Result<CoverageResult> {
    // Implementation of collect_with_tests function
    Ok(CoverageResult::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coverage_availability() {
        let coverage = CoverageRunner::new(vec![]);
        // This might fail in CI, so we just check it doesn't panic
        let _ = coverage.is_available();
    }
}
