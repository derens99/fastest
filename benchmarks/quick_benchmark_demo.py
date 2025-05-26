#!/usr/bin/env python3
"""
Quick benchmark demo comparing test runners.
Smaller scales for faster execution.
"""

import subprocess
import time
import tempfile
from pathlib import Path

# Test scales for quick demo
TEST_SCALES = [10, 50, 100, 500, 1000, 10000]

def generate_simple_tests(num_tests, test_dir):
    """Generate a simple test suite."""
    Path(test_dir).mkdir(parents=True, exist_ok=True)
    
    # Create test files
    tests_per_file = 25
    num_files = (num_tests + tests_per_file - 1) // tests_per_file
    
    for i in range(num_files):
        test_file = Path(test_dir) / f"test_{i:03d}.py"
        with open(test_file, "w") as f:
            f.write("import unittest\n\n")
            
            # Add some tests
            tests_in_file = min(tests_per_file, num_tests - i * tests_per_file)
            
            # Add a test class
            f.write(f"class TestSuite{i}(unittest.TestCase):\n")
            for j in range(tests_in_file):
                f.write(f"    def test_{j:03d}(self):\n")
                f.write(f"        self.assertEqual(1, 1)\n\n")
    
    # Create __init__.py
    (Path(test_dir) / "__init__.py").touch()


def time_discovery(runner, path):
    """Time test discovery."""
    commands = {
        "fastest": f"fastest {path} --no-run --no-cache",
        "pytest": f"python -m pytest {path} --collect-only -q",
        "unittest": f"python -m unittest discover {path} -v",
        "nose2": f"python -m nose2 -v --collect-only {path}"
    }
    
    if runner not in commands:
        return None
        
    try:
        start = time.time()
        result = subprocess.run(
            commands[runner],
            shell=True,
            capture_output=True,
            text=True,
            timeout=60
        )
        elapsed = time.time() - start
        
        # Check if successful
        if result.returncode != 0 and "No module named" in result.stderr:
            return None
            
        return elapsed
    except:
        return None


def main():
    """Run quick benchmark demo."""
    print("🚀 Quick Test Runner Benchmark Demo")
    print("=" * 50)
    
    # Check available runners
    runners = ["fastest", "pytest", "unittest", "nose2"]
    available = []
    
    print("Checking installed test runners...")
    for runner in runners:
        test_cmd = {
            "fastest": "fastest version",
            "pytest": "python -m pytest --version",
            "unittest": "python -m unittest --version",
            "nose2": "python -m nose2 --version"
        }[runner]
        
        result = subprocess.run(test_cmd, shell=True, capture_output=True)
        if result.returncode == 0:
            available.append(runner)
            print(f"  ✓ {runner}")
        else:
            print(f"  ✗ {runner} (not installed)")
    
    if len(available) < 2:
        print("\nNeed at least 2 test runners installed!")
        return
    
    print(f"\nRunning benchmark with: {', '.join(available)}")
    print("-" * 50)
    
    # Results storage
    results = {runner: [] for runner in available}
    
    # Run benchmarks
    with tempfile.TemporaryDirectory() as temp_dir:
        for scale in TEST_SCALES:
            print(f"\n📊 Testing with {scale} tests:")
            
            # Generate tests
            test_path = Path(temp_dir) / f"tests_{scale}"
            generate_simple_tests(scale, test_path)
            
            # Benchmark each runner
            for runner in available:
                discovery_time = time_discovery(runner, test_path)
                if discovery_time:
                    results[runner].append(discovery_time)
                    print(f"  {runner:8} - {discovery_time:6.3f}s")
                else:
                    print(f"  {runner:8} - failed")
    
    # Print summary
    print("\n" + "=" * 50)
    print("📈 DISCOVERY TIME SUMMARY (seconds)")
    print("-" * 50)
    
    # Header
    header = f"{'Tests':>6} |"
    for runner in available:
        if results[runner]:
            header += f" {runner:>8} |"
    print(header)
    print("-" * len(header))
    
    # Data rows
    for i, scale in enumerate(TEST_SCALES):
        row = f"{scale:>6} |"
        for runner in available:
            if i < len(results[runner]):
                row += f" {results[runner][i]:>8.3f} |"
        print(row)
    
    # Calculate speedups vs Fastest
    if "fastest" in available and results["fastest"]:
        print("\n📊 SPEEDUP vs FASTEST")
        print("-" * 50)
        
        for runner in available:
            if runner != "fastest" and results[runner]:
                avg_speedup = sum(
                    results[runner][i] / results["fastest"][i] 
                    for i in range(min(len(results[runner]), len(results["fastest"])))
                ) / min(len(results[runner]), len(results["fastest"]))
                
                print(f"{runner:>8}: {avg_speedup:>5.1f}x slower (Fastest is {avg_speedup:.1f}x faster)")
    
    print("\n✅ Demo complete!")


if __name__ == "__main__":
    main() 