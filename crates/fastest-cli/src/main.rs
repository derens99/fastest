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
use fastest_execution::timeout::TimeoutConfig;
use fastest_execution::HybridExecutor;

use crate::output::{
    format_result_line, format_results, print_header, print_summary, write_junit_xml, OutputFormat,
};
use crate::progress::create_progress_bar;

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

    /// Per-test timeout in seconds (default: 60)
    #[arg(long = "timeout")]
    timeout: Option<u64>,

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

    /// Show registered markers and exit
    #[arg(long = "markers")]
    show_markers: bool,

    /// Show available fixtures and exit
    #[arg(long = "fixtures")]
    show_fixtures: bool,

    /// Ignore paths during collection
    #[arg(long = "ignore", action = clap::ArgAction::Append)]
    ignore_paths: Vec<String>,

    /// Ignore paths matching glob pattern
    #[arg(long = "ignore-glob", action = clap::ArgAction::Append)]
    ignore_glob: Vec<String>,

    /// Deselect specific test IDs
    #[arg(long = "deselect", action = clap::ArgAction::Append)]
    deselect: Vec<String>,

    /// Stepwise: stop on first failure, next run starts from that test
    #[arg(long = "stepwise", visible_alias = "sw")]
    stepwise: bool,

    /// Override rootdir detection
    #[arg(long = "rootdir")]
    rootdir: Option<String>,

    /// Show only N slowest durations (used with --durations)
    #[arg(long = "durations-min", default_value = "0.005")]
    durations_min: f64,

    /// Emit GitHub Actions annotations for test failures
    #[arg(long = "github-actions")]
    github_actions: bool,

    /// Suppress the header line
    #[arg(long = "no-header")]
    no_header: bool,

    /// Generate shell completions and exit (bash, zsh, fish, powershell)
    #[arg(long = "completions")]
    completions: Option<String>,
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

    // Handle --completions: generate shell completion script and exit
    if let Some(ref shell_name) = cli.completions {
        use clap::CommandFactory;
        use clap_complete::{generate, Shell};
        let shell = match shell_name.to_lowercase().as_str() {
            "bash" => Shell::Bash,
            "zsh" => Shell::Zsh,
            "fish" => Shell::Fish,
            "powershell" | "ps" => Shell::PowerShell,
            _ => {
                eprintln!(
                    "{}: unknown shell '{}'. Supported: bash, zsh, fish, powershell",
                    "error".red().bold(),
                    shell_name
                );
                return ExitCode::FAILURE;
            }
        };
        let mut cmd = Cli::command();
        generate(shell, &mut cmd, "fastest", &mut std::io::stdout());
        return ExitCode::SUCCESS;
    }

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
            // Handle --markers
            if cli.show_markers {
                match run_show_markers() {
                    Ok(_) => return ExitCode::SUCCESS,
                    Err(e) => {
                        eprintln!("{}: {}", "error".red().bold(), e);
                        return ExitCode::FAILURE;
                    }
                }
            }

            // Handle --fixtures
            if cli.show_fixtures {
                match run_show_fixtures() {
                    Ok(_) => return ExitCode::SUCCESS,
                    Err(e) => {
                        eprintln!("{}: {}", "error".red().bold(), e);
                        return ExitCode::FAILURE;
                    }
                }
            }

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

/// A parsed node-ID like `tests/test_math.py::TestCalc::test_add`.
struct NodeId {
    /// The file/directory path portion.
    path: PathBuf,
    /// Optional class name filter (e.g., "TestCalc").
    class_filter: Option<String>,
    /// Optional function name filter (e.g., "test_add").
    function_filter: Option<String>,
}

/// Parse a CLI path argument, splitting on `::` for node-ID syntax.
///
/// Examples:
/// - `tests/` → path only
/// - `tests/test_math.py::test_add` → path + function
/// - `tests/test_math.py::TestCalc::test_add` → path + class + function
fn parse_node_id(arg: &str) -> NodeId {
    let parts: Vec<&str> = arg.splitn(2, "::").collect();
    if parts.len() == 1 {
        NodeId {
            path: PathBuf::from(arg),
            class_filter: None,
            function_filter: None,
        }
    } else {
        let path = PathBuf::from(parts[0]);
        let selectors: Vec<&str> = parts[1].split("::").collect();
        match selectors.len() {
            1 => NodeId {
                path,
                class_filter: None,
                function_filter: Some(selectors[0].to_string()),
            },
            _ => NodeId {
                path,
                class_filter: Some(selectors[0].to_string()),
                function_filter: Some(selectors[selectors.len() - 1].to_string()),
            },
        }
    }
}

/// Resolve the set of directories to search for tests.
///
/// Priority: explicit CLI paths > `testpaths` from config > current directory.
fn resolve_search_paths(paths: &[String], config: &Config) -> Vec<PathBuf> {
    if !paths.is_empty() {
        // Strip :: node-ID suffixes — only use the path portion for discovery
        paths.iter().map(|p| parse_node_id(p).path).collect()
    } else if !config.testpaths.is_empty() {
        config.testpaths.clone()
    } else {
        vec![PathBuf::from(".")]
    }
}

/// Extract node-ID filters from CLI path arguments.
///
/// Returns (class_filters, function_filters) from any `::` syntax in paths.
fn extract_node_filters(paths: &[String]) -> (Vec<String>, Vec<String>) {
    let mut class_filters = Vec::new();
    let mut function_filters = Vec::new();
    for p in paths {
        let node = parse_node_id(p);
        if let Some(c) = node.class_filter {
            class_filters.push(c);
        }
        if let Some(f) = node.function_filter {
            function_filters.push(f);
        }
    }
    (class_filters, function_filters)
}

/// Extract file path and optional line number from a test ID.
///
/// Test IDs are like `tests/test_math.py::TestCalc::test_add`. We extract
/// the file portion (`tests/test_math.py`) and return `None` for line since
/// we don't have line info in the test ID format.
fn extract_file_line_from_id(test_id: &str) -> (&str, Option<usize>) {
    if let Some(idx) = test_id.find("::") {
        (&test_id[..idx], None)
    } else {
        (test_id, None)
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

/// Common filter options shared between `run_tests` and `run_watch_cycle`.
struct FilterOpts<'a> {
    keyword: Option<&'a str>,
    marker: Option<&'a str>,
    ignore_paths: &'a [String],
    ignore_glob: &'a [String],
    deselect: &'a [String],
    incremental: bool,
    last_failed: bool,
    failed_first: bool,
}

/// Apply the full filter pipeline to a list of discovered tests.
///
/// Applies in order: markers → keyword → ignore → ignore-glob → deselect →
/// incremental → last-failed/failed-first.
fn apply_filters(
    tests: Vec<fastest_core::TestItem>,
    opts: &FilterOpts,
) -> anyhow::Result<Vec<fastest_core::TestItem>> {
    // Marker filter
    let tests = if let Some(expr) = opts.marker {
        filter_by_markers(&tests, expr)
    } else {
        tests
    };

    // Keyword filter
    let tests = if let Some(expr) = opts.keyword {
        filter_by_keyword(&tests, expr)
    } else {
        tests
    };

    // --ignore (path prefix)
    let tests = if !opts.ignore_paths.is_empty() {
        tests
            .into_iter()
            .filter(|t| {
                !opts
                    .ignore_paths
                    .iter()
                    .any(|p| t.path.starts_with(std::path::Path::new(p)))
            })
            .collect()
    } else {
        tests
    };

    // --ignore-glob
    let tests = if !opts.ignore_glob.is_empty() {
        tests
            .into_iter()
            .filter(|t| {
                let test_path = t.path.to_string_lossy();
                !opts
                    .ignore_glob
                    .iter()
                    .any(|pattern| glob_match::glob_match(pattern, test_path.as_ref()))
            })
            .collect()
    } else {
        tests
    };

    // --deselect
    let tests = if !opts.deselect.is_empty() {
        tests
            .into_iter()
            .filter(|t| !opts.deselect.contains(&t.id))
            .collect()
    } else {
        tests
    };

    // Incremental filtering
    let tests = if opts.incremental {
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

    // Last-failed / failed-first
    let tests = if opts.last_failed || opts.failed_first {
        let cwd = std::env::current_dir()?;
        let last_failed = fastest_core::load_lastfailed(&cwd);
        if opts.last_failed {
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

    Ok(tests)
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
            // Tree view: group by module, then class
            let mut current_module = String::new();
            let mut current_class: Option<String> = None;

            for test in &tests {
                let module = test.path.display().to_string();

                // Print module header on change
                if module != current_module {
                    if !current_module.is_empty() {
                        println!();
                    }
                    println!("{}", format!("<Module {}>", module).bold());
                    current_module = module;
                    current_class = None;
                }

                // Print class header on change
                if test.class_name != current_class {
                    if let Some(ref cls) = test.class_name {
                        println!("  {}", format!("<Class {}>", cls).bold());
                    }
                    current_class = test.class_name.clone();
                }

                // Print test function
                let indent = if test.class_name.is_some() {
                    "    "
                } else {
                    "  "
                };
                println!(
                    "{}{} {}",
                    indent,
                    "<Function".dimmed(),
                    format!("{}>", test.function_name).dimmed()
                );
            }

            println!(
                "\n{}",
                format!("{} tests collected", tests.len()).green().bold()
            );
        }
    }

    Ok(())
}

/// Show registered markers from config.
fn run_show_markers() -> anyhow::Result<()> {
    let config = Config::load()?;

    println!("{}", "=== registered markers (from config) ===".bold());

    // Built-in markers always available
    let builtins = [
        ("skip", "skip the test unconditionally"),
        ("skipif", "skip the test if condition is true"),
        ("xfail", "mark test as expected to fail"),
        ("parametrize", "generate multiple test cases"),
        ("timeout", "set per-test timeout in seconds"),
        ("usefixtures", "use fixtures without argument injection"),
    ];

    for (name, desc) in &builtins {
        println!("  @pytest.mark.{}: {}", name.cyan().bold(), desc);
    }

    // Custom markers from config
    if !config.markers.is_empty() {
        println!("\n{}", "--- custom markers ---".dimmed());
        for marker in &config.markers {
            // Markers in config are formatted as "name: description" or just "name"
            let (name, desc) = if let Some(idx) = marker.find(':') {
                (&marker[..idx], marker[idx + 1..].trim())
            } else {
                (marker.as_str(), "")
            };
            if desc.is_empty() {
                println!("  @pytest.mark.{}", name.cyan().bold());
            } else {
                println!("  @pytest.mark.{}: {}", name.cyan().bold(), desc);
            }
        }
    }

    Ok(())
}

/// Show available fixtures with scope info.
fn run_show_fixtures() -> anyhow::Result<()> {
    println!("{}", "=== available fixtures ===".bold());

    // Built-in fixtures
    let builtins = [
        (
            "tmp_path",
            "function",
            "Temporary directory unique to the test invocation",
        ),
        (
            "tmp_path_factory",
            "session",
            "Factory for creating temp directories",
        ),
        ("capsys", "function", "Capture stdout/stderr writes"),
        ("capfd", "function", "Capture file descriptor 1/2 output"),
        (
            "monkeypatch",
            "function",
            "Modify objects, dicts, or environment vars",
        ),
        ("caplog", "function", "Capture log records"),
        ("request", "function", "Information about the test request"),
    ];

    println!("{}", "--- builtin ---".dimmed());
    for (name, scope, desc) in &builtins {
        println!(
            "  {} [{}]\n      {}",
            name.cyan().bold(),
            scope.yellow(),
            desc
        );
    }

    // Conftest fixtures
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    if let Ok(conftest_fixtures) = discover_conftest_fixtures(&cwd) {
        if !conftest_fixtures.is_empty() {
            println!("\n{}", "--- from conftest.py ---".dimmed());
            let mut fixtures: Vec<_> = conftest_fixtures.values().collect();
            fixtures.sort_by_key(|f| &f.name);
            for f in fixtures {
                let autouse_tag = if f.autouse { " (autouse)" } else { "" };
                println!(
                    "  {} [{}]{}\n      defined in {}",
                    f.name.cyan().bold(),
                    format!("{}", f.scope).yellow(),
                    autouse_tag.dimmed(),
                    f.func_path.display().to_string().dimmed()
                );
            }
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
    // Split respecting shell quoting (handles quoted strings properly)
    let tokens = shlex_split(addopts);
    let mut iter = tokens.iter().peekable();
    while let Some(token) = iter.next() {
        let token = token.as_str();
        match token {
            "-v" | "--verbose" => cli.verbose = true,
            "-q" | "--quiet" => cli.quiet = true,
            "-x" | "--exitfirst" => cli.exitfirst = true,
            "-s" => cli.no_capture = true,
            "--lf" => cli.last_failed = true,
            "--ff" => cli.failed_first = true,
            "--no-progress" => cli.no_progress = true,
            "--no-header" => cli.no_header = true,
            "--stepwise" | "--sw" => cli.stepwise = true,
            "--github-actions" => cli.github_actions = true,
            _ => {
                if let Some(val) = token.strip_prefix("--tb=") {
                    cli.traceback = val.to_string();
                } else if let Some(val) = token.strip_prefix("--maxfail=") {
                    if let Ok(n) = val.parse::<usize>() {
                        cli.maxfail = Some(n);
                    }
                } else if token == "-k" {
                    // -k "expression" (space-separated)
                    if let Some(val) = iter.next() {
                        cli.keyword = Some(val.clone());
                    }
                } else if let Some(val) = token.strip_prefix("-k") {
                    // -k"expression" (no space)
                    if !val.is_empty() {
                        cli.keyword = Some(val.to_string());
                    }
                } else if token == "-m" {
                    if let Some(val) = iter.next() {
                        cli.marker = Some(val.clone());
                    }
                } else if let Some(val) = token.strip_prefix("--color=") {
                    cli.color = val.to_string();
                } else if let Some(val) = token.strip_prefix("--timeout=") {
                    if let Ok(n) = val.parse::<u64>() {
                        cli.timeout = Some(n);
                    }
                } else if let Some(val) = token.strip_prefix("-j") {
                    if let Ok(n) = val.parse::<usize>() {
                        cli.workers = Some(n);
                    }
                } else if let Some(val) = token.strip_prefix("--workers=") {
                    if let Ok(n) = val.parse::<usize>() {
                        cli.workers = Some(n);
                    }
                }
            }
        }
    }
}

/// Simple shlex-style splitting that handles single and double quotes.
fn shlex_split(s: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let chars = s.chars();
    let mut in_single = false;
    let mut in_double = false;

    for ch in chars {
        match ch {
            '\'' if !in_double => {
                in_single = !in_single;
            }
            '"' if !in_single => {
                in_double = !in_double;
            }
            ' ' | '\t' if !in_single && !in_double => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            _ => {
                current.push(ch);
            }
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

// ---------------------------------------------------------------------------
// Test execution pipeline
// ---------------------------------------------------------------------------

fn run_tests(cli: &Cli) -> anyhow::Result<bool> {
    let start = Instant::now();

    // 0. Apply --rootdir override: change cwd so config loading and relative paths work
    if let Some(ref rootdir) = cli.rootdir {
        let root = PathBuf::from(rootdir);
        if root.is_dir() {
            std::env::set_current_dir(&root)?;
        } else {
            anyhow::bail!("--rootdir path does not exist: {}", rootdir);
        }
    }

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

    // 5-8b. Apply unified filter pipeline
    let _ = plugins.call_hook("collection_modifyitems", &HookArgs::new());
    let tests = apply_filters(
        tests,
        &FilterOpts {
            keyword: cli.keyword.as_deref(),
            marker: cli.marker.as_deref(),
            ignore_paths: &cli.ignore_paths,
            ignore_glob: &cli.ignore_glob,
            deselect: &cli.deselect,
            incremental: cli.incremental,
            last_failed: cli.last_failed,
            failed_first: cli.failed_first,
        },
    )?;

    // 8c. Apply node-ID filters from :: syntax in paths
    let (class_filters, function_filters) = extract_node_filters(&cli.paths);
    let tests = if !class_filters.is_empty() || !function_filters.is_empty() {
        tests
            .into_iter()
            .filter(|t| {
                let class_ok = class_filters.is_empty()
                    || t.class_name
                        .as_ref()
                        .is_some_and(|c| class_filters.contains(c));
                let func_ok =
                    function_filters.is_empty() || function_filters.contains(&t.function_name);
                // If only function filters, match function name OR class name
                if class_filters.is_empty() {
                    func_ok
                        || t.class_name
                            .as_ref()
                            .is_some_and(|c| function_filters.contains(c))
                } else {
                    class_ok && func_ok
                }
            })
            .collect()
    } else {
        tests
    };

    // 8d. Stepwise mode: skip tests until the previously-failed test is found
    let tests = if cli.stepwise {
        let cwd = std::env::current_dir()?;
        if let Some(ref last_stop) = fastest_core::load_stepwise(&cwd) {
            // Find the index of the previously-failed test and resume from there
            if let Some(idx) = tests.iter().position(|t| &t.id == last_stop) {
                tests.into_iter().skip(idx).collect()
            } else {
                tests // test not found, run all
            }
        } else {
            tests // no stepwise state, run all
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
    let rootdir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    if !cli.no_header {
        print_header(&rootdir);
        eprintln!(
            "{} collecting {} test{}...",
            "fastest".cyan().bold(),
            tests.len(),
            if tests.len() == 1 { "" } else { "s" }
        );
    }

    // 9. Execute tests
    let timeout_config =
        TimeoutConfig::with_duration(std::time::Duration::from_secs(cli.timeout.unwrap_or(60)));
    let executor = HybridExecutor::with_config(cli.workers, timeout_config);

    // Stepwise implies exitfirst (stop on first failure)
    let max_failures = if cli.exitfirst || cli.stepwise {
        Some(1)
    } else {
        cli.maxfail
    };

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
    } else if !cli.no_progress {
        // Streaming execution with live progress bar
        let pb = create_progress_bar(tests.len());
        let verbose = cli.verbose;
        executor.execute_streaming(&tests, &move |result| {
            pb.inc(1);
            if verbose {
                pb.println(format_result_line(result, true));
            }
        })
    } else if cli.verbose {
        // Verbose without progress bar — stream results to stderr
        executor.execute_streaming(&tests, &|result| {
            eprintln!("{}", format_result_line(result, true));
        })
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

        // Stepwise: save the test we stopped at, or clear if all passed
        if cli.stepwise {
            if let Some(first_failed) = failed_ids.iter().next() {
                fastest_core::save_stepwise(&cwd, first_failed);
            } else {
                fastest_core::clear_stepwise(&cwd);
            }
        }
    }

    // 10. Plugin hook: runtest_logreport
    let _ = plugins.call_hook("runtest_logreport", &HookArgs::new());

    // 11. Format and print output
    // Display format and JUnit XML are now independent: --output json --junit-xml report.xml works
    let display_format = OutputFormat::parse_display_format(Some(&cli.output_format));
    let formatted = format_results(
        &results,
        &display_format,
        cli.verbose,
        &cli.traceback,
        cli.quiet,
    );
    if !formatted.is_empty() {
        println!("{}", formatted);
    }

    // Write JUnit XML as a side-channel if requested
    if let Some(ref junit_path) = cli.junit_xml {
        write_junit_xml(&results, junit_path)?;
        eprintln!("JUnit XML report written to {}", junit_path);
    }

    // Print summary
    let duration = start.elapsed();
    if !cli.no_header {
        print_summary(&results, duration);
    }

    // GitHub Actions annotations: ::error file=...::message
    if cli.github_actions {
        for r in &results {
            match &r.outcome {
                fastest_core::TestOutcome::Failed | fastest_core::TestOutcome::Error { .. } => {
                    // Try to extract file and line from test ID
                    let (file, line) = extract_file_line_from_id(&r.test_id);
                    let msg = r
                        .error
                        .as_deref()
                        .and_then(|e| e.lines().last())
                        .unwrap_or("Test failed");
                    if let Some(l) = line {
                        println!("::error file={},line={}::{}", file, l, msg);
                    } else {
                        println!("::error file={}::{}", file, msg);
                    }
                }
                _ => {}
            }
        }
    }

    // Print report summary if -r flag is set
    if let Some(ref report_chars) = cli.report {
        output::print_report_summary(&results, report_chars);
    }

    // Show slowest tests if --durations is set
    if let Some(n) = cli.durations {
        if n > 0 {
            let mut sorted: Vec<_> = results.iter().collect();
            sorted.sort_by(|a, b| b.duration.cmp(&a.duration));
            // Apply --durations-min threshold: filter out tests below the threshold
            let threshold = cli.durations_min;
            let sorted: Vec<_> = sorted
                .into_iter()
                .filter(|r| r.duration.as_secs_f64() >= threshold)
                .collect();
            let count = n.min(sorted.len());
            if count > 0 {
                eprintln!("\n{}", format!("slowest {} durations", count).bold());
                for r in sorted.iter().take(count) {
                    eprintln!("  {:.3}s {}", r.duration.as_secs_f64(), r.test_id);
                }
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
#[allow(dead_code)]
struct WatchConfig {
    paths: Vec<String>,
    keyword: Option<String>,
    marker: Option<String>,
    output_format: String,
    junit_xml: Option<String>,
    verbose: bool,
    no_plugins: bool,
    workers: Option<usize>,
    timeout: Option<u64>,
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
    stepwise: bool,
    durations: Option<usize>,
    durations_min: f64,
    no_progress: bool,
    no_capture: bool,
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
            timeout: cli.timeout,
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
            stepwise: cli.stepwise,
            durations: cli.durations,
            durations_min: cli.durations_min,
            no_progress: cli.no_progress,
            no_capture: cli.no_capture,
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

    // Apply unified filter pipeline
    let tests = apply_filters(
        tests,
        &FilterOpts {
            keyword: cfg.keyword.as_deref(),
            marker: cfg.marker.as_deref(),
            ignore_paths: &cfg.ignore_paths,
            ignore_glob: &cfg.ignore_glob,
            deselect: &cfg.deselect,
            incremental: cfg.incremental,
            last_failed: cfg.last_failed,
            failed_first: cfg.failed_first,
        },
    )?;

    if tests.is_empty() {
        eprintln!("{}", "no tests collected".yellow());
        plugins.shutdown_all()?;
        return Ok(());
    }

    let rootdir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    print_header(&rootdir);
    eprintln!(
        "{} collecting {} test{}...",
        "fastest".cyan().bold(),
        tests.len(),
        if tests.len() == 1 { "" } else { "s" }
    );

    let timeout_config =
        TimeoutConfig::with_duration(std::time::Duration::from_secs(cfg.timeout.unwrap_or(60)));
    let executor = HybridExecutor::with_config(cfg.workers, timeout_config);

    let max_failures = if cfg.exitfirst || cfg.stepwise {
        Some(1)
    } else {
        cfg.maxfail
    };

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
    } else if !cfg.no_progress {
        let pb = create_progress_bar(tests.len());
        let verbose = cfg.verbose;
        executor.execute_streaming(&tests, &move |result| {
            pb.inc(1);
            if verbose {
                pb.println(format_result_line(result, true));
            }
        })
    } else if cfg.verbose {
        executor.execute_streaming(&tests, &|result| {
            eprintln!("{}", format_result_line(result, true));
        })
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

        // Stepwise: save the test we stopped at, or clear if all passed
        if cfg.stepwise {
            if let Some(first_failed) = failed_ids.iter().next() {
                fastest_core::save_stepwise(&cwd, first_failed);
            } else {
                fastest_core::clear_stepwise(&cwd);
            }
        }
    }

    let display_format = OutputFormat::parse_display_format(Some(&cfg.output_format));
    let formatted = format_results(
        &results,
        &display_format,
        cfg.verbose,
        &cfg.traceback,
        cfg.quiet,
    );
    if !formatted.is_empty() {
        println!("{}", formatted);
    }

    // Write JUnit XML as a side-channel if requested
    if let Some(ref junit_path) = cfg.junit_xml {
        if let Err(e) = write_junit_xml(&results, junit_path) {
            eprintln!("Failed to write JUnit XML: {}", e);
        }
    }

    let duration = start.elapsed();
    print_summary(&results, duration);

    // Print report summary if -r flag is set
    if let Some(ref report_chars) = cfg.report {
        output::print_report_summary(&results, report_chars);
    }

    // Show slowest durations if requested
    if let Some(n) = cfg.durations {
        if n > 0 {
            let mut sorted: Vec<_> = results.iter().collect();
            sorted.sort_by(|a, b| b.duration.cmp(&a.duration));
            let threshold = cfg.durations_min;
            let sorted: Vec<_> = sorted
                .into_iter()
                .filter(|r| r.duration.as_secs_f64() >= threshold)
                .collect();
            let count = n.min(sorted.len());
            if count > 0 {
                eprintln!("\n{}", format!("slowest {} durations", count).bold());
                for r in sorted.iter().take(count) {
                    eprintln!("  {:.3}s {}", r.duration.as_secs_f64(), r.test_id);
                }
            }
        }
    }

    plugins.shutdown_all()?;
    Ok(())
}
