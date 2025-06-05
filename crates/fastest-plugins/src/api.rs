//! Plugin API - Core traits and types for plugin development
//!
//! This module provides the foundational types that all plugins must implement.

use std::any::Any;
use std::fmt::Debug;
use serde::{Deserialize, Serialize};

/// Result type for plugin operations
pub type PluginResult<T> = Result<T, PluginError>;

/// Plugin errors
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("Plugin initialization failed: {0}")]
    InitializationFailed(String),
    
    #[error("Hook execution failed: {0}")]
    HookFailed(String),
    
    #[error("Plugin not found: {0}")]
    NotFound(String),
    
    #[error("Plugin conflict: {0}")]
    Conflict(String),
    
    #[error("Invalid plugin: {0}")]
    Invalid(String),
    
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Plugin metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// Unique plugin name
    pub name: String,
    
    /// Plugin version
    pub version: String,
    
    /// Plugin description
    pub description: String,
    
    /// Author information
    pub author: Option<String>,
    
    /// Required plugins
    pub requires: Vec<String>,
    
    /// Conflicting plugins
    pub conflicts: Vec<String>,
    
    /// Plugin priority (higher = earlier execution)
    pub priority: i32,
}

impl Default for PluginMetadata {
    fn default() -> Self {
        Self {
            name: String::new(),
            version: "0.1.0".to_string(),
            description: String::new(),
            author: None,
            requires: Vec::new(),
            conflicts: Vec::new(),
            priority: 0,
        }
    }
}

/// Core plugin trait that all plugins must implement
pub trait Plugin: Debug + Send + Sync {
    /// Get plugin metadata
    fn metadata(&self) -> &PluginMetadata;
    
    /// Initialize the plugin
    fn initialize(&mut self) -> PluginResult<()> {
        Ok(())
    }
    
    /// Shutdown the plugin
    fn shutdown(&mut self) -> PluginResult<()> {
        Ok(())
    }
    
    /// Get the plugin as Any for downcasting
    fn as_any(&self) -> &dyn Any;
    
    /// Get mutable reference as Any
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// Plugin information for discovery
#[derive(Debug, Clone)]
pub struct PluginInfo {
    /// Plugin metadata
    pub metadata: PluginMetadata,
    
    /// Plugin type (builtin, python, native)
    pub plugin_type: PluginType,
    
    /// Plugin source (file path, package name, etc)
    pub source: String,
}

/// Types of plugins
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginType {
    /// Built-in plugin (part of fastest)
    Builtin,
    
    /// Python plugin (loaded from Python code)
    Python,
    
    /// Native plugin (Rust dynamic library)
    Native,
    
    /// Conftest plugin (loaded from conftest.py)
    Conftest,
}

/// Trait for plugins that can be discovered automatically
pub trait DiscoverablePlugin: Plugin {
    /// Entry point name for discovery
    const ENTRY_POINT: &'static str;
}

/// Builder pattern for creating plugins
pub struct PluginBuilder {
    metadata: PluginMetadata,
}

impl PluginBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            metadata: PluginMetadata {
                name: name.into(),
                ..Default::default()
            },
        }
    }
    
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.metadata.version = version.into();
        self
    }
    
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.metadata.description = desc.into();
        self
    }
    
    pub fn author(mut self, author: impl Into<String>) -> Self {
        self.metadata.author = Some(author.into());
        self
    }
    
    pub fn requires(mut self, plugin: impl Into<String>) -> Self {
        self.metadata.requires.push(plugin.into());
        self
    }
    
    pub fn conflicts(mut self, plugin: impl Into<String>) -> Self {
        self.metadata.conflicts.push(plugin.into());
        self
    }
    
    pub fn priority(mut self, priority: i32) -> Self {
        self.metadata.priority = priority;
        self
    }
    
    pub fn build(self) -> PluginMetadata {
        self.metadata
    }
}

/// Macro to implement Plugin trait boilerplate
#[macro_export]
macro_rules! impl_plugin {
    ($type:ty) => {
        impl Plugin for $type {
            fn metadata(&self) -> &PluginMetadata {
                &self.metadata
            }
            
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
            
            fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
                self
            }
        }
    };
}