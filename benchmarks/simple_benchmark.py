#!/usr/bin/env python3
"""Performance benchmark comparing pytest vs fastest."""

import time
import subprocess
import sys
import os
import statistics
from pathlib import Path

try:
    import fastest
except ImportError:
    print("❌ Error: fastest not installed. Please run 'maturin develop' first.")
    sys.exit(1)

def time_function(func, *args, **kwargs):
    """Time a function execution."""
    start = time.perf_counter()
    result = func(*args, **kwargs)
    elapsed = time.perf_counter() - start
    return result, elapsed

def benchmark_pytest_discovery(test_dir):
    """Benchmark pytest test discovery."""
    cmd = [sys.executable, "-m", "pytest", "--collect-only", "-q", test_dir]
    
    times = []
    for _ in range(5):  # Run 5 times for average
        start = time.perf_counter()
        result = subprocess.run(cmd, capture_output=True, text=True)
        elapsed = time.perf_counter() - start
        times.append(elapsed)
        
        # Parse test count from pytest output
        if "error" not in result.stdout.lower():
            # Count lines that look like test items
            test_count = len([line for line in result.stdout.splitlines() 
                            if line.strip() and not line.startswith(" ")])
        else:
            test_count = 0
    
    return {
        'avg_time': statistics.mean(times),
        'min_time': min(times),
        'max_time': max(times),
        'test_count': test_count
    }

def benchmark_fastest_discovery(test_dir):
    """Benchmark fastest test discovery."""
    times = []
    test_count = 0
    
    for _ in range(5):  # Run 5 times for average
        tests, elapsed = time_function(fastest.discover_tests, test_dir)
        times.append(elapsed)
        test_count = len(tests)
    
    return {
        'avg_time': statistics.mean(times),
        'min_time': min(times),
        'max_time': max(times),
        'test_count': test_count,
        'tests': tests  # Keep for execution benchmark
    }

def benchmark_pytest_execution(test_dir, num_tests=5):
    """Benchmark pytest test execution."""
    cmd = [sys.executable, "-m", "pytest", "-v", test_dir]
    
    start = time.perf_counter()
    result = subprocess.run(cmd, capture_output=True, text=True)
    elapsed = time.perf_counter() - start
    
    # Parse results
    passed = result.stdout.count(" PASSED")
    failed = result.stdout.count(" FAILED")
    
    return {
        'total_time': elapsed,
        'passed': passed,
        'failed': failed,
        'output': result.stdout
    }

def benchmark_fastest_execution(tests, num_tests=5):
    """Benchmark fastest test execution."""
    results = []
    start_total = time.perf_counter()
    
    # Run only the first num_tests for fair comparison
    for test in tests[:num_tests]:
        try:
            result = fastest.run_test(test)
            results.append(result)
        except Exception as e:
            print(f"Error running test: {e}")
    
    total_elapsed = time.perf_counter() - start_total
    
    passed = sum(1 for r in results if r.passed)
    failed = sum(1 for r in results if not r.passed)
    
    return {
        'total_time': total_elapsed,
        'passed': passed,
        'failed': failed,
        'results': results
    }

def create_benchmark_tests(num_files=10, tests_per_file=10):
    """Create a set of benchmark test files."""
    bench_dir = Path("benchmark_tests")
    bench_dir.mkdir(exist_ok=True)
    
    # Create __init__.py
    (bench_dir / "__init__.py").write_text("")
    
    # Create test files
    for i in range(num_files):
        content = f'''"""Benchmark test file {i}."""

def test_simple_{i}_0():
    """Simple assertion test."""
    assert 1 + 1 == 2

'''
        
        # Add more test functions
        for j in range(1, tests_per_file):
            content += f'''def test_calculation_{i}_{j}():
    """Test with some calculations."""
    result = sum(range({j * 10}))
    assert result == {sum(range(j * 10))}

'''
        
        # Add a few that will fail
        if i % 3 == 0:
            content += f'''def test_failing_{i}():
    """This test should fail."""
    assert False, "Intentional failure for benchmarking"

'''
        
        # Add an async test
        if i % 2 == 0:
            content += f'''async def test_async_{i}():
    """Async test function."""
    import asyncio
    await asyncio.sleep(0.001)
    assert True

'''
        
        (bench_dir / f"test_benchmark_{i}.py").write_text(content)
    
    return str(bench_dir)

def print_comparison_table(title, pytest_stats, fastest_stats):
    """Print a nice comparison table."""
    print(f"\n{title}")
    print("=" * 60)
    print(f"{'Metric':<20} {'pytest':<20} {'fastest':<20}")
    print("-" * 60)
    
    for key in ['avg_time', 'min_time', 'max_time']:
        if key in pytest_stats and key in fastest_stats:
            speedup = pytest_stats[key] / fastest_stats[key]
            print(f"{key:<20} {pytest_stats[key]:<20.4f} {fastest_stats[key]:<20.4f}")
            print(f"{'Speedup':<20} {'':<20} {speedup:.2f}x faster")
            print()
    
    if 'test_count' in pytest_stats:
        print(f"{'Tests found':<20} {pytest_stats['test_count']:<20} {fastest_stats['test_count']:<20}")

def main():
    print("🚀 Fastest vs pytest Performance Benchmark")
    print("=" * 60)
    
    # Create benchmark tests
    print("\n📝 Creating benchmark test suite...")
    bench_dir = create_benchmark_tests(num_files=10, tests_per_file=10)
    print(f"Created {10 * 10} tests in {bench_dir}/")
    
    # Benchmark discovery
    print("\n🔍 Benchmarking test discovery...")
    print("Running each discovery method 5 times...")
    
    pytest_discovery = benchmark_pytest_discovery(bench_dir)
    fastest_discovery = benchmark_fastest_discovery(bench_dir)
    
    print_comparison_table("Test Discovery Performance", pytest_discovery, fastest_discovery)
    
    # Benchmark execution
    print("\n🧪 Benchmarking test execution...")
    print("Running subset of tests...")
    
    # Run pytest
    print("\nRunning pytest...")
    pytest_exec = benchmark_pytest_execution(bench_dir, num_tests=20)
    
    # Run fastest
    print("Running fastest...")
    fastest_exec = benchmark_fastest_execution(fastest_discovery['tests'], num_tests=20)
    
    print("\n📊 Test Execution Results")
    print("=" * 60)
    print(f"{'Metric':<20} {'pytest':<20} {'fastest':<20}")
    print("-" * 60)
    print(f"{'Total time':<20} {pytest_exec['total_time']:<20.4f} {fastest_exec['total_time']:<20.4f}")
    print(f"{'Tests passed':<20} {pytest_exec['passed']:<20} {fastest_exec['passed']:<20}")
    print(f"{'Tests failed':<20} {pytest_exec['failed']:<20} {fastest_exec['failed']:<20}")
    
    if pytest_exec['total_time'] > 0 and fastest_exec['total_time'] > 0:
        speedup = pytest_exec['total_time'] / fastest_exec['total_time']
        print(f"\n{'Execution speedup':<20} {'':<20} {speedup:.2f}x faster")
    
    # Summary
    print("\n🎯 Summary")
    print("=" * 60)
    
    discovery_speedup = pytest_discovery['avg_time'] / fastest_discovery['avg_time']
    print(f"Discovery: fastest is {discovery_speedup:.2f}x faster than pytest")
    
    if pytest_exec['total_time'] > 0 and fastest_exec['total_time'] > 0:
        exec_speedup = pytest_exec['total_time'] / fastest_exec['total_time']
        print(f"Execution: fastest is {exec_speedup:.2f}x faster than pytest")
    
    # Note about limitations
    print("\n⚠️  Note: This is a basic benchmark. Pytest includes many features")
    print("that fastest doesn't support yet (fixtures, markers, plugins, etc.)")
    print("The performance comparison is for basic test discovery and execution only.")
    
    # Cleanup
    print("\n🧹 Cleaning up benchmark tests...")
    import shutil
    shutil.rmtree(bench_dir)
    print("Done!")

if __name__ == "__main__":
    main() 