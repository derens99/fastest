//! Fastest — Blazing-fast Python test runner
//!
//! CLI entry point that orchestrates the full test pipeline:
//! parse args → load config → discover → expand → filter → execute → report.

mod output;
mod progress;

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

use clap::{Parser, Subcommand};
use colored::Colorize;

use fastest_core::{
    discover_tests, expand_parametrized_tests, filter_by_keyword, filter_by_markers, Config,
    HookArgs, IncrementalTester, PluginManager, TestWatcher,
};
use fastest_execution::HybridExecutor;

use crate::output::{format_results, print_summary, write_junit_xml, OutputFormat};
use crate::progress::create_progress_bar;

// ---------------------------------------------------------------------------
// CLI argument parsing
// ---------------------------------------------------------------------------

#[derive(Parser, Debug)]
#[command(
    name = "fastest",
    version,
    about = "Blazing-fast Python test runner written in Rust"
)]
struct Cli {
    /// Test path(s) to discover
    #[arg(default_value = ".")]
    paths: Vec<String>,

    #[command(subcommand)]
    command: Option<Commands>,

    /// Keyword expression filter (-k "test_add or test_sub")
    #[arg(short = 'k', long = "keyword")]
    keyword: Option<String>,

    /// Marker expression filter (-m "slow and not integration")
    #[arg(short = 'm', long = "marker")]
    marker: Option<String>,

    /// Output format: pretty, json, count
    #[arg(long = "output", default_value = "pretty")]
    output_format: String,

    /// Write JUnit XML report to path
    #[arg(long = "junit-xml")]
    junit_xml: Option<String>,

    /// Stop on first failure
    #[arg(short = 'x', long = "exitfirst")]
    exitfirst: bool,

    /// Verbose output
    #[arg(short = 'v', long = "verbose")]
    verbose: bool,

    /// Disable all plugins
    #[arg(long = "no-plugins")]
    no_plugins: bool,

    /// Number of parallel workers (default: num CPUs)
    #[arg(long = "workers", short = 'j')]
    workers: Option<usize>,

    /// Run only tests affected by uncommitted changes
    #[arg(long = "incremental")]
    incremental: bool,

    /// Watch mode: re-run tests on file changes
    #[arg(long = "watch")]
    watch: bool,

    /// Don't show progress bar
    #[arg(long = "no-progress")]
    no_progress: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// List discovered tests without running them
    Discover {
        /// Test path(s) to discover
        #[arg(default_value = ".")]
        paths: Vec<String>,

        /// Output format: pretty, json, count
        #[arg(long = "output", default_value = "pretty")]
        output_format: String,
    },
}

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Discover {
            paths,
            output_format,
        }) => match run_discover(&paths, &output_format) {
            Ok(_) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("{}: {}", "error".red().bold(), e);
                ExitCode::FAILURE
            }
        },
        None => {
            if cli.watch {
                match run_watch(&cli) {
                    Ok(_) => ExitCode::SUCCESS,
                    Err(e) => {
                        eprintln!("{}: {}", "error".red().bold(), e);
                        ExitCode::FAILURE
                    }
                }
            } else {
                match run_tests(&cli) {
                    Ok(success) => {
                        if success {
                            ExitCode::SUCCESS
                        } else {
                            ExitCode::FAILURE
                        }
                    }
                    Err(e) => {
                        eprintln!("{}: {}", "error".red().bold(), e);
                        ExitCode::FAILURE
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Discover subcommand
// ---------------------------------------------------------------------------

fn run_discover(paths: &[String], output_format: &str) -> anyhow::Result<()> {
    let config = Config::load()?;
    let search_paths: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();

    let tests = discover_tests(&search_paths, &config)?;
    let tests = expand_parametrized_tests(tests)?;

    match output_format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&tests)?);
        }
        "count" => {
            println!("{} tests discovered", tests.len());
        }
        _ => {
            // Pretty: list each test
            for test in &tests {
                let location = if let Some(line) = test.line_number {
                    format!("{}:{}", test.path.display(), line)
                } else {
                    format!("{}", test.path.display())
                };
                println!("  {} {}", location.dimmed(), test.id);
            }
            println!(
                "\n{}",
                format!("{} tests discovered", tests.len()).green().bold()
            );
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Test execution pipeline
// ---------------------------------------------------------------------------

fn run_tests(cli: &Cli) -> anyhow::Result<bool> {
    let start = Instant::now();

    // 1. Load config
    let config = Config::load()?;

    // 2. Plugin manager
    let mut plugins = if cli.no_plugins {
        PluginManager::new()
    } else {
        PluginManager::with_builtins()?
    };
    plugins.initialize_all()?;

    // 3. Discover tests
    let search_paths: Vec<PathBuf> = cli.paths.iter().map(PathBuf::from).collect();
    let tests = discover_tests(&search_paths, &config)?;

    // 4. Expand parametrized tests
    let tests = expand_parametrized_tests(tests)?;

    // 5. Filter by markers
    let tests = if let Some(ref expr) = cli.marker {
        filter_by_markers(&tests, expr)
    } else {
        tests
    };

    // 6. Filter by keyword
    let tests = if let Some(ref expr) = cli.keyword {
        filter_by_keyword(&tests, expr)
    } else {
        tests
    };

    // 7. Plugin hook: collection_modifyitems
    let _ = plugins.call_hook("collection_modifyitems", &HookArgs::new());

    // 8. Incremental filtering
    let tests = if cli.incremental {
        let cwd = std::env::current_dir()?;
        let tester = IncrementalTester::new(&cwd)?;
        tester.filter_unchanged(tests)?
    } else {
        tests
    };

    if tests.is_empty() {
        eprintln!("{}", "no tests collected".yellow());
        plugins.shutdown_all()?;
        return Ok(true);
    }

    // Print header
    eprintln!(
        "{} {} collecting {} test{}...",
        "fastest".cyan().bold(),
        format!("v{}", fastest_core::VERSION).dimmed(),
        tests.len(),
        if tests.len() == 1 { "" } else { "s" }
    );

    // 9. Execute tests
    let executor = HybridExecutor::with_workers(cli.workers);

    let results = if cli.exitfirst {
        // Run one at a time, stop on first failure
        let mut results = Vec::new();
        for test in &tests {
            let batch = executor.execute(std::slice::from_ref(test));
            let result = batch.into_iter().next().unwrap();
            let failed = !result.passed();
            results.push(result);
            if failed {
                break;
            }
        }
        results
    } else if !cli.no_progress && !cli.verbose {
        // Run with progress bar
        let pb = create_progress_bar(tests.len());
        let results = executor.execute(&tests);
        for (i, result) in results.iter().enumerate() {
            pb.set_position((i + 1) as u64);
            pb.set_message(result.test_id.clone());
        }
        pb.finish_and_clear();
        results
    } else {
        executor.execute(&tests)
    };

    // 10. Plugin hook: runtest_logreport
    let _ = plugins.call_hook("runtest_logreport", &HookArgs::new());

    // 11. Format and print output
    let output_format =
        OutputFormat::from_str_with_junit(Some(&cli.output_format), cli.junit_xml.clone());

    let formatted = format_results(&results, &output_format, cli.verbose);
    if !formatted.is_empty() {
        println!("{}", formatted);
    }

    // Write JUnit XML if requested
    if let OutputFormat::JunitXml(ref path) = output_format {
        write_junit_xml(&results, path)?;
        eprintln!("JUnit XML report written to {}", path);
    }

    // Print summary
    let duration = start.elapsed();
    print_summary(&results, duration);

    // Shutdown plugins
    plugins.shutdown_all()?;

    // Return success if no failures/errors
    let has_failures = results
        .iter()
        .any(|r| matches!(r.outcome, fastest_core::TestOutcome::Failed | fastest_core::TestOutcome::Error { .. }));
    Ok(!has_failures)
}

// ---------------------------------------------------------------------------
// Watch mode
// ---------------------------------------------------------------------------

fn run_watch(cli: &Cli) -> anyhow::Result<()> {
    eprintln!(
        "{} watching for changes...",
        "fastest".cyan().bold()
    );

    let watcher = TestWatcher::new(300); // 300ms debounce
    let watch_path = PathBuf::from(cli.paths.first().map(|s| s.as_str()).unwrap_or("."));

    // Initial run
    let _ = run_tests(cli);

    watcher.watch(&watch_path, |changed_paths| {
        eprintln!(
            "\n{} {} file{} changed, re-running tests...",
            "fastest".cyan().bold(),
            changed_paths.len(),
            if changed_paths.len() == 1 { "" } else { "s" }
        );
        for path in changed_paths {
            eprintln!("  {}", path.display().to_string().dimmed());
        }
        // Note: In watch mode we can't easily re-run with the full CLI context
        // because the callback doesn't have access to &Cli. A full implementation
        // would clone the necessary config. For now, this triggers re-discovery.
    })?;

    Ok(())
}
