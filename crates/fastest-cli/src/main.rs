use clap::{Parser, Subcommand};
use colored::*;
use fastest_core::{
    check_for_updates, default_cache_path, discover_tests, discover_tests_cached,
    executor::UltraFastExecutor, filter_by_markers, Config, DevExperienceConfig, DiscoveryCache,
    PluginCompatibilityConfig, UpdateChecker,
};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
use std::time::Instant;

#[derive(Parser)]
#[command(name = "fastest")]
#[command(about = "A blazing fast Python test runner built with Rust", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Paths to discover tests from
    #[arg()]
    paths: Vec<PathBuf>,

    /// Filter tests by pattern
    #[arg(short = 'k', long)]
    filter: Option<String>,

    /// Filter tests by marker expression (e.g., "not slow", "skip or xfail")
    #[arg(short = 'm', long = "markers")]
    markers: Option<String>,

    /// Number of parallel workers (0 = auto-detect CPUs, 1 = sequential)
    #[arg(short = 'n', long)]
    workers: Option<usize>,

    /// Stop on first failure
    #[arg(short = 'x', long = "fail-fast")]
    fail_fast: bool,

    /// Verbose output
    #[arg(short = 'v', long = "verbose")]
    verbose: bool,

    /// Output format (pretty, json, junit)
    #[arg(long = "output", default_value = "pretty")]
    output_format: String,

    /// Disable discovery cache
    #[arg(long = "no-cache")]
    no_cache: bool,

    /// Use persistent worker pool (experimental)
    #[arg(long = "persistent-workers")]
    persistent_workers: bool,

    /// Only run tests affected by recent changes (experimental)
    #[arg(long = "incremental")]
    incremental: bool,

    /// Watch files for changes and re-run tests
    #[arg(short = 'w', long = "watch")]
    watch: bool,

    /// Enable coverage collection
    #[arg(long = "cov")]
    coverage: bool,

    /// Coverage report format (term, html, xml, json)
    #[arg(long = "cov-report", default_value = "term")]
    coverage_report: String,

    /// Source directories for coverage
    #[arg(long = "cov-source")]
    coverage_source: Vec<PathBuf>,

    /// Enable debugging with pdb
    #[arg(long = "pdb")]
    pdb: bool,

    /// Enhanced error reporting
    #[arg(long = "enhanced-errors")]
    enhanced_errors: bool,

    /// Enable mock support (pytest-mock compatibility)
    #[arg(long = "mock")]
    mock: bool,

    /// Asyncio mode for async tests (auto, strict)
    #[arg(long = "asyncio-mode")]
    asyncio_mode: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Discover tests without running them
    Discover {
        /// Output format for discovery
        #[arg(long = "format", default_value = "list")]
        format: String,
    },

    /// Run tests (default command)
    Run {
        /// Show test output even when passing
        #[arg(long = "show-output")]
        show_output: bool,
    },

    /// Show version information
    Version,

    /// Update fastest to the latest version
    Update {
        /// Check for updates without installing
        #[arg(long = "check")]
        check_only: bool,
    },
}

fn main() -> anyhow::Result<()> {
    let mut cli = Cli::parse();

    // Load config and apply defaults
    let config = Config::load().unwrap_or_default();
    apply_config_to_cli(&config, &mut cli);

    match &cli.command {
        Some(Commands::Discover { format }) => discover_command(&cli, format),
        Some(Commands::Version) => version_command(),
        Some(Commands::Update { check_only }) => update_command(&cli, *check_only),
        Some(Commands::Run { show_output }) => run_command(&cli, *show_output),
        None => {
            // Check for updates on normal runs (non-intrusive)
            let _ = check_for_updates();
            run_command(&cli, false)
        }
    }
}

fn apply_config_to_cli(config: &Config, cli: &mut Cli) {
    // Apply testpaths if no paths specified
    if cli.paths.is_empty() {
        cli.paths = config.testpaths.clone();
    }

    // Apply fastest-specific config
    if cli.workers.is_none() {
        cli.workers = config.fastest.workers;
    }

    // Apply other config values
    if cli.verbose {
        eprintln!("Loaded config from: {:?}", config);
    }
}

fn discover_command(cli: &Cli, format: &str) -> anyhow::Result<()> {
    let start = Instant::now();

    let mut all_tests = Vec::new();

    // Discover tests from all paths
    for path in &cli.paths {
        let tests = if cli.no_cache {
            if cli.verbose {
                eprintln!("Discovering tests in {} (cache disabled)", path.display());
            }
            discover_tests(path)?
        } else {
            if cli.verbose {
                eprintln!("Discovering tests in {} (cache enabled)", path.display());
            }
            let cache_path = default_cache_path();
            let mut cache =
                DiscoveryCache::load(&cache_path).unwrap_or_else(|_| DiscoveryCache::new());
            let tests = discover_tests_cached(path, &mut cache)?;

            // Save cache
            if let Err(e) = cache.save(&cache_path) {
                eprintln!("Warning: Failed to save discovery cache: {}", e);
            }

            tests
        };
        all_tests.extend(tests);
    }

    let tests = all_tests;

    let duration = start.elapsed();

    // Apply marker filter first if provided
    let tests = if let Some(markers) = &cli.markers {
        filter_by_markers(tests, markers)?
    } else {
        tests
    };

    // Apply text filter if provided
    let filtered_tests: Vec<_> = if let Some(filter) = &cli.filter {
        tests
            .into_iter()
            .filter(|t| t.name.contains(filter) || t.id.contains(filter))
            .collect()
    } else {
        tests
    };

    match format {
        "json" => {
            let json = serde_json::to_string_pretty(&filtered_tests)?;
            println!("{}", json);
        }
        "count" => {
            println!("{}", filtered_tests.len());
        }
        _ => {
            println!("{}", "Test Discovery Results".bold().green());
            println!("{}", "=".repeat(50));
            println!(
                "Found {} tests in {:.3}s\n",
                filtered_tests.len(),
                duration.as_secs_f64()
            );

            if let Some(markers) = &cli.markers {
                println!("  {} {}\n", "Marker filter:".dimmed(), markers.yellow());
            }

            for test in &filtered_tests {
                println!("  {} {}", "●".green(), test.id);
                if cli.verbose {
                    println!("    {} {}", "Path:".dimmed(), test.path.display());
                    println!("    {} {}", "Line:".dimmed(), test.line_number);
                    if test.is_async {
                        println!("    {} {}", "Type:".dimmed(), "async".yellow());
                    }
                    if !test.decorators.is_empty() {
                        println!(
                            "    {} {}",
                            "Decorators:".dimmed(),
                            test.decorators.join(", ")
                        );
                    }
                    if !test.fixture_deps.is_empty() {
                        println!(
                            "    {} {}",
                            "Fixtures:".dimmed(),
                            test.fixture_deps.join(", ").cyan()
                        );
                    }
                }
            }
        }
    }

    Ok(())
}

fn run_command(cli: &Cli, show_output: bool) -> anyhow::Result<()> {
    let start = Instant::now();

    // Discover tests from all paths
    let mut discovered_tests = Vec::new();

    for path in &cli.paths {
        let tests = if cli.no_cache {
            if cli.verbose {
                eprintln!("Discovering tests in {} (cache disabled)", path.display());
            }
            discover_tests(path)?
        } else {
            if cli.verbose {
                eprintln!("Discovering tests in {} (cache enabled)", path.display());
            }
            let cache_path = default_cache_path();
            let mut cache =
                DiscoveryCache::load(&cache_path).unwrap_or_else(|_| DiscoveryCache::new());
            let tests = discover_tests_cached(path, &mut cache)?;

            // Save cache
            if let Err(e) = cache.save(&cache_path) {
                eprintln!("Warning: Failed to save discovery cache: {}", e);
            }

            tests
        };
        discovered_tests.extend(tests);
    }

    let total_tests_discovered = discovered_tests.len();

    // Apply marker filter first if provided
    if let Some(markers) = &cli.markers {
        if cli.verbose {
            eprintln!("Applying marker filter: {}", markers);
        }
        discovered_tests = filter_by_markers(discovered_tests, markers)?;
    }

    // Apply text filter
    if let Some(filter) = &cli.filter {
        if cli.verbose {
            eprintln!("Applying text filter: {}", filter);
        }
        discovered_tests.retain(|t| t.name.contains(filter) || t.id.contains(filter));
    }

    if discovered_tests.is_empty() {
        println!("{}", "No tests found matching filters!".yellow());
        return Ok(());
    }

    println!("Found {} tests\n", total_tests_discovered);

    // Create progress bar
    let pb = ProgressBar::new(discovered_tests.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );

    // Run tests - all executors now use the same ultra-fast engine internally
    if cli.verbose {
        eprintln!("⚡ Using ultra-fast executor with persistent worker pool");
    }

    let workers = cli.workers.unwrap_or(0);
    let num_workers = if workers == 0 { None } else { Some(workers) };

    // Use the optimized executor wrapper for compatibility
    let mut executor = UltraFastExecutor::new_with_workers(num_workers, cli.verbose);

    // Configure developer experience features
    let mut dev_config = DevExperienceConfig::default();
    if cli.pdb {
        dev_config.debug_enabled = true;
    }
    if cli.enhanced_errors {
        dev_config.enhanced_reporting = true;
    }
    if dev_config.debug_enabled || dev_config.enhanced_reporting {
        executor = executor.with_dev_experience(dev_config);
    }

    // Configure plugin compatibility
    let mut plugin_config = PluginCompatibilityConfig::default();

    // Check for xdist (distributed testing) - workers specified means xdist
    if cli.workers.is_some() && cli.workers.unwrap() > 1 {
        plugin_config.xdist_enabled = true;
        plugin_config.xdist_workers = cli.workers.unwrap();
    }

    // Coverage support
    if cli.coverage {
        plugin_config.coverage_enabled = true;
        plugin_config.coverage_source = if cli.coverage_source.is_empty() {
            vec![std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))]
        } else {
            cli.coverage_source.clone()
        };
    }

    // Mock support
    if cli.mock {
        plugin_config.mock_enabled = true;
    }

    // Asyncio support
    if let Some(asyncio_mode) = &cli.asyncio_mode {
        plugin_config.asyncio_enabled = true;
        plugin_config.asyncio_mode = asyncio_mode.clone();
    }

    // Check for conflicting options before moving plugin_config
    let coverage_warning = cli.coverage && !plugin_config.coverage_enabled;

    // Enable plugin compatibility if any plugins are enabled
    if plugin_config.xdist_enabled
        || plugin_config.coverage_enabled
        || plugin_config.mock_enabled
        || plugin_config.asyncio_enabled
    {
        executor = executor.with_plugin_compatibility(plugin_config);
    }

    // Enable coverage if requested (legacy support for non-plugin coverage)
    if coverage_warning {
        let source_dirs = if cli.coverage_source.is_empty() {
            vec![std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))]
        } else {
            cli.coverage_source.clone()
        };
        executor = executor.with_coverage(source_dirs);
    }

    let results = executor.execute(discovered_tests)?;

    // Process results
    let mut passed = 0;
    let mut failed = 0;
    let mut failed_tests = Vec::new();

    for result in &results {
        pb.inc(1);
        if result.passed {
            passed += 1;
            pb.set_message(format!("{} {}", "✓".green(), result.test_id));
        } else {
            failed += 1;
            failed_tests.push(result);
            pb.set_message(format!("{} {}", "✗".red(), result.test_id));

            if cli.fail_fast {
                break;
            }
        }
    }

    pb.finish_and_clear();

    // Print results
    let duration = start.elapsed();
    println!("\n{}", "=".repeat(70));

    if failed == 0 {
        println!(
            "{} {} passed in {:.2}s",
            "✓".green().bold(),
            format!("{} tests", passed).green().bold(),
            duration.as_secs_f64()
        );
    } else {
        println!(
            "{} {} passed, {} {} failed in {:.2}s",
            passed,
            "passed".green(),
            failed,
            "FAILED".red().bold(),
            duration.as_secs_f64()
        );

        // Show failed test details
        println!("\n{}", "Failed Tests:".red().bold());
        for test in &failed_tests {
            println!("\n{} {}", "FAILED".red(), test.test_id);
            if let Some(error) = &test.error {
                println!("  {}", error);
            }
            if show_output || !test.stderr.is_empty() {
                println!("\n{}", "--- stderr ---".dimmed());
                println!("{}", test.stderr);
            }
        }
    }

    // Generate coverage report if enabled
    if cli.coverage {
        if cli.verbose {
            eprintln!("\nGenerating coverage report...");
        }

        // Use coverage module to generate report
        let coverage_format = match cli.coverage_report.as_str() {
            "html" => fastest_core::coverage::CoverageFormat::Html,
            "xml" => fastest_core::coverage::CoverageFormat::Xml,
            "json" => fastest_core::coverage::CoverageFormat::Json,
            _ => fastest_core::coverage::CoverageFormat::Terminal,
        };

        let source_dirs = if cli.coverage_source.is_empty() {
            vec![std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))]
        } else {
            cli.coverage_source.clone()
        };

        let coverage_runner = fastest_core::coverage::CoverageRunner::new(source_dirs);

        // Combine coverage data from multiple workers
        if let Err(e) = coverage_runner.combine_coverage() {
            eprintln!("Warning: Failed to combine coverage data: {}", e);
        }

        // Generate report
        match coverage_runner.generate_report(coverage_format) {
            Ok(report) => {
                if cli.verbose {
                    eprintln!(
                        "\nCoverage: {:.1}% ({}/{} statements)",
                        report.total_coverage, report.covered_statements, report.total_statements
                    );
                }
            }
            Err(e) => {
                eprintln!("Warning: Failed to generate coverage report: {}", e);
            }
        }
    }

    // Exit with error code if tests failed
    if failed > 0 {
        std::process::exit(1);
    }

    Ok(())
}

fn version_command() -> anyhow::Result<()> {
    println!("fastest {}", env!("CARGO_PKG_VERSION"));
    println!("The blazing fast Python test runner built with Rust");
    Ok(())
}

fn update_command(cli: &Cli, check_only: bool) -> anyhow::Result<()> {
    let checker = UpdateChecker::new();

    if check_only {
        match checker.check_update()? {
            Some(new_version) => {
                println!("Current version: v{}", env!("CARGO_PKG_VERSION"));
                println!("Latest version: v{}", new_version);
                println!("\nAn update is available! Run 'fastest update' to install it.");
            }
            None => {
                println!(
                    "You are running the latest version (v{})!",
                    env!("CARGO_PKG_VERSION")
                );
            }
        }
    } else {
        checker.update(cli.verbose)?;
    }

    Ok(())
}
