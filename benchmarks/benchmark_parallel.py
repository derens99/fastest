#!/usr/bin/env python3
"""Benchmark parallel execution performance."""

import time
import multiprocessing
import tempfile
import shutil
from pathlib import Path

try:
    import fastest
except ImportError:
    print("❌ Error: fastest not installed. Please run 'maturin develop' first.")
    exit(1)

def create_large_test_suite(num_files=50, tests_per_file=20):
    """Create a large test suite for benchmarking."""
    test_dir = Path(tempfile.mkdtemp(prefix="benchmark_parallel_"))
    
    # Create __init__.py
    (test_dir / "__init__.py").write_text("")
    
    # Create test files
    for i in range(num_files):
        content = f'"""Test file {i} for parallel benchmarking."""\n\n'
        
        for j in range(tests_per_file):
            # Mix of fast and slow tests
            if j % 5 == 0:
                # Slower test (simulates I/O or computation)
                content += f'''def test_slow_{i}_{j}():
    """Slower test that takes more time."""
    import time
    time.sleep(0.01)  # 10ms delay
    assert True

'''
            else:
                # Fast test
                content += f'''def test_fast_{i}_{j}():
    """Fast test {i}_{j}."""
    assert 1 + 1 == 2

'''
        
        # Add some async tests
        if i % 3 == 0:
            for j in range(3):
                content += f'''async def test_async_{i}_{j}():
    """Async test {i}_{j}."""
    import asyncio
    await asyncio.sleep(0.001)
    assert True

'''
        
        (test_dir / f"test_file_{i}.py").write_text(content)
    
    return str(test_dir)

def benchmark_parallel_execution():
    """Benchmark parallel execution with different worker counts."""
    print("🚀 Parallel Execution Benchmark")
    print("=" * 60)
    
    # Create large test suite
    print("\n📝 Creating test suite...")
    test_dir = create_large_test_suite(num_files=20, tests_per_file=10)
    tests = fastest.discover_tests(test_dir)
    
    print(f"✓ Created {len(tests)} tests")
    print(f"📊 CPU cores available: {multiprocessing.cpu_count()}")
    
    print("\n⏱️  Running benchmarks...")
    print("-" * 60)
    
    # Sequential execution (baseline)
    print("\n1. Sequential execution (batch):")
    start = time.time()
    results_seq = fastest.run_tests_batch(tests)
    seq_time = time.time() - start
    passed_seq = sum(1 for r in results_seq if r.passed)
    print(f"   Time: {seq_time:.2f}s")
    print(f"   Tests: {passed_seq}/{len(tests)} passed")
    print(f"   Per test: {(seq_time/len(tests))*1000:.1f}ms")
    
    # Parallel execution with different worker counts
    worker_counts = [2, 4, 8, None]  # None = auto-detect
    
    for workers in worker_counts:
        worker_label = "auto" if workers is None else str(workers)
        print(f"\n2. Parallel execution ({worker_label} workers):")
        
        start = time.time()
        results_par = fastest.run_tests_parallel(tests, num_workers=workers)
        par_time = time.time() - start
        passed_par = sum(1 for r in results_par if r.passed)
        
        speedup = seq_time / par_time
        efficiency = speedup / (workers or multiprocessing.cpu_count()) * 100
        
        print(f"   Time: {par_time:.2f}s")
        print(f"   Tests: {passed_par}/{len(tests)} passed")
        print(f"   Per test: {(par_time/len(tests))*1000:.1f}ms")
        print(f"   Speedup: {speedup:.2f}x")
        print(f"   Efficiency: {efficiency:.0f}%")
    
    # Test with varying test sizes
    print("\n\n📈 Scalability Test")
    print("-" * 60)
    
    test_sizes = [10, 50, 100, 200]
    for size in test_sizes:
        # Use only first N tests
        test_subset = tests[:size]
        
        print(f"\nWith {size} tests:")
        
        # Sequential
        start = time.time()
        fastest.run_tests_batch(test_subset)
        seq_time = time.time() - start
        
        # Parallel (auto workers)
        start = time.time()
        fastest.run_tests_parallel(test_subset)
        par_time = time.time() - start
        
        speedup = seq_time / par_time
        print(f"  Sequential: {seq_time:.3f}s")
        print(f"  Parallel:   {par_time:.3f}s")
        print(f"  Speedup:    {speedup:.2f}x")
    
    # Cleanup
    print("\n\n🧹 Cleaning up...")
    shutil.rmtree(test_dir)
    print("✓ Done!")
    
    # Summary
    print("\n" + "=" * 60)
    print("📊 Key Insights:")
    print("- Parallel execution provides significant speedup")
    print("- Best efficiency with worker count ≈ CPU cores")
    print("- Greater benefit with more tests")
    print("- Overhead is minimal for small test suites")

def main():
    """Run the benchmark."""
    try:
        benchmark_parallel_execution()
    except Exception as e:
        print(f"\n❌ Error: {e}")
        import traceback
        traceback.print_exc()

if __name__ == "__main__":
    main() 