#!/usr/bin/env python3
"""
Comprehensive benchmark suite comparing pytest vs fastest performance
Validates all execution strategy claims with concrete data
"""

import time
import subprocess
import json
import statistics
from pathlib import Path
from typing import Dict, List, Tuple
import tempfile
import shutil

class BenchmarkSuite:
    """Comprehensive pytest vs fastest performance validation"""
    
    def __init__(self):
        self.results = {}
        self.temp_dirs = []
        
    def cleanup(self):
        """Clean up temporary test directories"""
        for temp_dir in self.temp_dirs:
            if temp_dir.exists():
                shutil.rmtree(temp_dir)
    
    def create_test_suite(self, size: int, complexity: str = "simple") -> Path:
        """Create a test suite of specified size and complexity"""
        temp_dir = Path(tempfile.mkdtemp(prefix=f"fastest_bench_{size}_"))
        self.temp_dirs.append(temp_dir)
        
        if complexity == "simple":
            self._create_simple_tests(temp_dir, size)
        elif complexity == "fixtures":
            self._create_fixture_tests(temp_dir, size)
        elif complexity == "parametrized":
            self._create_parametrized_tests(temp_dir, size)
        elif complexity == "classes":
            self._create_class_tests(temp_dir, size)
        
        return temp_dir
    
    def _create_simple_tests(self, test_dir: Path, count: int):
        """Create simple arithmetic tests"""
        test_file = test_dir / "test_simple.py"
        
        tests = []
        for i in range(count):
            tests.append(f"""
def test_addition_{i}():
    assert {i} + {i} == {i * 2}

def test_subtraction_{i}():
    assert {i + 10} - {i} == 10
""")
        
        test_file.write_text("\n".join(tests))
    
    def _create_fixture_tests(self, test_dir: Path, count: int):
        """Create tests with various fixture types"""
        test_file = test_dir / "test_fixtures.py"
        
        content = '''
import pytest
import tempfile
from pathlib import Path

@pytest.fixture
def sample_data():
    return {"key": "value", "number": 42}

@pytest.fixture  
def tmp_file():
    with tempfile.NamedTemporaryFile(mode='w', delete=False) as f:
        f.write("test content")
        return Path(f.name)

@pytest.fixture(scope="session")
def session_data():
    return {"session": "global", "created_at": "startup"}

'''
        
        tests = []
        for i in range(count):
            tests.append(f"""
def test_with_fixture_{i}(sample_data):
    assert sample_data["number"] == 42
    assert sample_data["key"] == "value"

def test_with_tmp_file_{i}(tmp_file):
    assert tmp_file.exists()
    content = tmp_file.read_text()
    assert "test content" in content

def test_with_session_fixture_{i}(session_data):
    assert session_data["session"] == "global"
""")
        
        content += "\n".join(tests)
        test_file.write_text(content)
    
    def _create_parametrized_tests(self, test_dir: Path, count: int):
        """Create parametrized tests"""
        test_file = test_dir / "test_parametrized.py"
        
        content = '''
import pytest

'''
        
        # Create parametrized tests
        for i in range(count // 3):  # Each parametrized test generates 3 test cases
            content += f'''
@pytest.mark.parametrize("input,expected", [
    (1, 2), (2, 4), (3, 6)
])
def test_parametrized_{i}(input, expected):
    assert input * 2 == expected

'''
        
        test_file.write_text(content)
    
    def _create_class_tests(self, test_dir: Path, count: int):
        """Create class-based tests"""
        test_file = test_dir / "test_classes.py"
        
        content = '''
import pytest

class TestMath:
    def setup_method(self):
        self.base_value = 10
    
'''
        
        for i in range(count):
            content += f'''
    def test_method_{i}(self):
        assert self.base_value + {i} == {10 + i}
        
'''
        
        test_file.write_text(content)
    
    def run_pytest(self, test_dir: Path, iterations: int = 3) -> Dict:
        """Run pytest and measure performance"""
        times = []
        
        for _ in range(iterations):
            start = time.time()
            result = subprocess.run(
                ["python", "-m", "pytest", str(test_dir), "-v", "--tb=short"],
                capture_output=True,
                text=True,
                cwd=test_dir.parent
            )
            end = time.time()
            
            times.append(end - start)
            
            if result.returncode != 0:
                print(f"pytest failed: {result.stderr}")
        
        return {
            "mean_time": statistics.mean(times),
            "min_time": min(times),
            "max_time": max(times),
            "std_dev": statistics.stdev(times) if len(times) > 1 else 0,
            "times": times
        }
    
    def run_fastest(self, test_dir: Path, iterations: int = 3) -> Dict:
        """Run fastest and measure performance"""
        times = []
        fastest_binary = Path(__file__).parent.parent / "target" / "release" / "fastest"
        
        if not fastest_binary.exists():
            raise FileNotFoundError(f"Fastest binary not found at {fastest_binary}")
        
        for _ in range(iterations):
            start = time.time()
            result = subprocess.run(
                [str(fastest_binary), str(test_dir), "-v"],
                capture_output=True,
                text=True,
                cwd=test_dir.parent
            )
            end = time.time()
            
            times.append(end - start)
            
            if result.returncode != 0:
                print(f"fastest failed: {result.stderr}")
        
        return {
            "mean_time": statistics.mean(times),
            "min_time": min(times),
            "max_time": max(times),
            "std_dev": statistics.stdev(times) if len(times) > 1 else 0,
            "times": times
        }
    
    def benchmark_execution_strategies(self):
        """Benchmark all three execution strategies"""
        strategies = [
            ("InProcess", 8, "simple"),      # ≤20 tests
            ("InProcess", 15, "fixtures"),   # ≤20 tests with fixtures  
            ("WarmWorkers", 35, "simple"),   # 21-100 tests
            ("WarmWorkers", 50, "fixtures"), # 21-100 tests with fixtures
            ("WarmWorkers", 75, "parametrized"), # 21-100 parametrized
            ("FullParallel", 150, "simple"), # >100 tests
            ("FullParallel", 200, "classes"), # >100 class tests
            ("FullParallel", 300, "fixtures"), # >100 with fixtures
        ]
        
        results = {}
        
        for strategy, test_count, complexity in strategies:
            print(f"\n🔥 Benchmarking {strategy} strategy: {test_count} {complexity} tests")
            
            test_dir = self.create_test_suite(test_count, complexity)
            
            # Run pytest
            print("  Running pytest...")
            pytest_results = self.run_pytest(test_dir)
            
            # Run fastest
            print("  Running fastest...")
            fastest_results = self.run_fastest(test_dir)
            
            # Calculate speedup
            speedup = pytest_results["mean_time"] / fastest_results["mean_time"]
            
            benchmark_key = f"{strategy}_{test_count}_{complexity}"
            results[benchmark_key] = {
                "strategy": strategy,
                "test_count": test_count,
                "complexity": complexity,
                "pytest": pytest_results,
                "fastest": fastest_results,
                "speedup": speedup,
                "faster": speedup > 1.0
            }
            
            print(f"  Results: pytest={pytest_results['mean_time']:.3f}s, fastest={fastest_results['mean_time']:.3f}s")
            print(f"  Speedup: {speedup:.2f}x {'✓' if speedup > 1.0 else '✗'}")
        
        return results
    
    def benchmark_real_world_scenarios(self):
        """Benchmark realistic test scenarios"""
        print("\n🌍 Real-world scenario benchmarks")
        
        scenarios = [
            ("unit_tests", 45, "simple"),       # Typical unit test suite
            ("integration", 25, "fixtures"),    # Integration tests with setup
            ("api_tests", 30, "parametrized"),  # API tests with multiple inputs
            ("large_suite", 180, "classes"),    # Large codebase test suite
        ]
        
        results = {}
        
        for scenario, count, complexity in scenarios:
            print(f"\n  Scenario: {scenario} ({count} {complexity} tests)")
            
            test_dir = self.create_test_suite(count, complexity)
            
            pytest_results = self.run_pytest(test_dir)
            fastest_results = self.run_fastest(test_dir)
            
            speedup = pytest_results["mean_time"] / fastest_results["mean_time"]
            
            results[scenario] = {
                "test_count": count,
                "complexity": complexity,
                "pytest_time": pytest_results["mean_time"],
                "fastest_time": fastest_results["mean_time"],
                "speedup": speedup,
                "memory_efficiency": "TBD"  # TODO: Add memory profiling
            }
            
            print(f"    pytest: {pytest_results['mean_time']:.3f}s")
            print(f"    fastest: {fastest_results['mean_time']:.3f}s")
            print(f"    speedup: {speedup:.2f}x")
        
        return results
    
    def generate_report(self, strategy_results: Dict, scenario_results: Dict):
        """Generate comprehensive performance report"""
        
        report = {
            "benchmark_timestamp": time.strftime("%Y-%m-%d %H:%M:%S"),
            "execution_strategies": strategy_results,
            "real_world_scenarios": scenario_results,
            "summary": self._generate_summary(strategy_results, scenario_results)
        }
        
        # Save detailed results
        results_file = Path(__file__).parent / "performance_validation.json"
        with open(results_file, 'w') as f:
            json.dump(report, f, indent=2)
        
        # Generate markdown report
        self._generate_markdown_report(report)
        
        return report
    
    def _generate_summary(self, strategy_results: Dict, scenario_results: Dict) -> Dict:
        """Generate performance summary statistics"""
        
        # Strategy performance
        inprocess_speedups = [r["speedup"] for k, r in strategy_results.items() if r["strategy"] == "InProcess"]
        warmworkers_speedups = [r["speedup"] for k, r in strategy_results.items() if r["strategy"] == "WarmWorkers"] 
        fullparallel_speedups = [r["speedup"] for k, r in strategy_results.items() if r["strategy"] == "FullParallel"]
        
        # Scenario performance
        scenario_speedups = [r["speedup"] for r in scenario_results.values()]
        
        return {
            "strategy_performance": {
                "InProcess": {
                    "mean_speedup": statistics.mean(inprocess_speedups) if inprocess_speedups else 0,
                    "range": f"{min(inprocess_speedups):.1f}x - {max(inprocess_speedups):.1f}x" if inprocess_speedups else "N/A"
                },
                "WarmWorkers": {
                    "mean_speedup": statistics.mean(warmworkers_speedups) if warmworkers_speedups else 0,
                    "range": f"{min(warmworkers_speedups):.1f}x - {max(warmworkers_speedups):.1f}x" if warmworkers_speedups else "N/A"
                },
                "FullParallel": {
                    "mean_speedup": statistics.mean(fullparallel_speedups) if fullparallel_speedups else 0,
                    "range": f"{min(fullparallel_speedups):.1f}x - {max(fullparallel_speedups):.1f}x" if fullparallel_speedups else "N/A"
                }
            },
            "overall_performance": {
                "mean_speedup": statistics.mean(scenario_speedups) if scenario_speedups else 0,
                "scenarios_faster": sum(1 for s in scenario_speedups if s > 1.0),
                "total_scenarios": len(scenario_speedups)
            }
        }
    
    def _generate_markdown_report(self, report: Dict):
        """Generate markdown performance report"""
        
        md_content = f'''# Fastest vs pytest Performance Validation

**Generated**: {report["benchmark_timestamp"]}

## Executive Summary

'''
        
        summary = report["summary"]
        overall = summary["overall_performance"]
        
        md_content += f'''
- **Overall Performance**: {overall["mean_speedup"]:.2f}x faster than pytest
- **Success Rate**: {overall["scenarios_faster"]}/{overall["total_scenarios"]} scenarios faster
- **Strategy Validation**: All execution strategies tested

## Execution Strategy Performance

'''
        
        for strategy, perf in summary["strategy_performance"].items():
            md_content += f'''
### {strategy} Strategy
- **Mean Speedup**: {perf["mean_speedup"]:.2f}x
- **Range**: {perf["range"]}

'''
        
        md_content += '''
## Detailed Results

### Real-World Scenarios

| Scenario | Test Count | pytest Time | fastest Time | Speedup |
|----------|------------|-------------|--------------|---------|
'''
        
        for scenario, data in report["real_world_scenarios"].items():
            md_content += f'''| {scenario} | {data["test_count"]} | {data["pytest_time"]:.3f}s | {data["fastest_time"]:.3f}s | {data["speedup"]:.2f}x |
'''
        
        md_content += '''

### Strategy Breakdown

| Strategy | Test Count | Complexity | pytest Time | fastest Time | Speedup | Status |
|----------|------------|------------|-------------|--------------|---------|--------|
'''
        
        for key, data in report["execution_strategies"].items():
            status = "✅" if data["faster"] else "❌"
            md_content += f'''| {data["strategy"]} | {data["test_count"]} | {data["complexity"]} | {data["pytest"]["mean_time"]:.3f}s | {data["fastest"]["mean_time"]:.3f}s | {data["speedup"]:.2f}x | {status} |
'''
        
        # Save markdown report
        report_file = Path(__file__).parent / "PERFORMANCE_VALIDATION.md"
        report_file.write_text(md_content)
        
        print(f"\n📊 Performance reports generated:")
        print(f"   JSON: {Path(__file__).parent / 'performance_validation.json'}")
        print(f"   Markdown: {report_file}")

def main():
    """Run comprehensive benchmark suite"""
    print("🚀 Fastest vs pytest Performance Validation Suite")
    print("=" * 50)
    
    suite = BenchmarkSuite()
    
    try:
        # Benchmark execution strategies
        strategy_results = suite.benchmark_execution_strategies()
        
        # Benchmark real-world scenarios  
        scenario_results = suite.benchmark_real_world_scenarios()
        
        # Generate comprehensive report
        report = suite.generate_report(strategy_results, scenario_results)
        
        print("\n🎯 Benchmark Summary:")
        print(f"   Overall speedup: {report['summary']['overall_performance']['mean_speedup']:.2f}x")
        print(f"   Faster scenarios: {report['summary']['overall_performance']['scenarios_faster']}/{report['summary']['overall_performance']['total_scenarios']}")
        
        return report
        
    finally:
        suite.cleanup()

if __name__ == "__main__":
    main()