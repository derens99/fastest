//! Built-in plugins that provide core functionality

use std::any::Any;
use crate::api::{Plugin, PluginMetadata, PluginResult};

/// Fixture management plugin
#[derive(Debug)]
pub struct FixturePlugin {
    metadata: PluginMetadata,
}

impl FixturePlugin {
    pub fn new() -> Self {
        Self {
            metadata: PluginMetadata {
                name: "fixtures".to_string(),
                version: "0.1.0".to_string(),
                description: "Built-in fixture management".to_string(),
                priority: 100,
                ..Default::default()
            },
        }
    }
}

impl Plugin for FixturePlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Marker handling plugin
#[derive(Debug)]
pub struct MarkerPlugin {
    metadata: PluginMetadata,
}

impl MarkerPlugin {
    pub fn new() -> Self {
        Self {
            metadata: PluginMetadata {
                name: "markers".to_string(),
                version: "0.1.0".to_string(),
                description: "Test marker support".to_string(),
                priority: 90,
                ..Default::default()
            },
        }
    }
}

impl Plugin for MarkerPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Reporting plugin
#[derive(Debug)]
pub struct ReportingPlugin {
    metadata: PluginMetadata,
}

impl ReportingPlugin {
    pub fn new() -> Self {
        Self {
            metadata: PluginMetadata {
                name: "reporting".to_string(),
                version: "0.1.0".to_string(),
                description: "Test result reporting".to_string(),
                priority: 80,
                ..Default::default()
            },
        }
    }
}

impl Plugin for ReportingPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Output capture plugin
#[derive(Debug)]
pub struct CapturePlugin {
    metadata: PluginMetadata,
}

impl CapturePlugin {
    pub fn new() -> Self {
        Self {
            metadata: PluginMetadata {
                name: "capture".to_string(),
                version: "0.1.0".to_string(),
                description: "Output capture support".to_string(),
                priority: 70,
                ..Default::default()
            },
        }
    }
}

impl Plugin for CapturePlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}