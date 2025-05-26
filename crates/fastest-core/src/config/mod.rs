use std::path::{Path, PathBuf};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub addopts: Vec<String>,
    
    #[serde(default)]
    pub markers: Vec<String>,
    
    #[serde(default)]
    pub minversion: Option<String>,
    
    #[serde(default)]
    pub required_plugins: Vec<String>,
    
    #[serde(default)]
    pub cache_dir: Option<PathBuf>,
    
    #[serde(default)]
    pub max_workers: Option<usize>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            testpaths: vec![PathBuf::from(".")],
            python_files: vec!["test_*.py".to_string(), "*_test.py".to_string()],
            python_classes: vec!["Test*".to_string()],
            python_functions: vec!["test_*".to_string()],
            addopts: Vec::new(),
            markers: Vec::new(),
            minversion: None,
            required_plugins: Vec::new(),
            cache_dir: Some(PathBuf::from(".fastest_cache")),
            max_workers: None,
        }
    }
}

impl Config {
    /// Load configuration from various sources
    pub fn load(root: &Path) -> Result<Self> {
        let mut config = Self::default();
        
        // Try pytest.ini
        let pytest_ini = root.join("pytest.ini");
        if pytest_ini.exists() {
            config.merge_pytest_ini(&pytest_ini)?;
        }
        
        // Try pyproject.toml
        let pyproject = root.join("pyproject.toml");
        if pyproject.exists() {
            config.merge_pyproject_toml(&pyproject)?;
        }
        
        // Try setup.cfg
        let setup_cfg = root.join("setup.cfg");
        if setup_cfg.exists() {
            config.merge_setup_cfg(&setup_cfg)?;
        }
        
        // Try tox.ini
        let tox_ini = root.join("tox.ini");
        if tox_ini.exists() {
            config.merge_tox_ini(&tox_ini)?;
        }
        
        Ok(config)
    }
    
    /// Merge pytest.ini configuration
    fn merge_pytest_ini(&mut self, path: &Path) -> Result<()> {
        let content = fs::read_to_string(path)?;
        // TODO: Parse INI format
        // For now, just scan for common patterns
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("testpaths") {
                // Parse testpaths = tests/ src/
                if let Some(paths) = line.split('=').nth(1) {
                    self.testpaths = paths.split_whitespace()
                        .map(PathBuf::from)
                        .collect();
                }
            }
        }
        Ok(())
    }
    
    /// Merge pyproject.toml configuration
    fn merge_pyproject_toml(&mut self, path: &Path) -> Result<()> {
        let content = fs::read_to_string(path)?;
        // TODO: Use toml crate to parse properly
        // For now, basic implementation
        if content.contains("[tool.pytest.ini_options]") {
            // Extract pytest configuration section
        }
        Ok(())
    }
    
    /// Merge setup.cfg configuration
    fn merge_setup_cfg(&mut self, _path: &Path) -> Result<()> {
        // TODO: Parse setup.cfg format
        Ok(())
    }
    
    /// Merge tox.ini configuration
    fn merge_tox_ini(&mut self, _path: &Path) -> Result<()> {
        // TODO: Parse tox.ini format
        Ok(())
    }
    
    /// Check if a file matches the configured patterns
    pub fn is_test_file(&self, filename: &str) -> bool {
        self.python_files.iter().any(|pattern| {
            if pattern.contains('*') {
                // Simple glob matching
                if pattern.starts_with('*') && pattern.ends_with('*') {
                    filename.contains(&pattern[1..pattern.len()-1])
                } else if pattern.starts_with('*') {
                    filename.ends_with(&pattern[1..])
                } else if pattern.ends_with('*') {
                    filename.starts_with(&pattern[..pattern.len()-1])
                } else {
                    filename == pattern
                }
            } else {
                filename == pattern
            }
        })
    }
    
    /// Check if a class name matches the configured patterns
    pub fn is_test_class(&self, name: &str) -> bool {
        self.python_classes.iter().any(|pattern| {
            if pattern.ends_with('*') {
                name.starts_with(&pattern[..pattern.len()-1])
            } else {
                name == pattern
            }
        })
    }
    
    /// Check if a function name matches the configured patterns
    pub fn is_test_function(&self, name: &str) -> bool {
        self.python_functions.iter().any(|pattern| {
            if pattern.ends_with('*') {
                name.starts_with(&pattern[..pattern.len()-1])
            } else {
                name == pattern
            }
        })
    }
} 