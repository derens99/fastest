#!/usr/bin/env python3
"""Benchmark to measure discovery cache performance."""

import subprocess
import time
import tempfile
import os
import shutil
from pathlib import Path

def get_fastest_binary():
    """Get the path to the fastest binary."""
    # Check for release build first (faster)
    release_path = "./target/release/fastest"
    if os.path.exists(release_path):
        return release_path
    
    # Fall back to debug build
    debug_path = "./target/debug/fastest"
    if os.path.exists(debug_path):
        return debug_path
    
    # Try to find it anywhere in target
    for path in Path("target").rglob("fastest"):
        if path.is_file() and os.access(path, os.X_OK):
            return str(path)
    
    raise FileNotFoundError("Could not find fastest binary. Please run 'cargo build' first.")

def run_discovery(path, use_cache=True):
    """Run fastest discovery and return the time taken."""
    cmd = [get_fastest_binary()]
    if not use_cache:
        cmd.append("--no-cache")
    cmd.extend([str(path), "discover", "--format", "count"])
    
    start = time.time()
    result = subprocess.run(cmd, capture_output=True, text=True)
    elapsed = time.time() - start
    
    if result.returncode != 0:
        print(f"Error: {result.stderr}")
        return None
    
    test_count = int(result.stdout.strip())
    return elapsed, test_count

from contextlib import contextmanager

@contextmanager
def create_large_test_project(num_files=100, tests_per_file=10):
    """Create a large test project for benchmarking."""
    with tempfile.TemporaryDirectory() as tmpdir:
        test_dir = Path(tmpdir) / "tests"
        test_dir.mkdir()
        
        # Create test files
        for i in range(num_files):
            test_file = test_dir / f"test_module_{i}.py"
            content = f'"""Test module {i}"""\n\n'
            
            for j in range(tests_per_file):
                content += f"""
def test_function_{i}_{j}():
    \"\"\"Test function {i}_{j}\"\"\"
    assert True

"""
            test_file.write_text(content)
        
        yield tmpdir

def main():
    print("=== Fastest Discovery Cache Benchmark ===\n")
    
    # Define cache path at function level
    cache_path = Path.home() / ".cache" / "fastest" / "discovery_cache.json"
    
    # First, benchmark on the small test project
    print("Small project (10 tests):")
    
    # Check if test_project exists
    if os.path.exists("test_project"):
        # Clear cache first
        if cache_path.exists():
            cache_path.unlink()
        
        # First run (cold cache)
        result = run_discovery("test_project", use_cache=True)
        if result:
            time1, count1 = result
            print(f"  First run (cold cache):  {time1*1000:.1f}ms for {count1} tests")
        
            # Second run (warm cache)
            result2 = run_discovery("test_project", use_cache=True)
            if result2:
                time2, count2 = result2
                print(f"  Second run (warm cache): {time2*1000:.1f}ms for {count2} tests")
        
                # Run without cache
                result3 = run_discovery("test_project", use_cache=False)
                if result3:
                    time3, count3 = result3
                    print(f"  Without cache:           {time3*1000:.1f}ms for {count3} tests")
        
                    if time2 and time1:
                        speedup = time1 / time2
                        print(f"  Cache speedup:           {speedup:.1f}x faster")
    else:
        print("  Skipping - test_project directory not found")
    
    print("\nLarge project (1000 tests):")
    
    # Create and benchmark large project
    with create_large_test_project(num_files=100, tests_per_file=10) as test_path:
        # Clear cache
        if cache_path.exists():
            cache_path.unlink()
        
        # First run (cold cache)
        result = run_discovery(test_path, use_cache=True)
        if result:
            time1, count1 = result
            print(f"  First run (cold cache):  {time1*1000:.1f}ms for {count1} tests")
        
            # Second run (warm cache)
            result2 = run_discovery(test_path, use_cache=True)
            if result2:
                time2, count2 = result2
                print(f"  Second run (warm cache): {time2*1000:.1f}ms for {count2} tests")
        
                # Run without cache
                result3 = run_discovery(test_path, use_cache=False)
                if result3:
                    time3, count3 = result3
                    print(f"  Without cache:           {time3*1000:.1f}ms for {count3} tests")
        
                    if time2 and time1:
                        speedup = time1 / time2
                        print(f"  Cache speedup:           {speedup:.1f}x faster")
        
                    # Run pytest for comparison
                    print("\n  Pytest comparison:")
                    start = time.time()
                    result = subprocess.run(
                        ["python", "-m", "pytest", test_path, "--collect-only", "-q"],
                        capture_output=True,
                        text=True
                    )
                    pytest_time = time.time() - start
                    pytest_count = len([line for line in result.stdout.split('\n') if 'test_' in line])
                    print(f"    Pytest discovery:      {pytest_time*1000:.1f}ms for ~{pytest_count} tests")
        
                    if time2 and pytest_time:
                        print(f"    Fastest with cache is {pytest_time/time2:.1f}x faster than pytest")

if __name__ == "__main__":
    main() 