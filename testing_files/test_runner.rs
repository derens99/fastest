// Simple test runner to verify fastest core functionality
use fastest_core::{discover_tests_with_filtering};
use fastest_execution::UltraFastExecutor;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Testing Fastest core functionality...");
    
    // Discover tests in the testing_files directory
    let test_path = PathBuf::from("testing_files");
    let tests = discover_tests_with_filtering(&[test_path], true)?;
    
    println!("Found {} tests:", tests.len());
    for test in &tests {
        println!("  - {}", test.id);
    }
    
    if !tests.is_empty() {
        println!("\n⚡ Running tests with UltraFastExecutor...");
        let mut executor = UltraFastExecutor::new(true)?;
        let results = executor.execute(tests)?;
        
        let mut passed = 0;
        let mut failed = 0;
        
        for result in &results {
            if result.passed {
                passed += 1;
                println!("✅ {}", result.test_id);
            } else {
                failed += 1;
                println!("❌ {} - {}", result.test_id, result.error.as_deref().unwrap_or("Unknown error"));
            }
        }
        
        println!("\n📊 Results: {} passed, {} failed", passed, failed);
    }
    
    Ok(())
}