use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Main configuration structure
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub testpaths: Vec<PathBuf>,

    #[serde(default)]
    pub python_files: Vec<String>,

    #[serde(default)]
    pub python_classes: Vec<String>,

    #[serde(default)]
    pub python_functions: Vec<String>,

    #[serde(default)]
    pub markers: Vec<String>,

    #[serde(default)]
    pub addopts: String,

    #[serde(default)]
    pub minversion: Option<String>,

    #[serde(default)]
    pub cache_dir: Option<PathBuf>,

    #[serde(default)]
    pub norecursedirs: Vec<String>,

    /// Fastest-specific config
    #[serde(default)]
    pub fastest: FastestConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct FastestConfig {
    #[serde(default)]
    pub workers: Option<usize>,

    #[serde(default)]
    pub incremental: bool,

    #[serde(default)]
    pub verbose: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            testpaths: vec![PathBuf::from("tests"), PathBuf::from(".")],
            python_files: vec!["test_*.py".to_string(), "*_test.py".to_string()],
            python_classes: vec!["Test*".to_string()],
            python_functions: vec!["test_*".to_string()],
            markers: Vec::new(),
            addopts: String::new(),
            minversion: None,
            cache_dir: None,
            norecursedirs: vec![],
            fastest: FastestConfig::default(),
        }
    }
}

impl Config {
    /// Load config from the current directory
    pub fn load() -> Result<Self> {
        Self::load_from_dir(Path::new("."))
    }

    /// Load config from a specific directory, checking multiple sources
    pub fn load_from_dir(dir: &Path) -> Result<Self> {
        // Try pyproject.toml first
        let pyproject = dir.join("pyproject.toml");
        if pyproject.exists() {
            if let Ok(config) = Self::load_from_pyproject(&pyproject) {
                return Ok(config);
            }
        }

        // Try pytest.ini
        let pytest_ini = dir.join("pytest.ini");
        if pytest_ini.exists() {
            if let Ok(config) = Self::load_from_ini_file(&pytest_ini, "[pytest]") {
                return Ok(config);
            }
        }

        // Try setup.cfg
        let setup_cfg = dir.join("setup.cfg");
        if setup_cfg.exists() {
            if let Ok(config) = Self::load_from_ini_file(&setup_cfg, "[tool:pytest]") {
                return Ok(config);
            }
        }

        // Try tox.ini
        let tox_ini = dir.join("tox.ini");
        if tox_ini.exists() {
            if let Ok(config) = Self::load_from_ini_file(&tox_ini, "[pytest]") {
                return Ok(config);
            }
        }

        Ok(Self::default())
    }

    /// Load config from pyproject.toml
    fn load_from_pyproject(path: &Path) -> Result<Self> {
        let contents = fs::read_to_string(path).map_err(|e| {
            crate::error::Error::Config(format!("Failed to read {}: {}", path.display(), e))
        })?;

        let toml_value: toml::Value = toml::from_str(&contents).map_err(|e| {
            crate::error::Error::Config(format!("Failed to parse {}: {}", path.display(), e))
        })?;

        if let Some(tool) = toml_value.get("tool") {
            let mut config = Config::default();

            // Load pytest config from [tool.pytest.ini_options]
            if let Some(pytest) = tool.get("pytest") {
                if let Some(ini_options) = pytest.get("ini_options") {
                    Self::apply_toml_options(&mut config, ini_options);
                }
            }

            // Load fastest-specific config from [tool.fastest]
            if let Some(fastest) = tool.get("fastest") {
                if let Ok(fc) = fastest.clone().try_into::<FastestConfig>() {
                    config.fastest = fc;
                }
            }

            return Ok(config);
        }

        Err(crate::error::Error::Config(
            "No pytest or fastest config found in pyproject.toml".to_string(),
        ))
    }

    /// Apply TOML options to config
    fn apply_toml_options(config: &mut Config, options: &toml::Value) {
        if let Some(arr) = options.get("testpaths").and_then(|v| v.as_array()) {
            config.testpaths = arr
                .iter()
                .filter_map(|v| v.as_str().map(PathBuf::from))
                .collect();
        }
        if let Some(arr) = options.get("python_files").and_then(|v| v.as_array()) {
            config.python_files = arr
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
        }
        if let Some(arr) = options.get("python_classes").and_then(|v| v.as_array()) {
            config.python_classes = arr
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
        }
        if let Some(arr) = options.get("python_functions").and_then(|v| v.as_array()) {
            config.python_functions = arr
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
        }
        if let Some(arr) = options.get("markers").and_then(|v| v.as_array()) {
            config.markers = arr
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
        }
        if let Some(s) = options.get("addopts").and_then(|v| v.as_str()) {
            config.addopts = s.to_string();
        }
        // norecursedirs: accept an array of strings or a space-separated string
        if let Some(arr) = options.get("norecursedirs").and_then(|v| v.as_array()) {
            config.norecursedirs = arr
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
        } else if let Some(s) = options.get("norecursedirs").and_then(|v| v.as_str()) {
            config.norecursedirs = s.split_whitespace().map(String::from).collect();
        }
    }

    /// Load config from an INI-style file with a specific section
    fn load_from_ini_file(path: &Path, section: &str) -> Result<Self> {
        let contents = fs::read_to_string(path).map_err(|e| {
            crate::error::Error::Config(format!("Failed to read {}: {}", path.display(), e))
        })?;

        if let Some(section_content) = Self::extract_ini_section(&contents, section) {
            return Self::parse_ini_config(&section_content);
        }

        // For pytest.ini, the whole file is the [pytest] section
        if section == "[pytest]" && !contents.contains('[') {
            return Self::parse_ini_config(&contents);
        }

        Err(crate::error::Error::Config(format!(
            "No {} section found in {}",
            section,
            path.display()
        )))
    }

    /// Parse INI-style config content
    fn parse_ini_config(contents: &str) -> Result<Self> {
        let mut config = Config::default();
        let lines: Vec<&str> = contents.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i].trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                i += 1;
                continue;
            }

            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim();

                match key {
                    "testpaths" => {
                        config.testpaths = value.split_whitespace().map(PathBuf::from).collect();
                    }
                    "python_files" => {
                        config.python_files = value.split_whitespace().map(String::from).collect();
                    }
                    "python_classes" => {
                        config.python_classes =
                            value.split_whitespace().map(String::from).collect();
                    }
                    "python_functions" => {
                        config.python_functions =
                            value.split_whitespace().map(String::from).collect();
                    }
                    "markers" => {
                        let mut markers = Vec::new();
                        if !value.is_empty() {
                            markers.push(value.to_string());
                        }
                        // Handle multi-line markers
                        let mut j = i + 1;
                        while j < lines.len() {
                            let next_line = lines[j];
                            if next_line.starts_with(' ') || next_line.starts_with('\t') {
                                let marker_line = next_line.trim();
                                if !marker_line.is_empty() && !marker_line.starts_with('#') {
                                    markers.push(marker_line.to_string());
                                }
                                j += 1;
                            } else if next_line.trim().is_empty() {
                                j += 1;
                            } else {
                                break;
                            }
                        }
                        i = j - 1;
                        config.markers = markers;
                    }
                    "addopts" => {
                        config.addopts = value.to_string();
                    }
                    "minversion" => {
                        config.minversion = Some(value.to_string());
                    }
                    "cache_dir" => {
                        config.cache_dir = Some(PathBuf::from(value));
                    }
                    "norecursedirs" => {
                        config.norecursedirs = value.split_whitespace().map(String::from).collect();
                    }
                    _ => {}
                }
            }
            i += 1;
        }

        Ok(config)
    }

    /// Extract a section from INI file content
    fn extract_ini_section(contents: &str, section_name: &str) -> Option<String> {
        let mut in_section = false;
        let mut section_contents = String::new();

        for line in contents.lines() {
            let trimmed = line.trim();

            if trimmed == section_name {
                in_section = true;
                continue;
            }

            if in_section && trimmed.starts_with('[') && trimmed.ends_with(']') {
                break;
            }

            if in_section {
                section_contents.push_str(line);
                section_contents.push('\n');
            }
        }

        if section_contents.is_empty() {
            None
        } else {
            Some(section_contents)
        }
    }

    /// Check if a file matches python_files patterns
    pub fn is_test_file(&self, filename: &str) -> bool {
        self.python_files
            .iter()
            .any(|pattern| Self::matches_pattern(filename, pattern))
    }

    /// Check if a class name matches python_classes patterns
    pub fn is_test_class(&self, class_name: &str) -> bool {
        self.python_classes
            .iter()
            .any(|pattern| Self::matches_pattern(class_name, pattern))
    }

    /// Check if a function name matches python_functions patterns
    pub fn is_test_function(&self, func_name: &str) -> bool {
        self.python_functions
            .iter()
            .any(|pattern| Self::matches_pattern(func_name, pattern))
    }

    /// Glob pattern matching that supports `*` wildcards.
    ///
    /// Uses simple string matching for the common single-`*` case
    /// (e.g. `test_*.py`, `*_test.py`, `Test*`) to avoid regex overhead.
    /// Falls back to regex only for patterns with multiple wildcards.
    fn matches_pattern(text: &str, pattern: &str) -> bool {
        if !pattern.contains('*') {
            return text == pattern;
        }

        // Fast path: single `*` — covers all default pytest patterns
        let star_count = pattern.chars().filter(|&c| c == '*').count();
        if star_count == 1 {
            if let Some(suffix) = pattern.strip_prefix('*') {
                return text.ends_with(suffix);
            }
            if let Some(prefix) = pattern.strip_suffix('*') {
                return text.starts_with(prefix);
            }
            // `*` in the middle: split on it
            let parts: Vec<&str> = pattern.splitn(2, '*').collect();
            return text.len() >= parts[0].len() + parts[1].len()
                && text.starts_with(parts[0])
                && text.ends_with(parts[1]);
        }

        // Multi-wildcard fallback: build a regex
        let mut re = String::from("^");
        for ch in pattern.chars() {
            match ch {
                '*' => re.push_str(".*"),
                '.' | '+' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '^' | '$' | '|' | '\\' => {
                    re.push('\\');
                    re.push(ch);
                }
                _ => re.push(ch),
            }
        }
        re.push('$');
        regex::Regex::new(&re)
            .map(|r| r.is_match(text))
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.python_files, vec!["test_*.py", "*_test.py"]);
        assert_eq!(config.python_functions, vec!["test_*"]);
    }

    #[test]
    fn test_pattern_matching() {
        assert!(Config::matches_pattern("test_foo.py", "test_*.py"));
        assert!(Config::matches_pattern("foo_test.py", "*_test.py"));
        assert!(Config::matches_pattern("TestClass", "Test*"));
        assert!(!Config::matches_pattern("MyClass", "Test*"));
    }

    #[test]
    fn test_ini_parsing() {
        let ini_content = r#"
testpaths = tests integration_tests
python_files = test_*.py *_test.py
addopts = -v --tb=short
markers =
    slow: marks tests as slow
    integration: marks tests as integration tests
"#;

        let config = Config::parse_ini_config(ini_content).unwrap();
        assert_eq!(config.testpaths.len(), 2);
        assert_eq!(config.python_files.len(), 2);
        assert_eq!(config.addopts, "-v --tb=short");
        assert_eq!(config.markers.len(), 2);
    }

    #[test]
    fn test_load_pyproject_toml() {
        let dir = tempfile::tempdir().unwrap();
        let pyproject = dir.path().join("pyproject.toml");
        fs::write(
            &pyproject,
            r#"
[tool.pytest.ini_options]
testpaths = ["tests", "integration"]
python_files = ["check_*.py"]
markers = ["slow: marks tests as slow"]
"#,
        )
        .unwrap();
        let config = Config::load_from_dir(dir.path()).unwrap();
        assert_eq!(config.testpaths.len(), 2);
        assert_eq!(config.python_files, vec!["check_*.py"]);
    }

    #[test]
    fn test_load_pytest_ini() {
        let dir = tempfile::tempdir().unwrap();
        let ini = dir.path().join("pytest.ini");
        fs::write(
            &ini,
            "[pytest]\ntestpaths = tests\npython_functions = check_*\n",
        )
        .unwrap();
        let config = Config::load_from_dir(dir.path()).unwrap();
        assert_eq!(config.python_functions, vec!["check_*"]);
    }

    #[test]
    fn test_is_test_file() {
        let config = Config::default();
        assert!(config.is_test_file("test_math.py"));
        assert!(config.is_test_file("math_test.py"));
        assert!(!config.is_test_file("helper.py"));
    }

    #[test]
    fn test_fallback_to_default() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config::load_from_dir(dir.path()).unwrap();
        assert_eq!(config.python_files, vec!["test_*.py", "*_test.py"]);
    }
}
