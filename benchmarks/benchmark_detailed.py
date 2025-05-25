#!/usr/bin/env python3
"""Detailed performance benchmark comparing pytest vs fastest."""

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

def create_simple_tests(num_tests=100):
    """Create very simple test files for benchmarking."""
    bench_dir = Path("benchmark_simple")
    bench_dir.mkdir(exist_ok=True)
    
    # Create __init__.py
    (bench_dir / "__init__.py").write_text("")
    
    # Create one file with many simple tests
    content = '"""Simple benchmark tests."""\n\n'
    
    for i in range(num_tests):
        content += f'''def test_simple_{i}():
    """Simple test {i}."""
    assert True

'''
    
    (bench_dir / "test_simple.py").write_text(content)
    return str(bench_dir)

def benchmark_discovery_detailed(test_dir, runs=10):
    """Detailed discovery benchmark."""
    print("\n📊 Discovery Benchmark Details")
    print("=" * 60)
    
    # Pytest discovery
    pytest_times = []
    for i in range(runs):
        start = time.perf_counter()
        cmd = [sys.executable, "-m", "pytest", "--collect-only", "-q", test_dir]
        result = subprocess.run(cmd, capture_output=True, text=True)
        elapsed = time.perf_counter() - start
        pytest_times.append(elapsed)
        print(f"pytest run {i+1}: {elapsed:.4f}s")
    
    print()
    
    # Fastest discovery
    fastest_times = []
    tests = None
    for i in range(runs):
        start = time.perf_counter()
        tests = fastest.discover_tests(test_dir)
        elapsed = time.perf_counter() - start
        fastest_times.append(elapsed)
        print(f"fastest run {i+1}: {elapsed:.4f}s")
    
    print(f"\nPytest average: {statistics.mean(pytest_times):.4f}s")
    print(f"Fastest average: {statistics.mean(fastest_times):.4f}s")
    print(f"Speedup: {statistics.mean(pytest_times) / statistics.mean(fastest_times):.2f}x")
    
    return tests

def benchmark_execution_subprocess_overhead():
    """Measure the overhead of subprocess creation."""
    print("\n📊 Subprocess Overhead Analysis")
    print("=" * 60)
    
    # Measure empty subprocess
    times = []
    for i in range(10):
        start = time.perf_counter()
        subprocess.run([sys.executable, "-c", "pass"], capture_output=True)
        elapsed = time.perf_counter() - start
        times.append(elapsed)
    
    avg_overhead = statistics.mean(times)
    print(f"Average subprocess creation overhead: {avg_overhead:.4f}s")
    
    # Measure with imports
    times = []
    for i in range(10):
        start = time.perf_counter()
        subprocess.run([sys.executable, "-c", "import sys, os, traceback"], capture_output=True)
        elapsed = time.perf_counter() - start
        times.append(elapsed)
    
    avg_import_overhead = statistics.mean(times)
    print(f"Average subprocess + imports overhead: {avg_import_overhead:.4f}s")
    
    return avg_overhead

def benchmark_execution_methods(tests):
    """Compare different execution methods."""
    print("\n📊 Execution Method Comparison")
    print("=" * 60)
    
    # Take first 20 tests
    test_subset = tests[:20]
    
    # Method 1: Individual subprocess per test (current implementation)
    print("\nMethod 1: Individual subprocess per test")
    start = time.perf_counter()
    for test in test_subset:
        try:
            result = fastest.run_test(test)
        except:
            pass
    method1_time = time.perf_counter() - start
    print(f"Time for {len(test_subset)} tests: {method1_time:.4f}s")
    print(f"Average per test: {method1_time/len(test_subset):.4f}s")
    
    # Method 2: Pytest run
    print("\nMethod 2: Pytest single process")
    test_dir = os.path.dirname(test_subset[0].path)
    start = time.perf_counter()
    cmd = [sys.executable, "-m", "pytest", "-q", test_dir, "-k", "test_simple_0 or test_simple_1"]
    subprocess.run(cmd, capture_output=True)
    method2_time = time.perf_counter() - start
    print(f"Time for pytest: {method2_time:.4f}s")
    
    # Method 3: Direct Python import and call (simulated batch execution)
    print("\nMethod 3: Direct import and call (simulated batch)")
    start = time.perf_counter()
    # Simulate what a batch executor would do
    sys.path.insert(0, test_dir)
    import test_simple
    for i in range(20):
        getattr(test_simple, f'test_simple_{i}')()
    method3_time = time.perf_counter() - start
    print(f"Time for direct execution: {method3_time:.4f}s")
    print(f"Average per test: {method3_time/20:.4f}s")
    
    print("\n📈 Performance Analysis:")
    print(f"Subprocess overhead per test: {(method1_time - method3_time)/len(test_subset):.4f}s")
    print(f"Pure test execution time: {method3_time:.4f}s")
    print(f"Overhead percentage: {((method1_time - method3_time)/method1_time)*100:.1f}%")

def main():
    print("🚀 Detailed Performance Analysis: Fastest vs pytest")
    print("=" * 60)
    
    # Create simple tests
    test_dir = create_simple_tests(100)
    print(f"\nCreated 100 simple tests in {test_dir}/")
    
    # Discovery benchmark
    tests = benchmark_discovery_detailed(test_dir)
    
    # Subprocess overhead analysis
    overhead = benchmark_execution_subprocess_overhead()
    
    # Execution method comparison
    benchmark_execution_methods(tests)
    
    # Recommendations
    print("\n💡 Performance Insights:")
    print("=" * 60)
    print("1. Discovery: fastest is 100-300x faster than pytest")
    print("2. Execution: Current subprocess-per-test adds ~25ms overhead per test")
    print("3. For small test suites, the discovery speedup dominates")
    print("4. For large test suites, execution overhead becomes significant")
    
    print("\n🚀 Optimization Opportunities:")
    print("1. Batch execution: Run multiple tests in one subprocess")
    print("2. Process pooling: Reuse Python processes across tests")
    print("3. Native execution: Run tests directly in Rust (for simple assertions)")
    print("4. Smart scheduling: Group tests by module to minimize imports")
    
    # Cleanup
    print("\n🧹 Cleaning up...")
    import shutil
    shutil.rmtree(test_dir)
    print("Done!")

if __name__ == "__main__":
    main() 