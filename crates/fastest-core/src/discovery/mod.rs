//! Test discovery module.
//!
//! Recursively walks directories to find Python test files, then parses them
//! in parallel using rayon to extract test items.

pub mod cache;
pub mod parser;

use crate::config::Config;
use crate::error::Result;
use crate::model::TestItem;
use rayon::prelude::*;
use std::fs;
use std::path::PathBuf;
use walkdir::WalkDir;

/// Discover all tests from the given paths using the provided configuration.
///
/// This function:
/// 1. Recursively walks each path to find Python test files
/// 2. Filters files using `config.is_test_file()`
/// 3. Reads and parses files in parallel using rayon
/// 4. Collects all discovered [`TestItem`]s
pub fn discover_tests(paths: &[PathBuf], config: &Config) -> Result<Vec<TestItem>> {
    let test_files = collect_test_files(paths, config);

    let results: Vec<Result<Vec<TestItem>>> = test_files
        .par_iter()
        .map(|path| {
            let content = fs::read_to_string(path)?;
            parser::parse_test_file(&content, path)
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
fn collect_test_files(paths: &[PathBuf], config: &Config) -> Vec<PathBuf> {
    let mut test_files = Vec::new();

    for path in paths {
        if path.is_file() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if config.is_test_file(name) {
                    test_files.push(path.clone());
                }
            }
        } else if path.is_dir() {
            for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
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

/// Discover tests with caching support.
///
/// Uses the discovery cache to skip re-parsing files that have not changed.
/// Falls back to full parsing for cache misses or changed files.
pub fn discover_tests_cached(
    paths: &[PathBuf],
    config: &Config,
    cache: &mut cache::DiscoveryCache,
) -> Result<Vec<TestItem>> {
    let test_files = collect_test_files(paths, config);

    let results: Vec<(PathBuf, Vec<u8>)> = test_files
        .into_iter()
        .filter_map(|path| fs::read(&path).ok().map(|content| (path, content)))
        .collect();

    let mut all_items = Vec::new();

    for (path, content) in &results {
        let hash = cache::hash_content(content);

        if let Some(cached_items) = cache.get(path, hash) {
            all_items.extend(cached_items.clone());
        } else {
            let source = match std::str::from_utf8(content) {
                Ok(s) => s,
                Err(_) => continue,
            };
            match parser::parse_test_file(source, path) {
                Ok(items) => {
                    cache.insert(path.clone(), hash, items.clone());
                    all_items.extend(items);
                }
                Err(e) => {
                    eprintln!("Warning: {}", e);
                }
            }
        }
    }

    Ok(all_items)
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
        let files = collect_test_files(&paths, &config);

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
