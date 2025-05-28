//! Smart Hook Registry System
//!
//! Fast, minimal hook system compatible with pytest using external libraries

use anyhow::Result;
use async_trait::async_trait;
use dashmap::DashMap;
use linkme::distributed_slice;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{timeout, Duration};
use uuid::Uuid;

/// Hook function signature for maximum flexibility
pub type HookFn = Box<dyn Fn(&HookData) -> Result<HookResult> + Send + Sync>;

/// Hook result with efficient variants
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HookResult {
    None,
    Value(serde_json::Value),
    Stop,
    Skip,
    Modified(serde_json::Value),
    Error(String),
}

/// Hook data passed to hook functions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookData {
    pub name: String,
    pub args: serde_json::Value,
    pub context: HashMap<String, serde_json::Value>,
    pub test_id: Option<String>,
    pub session_id: String,
}

/// Hook specification for compile-time optimization
#[derive(Debug, Clone)]
pub struct HookSpec {
    pub name: &'static str,
    pub firstresult: bool,
    pub historic: bool,
    pub warn_on_impl: bool,
}

/// Fast hook registry using dashmap for concurrent access
pub struct HookRegistry {
    hooks: DashMap<String, Vec<HookFn>>,
    specs: DashMap<String, HookSpec>,
    timeout_ms: u64,
}

impl HookRegistry {
    pub fn new() -> Self {
        let registry = Self {
            hooks: DashMap::new(),
            specs: DashMap::new(),
            timeout_ms: 5000,
        };

        // Register all pytest hooks at startup
        registry.register_pytest_hooks();
        registry
    }

    /// Register a hook function with compile-time optimization
    pub fn add_hook<F>(&self, name: &str, hook_fn: F) -> Result<()>
    where
        F: Fn(&HookData) -> Result<HookResult> + Send + Sync + 'static,
    {
        let mut hooks = self.hooks.entry(name.to_string()).or_insert_with(Vec::new);
        hooks.push(Box::new(hook_fn));
        Ok(())
    }

    /// Call hook with automatic timeout and error handling
    pub async fn call_hook(&self, name: &str, data: HookData) -> Result<Vec<HookResult>> {
        let hooks = match self.hooks.get(name) {
            Some(hooks) => hooks,
            None => return Ok(vec![]),
        };

        let spec = self.specs.get(name);
        let firstresult = spec.as_ref().map(|s| s.firstresult).unwrap_or(false);

        let mut results = Vec::new();

        for hook_fn in hooks.iter() {
            // Execute with timeout
            let result = timeout(Duration::from_millis(self.timeout_ms), async {
                hook_fn(&data)
            })
            .await;

            match result {
                Ok(Ok(hook_result)) => {
                    // Handle firstresult optimization
                    if firstresult && !matches!(hook_result, HookResult::None) {
                        return Ok(vec![hook_result]);
                    }
                    results.push(hook_result);
                }
                Ok(Err(e)) => {
                    results.push(HookResult::Error(e.to_string()));
                }
                Err(_) => {
                    results.push(HookResult::Error("Hook timeout".to_string()));
                }
            }
        }

        Ok(results)
    }

    /// Get the total number of registered hooks
    pub fn hook_count(&self) -> usize {
        self.hooks.iter().map(|entry| entry.value().len()).sum()
    }

    /// Synchronous hook call for performance-critical paths
    pub fn call_hook_sync(&self, name: &str, data: HookData) -> Result<Vec<HookResult>> {
        let hooks = match self.hooks.get(name) {
            Some(hooks) => hooks,
            None => return Ok(vec![]),
        };

        let spec = self.specs.get(name);
        let firstresult = spec.as_ref().map(|s| s.firstresult).unwrap_or(false);

        let mut results = Vec::new();

        for hook_fn in hooks.iter() {
            match hook_fn(&data) {
                Ok(hook_result) => {
                    if firstresult && !matches!(hook_result, HookResult::None) {
                        return Ok(vec![hook_result]);
                    }
                    results.push(hook_result);
                }
                Err(e) => {
                    results.push(HookResult::Error(e.to_string()));
                }
            }
        }

        Ok(results)
    }

    fn register_pytest_hooks(&self) {
        let hooks = [
            // Configuration hooks
            HookSpec {
                name: "pytest_configure",
                firstresult: false,
                historic: false,
                warn_on_impl: false,
            },
            HookSpec {
                name: "pytest_unconfigure",
                firstresult: false,
                historic: false,
                warn_on_impl: false,
            },
            HookSpec {
                name: "pytest_addoption",
                firstresult: false,
                historic: false,
                warn_on_impl: false,
            },
            // Collection hooks
            HookSpec {
                name: "pytest_collect_file",
                firstresult: true,
                historic: false,
                warn_on_impl: false,
            },
            HookSpec {
                name: "pytest_collection_modifyitems",
                firstresult: false,
                historic: false,
                warn_on_impl: false,
            },
            HookSpec {
                name: "pytest_collection_finish",
                firstresult: false,
                historic: false,
                warn_on_impl: false,
            },
            // Test execution hooks
            HookSpec {
                name: "pytest_runtest_setup",
                firstresult: false,
                historic: false,
                warn_on_impl: false,
            },
            HookSpec {
                name: "pytest_runtest_call",
                firstresult: false,
                historic: false,
                warn_on_impl: false,
            },
            HookSpec {
                name: "pytest_runtest_teardown",
                firstresult: false,
                historic: false,
                warn_on_impl: false,
            },
            HookSpec {
                name: "pytest_runtest_makereport",
                firstresult: true,
                historic: false,
                warn_on_impl: false,
            },
            HookSpec {
                name: "pytest_runtest_logreport",
                firstresult: false,
                historic: false,
                warn_on_impl: false,
            },
            // Fixture hooks
            HookSpec {
                name: "pytest_fixture_setup",
                firstresult: true,
                historic: false,
                warn_on_impl: false,
            },
            HookSpec {
                name: "pytest_fixture_post_finalizer",
                firstresult: false,
                historic: false,
                warn_on_impl: false,
            },
            // Session hooks
            HookSpec {
                name: "pytest_sessionstart",
                firstresult: false,
                historic: false,
                warn_on_impl: false,
            },
            HookSpec {
                name: "pytest_sessionfinish",
                firstresult: false,
                historic: false,
                warn_on_impl: false,
            },
            // Reporting hooks
            HookSpec {
                name: "pytest_report_header",
                firstresult: false,
                historic: false,
                warn_on_impl: false,
            },
            HookSpec {
                name: "pytest_terminal_summary",
                firstresult: false,
                historic: false,
                warn_on_impl: false,
            },
        ];

        for spec in &hooks {
            self.specs.insert(spec.name.to_string(), spec.clone());
        }
    }
}

/// Pytest hook implementations for compatibility
pub struct PytestHooks {
    registry: Arc<HookRegistry>,
}

impl PytestHooks {
    pub fn new(registry: Arc<HookRegistry>) -> Self {
        Self { registry }
    }

    /// Configuration hooks
    pub async fn pytest_configure(&self, config: serde_json::Value) -> Result<Vec<HookResult>> {
        let data = HookData {
            name: "pytest_configure".to_string(),
            args: serde_json::json!({ "config": config }),
            context: HashMap::new(),
            test_id: None,
            session_id: Uuid::new_v4().to_string(),
        };
        self.registry.call_hook("pytest_configure", data).await
    }

    pub async fn pytest_addoption(&self, parser: serde_json::Value) -> Result<Vec<HookResult>> {
        let data = HookData {
            name: "pytest_addoption".to_string(),
            args: serde_json::json!({ "parser": parser }),
            context: HashMap::new(),
            test_id: None,
            session_id: Uuid::new_v4().to_string(),
        };
        self.registry.call_hook("pytest_addoption", data).await
    }

    /// Collection hooks
    pub async fn pytest_collect_file(
        &self,
        path: &str,
        parent: Option<&str>,
    ) -> Result<Vec<HookResult>> {
        let data = HookData {
            name: "pytest_collect_file".to_string(),
            args: serde_json::json!({ "path": path, "parent": parent }),
            context: HashMap::new(),
            test_id: None,
            session_id: Uuid::new_v4().to_string(),
        };
        self.registry.call_hook("pytest_collect_file", data).await
    }

    pub async fn pytest_collection_modifyitems(
        &self,
        session: serde_json::Value,
        config: serde_json::Value,
        items: &[serde_json::Value],
    ) -> Result<Vec<HookResult>> {
        let data = HookData {
            name: "pytest_collection_modifyitems".to_string(),
            args: serde_json::json!({ "session": session, "config": config, "items": items }),
            context: HashMap::new(),
            test_id: None,
            session_id: Uuid::new_v4().to_string(),
        };
        self.registry
            .call_hook("pytest_collection_modifyitems", data)
            .await
    }

    /// Test execution hooks
    pub async fn pytest_runtest_setup(&self, item: serde_json::Value) -> Result<Vec<HookResult>> {
        let data = HookData {
            name: "pytest_runtest_setup".to_string(),
            args: serde_json::json!({ "item": item }),
            context: HashMap::new(),
            test_id: item.get("id").and_then(|v| v.as_str()).map(String::from),
            session_id: Uuid::new_v4().to_string(),
        };
        self.registry.call_hook("pytest_runtest_setup", data).await
    }

    pub async fn pytest_runtest_call(&self, item: serde_json::Value) -> Result<Vec<HookResult>> {
        let data = HookData {
            name: "pytest_runtest_call".to_string(),
            args: serde_json::json!({ "item": item }),
            context: HashMap::new(),
            test_id: item.get("id").and_then(|v| v.as_str()).map(String::from),
            session_id: Uuid::new_v4().to_string(),
        };
        self.registry.call_hook("pytest_runtest_call", data).await
    }

    pub async fn pytest_runtest_teardown(
        &self,
        item: serde_json::Value,
    ) -> Result<Vec<HookResult>> {
        let data = HookData {
            name: "pytest_runtest_teardown".to_string(),
            args: serde_json::json!({ "item": item }),
            context: HashMap::new(),
            test_id: item.get("id").and_then(|v| v.as_str()).map(String::from),
            session_id: Uuid::new_v4().to_string(),
        };
        self.registry
            .call_hook("pytest_runtest_teardown", data)
            .await
    }

    /// Fixture hooks  
    pub async fn pytest_fixture_setup(
        &self,
        fixturedef: serde_json::Value,
        request: serde_json::Value,
    ) -> Result<Vec<HookResult>> {
        let data = HookData {
            name: "pytest_fixture_setup".to_string(),
            args: serde_json::json!({ "fixturedef": fixturedef, "request": request }),
            context: HashMap::new(),
            test_id: None,
            session_id: Uuid::new_v4().to_string(),
        };
        self.registry.call_hook("pytest_fixture_setup", data).await
    }

    /// Session hooks
    pub async fn pytest_sessionstart(&self, session: serde_json::Value) -> Result<Vec<HookResult>> {
        let data = HookData {
            name: "pytest_sessionstart".to_string(),
            args: serde_json::json!({ "session": session }),
            context: HashMap::new(),
            test_id: None,
            session_id: Uuid::new_v4().to_string(),
        };
        self.registry.call_hook("pytest_sessionstart", data).await
    }

    pub async fn pytest_sessionfinish(
        &self,
        session: serde_json::Value,
        exitstatus: i32,
    ) -> Result<Vec<HookResult>> {
        let data = HookData {
            name: "pytest_sessionfinish".to_string(),
            args: serde_json::json!({ "session": session, "exitstatus": exitstatus }),
            context: HashMap::new(),
            test_id: None,
            session_id: Uuid::new_v4().to_string(),
        };
        self.registry.call_hook("pytest_sessionfinish", data).await
    }
}

/// Static hook registration using linkme for zero-overhead
#[distributed_slice]
pub static BUILTIN_HOOKS: [fn(&HookRegistry) -> Result<()>] = [..];

/// Register built-in hooks at startup
pub fn register_builtin_hooks(registry: &HookRegistry) -> Result<()> {
    for register_fn in BUILTIN_HOOKS {
        register_fn(registry)?;
    }
    Ok(())
}

/// Helper macro for registering hooks statically
#[macro_export]
macro_rules! register_hook {
    ($name:literal, $fn:expr) => {
        #[linkme::distributed_slice(crate::plugin::hooks::BUILTIN_HOOKS)]
        fn register_hook_fn(registry: &crate::plugin::hooks::HookRegistry) -> anyhow::Result<()> {
            registry.add_hook($name, $fn)
        }
    };
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hook_registry() {
        let registry = HookRegistry::new();

        // Register a test hook
        registry
            .add_hook("test_hook", |data| {
                Ok(HookResult::Value(
                    serde_json::json!({"received": data.name}),
                ))
            })
            .unwrap();

        // Call the hook
        let data = HookData {
            name: "test_hook".to_string(),
            args: serde_json::json!({}),
            context: HashMap::new(),
            test_id: None,
            session_id: "test".to_string(),
        };

        let results = registry.call_hook("test_hook", data).await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_pytest_hooks_registration() {
        let registry = HookRegistry::new();

        // Should have pytest hooks registered
        assert!(registry.specs.contains_key("pytest_configure"));
        assert!(registry.specs.contains_key("pytest_runtest_setup"));
        assert!(registry.specs.contains_key("pytest_collection_modifyitems"));
    }
}
