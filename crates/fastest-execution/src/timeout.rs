//! Test timeout handling.
//!
//! Wraps test execution with a configurable timeout duration. If a test exceeds
//! the timeout, it is reported as an error rather than hanging indefinitely.

use std::time::{Duration, Instant};

/// Default timeout for a single test (60 seconds).
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);

/// Configuration for test timeouts.
#[derive(Debug, Clone)]
pub struct TimeoutConfig {
    /// Maximum duration a single test may run before being killed.
    pub per_test: Duration,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            per_test: DEFAULT_TIMEOUT,
        }
    }
}

impl TimeoutConfig {
    /// Create a timeout config with a custom per-test duration.
    pub fn with_duration(per_test: Duration) -> Self {
        Self { per_test }
    }

    /// Check whether a test has exceeded its timeout based on an elapsed duration.
    pub fn is_expired(&self, elapsed: Duration) -> bool {
        elapsed > self.per_test
    }
}

/// A running timer that tracks how long a test has been executing.
pub struct TestTimer {
    start: Instant,
    config: TimeoutConfig,
}

impl TestTimer {
    /// Start a new timer with the given timeout configuration.
    pub fn start(config: TimeoutConfig) -> Self {
        Self {
            start: Instant::now(),
            config,
        }
    }

    /// Start a new timer with the default timeout.
    pub fn start_default() -> Self {
        Self::start(TimeoutConfig::default())
    }

    /// Returns the elapsed duration since the timer was started.
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    /// Returns true if the test has exceeded its allowed duration.
    pub fn is_expired(&self) -> bool {
        self.config.is_expired(self.start.elapsed())
    }

    /// Returns the configured per-test timeout.
    pub fn timeout_duration(&self) -> Duration {
        self.config.per_test
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeout_not_exceeded() {
        let config = TimeoutConfig::with_duration(Duration::from_secs(10));
        assert!(!config.is_expired(Duration::from_secs(5)));
        assert!(!config.is_expired(Duration::from_secs(10)));
        assert!(config.is_expired(Duration::from_secs(11)));
    }

    #[test]
    fn test_default_timeout() {
        let config = TimeoutConfig::default();
        assert_eq!(config.per_test, Duration::from_secs(60));
    }

    #[test]
    fn test_timer_starts_not_expired() {
        let timer = TestTimer::start(TimeoutConfig::with_duration(Duration::from_secs(60)));
        assert!(!timer.is_expired());
        assert!(timer.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn test_timer_default() {
        let timer = TestTimer::start_default();
        assert_eq!(timer.timeout_duration(), Duration::from_secs(60));
    }
}
