//! Test Discovery Module - Using Python Introspection
//!
//! Reliable test discovery using:
//! - Python's native AST and introspection for perfect compatibility
//! - Full Unicode support for all identifiers
//! - Accurate detection of all Python test patterns
//! - Proper handling of decorators and fixtures

use crate::error::Result;
use ignore::WalkBuilder;
use once_cell::sync::Lazy;
use rayon::prelude::*;
use regex::Regex;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

mod python_introspection;
use crate::test::parametrize::expand_parametrized_tests;
use python_introspection::discover_tests_in_files;

// Removed string interning and tree-sitter parser - using Python introspection instead

/// Test item representing a discovered test - optimized for memory efficiency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestItem {
    pub id: String, // Keep as String for serialization compatibility
    pub path: PathBuf,
    pub function_name: String,
    pub line_number: Option<u32>, // Changed from usize to u32 to save memory
    pub decorators: SmallVec<[String; 2]>, // Most tests have 0-2 decorators
    pub is_async: bool,
    pub fixture_deps: SmallVec<[String; 4]>, // Most tests have <4 fixtures
    pub class_name: Option<String>,
    pub is_xfail: bool,
    pub name: String,
    /// Map of parameter names to whether they are indirect
    #[serde(default)]
    pub indirect_params: HashMap<String, bool>,
}

/// Test metadata packed into a smaller structure
#[derive(Debug, Clone, Copy)]
pub struct TestMetadata {
    pub line_number: u32,
    pub flags: u8, // Bit 0: is_async, Bit 1: is_xfail, Bit 2: has_class
}

impl TestMetadata {
    #[inline]
    pub fn is_async(&self) -> bool {
        self.flags & 0x01 != 0
    }

    #[inline]
    pub fn is_xfail(&self) -> bool {
        self.flags & 0x02 != 0
    }

    #[inline]
    pub fn has_class(&self) -> bool {
        self.flags & 0x04 != 0
    }
}

// Pattern matching now handled by Python AST parsing

/// Regex for pytest file patterns
static PYTEST_FILE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)^(test_.*|.*_test)\.py$").unwrap());

/// Fast test discovery using optimized strategies
pub fn discover_tests(paths: &[PathBuf]) -> Result<Vec<TestItem>> {
    discover_tests_with_filtering(paths, false)
}

/// Discover tests with optimal parallelization
pub fn discover_tests_with_filtering(
    paths: &[PathBuf],
    _apply_performance_filtering: bool, // Keep for API compatibility
) -> Result<Vec<TestItem>> {
    // Collect test files efficiently
    let test_files = collect_test_files(paths);

    if test_files.is_empty() {
        return Ok(Vec::new());
    }

    // Use Python introspection to discover all tests at once, then expand
    // parametrized tests into concrete executable cases.
    let discovered = discover_tests_in_files(&test_files)?;
    let mut expanded = Vec::with_capacity(discovered.len());
    for test in discovered {
        expanded.extend(expand_parametrized_tests(&test, &test.decorators)?);
    }

    Ok(expanded)
}

/// Collect test files with efficient walking
fn collect_test_files(paths: &[PathBuf]) -> Vec<PathBuf> {
    paths
        .par_iter()
        .flat_map(|path| {
            if path.is_file() {
                if is_python_test_file(path) {
                    vec![path.clone()]
                } else {
                    vec![]
                }
            } else {
                // Use ignore crate for fast directory walking
                WalkBuilder::new(path)
                    .standard_filters(false)
                    .hidden(false)
                    .git_ignore(true)
                    .git_exclude(true)
                    .follow_links(false)
                    .max_depth(None)
                    .filter_entry(|entry| {
                        // Skip __pycache__ directories
                        entry.file_name() != "__pycache__"
                    })
                    .build()
                    .filter_map(|entry| entry.ok())
                    .filter(|entry| entry.file_type().is_some_and(|ft| ft.is_file()))
                    .filter(|entry| is_python_test_file(entry.path()))
                    .map(|entry| entry.path().to_path_buf())
                    .collect::<Vec<_>>()
            }
        })
        .collect()
}

/// Check if file is a Python test file
#[inline]
fn is_python_test_file(path: &Path) -> bool {
    path.extension().and_then(|ext| ext.to_str()) == Some("py")
        && path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| PYTEST_FILE_RE.is_match(name))
}

// Memory mapping removed - Python introspection handles all file sizes efficiently

// Content parsing removed - Python introspection handles this directly

// ID normalization removed - Python introspection preserves Unicode correctly

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_discover_simple_tests() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test_example.py");

        fs::write(
            &test_file,
            r#"
def test_one():
    pass

async def test_two():
    pass

class TestClass:
    def test_three(self):
        pass
"#,
        )
        .unwrap();

        let tests = discover_tests(&[temp_dir.path().to_path_buf()]).unwrap();
        assert_eq!(tests.len(), 3);
    }

    #[test]
    fn test_discover_expands_parametrized_tests() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test_parametrized.py");

        fs::write(
            &test_file,
            r#"
import pytest

@pytest.mark.parametrize("number", [1, 2, 3])
def test_number(number):
    assert number > 0
"#,
        )
        .unwrap();

        let tests = discover_tests(&[temp_dir.path().to_path_buf()]).unwrap();
        assert_eq!(tests.len(), 3);
        assert!(tests.iter().all(|test| test.id.contains("test_number[")));
        assert!(tests
            .iter()
            .all(|test| test.decorators.iter().any(|d| d.starts_with("__params__="))));
    }

    #[test]
    fn test_discover_propagates_class_markers_to_methods() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test_class_markers.py");

        fs::write(
            &test_file,
            r#"
import pytest

@pytest.mark.xfail(reason="class level")
class TestMarked:
    def test_method(self):
        pass
"#,
        )
        .unwrap();

        let tests = discover_tests(&[temp_dir.path().to_path_buf()]).unwrap();
        assert_eq!(tests.len(), 1);
        assert!(tests[0]
            .decorators
            .iter()
            .any(|decorator| decorator.contains("pytest.mark.xfail")));
    }

    #[test]
    fn test_discover_propagates_module_pytestmark_to_tests() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test_module_markers.py");

        fs::write(
            &test_file,
            r#"
import pytest

pytestmark = pytest.mark.xfail(reason="module level")

def test_marked_function():
    pass

class TestMarkedClass:
    def test_marked_method(self):
        pass
"#,
        )
        .unwrap();

        let tests = discover_tests(&[temp_dir.path().to_path_buf()]).unwrap();
        assert_eq!(tests.len(), 2);
        assert!(tests.iter().all(|test| {
            test.decorators
                .iter()
                .any(|decorator| decorator.contains("pytest.mark.xfail"))
        }));
    }
}
