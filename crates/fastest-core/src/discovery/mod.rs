//! Test discovery module.
//!
//! Recursively walks directories to find Python test files, then parses them
//! in parallel using rayon to extract test items.

pub mod parser;

use crate::config::Config;
use crate::error::Result;
use crate::model::TestItem;
use rayon::prelude::*;
use rustpython_parser::ast::{self, Constant, Expr, Stmt};
use rustpython_parser::Parse;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Discover all tests from the given paths using the provided configuration.
///
/// This function:
/// 1. Recursively walks each path to find Python test files
/// 2. Filters files using `config.is_test_file()`
/// 3. Reads and parses files in parallel using rayon
/// 4. Collects all discovered [`TestItem`]s
pub fn discover_tests(paths: &[PathBuf], config: &Config) -> Result<Vec<TestItem>> {
    let test_files = collect_test_files(paths, config, &config.norecursedirs);

    // Deduplicate overlapping paths (e.g. testpaths = [".", "tests"])
    let test_files = dedup_paths(test_files);

    let results: Vec<Result<Vec<TestItem>>> = test_files
        .par_iter()
        .map(|path| {
            let content = fs::read_to_string(path)?;
            parser::parse_test_file_with_config(&content, path, Some(config))
        })
        .collect();

    let mut all_items = Vec::new();
    for result in results {
        match result {
            Ok(items) => all_items.extend(items),
            Err(e) => {
                // Log parse errors but continue discovery
                eprintln!("Warning: {}", e);
            }
        }
    }

    Ok(all_items)
}

/// Collect all test file paths by walking the given directories.
///
/// Filters files based on the configuration's `python_files` patterns
/// (e.g., `test_*.py`, `*_test.py`).
/// Directories that should never be traversed during test discovery.
const SKIP_DIRS: &[&str] = &[
    ".venv",
    "venv",
    ".env",
    "env",
    "node_modules",
    "__pycache__",
    ".git",
    ".hg",
    ".svn",
    ".tox",
    ".nox",
    ".mypy_cache",
    ".pytest_cache",
    "dist",
    "build",
    ".eggs",
];

/// Check whether a directory name should be skipped during traversal.
///
/// Matches exact names in [`SKIP_DIRS`], any directory ending with `.egg-info`,
/// and any directory name found in `extra_skip` (from `norecursedirs` config).
pub(crate) fn should_skip_dir(name: &str, extra_skip: &[String]) -> bool {
    SKIP_DIRS.contains(&name)
        || name.ends_with(".egg-info")
        || extra_skip.iter().any(|d| name == d.as_str())
}

/// Remove duplicate paths from a file list.
///
/// If `testpaths = [".", "tests"]` produces overlapping results,
/// this deduplicates by canonical path. On Windows, strips the UNC `\\?\`
/// prefix that `canonicalize()` produces so that raw and canonical forms
/// are compared consistently.
fn dedup_paths(mut paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen = std::collections::HashSet::new();
    paths.retain(|p| {
        let key = p
            .canonicalize()
            .map(normalize_canonical)
            .unwrap_or_else(|_| p.clone());
        seen.insert(key)
    });
    paths
}

/// Strip the Windows UNC `\\?\` prefix from canonical paths so that
/// raw paths and canonical paths are comparable in `HashSet`.
fn normalize_canonical(p: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        let s = p.to_string_lossy();
        if let Some(stripped) = s.strip_prefix(r"\\?\") {
            return PathBuf::from(stripped);
        }
    }
    p
}

fn collect_test_files(
    paths: &[PathBuf],
    config: &Config,
    norecursedirs: &[String],
) -> Vec<PathBuf> {
    let mut test_files = Vec::new();

    for path in paths {
        if path.is_file() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if config.is_test_file(name) {
                    test_files.push(path.clone());
                }
            }
        } else if path.is_dir() {
            let walker = WalkDir::new(path).into_iter().filter_entry(|e| {
                if e.file_type().is_dir() {
                    if let Some(name) = e.file_name().to_str() {
                        return !should_skip_dir(name, norecursedirs);
                    }
                }
                true
            });
            for entry in walker.filter_map(|e| e.ok()) {
                if entry.file_type().is_file() {
                    if let Some(name) = entry.file_name().to_str() {
                        if config.is_test_file(name) {
                            test_files.push(entry.into_path());
                        }
                    }
                }
            }
        }
    }

    test_files
}

/// Discover all conftest.py files in the given paths and extract session-scoped fixture names.
///
/// Walks the directory tree collecting conftest.py files, parses each one to find
/// functions decorated with `@pytest.fixture(scope="session")`, and returns
/// their names as a set.
pub fn discover_session_fixtures(paths: &[PathBuf], norecursedirs: &[String]) -> HashSet<String> {
    let conftest_files = collect_conftest_files(paths, norecursedirs);
    let mut session_fixtures = HashSet::new();

    for path in &conftest_files {
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(stmts) = ast::Suite::parse(&content, &path.to_string_lossy()) {
                for stmt in &stmts {
                    if let Some(name) = extract_session_fixture_name(stmt) {
                        session_fixtures.insert(name);
                    }
                }
            }
        }
    }

    session_fixtures
}

/// Collect conftest.py files from the given paths.
fn collect_conftest_files(paths: &[PathBuf], norecursedirs: &[String]) -> Vec<PathBuf> {
    let mut conftest_files = Vec::new();

    for path in paths {
        let root = if path.is_file() {
            path.parent().unwrap_or(Path::new(".")).to_path_buf()
        } else {
            path.clone()
        };

        let walker = WalkDir::new(&root).into_iter().filter_entry(|e| {
            if e.file_type().is_dir() {
                if let Some(name) = e.file_name().to_str() {
                    return !should_skip_dir(name, norecursedirs);
                }
            }
            true
        });

        for entry in walker.filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                if let Some(name) = entry.file_name().to_str() {
                    if name == "conftest.py" {
                        conftest_files.push(entry.into_path());
                    }
                }
            }
        }
    }

    conftest_files
}

/// Check if a statement is a function decorated with `@pytest.fixture(scope="session")`
/// and return the function name if so.
fn extract_session_fixture_name(stmt: &Stmt) -> Option<String> {
    let func = match stmt {
        Stmt::FunctionDef(f) => f,
        Stmt::AsyncFunctionDef(f) => {
            // AsyncFunctionDef has the same shape — treat it the same
            // by returning from the decorated check below
            return check_session_fixture_decorators(&f.decorator_list, &f.name);
        }
        _ => return None,
    };

    check_session_fixture_decorators(&func.decorator_list, &func.name)
}

/// Check a list of decorators for `@pytest.fixture(scope="session")`.
fn check_session_fixture_decorators(decorators: &[Expr], func_name: &str) -> Option<String> {
    for decorator in decorators {
        if let Expr::Call(call) = decorator {
            // Check if the function is pytest.fixture or fixture
            let is_fixture = match call.func.as_ref() {
                Expr::Attribute(attr) => {
                    attr.attr.as_str() == "fixture"
                        && matches!(attr.value.as_ref(), Expr::Name(n) if n.id.as_str() == "pytest")
                }
                Expr::Name(name) => name.id.as_str() == "fixture",
                _ => false,
            };

            if is_fixture {
                // Check for scope="session" in keyword arguments
                for kw in &call.keywords {
                    if let Some(ref arg) = kw.arg {
                        if arg.as_str() == "scope" {
                            if let Expr::Constant(c) = &kw.value {
                                if let Constant::Str(s) = &c.value {
                                    if s == "session" {
                                        return Some(func_name.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    fn write_test_file(dir: &Path, name: &str, content: &str) -> PathBuf {
        let file_path = dir.join(name);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&file_path, content).unwrap();
        file_path
    }

    #[test]
    fn test_collect_test_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // Create test files
        write_test_file(root, "test_basic.py", "def test_one(): pass");
        write_test_file(root, "basic_test.py", "def test_two(): pass");
        write_test_file(root, "helper.py", "def helper(): pass");
        write_test_file(root, "conftest.py", "import pytest");

        // Create nested test files
        write_test_file(root, "subdir/test_nested.py", "def test_three(): pass");

        let config = Config::default();
        let paths = vec![root.to_path_buf()];
        let files = collect_test_files(&paths, &config, &config.norecursedirs);

        // Should find test_basic.py, basic_test.py, subdir/test_nested.py
        assert_eq!(files.len(), 3);

        let filenames: Vec<String> = files
            .iter()
            .filter_map(|p| p.file_name().and_then(|n| n.to_str()).map(String::from))
            .collect();

        assert!(filenames.contains(&"test_basic.py".to_string()));
        assert!(filenames.contains(&"basic_test.py".to_string()));
        assert!(filenames.contains(&"test_nested.py".to_string()));

        // helper.py and conftest.py should not be included
        assert!(!filenames.contains(&"helper.py".to_string()));
        assert!(!filenames.contains(&"conftest.py".to_string()));
    }

    #[test]
    fn test_discover_tests_basic() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        write_test_file(
            root,
            "test_math.py",
            r#"
def test_addition():
    assert 1 + 1 == 2

def test_subtraction():
    assert 2 - 1 == 1
"#,
        );

        write_test_file(
            root,
            "test_string.py",
            r#"
class TestString:
    def test_upper(self):
        assert "hello".upper() == "HELLO"

    def test_lower(self):
        assert "HELLO".lower() == "hello"
"#,
        );

        // Non-test file should be ignored
        write_test_file(root, "conftest.py", "import pytest\n");

        let config = Config::default();
        let paths = vec![root.to_path_buf()];
        let items = discover_tests(&paths, &config).unwrap();

        assert_eq!(items.len(), 4);

        let names: Vec<&str> = items.iter().map(|i| i.function_name.as_str()).collect();
        assert!(names.contains(&"test_addition"));
        assert!(names.contains(&"test_subtraction"));
        assert!(names.contains(&"test_upper"));
        assert!(names.contains(&"test_lower"));

        // Verify class tests have class_name set
        let class_tests: Vec<&TestItem> = items.iter().filter(|i| i.class_name.is_some()).collect();
        assert_eq!(class_tests.len(), 2);
        assert!(class_tests
            .iter()
            .all(|t| t.class_name.as_deref() == Some("TestString")));
    }
}
