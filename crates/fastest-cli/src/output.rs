//! Output formatting for test results.
//!
//! Supports multiple output formats: pretty (pytest-style), JSON, count summary,
//! and JUnit XML for CI integration.

use std::io::Write;
use std::path::Path;
use std::time::Duration;

use colored::Colorize;
use fastest_core::{TestOutcome, TestResult};

/// Supported output formats.
#[derive(Debug, Clone)]
pub enum OutputFormat {
    /// Human-readable, colorized pytest-style output.
    Pretty,
    /// Machine-readable JSON array.
    Json,
    /// One-line summary: "N passed, N failed, N skipped".
    Count,
    /// JUnit XML written to the given file path.
    JunitXml(String),
}

impl OutputFormat {
    /// Parse an output format from CLI string.
    ///
    /// Recognises "json", "pretty", "count". Anything else is treated as Pretty.
    pub fn from_str_with_junit(s: Option<&str>, junit_path: Option<String>) -> Self {
        if let Some(path) = junit_path {
            return OutputFormat::JunitXml(path);
        }
        match s.map(|s| s.to_lowercase()).as_deref() {
            Some("json") => OutputFormat::Json,
            Some("count") => OutputFormat::Count,
            _ => OutputFormat::Pretty,
        }
    }
}

/// Format test results according to the chosen output format.
pub fn format_results(results: &[TestResult], format: &OutputFormat, verbose: bool) -> String {
    match format {
        OutputFormat::Pretty => format_pretty(results, verbose),
        OutputFormat::Json => format_json(results),
        OutputFormat::Count => format_count(results),
        OutputFormat::JunitXml(_) => format_pretty(results, verbose),
    }
}

/// Pytest-style colorized output.
///
/// Each test is printed with a status indicator (PASSED/FAILED/SKIPPED/etc.)
/// followed by a summary line.
fn format_pretty(results: &[TestResult], verbose: bool) -> String {
    let mut out = String::new();

    for result in results {
        let status = match &result.outcome {
            TestOutcome::Passed => "PASSED".green().bold().to_string(),
            TestOutcome::Failed => "FAILED".red().bold().to_string(),
            TestOutcome::Skipped { .. } => "SKIPPED".yellow().bold().to_string(),
            TestOutcome::XFailed { .. } => "XFAIL".yellow().to_string(),
            TestOutcome::XPassed => "XPASS".yellow().bold().to_string(),
            TestOutcome::Error { .. } => "ERROR".red().bold().to_string(),
        };

        out.push_str(&format!("{} {}", status, result.test_id));

        if verbose {
            out.push_str(&format!(" ({:.3}s)", result.duration.as_secs_f64()));
        }

        out.push('\n');

        // Print failure details
        if verbose {
            if let Some(ref err) = result.error {
                out.push_str(&format!("    {}\n", err.red()));
            }
            if !result.stdout.is_empty() {
                out.push_str(&format!("    --- stdout ---\n    {}\n", result.stdout));
            }
            if !result.stderr.is_empty() {
                out.push_str(&format!("    --- stderr ---\n    {}\n", result.stderr));
            }
        } else {
            // Even in non-verbose mode, show error for failures
            match &result.outcome {
                TestOutcome::Failed | TestOutcome::Error { .. } => {
                    if let Some(ref err) = result.error {
                        out.push_str(&format!("    {}\n", err.red()));
                    }
                }
                _ => {}
            }
        }
    }

    out
}

/// JSON output: serialises the entire results array.
fn format_json(results: &[TestResult]) -> String {
    serde_json::to_string_pretty(results).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e))
}

/// Count summary: "N passed, N failed, N skipped"
fn format_count(results: &[TestResult]) -> String {
    let (passed, failed, skipped, errors) = count_outcomes(results);
    format!(
        "{} passed, {} failed, {} skipped, {} errors",
        passed, failed, skipped, errors
    )
}

/// Write JUnit XML to a file.
///
/// Produces a minimal JUnit XML report compatible with most CI systems.
pub fn write_junit_xml(results: &[TestResult], path: &str) -> anyhow::Result<()> {
    let (_passed, failed, skipped, errors) = count_outcomes(results);
    let total_time: f64 = results.iter().map(|r| r.duration.as_secs_f64()).sum();

    let mut file = std::fs::File::create(Path::new(path))?;

    writeln!(file, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>")?;
    writeln!(
        file,
        "<testsuites tests=\"{}\" failures=\"{}\" errors=\"{}\" skipped=\"{}\" time=\"{:.3}\">",
        results.len(),
        failed,
        errors,
        skipped,
        total_time
    )?;
    writeln!(
        file,
        "  <testsuite name=\"fastest\" tests=\"{}\" failures=\"{}\" errors=\"{}\" skipped=\"{}\" time=\"{:.3}\">",
        results.len(),
        failed,
        errors,
        skipped,
        total_time
    )?;

    for result in results {
        let (classname, name) = split_test_id(&result.test_id);
        writeln!(
            file,
            "    <testcase classname=\"{}\" name=\"{}\" time=\"{:.3}\">",
            xml_escape(&classname),
            xml_escape(&name),
            result.duration.as_secs_f64()
        )?;

        match &result.outcome {
            TestOutcome::Failed => {
                let message = result.error.as_deref().unwrap_or("Test failed");
                writeln!(
                    file,
                    "      <failure message=\"{}\">{}</failure>",
                    xml_escape(message),
                    xml_escape(&result.output)
                )?;
            }
            TestOutcome::Error { message } => {
                writeln!(
                    file,
                    "      <error message=\"{}\">{}</error>",
                    xml_escape(message),
                    xml_escape(&result.output)
                )?;
            }
            TestOutcome::Skipped { reason } => {
                let msg = reason.as_deref().unwrap_or("Skipped");
                writeln!(file, "      <skipped message=\"{}\" />", xml_escape(msg))?;
            }
            _ => {}
        }

        if !result.stdout.is_empty() {
            writeln!(
                file,
                "      <system-out>{}</system-out>",
                xml_escape(&result.stdout)
            )?;
        }
        if !result.stderr.is_empty() {
            writeln!(
                file,
                "      <system-err>{}</system-err>",
                xml_escape(&result.stderr)
            )?;
        }

        writeln!(file, "    </testcase>")?;
    }

    writeln!(file, "  </testsuite>")?;
    writeln!(file, "</testsuites>")?;

    Ok(())
}

/// Print a coloured summary line with timing.
///
/// Example: "4 passed, 1 failed in 2.34s"
pub fn print_summary(results: &[TestResult], duration: Duration) {
    let (passed, failed, skipped, errors) = count_outcomes(results);
    let total = results.len();

    let mut parts: Vec<String> = Vec::new();

    if passed > 0 {
        parts.push(format!("{} passed", passed).green().bold().to_string());
    }
    if failed > 0 {
        parts.push(format!("{} failed", failed).red().bold().to_string());
    }
    if skipped > 0 {
        parts.push(format!("{} skipped", skipped).yellow().to_string());
    }
    if errors > 0 {
        parts.push(format!("{} errors", errors).red().to_string());
    }

    let summary = if parts.is_empty() {
        "no tests ran".dimmed().to_string()
    } else {
        parts.join(", ")
    };

    let decoration = if failed > 0 || errors > 0 {
        "=".repeat(60).red().to_string()
    } else {
        "=".repeat(60).green().to_string()
    };

    eprintln!("{}", decoration);
    eprintln!(
        "{} {} in {:.2}s",
        summary,
        format!("({} total)", total).dimmed(),
        duration.as_secs_f64()
    );
    eprintln!("{}", decoration);
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Count outcomes by category.
fn count_outcomes(results: &[TestResult]) -> (usize, usize, usize, usize) {
    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut skipped = 0usize;
    let mut errors = 0usize;

    for r in results {
        match &r.outcome {
            TestOutcome::Passed | TestOutcome::XFailed { .. } => passed += 1,
            TestOutcome::Failed | TestOutcome::XPassed => failed += 1,
            TestOutcome::Skipped { .. } => skipped += 1,
            TestOutcome::Error { .. } => errors += 1,
        }
    }

    (passed, failed, skipped, errors)
}

/// Split a test ID like "tests/test_math.py::TestCalc::test_add" into
/// (classname, testname) for JUnit XML.
fn split_test_id(id: &str) -> (String, String) {
    let parts: Vec<&str> = id.rsplitn(2, "::").collect();
    if parts.len() == 2 {
        (parts[1].to_string(), parts[0].to_string())
    } else {
        (String::new(), id.to_string())
    }
}

/// Escape special XML characters.
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_result(id: &str, outcome: TestOutcome, ms: u64) -> TestResult {
        TestResult {
            test_id: id.to_string(),
            outcome,
            duration: Duration::from_millis(ms),
            output: String::new(),
            error: None,
            stdout: String::new(),
            stderr: String::new(),
        }
    }

    #[test]
    fn test_format_count() {
        let results = vec![
            make_result("test_a", TestOutcome::Passed, 10),
            make_result("test_b", TestOutcome::Failed, 20),
            make_result(
                "test_c",
                TestOutcome::Skipped {
                    reason: Some("no db".into()),
                },
                0,
            ),
        ];
        let out = format_count(&results);
        assert!(out.contains("1 passed"));
        assert!(out.contains("1 failed"));
        assert!(out.contains("1 skipped"));
    }

    #[test]
    fn test_format_json() {
        let results = vec![make_result("test_a", TestOutcome::Passed, 10)];
        let json = format_json(&results);
        assert!(json.contains("test_a"));
        assert!(json.contains("Passed"));
    }

    #[test]
    fn test_split_test_id() {
        let (cls, name) = split_test_id("tests/test_math.py::TestCalc::test_add");
        assert_eq!(cls, "tests/test_math.py::TestCalc");
        assert_eq!(name, "test_add");

        let (cls, name) = split_test_id("test_simple");
        assert_eq!(cls, "");
        assert_eq!(name, "test_simple");
    }

    #[test]
    fn test_xml_escape() {
        assert_eq!(xml_escape("a<b>c&d\"e'f"), "a&lt;b&gt;c&amp;d&quot;e&apos;f");
    }

    #[test]
    fn test_output_format_parsing() {
        assert!(matches!(
            OutputFormat::from_str_with_junit(Some("json"), None),
            OutputFormat::Json
        ));
        assert!(matches!(
            OutputFormat::from_str_with_junit(Some("count"), None),
            OutputFormat::Count
        ));
        assert!(matches!(
            OutputFormat::from_str_with_junit(Some("pretty"), None),
            OutputFormat::Pretty
        ));
        assert!(matches!(
            OutputFormat::from_str_with_junit(None, Some("report.xml".into())),
            OutputFormat::JunitXml(_)
        ));
    }
}
