//! Output formatting for test results.
//!
//! Supports multiple output formats: pretty (pytest-style), JSON, count summary,
//! and JUnit XML for CI integration.

use std::fmt::Write as FmtWrite;
use std::io::Write;
use std::path::Path;
use std::time::Duration;

use colored::Colorize;
use fastest_core::{TestOutcome, TestResult};

/// Default separator width when terminal size cannot be detected.
const DEFAULT_WIDTH: usize = 80;

/// Get the current terminal width, falling back to DEFAULT_WIDTH.
fn term_width() -> usize {
    terminal_size::terminal_size()
        .map(|(w, _)| w.0 as usize)
        .unwrap_or(DEFAULT_WIDTH)
}

/// Supported output formats.
#[derive(Debug, Clone)]
pub enum OutputFormat {
    /// Human-readable, colorized pytest-style output.
    Pretty,
    /// Machine-readable JSON array.
    Json,
    /// One-line summary: "N passed, N failed, N skipped".
    Count,
    /// JUnit XML written to the given file path (legacy — now a side-channel).
    #[allow(dead_code)]
    JunitXml(String),
}

impl OutputFormat {
    /// Parse an output format from CLI string.
    ///
    /// Recognises "json", "pretty", "count". Anything else is treated as Pretty.
    /// JUnit XML is now a side-channel — use `parse_display_format` for the display
    /// format and write JUnit XML separately via `write_junit_xml`.
    #[allow(dead_code)]
    pub fn from_str_with_junit(s: Option<&str>, junit_path: Option<String>) -> Self {
        if let Some(path) = junit_path {
            return OutputFormat::JunitXml(path);
        }
        Self::parse_display_format(s)
    }

    /// Parse only the display format (Pretty, Json, Count) without JUnit XML.
    pub fn parse_display_format(s: Option<&str>) -> Self {
        match s.map(|s| s.to_lowercase()).as_deref() {
            Some("json") => OutputFormat::Json,
            Some("count") => OutputFormat::Count,
            _ => OutputFormat::Pretty,
        }
    }
}

/// Format test results according to the chosen output format.
pub fn format_results(
    results: &[TestResult],
    format: &OutputFormat,
    verbose: bool,
    tb: &str,
    quiet: bool,
) -> String {
    match format {
        OutputFormat::Pretty => format_pretty(results, verbose, tb, quiet),
        OutputFormat::Json => format_json(results),
        OutputFormat::Count => format_count(results),
        OutputFormat::JunitXml(_) => format_pretty(results, verbose, tb, quiet),
    }
}

/// Pytest-style colorized output.
///
/// Each test is printed with a status indicator (PASSED/FAILED/SKIPPED/etc.)
/// followed by a summary line.
fn format_pretty(results: &[TestResult], verbose: bool, tb: &str, quiet: bool) -> String {
    let mut out = String::new();
    let width = term_width();

    // In quiet mode with no failures, show pytest-style dot progress
    if quiet {
        let mut dots = String::new();
        for result in results {
            match &result.outcome {
                TestOutcome::Passed => dots.push('.'),
                TestOutcome::Failed => dots.push('F'),
                TestOutcome::Skipped { .. } => dots.push('s'),
                TestOutcome::XFailed { .. } => dots.push('x'),
                TestOutcome::XPassed => dots.push('X'),
                TestOutcome::Error { .. } => dots.push('E'),
            }
        }
        if !dots.is_empty() {
            let _ = writeln!(out, "{}", dots);
        }
    } else {
        for result in results {
            let status = match &result.outcome {
                TestOutcome::Passed => "PASSED".green().bold().to_string(),
                TestOutcome::Failed => "FAILED".red().bold().to_string(),
                TestOutcome::Skipped { .. } => "SKIPPED".yellow().bold().to_string(),
                TestOutcome::XFailed { .. } => "XFAIL".yellow().to_string(),
                TestOutcome::XPassed => "XPASS".yellow().bold().to_string(),
                TestOutcome::Error { .. } => "ERROR".red().bold().to_string(),
            };

            let _ = write!(out, "{} {}", status, result.test_id);

            if verbose {
                let _ = write!(out, " ({:.3}s)", result.duration.as_secs_f64());
            }

            out.push('\n');

            // Print failure details
            if verbose {
                if let Some(ref err) = result.error {
                    let _ = writeln!(out, "    {}", err.red());
                }
                if !result.stdout.is_empty() {
                    let _ = writeln!(out, "    --- stdout ---\n    {}", result.stdout);
                }
                if !result.stderr.is_empty() {
                    let _ = writeln!(out, "    --- stderr ---\n    {}", result.stderr);
                }
            } else if tb != "no" {
                // In non-verbose mode, show error for failures based on --tb setting
                match &result.outcome {
                    TestOutcome::Failed | TestOutcome::Error { .. } => {
                        if let Some(ref err) = result.error {
                            match tb {
                                "line" => {
                                    // Single-line: just FAILED test_id - last error line
                                    // (shown in the short summary instead)
                                }
                                "short" => {
                                    // Show file:line reference + assertion line
                                    let lines: Vec<&str> = err.lines().collect();
                                    // Find the last file reference line and the assertion
                                    let file_line = lines
                                        .iter()
                                        .rev()
                                        .find(|l| l.contains("File \"") || l.contains(".py:"));
                                    let assertion = lines.last();
                                    if let Some(fl) = file_line {
                                        let _ = writeln!(out, "    {}", fl.trim().red());
                                    }
                                    if let Some(a) = assertion {
                                        if file_line.is_none_or(|fl| *a != *fl) {
                                            let _ = writeln!(out, "    {}", a.trim().red());
                                        }
                                    }
                                }
                                _ => {
                                    // "long" - show full traceback
                                    let _ = writeln!(out, "    {}", err.red());
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // Collect failures for FAILURES section
    let failures: Vec<&TestResult> = results
        .iter()
        .filter(|r| matches!(r.outcome, TestOutcome::Failed | TestOutcome::Error { .. }))
        .collect();

    if !failures.is_empty() {
        // Centered FAILURES header
        let header = format!("{:=^width$}", " FAILURES ", width = width);
        let _ = writeln!(out, "\n{}", header.red().bold());

        for result in &failures {
            let test_header = format!(
                "{:_^width$}",
                format!(" {} ", result.test_id),
                width = width
            );
            let _ = writeln!(out, "{}", test_header);

            if let Some(ref err) = result.error {
                let _ = writeln!(out, "{}", err);
            }

            // Show captured stdout in FAILURES section
            if !result.stdout.is_empty() {
                let cap_header = format!("{:-^width$}", " Captured stdout call ", width = width);
                let _ = writeln!(out, "{}", cap_header);
                let _ = writeln!(out, "{}", result.stdout);
            }

            // Show captured stderr in FAILURES section
            if !result.stderr.is_empty() {
                let cap_header = format!("{:-^width$}", " Captured stderr call ", width = width);
                let _ = writeln!(out, "{}", cap_header);
                let _ = writeln!(out, "{}", result.stderr);
            }

            out.push('\n');
        }
    }

    // Short test summary info
    if !failures.is_empty() {
        let summary_header = format!("{:=^width$}", " short test summary info ", width = width);
        let _ = writeln!(out, "{}", summary_header.yellow());
        for result in &failures {
            let error_summary = result
                .error
                .as_deref()
                .and_then(|e| e.lines().last())
                .unwrap_or("(no details)");
            let prefix = match &result.outcome {
                TestOutcome::Error { .. } => "ERROR",
                _ => "FAILED",
            };
            let _ = writeln!(
                out,
                "{} {} - {}",
                prefix.red().bold(),
                result.test_id,
                error_summary
            );
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
    let c = count_outcomes(results);
    let mut parts = vec![
        format!("{} passed", c.passed),
        format!("{} failed", c.failed),
        format!("{} skipped", c.skipped),
        format!("{} errors", c.errors),
    ];
    if c.xfailed > 0 {
        parts.push(format!("{} xfailed", c.xfailed));
    }
    if c.xpassed > 0 {
        parts.push(format!("{} xpassed", c.xpassed));
    }
    parts.join(", ")
}

/// Write JUnit XML to a file.
///
/// Produces a minimal JUnit XML report compatible with most CI systems.
pub fn write_junit_xml(results: &[TestResult], path: &str) -> anyhow::Result<()> {
    let c = count_outcomes(results);
    let failed = c.failed;
    let skipped = c.skipped;
    let errors = c.errors;
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

/// Print a pytest-style header line with context.
///
/// Example: `====== fastest v2.3.1 — rootdir: /home/user/project, platform: linux ======`
pub fn print_header(rootdir: &Path) {
    let width = term_width();
    let info = format!(
        " fastest v{} — rootdir: {}, platform: {} ",
        fastest_core::VERSION,
        rootdir.display(),
        std::env::consts::OS,
    );
    let header = format!("{:=^width$}", info, width = width);
    eprintln!("{}", header.bold());
}

/// Print a coloured summary line with timing.
///
/// Example: "====== 4 passed, 1 failed in 2.34s ======"
pub fn print_summary(results: &[TestResult], duration: Duration) {
    let width = term_width();
    let c = count_outcomes(results);

    let mut parts: Vec<String> = Vec::new();

    if c.passed > 0 {
        parts.push(format!("{} passed", c.passed).green().bold().to_string());
    }
    if c.failed > 0 {
        parts.push(format!("{} failed", c.failed).red().bold().to_string());
    }
    if c.skipped > 0 {
        parts.push(format!("{} skipped", c.skipped).yellow().to_string());
    }
    if c.xfailed > 0 {
        parts.push(format!("{} xfailed", c.xfailed).yellow().to_string());
    }
    if c.xpassed > 0 {
        parts.push(format!("{} xpassed", c.xpassed).yellow().bold().to_string());
    }
    if c.errors > 0 {
        parts.push(format!("{} errors", c.errors).red().to_string());
    }

    let summary = if parts.is_empty() {
        "no tests ran".dimmed().to_string()
    } else {
        parts.join(", ")
    };

    let has_failures = c.failed > 0 || c.errors > 0 || c.xpassed > 0;

    // Build the centered summary line: "====== N passed in 1.23s ======"
    // We need to strip ANSI codes to compute the visual width
    let plain_summary = strip_ansi_codes(&summary);
    let timing = format!("in {:.2}s", duration.as_secs_f64());
    let inner = format!(" {} {} ", plain_summary, timing);
    let pad = width.saturating_sub(inner.len());
    let left_pad = pad / 2;
    let right_pad = pad - left_pad;
    let line = format!(
        "{} {} {} {}",
        "=".repeat(left_pad),
        summary,
        timing,
        "=".repeat(right_pad)
    );

    if has_failures {
        eprintln!("{}", line.red());
    } else {
        eprintln!("{}", line.green());
    }
}

/// Print a report summary filtered by the given report characters.
///
/// Each character selects a category of test results:
/// - `f`/`F`: failed tests
/// - `E`: errors
/// - `s`/`S`: skipped tests
/// - `x`: xfailed tests
/// - `X`: xpassed tests
/// - `p`/`P`: passed tests
pub fn print_report_summary(results: &[TestResult], report_chars: &str) {
    for ch in report_chars.chars() {
        let (label, matches): (&str, Vec<&TestResult>) = match ch {
            'f' | 'F' => (
                "FAILED",
                results
                    .iter()
                    .filter(|r| matches!(r.outcome, TestOutcome::Failed))
                    .collect(),
            ),
            'E' => (
                "ERRORS",
                results
                    .iter()
                    .filter(|r| matches!(r.outcome, TestOutcome::Error { .. }))
                    .collect(),
            ),
            's' | 'S' => (
                "SKIPPED",
                results
                    .iter()
                    .filter(|r| matches!(r.outcome, TestOutcome::Skipped { .. }))
                    .collect(),
            ),
            'x' => (
                "XFAILED",
                results
                    .iter()
                    .filter(|r| matches!(r.outcome, TestOutcome::XFailed { .. }))
                    .collect(),
            ),
            'X' => (
                "XPASSED",
                results
                    .iter()
                    .filter(|r| matches!(r.outcome, TestOutcome::XPassed))
                    .collect(),
            ),
            'p' | 'P' => (
                "PASSED",
                results
                    .iter()
                    .filter(|r| matches!(r.outcome, TestOutcome::Passed))
                    .collect(),
            ),
            _ => continue,
        };
        if !matches.is_empty() {
            eprintln!("{} {}:", label, matches.len());
            for r in &matches {
                let detail = r
                    .error
                    .as_deref()
                    .or(match &r.outcome {
                        TestOutcome::Skipped { reason } => reason.as_deref(),
                        TestOutcome::XFailed { reason } => reason.as_deref(),
                        _ => None,
                    })
                    .unwrap_or("");
                let short = detail.lines().last().unwrap_or(detail);
                if short.is_empty() {
                    eprintln!("  {}", r.test_id);
                } else {
                    eprintln!("  {} - {}", r.test_id, short);
                }
            }
        }
    }
}

/// Format a single test result for live/streaming output.
pub fn format_result_line(result: &TestResult, verbose: bool) -> String {
    let status = match &result.outcome {
        TestOutcome::Passed => "PASSED".green().bold().to_string(),
        TestOutcome::Failed => "FAILED".red().bold().to_string(),
        TestOutcome::Skipped { .. } => "SKIPPED".yellow().bold().to_string(),
        TestOutcome::XFailed { .. } => "XFAIL".yellow().to_string(),
        TestOutcome::XPassed => "XPASS".yellow().bold().to_string(),
        TestOutcome::Error { .. } => "ERROR".red().bold().to_string(),
    };
    if verbose {
        format!(
            "{} {} ({:.3}s)",
            status,
            result.test_id,
            result.duration.as_secs_f64()
        )
    } else {
        format!("{} {}", status, result.test_id)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Strip ANSI escape codes for computing visual width.
fn strip_ansi_codes(s: &str) -> String {
    let mut out = String::new();
    let mut in_escape = false;
    for ch in s.chars() {
        if in_escape {
            if ch.is_ascii_alphabetic() {
                in_escape = false;
            }
        } else if ch == '\x1b' {
            in_escape = true;
        } else {
            out.push(ch);
        }
    }
    out
}

/// Outcome counts for summary display.
struct OutcomeCounts {
    passed: usize,
    failed: usize,
    skipped: usize,
    errors: usize,
    xfailed: usize,
    xpassed: usize,
}

/// Count outcomes by category.
fn count_outcomes(results: &[TestResult]) -> OutcomeCounts {
    let mut c = OutcomeCounts {
        passed: 0,
        failed: 0,
        skipped: 0,
        errors: 0,
        xfailed: 0,
        xpassed: 0,
    };

    for r in results {
        match &r.outcome {
            TestOutcome::Passed => c.passed += 1,
            TestOutcome::Failed => c.failed += 1,
            TestOutcome::Skipped { .. } => c.skipped += 1,
            TestOutcome::Error { .. } => c.errors += 1,
            TestOutcome::XFailed { .. } => c.xfailed += 1,
            TestOutcome::XPassed => c.xpassed += 1,
        }
    }

    c
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

/// Escape special XML characters and strip control characters illegal in XML 1.0.
///
/// Single-pass implementation that filters illegal chars and escapes XML
/// entities without intermediate allocations.
fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        // XML 1.0 legal characters: #x9 | #xA | #xD | [#x20-#xD7FF] | [#xE000-#xFFFD] | [#x10000-#x10FFFF]
        if !matches!(c, '\u{09}' | '\u{0A}' | '\u{0D}' | '\u{20}'..='\u{D7FF}' | '\u{E000}'..='\u{FFFD}' | '\u{10000}'..='\u{10FFFF}')
        {
            continue;
        }
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
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
        assert_eq!(
            xml_escape("a<b>c&d\"e'f"),
            "a&lt;b&gt;c&amp;d&quot;e&apos;f"
        );
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

    #[test]
    fn test_strip_ansi_codes() {
        assert_eq!(strip_ansi_codes("hello"), "hello");
        assert_eq!(strip_ansi_codes("\x1b[32mgreen\x1b[0m"), "green");
    }

    #[test]
    fn test_term_width_fallback() {
        // In test environment, term_width might return default
        let w = term_width();
        assert!(w > 0);
    }
}
