use crate::discovery::TestItem;
use crate::error::Result;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Tracks dependencies between tests and files
pub struct DependencyTracker {
    /// Maps test IDs to the files they depend on
    test_dependencies: HashMap<String, HashSet<PathBuf>>,
    /// Maps file paths to the tests that depend on them
    file_to_tests: HashMap<PathBuf, HashSet<String>>,
}

impl DependencyTracker {
    pub fn new() -> Self {
        Self {
            test_dependencies: HashMap::new(),
            file_to_tests: HashMap::new(),
        }
    }

    /// Update dependencies from test items
    pub fn update_from_tests(&mut self, tests: &[TestItem]) -> Result<()> {
        for test in tests {
            // Extract dependencies for this test
            if let Ok(deps) = extract_python_dependencies(&test.path) {
                self.update_test_dependencies(test.id.clone(), deps);
            }
        }
        Ok(())
    }

    /// Get tests that depend on a file
    pub fn get_dependents(&self, file: &Path) -> Option<&HashSet<String>> {
        self.file_to_tests.get(file)
    }

    /// Update dependencies for a test
    pub fn update_test_dependencies(&mut self, test_id: String, dependencies: HashSet<PathBuf>) {
        // Remove old mappings
        if let Some(old_deps) = self.test_dependencies.get(&test_id) {
            for dep in old_deps {
                if let Some(tests) = self.file_to_tests.get_mut(dep) {
                    tests.remove(&test_id);
                }
            }
        }

        // Add new mappings
        for dep in &dependencies {
            self.file_to_tests
                .entry(dep.clone())
                .or_default()
                .insert(test_id.clone());
        }

        self.test_dependencies.insert(test_id, dependencies);
    }
}

impl Default for DependencyTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Tracks file dependencies and test coverage to enable incremental testing
pub struct IncrementalTestRunner {
    /// Maps test IDs to the files they depend on
    test_dependencies: HashMap<String, HashSet<PathBuf>>,
    /// Maps file paths to their last known modification time
    file_timestamps: HashMap<PathBuf, SystemTime>,
    /// Maps file paths to the tests that depend on them
    file_to_tests: HashMap<PathBuf, HashSet<String>>,
    /// Maps test IDs to the files they depend on
    last_run_cache: HashMap<String, HashSet<PathBuf>>,
    /// Maps test IDs to the files they depend on
    _cache_file: PathBuf,
}

impl IncrementalTestRunner {
    pub fn new() -> Self {
        Self {
            test_dependencies: HashMap::new(),
            file_timestamps: HashMap::new(),
            file_to_tests: HashMap::new(),
            last_run_cache: HashMap::new(),
            _cache_file: PathBuf::from(".fastest_cache"),
        }
    }

    /// Load dependency data from disk
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let data: IncrementalData = serde_json::from_str(&content)?;

        Ok(Self {
            test_dependencies: data.test_dependencies,
            file_timestamps: data.file_timestamps,
            file_to_tests: data.file_to_tests,
            last_run_cache: HashMap::new(),
            _cache_file: PathBuf::from(".fastest_cache"),
        })
    }

    /// Save dependency data to disk
    pub fn save(&self, path: &Path) -> Result<()> {
        let data = IncrementalData {
            test_dependencies: self.test_dependencies.clone(),
            file_timestamps: self.file_timestamps.clone(),
            file_to_tests: self.file_to_tests.clone(),
        };

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(&data)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Determine which tests need to be run based on file changes
    pub fn get_affected_tests(&mut self, all_tests: &[TestItem]) -> Vec<TestItem> {
        let mut affected_tests = HashSet::new();
        let mut changed_files = Vec::new();

        // Check for changed files
        for (path, last_modified) in &self.file_timestamps {
            if let Ok(metadata) = fs::metadata(path) {
                if let Ok(current_modified) = metadata.modified() {
                    if current_modified > *last_modified {
                        changed_files.push(path.clone());
                    }
                }
            }
        }

        // Find tests affected by changed files
        for changed_file in &changed_files {
            if let Some(tests) = self.file_to_tests.get(changed_file) {
                affected_tests.extend(tests.iter().cloned());
            }
        }

        // Also include any new tests
        for test in all_tests {
            if !self.test_dependencies.contains_key(&test.id) {
                affected_tests.insert(test.id.clone());
            }
        }

        // Filter the test list to only affected tests
        all_tests
            .iter()
            .filter(|test| affected_tests.contains(&test.id))
            .cloned()
            .collect()
    }

    /// Update dependencies for a test after it runs
    pub fn update_test_dependencies(&mut self, test_id: String, dependencies: HashSet<PathBuf>) {
        // Remove old mappings
        if let Some(old_deps) = self.test_dependencies.get(&test_id) {
            for dep in old_deps {
                if let Some(tests) = self.file_to_tests.get_mut(dep) {
                    tests.remove(&test_id);
                }
            }
        }

        // Add new mappings
        for dep in &dependencies {
            self.file_to_tests
                .entry(dep.clone())
                .or_default()
                .insert(test_id.clone());

            // Update timestamp
            if let Ok(metadata) = fs::metadata(dep) {
                if let Ok(modified) = metadata.modified() {
                    self.file_timestamps.insert(dep.clone(), modified);
                }
            }
        }

        self.test_dependencies.insert(test_id, dependencies);
    }

    /// Clear all dependency data
    pub fn clear(&mut self) {
        self.test_dependencies.clear();
        self.file_timestamps.clear();
        self.file_to_tests.clear();
        self.last_run_cache.clear();
    }

    /// Get statistics about the dependency tracking
    pub fn stats(&self) -> IncrementalStats {
        IncrementalStats {
            tracked_tests: self.test_dependencies.len(),
            tracked_files: self.file_timestamps.len(),
            total_dependencies: self.test_dependencies.values().map(|deps| deps.len()).sum(),
        }
    }
}

impl Default for IncrementalTestRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct IncrementalData {
    test_dependencies: HashMap<String, HashSet<PathBuf>>,
    file_timestamps: HashMap<PathBuf, SystemTime>,
    file_to_tests: HashMap<PathBuf, HashSet<String>>,
}

#[derive(Debug)]
pub struct IncrementalStats {
    pub tracked_tests: usize,
    pub tracked_files: usize,
    pub total_dependencies: usize,
}

/// Extract Python import dependencies from test code
pub fn extract_python_dependencies(test_file: &Path) -> Result<HashSet<PathBuf>> {
    let content = fs::read_to_string(test_file)?;
    let mut dependencies = HashSet::new();

    // Always include the test file itself
    dependencies.insert(test_file.to_path_buf());

    // Simple regex-based import extraction (can be improved with AST parsing)
    for line in content.lines() {
        let line = line.trim();

        // Handle 'import module' and 'from module import ...'
        if line.starts_with("import ") || line.starts_with("from ") {
            if let Some(module_name) = extract_module_name(line) {
                // Convert module name to potential file paths
                let paths = module_to_paths(&module_name, test_file.parent());
                dependencies.extend(paths);
            }
        }
    }

    Ok(dependencies)
}

fn extract_module_name(import_line: &str) -> Option<String> {
    let line = import_line.trim();

    if let Some(stripped) = line.strip_prefix("import ") {
        // import module.submodule as alias
        let parts: Vec<&str> = stripped.split_whitespace().collect();
        if !parts.is_empty() {
            return Some(parts[0].split(',').next()?.trim().to_string());
        }
    } else if let Some(stripped) = line.strip_prefix("from ") {
        // from module.submodule import something
        let parts: Vec<&str> = stripped.split(" import ").collect();
        if !parts.is_empty() {
            return Some(parts[0].trim().to_string());
        }
    }

    None
}

fn module_to_paths(module_name: &str, base_dir: Option<&Path>) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let base = base_dir.unwrap_or(Path::new("."));

    // Replace dots with path separators
    let module_path = module_name.replace('.', "/");

    // Try as a module directory with __init__.py
    let init_path = base.join(&module_path).join("__init__.py");
    if init_path.exists() {
        paths.push(init_path);
    }

    // Try as a Python file
    let py_path = base.join(format!("{}.py", module_path));
    if py_path.exists() {
        paths.push(py_path);
    }

    // For relative imports, also check parent directories
    if module_name.starts_with('.') {
        let parent_path = base.parent().map(|p| p.to_path_buf());
        if let Some(parent) = parent_path {
            let relative_module = module_name.trim_start_matches('.');
            paths.extend(module_to_paths(relative_module, Some(&parent)));
        }
    }

    paths
}

fn _extract_imports(content: &str) -> HashSet<String> {
    let mut imports = HashSet::new();

    for line in content.lines() {
        let line = line.trim();

        if let Some(stripped) = line.strip_prefix("import ") {
            // import module.submodule as alias
            let parts: Vec<&str> = stripped.split_whitespace().collect();
            if let Some(module) = parts.first() {
                imports.insert(module.split('.').next().unwrap_or(module).to_string());
            }
        } else if let Some(stripped) = line.strip_prefix("from ") {
            // from module.submodule import something
            let parts: Vec<&str> = stripped.split(" import ").collect();
            if let Some(module) = parts.first() {
                imports.insert(module.split('.').next().unwrap_or(module).to_string());
            }
        }
    }

    imports
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_module_name() {
        assert_eq!(extract_module_name("import os"), Some("os".to_string()));
        assert_eq!(
            extract_module_name("from pathlib import Path"),
            Some("pathlib".to_string())
        );
        assert_eq!(
            extract_module_name("import numpy as np"),
            Some("numpy".to_string())
        );
        assert_eq!(
            extract_module_name("from ..utils import helper"),
            Some("..utils".to_string())
        );
    }
}
