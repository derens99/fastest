//! Lifecycle management for setup/teardown methods
//! 
//! This module provides infrastructure for managing the execution lifecycle
//! of setup and teardown methods at various scopes (module, class, method).

use std::collections::{HashMap, HashSet};
use anyhow::{anyhow, Result};
use pyo3::prelude::*;

/// Scope of setup/teardown execution
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LifecycleScope {
    Module,
    Class,
    Method,
    Function,
}

/// State of a lifecycle scope
#[derive(Debug, Clone)]
pub struct LifecycleState {
    pub scope: LifecycleScope,
    pub identifier: String,
    pub setup_completed: bool,
    pub teardown_completed: bool,
    pub setup_failed: bool,
}

/// Manages the lifecycle of setup/teardown methods
pub struct LifecycleManager {
    /// Track the order in which setups were performed (for reverse teardown)
    setup_order: Vec<(LifecycleScope, String)>,
    
    /// Current state of each scope
    active_scopes: HashMap<String, LifecycleState>,
    
    /// Track which modules have been set up
    setup_modules: HashSet<String>,
    
    /// Track which classes have been set up
    setup_classes: HashSet<String>,
}

impl LifecycleManager {
    /// Create a new lifecycle manager
    pub fn new() -> Self {
        Self {
            setup_order: Vec::new(),
            active_scopes: HashMap::new(),
            setup_modules: HashSet::new(),
            setup_classes: HashSet::new(),
        }
    }
    
    /// Check if a module needs setup
    pub fn needs_module_setup(&self, module_path: &str) -> bool {
        !self.setup_modules.contains(module_path)
    }
    
    /// Check if a class needs setup
    pub fn needs_class_setup(&self, class_path: &str) -> bool {
        !self.setup_classes.contains(class_path)
    }
    
    /// Record that a module setup has been completed
    pub fn record_module_setup(&mut self, module_path: &str) -> Result<()> {
        if self.setup_modules.contains(module_path) {
            return Err(anyhow!("Module {} already set up", module_path));
        }
        
        self.setup_modules.insert(module_path.to_string());
        self.setup_order.push((LifecycleScope::Module, module_path.to_string()));
        
        self.active_scopes.insert(
            module_path.to_string(),
            LifecycleState {
                scope: LifecycleScope::Module,
                identifier: module_path.to_string(),
                setup_completed: true,
                teardown_completed: false,
                setup_failed: false,
            },
        );
        
        Ok(())
    }
    
    /// Record that a class setup has been completed
    pub fn record_class_setup(&mut self, class_path: &str) -> Result<()> {
        if self.setup_classes.contains(class_path) {
            return Err(anyhow!("Class {} already set up", class_path));
        }
        
        self.setup_classes.insert(class_path.to_string());
        self.setup_order.push((LifecycleScope::Class, class_path.to_string()));
        
        self.active_scopes.insert(
            class_path.to_string(),
            LifecycleState {
                scope: LifecycleScope::Class,
                identifier: class_path.to_string(),
                setup_completed: true,
                teardown_completed: false,
                setup_failed: false,
            },
        );
        
        Ok(())
    }
    
    /// Record a setup failure
    pub fn record_setup_failure(&mut self, scope: LifecycleScope, identifier: &str) {
        if let Some(state) = self.active_scopes.get_mut(identifier) {
            state.setup_failed = true;
        } else {
            self.active_scopes.insert(
                identifier.to_string(),
                LifecycleState {
                    scope,
                    identifier: identifier.to_string(),
                    setup_completed: false,
                    teardown_completed: false,
                    setup_failed: true,
                },
            );
        }
    }
    
    /// Get the teardown order (reverse of setup order)
    pub fn get_teardown_order(&self) -> Vec<(LifecycleScope, String)> {
        let mut teardown_order = self.setup_order.clone();
        teardown_order.reverse();
        teardown_order
    }
    
    /// Record that a teardown has been completed
    pub fn record_teardown(&mut self, identifier: &str) {
        if let Some(state) = self.active_scopes.get_mut(identifier) {
            state.teardown_completed = true;
        }
        
        // Remove from tracking sets
        self.setup_modules.remove(identifier);
        self.setup_classes.remove(identifier);
    }
    
    /// Check if a scope had a setup failure
    pub fn had_setup_failure(&self, identifier: &str) -> bool {
        self.active_scopes
            .get(identifier)
            .map(|state| state.setup_failed)
            .unwrap_or(false)
    }
    
    /// Clear all state (useful for test isolation)
    pub fn clear(&mut self) {
        self.setup_order.clear();
        self.active_scopes.clear();
        self.setup_modules.clear();
        self.setup_classes.clear();
    }
    
    /// Get a summary of the current lifecycle state
    pub fn get_summary(&self) -> HashMap<String, String> {
        let mut summary = HashMap::new();
        
        summary.insert("active_modules".to_string(), self.setup_modules.len().to_string());
        summary.insert("active_classes".to_string(), self.setup_classes.len().to_string());
        summary.insert("total_scopes".to_string(), self.active_scopes.len().to_string());
        
        summary
    }
}

/// Python integration for lifecycle management
/// This will be called from the Python worker to coordinate setup/teardown
pub struct PythonLifecycleCoordinator {
    manager: LifecycleManager,
}

impl PythonLifecycleCoordinator {
    pub fn new() -> Self {
        Self {
            manager: LifecycleManager::new(),
        }
    }
    
    /// Execute module setup if needed
    pub fn setup_module_if_needed(
        &mut self,
        _py: Python,
        module_path: &str,
        has_setup: bool,
    ) -> PyResult<bool> {
        if !self.manager.needs_module_setup(module_path) {
            return Ok(false);
        }
        
        if has_setup {
            // The actual execution will be handled by Python worker
            self.manager.record_module_setup(module_path)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
            Ok(true)
        } else {
            // No setup needed, but record that we checked
            self.manager.setup_modules.insert(module_path.to_string());
            Ok(false)
        }
    }
    
    /// Execute class setup if needed
    pub fn setup_class_if_needed(
        &mut self,
        _py: Python,
        class_path: &str,
        has_setup: bool,
    ) -> PyResult<bool> {
        if !self.manager.needs_class_setup(class_path) {
            return Ok(false);
        }
        
        if has_setup {
            // The actual execution will be handled by Python worker
            self.manager.record_class_setup(class_path)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
            Ok(true)
        } else {
            // No setup needed, but record that we checked
            self.manager.setup_classes.insert(class_path.to_string());
            Ok(false)
        }
    }
    
    /// Get the teardown order for cleanup
    pub fn get_teardown_order(&self) -> Vec<(String, String)> {
        self.manager.get_teardown_order()
            .into_iter()
            .map(|(scope, id)| (format!("{:?}", scope), id))
            .collect()
    }
    
    /// Record a teardown completion
    pub fn record_teardown(&mut self, identifier: &str) {
        self.manager.record_teardown(identifier);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_lifecycle_order() {
        let mut manager = LifecycleManager::new();
        
        // Setup in order: module -> class -> method
        assert!(manager.needs_module_setup("test_module"));
        manager.record_module_setup("test_module").unwrap();
        assert!(!manager.needs_module_setup("test_module"));
        
        assert!(manager.needs_class_setup("test_module::TestClass"));
        manager.record_class_setup("test_module::TestClass").unwrap();
        assert!(!manager.needs_class_setup("test_module::TestClass"));
        
        // Teardown should be in reverse order
        let teardown_order = manager.get_teardown_order();
        assert_eq!(teardown_order.len(), 2);
        assert_eq!(teardown_order[0].1, "test_module::TestClass");
        assert_eq!(teardown_order[1].1, "test_module");
    }
    
    #[test]
    fn test_setup_failure_tracking() {
        let mut manager = LifecycleManager::new();
        
        manager.record_setup_failure(LifecycleScope::Module, "failing_module");
        assert!(manager.had_setup_failure("failing_module"));
        assert!(!manager.had_setup_failure("other_module"));
    }
}