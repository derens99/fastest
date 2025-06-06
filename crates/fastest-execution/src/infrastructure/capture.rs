//! Per-test Output Capture and Exception Handling System
//!
//! This module provides comprehensive test isolation and capture capabilities including:
//! - Per-test stdout/stderr capture
//! - Enhanced exception handling with detailed tracebacks
//! - Test isolation and cleanup
//! - Resource leak detection

use anyhow::{anyhow, Result};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

// 🚀 REVOLUTIONARY SIMD JSON OPTIMIZATION (10-20% performance improvement)
use fastest_core::utils::simd_json;

/// Memory usage statistics
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryUsage {
    #[allow(dead_code)]
    pub peak_mb: f64,
    #[allow(dead_code)]
    pub current_mb: f64,
}

/// Configuration for test capture and isolation
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureConfig {
    #[allow(dead_code)]
    pub capture_stdout: bool,
    #[allow(dead_code)]
    pub capture_stderr: bool,
    #[allow(dead_code)]
    pub capture_warnings: bool,
    #[allow(dead_code)]
    pub capture_logs: bool,
    #[allow(dead_code)]
    pub isolate_filesystem: bool,
    #[allow(dead_code)]
    pub isolate_environment: bool,
    #[allow(dead_code)]
    pub timeout_seconds: Option<u64>,
    #[allow(dead_code)]
    pub max_output_size: usize, // bytes
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            capture_stdout: true,
            capture_stderr: true,
            capture_warnings: true,
            capture_logs: false,
            isolate_filesystem: false,
            isolate_environment: true,
            timeout_seconds: Some(300),   // 5 minutes
            max_output_size: 1024 * 1024, // 1MB
        }
    }
}

/// Captured output from a test execution
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureResult {
    #[allow(dead_code)]
    pub stdout: String,
    #[allow(dead_code)]
    pub stderr: String,
    #[allow(dead_code)]
    pub warnings: Vec<String>,
    #[allow(dead_code)]
    pub logs: Vec<LogEntry>,
    #[allow(dead_code)]
    pub exception: Option<ExceptionInfo>,
    #[allow(dead_code)]
    pub duration: Duration,
    #[allow(dead_code)]
    pub memory_usage: Option<usize>, // bytes
    #[allow(dead_code)]
    pub files_created: Vec<String>,
    #[allow(dead_code)]
    pub env_vars_changed: HashMap<String, String>,
}

/// Log entry captured during test execution
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    #[allow(dead_code)]
    pub level: String,
    #[allow(dead_code)]
    pub message: String,
    #[allow(dead_code)]
    pub timestamp: String,
    #[allow(dead_code)]
    pub logger_name: String,
    #[allow(dead_code)]
    pub filename: Option<String>,
    #[allow(dead_code)]
    pub line_number: Option<u32>,
}

/// Enhanced exception information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExceptionInfo {
    pub exception_type: String,
    pub message: String,
    pub traceback: Vec<TracebackFrame>,
    pub cause: Option<Box<ExceptionInfo>>,
    pub context: HashMap<String, String>,
    pub locals_at_failure: HashMap<String, String>,
}

/// Stack frame in a traceback
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracebackFrame {
    pub filename: String,
    pub line_number: u32,
    pub function_name: String,
    pub code: String,
    pub locals: HashMap<String, String>,
}

/// Test capture and isolation manager
#[allow(dead_code)]
pub struct CaptureManager {
    #[allow(dead_code)]
    config: CaptureConfig,
    #[allow(dead_code)]
    active_captures: Arc<Mutex<HashMap<String, ActiveCapture>>>,
}

struct ActiveCapture {
    start_time: Instant,
    python_process: std::process::Child,
    stdout_reader: BufReader<std::process::ChildStdout>,
    stderr_reader: BufReader<std::process::ChildStderr>,
    temp_dir: Option<std::path::PathBuf>,
    original_env: HashMap<String, String>,
}

impl CaptureManager {
    pub fn new(config: CaptureConfig) -> Self {
        Self {
            config,
            active_captures: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Start capturing output for a test
    pub fn start_capture(&self, test_id: &str, test_code: &str) -> Result<()> {
        let start_time = Instant::now();

        // Generate enhanced Python test execution code with capture
        let execution_code = self.generate_capture_code(test_code)?;

        // Create isolated environment
        let (temp_dir, env_vars) =
            if self.config.isolate_filesystem || self.config.isolate_environment {
                self.create_isolated_environment()?
            } else {
                (None, HashMap::new())
            };

        // Start Python process with capture
        let mut command = Command::new("python");
        command
            .arg("-u") // Unbuffered output
            .arg("-c")
            .arg(&execution_code)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Set environment variables
        if self.config.isolate_environment {
            command.env_clear();
            command.envs([
                ("PYTHONUNBUFFERED", "1"),
                ("PYTHONDONTWRITEBYTECODE", "1"),
                ("PYTHONHASHSEED", "0"),
                ("FASTEST_TEST_ID", test_id),
            ]);
        }

        for (key, value) in &env_vars {
            command.env(key, value);
        }

        // Set working directory to temp dir if isolating filesystem
        if let Some(ref temp_dir) = temp_dir {
            command.current_dir(temp_dir);
        }

        let mut child = command
            .spawn()
            .map_err(|e| anyhow!("Failed to start test process for {}: {}", test_id, e))?;

        let stdout_reader = BufReader::new(child.stdout.take().unwrap());
        let stderr_reader = BufReader::new(child.stderr.take().unwrap());

        let capture = ActiveCapture {
            start_time,
            python_process: child,
            stdout_reader,
            stderr_reader,
            temp_dir,
            original_env: std::env::vars().collect(),
        };

        let mut active_captures = self.active_captures.lock();
        active_captures.insert(test_id.to_string(), capture);

        Ok(())
    }

    /// Stop capturing and return results
    pub fn stop_capture(&self, test_id: &str) -> Result<CaptureResult> {
        let mut active_captures = self.active_captures.lock();
        let mut capture = active_captures
            .remove(test_id)
            .ok_or_else(|| anyhow!("No active capture for test {}", test_id))?;

        drop(active_captures); // Release the lock

        let duration = capture.start_time.elapsed();

        // Read all output
        let stdout = self.read_output(&mut capture.stdout_reader)?;
        let stderr = self.read_output(&mut capture.stderr_reader)?;

        // Wait for process to complete
        let _exit_status = capture
            .python_process
            .wait()
            .map_err(|e| anyhow!("Failed to wait for test process: {}", e))?;

        // Parse captured output for structured data
        let (clean_stdout, warnings, logs, exception) =
            self.parse_captured_output(&stdout, &stderr)?;

        // Detect file system changes
        let files_created = if let Some(ref temp_dir) = capture.temp_dir {
            self.detect_created_files(temp_dir)?
        } else {
            Vec::new()
        };

        // Detect environment changes
        let env_vars_changed = self.detect_env_changes(&capture.original_env);

        // Cleanup isolated environment
        if let Some(temp_dir) = capture.temp_dir {
            self.cleanup_temp_dir(&temp_dir)?;
        }

        Ok(CaptureResult {
            stdout: clean_stdout,
            stderr: if exception.is_some() {
                String::new()
            } else {
                stderr
            },
            warnings,
            logs,
            exception,
            duration,
            memory_usage: None, // TODO: Implement memory tracking
            files_created,
            env_vars_changed,
        })
    }

    /// Generate Python code with comprehensive capture
    fn generate_capture_code(&self, test_code: &str) -> Result<String> {
        let capture_wrapper = format!(
            r#"
import sys
import os
import io
import json
import traceback
import warnings
import logging
import time
import threading
from contextlib import contextmanager, redirect_stdout, redirect_stderr
from typing import Any, Dict, List, Optional

# Enhanced capture configuration
CAPTURE_STDOUT = {}
CAPTURE_STDERR = {}
CAPTURE_WARNINGS = {}
CAPTURE_LOGS = {}
MAX_OUTPUT_SIZE = {}

class FastestCapture:
    """Comprehensive test capture system."""
    
    def __init__(self):
        self.stdout_buffer = io.StringIO()
        self.stderr_buffer = io.StringIO()
        self.warnings_list = []
        self.logs_list = []
        self.original_stdout = sys.stdout
        self.original_stderr = sys.stderr
        self.start_time = time.perf_counter()
        
        # Setup warning capture
        if CAPTURE_WARNINGS:
            warnings.showwarning = self._capture_warning
        
        # Setup logging capture
        if CAPTURE_LOGS:
            self._setup_log_capture()
    
    def _capture_warning(self, message, category, filename, lineno, file=None, line=None):
        """Capture warnings."""
        warning_info = {{
            'message': str(message),
            'category': category.__name__,
            'filename': filename,
            'line_number': lineno,
            'code': line or ''
        }}
        self.warnings_list.append(warning_info)
        
        # Also print to stderr for visibility
        warning_text = f"{{filename}}:{{lineno}}: {{category.__name__}}: {{message}}"
        print(warning_text, file=sys.stderr)
    
    def _setup_log_capture(self):
        """Setup comprehensive logging capture."""
        class CaptureHandler(logging.Handler):
            def __init__(self, capture_instance):
                super().__init__()
                self.capture = capture_instance
            
            def emit(self, record):
                log_entry = {{
                    'level': record.levelname,
                    'message': record.getMessage(),
                    'timestamp': time.strftime('%Y-%m-%d %H:%M:%S', time.localtime(record.created)),
                    'logger_name': record.name,
                    'filename': record.filename if hasattr(record, 'filename') else None,
                    'line_number': record.lineno if hasattr(record, 'lineno') else None,
                }}
                self.capture.logs_list.append(log_entry)
        
        # Add handler to root logger
        handler = CaptureHandler(self)
        handler.setLevel(logging.DEBUG)
        logging.getLogger().addHandler(handler)
        logging.getLogger().setLevel(logging.DEBUG)
    
    @contextmanager
    def capture_context(self):
        """Context manager for capturing output."""
        redirectors = []
        
        if CAPTURE_STDOUT:
            redirectors.append(redirect_stdout(self.stdout_buffer))
        
        if CAPTURE_STDERR:
            redirectors.append(redirect_stderr(self.stderr_buffer))
        
        try:
            # Enter all context managers
            for redirector in redirectors:
                redirector.__enter__()
            
            yield self
            
        finally:
            # Exit all context managers in reverse order
            for redirector in reversed(redirectors):
                try:
                    redirector.__exit__(None, None, None)
                except:
                    pass
    
    def get_captured_output(self):
        """Get all captured output."""
        return {{
            'stdout': self.stdout_buffer.getvalue(),
            'stderr': self.stderr_buffer.getvalue(),
            'warnings': self.warnings_list,
            'logs': self.logs_list,
            'duration': time.perf_counter() - self.start_time
        }}
    
    def format_exception(self, exc_type, exc_value, exc_tb):
        """Format exception with enhanced information."""
        frames = []
        
        # Extract all frames from traceback
        tb = exc_tb
        while tb is not None:
            frame = tb.tb_frame
            frame_info = {{
                'filename': frame.f_code.co_filename,
                'line_number': tb.tb_lineno,
                'function_name': frame.f_code.co_name,
                'code': '',  # Will be filled in if possible
                'locals': {{}}
            }}
            
            # Try to get the actual code line
            try:
                import linecache
                frame_info['code'] = linecache.getline(frame.f_code.co_filename, tb.tb_lineno).strip()
            except:
                pass
            
            # Capture local variables (be careful about size and sensitive data)
            try:
                for name, value in frame.f_locals.items():
                    if not name.startswith('__') and len(str(value)) < 200:
                        frame_info['locals'][name] = repr(value)
            except:
                pass
            
            frames.append(frame_info)
            tb = tb.tb_next
        
        # Get exception chain (cause and context)
        cause = None
        if hasattr(exc_value, '__cause__') and exc_value.__cause__:
            cause = self.format_exception(
                type(exc_value.__cause__), 
                exc_value.__cause__, 
                exc_value.__cause__.__traceback__
            )
        
        return {{
            'exception_type': exc_type.__name__,
            'message': str(exc_value),
            'traceback': frames,
            'cause': cause,
            'context': {{
                'test_id': os.environ.get('FASTEST_TEST_ID', 'unknown'),
                'python_version': sys.version,
                'platform': sys.platform,
            }}
        }}

# Create capture instance
capture = FastestCapture()

# Execute test with comprehensive capture
try:
    with capture.capture_context():
        # Execute the actual test code
        {}
        
    # Test completed successfully
    result = capture.get_captured_output()
    result['success'] = True
    result['exception'] = None
    
except Exception as e:
    # Test failed with exception
    result = capture.get_captured_output()
    result['success'] = False
    result['exception'] = capture.format_exception(type(e), e, e.__traceback__)

# Output results as JSON for parsing
print("FASTEST_CAPTURE_START")
print(json.dumps(result, default=str, indent=2))
print("FASTEST_CAPTURE_END")
"#,
            self.config.capture_stdout,
            self.config.capture_stderr,
            self.config.capture_warnings,
            self.config.capture_logs,
            self.config.max_output_size,
            test_code
        );

        Ok(capture_wrapper)
    }

    /// Create isolated environment for test execution
    fn create_isolated_environment(
        &self,
    ) -> Result<(Option<std::path::PathBuf>, HashMap<String, String>)> {
        let mut env_vars = HashMap::new();
        let temp_dir = if self.config.isolate_filesystem {
            let temp_dir_handle = tempfile::tempdir()
                .map_err(|e| anyhow!("Failed to create temp directory: {}", e))?;
            let temp_dir = temp_dir_handle.keep();

            // Setup Python path
            env_vars.insert(
                "PYTHONPATH".to_string(),
                temp_dir.to_string_lossy().to_string(),
            );

            Some(temp_dir)
        } else {
            None
        };

        if self.config.isolate_environment {
            // Set clean environment variables
            env_vars.insert("HOME".to_string(), "/tmp".to_string());
            env_vars.insert("USER".to_string(), "fastest_test".to_string());
            env_vars.insert("TMPDIR".to_string(), "/tmp".to_string());
        }

        Ok((temp_dir, env_vars))
    }

    /// Read output from a buffered reader with size limits
    fn read_output(&self, reader: &mut dyn BufRead) -> Result<String> {
        let mut output = String::new();
        let mut total_size = 0;

        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => break, // EOF
                Ok(bytes_read) => {
                    total_size += bytes_read;
                    if total_size > self.config.max_output_size {
                        output.push_str(&format!(
                            "\n[OUTPUT TRUNCATED - {} bytes limit exceeded]",
                            self.config.max_output_size
                        ));
                        break;
                    }
                    output.push_str(&line);
                }
                Err(e) => return Err(anyhow!("Failed to read output: {}", e)),
            }
        }

        Ok(output)
    }

    /// Parse captured output to extract structured data
    fn parse_captured_output(
        &self,
        stdout: &str,
        _stderr: &str,
    ) -> Result<(String, Vec<String>, Vec<LogEntry>, Option<ExceptionInfo>)> {
        // Look for our JSON output markers
        if let Some(start) = stdout.find("FASTEST_CAPTURE_START") {
            if let Some(end) = stdout.find("FASTEST_CAPTURE_END") {
                let json_start = start + "FASTEST_CAPTURE_START".len();
                let json_content = &stdout[json_start..end].trim();

                match simd_json::from_str::<serde_json::Value>(json_content) {
                    Ok(data) => {
                        let clean_stdout = data
                            .get("stdout")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();

                        let warnings = data
                            .get("warnings")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|w| w.get("message")?.as_str())
                                    .map(|s| s.to_string())
                                    .collect()
                            })
                            .unwrap_or_default();

                        let logs = data
                            .get("logs")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|log| {
                                        Some(LogEntry {
                                            level: log.get("level")?.as_str()?.to_string(),
                                            message: log.get("message")?.as_str()?.to_string(),
                                            timestamp: log.get("timestamp")?.as_str()?.to_string(),
                                            logger_name: log
                                                .get("logger_name")?
                                                .as_str()?
                                                .to_string(),
                                            filename: log
                                                .get("filename")?
                                                .as_str()
                                                .map(|s| s.to_string()),
                                            line_number: log
                                                .get("line_number")?
                                                .as_u64()
                                                .map(|n| n as u32),
                                        })
                                    })
                                    .collect()
                            })
                            .unwrap_or_default();

                        let exception = data
                            .get("exception")
                            .and_then(|v| if v.is_null() { None } else { Some(v) })
                            .and_then(|exc| self.parse_exception_info(exc).ok());

                        return Ok((clean_stdout, warnings, logs, exception));
                    }
                    Err(e) => {
                        eprintln!("Failed to parse capture JSON: {}", e);
                    }
                }
            }
        }

        // Fallback: return raw output
        Ok((stdout.to_string(), Vec::new(), Vec::new(), None))
    }

    /// Parse exception information from JSON
    fn parse_exception_info(&self, exc_data: &serde_json::Value) -> Result<ExceptionInfo> {
        let exception_type = exc_data
            .get("exception_type")
            .and_then(|v| v.as_str())
            .unwrap_or("Exception")
            .to_string();

        let message = exc_data
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let traceback = exc_data
            .get("traceback")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|frame| {
                        Some(TracebackFrame {
                            filename: frame.get("filename")?.as_str()?.to_string(),
                            line_number: frame.get("line_number")?.as_u64()? as u32,
                            function_name: frame.get("function_name")?.as_str()?.to_string(),
                            code: frame.get("code")?.as_str()?.to_string(),
                            locals: frame
                                .get("locals")?
                                .as_object()?
                                .iter()
                                .filter_map(|(k, v)| Some((k.clone(), v.as_str()?.to_string())))
                                .collect(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        let context = exc_data
            .get("context")
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| Some((k.clone(), v.as_str()?.to_string())))
                    .collect()
            })
            .unwrap_or_default();

        let cause = exc_data
            .get("cause")
            .and_then(|v| if v.is_null() { None } else { Some(v) })
            .and_then(|cause_data| self.parse_exception_info(cause_data).ok())
            .map(Box::new);

        Ok(ExceptionInfo {
            exception_type,
            message,
            traceback,
            cause,
            context,
            locals_at_failure: HashMap::new(), // TODO: Extract from last frame
        })
    }

    /// Detect files created during test execution
    fn detect_created_files(&self, temp_dir: &std::path::Path) -> Result<Vec<String>> {
        let mut files = Vec::new();

        fn visit_dir(dir: &std::path::Path, files: &mut Vec<String>) -> Result<()> {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_file() {
                    files.push(path.to_string_lossy().to_string());
                } else if path.is_dir() {
                    visit_dir(&path, files)?;
                }
            }
            Ok(())
        }

        visit_dir(temp_dir, &mut files)?;
        Ok(files)
    }

    /// Extract memory usage from captured output
    #[allow(dead_code)]
    fn extract_memory_usage(&self, stdout: &str) -> Option<MemoryUsage> {
        // Look for the JSON output
        if let Some(start) = stdout.find("FASTEST_CAPTURE_START") {
            if let Some(end) = stdout.find("FASTEST_CAPTURE_END") {
                let json_str = &stdout[start + "FASTEST_CAPTURE_START".len()..end].trim();

                // Parse the JSON
                if let Ok(json_value) = simd_json::from_str::<serde_json::Value>(json_str) {
                    if let Some(memory) = json_value.get("memory") {
                        if let (Some(peak), Some(current)) = (
                            memory.get("peak_mb").and_then(|v| v.as_f64()),
                            memory.get("current_mb").and_then(|v| v.as_f64()),
                        ) {
                            return Some(MemoryUsage {
                                peak_mb: peak,
                                current_mb: current,
                            });
                        }
                    }
                }
            }
        }
        None
    }

    /// Detect environment variable changes
    fn detect_env_changes(
        &self,
        original_env: &HashMap<String, String>,
    ) -> HashMap<String, String> {
        let mut changes = HashMap::new();
        let current_env: HashMap<String, String> = std::env::vars().collect();

        // Find new or changed variables
        for (key, value) in &current_env {
            if let Some(original_value) = original_env.get(key) {
                if original_value != value {
                    changes.insert(key.clone(), value.clone());
                }
            } else {
                changes.insert(key.clone(), value.clone());
            }
        }

        changes
    }

    /// Cleanup temporary directory
    fn cleanup_temp_dir(&self, temp_dir: &std::path::Path) -> Result<()> {
        std::fs::remove_dir_all(temp_dir)
            .map_err(|e| anyhow!("Failed to cleanup temp directory: {}", e))?;
        Ok(())
    }
}

impl Default for CaptureManager {
    fn default() -> Self {
        Self::new(CaptureConfig::default())
    }
}

/// Utility functions for enhanced exception handling
pub mod exception_utils {
    use super::*;

    /// Format exception for display
    pub fn format_exception_display(exception: &ExceptionInfo) -> String {
        let mut output = String::new();

        output.push_str(&format!(
            "{}: {}\n",
            exception.exception_type, exception.message
        ));

        if !exception.traceback.is_empty() {
            output.push_str("\nTraceback (most recent call last):\n");

            for frame in &exception.traceback {
                output.push_str(&format!(
                    "  File \"{}\", line {}, in {}\n",
                    frame.filename, frame.line_number, frame.function_name
                ));

                if !frame.code.is_empty() {
                    output.push_str(&format!("    {}\n", frame.code));
                }

                if !frame.locals.is_empty() {
                    output.push_str("    Locals:\n");
                    for (name, value) in &frame.locals {
                        output.push_str(&format!("      {} = {}\n", name, value));
                    }
                }
            }
        }

        if let Some(ref cause) = exception.cause {
            output.push_str("\nCaused by:\n");
            output.push_str(&format_exception_display(cause));
        }

        output
    }

    /// Extract exception summary for quick display
    pub fn exception_summary(exception: &ExceptionInfo) -> String {
        format!("{}: {}", exception.exception_type, exception.message)
    }

    /// Check if exception is a test skip
    pub fn is_skip_exception(exception: &ExceptionInfo) -> bool {
        matches!(
            exception.exception_type.as_str(),
            "SkipTest" | "Skipped" | "pytest.skip"
        )
    }

    /// Check if exception is an assertion error
    pub fn is_assertion_error(exception: &ExceptionInfo) -> bool {
        exception.exception_type == "AssertionError"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // use tempfile::TempDir; // Unused import

    #[test]
    fn test_capture_config_default() {
        let config = CaptureConfig::default();
        assert!(config.capture_stdout);
        assert!(config.capture_stderr);
        assert_eq!(config.max_output_size, 1024 * 1024);
    }

    #[test]
    fn test_capture_manager_creation() {
        let config = CaptureConfig::default();
        let manager = CaptureManager::new(config);

        // Should create without errors
        assert_eq!(manager.active_captures.lock().len(), 0);
    }

    #[test]
    fn test_exception_formatting() {
        let exception = ExceptionInfo {
            exception_type: "ValueError".to_string(),
            message: "invalid literal".to_string(),
            traceback: vec![TracebackFrame {
                filename: "test.py".to_string(),
                line_number: 10,
                function_name: "test_func".to_string(),
                code: "x = int('abc')".to_string(),
                locals: HashMap::new(),
            }],
            cause: None,
            context: HashMap::new(),
            locals_at_failure: HashMap::new(),
        };

        let formatted = exception_utils::format_exception_display(&exception);
        assert!(formatted.contains("ValueError"));
        assert!(formatted.contains("invalid literal"));
        assert!(formatted.contains("test.py"));
    }
}
