#!/usr/bin/env python3
"""Benchmark Fastest vs pytest at different scales."""
import subprocess
import time
import statistics
import json
from pathlib import Path

def run_fastest(test_file, runs=3):
    """Run Fastest and measure performance."""
    times = []
    test_counts = []
    
    for _ in range(runs):
        start = time.time()
        result = subprocess.run(
            ["target/release/fastest", test_file, "--parser", "tree-sitter"],
            capture_output=True,
            text=True
        )
        elapsed = time.time() - start
        times.append(elapsed)
        
        # Extract test count
        if "Found" in result.stdout:
            for line in result.stdout.split('\n'):
                if "Found" in line and "tests" in line:
                    count = int(line.split()[1])
                    test_counts.append(count)
                    break
    
    return {
        "tool": "fastest",
        "file": test_file,
        "runs": runs,
        "times": times,
        "avg_time": statistics.mean(times),
        "min_time": min(times),
        "max_time": max(times),
        "stdev": statistics.stdev(times) if len(times) > 1 else 0,
        "test_count": test_counts[0] if test_counts else 0
    }

def run_pytest(test_file, runs=3):
    """Run pytest and measure performance."""
    times = []
    test_counts = []
    
    for _ in range(runs):
        start = time.time()
        result = subprocess.run(
            ["python3", "-m", "pytest", test_file, "-q"],
            capture_output=True,
            text=True
        )
        elapsed = time.time() - start
        times.append(elapsed)
        
        # Extract test count from pytest output
        output = result.stdout + result.stderr  # Check both stdout and stderr
        if "passed" in output:
            # Look for pattern like "12 passed"
            import re
            match = re.search(r'(\d+) passed', output)
            if match:
                count = int(match.group(1))
                test_counts.append(count)
    
    return {
        "tool": "pytest",
        "file": test_file,
        "runs": runs,
        "times": times,
        "avg_time": statistics.mean(times),
        "min_time": min(times),
        "max_time": max(times),
        "stdev": statistics.stdev(times) if len(times) > 1 else 0,
        "test_count": test_counts[0] if test_counts else 0
    }

def main():
    """Run scalability benchmarks."""
    test_files = [
        "scale_tests/test_10_tests.py",
        "scale_tests/test_100_tests.py", 
        "scale_tests/test_1000_tests.py",
        "scale_tests/test_10000_tests.py"
    ]
    
    print("=== Fastest vs pytest Scalability Benchmark ===")
    print("Running 3 iterations for each test size...\n")
    
    results = []
    
    for test_file in test_files:
        if not Path(test_file).exists():
            print(f"Skipping {test_file} (not found)")
            continue
        
        # Extract size from filename like "test_100_tests.py"
        size = test_file.split('/')[-1].split('_')[1]
        print(f"\nBenchmarking {size} tests:")
        print("-" * 50)
        
        # Run Fastest
        print("Running Fastest...")
        fastest_result = run_fastest(test_file)
        print(f"  Tests found: {fastest_result['test_count']}")
        print(f"  Avg time: {fastest_result['avg_time']:.3f}s (±{fastest_result['stdev']:.3f}s)")
        
        # Run pytest
        print("Running pytest...")
        pytest_result = run_pytest(test_file)
        print(f"  Tests run: {pytest_result['test_count']}")
        print(f"  Avg time: {pytest_result['avg_time']:.3f}s (±{pytest_result['stdev']:.3f}s)")
        
        # Calculate speedup
        speedup = pytest_result['avg_time'] / fastest_result['avg_time']
        print(f"\n  Speedup: Fastest is {speedup:.2f}x faster than pytest")
        
        results.append({
            "size": size,
            "fastest": fastest_result,
            "pytest": pytest_result,
            "speedup": speedup
        })
    
    # Summary table
    print("\n\n=== Summary ===")
    print(f"{'Test Count':<12} {'Fastest (s)':<12} {'pytest (s)':<12} {'Speedup':<10} {'Tests/sec (Fastest)':<20} {'Tests/sec (pytest)':<20}")
    print("-" * 100)
    
    for result in results:
        fastest = result['fastest']
        pytest = result['pytest']
        
        fastest_tps = fastest['test_count'] / fastest['avg_time'] if fastest['avg_time'] > 0 else 0
        pytest_tps = pytest['test_count'] / pytest['avg_time'] if pytest['avg_time'] > 0 else 0
        
        print(f"{fastest['test_count']:<12} "
              f"{fastest['avg_time']:<12.3f} "
              f"{pytest['avg_time']:<12.3f} "
              f"{result['speedup']:<10.2f} "
              f"{fastest_tps:<20.1f} "
              f"{pytest_tps:<20.1f}")
    
    # Performance scaling analysis
    print("\n=== Scaling Analysis ===")
    if len(results) > 1:
        # Check if speedup improves with scale
        speedups = [r['speedup'] for r in results]
        sizes = [int(r['size']) for r in results]
        
        print(f"Speedup trend: ", end="")
        if speedups[-1] > speedups[0]:
            print("✅ Fastest scales better (speedup increases with test count)")
        else:
            print("❌ Fastest scales worse (speedup decreases with test count)")
        
        # Calculate throughput scaling
        fastest_tps = [r['fastest']['test_count'] / r['fastest']['avg_time'] for r in results]
        pytest_tps = [r['pytest']['test_count'] / r['pytest']['avg_time'] for r in results]
        
        print(f"\nThroughput scaling:")
        print(f"  Fastest: {fastest_tps[0]:.1f} → {fastest_tps[-1]:.1f} tests/sec")
        print(f"  pytest:  {pytest_tps[0]:.1f} → {pytest_tps[-1]:.1f} tests/sec")
    
    # Save results
    with open('scale_benchmark_results.json', 'w') as f:
        json.dump(results, f, indent=2)
    
    print("\nDetailed results saved to scale_benchmark_results.json")

if __name__ == "__main__":
    main()