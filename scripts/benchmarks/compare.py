#!/usr/bin/env python3
"""
Automated Pytest vs Fastest Comparison Tool

This script provides comprehensive comparison between pytest and fastest
to track development progress and compatibility improvements.
"""

import argparse
import json
import subprocess
import time
import sys
import os
from pathlib import Path
from typing import Dict, List, Tuple, Optional
from dataclasses import dataclass, asdict
import tempfile
import shutil

@dataclass
class TestResult:
    """Test execution result"""
    passed: int
    failed: int
    skipped: int
    errors: int
    total: int
    duration: float
    exit_code: int
    output: str
    compatibility_score: float = 0.0

@dataclass
class DiscoveryResult:
    """Test discovery result"""
    test_count: int
    duration: float
    exit_code: int
    output: str

@dataclass
class ComparisonResult:
    """Complete comparison result"""
    timestamp: str
    test_directory: str
    fastest_discovery: DiscoveryResult
    pytest_discovery: DiscoveryResult
    fastest_execution: TestResult
    pytest_execution: TestResult
    performance_ratio: Dict[str, float]
    compatibility_analysis: Dict[str, any]
    summary: Dict[str, str]

class PytestFastestComparator:
    """Main comparison engine"""
    
    def __init__(self, fastest_binary: str = "fastest", pytest_binary: str = "pytest"):
        # Default to local binary if no specific path given
        if fastest_binary == "fastest" and Path("./target/release/fastest").exists():
            self.fastest_binary = "./target/release/fastest"
        else:
            self.fastest_binary = fastest_binary
        self.pytest_binary = pytest_binary
        self.results_dir = Path("comparison_results")
        self.results_dir.mkdir(exist_ok=True)
        
    def run_command(self, cmd: List[str], timeout: int = 120) -> Tuple[int, str, float]:
        """Run command and return exit code, output, and duration"""
        start_time = time.time()
        try:
            result = subprocess.run(
                cmd, 
                capture_output=True, 
                text=True, 
                timeout=timeout,
                cwd=os.getcwd()
            )
            duration = time.time() - start_time
            output = result.stdout + result.stderr
            return result.returncode, output, duration
        except subprocess.TimeoutExpired:
            duration = time.time() - start_time
            return -1, f"Command timed out after {timeout}s", duration
        except Exception as e:
            duration = time.time() - start_time
            return -2, f"Command failed: {e}", duration

    def strip_ansi_codes(self, text: str) -> str:
        """Remove ANSI color codes from text"""
        import re
        ansi_escape = re.compile(r'\x1B(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~])')
        return ansi_escape.sub('', text)

    def discover_tests_fastest(self, test_dir: str) -> DiscoveryResult:
        """Run test discovery with fastest"""
        cmd = [self.fastest_binary, "--dry-run", test_dir]
        exit_code, output, duration = self.run_command(cmd)
        
        # Parse test count from fastest's output
        test_count = 0
        
        # Remove ANSI color codes for easier parsing
        clean_output = self.strip_ansi_codes(output)
        
        for line in clean_output.split('\n'):
            line = line.strip()
            # Look for patterns like "Running 27 tests" or "27 tests passed"
            if "running" in line.lower() and "test" in line.lower():
                try:
                    words = line.split()
                    for i, word in enumerate(words):
                        if word.isdigit():
                            test_count = int(word)
                            break
                except (IndexError, ValueError):
                    pass
            elif "test" in line.lower() and "passed" in line.lower():
                try:
                    words = line.split()
                    for i, word in enumerate(words):
                        if word.isdigit():
                            test_count = int(word)
                            break
                except (IndexError, ValueError):
                    pass
        
        return DiscoveryResult(
            test_count=test_count,
            duration=duration,
            exit_code=exit_code,
            output=output
        )

    def discover_tests_pytest(self, test_dir: str) -> DiscoveryResult:
        """Run test discovery with pytest"""
        cmd = [self.pytest_binary, "--cache-clear", "--collect-only", "-q", test_dir]
        exit_code, output, duration = self.run_command(cmd)
        
        # Parse test count from pytest output
        test_count = 0
        for line in output.split('\n'):
            if "test" in line and "collected" in line:
                try:
                    words = line.split()
                    for i, word in enumerate(words):
                        if word.isdigit():
                            test_count = int(word)
                            break
                except (IndexError, ValueError):
                    pass
        
        return DiscoveryResult(
            test_count=test_count,
            duration=duration,
            exit_code=exit_code,
            output=output
        )

    def execute_tests_fastest(self, test_dir: str) -> TestResult:
        """Run tests with fastest"""
        cmd = [self.fastest_binary, "-v", test_dir]
        exit_code, output, duration = self.run_command(cmd)
        
        return self.parse_test_results(output, duration, exit_code)

    def execute_tests_pytest(self, test_dir: str) -> TestResult:
        """Run tests with pytest"""
        cmd = [self.pytest_binary, "--cache-clear", "-v", test_dir]
        exit_code, output, duration = self.run_command(cmd)
        
        return self.parse_test_results(output, duration, exit_code)

    def parse_test_results(self, output: str, duration: float, exit_code: int) -> TestResult:
        """Parse test execution results from output"""
        passed = failed = skipped = errors = 0
        
        # Remove ANSI color codes for easier parsing
        clean_output = self.strip_ansi_codes(output)
        
        # Parse different output formats
        lines = clean_output.split('\n')
        for line in lines:
            line_lower = line.lower().strip()
            
            # Handle fastest's output format: "27 tests passed"
            if "test" in line_lower and "passed" in line_lower:
                try:
                    words = line.split()
                    for i, word in enumerate(words):
                        if word.isdigit():
                            count = int(word)
                            if "test" in " ".join(words[i:i+3]).lower():
                                passed = count
                                break
                except (IndexError, ValueError):
                    pass
            
            # Handle pytest's output format: "5 passed, 2 failed, 1 skipped"
            elif "passed" in line_lower or "failed" in line_lower or "skipped" in line_lower or "error" in line_lower:
                words = line.split()
                for i, word in enumerate(words):
                    if word.isdigit():
                        count = int(word)
                        if i + 1 < len(words):
                            status = words[i + 1].lower()
                            if status.startswith("passed"):
                                passed = count
                            elif status.startswith("failed"):
                                failed = count
                            elif status.startswith("skipped"):
                                skipped = count
                            elif status.startswith("error"):
                                errors = count
        
        total = passed + failed + skipped + errors
        
        return TestResult(
            passed=passed,
            failed=failed,
            skipped=skipped,
            errors=errors,
            total=total,
            duration=duration,
            exit_code=exit_code,
            output=output
        )

    def analyze_compatibility(self, fastest_result: TestResult, pytest_result: TestResult) -> Dict[str, any]:
        """Analyze compatibility between fastest and pytest results"""
        if pytest_result.total == 0:
            return {"compatibility_score": 0.0, "analysis": "No pytest baseline"}
        
        # Calculate compatibility score based on result similarity
        total_diff = abs(fastest_result.total - pytest_result.total)
        passed_diff = abs(fastest_result.passed - pytest_result.passed)
        failed_diff = abs(fastest_result.failed - pytest_result.failed)
        
        max_tests = max(fastest_result.total, pytest_result.total, 1)
        compatibility_score = max(0.0, 1.0 - (total_diff + passed_diff + failed_diff) / (max_tests * 2))
        
        analysis = {
            "test_count_match": fastest_result.total == pytest_result.total,
            "result_distribution_similarity": compatibility_score > 0.9,
            "both_successful": fastest_result.exit_code == 0 and pytest_result.exit_code == 0,
            "total_diff": total_diff,
            "passed_diff": passed_diff,
            "failed_diff": failed_diff
        }
        
        return {
            "compatibility_score": compatibility_score,
            "analysis": analysis
        }

    def calculate_performance_ratios(self, fastest_discovery: DiscoveryResult, 
                                   pytest_discovery: DiscoveryResult,
                                   fastest_execution: TestResult, 
                                   pytest_execution: TestResult) -> Dict[str, float]:
        """Calculate performance improvement ratios"""
        ratios = {}
        
        # Discovery speedup
        if pytest_discovery.duration > 0:
            ratios["discovery_speedup"] = pytest_discovery.duration / max(fastest_discovery.duration, 0.001)
        else:
            ratios["discovery_speedup"] = 1.0
            
        # Execution speedup  
        if pytest_execution.duration > 0:
            ratios["execution_speedup"] = pytest_execution.duration / max(fastest_execution.duration, 0.001)
        else:
            ratios["execution_speedup"] = 1.0
            
        # Overall speedup
        fastest_total = fastest_discovery.duration + fastest_execution.duration
        pytest_total = pytest_discovery.duration + pytest_execution.duration
        if pytest_total > 0:
            ratios["total_speedup"] = pytest_total / max(fastest_total, 0.001)
        else:
            ratios["total_speedup"] = 1.0
        
        return ratios

    def compare(self, test_directory: str, save_results: bool = True) -> ComparisonResult:
        """Run complete comparison"""
        print(f"🔍 Comparing fastest vs pytest on: {test_directory}")
        
        # Test discovery
        print("📋 Running test discovery...")
        fastest_discovery = self.discover_tests_fastest(test_directory)
        pytest_discovery = self.discover_tests_pytest(test_directory)
        
        # Test execution
        print("⚡ Running test execution...")
        fastest_execution = self.execute_tests_fastest(test_directory)
        pytest_execution = self.execute_tests_pytest(test_directory)
        
        # Analysis
        print("📊 Analyzing results...")
        compatibility = self.analyze_compatibility(fastest_execution, pytest_execution)
        performance = self.calculate_performance_ratios(
            fastest_discovery, pytest_discovery, fastest_execution, pytest_execution
        )
        
        # Generate summary
        summary = self.generate_summary(fastest_discovery, pytest_discovery, 
                                      fastest_execution, pytest_execution, 
                                      compatibility, performance)
        
        result = ComparisonResult(
            timestamp=time.strftime("%Y-%m-%d %H:%M:%S"),
            test_directory=test_directory,
            fastest_discovery=fastest_discovery,
            pytest_discovery=pytest_discovery,
            fastest_execution=fastest_execution,
            pytest_execution=pytest_execution,
            performance_ratio=performance,
            compatibility_analysis=compatibility,
            summary=summary
        )
        
        if save_results:
            self.save_results(result)
            
        return result

    def generate_summary(self, fastest_discovery: DiscoveryResult, pytest_discovery: DiscoveryResult,
                        fastest_execution: TestResult, pytest_execution: TestResult,
                        compatibility: Dict, performance: Dict) -> Dict[str, str]:
        """Generate human-readable summary"""
        
        discovery_status = "✅" if fastest_discovery.exit_code == 0 else "❌"
        execution_status = "✅" if fastest_execution.exit_code == 0 else "❌" 
        
        compat_score = compatibility.get("compatibility_score", 0.0)
        compat_status = "✅" if compat_score > 0.9 else "⚠️" if compat_score > 0.7 else "❌"
        
        return {
            "discovery_status": f"{discovery_status} Discovery: {performance.get('discovery_speedup', 1):.1f}x faster",
            "execution_status": f"{execution_status} Execution: {performance.get('execution_speedup', 1):.1f}x faster", 
            "compatibility_status": f"{compat_status} Compatibility: {compat_score:.1%}",
            "overall_performance": f"🚀 Overall: {performance.get('total_speedup', 1):.1f}x faster",
            "test_results": f"Found {fastest_discovery.test_count} tests, {fastest_execution.passed} passed"
        }

    def save_results(self, result: ComparisonResult):
        """Save results to JSON file"""
        timestamp = result.timestamp.replace(" ", "_").replace(":", "-")
        filename = f"comparison_{timestamp}.json"
        filepath = self.results_dir / filename
        
        with open(filepath, 'w') as f:
            json.dump(asdict(result), f, indent=2)
        
        # Also save as latest.json for easy access
        latest_path = self.results_dir / "latest.json"
        with open(latest_path, 'w') as f:
            json.dump(asdict(result), f, indent=2)
        
        print(f"💾 Results saved to: {filepath}")

    def print_results(self, result: ComparisonResult):
        """Print formatted results to console"""
        print("\n" + "="*80)
        print("🏆 FASTEST vs PYTEST COMPARISON RESULTS")
        print("="*80)
        print(f"📁 Test Directory: {result.test_directory}")
        print(f"⏰ Timestamp: {result.timestamp}")
        print()
        
        print("📋 DISCOVERY COMPARISON:")
        print(f"  Fastest: {result.fastest_discovery.test_count} tests in {result.fastest_discovery.duration:.3f}s")
        print(f"  pytest:  {result.pytest_discovery.test_count} tests in {result.pytest_discovery.duration:.3f}s")
        print(f"  🚀 Speedup: {result.performance_ratio.get('discovery_speedup', 1):.1f}x faster")
        print()
        
        print("⚡ EXECUTION COMPARISON:")
        print(f"  Fastest: {result.fastest_execution.total} tests in {result.fastest_execution.duration:.3f}s")
        print(f"           {result.fastest_execution.passed} passed, {result.fastest_execution.failed} failed")
        print(f"  pytest:  {result.pytest_execution.total} tests in {result.pytest_execution.duration:.3f}s") 
        print(f"           {result.pytest_execution.passed} passed, {result.pytest_execution.failed} failed")
        print(f"  🚀 Speedup: {result.performance_ratio.get('execution_speedup', 1):.1f}x faster")
        print()
        
        print("🎯 COMPATIBILITY ANALYSIS:")
        compat_score = result.compatibility_analysis.get("compatibility_score", 0.0)
        print(f"  Score: {compat_score:.1%}")
        analysis = result.compatibility_analysis.get("analysis", {})
        for key, value in analysis.items():
            if isinstance(value, bool):
                status = "✅" if value else "❌"
                print(f"  {key.replace('_', ' ').title()}: {status}")
        print()
        
        print("📊 SUMMARY:")
        for key, value in result.summary.items():
            print(f"  {value}")
        
        print("="*80)

    def create_sample_tests(self, num_tests: int = 10) -> str:
        """Create sample test directory for testing"""
        temp_dir = tempfile.mkdtemp(prefix="fastest_test_")
        
        # Create simple test file
        test_content = f"""
import pytest

def test_simple():
    assert 1 + 1 == 2

def test_string_ops():
    assert "hello".upper() == "HELLO"

class TestClass:
    def test_method(self):
        assert True
        
    def test_another_method(self):
        assert len([1, 2, 3]) == 3

@pytest.mark.parametrize("x,y,expected", [
    (1, 2, 3),
    (2, 3, 5),
    (10, 20, 30)
])
def test_parametrized(x, y, expected):
    assert x + y == expected

def test_with_fixture(tmp_path):
    file_path = tmp_path / "test.txt"
    file_path.write_text("hello")
    assert file_path.read_text() == "hello"

@pytest.mark.skip(reason="Testing skip")
def test_skipped():
    assert False
"""
        
        # Write multiple test files
        for i in range(max(1, num_tests // 7)):  # ~7 tests per file
            test_file = Path(temp_dir) / f"test_sample_{i}.py"
            test_file.write_text(test_content)
        
        return temp_dir

def main():
    parser = argparse.ArgumentParser(description="Compare fastest vs pytest performance and compatibility")
    parser.add_argument("test_directory", nargs="?", help="Directory containing tests to compare")
    parser.add_argument("--fastest-binary", default="fastest", help="Path to fastest binary")
    parser.add_argument("--pytest-binary", default="pytest", help="Path to pytest binary")
    parser.add_argument("--create-sample", type=int, metavar="N", help="Create sample test directory with N tests")
    parser.add_argument("--save-results", action="store_true", default=True, help="Save results to JSON")
    parser.add_argument("--quiet", "-q", action="store_true", help="Only show summary")
    parser.add_argument("--watch", action="store_true", help="Run continuously, watching for changes")
    
    args = parser.parse_args()
    
    comparator = PytestFastestComparator(args.fastest_binary, args.pytest_binary)
    
    # Handle sample test creation
    if args.create_sample:
        print(f"🏗️  Creating sample test directory with ~{args.create_sample} tests...")
        test_dir = comparator.create_sample_tests(args.create_sample)
        print(f"📁 Created: {test_dir}")
        if not args.test_directory:
            args.test_directory = test_dir
    
    if not args.test_directory:
        print("❌ Error: No test directory provided. Use --create-sample or specify a directory.")
        return 1
    
    if not Path(args.test_directory).exists():
        print(f"❌ Error: Test directory does not exist: {args.test_directory}")
        return 1
    
    try:
        if args.watch:
            print("👀 Watching for changes... (Ctrl+C to stop)")
            import time
            while True:
                result = comparator.compare(args.test_directory, args.save_results)
                if not args.quiet:
                    comparator.print_results(result)
                print(f"\n⏳ Waiting 10 seconds before next comparison...")
                time.sleep(10)
        else:
            result = comparator.compare(args.test_directory, args.save_results)
            if not args.quiet:
                comparator.print_results(result)
            else:
                print("\n".join(result.summary.values()))
    
    except KeyboardInterrupt:
        print("\n👋 Comparison stopped by user")
        return 0
    except Exception as e:
        print(f"❌ Error during comparison: {e}")
        return 1
    
    return 0

if __name__ == "__main__":
    sys.exit(main())