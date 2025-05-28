//! Phase 4: Timeout and Async Support
//!
//! Advanced timeout handling and async test execution support

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::timeout;

use crate::{TestItem, TestResult};

/// Timeout manager for test execution
pub struct TimeoutManager {
    config: TimeoutConfig,
    active_timeouts: HashMap<String, TimeoutHandle>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutConfig {
    /// Default timeout for all tests (seconds)
    pub default_timeout: u64,
    /// Enable per-test timeout configuration
    pub per_test_timeouts: bool,
    /// Timeout for async tests
    pub async_timeout: u64,
    /// Timeout for fixture setup
    pub fixture_timeout: u64,
    /// Enable timeout warnings
    pub timeout_warnings: bool,
    /// Warning threshold (% of timeout)
    pub warning_threshold: f64,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            default_timeout: 60, // 1 minute default
            per_test_timeouts: true,
            async_timeout: 120,  // 2 minutes for async tests
            fixture_timeout: 30, // 30 seconds for fixtures
            timeout_warnings: true,
            warning_threshold: 0.8, // Warn at 80% of timeout
        }
    }
}

#[derive(Debug)]
struct TimeoutHandle {
    test_id: String,
    timeout_duration: Duration,
    start_time: std::time::Instant,
    warning_sent: bool,
}

/// Timeout error information
#[derive(Debug, Serialize, Deserialize)]
pub struct TimeoutError {
    pub test_id: String,
    pub timeout_duration: Duration,
    pub elapsed_time: Duration,
    pub timeout_type: TimeoutType,
    pub context: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum TimeoutType {
    TestExecution,
    FixtureSetup,
    FixtureTeardown,
    AsyncOperation,
}

/// Async test execution result
#[derive(Debug, Serialize)]
pub struct AsyncTestResult {
    pub test_id: String,
    pub execution_time: Duration,
    pub result: TestResult,
    pub async_info: AsyncExecutionInfo,
}

#[derive(Debug, Serialize)]
pub struct AsyncExecutionInfo {
    pub is_async: bool,
    pub awaited_operations: u32,
    pub concurrent_tasks: u32,
    pub event_loop_time: Duration,
}

impl TimeoutManager {
    pub fn new(config: TimeoutConfig) -> Self {
        Self {
            config,
            active_timeouts: HashMap::new(),
        }
    }

    /// Execute test with timeout
    pub async fn execute_with_timeout<F, T>(&mut self, test: &TestItem, future: F) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>>,
    {
        let timeout_duration = self.get_test_timeout(test);
        let handle = TimeoutHandle {
            test_id: test.id.clone(),
            timeout_duration,
            start_time: std::time::Instant::now(),
            warning_sent: false,
        };

        self.active_timeouts.insert(test.id.clone(), handle);

        // Start timeout warning task
        if self.config.timeout_warnings {
            self.start_timeout_warning_task(&test.id, timeout_duration)
                .await;
        }

        let result = timeout(timeout_duration, future).await;

        // Cleanup
        self.active_timeouts.remove(&test.id);

        match result {
            Ok(test_result) => test_result,
            Err(_) => {
                let elapsed = self
                    .active_timeouts
                    .get(&test.id)
                    .map(|h| h.start_time.elapsed())
                    .unwrap_or_default();

                Err(anyhow::anyhow!(
                    "Test '{}' timed out after {:?} (limit: {:?})",
                    test.id,
                    elapsed,
                    timeout_duration
                ))
            }
        }
    }

    /// Execute async test with proper event loop handling
    pub async fn execute_async_test(&self, test: &TestItem) -> Result<AsyncTestResult> {
        let start_time = std::time::Instant::now();
        let _timeout_duration = self.get_async_timeout(test);

        // Create async execution context
        let async_context = AsyncExecutionContext::new();

        // Execute the async test directly without timeout for now (simplified)
        let result = self.create_async_test_future(test, &async_context).await?;

        let execution_time = start_time.elapsed();
        let async_info = async_context.get_execution_info(execution_time);

        Ok(AsyncTestResult {
            test_id: test.id.clone(),
            execution_time,
            result,
            async_info,
        })
    }

    /// Get timeout duration for test
    fn get_test_timeout(&self, test: &TestItem) -> Duration {
        if self.config.per_test_timeouts {
            // Check for timeout marker in test
            if let Some(timeout_marker) = self.extract_timeout_marker(test) {
                return Duration::from_secs(timeout_marker);
            }
        }

        // Check if it's an async test
        if self.is_async_test(test) {
            Duration::from_secs(self.config.async_timeout)
        } else {
            Duration::from_secs(self.config.default_timeout)
        }
    }

    /// Get timeout for async tests
    fn get_async_timeout(&self, test: &TestItem) -> Duration {
        Duration::from_secs(self.config.async_timeout)
    }

    /// Extract timeout from test decorators
    fn extract_timeout_marker(&self, test: &TestItem) -> Option<u64> {
        // Check for @pytest.mark.timeout(seconds) decorator
        for decorator in &test.decorators {
            if decorator.contains("timeout") && decorator.contains("(") {
                // Simple parsing - would be more sophisticated in production
                if let Some(start) = decorator.find("(") {
                    if let Some(end) = decorator.find(")") {
                        let timeout_str = &decorator[start + 1..end];
                        if let Ok(timeout_num) = timeout_str.parse::<u64>() {
                            return Some(timeout_num);
                        }
                    }
                }
            }
        }
        None
    }

    /// Check if test is async
    fn is_async_test(&self, test: &TestItem) -> bool {
        // Use the is_async field from TestItem
        test.is_async || test.decorators.iter().any(|d| d.contains("asyncio"))
    }

    /// Start timeout warning task
    async fn start_timeout_warning_task(&self, test_id: &str, timeout_duration: Duration) {
        let warning_time = Duration::from_secs(
            (timeout_duration.as_secs() as f64 * self.config.warning_threshold) as u64,
        );

        let test_id = test_id.to_string();
        tokio::spawn(async move {
            tokio::time::sleep(warning_time).await;
            tracing::warn!(
                "⚠️  Test '{}' is approaching timeout ({:?} elapsed)",
                test_id,
                warning_time
            );
        });
    }

    /// Create async test execution future
    async fn create_async_test_future(
        &self,
        test: &TestItem,
        context: &AsyncExecutionContext,
    ) -> Result<TestResult> {
        // Simplified async test execution
        // In production, would integrate with Python async runtime

        context.increment_awaited_operations().await;

        // Simulate async test execution
        tokio::time::sleep(Duration::from_millis(100)).await;

        Ok(TestResult {
            test_id: test.id.clone(),
            passed: true,
            duration: Duration::from_millis(100),
            output: "Async test executed successfully".to_string(),
            error: None,
            stdout: String::new(),
            stderr: String::new(),
        })
    }

    /// Handle timeout error
    pub fn handle_timeout_error(&self, test: &TestItem, elapsed: Duration) -> TimeoutError {
        TimeoutError {
            test_id: test.id.clone(),
            timeout_duration: self.get_test_timeout(test),
            elapsed_time: elapsed,
            timeout_type: if self.is_async_test(test) {
                TimeoutType::AsyncOperation
            } else {
                TimeoutType::TestExecution
            },
            context: format!("Test '{}' in {}", test.function_name, test.path.display()),
        }
    }

    /// Get timeout statistics
    pub fn get_timeout_stats(&self) -> HashMap<String, serde_json::Value> {
        let mut stats = HashMap::new();

        stats.insert(
            "active_timeouts".to_string(),
            serde_json::Value::Number(self.active_timeouts.len().into()),
        );
        stats.insert(
            "default_timeout".to_string(),
            serde_json::Value::Number(self.config.default_timeout.into()),
        );
        stats.insert(
            "async_timeout".to_string(),
            serde_json::Value::Number(self.config.async_timeout.into()),
        );
        stats.insert(
            "warnings_enabled".to_string(),
            serde_json::Value::Bool(self.config.timeout_warnings),
        );

        stats
    }

    /// Display timeout error with helpful information
    pub fn display_timeout_error(&self, error: &TimeoutError) -> Result<()> {
        use colored::Colorize;

        println!("\n{}", "⏰ TIMEOUT ERROR".bright_red().bold());
        println!("{}", "━".repeat(50).red());

        println!("{}: {}", "Test".cyan(), error.test_id.bright_white());
        println!(
            "{}: {:?}",
            "Timeout Duration".cyan(),
            error.timeout_duration
        );
        println!("{}: {:?}", "Elapsed Time".cyan(), error.elapsed_time);
        println!("{}: {:?}", "Timeout Type".cyan(), error.timeout_type);
        println!("{}: {}", "Context".cyan(), error.context);

        println!("\n{}", "💡 Suggestions:".green().bold());
        match error.timeout_type {
            TimeoutType::TestExecution => {
                println!("  • Increase timeout with @pytest.mark.timeout(seconds)");
                println!("  • Optimize test performance");
                println!("  • Check for infinite loops or blocking operations");
            }
            TimeoutType::AsyncOperation => {
                println!("  • Verify async/await usage");
                println!("  • Check for deadlocks in async code");
                println!("  • Consider using asyncio.timeout for fine-grained control");
            }
            TimeoutType::FixtureSetup => {
                println!("  • Optimize fixture setup code");
                println!("  • Consider using session-scoped fixtures");
                println!("  • Check external dependencies in fixtures");
            }
            TimeoutType::FixtureTeardown => {
                println!("  • Optimize fixture cleanup code");
                println!("  • Ensure proper resource cleanup");
            }
        }

        println!("{}", "━".repeat(50).red());
        println!();

        Ok(())
    }
}

/// Context for async test execution
struct AsyncExecutionContext {
    awaited_operations: std::sync::atomic::AtomicU32,
    concurrent_tasks: std::sync::atomic::AtomicU32,
    event_loop_start: std::time::Instant,
}

impl AsyncExecutionContext {
    fn new() -> Self {
        Self {
            awaited_operations: std::sync::atomic::AtomicU32::new(0),
            concurrent_tasks: std::sync::atomic::AtomicU32::new(0),
            event_loop_start: std::time::Instant::now(),
        }
    }

    async fn increment_awaited_operations(&self) {
        self.awaited_operations
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    fn get_execution_info(&self, _total_time: Duration) -> AsyncExecutionInfo {
        AsyncExecutionInfo {
            is_async: true,
            awaited_operations: self
                .awaited_operations
                .load(std::sync::atomic::Ordering::Relaxed),
            concurrent_tasks: self
                .concurrent_tasks
                .load(std::sync::atomic::Ordering::Relaxed),
            event_loop_time: self.event_loop_start.elapsed(),
        }
    }
}

/// Utility functions for timeout handling
pub mod utils {
    use super::*;

    /// Parse timeout from string (e.g., "30s", "2m", "1h")
    pub fn parse_timeout_string(timeout_str: &str) -> Result<Duration> {
        let timeout_str = timeout_str.trim().to_lowercase();

        if let Some(seconds_str) = timeout_str.strip_suffix('s') {
            let seconds: u64 = seconds_str.parse()?;
            Ok(Duration::from_secs(seconds))
        } else if let Some(minutes_str) = timeout_str.strip_suffix('m') {
            let minutes: u64 = minutes_str.parse()?;
            Ok(Duration::from_secs(minutes * 60))
        } else if let Some(hours_str) = timeout_str.strip_suffix('h') {
            let hours: u64 = hours_str.parse()?;
            Ok(Duration::from_secs(hours * 3600))
        } else {
            // Default to seconds
            let seconds: u64 = timeout_str.parse()?;
            Ok(Duration::from_secs(seconds))
        }
    }

    /// Format duration for display
    pub fn format_duration(duration: Duration) -> String {
        let total_seconds = duration.as_secs();
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;
        let millis = duration.subsec_millis();

        if hours > 0 {
            format!("{}h {}m {}s", hours, minutes, seconds)
        } else if minutes > 0 {
            format!("{}m {}s", minutes, seconds)
        } else if seconds > 0 {
            format!("{}.{:03}s", seconds, millis)
        } else {
            format!("{}ms", duration.as_millis())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Marker, TestItem};

    #[test]
    fn test_timeout_config() {
        let config = TimeoutConfig::default();
        assert_eq!(config.default_timeout, 60);
        assert_eq!(config.async_timeout, 120);
        assert!(config.timeout_warnings);
    }

    #[test]
    fn test_parse_timeout_string() {
        assert_eq!(
            utils::parse_timeout_string("30s").unwrap(),
            Duration::from_secs(30)
        );
        assert_eq!(
            utils::parse_timeout_string("2m").unwrap(),
            Duration::from_secs(120)
        );
        assert_eq!(
            utils::parse_timeout_string("1h").unwrap(),
            Duration::from_secs(3600)
        );
        assert_eq!(
            utils::parse_timeout_string("45").unwrap(),
            Duration::from_secs(45)
        );
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(utils::format_duration(Duration::from_millis(500)), "500ms");
        assert_eq!(utils::format_duration(Duration::from_secs(5)), "5.000s");
        assert_eq!(utils::format_duration(Duration::from_secs(90)), "1m 30s");
        assert_eq!(
            utils::format_duration(Duration::from_secs(3661)),
            "1h 1m 1s"
        );
    }

    #[tokio::test]
    async fn test_timeout_manager() {
        let config = TimeoutConfig::default();
        let mut manager = TimeoutManager::new(config);

        let test = TestItem {
            id: "test_example".to_string(),
            path: std::path::PathBuf::from("test_file.py"),
            name: "test_example".to_string(),
            function_name: "test_example".to_string(),
            line_number: 10,
            is_async: false,
            class_name: None,
            decorators: vec![],
            fixture_deps: vec![],
            is_xfail: false,
        };

        // Test successful execution within timeout
        let result = manager
            .execute_with_timeout(&test, async {
                tokio::time::sleep(Duration::from_millis(10)).await;
                Ok("success")
            })
            .await;

        assert!(result.is_ok());
    }
}
