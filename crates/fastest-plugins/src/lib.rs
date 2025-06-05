//! Fastest Plugin System
//! 
//! A brilliant, type-safe plugin architecture that enables extensibility
//! while maintaining Fastest's blazing performance.
//!
//! # Design Philosophy
//! 
//! - **Everything is a plugin**: Core functionality is implemented as built-in plugins
//! - **Zero-cost abstractions**: No overhead when plugins aren't used
//! - **Type safety**: Compile-time guarantees for plugin interfaces
//! - **Pytest compatibility**: Familiar hook names and semantics
//! - **Performance first**: Plugins can't slow down the fast path

#![allow(dead_code)] // During development

pub mod api;
pub mod builtin;
pub mod minimal;

// These modules need fixing, commented out for now
// pub mod conftest;
// pub mod hooks;
// pub mod loader;
// pub mod manager;
// pub mod registry;
// pub mod pytest_compat;

// Re-export core types
pub use api::{Plugin, PluginMetadata, PluginResult, PluginError};
pub use minimal::{PluginManager, PluginManagerBuilder, HookArgs};

// Re-export built-in plugins
pub use builtin::{
    FixturePlugin,
    MarkerPlugin,
    ReportingPlugin,
    CapturePlugin,
};


/// Initialize the plugin system with default plugins
pub fn initialize_default_plugins() -> PluginManager {
    let mut manager = PluginManager::new();
    
    // Register built-in plugins
    manager.register(Box::new(FixturePlugin::new())).unwrap();
    manager.register(Box::new(MarkerPlugin::new())).unwrap();
    manager.register(Box::new(ReportingPlugin::new())).unwrap();
    manager.register(Box::new(CapturePlugin::new())).unwrap();
    
    manager
}