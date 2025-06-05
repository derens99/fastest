//! Plugin Registry - Global plugin registration system
//!
//! This module provides mechanisms for plugins to register themselves
//! automatically when loaded.

use std::sync::Arc;
use once_cell::sync::Lazy;
use parking_lot::RwLock;

use crate::api::{Plugin, PluginMetadata};

/// Global plugin registry
static REGISTRY: Lazy<Arc<RwLock<PluginRegistry>>> = Lazy::new(|| {
    Arc::new(RwLock::new(PluginRegistry::new()))
});

/// Plugin constructor function type
pub type PluginConstructor = fn() -> Box<dyn Plugin>;

/// Plugin registry entry
#[derive(Clone)]
pub struct RegistryEntry {
    /// Plugin metadata
    pub metadata: PluginMetadata,
    
    /// Constructor function
    pub constructor: PluginConstructor,
}

/// Global plugin registry
pub struct PluginRegistry {
    /// Registered plugins
    entries: Vec<RegistryEntry>,
}

impl PluginRegistry {
    /// Create a new registry
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
    
    /// Register a plugin
    pub fn register(&mut self, entry: RegistryEntry) {
        // Check for duplicates
        if !self.entries.iter().any(|e| e.metadata.name == entry.metadata.name) {
            self.entries.push(entry);
        }
    }
    
    /// Get all registered plugins
    pub fn entries(&self) -> &[RegistryEntry] {
        &self.entries
    }
    
    /// Create instances of all registered plugins
    pub fn create_all(&self) -> Vec<Box<dyn Plugin>> {
        self.entries.iter()
            .map(|entry| (entry.constructor)())
            .collect()
    }
}

/// Register a plugin in the global registry
pub fn register_plugin(metadata: PluginMetadata, constructor: PluginConstructor) {
    REGISTRY.write().register(RegistryEntry {
        metadata,
        constructor,
    });
}

/// Get the global registry
pub fn global_registry() -> Arc<RwLock<PluginRegistry>> {
    REGISTRY.clone()
}

/// Inventory-based automatic registration
#[macro_export]
macro_rules! register_plugin_inventory {
    ($type:ty) => {
        ::inventory::submit! {
            PluginRegistration {
                metadata: <$type>::METADATA,
                constructor: || Box::new(<$type>::new()) as Box<dyn Plugin>,
            }
        }
    };
}

/// Plugin registration item for inventory
pub struct PluginRegistration {
    pub metadata: &'static PluginMetadata,
    pub constructor: fn() -> Box<dyn Plugin>,
}

inventory::collect!(PluginRegistration);

/// Initialize all plugins registered via inventory
pub fn initialize_inventory_plugins() -> Vec<Box<dyn Plugin>> {
    inventory::iter::<PluginRegistration>()
        .map(|reg| (reg.constructor)())
        .collect()
}

/// Linkme-based automatic registration for better performance
use linkme::distributed_slice;

#[distributed_slice]
pub static PLUGIN_REGISTRATIONS: [PluginRegistration];

/// Register a plugin using linkme
#[macro_export]
macro_rules! register_plugin_linkme {
    ($type:ty) => {
        #[distributed_slice($crate::registry::PLUGIN_REGISTRATIONS)]
        static PLUGIN_REG: $crate::registry::PluginRegistration = 
            $crate::registry::PluginRegistration {
                metadata: &<$type>::METADATA,
                constructor: || Box::new(<$type>::new()) as Box<dyn $crate::api::Plugin>,
            };
    };
}

/// Initialize all plugins registered via linkme
pub fn initialize_linkme_plugins() -> Vec<Box<dyn Plugin>> {
    PLUGIN_REGISTRATIONS.iter()
        .map(|reg| (reg.constructor)())
        .collect()
}

/// Convenience macro for creating a plugin with automatic registration
#[macro_export]
macro_rules! create_plugin {
    (
        $name:ident,
        name: $plugin_name:expr,
        version: $version:expr,
        description: $desc:expr
        $(, requires: [$($req:expr),*])?
        $(, hooks: {
            $($hook_name:ident => $hook_impl:expr),*
        })?
    ) => {
        pub struct $name {
            metadata: $crate::api::PluginMetadata,
        }
        
        impl $name {
            pub const METADATA: $crate::api::PluginMetadata = $crate::api::PluginMetadata {
                name: $plugin_name,
                version: $version,
                description: $desc,
                author: None,
                requires: vec![$($($req.to_string()),*)?],
                conflicts: vec![],
                priority: 0,
            };
            
            pub fn new() -> Self {
                Self {
                    metadata: Self::METADATA.clone(),
                }
            }
        }
        
        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct(stringify!($name))
                    .field("metadata", &self.metadata)
                    .finish()
            }
        }
        
        $crate::impl_plugin!($name);
        
        // Auto-register with linkme
        $crate::register_plugin_linkme!($name);
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_registry() {
        let mut registry = PluginRegistry::new();
        
        let entry = RegistryEntry {
            metadata: PluginMetadata {
                name: "test_plugin".to_string(),
                ..Default::default()
            },
            constructor: || {
                struct TestPlugin {
                    metadata: PluginMetadata,
                }
                
                impl std::fmt::Debug for TestPlugin {
                    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        f.debug_struct("TestPlugin").finish()
                    }
                }
                
                impl Plugin for TestPlugin {
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
                
                Box::new(TestPlugin {
                    metadata: PluginMetadata {
                        name: "test_plugin".to_string(),
                        ..Default::default()
                    },
                })
            },
        };
        
        registry.register(entry.clone());
        assert_eq!(registry.entries().len(), 1);
        
        // Duplicate should not be added
        registry.register(entry);
        assert_eq!(registry.entries().len(), 1);
        
        let plugins = registry.create_all();
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].metadata().name, "test_plugin");
    }
}