use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "fastest", version, about = "Blazing-fast Python test runner")]
struct Cli {
    /// Test path(s) to discover
    #[arg(default_value = ".")]
    paths: Vec<String>,
}

fn main() -> anyhow::Result<()> {
    let _cli = Cli::parse();
    println!("fastest v{}", fastest_core::VERSION);
    Ok(())
}
