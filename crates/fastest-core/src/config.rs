use std::time::Duration;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct TestConfig {
    /// Maximum time a test can run before being terminated
    pub timeout: Duration,
    
    /// Number of parallel test workers (0 = number of CPU cores)
    pub parallel_workers: usize,
    
    /// Whether to capture stdout/stderr
    pub capture_output: bool,
    
    /// Test name pattern to filter tests
    pub filter_pattern: Option<String>,
    
    /// Whether to stop on first failure
    pub fail_fast: bool,
    
    /// Verbose output
    pub verbose: bool,
    
    /// Python executable path
    pub python_executable: String,
    
    /// Additional Python path entries
    pub python_path: Vec<PathBuf>,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(300), // 5 minutes default
            parallel_workers: 0, // Auto-detect
            capture_output: true,
            filter_pattern: None,
            fail_fast: false,
            verbose: false,
            python_executable: "python".to_string(),
            python_path: Vec::new(),
        }
    }
}

impl TestConfig {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
    
    pub fn with_parallel_workers(mut self, workers: usize) -> Self {
        self.parallel_workers = workers;
        self
    }
    
    pub fn with_filter(mut self, pattern: &str) -> Self {
        self.filter_pattern = Some(pattern.to_string());
        self
    }
    
    pub fn with_fail_fast(mut self, fail_fast: bool) -> Self {
        self.fail_fast = fail_fast;
        self
    }
    
    pub fn with_python_executable(mut self, executable: &str) -> Self {
        self.python_executable = executable.to_string();
        self
    }
    
    pub fn should_run_test(&self, test_name: &str) -> bool {
        if let Some(pattern) = &self.filter_pattern {
            test_name.contains(pattern)
        } else {
            true
        }
    }
} 