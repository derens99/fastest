use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

/// A single test item discovered from a Python test file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestItem {
    /// Unique identifier: "path::class::func[params]"
    pub id: String,
    /// Path to the Python test file
    pub path: PathBuf,
    /// Name of the test function
    pub function_name: String,
    /// Line number in source file
    pub line_number: Option<usize>,
    /// Raw decorator strings from the source
    pub decorators: Vec<String>,
    /// Whether this is an async test
    pub is_async: bool,
    /// Fixture names required by this test (from function parameters)
    pub fixture_deps: Vec<String>,
    /// Class name if this is a method inside a test class
    pub class_name: Option<String>,
    /// Parsed markers (@pytest.mark.*)
    pub markers: Vec<Marker>,
    /// Parametrize values if this test was expanded
    pub parameters: Option<Parameters>,
    /// Display name for output
    pub name: String,
}

/// A pytest marker attached to a test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Marker {
    pub name: String,
    pub args: Vec<serde_json::Value>,
    pub kwargs: HashMap<String, serde_json::Value>,
}

/// Parametrize values for an expanded test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameters {
    pub names: Vec<String>,
    pub values: HashMap<String, serde_json::Value>,
    pub id_suffix: String,
}

/// Outcome of a test execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TestOutcome {
    Passed,
    Failed,
    Skipped { reason: Option<String> },
    XFailed { reason: Option<String> },
    XPassed,
    Error { message: String },
}

/// Result of running a single test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub test_id: String,
    pub outcome: TestOutcome,
    pub duration: Duration,
    pub output: String,
    pub error: Option<String>,
    pub stdout: String,
    pub stderr: String,
}

impl TestResult {
    pub fn passed(&self) -> bool {
        matches!(
            self.outcome,
            TestOutcome::Passed | TestOutcome::XFailed { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_item_id_format() {
        let item = TestItem {
            id: "tests/test_math.py::TestCalc::test_add".into(),
            path: PathBuf::from("tests/test_math.py"),
            function_name: "test_add".into(),
            line_number: Some(10),
            decorators: vec![],
            is_async: false,
            fixture_deps: vec!["tmp_path".into()],
            class_name: Some("TestCalc".into()),
            markers: vec![],
            parameters: None,
            name: "test_add".into(),
        };
        assert!(item.id.contains("::"));
        assert_eq!(item.function_name, "test_add");
    }

    #[test]
    fn test_result_serialization() {
        let result = TestResult {
            test_id: "test::id".into(),
            outcome: TestOutcome::Passed,
            duration: Duration::from_millis(42),
            output: String::new(),
            error: None,
            stdout: String::new(),
            stderr: String::new(),
        };
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: TestResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.outcome, TestOutcome::Passed);
    }

    #[test]
    fn test_outcome_variants() {
        assert_eq!(
            TestOutcome::Skipped {
                reason: Some("no db".into())
            },
            TestOutcome::Skipped {
                reason: Some("no db".into())
            }
        );
        assert_ne!(TestOutcome::Passed, TestOutcome::Failed);
    }

    #[test]
    fn test_result_passed_helper() {
        let passed = TestResult {
            test_id: "t".into(),
            outcome: TestOutcome::Passed,
            duration: Duration::ZERO,
            output: String::new(),
            error: None,
            stdout: String::new(),
            stderr: String::new(),
        };
        assert!(passed.passed());

        let failed = TestResult {
            test_id: "t".into(),
            outcome: TestOutcome::Failed,
            duration: Duration::ZERO,
            output: String::new(),
            error: None,
            stdout: String::new(),
            stderr: String::new(),
        };
        assert!(!failed.passed());

        let xfailed = TestResult {
            test_id: "t".into(),
            outcome: TestOutcome::XFailed { reason: None },
            duration: Duration::ZERO,
            output: String::new(),
            error: None,
            stdout: String::new(),
            stderr: String::new(),
        };
        assert!(xfailed.passed());
    }
}
