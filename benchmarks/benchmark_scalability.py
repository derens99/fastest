#!/usr/bin/env python3
"""
Benchmark test runner scalability from 10 to 10,000 tests.
Compares: Fastest, pytest, unittest, nose2
"""

import os
import shutil
import subprocess
import time
import json
import tempfile
from pathlib import Path
import matplotlib.pyplot as plt
import numpy as np

# Test runners to benchmark
TEST_RUNNERS = {
    "fastest": {
        "discovery": "fastest {path} --no-run --no-cache",
        "execution": "fastest {path} --no-cache",
        "name": "Fastest",
        "color": "#FF6B6B"
    },
    "pytest": {
        "discovery": "python3 -m pytest {path} --collect-only -q",
        "execution": "python3 -m pytest {path} -q --tb=no",
        "name": "pytest",
        "color": "#4ECDC4"
    },
    "unittest": {
        "discovery": "python3 -m unittest discover {path} -v",
        "execution": "python3 -m unittest discover {path}",
        "name": "unittest",
        "color": "#45B7D1"
    },
    "nose2": {
        "discovery": "python3 -m nose2 -v --collect-only {path}",
        "execution": "python3 -m nose2 {path}",
        "name": "nose2",
        "color": "#96CEB4"
    }
}

# Test scales to benchmark
TEST_SCALES = [10, 50, 100, 250, 500, 1000, 2500, 5000, 10000]


def generate_test_suite(num_tests, test_dir):
    """Generate a test suite with the specified number of tests."""
    os.makedirs(test_dir, exist_ok=True)
    
    # Calculate optimal distribution
    tests_per_file = min(50, max(10, num_tests // 20))
    num_files = (num_tests + tests_per_file - 1) // tests_per_file
    
    # Distribute tests across files and classes
    tests_created = 0
    
    for file_idx in range(num_files):
        if tests_created >= num_tests:
            break
            
        file_path = Path(test_dir) / f"test_module_{file_idx:03d}.py"
        
        with open(file_path, "w") as f:
            f.write('"""Auto-generated test module."""\n\n')
            f.write("import unittest\n\n")
            
            # Mix of class-based and function-based tests
            remaining = min(tests_per_file, num_tests - tests_created)
            
            # 80% class-based, 20% function-based (like real projects)
            class_tests = int(remaining * 0.8)
            func_tests = remaining - class_tests
            
                        # Write function-based tests
            for i in range(func_tests):
                f.write(f"def test_function_{file_idx}_{i}():\n")
                f.write(f'    """Test function {file_idx}_{i}"""\n')
                f.write("    assert True\n\n")
                tests_created += 1
            
            # Write class-based tests
            if class_tests > 0:
                tests_per_class = min(10, class_tests)
                num_classes = (class_tests + tests_per_class - 1) // tests_per_class
                
                for class_idx in range(num_classes):
                    f.write(f"class TestClass{file_idx}_{class_idx}(unittest.TestCase):\n")
                    f.write(f'    """Test class {file_idx}_{class_idx}"""\n\n')
                    
                    for method_idx in range(min(tests_per_class, class_tests - class_idx * tests_per_class)):
                        f.write(f"    def test_method_{method_idx}(self):\n")
                        f.write(f'        """Test method {method_idx}"""\n')
                        f.write("        self.assertTrue(True)\n\n")
                        tests_created += 1
    
    # Create __init__.py
    with open(Path(test_dir) / "__init__.py", "w") as f:
        f.write("# Test package\n")
    
    return tests_created


def time_command(cmd, timeout=300):
    """Execute command and return execution time."""
    try:
        start = time.time()
        result = subprocess.run(
            cmd,
            shell=True,
            capture_output=True,
            text=True,
            timeout=timeout
        )
        elapsed = time.time() - start
        
        # Check if command succeeded
        if result.returncode != 0 and "fastest" not in cmd:
            # For other runners, check stderr
            if "ImportError" in result.stderr or "ModuleNotFoundError" in result.stderr:
                return None  # Runner not installed
        
        return elapsed
    except subprocess.TimeoutExpired:
        return timeout
    except Exception as e:
        print(f"Error running {cmd}: {e}")
        return None


def check_runner_installed(runner_name):
    """Check if a test runner is installed."""
    check_cmds = {
        "fastest": "fastest version",
        "pytest": "python3 -m pytest --version",
        "unittest": "python3 -m unittest --version",
        "nose2": "python3 -m nose2 --version"
    }
    
    try:
        result = subprocess.run(
            check_cmds[runner_name],
            shell=True,
            capture_output=True,
            timeout=5
        )
        return result.returncode == 0
    except:
        return False


def run_benchmarks():
    """Run scalability benchmarks."""
    results = {
        runner: {
            "discovery": [],
            "execution": [],
            "scales": []
        }
        for runner in TEST_RUNNERS
    }
    
    # Check which runners are installed
    available_runners = {}
    print("Checking available test runners...")
    for runner in TEST_RUNNERS:
        if check_runner_installed(runner):
            available_runners[runner] = TEST_RUNNERS[runner]
            print(f"  ✓ {TEST_RUNNERS[runner]['name']} is available")
        else:
            print(f"  ✗ {TEST_RUNNERS[runner]['name']} is not installed")
    
    if len(available_runners) < 2:
        print("\nError: At least 2 test runners must be installed for comparison.")
        print("Install missing runners:")
        print("  pip install pytest nose2")
        return
    
    print(f"\nBenchmarking with {len(available_runners)} test runners...")
    
    # Create temporary directory for tests
    with tempfile.TemporaryDirectory() as temp_dir:
        for scale in TEST_SCALES:
            print(f"\n📊 Benchmarking with {scale} tests...")
            
            # Generate test suite
            test_dir = Path(temp_dir) / f"tests_{scale}"
            actual_tests = generate_test_suite(scale, test_dir)
            print(f"  Generated {actual_tests} tests")
            
            # Benchmark each runner
            for runner_name, runner_config in available_runners.items():
                print(f"  Testing {runner_config['name']}...")
                
                # Discovery benchmark
                discovery_cmd = runner_config["discovery"].format(path=test_dir)
                discovery_time = time_command(discovery_cmd)
                
                # Execution benchmark
                exec_cmd = runner_config["execution"].format(path=test_dir)
                exec_time = time_command(exec_cmd)
                
                if discovery_time is not None and exec_time is not None:
                    results[runner_name]["scales"].append(scale)
                    results[runner_name]["discovery"].append(discovery_time)
                    results[runner_name]["execution"].append(exec_time)
                    
                    print(f"    Discovery: {discovery_time:.3f}s")
                    print(f"    Execution: {exec_time:.3f}s")
                else:
                    print(f"    Failed to benchmark")
    
    return results, available_runners


def plot_results(results, available_runners):
    """Create visualization of benchmark results."""
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(15, 6))
    
    # Plot discovery times
    ax1.set_title("Test Discovery Time Scaling", fontsize=14, fontweight='bold')
    ax1.set_xlabel("Number of Tests", fontsize=12)
    ax1.set_ylabel("Time (seconds)", fontsize=12)
    ax1.set_xscale('log')
    ax1.set_yscale('log')
    ax1.grid(True, alpha=0.3)
    
    for runner_name, data in results.items():
        if data["scales"]:  # Only plot if we have data
            runner_config = available_runners.get(runner_name, TEST_RUNNERS.get(runner_name))
            ax1.plot(
                data["scales"],
                data["discovery"],
                marker='o',
                label=runner_config["name"],
                color=runner_config["color"],
                linewidth=2,
                markersize=8
            )
    
    ax1.legend(loc='upper left')
    
    # Plot execution times
    ax2.set_title("Test Execution Time Scaling", fontsize=14, fontweight='bold')
    ax2.set_xlabel("Number of Tests", fontsize=12)
    ax2.set_ylabel("Time (seconds)", fontsize=12)
    ax2.set_xscale('log')
    ax2.set_yscale('log')
    ax2.grid(True, alpha=0.3)
    
    for runner_name, data in results.items():
        if data["scales"]:  # Only plot if we have data
            runner_config = available_runners.get(runner_name, TEST_RUNNERS.get(runner_name))
            ax2.plot(
                data["scales"],
                data["execution"],
                marker='s',
                label=runner_config["name"],
                color=runner_config["color"],
                linewidth=2,
                markersize=8
            )
    
    ax2.legend(loc='upper left')
    
    plt.suptitle("Test Runner Performance Comparison (10-10,000 tests)", fontsize=16, fontweight='bold')
    plt.tight_layout()
    
    # Save plot
    output_path = Path(__file__).parent / "scalability_benchmark.png"
    plt.savefig(output_path, dpi=300, bbox_inches='tight')
    print(f"\n📈 Saved plot to {output_path}")
    
    # Also save as SVG for better quality
    svg_path = Path(__file__).parent / "scalability_benchmark.svg"
    plt.savefig(svg_path, format='svg', bbox_inches='tight')
    print(f"📈 Saved SVG to {svg_path}")


def generate_speedup_table(results, available_runners):
    """Generate a comparison table showing speedups."""
    if "fastest" not in results or not results["fastest"]["scales"]:
        print("\nCannot generate speedup table without Fastest results.")
        return
    
    print("\n📊 SPEEDUP COMPARISON (vs Fastest)")
    print("=" * 80)
    
    # Header
    header = f"{'Tests':>8} |"
    for runner in available_runners:
        if runner != "fastest":
            header += f" {available_runners[runner]['name']:>12} |"
    print(header)
    print("-" * len(header))
    
    # Data rows
    fastest_data = results["fastest"]
    for idx, scale in enumerate(fastest_data["scales"]):
        row = f"{scale:>8} |"
        
        for runner_name, runner_config in available_runners.items():
            if runner_name != "fastest" and scale in results[runner_name]["scales"]:
                runner_idx = results[runner_name]["scales"].index(scale)
                
                # Calculate speedup for discovery
                discovery_speedup = results[runner_name]["discovery"][runner_idx] / fastest_data["discovery"][idx]
                # Calculate speedup for execution
                exec_speedup = results[runner_name]["execution"][runner_idx] / fastest_data["execution"][idx]
                
                row += f" {discovery_speedup:>5.1f}x/{exec_speedup:>4.1f}x |"
        
        print(row)
    
    print("\nFormat: Discovery speedup / Execution speedup")
    print("Higher numbers = Fastest is faster")


def save_results_json(results, available_runners):
    """Save results as JSON for later analysis."""
    output = {
        "timestamp": time.strftime("%Y-%m-%d %H:%M:%S"),
        "test_scales": TEST_SCALES,
        "runners": {
            name: {
                "name": config["name"],
                "discovery_times": results[name]["discovery"],
                "execution_times": results[name]["execution"],
                "scales": results[name]["scales"]
            }
            for name, config in available_runners.items()
            if name in results
        }
    }
    
    json_path = Path(__file__).parent / "scalability_results.json"
    with open(json_path, "w") as f:
        json.dump(output, f, indent=2)
    
    print(f"\n💾 Saved results to {json_path}")


def main():
    """Run the scalability benchmark."""
    print("🚀 Test Runner Scalability Benchmark")
    print("=" * 50)
    print("Comparing test runners from 10 to 10,000 tests")
    print("=" * 50)
    
    # Ensure matplotlib doesn't show interactive plots
    plt.switch_backend('Agg')
    
    # Run benchmarks
    results, available_runners = run_benchmarks()
    
    if not any(results[r]["scales"] for r in results):
        print("\nNo benchmark results collected. Exiting.")
        return
    
    # Generate visualizations
    plot_results(results, available_runners)
    
    # Generate comparison table
    generate_speedup_table(results, available_runners)
    
    # Save results
    save_results_json(results, available_runners)
    
    print("\n✅ Benchmark complete!")


if __name__ == "__main__":
    main() 