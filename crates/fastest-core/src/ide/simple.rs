//! Simplified IDE Integration
//!
//! Basic IDE integration without complex LSP dependencies

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::collections::HashMap;

use crate::{TestItem, TestResult};

/// Simple IDE integration manager
pub struct SimpleIdeIntegration {
    test_cache: HashMap<String, Vec<IdeTestItem>>,
    results_cache: HashMap<String, IdeTestResult>,
}

/// Test status for IDE display
#[derive(Debug, Clone, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum TestStatus {
    NotRun = 0,
    Running = 1,
    Passed = 2,
    Failed = 3,
    Skipped = 4,
    Error = 5,
}

/// Test information for IDE
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdeTestItem {
    pub id: String,
    pub label: String,
    pub file_path: String,
    pub line_number: u32,
    pub kind: TestKind,
    pub status: TestStatus,
    pub parent: Option<String>,
    pub children: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TestKind {
    File,
    Class,
    Function,
    Parametrized,
}

/// Test result for IDE
#[derive(Debug, Serialize)]
pub struct IdeTestResult {
    pub test_id: String,
    pub status: TestStatus,
    pub duration_ms: u64,
    pub error_message: Option<String>,
    pub output: String,
}

impl SimpleIdeIntegration {
    pub fn new() -> Self {
        Self {
            test_cache: HashMap::new(),
            results_cache: HashMap::new(),
        }
    }

    /// Convert test items to IDE format
    pub fn convert_tests(&self, tests: Vec<TestItem>) -> Vec<IdeTestItem> {
        tests
            .into_iter()
            .map(|test| {
                let kind = if test.id.contains("::") {
                    if test.decorators.iter().any(|d| d.contains("parametrize")) {
                        TestKind::Parametrized
                    } else {
                        TestKind::Function
                    }
                } else {
                    TestKind::File
                };

                IdeTestItem {
                    id: test.id.clone(),
                    label: self.create_test_label(&test),
                    file_path: test.path.to_string_lossy().to_string(),
                    line_number: test.line_number as u32,
                    kind,
                    status: TestStatus::NotRun,
                    parent: self.get_parent_test_id(&test),
                    children: Vec::new(),
                }
            })
            .collect()
    }

    /// Create human-readable test label
    fn create_test_label(&self, test: &TestItem) -> String {
        // Check if test has parameters in decorators
        let has_params = test.decorators.iter().any(|d| d.contains("parametrize"));
        if has_params {
            format!("{}[parametrized]", test.function_name)
        } else {
            test.function_name.clone()
        }
    }

    /// Format test parameters for display
    fn format_params(&self, params: &serde_json::Value) -> String {
        match params {
            serde_json::Value::Object(map) => map
                .iter()
                .map(|(k, v)| format!("{}={}", k, self.format_param_value(v)))
                .collect::<Vec<_>>()
                .join(", "),
            _ => params.to_string(),
        }
    }

    /// Format single parameter value
    fn format_param_value(&self, value: &serde_json::Value) -> String {
        match value {
            serde_json::Value::String(s) => format!("'{}'", s),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            _ => value.to_string(),
        }
    }

    /// Get parent test ID for hierarchical display
    fn get_parent_test_id(&self, test: &TestItem) -> Option<String> {
        if test.id.contains("::") {
            let parts: Vec<&str> = test.id.split("::").collect();
            if parts.len() > 1 {
                Some(parts[0..parts.len() - 1].join("::"))
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Convert test result to IDE format
    pub fn convert_result(&self, result: &TestResult) -> IdeTestResult {
        let status = if result.passed {
            TestStatus::Passed
        } else {
            TestStatus::Failed
        };

        IdeTestResult {
            test_id: result.test_id.clone(),
            status,
            duration_ms: result.duration.as_millis() as u64,
            error_message: result.error.clone(),
            output: result.output.clone(),
        }
    }

    /// Generate test discovery information
    pub fn generate_test_discovery(
        &self,
        tests: &[IdeTestItem],
    ) -> HashMap<String, serde_json::Value> {
        let mut discovery = HashMap::new();

        discovery.insert(
            "total_tests".to_string(),
            serde_json::Value::Number(tests.len().into()),
        );

        let mut kind_counts = HashMap::new();
        for test in tests {
            let kind_str = format!("{:?}", test.kind);
            *kind_counts.entry(kind_str).or_insert(0) += 1;
        }

        discovery.insert(
            "test_kinds".to_string(),
            serde_json::to_value(kind_counts).unwrap(),
        );

        discovery
    }

    /// Export test data for IDE consumption
    pub fn export_for_ide(&self, tests: &[IdeTestItem]) -> Result<String> {
        let export_data = serde_json::json!({
            "tests": tests,
            "metadata": {
                "generator": "fastest",
                "version": env!("CARGO_PKG_VERSION"),
                "timestamp": chrono::Utc::now().to_rfc3339()
            }
        });

        Ok(serde_json::to_string_pretty(&export_data)?)
    }

    /// Generate IDE statistics
    pub fn get_ide_stats(&self) -> HashMap<String, serde_json::Value> {
        let mut stats = HashMap::new();

        stats.insert(
            "cached_tests".to_string(),
            serde_json::Value::Number(self.test_cache.len().into()),
        );
        stats.insert(
            "cached_results".to_string(),
            serde_json::Value::Number(self.results_cache.len().into()),
        );

        stats
    }
}

impl Default for SimpleIdeIntegration {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TestItem;

    #[test]
    fn test_status_serialization() {
        assert_eq!(TestStatus::NotRun as u8, 0);
        assert_eq!(TestStatus::Passed as u8, 2);
        assert_eq!(TestStatus::Failed as u8, 3);
    }

    #[test]
    fn test_format_params() {
        let ide = SimpleIdeIntegration::new();

        let params = serde_json::json!({
            "x": 1,
            "y": "test",
            "z": true
        });

        let formatted = ide.format_params(&params);
        assert!(formatted.contains("x=1"));
        assert!(formatted.contains("y='test'"));
        assert!(formatted.contains("z=true"));
    }
}
