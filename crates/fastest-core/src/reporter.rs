use crate::executor::TestResult;
use colored::*;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub trait Reporter: Send + Sync {
    fn on_test_start(&self, test_id: &str);
    fn on_test_complete(&self, result: &TestResult);
    fn on_run_complete(&self, results: &[TestResult], duration: Duration);
}

/// Pretty reporter with colored output and progress bars
pub struct PrettyReporter {
    multi_progress: MultiProgress,
    progress_bar: ProgressBar,
    passed: Arc<Mutex<usize>>,
    failed: Arc<Mutex<usize>>,
    skipped: Arc<Mutex<usize>>,
    verbose: bool,
}

impl PrettyReporter {
    pub fn new(total_tests: usize, verbose: bool) -> Self {
        let multi_progress = MultiProgress::new();
        let progress_bar = multi_progress.add(ProgressBar::new(total_tests as u64));

        progress_bar.set_style(
            ProgressStyle::default_bar()
                .template(
                    "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}",
                )
                .unwrap()
                .progress_chars("#>-"),
        );

        Self {
            multi_progress,
            progress_bar,
            passed: Arc::new(Mutex::new(0)),
            failed: Arc::new(Mutex::new(0)),
            skipped: Arc::new(Mutex::new(0)),
            verbose,
        }
    }

    pub fn multi_progress(&self) -> &MultiProgress {
        &self.multi_progress
    }
}

impl Reporter for PrettyReporter {
    fn on_test_start(&self, test_id: &str) {
        if self.verbose {
            self.progress_bar
                .set_message(format!("Running {}", test_id));
        }
    }

    fn on_test_complete(&self, result: &TestResult) {
        self.progress_bar.inc(1);

        let symbol = if result.passed {
            if result.output == "SKIPPED" {
                *self.skipped.lock().unwrap() += 1;
                "S".yellow()
            } else if result.output == "XFAIL" {
                *self.passed.lock().unwrap() += 1;
                "x".green()
            } else {
                *self.passed.lock().unwrap() += 1;
                "✓".green()
            }
        } else {
            *self.failed.lock().unwrap() += 1;
            if result.output == "XPASS" {
                "X".red()
            } else {
                "✗".red()
            }
        };

        let message = format!("{} {}", symbol, result.test_id);
        self.progress_bar.set_message(message);

        // Show failures immediately in verbose mode
        if self.verbose && !result.passed {
            self.progress_bar.println(format!(
                "\n{} {}\n{}",
                "FAILED".red().bold(),
                result.test_id,
                result
                    .error
                    .as_ref()
                    .unwrap_or(&"Unknown error".to_string())
            ));
        }
    }

    fn on_run_complete(&self, results: &[TestResult], duration: Duration) {
        self.progress_bar.finish_and_clear();

        let passed = *self.passed.lock().unwrap();
        let failed = *self.failed.lock().unwrap();
        let skipped = *self.skipped.lock().unwrap();

        println!("\n{}", "=".repeat(70));

        // Summary line
        if failed == 0 {
            println!(
                "{} {} passed{} in {:.2}s",
                "✓".green().bold(),
                format!("{} tests", passed).green().bold(),
                if skipped > 0 {
                    format!(", {} skipped", skipped).yellow().to_string()
                } else {
                    String::new()
                },
                duration.as_secs_f64()
            );
        } else {
            println!(
                "{} {} passed, {} {} failed{} in {:.2}s",
                passed,
                "passed".green(),
                failed,
                "FAILED".red().bold(),
                if skipped > 0 {
                    format!(", {} skipped", skipped).yellow().to_string()
                } else {
                    String::new()
                },
                duration.as_secs_f64()
            );

            // Show failed test details
            println!("\n{}", "Failed Tests:".red().bold());
            for result in results {
                if !result.passed && result.output != "SKIPPED" {
                    println!("\n{} {}", "FAILED".red(), result.test_id);
                    if let Some(error) = &result.error {
                        // Pretty print the error with indentation
                        for line in error.lines() {
                            println!("  {}", line);
                        }
                    }

                    // Show stdout/stderr if available
                    if !result.stdout.is_empty() {
                        println!("\n  {}:", "stdout".dimmed());
                        for line in result.stdout.lines() {
                            println!("    {}", line);
                        }
                    }

                    if !result.stderr.is_empty() {
                        println!("\n  {}:", "stderr".dimmed());
                        for line in result.stderr.lines() {
                            println!("    {}", line);
                        }
                    }
                }
            }
        }
    }
}

/// JSON reporter for CI/CD integration
pub struct JsonReporter {
    results: Arc<Mutex<Vec<TestResult>>>,
}

impl JsonReporter {
    pub fn new() -> Self {
        Self {
            results: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl Default for JsonReporter {
    fn default() -> Self {
        Self::new()
    }
}

impl Reporter for JsonReporter {
    fn on_test_start(&self, _test_id: &str) {}

    fn on_test_complete(&self, result: &TestResult) {
        self.results.lock().unwrap().push(result.clone());
    }

    fn on_run_complete(&self, results: &[TestResult], duration: Duration) {
        let output = serde_json::json!({
            "duration": duration.as_secs_f64(),
            "total": results.len(),
            "passed": results.iter().filter(|r| r.passed).count(),
            "failed": results.iter().filter(|r| !r.passed).count(),
            "results": results,
        });

        println!("{}", serde_json::to_string_pretty(&output).unwrap());
    }
}

/// JUnit XML reporter for CI/CD integration
pub struct JunitReporter {
    output_path: Option<String>,
}

impl JunitReporter {
    pub fn new(output_path: Option<String>) -> Self {
        Self { output_path }
    }
}

impl Reporter for JunitReporter {
    fn on_test_start(&self, _test_id: &str) {}

    fn on_test_complete(&self, _result: &TestResult) {}

    fn on_run_complete(&self, results: &[TestResult], duration: Duration) {
        let mut xml = String::new();
        xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str(&format!(
            "<testsuite name=\"fastest\" tests=\"{}\" failures=\"{}\" time=\"{:.3}\">\n",
            results.len(),
            results.iter().filter(|r| !r.passed).count(),
            duration.as_secs_f64()
        ));

        for result in results {
            let class_name = result.test_id.split("::").next().unwrap_or("unknown");
            let test_name = result.test_id.split("::").last().unwrap_or(&result.test_id);

            xml.push_str(&format!(
                "  <testcase classname=\"{}\" name=\"{}\" time=\"{:.3}\"",
                class_name,
                test_name,
                result.duration.as_secs_f64()
            ));

            if result.passed {
                xml.push_str("/>\n");
            } else {
                xml.push_str(">\n");
                if let Some(error) = &result.error {
                    xml.push_str(&format!(
                        "    <failure message=\"{}\">{}</failure>\n",
                        escape_xml(error.lines().next().unwrap_or("Test failed")),
                        escape_xml(error)
                    ));
                }
                xml.push_str("  </testcase>\n");
            }
        }

        xml.push_str("</testsuite>\n");

        if let Some(path) = &self.output_path {
            std::fs::write(path, xml).expect("Failed to write JUnit XML");
        } else {
            print!("{}", xml);
        }
    }
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
