#!/usr/bin/env python3
"""Benchmark script to compare parser performance."""
import subprocess
import time
import json
import sys
from pathlib import Path

def run_fastest_with_parser(parser_type, test_file, runs=3):
    """Run fastest with specified parser and measure time."""
    times = []
    test_counts = []
    
    for i in range(runs):
        start = time.time()
        result = subprocess.run(
            ["target/release/fastest", test_file, "--parser", parser_type, "--no-cache", "-v"],
            capture_output=True,
            text=True
        )
        elapsed = time.time() - start
        times.append(elapsed)
        
        # Extract test count from output
        if "Found" in result.stdout:
            for line in result.stdout.split('\n'):
                if "Found" in line and "tests" in line:
                    count = int(line.split()[1])
                    test_counts.append(count)
                    break
        
        if result.returncode != 0:
            print(f"Error with {parser_type} parser: {result.stderr}")
    
    avg_time = sum(times) / len(times) if times else 0
    test_count = test_counts[0] if test_counts else 0
    
    return {
        "parser": parser_type,
        "avg_time": avg_time,
        "min_time": min(times) if times else 0,
        "max_time": max(times) if times else 0,
        "test_count": test_count,
        "times": times
    }

def run_pytest_discovery(test_file, runs=3):
    """Run pytest --collect-only and measure time."""
    times = []
    test_counts = []
    
    for i in range(runs):
        start = time.time()
        result = subprocess.run(
            ["python3", "-m", "pytest", test_file, "--collect-only", "-q"],
            capture_output=True,
            text=True
        )
        elapsed = time.time() - start
        times.append(elapsed)
        
        # Count tests from pytest output
        if "collected" in result.stdout:
            # Extract number from "collected X items"
            for line in result.stdout.split('\n'):
                if "collected" in line:
                    parts = line.split()
                    for i, part in enumerate(parts):
                        if part == "collected" and i+1 < len(parts):
                            try:
                                test_count = int(parts[i+1])
                                break
                            except:
                                pass
        else:
            test_count = len([line for line in result.stdout.split('\n') if line.strip() and '::' in line])
        test_counts.append(test_count)
    
    avg_time = sum(times) / len(times) if times else 0
    test_count = test_counts[0] if test_counts else 0
    
    return {
        "parser": "pytest",
        "avg_time": avg_time,
        "min_time": min(times) if times else 0,
        "max_time": max(times) if times else 0,
        "test_count": test_count,
        "times": times
    }

def main():
    """Run benchmarks and compare results."""
    test_file = "test_parser_comparison.py"
    
    print("=== Parser Performance Comparison ===")
    print(f"Test file: {test_file}")
    print(f"Runs per parser: 3")
    print()
    
    # Run benchmarks
    results = []
    
    # Test each parser
    for parser in ["tree-sitter", "ast", "regex"]:
        print(f"Testing {parser} parser...")
        result = run_fastest_with_parser(parser, test_file)
        results.append(result)
        print(f"  Avg time: {result['avg_time']:.3f}s, Tests found: {result['test_count']}")
    
    # Test pytest for comparison
    print("Testing pytest discovery...")
    pytest_result = run_pytest_discovery(test_file)
    results.append(pytest_result)
    print(f"  Avg time: {pytest_result['avg_time']:.3f}s, Tests found: {pytest_result['test_count']}")
    
    # Print results table
    print("\n=== Results Summary ===")
    print(f"{'Parser':<15} {'Avg Time':<10} {'Min Time':<10} {'Max Time':<10} {'Tests Found':<12} {'vs pytest':<10}")
    print("-" * 80)
    
    pytest_time = next(r['avg_time'] for r in results if r['parser'] == 'pytest')
    
    for result in results:
        speedup = pytest_time / result['avg_time'] if result['avg_time'] > 0 else 0
        speedup_str = f"{speedup:.2f}x" if result['parser'] != 'pytest' else "-"
        
        print(f"{result['parser']:<15} "
              f"{result['avg_time']:.3f}s     "
              f"{result['min_time']:.3f}s     "
              f"{result['max_time']:.3f}s     "
              f"{result['test_count']:<12} "
              f"{speedup_str:<10}")
    
    # Check accuracy
    print("\n=== Accuracy Check ===")
    test_counts = {r['parser']: r['test_count'] for r in results}
    expected_count = test_counts.get('pytest', 0)
    
    for parser, count in test_counts.items():
        if parser != 'pytest':
            if count == expected_count:
                print(f"✓ {parser}: Correct ({count} tests)")
            else:
                diff = count - expected_count
                print(f"✗ {parser}: {count} tests ({diff:+d} from pytest)")
    
    # Save results
    with open('parser_benchmark_results.json', 'w') as f:
        json.dump(results, f, indent=2)
    print(f"\nDetailed results saved to parser_benchmark_results.json")

if __name__ == "__main__":
    main()