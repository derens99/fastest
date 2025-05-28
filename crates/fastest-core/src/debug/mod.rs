//! Phase 4: Debugging Support
//!
//! Professional debugging integration with pdb, IDE support, and enhanced error reporting

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::{TestItem, TestResult};

/// Debug manager for test debugging and IDE integration
pub struct DebugManager {
    config: DebugConfig,
    active_sessions: HashMap<String, DebugSession>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugConfig {
    /// Enable PDB on test failures
    pub pdb_enabled: bool,
    /// Custom debugger class (e.g., "ipdb", "pudb")
    pub pdb_class: String,
    /// Debug on first failure only
    pub pdb_on_first_failure: bool,
    /// Enable breakpoint support
    pub breakpoints_enabled: bool,
    /// IDE integration mode
    pub ide_mode: bool,
    /// Enhanced error reporting
    pub enhanced_errors: bool,
}

impl Default for DebugConfig {
    fn default() -> Self {
        Self {
            pdb_enabled: false,
            pdb_class: "pdb".to_string(),
            pdb_on_first_failure: false,
            breakpoints_enabled: true,
            ide_mode: false,
            enhanced_errors: true,
        }
    }
}

#[derive(Debug)]
struct DebugSession {
    test_id: String,
    debugger_process: Option<std::process::Child>,
    breakpoints: Vec<Breakpoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Breakpoint {
    pub file_path: PathBuf,
    pub line: u32,
    pub condition: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EnhancedError {
    pub test_id: String,
    pub error_type: String,
    pub message: String,
    pub traceback: Vec<TraceFrame>,
    pub context: ErrorContext,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TraceFrame {
    pub file_path: String,
    pub line_number: u32,
    pub function_name: String,
    pub code_line: String,
    pub locals: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorContext {
    pub test_file: String,
    pub test_function: String,
    pub fixtures_used: Vec<String>,
    pub parameters: Option<serde_json::Value>,
}

impl DebugManager {
    pub fn new(config: DebugConfig) -> Self {
        Self {
            config,
            active_sessions: HashMap::new(),
        }
    }

    /// Handle test failure with debugging support
    pub async fn handle_test_failure(&mut self, test: &TestItem, result: &TestResult) -> Result<()> {
        if self.config.pdb_enabled && self.should_debug_test(test, result) {
            self.launch_debugger(test).await?;
        }

        if self.config.enhanced_errors {
            self.generate_enhanced_error(test, result).await?;
        }

        Ok(())
    }

    /// Launch PDB debugger for failed test
    async fn launch_debugger(&mut self, test: &TestItem) -> Result<()> {
        tracing::info!("🐛 Launching debugger for test: {}", test.id);

        // Prepare Python debug command
        let debug_script = self.create_debug_script(test)?;
        
        let mut cmd = Command::new("python")
            .args(["-c", &debug_script])
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()?;

        // Store debug session
        let session = DebugSession {
            test_id: test.id.clone(),
            debugger_process: Some(cmd),
            breakpoints: self.get_breakpoints_for_test(test),
        };

        self.active_sessions.insert(test.id.clone(), session);

        tracing::info!("🐛 Debugger launched. Use 'c' to continue, 'q' to quit.");
        Ok(())
    }

    /// Create Python debug script for test
    fn create_debug_script(&self, test: &TestItem) -> Result<String> {
        let module_path = test.path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("test_module");
        let function_name = &test.function_name;
        
        let script = format!(
            r#"
import sys
import {module_path}
import traceback

# Configure debugger
debugger_module = "{debugger_class}"
if debugger_module != "pdb":
    try:
        exec(f"import {{debugger_module}} as debugger")
    except ImportError:
        print(f"Warning: {{debugger_module}} not available, using pdb")
        import pdb as debugger
else:
    import pdb as debugger

# Set up test environment
def debug_test():
    try:
        # Get test function
        test_func = getattr({module_path}, "{function_name}")
        
        # Set trace before test execution
        debugger.set_trace()
        
        # Execute test function
        test_func()
        
    except Exception as e:
        print(f"Test failed with error: {{e}}")
        print("\\nTraceback:")
        traceback.print_exc()
        
        # Enter post-mortem debugging
        debugger.post_mortem()

if __name__ == "__main__":
    debug_test()
"#,
            module_path = module_path,
            debugger_class = self.config.pdb_class,
            function_name = function_name
        );

        Ok(script)
    }

    /// Check if test should be debugged
    fn should_debug_test(&self, _test: &TestItem, result: &TestResult) -> bool {
        // Only debug on failures or errors
        !result.passed && 
        // If first failure only, check if this is the first failure
        (!self.config.pdb_on_first_failure || self.active_sessions.is_empty())
    }

    /// Get breakpoints for specific test
    fn get_breakpoints_for_test(&self, test: &TestItem) -> Vec<Breakpoint> {
        // For now, return empty - would integrate with IDE/config
        Vec::new()
    }

    /// Generate enhanced error information
    async fn generate_enhanced_error(&self, test: &TestItem, result: &TestResult) -> Result<()> {
        if let Some(error) = &result.error {
            let enhanced = EnhancedError {
                test_id: test.id.clone(),
                error_type: self.classify_error(error),
                message: error.clone(),
                traceback: self.parse_traceback(error).await?,
                context: self.build_error_context(test),
                suggestions: self.generate_suggestions(error),
            };

            self.display_enhanced_error(&enhanced)?;
        }

        Ok(())
    }

    /// Classify error type for better reporting
    fn classify_error(&self, error: &str) -> String {
        if error.contains("AssertionError") {
            "Assertion Failure".to_string()
        } else if error.contains("AttributeError") {
            "Attribute Error".to_string()
        } else if error.contains("TypeError") {
            "Type Error".to_string()
        } else if error.contains("ValueError") {
            "Value Error".to_string()
        } else if error.contains("KeyError") {
            "Key Error".to_string()
        } else if error.contains("IndexError") {
            "Index Error".to_string()
        } else if error.contains("ImportError") || error.contains("ModuleNotFoundError") {
            "Import Error".to_string()
        } else {
            "Unknown Error".to_string()
        }
    }

    /// Parse Python traceback into structured format
    async fn parse_traceback(&self, error: &str) -> Result<Vec<TraceFrame>> {
        let mut frames = Vec::new();
        
        // Simple traceback parsing - in production would use Python's traceback module
        for line in error.lines() {
            if line.trim().starts_with("File \"") {
                if let Some(frame) = self.parse_trace_line(line) {
                    frames.push(frame);
                }
            }
        }

        Ok(frames)
    }

    /// Parse single traceback line
    fn parse_trace_line(&self, line: &str) -> Option<TraceFrame> {
        // Parse format: File "/path/file.py", line 123, in function_name
        let parts: Vec<&str> = line.split(", ").collect();
        if parts.len() >= 3 {
            let file_path = parts[0].trim_start_matches("  File \"").trim_end_matches("\"");
            let line_number = parts[1].trim_start_matches("line ").parse().unwrap_or(0);
            let function_name = parts[2].trim_start_matches("in ");

            Some(TraceFrame {
                file_path: file_path.to_string(),
                line_number,
                function_name: function_name.to_string(),
                code_line: String::new(), // Would read from file
                locals: HashMap::new(),   // Would extract from debug info
            })
        } else {
            None
        }
    }

    /// Build error context for test
    fn build_error_context(&self, test: &TestItem) -> ErrorContext {
        ErrorContext {
            test_file: test.path.to_string_lossy().to_string(),
            test_function: test.function_name.clone(),
            fixtures_used: Vec::new(), // Would extract from test metadata
            parameters: None, // TestItem doesn't have params field
        }
    }

    /// Generate helpful suggestions for common errors
    fn generate_suggestions(&self, error: &str) -> Vec<String> {
        let mut suggestions = Vec::new();

        if error.contains("AssertionError") {
            suggestions.push("Check your assertion logic and expected values".to_string());
            suggestions.push("Use pytest's assert introspection for better error messages".to_string());
        }

        if error.contains("AttributeError") {
            suggestions.push("Check if the object has the expected attribute".to_string());
            suggestions.push("Verify object initialization and type".to_string());
        }

        if error.contains("ImportError") {
            suggestions.push("Check if the module is installed: pip install <module>".to_string());
            suggestions.push("Verify the module path and spelling".to_string());
        }

        if error.contains("fixture") {
            suggestions.push("Check fixture scope and availability".to_string());
            suggestions.push("Verify fixture dependencies and setup".to_string());
        }

        if suggestions.is_empty() {
            suggestions.push("Check the test logic and expected behavior".to_string());
        }

        suggestions
    }

    /// Display enhanced error with formatting
    fn display_enhanced_error(&self, error: &EnhancedError) -> Result<()> {
        use colored::Colorize;

        println!("\n{}", "🚨 Enhanced Error Report".bright_red().bold());
        println!("{}: {}", "Test".cyan(), error.test_id);
        println!("{}: {}", "Error Type".cyan(), error.error_type.red());
        println!("{}: {}", "Message".cyan(), error.message);

        if !error.traceback.is_empty() {
            println!("\n{}", "📍 Traceback:".yellow().bold());
            for (i, frame) in error.traceback.iter().enumerate() {
                println!("  {}. {} (line {}) in {}", 
                    i + 1, 
                    frame.file_path.bright_blue(),
                    frame.line_number.to_string().green(),
                    frame.function_name.magenta()
                );
            }
        }

        if !error.suggestions.is_empty() {
            println!("\n{}", "💡 Suggestions:".green().bold());
            for suggestion in &error.suggestions {
                println!("  • {}", suggestion);
            }
        }

        println!(); // Empty line for spacing
        Ok(())
    }

    /// Set breakpoint for debugging
    pub fn set_breakpoint(&mut self, breakpoint: Breakpoint) -> Result<()> {
        tracing::info!("🔍 Setting breakpoint at {}:{}", 
                      breakpoint.file_path.display(), 
                      breakpoint.line);

        // In production, would integrate with debugger API
        Ok(())
    }

    /// Remove breakpoint
    pub fn remove_breakpoint(&mut self, file_path: &PathBuf, line: u32) -> Result<()> {
        tracing::info!("🔍 Removing breakpoint at {}:{}", file_path.display(), line);
        Ok(())
    }

    /// Get debug statistics
    pub fn get_debug_stats(&self) -> HashMap<String, serde_json::Value> {
        let mut stats = HashMap::new();
        
        stats.insert("active_sessions".to_string(), 
                    serde_json::Value::Number(self.active_sessions.len().into()));
        stats.insert("pdb_enabled".to_string(), 
                    serde_json::Value::Bool(self.config.pdb_enabled));
        stats.insert("enhanced_errors".to_string(), 
                    serde_json::Value::Bool(self.config.enhanced_errors));
        stats.insert("debugger_class".to_string(), 
                    serde_json::Value::String(self.config.pdb_class.clone()));
        
        stats
    }

    /// Cleanup debug sessions
    pub async fn cleanup(&mut self) -> Result<()> {
        for (test_id, mut session) in self.active_sessions.drain() {
            if let Some(mut process) = session.debugger_process.take() {
                let _ = process.kill();
                let _ = process.wait();
                tracing::debug!("Cleaned up debug session for test: {}", test_id);
            }
        }
        Ok(())
    }
}

/// Debug command line argument parsing
pub fn parse_debug_args(args: &[String]) -> DebugConfig {
    let mut config = DebugConfig::default();
    
    for arg in args {
        match arg.as_str() {
            "--pdb" => config.pdb_enabled = true,
            "--pdb-on-failure" => {
                config.pdb_enabled = true;
                config.pdb_on_first_failure = true;
            }
            arg if arg.starts_with("--pdbcls=") => {
                config.pdb_enabled = true;
                config.pdb_class = arg.strip_prefix("--pdbcls=").unwrap().to_string();
            }
            "--enhanced-errors" => config.enhanced_errors = true,
            "--ide-mode" => config.ide_mode = true,
            _ => {}
        }
    }
    
    config
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_config_default() {
        let config = DebugConfig::default();
        assert!(!config.pdb_enabled);
        assert_eq!(config.pdb_class, "pdb");
        assert!(config.enhanced_errors);
    }

    #[test]
    fn test_parse_debug_args() {
        let args = vec!["--pdb".to_string(), "--pdbcls=ipdb".to_string()];
        let config = parse_debug_args(&args);
        
        assert!(config.pdb_enabled);
        assert_eq!(config.pdb_class, "ipdb");
    }

    #[test]
    fn test_error_classification() {
        let manager = DebugManager::new(DebugConfig::default());
        
        assert_eq!(manager.classify_error("AssertionError: test failed"), "Assertion Failure");
        assert_eq!(manager.classify_error("AttributeError: 'str' object has no attribute 'foo'"), "Attribute Error");
        assert_eq!(manager.classify_error("SomeRandomError: unknown"), "Unknown Error");
    }
}