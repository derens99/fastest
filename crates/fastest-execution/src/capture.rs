//! stdout/stderr capture utilities for test execution.
//!
//! Provides a lightweight container for captured output from test runs,
//! used by both the in-process and subprocess executors.

/// Captured output from a test execution.
#[derive(Debug, Clone, Default)]
pub struct CapturedOutput {
    pub stdout: String,
    pub stderr: String,
}

impl CapturedOutput {
    /// Create a new empty capture.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a capture from existing output strings.
    pub fn from_strings(stdout: String, stderr: String) -> Self {
        Self { stdout, stderr }
    }

    /// Returns true if both stdout and stderr are empty.
    pub fn is_empty(&self) -> bool {
        self.stdout.is_empty() && self.stderr.is_empty()
    }

    /// Merge another captured output into this one.
    pub fn merge(&mut self, other: &CapturedOutput) {
        self.stdout.push_str(&other.stdout);
        self.stderr.push_str(&other.stderr);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_captured_output_default_is_empty() {
        let cap = CapturedOutput::new();
        assert!(cap.is_empty());
        assert!(cap.stdout.is_empty());
        assert!(cap.stderr.is_empty());
    }

    #[test]
    fn test_captured_output_from_strings() {
        let cap = CapturedOutput::from_strings("hello\n".into(), "warn\n".into());
        assert!(!cap.is_empty());
        assert_eq!(cap.stdout, "hello\n");
        assert_eq!(cap.stderr, "warn\n");
    }

    #[test]
    fn test_captured_output_merge() {
        let mut a = CapturedOutput::from_strings("a".into(), "1".into());
        let b = CapturedOutput::from_strings("b".into(), "2".into());
        a.merge(&b);
        assert_eq!(a.stdout, "ab");
        assert_eq!(a.stderr, "12");
    }
}
