#!/usr/bin/env python3
"""
Benchmark fastest vs pytest on various test suites
"""
import os
import subprocess
import time
import sys
from pathlib import Path

def run_command(cmd, cwd=None):
    """Run command and return execution time and success status"""
    start = time.time()
    try:
        result = subprocess.run(cmd, shell=True, capture_output=True, text=True, cwd=cwd)
        duration = time.time() - start
        return duration, result.returncode == 0, result.stdout, result.stderr
    except Exception as e:
        return time.time() - start, False, "", str(e)

def count_tests(output):
    """Extract test count from output"""
    # For pytest
    if "passed" in output or "failed" in output:
        import re
        match = re.search(r'(\d+) passed|(\d+) failed', output)
        if match:
            return int(match.group(1) or match.group(2) or 0)
    
    # For fastest
    if "Found" in output and "tests" in output:
        import re
        match = re.search(r'Found (\d+) tests', output)
        if match:
            return int(match.group(1))
    
    return 0

def benchmark_suite(name, path, fastest_path="./target/release/fastest"):
    """Benchmark a test suite with both pytest and fastest"""
    print(f"\n{'='*60}")
    print(f"Benchmarking: {name}")
    print(f"Path: {path}")
    print(f"{'='*60}")
    
    results = {}
    
    # Check if path exists
    if not os.path.exists(path):
        print(f"❌ Path not found: {path}")
        return results
    
    # Run with pytest
    print("\n📊 Running with pytest...")
    pytest_cmd = f"python3 -m pytest {path} -q --tb=no"
    duration, success, stdout, stderr = run_command(pytest_cmd)
    results['pytest'] = {
        'duration': duration,
        'success': success,
        'tests': count_tests(stdout + stderr),
        'output': stdout[:500] if success else stderr[:500]
    }
    print(f"  Time: {duration:.3f}s")
    print(f"  Tests: {results['pytest']['tests']}")
    print(f"  Status: {'✅ Success' if success else '❌ Failed'}")
    
    # Run with fastest (different optimizers)
    for optimizer in ['simple', 'lightning', 'optimized']:
        print(f"\n📊 Running with fastest ({optimizer})...")
        fastest_cmd = f"{fastest_path} {path} --optimizer {optimizer}"
        duration, success, stdout, stderr = run_command(fastest_cmd)
        results[f'fastest_{optimizer}'] = {
            'duration': duration,
            'success': success,
            'tests': count_tests(stdout + stderr),
            'output': stdout[:500] if success else stderr[:500]
        }
        print(f"  Time: {duration:.3f}s")
        print(f"  Tests: {results[f'fastest_{optimizer}']['tests']}")
        print(f"  Status: {'✅ Success' if success else '❌ Failed'}")
        
        # Calculate speedup
        if results['pytest']['duration'] > 0:
            speedup = results['pytest']['duration'] / duration
            print(f"  Speedup vs pytest: {speedup:.2f}x")
    
    return results

def main():
    # Test suites to benchmark
    test_suites = [
        ("Simple Tests", "test_simple_fast.py"),
        ("Fastest Unit Tests", "tests/"),
        ("Scale Tests - 10", "scale_tests/test_10_tests.py"),
        ("Scale Tests - 100", "scale_tests/test_100_tests.py"),
    ]
    
    all_results = {}
    
    for name, path in test_suites:
        results = benchmark_suite(name, path)
        if results:
            all_results[name] = results
    
    # Print summary
    print(f"\n\n{'='*60}")
    print("📊 PERFORMANCE SUMMARY")
    print(f"{'='*60}")
    
    for suite_name, results in all_results.items():
        print(f"\n{suite_name}:")
        print(f"  {'Tool':<20} {'Time (s)':<10} {'Tests':<10} {'Status':<10}")
        print(f"  {'-'*50}")
        
        for tool, data in results.items():
            status = '✅' if data['success'] else '❌'
            print(f"  {tool:<20} {data['duration']:<10.3f} {data['tests']:<10} {status}")
        
        # Calculate best performer
        successful_results = [(k, v) for k, v in results.items() if v['success']]
        if successful_results and results.get('pytest', {}).get('duration', float('inf')) > 0:
            fastest_time = min(r['duration'] for _, r in successful_results)
            fastest_tool = [k for k, v in successful_results if v['duration'] == fastest_time][0]
            if fastest_tool != 'pytest' and results['pytest']['success']:
                speedup = results['pytest']['duration'] / fastest_time
                print(f"\n  🏆 Winner: {fastest_tool} ({speedup:.2f}x faster than pytest)")
            elif fastest_tool != 'pytest':
                print(f"\n  🏆 Winner: {fastest_tool} (pytest failed)")

if __name__ == "__main__":
    main()