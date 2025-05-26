#!/usr/bin/env python3
"""Benchmark test discovery performance between pytest and fastest."""

import subprocess
import time
import statistics
import sys
import os
from pathlib import Path

# Add parent directory to Python path
sys.path.insert(0, str(Path(__file__).parent.parent))


def time_command(cmd, cwd=None):
    """Time how long a command takes to run."""
    start = time.perf_counter()
    result = subprocess.run(
        cmd,
        shell=True,
        capture_output=True,
        text=True,
        cwd=cwd
    )
    end = time.perf_counter()
    
    if result.returncode != 0:
        print(f"Command failed: {cmd}")
        print(f"stderr: {result.stderr}")
        return None
    
    return end - start


def benchmark_discovery(test_dir, num_runs=5):
    """Benchmark test discovery for both pytest and fastest."""
    print(f"\nBenchmarking discovery on: {test_dir}")
    print(f"Number of runs: {num_runs}")
    print("-" * 60)
    
    # Warm up Python
    subprocess.run([sys.executable, "-c", "pass"], capture_output=True)
    
    # Benchmark pytest
    pytest_times = []
    print("Running pytest discovery...")
    for i in range(num_runs):
        duration = time_command(
            f"{sys.executable} -m pytest --collect-only -q {test_dir}",
            cwd=test_dir
        )
        if duration is not None:
            pytest_times.append(duration)
            print(f"  Run {i+1}: {duration:.3f}s")
    
    # Benchmark fastest (regex parser)
    fastest_regex_times = []
    print("\nRunning fastest discovery (regex parser)...")
    for i in range(num_runs):
        duration = time_command(
            f"./target/release/fastest --parser regex {test_dir} discover --format count",
            cwd=Path(__file__).parent.parent
        )
        if duration is not None:
            fastest_regex_times.append(duration)
            print(f"  Run {i+1}: {duration:.3f}s")
    
    # Benchmark fastest (AST parser)
    fastest_ast_times = []
    print("\nRunning fastest discovery (AST parser)...")
    for i in range(num_runs):
        duration = time_command(
            f"./target/release/fastest --parser ast {test_dir} discover --format count",
            cwd=Path(__file__).parent.parent
        )
        if duration is not None:
            fastest_ast_times.append(duration)
            print(f"  Run {i+1}: {duration:.3f}s")
    
    # Calculate statistics
    print("\n" + "=" * 60)
    print("RESULTS:")
    print("=" * 60)
    
    if pytest_times:
        pytest_mean = statistics.mean(pytest_times)
        pytest_stdev = statistics.stdev(pytest_times) if len(pytest_times) > 1 else 0
        print(f"\npytest:")
        print(f"  Mean:   {pytest_mean:.3f}s ± {pytest_stdev:.3f}s")
        print(f"  Min:    {min(pytest_times):.3f}s")
        print(f"  Max:    {max(pytest_times):.3f}s")
    
    if fastest_regex_times:
        fastest_regex_mean = statistics.mean(fastest_regex_times)
        fastest_regex_stdev = statistics.stdev(fastest_regex_times) if len(fastest_regex_times) > 1 else 0
        print(f"\nfastest (regex):")
        print(f"  Mean:   {fastest_regex_mean:.3f}s ± {fastest_regex_stdev:.3f}s")
        print(f"  Min:    {min(fastest_regex_times):.3f}s")
        print(f"  Max:    {max(fastest_regex_times):.3f}s")
        
        if pytest_times:
            speedup = pytest_mean / fastest_regex_mean
            print(f"  Speedup: {speedup:.2f}x faster than pytest")
    
    if fastest_ast_times:
        fastest_ast_mean = statistics.mean(fastest_ast_times)
        fastest_ast_stdev = statistics.stdev(fastest_ast_times) if len(fastest_ast_times) > 1 else 0
        print(f"\nfastest (AST):")
        print(f"  Mean:   {fastest_ast_mean:.3f}s ± {fastest_ast_stdev:.3f}s")
        print(f"  Min:    {min(fastest_ast_times):.3f}s")
        print(f"  Max:    {max(fastest_ast_times):.3f}s")
        
        if pytest_times:
            speedup = pytest_mean / fastest_ast_mean
            print(f"  Speedup: {speedup:.2f}x faster than pytest")


def main():
    """Run benchmarks on different test directories."""
    # Ensure we have a release build
    print("Building fastest in release mode...")
    subprocess.run(
        ["cargo", "build", "--release", "-p", "fastest-cli"],
        cwd=Path(__file__).parent.parent,
        check=True
    )
    
    # Test directories to benchmark
    test_dirs = [
        "tests",  # Small test suite
        # Add more directories as needed
    ]
    
    for test_dir in test_dirs:
        if Path(test_dir).exists():
            benchmark_discovery(test_dir)
        else:
            print(f"Skipping {test_dir} - directory not found")


if __name__ == "__main__":
    main() 