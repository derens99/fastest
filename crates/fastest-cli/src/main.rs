use clap::{Parser, Subcommand};
use colored::*;
use fastest_core::{
    default_cache_path, discover_tests, discover_tests_cached, executor::OptimizedExecutor,
    filter_by_markers, parser::ParserType, BatchExecutor, Config, DiscoveryCache, ParallelExecutor,
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

    /// Parser to use for test discovery (regex or ast)
    #[arg(long = "parser")]
    parser: Option<String>,

    /// Optimization level (standard, optimized, aggressive)
    #[arg(long = "optimizer")]
    optimizer: Option<String>,

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
}

// Helper function to convert parser string to ParserType
fn get_parser_type(parser_str: &str, verbose: bool) -> ParserType {
    match parser_str {
        "ast" => ParserType::Ast,
        "regex" => ParserType::Regex,
        _ => {
            if verbose {
                eprintln!(
                    "Warning: Unknown parser type '{}' specified. Defaulting to AST parser.",
                    parser_str
                );
            }
            ParserType::Ast // Default to AST if unknown
        }
    }
}

fn main() -> anyhow::Result<()> {
    let mut cli = Cli::parse();

    // Load config and apply defaults
    let config = Config::load().unwrap_or_default();
    apply_config_to_cli(&config, &mut cli);

    match &cli.command {
        Some(Commands::Discover { format }) => discover_command(&cli, format),
        Some(Commands::Version) => version_command(),
        Some(Commands::Run { show_output }) => run_command(&cli, *show_output),
        None => run_command(&cli, false),
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

    if cli.parser.is_none() {
        cli.parser = Some(config.fastest.parser.clone());
    }

    if cli.optimizer.is_none() {
        cli.optimizer = Some(config.fastest.optimizer.clone());
    }

    // Apply other config values
    if cli.verbose {
        eprintln!("Loaded config from: {:?}", config);
    }
}

fn discover_command(cli: &Cli, format: &str) -> anyhow::Result<()> {
    let start = Instant::now();
    let parser_str = cli.parser.as_deref().unwrap_or("ast");
    let parser_type = get_parser_type(parser_str, cli.verbose);

    let mut all_tests = Vec::new();

    // Discover tests from all paths
    for path in &cli.paths {
        let tests = if cli.no_cache {
            if cli.verbose {
                eprintln!(
                    "Discovering tests in {} with {:?} parser (cache disabled)",
                    path.display(),
                    parser_type
                );
            }
            discover_tests(path, parser_type)?
        } else {
            if cli.verbose {
                eprintln!(
                    "Discovering tests in {} with {:?} parser (cache enabled)",
                    path.display(),
                    parser_type
                );
            }
            let cache_path = default_cache_path();
            let mut cache =
                DiscoveryCache::load(&cache_path).unwrap_or_else(|_| DiscoveryCache::new());
            // discover_tests_cached expects a ParserType now
            let tests = discover_tests_cached(path, &mut cache, parser_type)?;

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
    let parser_str = cli.parser.as_deref().unwrap_or("ast");
    let parser_type = get_parser_type(parser_str, cli.verbose);

    // Discover tests from all paths
    let mut discovered_tests = Vec::new();

    for path in &cli.paths {
        let tests = if cli.no_cache {
            if cli.verbose {
                eprintln!(
                    "Discovering tests in {} with {:?} parser (cache disabled)",
                    path.display(),
                    parser_type
                );
            }
            discover_tests(path, parser_type)?
        } else {
            if cli.verbose {
                eprintln!(
                    "Discovering tests in {} with {:?} parser (cache enabled)",
                    path.display(),
                    parser_type
                );
            }
            let cache_path = default_cache_path();
            let mut cache =
                DiscoveryCache::load(&cache_path).unwrap_or_else(|_| DiscoveryCache::new());
            // discover_tests_cached expects a ParserType now
            let tests = discover_tests_cached(path, &mut cache, parser_type)?;

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
        discovered_tests = discovered_tests
            .into_iter()
            .filter(|t| t.name.contains(filter) || t.id.contains(filter))
            .collect();
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

    // Run tests using appropriate executor based on configuration
    let optimizer = cli.optimizer.as_deref().unwrap_or("optimized");
    let workers = cli.workers.unwrap_or(0);

    let results = match optimizer {
        "standard" => {
            if cli.verbose {
                eprintln!("Using standard batch executor");
            }
            if workers == 1 {
                let executor = BatchExecutor::new();
                executor.execute_tests(discovered_tests)
            } else {
                let num_workers = if workers == 0 { None } else { Some(workers) };
                let executor = ParallelExecutor::new(num_workers, cli.verbose);
                executor.execute(discovered_tests)?
            }
        }
        "aggressive" | "optimized" | _ => {
            if cli.verbose {
                eprintln!(
                    "Using optimized executor with {} workers",
                    if workers == 0 {
                        "auto-detected".to_string()
                    } else {
                        workers.to_string()
                    }
                );
                if cli.persistent_workers {
                    eprintln!("Persistent worker pool: enabled (experimental)");
                }
            }
            // Default to optimized executor
            let num_workers = if workers == 0 { None } else { Some(workers) };
            let mut executor = OptimizedExecutor::new(num_workers, cli.verbose);

            // Enable coverage if requested
            if cli.coverage {
                let source_dirs = if cli.coverage_source.is_empty() {
                    // Default to current directory
                    vec![std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))]
                } else {
                    cli.coverage_source.clone()
                };
                executor = executor.with_coverage(source_dirs);
            }

            executor.execute(discovered_tests)?
        }
    };

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
