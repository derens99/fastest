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
    pub required_plugins: Vec<String>,

    #[serde(default)]
    pub cache_dir: Option<PathBuf>,

    #[serde(default)]
    pub junit_family: String,

    #[serde(default)]
    pub junit_logging: String,

    #[serde(default)]
    pub junit_log_passing_tests: bool,

    #[serde(default)]
    pub junit_duration_report: String,

    #[serde(default)]
    pub junit_suite_name: String,

    // Fastest-specific config
    #[serde(default)]
    pub fastest: FastestConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct FastestConfig {
    #[serde(default)]
    pub workers: Option<usize>,

    #[serde(default = "default_parser")]
    pub parser: String,

    #[serde(default = "default_optimizer")]
    pub optimizer: String,

    #[serde(default)]
    pub batch_size: Option<usize>,

    #[serde(default)]
    pub incremental: bool,

    #[serde(default)]
    pub persistent_workers: bool,

    #[serde(default)]
    pub verbose: bool,
}

fn default_parser() -> String {
    "ast".to_string()
}

fn default_optimizer() -> String {
    "optimized".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            testpaths: vec![PathBuf::from("tests"), PathBuf::from(".")],
            python_files: vec!["test_*.py".to_string(), "*_test.py".to_string()],
            python_classes: vec!["Test*".to_string()],
            python_functions: vec!["test_*".to_string()],
            markers: vec![],
            addopts: String::new(),
            minversion: None,
            required_plugins: vec![],
            cache_dir: None,
            junit_family: "xunit2".to_string(),
            junit_logging: "no".to_string(),
            junit_log_passing_tests: true,
            junit_duration_report: "call".to_string(),
            junit_suite_name: "pytest".to_string(),
            fastest: FastestConfig::default(),
        }
    }
}

impl Config {
    /// Load config from the current directory, checking multiple sources
    pub fn load() -> Result<Self> {
        // Try pyproject.toml first
        if let Ok(config) = Self::load_from_pyproject() {
            return Ok(config);
        }

        // Try pytest.ini
        if let Ok(config) = Self::load_from_pytest_ini() {
            return Ok(config);
        }

        // Try setup.cfg
        if let Ok(config) = Self::load_from_setup_cfg() {
            return Ok(config);
        }

        // Try tox.ini
        if let Ok(config) = Self::load_from_tox_ini() {
            return Ok(config);
        }

        // Return default config
        Ok(Self::default())
    }

    /// Load config from pyproject.toml
    pub fn load_from_pyproject() -> Result<Self> {
        let path = Path::new("pyproject.toml");
        if !path.exists() {
            return Err(crate::error::Error::Config(
                "pyproject.toml not found".to_string(),
            ));
        }

        let contents = fs::read_to_string(path).map_err(|e| {
            crate::error::Error::Config(format!("Failed to read pyproject.toml: {}", e))
        })?;

        let toml_value: toml::Value = toml::from_str(&contents).map_err(|e| {
            crate::error::Error::Config(format!("Failed to parse pyproject.toml: {}", e))
        })?;

        // Look for [tool.pytest.ini_options] or [tool.fastest]
        if let Some(tool) = toml_value.get("tool") {
            let mut config = Config::default();

            // Load pytest config
            if let Some(pytest) = tool.get("pytest") {
                if let Some(ini_options) = pytest.get("ini_options") {
                    // Manually extract fields since TOML structure might not match exactly
                    if let Some(testpaths) = ini_options.get("testpaths") {
                        if let Some(arr) = testpaths.as_array() {
                            config.testpaths = arr
                                .iter()
                                .filter_map(|v| v.as_str().map(PathBuf::from))
                                .collect();
                        }
                    }
                    if let Some(python_files) = ini_options.get("python_files") {
                        if let Some(arr) = python_files.as_array() {
                            config.python_files = arr
                                .iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect();
                        }
                    }
                    // ... extract other fields similarly
                }
            }

            // Load fastest-specific config
            if let Some(fastest) = tool.get("fastest") {
                config.fastest = fastest.clone().try_into().map_err(|e| {
                    crate::error::Error::Config(format!(
                        "Failed to deserialize fastest config: {}",
                        e
                    ))
                })?;
            }

            return Ok(config);
        }

        Err(crate::error::Error::Config(
            "No pytest or fastest config found in pyproject.toml".to_string(),
        ))
    }

    /// Load config from pytest.ini
    pub fn load_from_pytest_ini() -> Result<Self> {
        let path = Path::new("pytest.ini");
        if !path.exists() {
            return Err(crate::error::Error::Config(
                "pytest.ini not found".to_string(),
            ));
        }

        let contents = fs::read_to_string(path).map_err(|e| {
            crate::error::Error::Config(format!("Failed to read pytest.ini: {}", e))
        })?;

        Self::parse_ini_config(&contents)
    }

    /// Load config from setup.cfg
    pub fn load_from_setup_cfg() -> Result<Self> {
        let path = Path::new("setup.cfg");
        if !path.exists() {
            return Err(crate::error::Error::Config(
                "setup.cfg not found".to_string(),
            ));
        }

        let contents = fs::read_to_string(path)
            .map_err(|e| crate::error::Error::Config(format!("Failed to read setup.cfg: {}", e)))?;

        // Look for [tool:pytest] section
        if let Some(pytest_section) = Self::extract_ini_section(&contents, "[tool:pytest]") {
            return Self::parse_ini_config(&pytest_section);
        }

        Err(crate::error::Error::Config(
            "No pytest config found in setup.cfg".to_string(),
        ))
    }

    /// Load config from tox.ini
    pub fn load_from_tox_ini() -> Result<Self> {
        let path = Path::new("tox.ini");
        if !path.exists() {
            return Err(crate::error::Error::Config("tox.ini not found".to_string()));
        }

        let contents = fs::read_to_string(path)
            .map_err(|e| crate::error::Error::Config(format!("Failed to read tox.ini: {}", e)))?;

        // Look for [pytest] section
        if let Some(pytest_section) = Self::extract_ini_section(&contents, "[pytest]") {
            return Self::parse_ini_config(&pytest_section);
        }

        Err(crate::error::Error::Config(
            "No pytest config found in tox.ini".to_string(),
        ))
    }

    /// Parse INI-style config
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
                        // Handle multi-line markers
                        let mut markers = Vec::new();

                        // Check if there's a marker on the same line
                        if !value.is_empty() {
                            markers.push(value.to_string());
                        }

                        // Look for continuation lines (indented lines after markers =)
                        let mut j = i + 1;
                        while j < lines.len() {
                            let next_line = lines[j];
                            // If line starts with whitespace, it's a continuation
                            if next_line.starts_with(' ') || next_line.starts_with('\t') {
                                let marker_line = next_line.trim();
                                if !marker_line.is_empty() && !marker_line.starts_with('#') {
                                    markers.push(marker_line.to_string());
                                }
                                j += 1;
                            } else if next_line.trim().is_empty() {
                                // Empty lines are allowed in multi-line values
                                j += 1;
                            } else {
                                // Non-indented non-empty line means end of multi-line value
                                break;
                            }
                        }
                        i = j - 1; // Skip the lines we've already processed
                        config.markers = markers;
                    }
                    "addopts" => {
                        config.addopts = value.to_string();
                    }
                    "minversion" => {
                        config.minversion = Some(value.to_string());
                    }
                    "required_plugins" => {
                        config.required_plugins =
                            value.split_whitespace().map(String::from).collect();
                    }
                    "cache_dir" => {
                        config.cache_dir = Some(PathBuf::from(value));
                    }
                    "junit_family" => {
                        config.junit_family = value.to_string();
                    }
                    // Fastest-specific options
                    "fastest_workers" => {
                        if let Ok(n) = value.parse() {
                            config.fastest.workers = Some(n);
                        }
                    }
                    "fastest_parser" => {
                        config.fastest.parser = value.to_string();
                    }
                    "fastest_optimizer" => {
                        config.fastest.optimizer = value.to_string();
                    }
                    _ => {}
                }
            }
            i += 1;
        }

        Ok(config)
    }

    /// Extract a section from INI file
    fn extract_ini_section(contents: &str, section_name: &str) -> Option<String> {
        let mut in_section = false;
        let mut section_contents = String::new();

        for line in contents.lines() {
            let trimmed = line.trim();

            // Check for section start
            if trimmed == section_name {
                in_section = true;
                continue;
            }

            // Check for next section
            if in_section && trimmed.starts_with('[') && trimmed.ends_with(']') {
                break;
            }

            // Collect lines in section
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
        for pattern in &self.python_files {
            if Self::matches_pattern(filename, pattern) {
                return true;
            }
        }
        false
    }

    /// Check if a class name matches python_classes patterns
    pub fn is_test_class(&self, class_name: &str) -> bool {
        for pattern in &self.python_classes {
            if Self::matches_pattern(class_name, pattern) {
                return true;
            }
        }
        false
    }

    /// Check if a function name matches python_functions patterns
    pub fn is_test_function(&self, func_name: &str) -> bool {
        for pattern in &self.python_functions {
            if Self::matches_pattern(func_name, pattern) {
                return true;
            }
        }
        false
    }

    /// Simple glob pattern matching
    fn matches_pattern(text: &str, pattern: &str) -> bool {
        // Handle patterns with * in the middle (e.g., test_*.py)
        if let Some(star_pos) = pattern.find('*') {
            let prefix = &pattern[..star_pos];
            let suffix = &pattern[star_pos + 1..];

            // Check if text starts with prefix and ends with suffix
            if text.len() >= prefix.len() + suffix.len() {
                text.starts_with(prefix) && text.ends_with(suffix)
            } else {
                false
            }
        } else {
            // No wildcard, exact match
            text == pattern
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
