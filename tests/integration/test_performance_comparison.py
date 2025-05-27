#!/usr/bin/env python3
"""Performance comparison between Fastest and pytest."""
import subprocess
import time
import statistics
import json
from pathlib import Path

def run_fastest_full(test_file, parser="tree-sitter", runs=5):
    """Run full test execution with Fastest."""
    times = []
    passed = []
    failed = []
    
    for _ in range(runs):
        start = time.time()
        result = subprocess.run(
            ["target/release/fastest", test_file, "--parser", parser],
            capture_output=True,
            text=True
        )
        elapsed = time.time() - start
        times.append(elapsed)
        
        # Extract results
        if "passed" in result.stdout:
            for line in result.stdout.split('\n'):
                if "passed passed" in line:
                    parts = line.split()
                    p = int(parts[0])
                    f = int(parts[3]) if "failed" in line else 0
                    passed.append(p)
                    failed.append(f)
                    break
    
    return {
        "tool": f"fastest-{parser}",
        "times": times,
        "avg_time": statistics.mean(times),
        "min_time": min(times),
        "max_time": max(times),
        "stdev": statistics.stdev(times) if len(times) > 1 else 0,
        "passed": passed[0] if passed else 0,
        "failed": failed[0] if failed else 0
    }

def run_pytest_full(test_file, runs=5):
    """Run full test execution with pytest."""
    times = []
    passed = []
    failed = []
    
    for _ in range(runs):
        start = time.time()
        result = subprocess.run(
            ["python3", "-m", "pytest", test_file, "-v"],
            capture_output=True,
            text=True
        )
        elapsed = time.time() - start
        times.append(elapsed)
        
        # Extract results
        if "passed" in result.stdout:
            p = result.stdout.count(" PASSED")
            f = result.stdout.count(" FAILED")
            passed.append(p)
            failed.append(f)
    
    return {
        "tool": "pytest",
        "times": times,
        "avg_time": statistics.mean(times),
        "min_time": min(times),
        "max_time": max(times),
        "stdev": statistics.stdev(times) if len(times) > 1 else 0,
        "passed": passed[0] if passed else 0,
        "failed": failed[0] if failed else 0
    }

def main():
    """Run performance comparison."""
    test_files = [
        "test_parser_simple.py",
        "tests/test_basic.py",
        "tests/test_sample.py"
    ]
    
    print("=== Fastest vs pytest Performance Comparison ===")
    print("Running 5 iterations for each tool...\n")
    
    all_results = []
    
    for test_file in test_files:
        if not Path(test_file).exists():
            print(f"Skipping {test_file} (not found)")
            continue
            
        print(f"\nTesting {test_file}:")
        print("-" * 50)
        
        # Run tests
        results = []
        
        # Fastest with different parsers
        for parser in ["tree-sitter", "ast", "regex"]:
            print(f"Running fastest ({parser})...")
            result = run_fastest_full(test_file, parser)
            results.append(result)
            print(f"  Avg: {result['avg_time']:.3f}s (±{result['stdev']:.3f}s)")
        
        # pytest
        print("Running pytest...")
        result = run_pytest_full(test_file)
        results.append(result)
        print(f"  Avg: {result['avg_time']:.3f}s (±{result['stdev']:.3f}s)")
        
        # Calculate speedup
        pytest_time = result['avg_time']
        print(f"\nSpeedup vs pytest:")
        for r in results[:-1]:
            speedup = pytest_time / r['avg_time']
            print(f"  {r['tool']}: {speedup:.2f}x faster")
        
        all_results.append({
            "file": test_file,
            "results": results
        })
    
    # Summary table
    print("\n\n=== Summary Table ===")
    print(f"{'Tool':<20} {'Avg Time':<10} {'Min Time':<10} {'Max Time':<10} {'Std Dev':<10}")
    print("-" * 70)
    
    for file_results in all_results:
        print(f"\n{file_results['file']}:")
        for result in file_results['results']:
            print(f"{result['tool']:<20} "
                  f"{result['avg_time']:<10.3f} "
                  f"{result['min_time']:<10.3f} "
                  f"{result['max_time']:<10.3f} "
                  f"{result['stdev']:<10.3f}")
    
    # Save results
    with open('performance_comparison.json', 'w') as f:
        json.dump(all_results, f, indent=2)
    
    print("\nDetailed results saved to performance_comparison.json")

if __name__ == "__main__":
    main()