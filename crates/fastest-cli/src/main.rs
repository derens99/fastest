use clap::{Parser, Subcommand};
use colored::*;
use fastest_core::{
    discover_tests, discover_tests_cached, discover_tests_ast, 
    BatchExecutor, DiscoveryCache, default_cache_path, ParallelExecutor,
    filter_by_markers
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
    
    /// Path to discover tests from
    #[arg(default_value = ".")]
    path: PathBuf,
    
    /// Filter tests by pattern
    #[arg(short = 'k', long)]
    filter: Option<String>,
    
    /// Filter tests by marker expression (e.g., "not slow", "skip or xfail")
    #[arg(short = 'm', long = "markers")]
    markers: Option<String>,
    
    /// Number of parallel workers (0 = auto-detect CPUs, 1 = sequential)
    #[arg(short = 'n', long, default_value = "1")]
    workers: usize,
    
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
    #[arg(long = "parser", default_value = "regex")]
    parser: String,
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

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    
    match &cli.command {
        Some(Commands::Discover { format }) => {
            discover_command(&cli, format)
        }
        Some(Commands::Version) => {
            version_command()
        }
        Some(Commands::Run { show_output }) => {
            run_command(&cli, *show_output)
        }
        None => {
            run_command(&cli, false)
        }
    }
}

fn discover_command(cli: &Cli, format: &str) -> anyhow::Result<()> {
    let start = Instant::now();
    
    let tests = match cli.parser.as_str() {
        "ast" => {
            if cli.verbose {
                eprintln!("Using AST parser for test discovery");
            }
            discover_tests_ast(&cli.path)?
        }
        _ => {
            if cli.verbose && cli.parser != "regex" {
                eprintln!("Unknown parser '{}', using regex parser", cli.parser);
            }
            if cli.no_cache {
                discover_tests(&cli.path)?
            } else {
                let cache_path = default_cache_path();
                let mut cache = DiscoveryCache::load(&cache_path).unwrap_or_else(|_| DiscoveryCache::new());
                let tests = discover_tests_cached(&cli.path, &mut cache)?;
                
                // Save cache
                if let Err(e) = cache.save(&cache_path) {
                    eprintln!("Warning: Failed to save discovery cache: {}", e);
                }
                
                tests
            }
        }
    };
    
    let duration = start.elapsed();
    
    // Apply marker filter first if provided
    let tests = if let Some(markers) = &cli.markers {
        filter_by_markers(tests, markers)?
    } else {
        tests
    };
    
    // Apply text filter if provided
    let filtered_tests: Vec<_> = if let Some(filter) = &cli.filter {
        tests.into_iter()
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
            println!("Found {} tests in {:.3}s\n", 
                filtered_tests.len(), 
                duration.as_secs_f64()
            );
            
            if let Some(markers) = &cli.markers {
                println!("  {} {}\n", 
                    "Marker filter:".dimmed(),
                    markers.yellow()
                );
            }
            
            for test in &filtered_tests {
                println!("  {} {}", 
                    "●".green(),
                    test.id
                );
                if cli.verbose {
                    println!("    {} {}", "Path:".dimmed(), test.path.display());
                    println!("    {} {}", "Line:".dimmed(), test.line_number);
                    if test.is_async {
                        println!("    {} {}", "Type:".dimmed(), "async".yellow());
                    }
                    if !test.decorators.is_empty() {
                        println!("    {} {}", "Decorators:".dimmed(), test.decorators.join(", "));
                    }
                }
            }
        }
    }
    
    Ok(())
}

fn run_command(cli: &Cli, show_output: bool) -> anyhow::Result<()> {
    let start = Instant::now();
    
    // Discover tests
    println!("{}", "Discovering tests...".dimmed());
    
    let tests = match cli.parser.as_str() {
        "ast" => {
            if cli.verbose {
                eprintln!("Using AST parser for test discovery");
            }
            discover_tests_ast(&cli.path)?
        }
        _ => {
            if cli.verbose && cli.parser != "regex" {
                eprintln!("Unknown parser '{}', using regex parser", cli.parser);
            }
            if cli.no_cache {
                discover_tests(&cli.path)?
            } else {
                let cache_path = default_cache_path();
                let mut cache = DiscoveryCache::load(&cache_path).unwrap_or_else(|_| DiscoveryCache::new());
                let tests = discover_tests_cached(&cli.path, &mut cache)?;
                
                // Save cache
                if let Err(e) = cache.save(&cache_path) {
                    eprintln!("Warning: Failed to save discovery cache: {}", e);
                }
                
                tests
            }
        }
    };
    
    // Apply marker filter first if provided
    let tests = if let Some(markers) = &cli.markers {
        if cli.verbose {
            eprintln!("Applying marker filter: {}", markers);
        }
        filter_by_markers(tests, markers)?
    } else {
        tests
    };
    
    // Apply text filter
    let filtered_tests: Vec<_> = if let Some(filter) = &cli.filter {
        tests.into_iter()
            .filter(|t| t.name.contains(filter) || t.id.contains(filter))
            .collect()
    } else {
        tests
    };
    
    if filtered_tests.is_empty() {
        println!("{}", "No tests found!".yellow());
        return Ok(());
    }
    
    println!("Found {} tests\n", filtered_tests.len());
    
    // Create progress bar
    let pb = ProgressBar::new(filtered_tests.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("#>-")
    );
    
    // Run tests using appropriate executor
    let results = if cli.workers != 1 {
        // Use parallel executor (0 = auto-detect, >1 = specific count)
        let num_workers = if cli.workers == 0 { None } else { Some(cli.workers) };
        let executor = ParallelExecutor::new(num_workers, cli.verbose);
        executor.execute(filtered_tests)?
    } else {
        // Use batch executor (sequential)
        let executor = BatchExecutor::new();
        executor.execute_tests(filtered_tests)
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
        println!("{} {} passed in {:.2}s", 
            "✓".green().bold(),
            format!("{} tests", passed).green().bold(),
            duration.as_secs_f64()
        );
    } else {
        println!("{} {} passed, {} {} failed in {:.2}s",
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