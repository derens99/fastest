#!/usr/bin/env python3
"""Compare performance between standard and optimized executors."""

import subprocess
import time
import sys
import os

def run_benchmark(optimizer, test_path="tests/"):
    """Run fastest with specified optimizer and measure time."""
    start = time.time()
    cmd = [
        "./target/release/fastest",
        test_path,
        "--parser", "ast",
        "--optimizer", optimizer
    ]
    
    result = subprocess.run(cmd, capture_output=True, text=True)
    duration = time.time() - start
    
    # Extract test counts from output
    passed = 0
    failed = 0
    for line in result.stdout.split('\n'):
        if "passed passed" in line:
            parts = line.split()
            if parts:
                try:
                    passed = int(parts[0])
                    failed = int(parts[2]) if len(parts) > 2 else 0
                except:
                    pass
    
    return {
        'duration': duration,
        'passed': passed,
        'failed': failed,
        'returncode': result.returncode,
        'output': result.stdout,
        'error': result.stderr
    }

def main():
    print("🚀 Fastest Optimizer Comparison\n")
    print("=" * 60)
    
    # Check if binary exists
    if not os.path.exists("./target/release/fastest"):
        print("❌ Error: fastest binary not found!")
        print("   Please run: cargo build --release")
        sys.exit(1)
    
    # Test path
    test_path = sys.argv[1] if len(sys.argv) > 1 else "tests/"
    
    # Run with standard optimizer
    print(f"Running tests with STANDARD optimizer...")
    standard = run_benchmark("standard", test_path)
    
    # Run with optimized optimizer
    print(f"Running tests with OPTIMIZED optimizer...")
    optimized = run_benchmark("optimized", test_path)
    
    # Print results
    print("\n" + "=" * 60)
    print("📊 RESULTS\n")
    
    print(f"Test path: {test_path}")
    print(f"Total tests: {standard['passed'] + standard['failed']}")
    print()
    
    print("⏱️  Execution Time:")
    print(f"  Standard:  {standard['duration']:.3f}s")
    print(f"  Optimized: {optimized['duration']:.3f}s")
    
    if standard['duration'] > 0:
        speedup = standard['duration'] / optimized['duration']
        print(f"  Speedup:   {speedup:.2f}x {'🚀' if speedup > 1.1 else ''}")
    
    print("\n✅ Test Results:")
    print(f"  Standard:  {standard['passed']} passed, {standard['failed']} failed")
    print(f"  Optimized: {optimized['passed']} passed, {optimized['failed']} failed")
    
    # Show errors if any
    if standard['error'] or optimized['error']:
        print("\n⚠️  Errors:")
        if standard['error']:
            print(f"  Standard: {standard['error']}")
        if optimized['error']:
            print(f"  Optimized: {optimized['error']}")

if __name__ == "__main__":
    main() 