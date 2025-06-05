//! Hook definitions for the Fastest plugin system
//!
//! This module defines all the hooks that plugins can implement,
//! similar to pytest's hook system.

use std::any::Any;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use crate::test::discovery::TestItem;
use crate::error::Result;

/// Result type for hook execution
pub type HookResult<T> = Result<T>;

/// Represents a callable hook
pub trait Hook: Send + Sync {
    /// Execute the hook with given arguments
    fn call(&self, args: &dyn Any) -> HookResult<Box<dyn Any>>;
    
    /// Get the name of the hook
    fn name(&self) -> &str;
}

/// Helper trait for creating type-safe hook callers
pub trait HookCaller<T> {
    /// Call the hook with typed arguments
    fn call(&self, args: T) -> HookResult<Box<dyn Any>>;
}

/// Configuration hook arguments
#[derive(Debug)]
pub struct ConfigureArgs {
    pub config: crate::config::Config,
    pub plugin_config: super::PluginConfig,
}

/// Collection modification hook arguments
#[derive(Debug)]
pub struct CollectionModifyItemsArgs {
    pub items: Vec<TestItem>,
    pub config: crate::config::Config,
}

/// Test setup hook arguments
#[derive(Debug)]
pub struct TestSetupArgs {
    pub item: TestItem,
    pub fixtures: HashMap<String, Box<dyn Any>>,
}

/// Test call hook arguments
#[derive(Debug)]
pub struct TestCallArgs {
    pub item: TestItem,
    pub fixtures: HashMap<String, Box<dyn Any>>,
}

/// Test teardown hook arguments
#[derive(Debug)]
pub struct TestTeardownArgs {
    pub item: TestItem,
    pub fixtures: HashMap<String, Box<dyn Any>>,
    pub exception: Option<String>,
}

/// Test report hook arguments
#[derive(Debug)]
pub struct TestReportArgs {
    pub item: TestItem,
    pub duration: std::time::Duration,
    pub outcome: TestOutcome,
    pub stdout: String,
    pub stderr: String,
    pub exception: Option<String>,
}

/// Test outcome
#[derive(Debug, Clone, PartialEq)]
pub enum TestOutcome {
    Passed,
    Failed,
    Skipped,
    XFailed,
    XPassed,
    Error,
}

/// Collection start hook arguments
#[derive(Debug)]
pub struct CollectionStartArgs {
    pub test_paths: Vec<PathBuf>,
}

/// Collection finish hook arguments
#[derive(Debug)]
pub struct CollectionFinishArgs {
    pub items: Vec<TestItem>,
    pub duration: std::time::Duration,
}

/// Session start hook arguments
#[derive(Debug)]
pub struct SessionStartArgs {
    pub config: crate::config::Config,
}

/// Session finish hook arguments
#[derive(Debug)]
pub struct SessionFinishArgs {
    pub exit_code: i32,
    pub duration: std::time::Duration,
}

/// Standard hook implementations
pub mod hooks {
    use super::*;
    
    /// Hook called when Fastest is being configured
    pub struct ConfigureHook;
    
    impl Hook for ConfigureHook {
        fn call(&self, args: &dyn Any) -> HookResult<Box<dyn Any>> {
            // Default implementation
            Ok(Box::new(()))
        }
        
        fn name(&self) -> &str {
            "pytest_configure"
        }
    }
    
    /// Hook called to modify collected test items
    pub struct CollectionModifyItemsHook;
    
    impl Hook for CollectionModifyItemsHook {
        fn call(&self, args: &dyn Any) -> HookResult<Box<dyn Any>> {
            // Default implementation
            if let Some(args) = args.downcast_ref::<CollectionModifyItemsArgs>() {
                Ok(Box::new(args.items.clone()))
            } else {
                Ok(Box::new(()))
            }
        }
        
        fn name(&self) -> &str {
            "pytest_collection_modifyitems"
        }
    }
    
    /// Hook called before running a test
    pub struct TestSetupHook;
    
    impl Hook for TestSetupHook {
        fn call(&self, args: &dyn Any) -> HookResult<Box<dyn Any>> {
            // Default implementation
            Ok(Box::new(()))
        }
        
        fn name(&self) -> &str {
            "pytest_runtest_setup"
        }
    }
    
    /// Hook called to run a test
    pub struct TestCallHook;
    
    impl Hook for TestCallHook {
        fn call(&self, args: &dyn Any) -> HookResult<Box<dyn Any>> {
            // Default implementation
            Ok(Box::new(()))
        }
        
        fn name(&self) -> &str {
            "pytest_runtest_call"
        }
    }
    
    /// Hook called after running a test
    pub struct TestTeardownHook;
    
    impl Hook for TestTeardownHook {
        fn call(&self, args: &dyn Any) -> HookResult<Box<dyn Any>> {
            // Default implementation
            Ok(Box::new(()))
        }
        
        fn name(&self) -> &str {
            "pytest_runtest_teardown"
        }
    }
    
    /// Hook called to create a test report
    pub struct TestMakeReportHook;
    
    impl Hook for TestMakeReportHook {
        fn call(&self, args: &dyn Any) -> HookResult<Box<dyn Any>> {
            // Default implementation
            Ok(Box::new(()))
        }
        
        fn name(&self) -> &str {
            "pytest_runtest_makereport"
        }
    }
}

/// Hook specification for plugins to implement
pub trait HookSpec {
    /// Called when Fastest is being configured
    fn pytest_configure(&self, config: &mut crate::config::Config) -> HookResult<()> {
        Ok(())
    }
    
    /// Called after collection has been performed
    fn pytest_collection_modifyitems(
        &self, 
        items: &mut Vec<TestItem>,
        config: &crate::config::Config
    ) -> HookResult<()> {
        Ok(())
    }
    
    /// Called to perform setup for a test
    fn pytest_runtest_setup(
        &self,
        item: &TestItem,
        fixtures: &HashMap<String, Box<dyn Any>>
    ) -> HookResult<()> {
        Ok(())
    }
    
    /// Called to run a test
    fn pytest_runtest_call(
        &self,
        item: &TestItem,
        fixtures: &HashMap<String, Box<dyn Any>>
    ) -> HookResult<()> {
        Ok(())
    }
    
    /// Called to perform teardown for a test
    fn pytest_runtest_teardown(
        &self,
        item: &TestItem,
        fixtures: &HashMap<String, Box<dyn Any>>,
        exception: Option<&str>
    ) -> HookResult<()> {
        Ok(())
    }
    
    /// Called to create a test report
    fn pytest_runtest_makereport(
        &self,
        item: &TestItem,
        outcome: &TestOutcome,
        duration: std::time::Duration,
        exception: Option<&str>
    ) -> HookResult<TestReport> {
        Ok(TestReport {
            item_id: item.id.clone(),
            outcome: outcome.clone(),
            duration,
            exception: exception.map(String::from),
            stdout: String::new(),
            stderr: String::new(),
        })
    }
    
    /// Called when collection starts
    fn pytest_collection_start(&self, test_paths: &[PathBuf]) -> HookResult<()> {
        Ok(())
    }
    
    /// Called when collection finishes
    fn pytest_collection_finish(
        &self, 
        items: &[TestItem],
        duration: std::time::Duration
    ) -> HookResult<()> {
        Ok(())
    }
    
    /// Called when test session starts
    fn pytest_sessionstart(&self, config: &crate::config::Config) -> HookResult<()> {
        Ok(())
    }
    
    /// Called when test session finishes
    fn pytest_sessionfinish(
        &self,
        exit_code: i32,
        duration: std::time::Duration
    ) -> HookResult<()> {
        Ok(())
    }
}

/// Test report structure
#[derive(Debug, Clone)]
pub struct TestReport {
    pub item_id: String,
    pub outcome: TestOutcome,
    pub duration: std::time::Duration,
    pub exception: Option<String>,
    pub stdout: String,
    pub stderr: String,
}

/// Wrapper to make a HookSpec implementation into a callable Hook
pub struct HookWrapper<F> {
    name: String,
    func: Arc<F>,
}

impl<F> HookWrapper<F> 
where
    F: Fn(&dyn Any) -> HookResult<Box<dyn Any>> + Send + Sync + 'static
{
    pub fn new(name: impl Into<String>, func: F) -> Self {
        Self {
            name: name.into(),
            func: Arc::new(func),
        }
    }
}

impl<F> Hook for HookWrapper<F>
where
    F: Fn(&dyn Any) -> HookResult<Box<dyn Any>> + Send + Sync + 'static
{
    fn call(&self, args: &dyn Any) -> HookResult<Box<dyn Any>> {
        (self.func)(args)
    }
    
    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_hook_wrapper() {
        let hook = HookWrapper::new("test_hook", |_args| {
            Ok(Box::new(42))
        });
        
        assert_eq!(hook.name(), "test_hook");
        
        let result = hook.call(&()).unwrap();
        let value = result.downcast_ref::<i32>().unwrap();
        assert_eq!(*value, 42);
    }
    
    #[test]
    fn test_test_outcome() {
        let outcome = TestOutcome::Passed;
        assert_eq!(outcome, TestOutcome::Passed);
        
        let outcome = TestOutcome::Failed;
        assert_eq!(outcome, TestOutcome::Failed);
    }
}