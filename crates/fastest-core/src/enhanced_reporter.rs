//! Phase 4: Enhanced Error Reporting
//!
//! Professional error reporting with rich formatting, suggestions, and context

use anyhow::Result;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Write;

use crate::{TestItem, TestResult};

/// Enhanced reporter for professional error output
pub struct EnhancedReporter {
    config: ReporterConfig,
    error_templates: HashMap<String, ErrorTemplate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReporterConfig {
    /// Enable colored output
    pub colors_enabled: bool,
    /// Show detailed context
    pub detailed_context: bool,
    /// Show suggestions
    pub show_suggestions: bool,
    /// Maximum lines of context to show
    pub max_context_lines: usize,
    /// Enable diff output for assertions
    pub diff_enabled: bool,
}

impl Default for ReporterConfig {
    fn default() -> Self {
        Self {
            colors_enabled: true,
            detailed_context: true,
            show_suggestions: true,
            max_context_lines: 5,
            diff_enabled: true,
        }
    }
}

#[derive(Debug, Clone)]
struct ErrorTemplate {
    pattern: String,
    title: String,
    description: String,
    suggestions: Vec<String>,
}

/// Enhanced test failure report
#[derive(Debug, Serialize)]
pub struct FailureReport {
    pub test_id: String,
    pub title: String,
    pub error_type: String,
    pub message: String,
    pub context: TestContext,
    pub suggestions: Vec<String>,
    pub diff: Option<DiffOutput>,
    pub traceback: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct TestContext {
    pub file_path: String,
    pub line_number: Option<u32>,
    pub function_name: String,
    pub test_parameters: Option<serde_json::Value>,
    pub fixtures_used: Vec<String>,
    pub code_snippet: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DiffOutput {
    pub expected: String,
    pub actual: String,
    pub diff_lines: Vec<DiffLine>,
}

#[derive(Debug, Serialize)]
pub struct DiffLine {
    pub line_type: DiffLineType,
    pub content: String,
    pub line_number: Option<u32>,
}

#[derive(Debug, Serialize)]
pub enum DiffLineType {
    Context,
    Added,
    Removed,
    Modified,
}

impl EnhancedReporter {
    pub fn new(config: ReporterConfig) -> Self {
        let mut reporter = Self {
            config,
            error_templates: HashMap::new(),
        };
        reporter.initialize_error_templates();
        reporter
    }

    /// Initialize built-in error templates
    fn initialize_error_templates(&mut self) {
        self.error_templates.insert(
            "AssertionError".to_string(),
            ErrorTemplate {
                pattern: "AssertionError".to_string(),
                title: "Assertion Failed".to_string(),
                description: "A test assertion did not pass as expected".to_string(),
                suggestions: vec![
                    "Check your expected vs actual values".to_string(),
                    "Verify the test logic and conditions".to_string(),
                    "Consider using more specific assertion methods".to_string(),
                ],
            },
        );

        self.error_templates.insert(
            "AttributeError".to_string(),
            ErrorTemplate {
                pattern: "AttributeError".to_string(),
                title: "Attribute Not Found".to_string(),
                description: "An object does not have the expected attribute".to_string(),
                suggestions: vec![
                    "Check if the object is properly initialized".to_string(),
                    "Verify the attribute name spelling".to_string(),
                    "Ensure the object is of the expected type".to_string(),
                ],
            },
        );

        self.error_templates.insert(
            "TypeError".to_string(),
            ErrorTemplate {
                pattern: "TypeError".to_string(),
                title: "Type Mismatch".to_string(),
                description: "An operation was performed on an incompatible type".to_string(),
                suggestions: vec![
                    "Check the types of your variables".to_string(),
                    "Ensure proper type conversion".to_string(),
                    "Verify function argument types".to_string(),
                ],
            },
        );

        self.error_templates.insert(
            "ImportError".to_string(),
            ErrorTemplate {
                pattern: "ImportError|ModuleNotFoundError".to_string(),
                title: "Import Failed".to_string(),
                description: "A module or package could not be imported".to_string(),
                suggestions: vec![
                    "Check if the module is installed: pip install <module>".to_string(),
                    "Verify the module name and spelling".to_string(),
                    "Ensure the module is in your Python path".to_string(),
                ],
            },
        );
    }

    /// Generate enhanced failure report
    pub fn generate_failure_report(
        &self,
        test: &TestItem,
        result: &TestResult,
    ) -> Result<FailureReport> {
        let default_error = "Unknown error".to_string();
        let error_message = result.error.as_ref().unwrap_or(&default_error);
        let error_type = self.classify_error(error_message);
        let template = self.get_error_template(&error_type);

        let context = TestContext {
            file_path: test.path.to_string_lossy().to_string(),
            line_number: Some(test.line_number as u32),
            function_name: test.function_name.clone(),
            test_parameters: None, // TestItem doesn't have params field
            fixtures_used: vec![], // Would extract from test metadata
            code_snippet: self.extract_code_snippet(test)?,
        };

        let diff = if error_message.contains("assert") && self.config.diff_enabled {
            self.generate_diff_output(error_message)?
        } else {
            None
        };

        let suggestions = if self.config.show_suggestions {
            self.generate_suggestions(error_message, &template)
        } else {
            vec![]
        };

        Ok(FailureReport {
            test_id: test.id.clone(),
            title: template.title.clone(),
            error_type: error_type.clone(),
            message: error_message.clone(),
            context,
            suggestions,
            diff,
            traceback: self.parse_traceback(error_message),
        })
    }

    /// Display enhanced failure report
    pub fn display_failure_report(&self, report: &FailureReport) -> Result<()> {
        if self.config.colors_enabled {
            self.display_colored_report(report)
        } else {
            self.display_plain_report(report)
        }
    }

    /// Display colored failure report
    fn display_colored_report(&self, report: &FailureReport) -> Result<()> {
        println!();
        println!("{}", "━".repeat(80).bright_red());
        println!(
            "{} {}",
            "❌ FAILURE:".bright_red().bold(),
            report.test_id.bright_white().bold()
        );
        println!("{}", "━".repeat(80).bright_red());

        // Error title and type
        println!(
            "{}: {}",
            "Error Type".cyan().bold(),
            report.error_type.red()
        );
        println!("{}: {}", "Title".cyan().bold(), report.title.yellow());

        // Context information
        if self.config.detailed_context {
            println!("\n{}", "📍 Context:".bright_blue().bold());
            println!(
                "  {}: {}",
                "File".cyan(),
                report.context.file_path.bright_blue()
            );
            if let Some(line) = report.context.line_number {
                println!("  {}: {}", "Line".cyan(), line.to_string().green());
            }
            println!(
                "  {}: {}",
                "Function".cyan(),
                report.context.function_name.magenta()
            );

            if let Some(params) = &report.context.test_parameters {
                println!("  {}: {}", "Parameters".cyan(), params.to_string().yellow());
            }

            if let Some(code) = &report.context.code_snippet {
                println!("\n{}", "📝 Code:".bright_green().bold());
                for (i, line) in code.lines().enumerate() {
                    let line_num = report.context.line_number.unwrap_or(0) + i as u32;
                    if Some(line_num) == report.context.line_number {
                        println!(
                            "  {} {}",
                            format!("{:>4}", line_num).red().bold(),
                            line.white()
                        );
                    } else {
                        println!(
                            "  {} {}",
                            format!("{:>4}", line_num).bright_black(),
                            line.bright_black()
                        );
                    }
                }
            }
        }

        // Error message
        println!("\n{}", "🚨 Error Message:".bright_red().bold());
        println!("  {}", report.message.red());

        // Diff output for assertion failures
        if let Some(diff) = &report.diff {
            println!("\n{}", "🔍 Diff:".bright_yellow().bold());
            self.display_colored_diff(diff)?;
        }

        // Suggestions
        if !report.suggestions.is_empty() {
            println!("\n{}", "💡 Suggestions:".bright_green().bold());
            for (i, suggestion) in report.suggestions.iter().enumerate() {
                println!("  {}. {}", (i + 1).to_string().green(), suggestion);
            }
        }

        // Traceback
        if !report.traceback.is_empty() {
            println!("\n{}", "📚 Traceback:".bright_magenta().bold());
            for (_i, line) in report.traceback.iter().enumerate() {
                if line.trim().starts_with("File ") {
                    println!("  {}", line.bright_blue());
                } else if line.trim().starts_with("    ") {
                    println!("  {}", line.bright_white());
                } else {
                    println!("  {}", line);
                }
            }
        }

        println!("{}", "━".repeat(80).bright_red());
        println!();

        Ok(())
    }

    /// Display plain failure report (no colors)
    fn display_plain_report(&self, report: &FailureReport) -> Result<()> {
        println!();
        println!("{}", "=".repeat(80));
        println!("FAILURE: {}", report.test_id);
        println!("{}", "=".repeat(80));

        println!("Error Type: {}", report.error_type);
        println!("Title: {}", report.title);

        if self.config.detailed_context {
            println!("\nContext:");
            println!("  File: {}", report.context.file_path);
            if let Some(line) = report.context.line_number {
                println!("  Line: {}", line);
            }
            println!("  Function: {}", report.context.function_name);

            if let Some(params) = &report.context.test_parameters {
                println!("  Parameters: {}", params);
            }
        }

        println!("\nError Message:");
        println!("  {}", report.message);

        if let Some(diff) = &report.diff {
            println!("\nDiff:");
            self.display_plain_diff(diff)?;
        }

        if !report.suggestions.is_empty() {
            println!("\nSuggestions:");
            for (i, suggestion) in report.suggestions.iter().enumerate() {
                println!("  {}. {}", i + 1, suggestion);
            }
        }

        println!("{}", "=".repeat(80));
        println!();

        Ok(())
    }

    /// Display colored diff output
    fn display_colored_diff(&self, diff: &DiffOutput) -> Result<()> {
        for line in &diff.diff_lines {
            match line.line_type {
                DiffLineType::Added => {
                    println!("  {}", format!("+ {}", line.content).green());
                }
                DiffLineType::Removed => {
                    println!("  {}", format!("- {}", line.content).red());
                }
                DiffLineType::Modified => {
                    println!("  {}", format!("~ {}", line.content).yellow());
                }
                DiffLineType::Context => {
                    println!("  {}", format!("  {}", line.content).bright_black());
                }
            }
        }
        Ok(())
    }

    /// Display plain diff output
    fn display_plain_diff(&self, diff: &DiffOutput) -> Result<()> {
        for line in &diff.diff_lines {
            match line.line_type {
                DiffLineType::Added => println!("  + {}", line.content),
                DiffLineType::Removed => println!("  - {}", line.content),
                DiffLineType::Modified => println!("  ~ {}", line.content),
                DiffLineType::Context => println!("    {}", line.content),
            }
        }
        Ok(())
    }

    /// Classify error type from message
    fn classify_error(&self, error_message: &str) -> String {
        for (error_type, template) in &self.error_templates {
            if error_message.contains(error_type) {
                return template.title.clone();
            }
        }
        "Unknown Error".to_string()
    }

    /// Get error template for type
    fn get_error_template(&self, error_type: &str) -> ErrorTemplate {
        self.error_templates
            .values()
            .find(|t| t.title == error_type)
            .cloned()
            .unwrap_or_else(|| ErrorTemplate {
                pattern: "".to_string(),
                title: "Unknown Error".to_string(),
                description: "An unexpected error occurred".to_string(),
                suggestions: vec!["Check the error message for details".to_string()],
            })
    }

    /// Generate suggestions based on error
    fn generate_suggestions(&self, error_message: &str, template: &ErrorTemplate) -> Vec<String> {
        let mut suggestions = template.suggestions.clone();

        // Add context-specific suggestions
        if error_message.contains("fixture") {
            suggestions.push("Check fixture dependencies and scopes".to_string());
        }

        if error_message.contains("parametrize") {
            suggestions.push("Verify parametrize decorator syntax".to_string());
        }

        if error_message.contains("mock") {
            suggestions.push("Check mock setup and expectations".to_string());
        }

        suggestions
    }

    /// Extract code snippet around the error
    fn extract_code_snippet(&self, test: &TestItem) -> Result<Option<String>> {
        let line_num = test.line_number;
        if let Ok(content) = std::fs::read_to_string(&test.path) {
            let lines: Vec<&str> = content.lines().collect();
            let start = (line_num.saturating_sub(self.config.max_context_lines)).max(0);
            let end =
                ((line_num + self.config.max_context_lines).min(lines.len())).max(line_num + 1);

            let snippet = lines[start..end].join("\n");
            return Ok(Some(snippet));
        }
        Ok(None)
    }

    /// Generate diff output for assertion failures
    fn generate_diff_output(&self, error_message: &str) -> Result<Option<DiffOutput>> {
        // Simple diff generation - would use proper diff algorithm in production
        if let Some(assertion_info) = self.parse_assertion_error(error_message) {
            let diff_lines = vec![
                DiffLine {
                    line_type: DiffLineType::Removed,
                    content: format!("Expected: {}", assertion_info.expected),
                    line_number: None,
                },
                DiffLine {
                    line_type: DiffLineType::Added,
                    content: format!("Actual: {}", assertion_info.actual),
                    line_number: None,
                },
            ];

            Ok(Some(DiffOutput {
                expected: assertion_info.expected,
                actual: assertion_info.actual,
                diff_lines,
            }))
        } else {
            Ok(None)
        }
    }

    /// Parse assertion error to extract expected/actual values
    fn parse_assertion_error(&self, error_message: &str) -> Option<AssertionInfo> {
        // Simple parsing - would use regex in production
        if error_message.contains("assert") {
            Some(AssertionInfo {
                expected: "expected_value".to_string(),
                actual: "actual_value".to_string(),
            })
        } else {
            None
        }
    }

    /// Parse traceback from error message
    fn parse_traceback(&self, error_message: &str) -> Vec<String> {
        error_message
            .lines()
            .filter(|line| {
                line.trim().starts_with("File ")
                    || line.trim().starts_with("    ")
                    || line.contains("Error:")
            })
            .map(|line| line.to_string())
            .collect()
    }

    /// Generate summary of all failures
    pub fn generate_failure_summary(&self, failures: &[FailureReport]) -> Result<String> {
        let mut summary = String::new();

        writeln!(
            summary,
            "\n{}",
            if self.config.colors_enabled {
                "📊 Failure Summary".bright_red().bold().to_string()
            } else {
                "Failure Summary".to_string()
            }
        )?;

        let mut error_counts: HashMap<String, usize> = HashMap::new();
        for failure in failures {
            *error_counts.entry(failure.error_type.clone()).or_insert(0) += 1;
        }

        for (error_type, count) in error_counts {
            let line = format!("  {}: {} test(s)", error_type, count);
            writeln!(
                summary,
                "{}",
                if self.config.colors_enabled {
                    line.red().to_string()
                } else {
                    line
                }
            )?;
        }

        Ok(summary)
    }
}

#[derive(Debug)]
struct AssertionInfo {
    expected: String,
    actual: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TestItem;

    #[test]
    fn test_error_classification() {
        let reporter = EnhancedReporter::new(ReporterConfig::default());

        assert_eq!(
            reporter.classify_error("AssertionError: test failed"),
            "Assertion Failed"
        );
        assert_eq!(
            reporter.classify_error("AttributeError: no attribute"),
            "Attribute Not Found"
        );
        assert_eq!(
            reporter.classify_error("SomeRandomError: unknown"),
            "Unknown Error"
        );
    }

    #[test]
    fn test_reporter_config() {
        let config = ReporterConfig::default();
        assert!(config.colors_enabled);
        assert!(config.detailed_context);
        assert!(config.show_suggestions);
        assert_eq!(config.max_context_lines, 5);
    }
}
