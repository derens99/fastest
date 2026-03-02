//! External plugin loading.
//!
//! This module provides a stub for loading plugins from an external directory.
//! A future implementation could use dynamic library loading (`libloading`) or
//! a scripting-language bridge to discover and instantiate third-party plugins.

use super::Plugin;
use crate::error::Result;
use std::path::Path;

/// Attempt to discover and load plugins from `_dir`.
///
/// Currently returns an empty list. A real implementation would scan for
/// shared libraries or plugin manifests and return instantiated [`Plugin`]
/// trait objects.
pub fn load_plugins_from_dir(_dir: &Path) -> Result<Vec<Box<dyn Plugin>>> {
    // Stub — external plugin loading is not yet implemented.
    Ok(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_load_plugins_returns_empty() {
        let dir = PathBuf::from("nonexistent_plugin_dir");
        let plugins = load_plugins_from_dir(&dir).unwrap();
        assert!(plugins.is_empty());
    }

    #[test]
    fn test_load_plugins_from_tempdir() {
        let dir = tempfile::tempdir().unwrap();
        let plugins = load_plugins_from_dir(dir.path()).unwrap();
        assert!(plugins.is_empty());
    }
}
