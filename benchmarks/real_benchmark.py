#!/usr/bin/env python3
"""
Real benchmarking system for Fastest vs pytest.
This script performs actual timing comparisons with real test suites.
"""

import os
import sys
import time
import json
import subprocess
import statistics
from pathlib import Path
from typing import Dict, List, Tuple, Any
import tempfile
import shutil


class BenchmarkRunner:
    """Handles benchmarking Fastest against pytest."""
    
    def __init__(self, fastest_binary: Path, python_executable: str = "python"):
        self.fastest_binary = fastest_binary
        self.python_executable = python_executable
        self.results = {}
        
    def run_benchmark_suite(self, test_sizes: List[int] = None) -> Dict[str, Any]:
        """Run the complete benchmark suite."""
        if test_sizes is None:
            test_sizes = [5, 10, 25, 50, 100, 200]
            
        print("🚀 Running Real Fastest vs pytest Benchmark")
        print("=" * 50)
        
        all_results = {}
        
        for size in test_sizes:
            print(f"\n📊 Benchmarking {size} tests...")
            result = self._benchmark_test_size(size)
            all_results[f"{size}_tests"] = result
            self._print_size_results(size, result)
            
        # Generate summary
        summary = self._generate_summary(all_results)
        all_results["summary"] = summary
        
        return all_results
    
    def _benchmark_test_size(self, test_count: int, iterations: int = 5) -> Dict[str, Any]:
        """Benchmark a specific test count."""
        # Create temporary test directory
        with tempfile.TemporaryDirectory() as temp_dir:
            temp_path = Path(temp_dir)
            test_file = self._create_test_file(temp_path, test_count)
            
            # Benchmark Fastest
            fastest_times = []
            for i in range(iterations):
                start = time.time()
                result = self._run_fastest(test_file)
                end = time.time()
                if result["success"]:
                    fastest_times.append(end - start)
                else:
                    print(f"⚠️ Fastest failed on iteration {i+1}: {result['error']}")
            
            # Benchmark pytest (only if available)
            pytest_times = []
            pytest_available = self._check_pytest_available()
            
            if pytest_available:
                for i in range(iterations):
                    start = time.time()
                    result = self._run_pytest(test_file)
                    end = time.time()
                    if result["success"]:
                        pytest_times.append(end - start)
                    else:
                        print(f"⚠️ pytest failed on iteration {i+1}: {result['error']}")
            
            # Calculate statistics
            fastest_stats = self._calculate_stats(fastest_times) if fastest_times else None
            pytest_stats = self._calculate_stats(pytest_times) if pytest_times else None
            
            return {
                "test_count": test_count,
                "fastest": fastest_stats,
                "pytest": pytest_stats,
                "speedup": self._calculate_speedup(fastest_stats, pytest_stats),
                "fastest_available": len(fastest_times) > 0,
                "pytest_available": pytest_available and len(pytest_times) > 0,
            }
    
    def _create_test_file(self, temp_dir: Path, test_count: int) -> Path:
        """Create a test file with the specified number of tests."""
        test_file = temp_dir / "test_benchmark.py"
        
        content = '''"""Generated test file for benchmarking."""

def test_simple():
    """A simple test that should pass."""
    assert 1 + 1 == 2

'''
        
        # Add more test functions
        for i in range(2, test_count + 1):
            content += f'''
def test_function_{i}():
    """Test function {i}."""
    assert {i} * 2 == {i * 2}
'''
        
        test_file.write_text(content)
        return test_file
    
    def _run_fastest(self, test_file: Path) -> Dict[str, Any]:
        """Run Fastest on a test file."""
        try:
            cmd = [str(self.fastest_binary), str(test_file), "--no-color"]
            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                timeout=30,
            )
            
            return {
                "success": result.returncode == 0,
                "stdout": result.stdout,
                "stderr": result.stderr,
                "error": None if result.returncode == 0 else f"Exit code: {result.returncode}"
            }
        except subprocess.TimeoutExpired:
            return {"success": False, "error": "Timeout"}
        except Exception as e:
            return {"success": False, "error": str(e)}
    
    def _run_pytest(self, test_file: Path) -> Dict[str, Any]:
        """Run pytest on a test file."""
        try:
            cmd = [self.python_executable, "-m", "pytest", str(test_file), "-v", "--tb=no", "--no-header"]
            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                timeout=30,
            )
            
            return {
                "success": result.returncode == 0,
                "stdout": result.stdout,
                "stderr": result.stderr,
                "error": None if result.returncode == 0 else f"Exit code: {result.returncode}"
            }
        except subprocess.TimeoutExpired:
            return {"success": False, "error": "Timeout"}
        except Exception as e:
            return {"success": False, "error": str(e)}
    
    def _check_pytest_available(self) -> bool:
        """Check if pytest is available."""
        try:
            result = subprocess.run(
                [self.python_executable, "-m", "pytest", "--version"],
                capture_output=True,
                timeout=5,
            )
            return result.returncode == 0
        except:
            return False
    
    def _calculate_stats(self, times: List[float]) -> Dict[str, float]:
        """Calculate statistics from timing data."""
        if not times:
            return None
            
        return {
            "mean": statistics.mean(times),
            "median": statistics.median(times),
            "min": min(times),
            "max": max(times),
            "std_dev": statistics.stdev(times) if len(times) > 1 else 0.0,
            "runs": len(times),
        }
    
    def _calculate_speedup(self, fastest_stats: Dict, pytest_stats: Dict) -> float:
        """Calculate speedup factor."""
        if not fastest_stats or not pytest_stats:
            return None
        
        return pytest_stats["mean"] / fastest_stats["mean"]
    
    def _print_size_results(self, size: int, result: Dict[str, Any]):
        """Print results for a specific test size."""
        fastest = result["fastest"]
        pytest = result["pytest"]
        speedup = result["speedup"]
        
        print(f"  Tests: {size}")
        
        if fastest:
            print(f"  Fastest: {fastest['mean']:.3f}s ± {fastest['std_dev']:.3f}s")
        else:
            print(f"  Fastest: FAILED")
            
        if pytest:
            print(f"  pytest:  {pytest['mean']:.3f}s ± {pytest['std_dev']:.3f}s")
        else:
            print(f"  pytest:  {'NOT AVAILABLE' if not result['pytest_available'] else 'FAILED'}")
            
        if speedup:
            print(f"  Speedup: {speedup:.2f}x")
        else:
            print(f"  Speedup: Cannot calculate")
    
    def _generate_summary(self, results: Dict[str, Any]) -> Dict[str, Any]:
        """Generate benchmark summary."""
        speedups = []
        fastest_times = []
        pytest_times = []
        
        for key, result in results.items():
            if key.endswith("_tests") and result["speedup"]:
                speedups.append(result["speedup"])
                fastest_times.append(result["fastest"]["mean"])
                pytest_times.append(result["pytest"]["mean"])
        
        if not speedups:
            return {"error": "No valid comparisons found"}
        
        return {
            "average_speedup": statistics.mean(speedups),
            "median_speedup": statistics.median(speedups),
            "max_speedup": max(speedups),
            "min_speedup": min(speedups),
            "total_fastest_time": sum(fastest_times),
            "total_pytest_time": sum(pytest_times),
            "valid_comparisons": len(speedups),
        }
    
    def save_results(self, results: Dict[str, Any], filename: str = "benchmark_results.json"):
        """Save benchmark results to file."""
        output_path = Path(__file__).parent / filename
        with open(output_path, 'w') as f:
            json.dump(results, f, indent=2)
        print(f"\n📊 Results saved to: {output_path}")


def main():
    """Main benchmark execution."""
    # Find fastest binary
    fastest_binary = Path(__file__).parent.parent / "target" / "release" / "fastest"
    
    if not fastest_binary.exists():
        print("❌ Fastest binary not found. Please build with: cargo build --release")
        sys.exit(1)
    
    # Run benchmarks
    runner = BenchmarkRunner(fastest_binary)
    results = runner.run_benchmark_suite()
    
    # Print summary
    summary = results.get("summary", {})
    if "error" not in summary:
        print("\n" + "=" * 50)
        print("🏆 BENCHMARK SUMMARY")
        print("=" * 50)
        print(f"Average speedup: {summary['average_speedup']:.2f}x")
        print(f"Median speedup:  {summary['median_speedup']:.2f}x")
        print(f"Max speedup:     {summary['max_speedup']:.2f}x")
        print(f"Min speedup:     {summary['min_speedup']:.2f}x")
        print(f"Valid comparisons: {summary['valid_comparisons']}")
        
        if summary['average_speedup'] > 1.0:
            print(f"\n🎉 Fastest is {summary['average_speedup']:.2f}x faster than pytest on average!")
        elif summary['average_speedup'] < 1.0:
            print(f"\n⚠️ Fastest is {1/summary['average_speedup']:.2f}x slower than pytest on average")
        else:
            print(f"\n🤝 Fastest and pytest have similar performance")
    else:
        print(f"\n❌ Benchmark failed: {summary['error']}")
    
    # Save results
    runner.save_results(results)


if __name__ == "__main__":
    main()