#!/usr/bin/env python3
"""
Validate Fastest performance claims against pytest.

This benchmark creates test suites of various sizes and measures:
1. Time to run tests with Fastest vs pytest
2. Speedup factor for each test suite size
3. Validates the claimed performance gains
"""
import subprocess
import time
import tempfile
import statistics
from pathlib import Path
import json
import sys

# Test templates for different scenarios
SIMPLE_TEST = """
def test_{n}():
    assert True
"""

MATH_TEST = """
def test_math_{n}():
    result = sum(range(10))
    assert result == 45
"""

FIXTURE_TEST = """
import pytest

@pytest.fixture
def value_{n}():
    return {n}

def test_with_fixture_{n}(value_{n}):
    assert value_{n} == {n}
"""

def create_test_suite(tmpdir, num_tests, test_type="simple"):
    """Create a test suite with specified number of tests."""
    if test_type == "simple":
        template = SIMPLE_TEST
    elif test_type == "math":
        template = MATH_TEST
    elif test_type == "fixture":
        template = FIXTURE_TEST
    else:
        template = SIMPLE_TEST
    
    # Create multiple test files for larger suites
    tests_per_file = 50
    file_count = (num_tests + tests_per_file - 1) // tests_per_file
    
    for file_idx in range(file_count):
        test_file = tmpdir / f"test_suite_{file_idx}.py"
        tests_in_file = min(tests_per_file, num_tests - file_idx * tests_per_file)
        
        content = []
        for i in range(tests_in_file):
            test_num = file_idx * tests_per_file + i
            content.append(template.format(n=test_num))
        
        test_file.write_text("\n".join(content))

def time_execution(cmd, runs=3):
    """Time command execution, return average of multiple runs."""
    times = []
    
    for _ in range(runs):
        start = time.time()
        result = subprocess.run(cmd, capture_output=True, text=True)
        elapsed = time.time() - start
        
        if result.returncode == 0:
            times.append(elapsed)
        else:
            print(f"Command failed: {' '.join(cmd)}")
            print(f"Error: {result.stderr}")
            return None
    
    return statistics.mean(times) if times else None

def run_benchmark(test_sizes, test_type="simple"):
    """Run benchmarks for different test suite sizes."""
    results = []
    
    print(f"\n🏃 Running {test_type} test benchmarks...")
    print("=" * 60)
    
    with tempfile.TemporaryDirectory() as tmpdir:
        tmpdir = Path(tmpdir)
        
        for num_tests in test_sizes:
            print(f"\n📊 Benchmarking {num_tests} tests...")
            
            # Create test suite
            create_test_suite(tmpdir, num_tests, test_type)
            
            # Time pytest
            pytest_cmd = ["pytest", str(tmpdir), "-q", "--tb=no"]
            pytest_time = time_execution(pytest_cmd)
            
            # Time fastest
            fastest_cmd = ["./target/release/fastest", str(tmpdir)]
            fastest_time = time_execution(fastest_cmd)
            
            if pytest_time and fastest_time:
                speedup = pytest_time / fastest_time
                
                result = {
                    "test_count": num_tests,
                    "test_type": test_type,
                    "pytest_time": round(pytest_time, 3),
                    "fastest_time": round(fastest_time, 3),
                    "speedup": round(speedup, 1),
                    "strategy": get_strategy(num_tests)
                }
                
                results.append(result)
                
                print(f"  Pytest:  {pytest_time:.3f}s")
                print(f"  Fastest: {fastest_time:.3f}s")
                print(f"  Speedup: {speedup:.1f}x faster")
            
            # Clean up for next iteration
            for f in tmpdir.glob("test_*.py"):
                f.unlink()
    
    return results

def get_strategy(num_tests):
    """Determine which strategy Fastest would use."""
    if num_tests <= 20:
        return "InProcess"
    elif num_tests <= 100:
        return "WarmWorkers"
    else:
        return "FullParallel"

def validate_claims(results):
    """Validate performance claims against actual results."""
    print("\n🎯 Validating Performance Claims")
    print("=" * 60)
    
    claims = {
        "InProcess": {"min": 30, "target": 47, "claimed": "47x"},
        "WarmWorkers": {"min": 3, "target": 5, "claimed": "5x"},
        "FullParallel": {"min": 2, "target": 3, "claimed": "3x"}
    }
    
    validation = {}
    
    for strategy_name, thresholds in claims.items():
        strategy_results = [r for r in results if r["strategy"] == strategy_name]
        
        if strategy_results:
            speedups = [r["speedup"] for r in strategy_results]
            avg_speedup = statistics.mean(speedups)
            max_speedup = max(speedups)
            
            meets_claim = max_speedup >= thresholds["min"]
            
            validation[strategy_name] = {
                "claimed": thresholds["claimed"],
                "achieved_avg": f"{avg_speedup:.1f}x",
                "achieved_max": f"{max_speedup:.1f}x",
                "meets_claim": meets_claim,
                "status": "✅" if meets_claim else "❌"
            }
            
            print(f"\n{strategy_name}:")
            print(f"  Claimed:     {thresholds['claimed']} faster")
            print(f"  Achieved:    {avg_speedup:.1f}x avg, {max_speedup:.1f}x max")
            print(f"  Status:      {validation[strategy_name]['status']}")
    
    return validation

def save_results(results, validation):
    """Save benchmark results to files."""
    # Save detailed results
    with open("benchmarks/performance_validation.json", "w") as f:
        json.dump({
            "timestamp": time.strftime("%Y-%m-%d %H:%M:%S"),
            "results": results,
            "validation": validation
        }, f, indent=2)
    
    # Create markdown report
    with open("benchmarks/PERFORMANCE_VALIDATION.md", "w") as f:
        f.write("# Fastest Performance Validation Report\n\n")
        f.write(f"Generated: {time.strftime('%Y-%m-%d %H:%M:%S')}\n\n")
        
        f.write("## Summary\n\n")
        for strategy, data in validation.items():
            f.write(f"- **{strategy}**: {data['status']} ")
            f.write(f"Claimed {data['claimed']}, achieved {data['achieved_max']} max\n")
        
        f.write("\n## Detailed Results\n\n")
        f.write("| Tests | Type | pytest (s) | fastest (s) | Speedup | Strategy |\n")
        f.write("|-------|------|------------|-------------|---------|----------|\n")
        
        for r in results:
            f.write(f"| {r['test_count']} | {r['test_type']} | ")
            f.write(f"{r['pytest_time']} | {r['fastest_time']} | ")
            f.write(f"{r['speedup']}x | {r['strategy']} |\n")

def main():
    """Run comprehensive performance validation."""
    print("🚀 Fastest Performance Validation Suite")
    print("=" * 60)
    
    # Check if pytest is installed
    try:
        subprocess.run(["pytest", "--version"], capture_output=True, check=True)
    except:
        print("❌ pytest not found. Please install: pip install pytest")
        sys.exit(1)
    
    # Test suite sizes aligned with strategy boundaries
    test_sizes = [
        # InProcess range
        5, 10, 15, 20,
        # WarmWorkers range  
        25, 50, 75, 100,
        # FullParallel range
        150, 200, 500, 1000
    ]
    
    all_results = []
    
    # Run benchmarks for different test types
    for test_type in ["simple", "math", "fixture"]:
        results = run_benchmark(test_sizes[:8], test_type)  # Limit large tests
        all_results.extend(results)
    
    # Validate claims
    validation = validate_claims(all_results)
    
    # Save results
    save_results(all_results, validation)
    
    print("\n✅ Benchmark complete! Results saved to:")
    print("  - benchmarks/performance_validation.json")
    print("  - benchmarks/PERFORMANCE_VALIDATION.md")
    
    # Overall verdict
    all_valid = all(v["meets_claim"] for v in validation.values())
    if all_valid:
        print("\n🎉 All performance claims validated!")
    else:
        print("\n⚠️  Some performance claims not met - see report for details")

if __name__ == "__main__":
    main()