//! Hook System - Type-safe, high-performance hook mechanism
//!
//! This module implements a pytest-compatible hook system with Rust's type safety.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use parking_lot::RwLock;
use async_trait::async_trait;

use crate::api::{PluginError, PluginResult};
use fastest_core::TestItem;

/// Result type for hook execution
pub type HookResult<T> = Result<T, HookError>;

/// Hook execution errors
#[derive(Debug, thiserror::Error)]
pub enum HookError {
    #[error("Hook execution failed: {0}")]
    ExecutionFailed(String),
    
    #[error("Hook not found: {0}")]
    NotFound(String),
    
    #[error("Invalid hook result")]
    InvalidResult,
    
    #[error("Hook cancelled")]
    Cancelled,
}

/// Trait for all hooks
pub trait Hook: Send + Sync + Debug {
    /// Hook name
    fn name(&self) -> &str;
    
    /// Execute the hook
    fn execute(&self, args: HookArgs) -> HookResult<HookReturn>;
    
    /// Whether this hook is async
    fn is_async(&self) -> bool {
        false
    }
}

/// Async hook trait
#[async_trait]
pub trait AsyncHook: Send + Sync + Debug {
    /// Hook name
    fn name(&self) -> &str;
    
    /// Execute the hook asynchronously
    async fn execute_async(&self, args: HookArgs) -> HookResult<HookReturn>;
}

/// Hook arguments container
#[derive(Debug)]
pub struct HookArgs {
    args: HashMap<String, Box<dyn Any + Send + Sync>>,
}

impl HookArgs {
    pub fn new() -> Self {
        Self {
            args: HashMap::new(),
        }
    }
    
    pub fn insert<T: Any + Send + Sync>(&mut self, key: &str, value: T) {
        self.args.insert(key.to_string(), Box::new(value));
    }
    
    pub fn get<T: Any + Send + Sync>(&self, key: &str) -> Option<&T> {
        self.args.get(key)?.downcast_ref()
    }
    
    pub fn get_mut<T: Any + Send + Sync>(&mut self, key: &str) -> Option<&mut T> {
        self.args.get_mut(key)?.downcast_mut()
    }
}

/// Hook return value container
#[derive(Debug)]
pub enum HookReturn {
    /// No return value
    None,
    
    /// Boolean result
    Bool(bool),
    
    /// String result
    String(String),
    
    /// Generic boxed result
    Value(Box<dyn Any + Send + Sync>),
    
    /// Multiple results from different implementations
    Multiple(Vec<HookReturn>),
}

impl HookReturn {
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            HookReturn::Bool(b) => Some(*b),
            _ => None,
        }
    }
    
    pub fn as_string(&self) -> Option<&str> {
        match self {
            HookReturn::String(s) => Some(s),
            _ => None,
        }
    }
    
    pub fn as_value<T: Any + Send + Sync>(&self) -> Option<&T> {
        match self {
            HookReturn::Value(v) => v.downcast_ref(),
            _ => None,
        }
    }
}

/// Hook implementation wrapper
struct HookImpl {
    /// Plugin name that registered this hook
    plugin_name: String,
    
    /// Hook priority (higher = earlier execution)
    priority: i32,
    
    /// The actual hook implementation
    hook: Box<dyn Hook>,
}

/// Hook registry for managing all hooks
pub struct HookRegistry {
    /// Map of hook name to implementations
    hooks: Arc<RwLock<HashMap<String, Vec<HookImpl>>>>,
    
    /// Hook call history for debugging
    history: Arc<RwLock<Vec<String>>>,
}

impl HookRegistry {
    pub fn new() -> Self {
        Self {
            hooks: Arc::new(RwLock::new(HashMap::new())),
            history: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Register a hook implementation
    pub fn register(
        &self,
        hook_name: &str,
        plugin_name: &str,
        priority: i32,
        hook: Box<dyn Hook>,
    ) {
        let mut hooks = self.hooks.write();
        let hook_impl = HookImpl {
            plugin_name: plugin_name.to_string(),
            priority,
            hook,
        };
        
        hooks.entry(hook_name.to_string())
            .or_insert_with(Vec::new)
            .push(hook_impl);
        
        // Sort by priority (descending)
        if let Some(impls) = hooks.get_mut(hook_name) {
            impls.sort_by(|a, b| b.priority.cmp(&a.priority));
        }
    }
    
    /// Call a hook with the given arguments
    pub fn call(&self, hook_name: &str, args: HookArgs) -> HookResult<HookReturn> {
        let hooks = self.hooks.read();
        
        if let Some(impls) = hooks.get(hook_name) {
            // Record in history
            self.history.write().push(hook_name.to_string());
            
            let mut results = Vec::new();
            
            for hook_impl in impls {
                match hook_impl.hook.execute(args) {
                    Ok(result) => results.push(result),
                    Err(HookError::Cancelled) => {
                        // Stop processing if a hook cancels
                        return Ok(HookReturn::Bool(false));
                    }
                    Err(e) => {
                        // Log error but continue with other hooks
                        eprintln!("Hook {} from {} failed: {}", 
                                  hook_name, hook_impl.plugin_name, e);
                    }
                }
            }
            
            // Return results based on count
            match results.len() {
                0 => Ok(HookReturn::None),
                1 => Ok(results.into_iter().next().unwrap()),
                _ => Ok(HookReturn::Multiple(results)),
            }
        } else {
            Ok(HookReturn::None)
        }
    }
    
    /// Get hook call history
    pub fn history(&self) -> Vec<String> {
        self.history.read().clone()
    }
}

/// Hook caller for convenient hook invocation
pub struct HookCaller<'a> {
    registry: &'a HookRegistry,
    hook_name: String,
    args: HookArgs,
}

impl<'a> HookCaller<'a> {
    pub fn new(registry: &'a HookRegistry, hook_name: &str) -> Self {
        Self {
            registry,
            hook_name: hook_name.to_string(),
            args: HookArgs::new(),
        }
    }
    
    pub fn arg<T: Any + Send + Sync>(mut self, key: &str, value: T) -> Self {
        self.args.insert(key, value);
        self
    }
    
    pub fn call(self) -> HookResult<HookReturn> {
        self.registry.call(&self.hook_name, self.args)
    }
}

// Define standard pytest-compatible hooks

/// Configuration hook
pub struct ConfigureHook;

impl Hook for ConfigureHook {
    fn name(&self) -> &str {
        "pytest_configure"
    }
    
    fn execute(&self, _args: HookArgs) -> HookResult<HookReturn> {
        Ok(HookReturn::None)
    }
}

/// Collection modification hook
pub struct CollectionModifyItemsHook {
    handler: Box<dyn Fn(&mut Vec<TestItem>) + Send + Sync>,
}

impl CollectionModifyItemsHook {
    pub fn new<F>(handler: F) -> Self
    where
        F: Fn(&mut Vec<TestItem>) + Send + Sync + 'static,
    {
        Self {
            handler: Box::new(handler),
        }
    }
}

impl Hook for CollectionModifyItemsHook {
    fn name(&self) -> &str {
        "pytest_collection_modifyitems"
    }
    
    fn execute(&self, mut args: HookArgs) -> HookResult<HookReturn> {
        if let Some(items) = args.get_mut::<Vec<TestItem>>("items") {
            (self.handler)(items);
        }
        Ok(HookReturn::None)
    }
}

impl Debug for CollectionModifyItemsHook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CollectionModifyItemsHook").finish()
    }
}

/// Test execution hooks
#[derive(Debug)]
pub struct RunTestSetupHook;

impl Hook for RunTestSetupHook {
    fn name(&self) -> &str {
        "pytest_runtest_setup"
    }
    
    fn execute(&self, _args: HookArgs) -> HookResult<HookReturn> {
        Ok(HookReturn::None)
    }
}

#[derive(Debug)]
pub struct RunTestCallHook;

impl Hook for RunTestCallHook {
    fn name(&self) -> &str {
        "pytest_runtest_call"
    }
    
    fn execute(&self, _args: HookArgs) -> HookResult<HookReturn> {
        Ok(HookReturn::None)
    }
}

#[derive(Debug)]
pub struct RunTestTeardownHook;

impl Hook for RunTestTeardownHook {
    fn name(&self) -> &str {
        "pytest_runtest_teardown"
    }
    
    fn execute(&self, _args: HookArgs) -> HookResult<HookReturn> {
        Ok(HookReturn::None)
    }
}

/// Macro for creating simple hooks
#[macro_export]
macro_rules! simple_hook {
    ($name:ident, $hook_name:expr) => {
        #[derive(Debug)]
        pub struct $name;
        
        impl Hook for $name {
            fn name(&self) -> &str {
                $hook_name
            }
            
            fn execute(&self, _args: HookArgs) -> HookResult<HookReturn> {
                Ok(HookReturn::None)
            }
        }
    };
}

// Export commonly used hooks
simple_hook!(SessionStartHook, "pytest_sessionstart");
simple_hook!(SessionFinishHook, "pytest_sessionfinish");
simple_hook!(CollectionStartHook, "pytest_collection_start");
simple_hook!(CollectionFinishHook, "pytest_collection_finish");