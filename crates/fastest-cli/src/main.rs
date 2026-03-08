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
    discover_conftest_fixtures, discover_tests, expand_parametrized_tests, filter_by_keyword,
    filter_by_markers, Config, HookArgs, IncrementalTester, PluginManager, TestWatcher,
};
use fastest_execution::HybridExecutor;

use crate::output::{format_results, print_summary, write_junit_xml, OutputFormat};
use crate::progress::create_spinner;

// ---------------------------------------------------------------------------
// CLI argument parsing
// ---------------------------------------------------------------------------

#[derive(Parser, Debug, Clone)]
#[command(
    name = "fastest",
    version,
    about = "Blazing-fast Python test runner written in Rust"
)]
struct Cli {
    /// Test path(s) to discover
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

    /// Disable output capturing (show stdout/stderr in real-time)
    #[arg(short = 's')]
    no_capture: bool,

    /// Stop after N failures
    #[arg(long = "maxfail")]
    maxfail: Option<usize>,

    /// Traceback format: short, long, no
    #[arg(long = "tb", default_value = "short")]
    traceback: String,

    /// Show N slowest test durations (0 to disable)
    #[arg(long = "durations")]
    durations: Option<usize>,

    /// Force color output (yes/no/auto)
    #[arg(long = "color", default_value = "auto")]
    color: String,

    /// Quiet output (only show failures and summary)
    #[arg(short = 'q', long = "quiet")]
    quiet: bool,

    /// Re-run only tests that failed in the last run
    #[arg(long = "lf")]
    last_failed: bool,

    /// Run previously failed tests first, then the rest
    #[arg(long = "ff")]
    failed_first: bool,

    /// Show extra test summary info: (f)ailed, (E)rror, (s)kipped, (x)failed, (X)passed, (p)assed
    #[arg(short = 'r', long = "report")]
    report: Option<String>,

    /// Alias for discover subcommand
    #[arg(long = "collect-only", visible_alias = "co")]
    collect_only: bool,

    /// Ignore paths during collection
    #[arg(long = "ignore", action = clap::ArgAction::Append)]
    ignore_paths: Vec<String>,

    /// Ignore paths matching glob pattern
    #[arg(long = "ignore-glob", action = clap::ArgAction::Append)]
    ignore_glob: Vec<String>,

    /// Deselect specific test IDs
    #[arg(long = "deselect", action = clap::ArgAction::Append)]
    deselect: Vec<String>,
}

#[derive(Subcommand, Debug, Clone)]
enum Commands {
    /// List discovered tests without running them
    Discover {
        /// Test path(s) to discover
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

    // Handle color setting
    match cli.color.as_str() {
        "yes" | "true" | "always" => colored::control::set_override(true),
        "no" | "false" | "never" => colored::control::set_override(false),
        _ => {} // "auto" - let colored decide
    }

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
            if cli.collect_only {
                match run_discover(&cli.paths, &cli.output_format) {
                    Ok(_) => return ExitCode::SUCCESS,
                    Err(e) => {
                        eprintln!("{}: {}", "error".red().bold(), e);
                        return ExitCode::FAILURE;
                    }
                }
            }

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
// Shared helpers
// ---------------------------------------------------------------------------

/// Debounce interval for watch mode (milliseconds).
const WATCH_DEBOUNCE_MS: u64 = 300;

/// Resolve the set of directories to search for tests.
///
/// Priority: explicit CLI paths > `testpaths` from config > current directory.
fn resolve_search_paths(paths: &[String], config: &Config) -> Vec<PathBuf> {
    if !paths.is_empty() {
        paths.iter().map(PathBuf::from).collect()
    } else if !config.testpaths.is_empty() {
        config.testpaths.clone()
    } else {
        vec![PathBuf::from(".")]
    }
}

/// Inject autouse fixtures from conftest.py files into test items.
fn inject_autouse_fixtures(tests: &mut [fastest_core::TestItem]) {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    if let Ok(conftest_fixtures) = discover_conftest_fixtures(&cwd) {
        let autouse_names: Vec<String> = conftest_fixtures
            .values()
            .filter(|f| f.autouse)
            .map(|f| f.name.clone())
            .collect();
        if !autouse_names.is_empty() {
            for test in tests.iter_mut() {
                for name in &autouse_names {
                    if !test.fixture_deps.contains(name) {
                        test.fixture_deps.push(name.clone());
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
    let search_paths = resolve_search_paths(paths, &config);

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
                    test.path.display().to_string()
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
// addopts support
// ---------------------------------------------------------------------------

/// Parse the `addopts` string from config and apply recognised flags to the CLI
/// state. CLI args that were explicitly set by the user already take precedence
/// because clap will have populated them before this function runs; `apply_addopts`
/// only fills in values that are still at their defaults.
fn apply_addopts(cli: &mut Cli, addopts: &str) {
    if addopts.is_empty() {
        return;
    }
    // Simple split on whitespace, apply known flags
    for token in addopts.split_whitespace() {
        match token {
            "-v" | "--verbose" => cli.verbose = true,
            "-q" | "--quiet" => cli.quiet = true,
            "-x" | "--exitfirst" => cli.exitfirst = true,
            "-s" => cli.no_capture = true,
            _ => {
                if let Some(val) = token.strip_prefix("--tb=") {
                    cli.traceback = val.to_string();
                } else if let Some(val) = token.strip_prefix("--maxfail=") {
                    if let Ok(n) = val.parse::<usize>() {
                        cli.maxfail = Some(n);
                    }
                } else if let Some(val) = token.strip_prefix("-k") {
                    if !val.is_empty() {
                        cli.keyword = Some(val.to_string());
                    }
                } else if let Some(val) = token.strip_prefix("--color=") {
                    cli.color = val.to_string();
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Test execution pipeline
// ---------------------------------------------------------------------------

fn run_tests(cli: &Cli) -> anyhow::Result<bool> {
    let start = Instant::now();

    // 1. Load config
    let config = Config::load()?;

    // Apply addopts from config to a mutable copy of CLI args
    let mut cli = cli.clone();
    apply_addopts(&mut cli, &config.addopts);

    // 2. Plugin manager
    let mut plugins = if cli.no_plugins {
        PluginManager::new()
    } else {
        PluginManager::with_builtins()?
    };
    plugins.initialize_all()?;

    // 3. Discover tests
    let search_paths = resolve_search_paths(&cli.paths, &config);
    let tests = discover_tests(&search_paths, &config)?;

    // 4. Expand parametrized tests
    let mut tests = expand_parametrized_tests(tests)?;

    // 4b. Inject autouse fixtures into test fixture_deps
    inject_autouse_fixtures(&mut tests);

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

    // 6b. Filter by --ignore, --ignore-glob, --deselect
    let tests = if !cli.ignore_paths.is_empty() {
        tests
            .into_iter()
            .filter(|t| {
                !cli.ignore_paths
                    .iter()
                    .any(|p| t.path.starts_with(std::path::Path::new(p)))
            })
            .collect()
    } else {
        tests
    };
    let tests = if !cli.ignore_glob.is_empty() {
        tests
            .into_iter()
            .filter(|t| {
                let test_path = t.path.to_string_lossy();
                !cli.ignore_glob
                    .iter()
                    .any(|pattern| glob_match::glob_match(pattern, test_path.as_ref()))
            })
            .collect()
    } else {
        tests
    };
    let tests = if !cli.deselect.is_empty() {
        tests
            .into_iter()
            .filter(|t| !cli.deselect.contains(&t.id))
            .collect()
    } else {
        tests
    };

    // 7. Plugin hook: collection_modifyitems
    let _ = plugins.call_hook("collection_modifyitems", &HookArgs::new());

    // 8. Incremental filtering
    let tests = if cli.incremental {
        let cwd = std::env::current_dir()?;
        let tester = IncrementalTester::new(&cwd)?;
        if !tester.is_git_repo() {
            eprintln!(
                "{}: --incremental requires a git repository; running all tests",
                "warning".yellow().bold()
            );
        }
        tester.filter_unchanged(tests)?
    } else {
        tests
    };

    // 8b. Last-failed / failed-first filtering
    let tests = if cli.last_failed || cli.failed_first {
        let cwd = std::env::current_dir()?;
        let last_failed = fastest_core::load_lastfailed(&cwd);
        if cli.last_failed {
            // Only run tests that failed last time
            tests
                .into_iter()
                .filter(|t| last_failed.contains(&t.id))
                .collect()
        } else {
            // failed-first: put previously-failed tests first, then the rest
            let mut failed: Vec<fastest_core::TestItem> = Vec::new();
            let mut rest: Vec<fastest_core::TestItem> = Vec::new();
            for t in tests {
                if last_failed.contains(&t.id) {
                    failed.push(t);
                } else {
                    rest.push(t);
                }
            }
            failed.extend(rest);
            failed
        }
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

    let max_failures = if cli.exitfirst { Some(1) } else { cli.maxfail };

    let results = if let Some(max_fail) = max_failures {
        // Run tests one at a time, stop after max_fail failures.
        let inprocess = executor.inprocess();
        let mut results = Vec::new();
        let mut fail_count = 0;
        for test in &tests {
            let batch = inprocess.execute(std::slice::from_ref(test));
            let result = match batch.into_iter().next() {
                Some(r) => r,
                None => {
                    eprintln!(
                        "{}: executor returned no result for {}",
                        "error".red().bold(),
                        test.id
                    );
                    break;
                }
            };
            if !result.passed() {
                fail_count += 1;
            }
            results.push(result);
            if fail_count >= max_fail {
                break;
            }
        }
        results
    } else if !cli.no_progress && !cli.verbose {
        // Run with spinner (execution is synchronous, results arrive in bulk)
        let spinner = create_spinner(tests.len());
        let results = executor.execute(&tests);
        spinner.finish_and_clear();
        results
    } else {
        executor.execute(&tests)
    };

    // 9b. Save last-failed cache
    {
        let cwd = std::env::current_dir()?;
        let failed_ids: std::collections::HashSet<String> = results
            .iter()
            .filter(|r| {
                matches!(
                    r.outcome,
                    fastest_core::TestOutcome::Failed | fastest_core::TestOutcome::Error { .. }
                )
            })
            .map(|r| r.test_id.clone())
            .collect();
        fastest_core::save_lastfailed(&cwd, &failed_ids);
    }

    // 10. Plugin hook: runtest_logreport
    let _ = plugins.call_hook("runtest_logreport", &HookArgs::new());

    // 11. Format and print output
    let output_format =
        OutputFormat::from_str_with_junit(Some(&cli.output_format), cli.junit_xml.clone());

    let formatted = format_results(
        &results,
        &output_format,
        cli.verbose,
        &cli.traceback,
        cli.quiet,
    );
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

    // Print report summary if -r flag is set
    if let Some(ref report_chars) = cli.report {
        output::print_report_summary(&results, report_chars);
    }

    // Show slowest tests if --durations is set
    if let Some(n) = cli.durations {
        if n > 0 {
            let mut sorted: Vec<_> = results.iter().collect();
            sorted.sort_by(|a, b| b.duration.cmp(&a.duration));
            let count = n.min(sorted.len());
            eprintln!("\n{}", format!("slowest {} durations", count).bold());
            for r in sorted.iter().take(count) {
                eprintln!("  {:.3}s {}", r.duration.as_secs_f64(), r.test_id);
            }
        }
    }

    // Shutdown plugins
    plugins.shutdown_all()?;

    // Return success if no failures/errors (XPassed is a failure per pytest conventions)
    let has_failures = results.iter().any(|r| {
        matches!(
            r.outcome,
            fastest_core::TestOutcome::Failed
                | fastest_core::TestOutcome::XPassed
                | fastest_core::TestOutcome::Error { .. }
        )
    });
    Ok(!has_failures)
}

// ---------------------------------------------------------------------------
// Watch mode
// ---------------------------------------------------------------------------

/// Configuration snapshot for watch mode re-runs (captured from CLI args).
#[derive(Clone)]
struct WatchConfig {
    paths: Vec<String>,
    keyword: Option<String>,
    marker: Option<String>,
    output_format: String,
    junit_xml: Option<String>,
    verbose: bool,
    no_plugins: bool,
    workers: Option<usize>,
    exitfirst: bool,
    incremental: bool,
    maxfail: Option<usize>,
    traceback: String,
    quiet: bool,
    last_failed: bool,
    failed_first: bool,
    report: Option<String>,
    ignore_paths: Vec<String>,
    ignore_glob: Vec<String>,
    deselect: Vec<String>,
}

impl WatchConfig {
    fn from_cli(cli: &Cli) -> Self {
        Self {
            paths: cli.paths.clone(),
            keyword: cli.keyword.clone(),
            marker: cli.marker.clone(),
            output_format: cli.output_format.clone(),
            junit_xml: cli.junit_xml.clone(),
            verbose: cli.verbose,
            no_plugins: cli.no_plugins,
            workers: cli.workers,
            exitfirst: cli.exitfirst,
            incremental: cli.incremental,
            maxfail: cli.maxfail,
            traceback: cli.traceback.clone(),
            quiet: cli.quiet,
            last_failed: cli.last_failed,
            failed_first: cli.failed_first,
            report: cli.report.clone(),
            ignore_paths: cli.ignore_paths.clone(),
            ignore_glob: cli.ignore_glob.clone(),
            deselect: cli.deselect.clone(),
        }
    }
}

fn run_watch(cli: &Cli) -> anyhow::Result<()> {
    eprintln!("{} watching for changes...", "fastest".cyan().bold());

    let watcher = TestWatcher::new(WATCH_DEBOUNCE_MS);
    let watch_paths: Vec<PathBuf> = if cli.paths.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        cli.paths.iter().map(PathBuf::from).collect()
    };

    // Initial run
    let _ = run_tests(cli);

    let watch_cfg = WatchConfig::from_cli(cli);

    watcher.watch_paths(&watch_paths, move |changed_paths| {
        eprintln!(
            "\n{} {} file{} changed, re-running tests...",
            "fastest".cyan().bold(),
            changed_paths.len(),
            if changed_paths.len() == 1 { "" } else { "s" }
        );
        for path in changed_paths {
            eprintln!("  {}", path.display().to_string().dimmed());
        }
        if let Err(e) = run_watch_cycle(&watch_cfg) {
            eprintln!("{}: {}", "error".red().bold(), e);
        }
    })?;

    Ok(())
}

/// Execute a test cycle from a watch-mode re-run.
fn run_watch_cycle(cfg: &WatchConfig) -> anyhow::Result<()> {
    let start = Instant::now();
    let config = Config::load()?;

    let mut plugins = if cfg.no_plugins {
        PluginManager::new()
    } else {
        PluginManager::with_builtins()?
    };
    plugins.initialize_all()?;

    let search_paths = resolve_search_paths(&cfg.paths, &config);
    let tests = discover_tests(&search_paths, &config)?;
    let mut tests = expand_parametrized_tests(tests)?;

    // Inject autouse fixtures into test fixture_deps
    inject_autouse_fixtures(&mut tests);

    let tests = if let Some(ref expr) = cfg.marker {
        filter_by_markers(&tests, expr)
    } else {
        tests
    };
    let tests = if let Some(ref expr) = cfg.keyword {
        filter_by_keyword(&tests, expr)
    } else {
        tests
    };

    // Filter by --ignore, --ignore-glob, --deselect
    let tests = if !cfg.ignore_paths.is_empty() {
        tests
            .into_iter()
            .filter(|t| {
                !cfg.ignore_paths
                    .iter()
                    .any(|p| t.path.starts_with(std::path::Path::new(p)))
            })
            .collect()
    } else {
        tests
    };
    let tests = if !cfg.ignore_glob.is_empty() {
        tests
            .into_iter()
            .filter(|t| {
                let test_path = t.path.to_string_lossy();
                !cfg.ignore_glob
                    .iter()
                    .any(|pattern| glob_match::glob_match(pattern, test_path.as_ref()))
            })
            .collect()
    } else {
        tests
    };
    let tests = if !cfg.deselect.is_empty() {
        tests
            .into_iter()
            .filter(|t| !cfg.deselect.contains(&t.id))
            .collect()
    } else {
        tests
    };

    // Incremental filtering
    let tests = if cfg.incremental {
        let cwd = std::env::current_dir()?;
        let tester = IncrementalTester::new(&cwd)?;
        tester.filter_unchanged(tests)?
    } else {
        tests
    };

    // Last-failed / failed-first filtering
    let tests = if cfg.last_failed || cfg.failed_first {
        let cwd = std::env::current_dir()?;
        let last_failed = fastest_core::load_lastfailed(&cwd);
        if cfg.last_failed {
            tests
                .into_iter()
                .filter(|t| last_failed.contains(&t.id))
                .collect()
        } else {
            let mut failed: Vec<fastest_core::TestItem> = Vec::new();
            let mut rest: Vec<fastest_core::TestItem> = Vec::new();
            for t in tests {
                if last_failed.contains(&t.id) {
                    failed.push(t);
                } else {
                    rest.push(t);
                }
            }
            failed.extend(rest);
            failed
        }
    } else {
        tests
    };

    if tests.is_empty() {
        eprintln!("{}", "no tests collected".yellow());
        plugins.shutdown_all()?;
        return Ok(());
    }

    eprintln!(
        "{} {} collecting {} test{}...",
        "fastest".cyan().bold(),
        format!("v{}", fastest_core::VERSION).dimmed(),
        tests.len(),
        if tests.len() == 1 { "" } else { "s" }
    );

    let executor = HybridExecutor::with_workers(cfg.workers);

    let max_failures = if cfg.exitfirst { Some(1) } else { cfg.maxfail };

    let results = if let Some(max_fail) = max_failures {
        let inprocess = executor.inprocess();
        let mut results = Vec::new();
        let mut fail_count = 0;
        for test in &tests {
            let batch = inprocess.execute(std::slice::from_ref(test));
            let result = match batch.into_iter().next() {
                Some(r) => r,
                None => {
                    eprintln!(
                        "{}: executor returned no result for {}",
                        "error".red().bold(),
                        test.id
                    );
                    break;
                }
            };
            if !result.passed() {
                fail_count += 1;
            }
            results.push(result);
            if fail_count >= max_fail {
                break;
            }
        }
        results
    } else {
        executor.execute(&tests)
    };

    // Save last-failed cache
    {
        let cwd = std::env::current_dir()?;
        let failed_ids: std::collections::HashSet<String> = results
            .iter()
            .filter(|r| {
                matches!(
                    r.outcome,
                    fastest_core::TestOutcome::Failed | fastest_core::TestOutcome::Error { .. }
                )
            })
            .map(|r| r.test_id.clone())
            .collect();
        fastest_core::save_lastfailed(&cwd, &failed_ids);
    }

    let output_format =
        OutputFormat::from_str_with_junit(Some(&cfg.output_format), cfg.junit_xml.clone());
    let formatted = format_results(
        &results,
        &output_format,
        cfg.verbose,
        &cfg.traceback,
        cfg.quiet,
    );
    if !formatted.is_empty() {
        println!("{}", formatted);
    }

    let duration = start.elapsed();
    print_summary(&results, duration);

    // Print report summary if -r flag is set
    if let Some(ref report_chars) = cfg.report {
        output::print_report_summary(&results, report_chars);
    }

    plugins.shutdown_all()?;
    Ok(())
}
