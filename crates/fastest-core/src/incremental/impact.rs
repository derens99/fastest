//! Impact analysis for incremental testing
//!
//! Determines which tests are affected by a set of changed files.
//! If any project configuration file changed, all tests are considered affected.

use std::collections::HashSet;
use std::path::PathBuf;

use crate::model::TestItem;

/// Configuration files whose modification affects every test.
const CONFIG_FILES: &[&str] = &[
    "pyproject.toml",
    "pytest.ini",
    "setup.cfg",
    "tox.ini",
    "setup.py",
    "requirements.txt",
];

/// Given a list of tests and a set of changed file paths, return only the tests
/// that are affected by the changes.
///
/// Rules:
/// - If any config file (pyproject.toml, pytest.ini, etc.) was changed, **all** tests
///   are considered affected.
/// - Otherwise, only tests whose source file appears in `changed_files` are returned.
pub fn find_affected_tests(tests: &[TestItem], changed_files: &HashSet<PathBuf>) -> Vec<TestItem> {
    // Check whether any changed file is a known config file
    let config_changed = changed_files.iter().any(|p| {
        p.file_name()
            .and_then(|name| name.to_str())
            .map(|name| CONFIG_FILES.contains(&name))
            .unwrap_or(false)
    });

    if config_changed {
        return tests.to_vec();
    }

    tests
        .iter()
        .filter(|t| changed_files.contains(&t.path))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::TestItem;
    use std::path::PathBuf;

    fn make_test(id: &str, path: &str) -> TestItem {
        TestItem {
            id: id.to_string(),
            path: PathBuf::from(path),
            function_name: id.to_string(),
            line_number: Some(1),
            decorators: vec![],
            is_async: false,
            fixture_deps: vec![],
            class_name: None,
            markers: vec![],
            parameters: None,
            name: id.to_string(),
        }
    }

    #[test]
    fn test_changed_test_file_is_affected() {
        let tests = vec![
            make_test("test_a", "tests/test_a.py"),
            make_test("test_b", "tests/test_b.py"),
        ];

        let changed: HashSet<PathBuf> = [PathBuf::from("tests/test_a.py")].into_iter().collect();

        let affected = find_affected_tests(&tests, &changed);
        assert_eq!(affected.len(), 1);
        assert_eq!(affected[0].id, "test_a");
    }

    #[test]
    fn test_unchanged_file_is_filtered() {
        let tests = vec![
            make_test("test_a", "tests/test_a.py"),
            make_test("test_b", "tests/test_b.py"),
        ];

        let changed: HashSet<PathBuf> = [PathBuf::from("tests/test_a.py")].into_iter().collect();

        let affected = find_affected_tests(&tests, &changed);
        // test_b's file was NOT changed, so it must not appear
        assert!(
            affected.iter().all(|t| t.id != "test_b"),
            "test_b should have been filtered out"
        );
    }

    #[test]
    fn test_config_change_affects_all() {
        let tests = vec![
            make_test("test_a", "tests/test_a.py"),
            make_test("test_b", "tests/test_b.py"),
            make_test("test_c", "tests/test_c.py"),
        ];

        let changed: HashSet<PathBuf> = [PathBuf::from("pyproject.toml")].into_iter().collect();

        let affected = find_affected_tests(&tests, &changed);
        assert_eq!(
            affected.len(),
            tests.len(),
            "all tests should be affected when a config file changes"
        );
    }
}
