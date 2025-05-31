#!/usr/bin/env python3
"""
Comprehensive performance benchmark comparing fastest vs pytest.

This is the main benchmark script that validates performance claims
across different test suite sizes and execution strategies.
"""

import subprocess
import time
import tempfile
import statistics
import json
import sys
from pathlib import Path
from typing import Dict, List, Tuple


class FastestBenchmark:
    """Main benchmark suite for fastest vs pytest comparison"""
    
    def __init__(self):
        self.results = {}
        self.temp_dirs = []
    
    def create_test_suite(self, num_tests: int, test_dir: Path) -> None:
        """Create a test suite with the specified number of tests"""
        test_dir.mkdir(parents=True, exist_ok=True)
        
        # Create __init__.py
        (test_dir / "__init__.py").write_text("")
        
        # Distribute tests across multiple files for realism
        tests_per_file = min(25, max(5, num_tests // 4))
        num_files = (num_tests + tests_per_file - 1) // tests_per_file
        
        test_count = 0
        for file_i in range(num_files):
            if test_count >= num_tests:
                break
                
            file_content = f'"""Test file {file_i}."""\n\n'
            
            tests_in_this_file = min(tests_per_file, num_tests - test_count)
            for test_i in range(tests_in_this_file):
                file_content += f'''def test_function_{file_i}_{test_i}():
    """Simple test function."""
    assert True

'''
                test_count += 1
            
            test_file = test_dir / f"test_module_{file_i}.py"
            test_file.write_text(file_content)
    
    def benchmark_discovery(self, test_dir: Path) -> Dict[str, float]:
        """Benchmark test discovery for both runners"""
        results = {}
        
        # Benchmark fastest discovery
        fastest_times = []
        for _ in range(5):
            start = time.perf_counter()
            result = subprocess.run([
                "./target/release/fastest", str(test_dir), "--collect-only", "-q"
            ], capture_output=True, text=True, cwd="/Users/derensnonwork/Desktop/Files/Coding/fastest")
            
            if result.returncode == 0:
                fastest_times.append(time.perf_counter() - start)
        
        if fastest_times:
            results['fastest'] = statistics.mean(fastest_times)
        
        # Benchmark pytest discovery
        pytest_times = []
        for _ in range(5):
            start = time.perf_counter()
            result = subprocess.run([
                sys.executable, "-m", "pytest", str(test_dir), "--collect-only", "-q"
            ], capture_output=True, text=True)
            
            if result.returncode == 0:
                pytest_times.append(time.perf_counter() - start)
        
        if pytest_times:
            results['pytest'] = statistics.mean(pytest_times)
        
        return results
    
    def benchmark_execution(self, test_dir: Path) -> Dict[str, float]:
        """Benchmark test execution for both runners"""
        results = {}
        
        # Benchmark fastest execution
        start = time.perf_counter()
        result = subprocess.run([
            "./target/release/fastest", str(test_dir), "-q"
        ], capture_output=True, text=True, cwd="/Users/derensnonwork/Desktop/Files/Coding/fastest")
        
        if result.returncode == 0:
            results['fastest'] = time.perf_counter() - start
        
        # Benchmark pytest execution
        start = time.perf_counter()
        result = subprocess.run([
            sys.executable, "-m", "pytest", str(test_dir), "-q"
        ], capture_output=True, text=True)
        
        if result.returncode == 0:
            results['pytest'] = time.perf_counter() - start
        
        return results
    
    def run_benchmark_suite(self, test_sizes: List[int] = None) -> Dict:
        """Run the complete benchmark suite"""
        if test_sizes is None:
            test_sizes = [10, 20, 50, 100, 500, 1000]
        
        print("🚀 Fastest vs Pytest Comprehensive Benchmark")
        print("=" * 60)
        
        all_results = {
            'discovery': {},
            'execution': {},
            'metadata': {
                'test_sizes': test_sizes,
                'timestamp': time.time()
            }
        }
        
        for size in test_sizes:
            print(f"\n📊 Testing with {size} tests...")
            
            # Create temporary test directory
            temp_dir = Path(tempfile.mkdtemp(prefix=f"benchmark_{size}_"))
            self.temp_dirs.append(temp_dir)
            
            try:
                # Create test suite
                self.create_test_suite(size, temp_dir)
                
                # Benchmark discovery
                print(f"   🔍 Discovery benchmark...")
                discovery_results = self.benchmark_discovery(temp_dir)
                all_results['discovery'][size] = discovery_results
                
                if 'fastest' in discovery_results and 'pytest' in discovery_results:
                    speedup = discovery_results['pytest'] / discovery_results['fastest']
                    print(f"      Fastest: {discovery_results['fastest']:.3f}s")
                    print(f"      Pytest:  {discovery_results['pytest']:.3f}s")
                    print(f"      Speedup: {speedup:.1f}x faster")
                
                # Benchmark execution
                print(f"   ⚡ Execution benchmark...")
                execution_results = self.benchmark_execution(temp_dir)
                all_results['execution'][size] = execution_results
                
                if 'fastest' in execution_results and 'pytest' in execution_results:
                    speedup = execution_results['pytest'] / execution_results['fastest']
                    print(f"      Fastest: {execution_results['fastest']:.3f}s")
                    print(f"      Pytest:  {execution_results['pytest']:.3f}s")
                    print(f"      Speedup: {speedup:.1f}x faster")
                
            except Exception as e:
                print(f"   ❌ Error benchmarking {size} tests: {e}")
        
        return all_results
    
    def print_summary(self, results: Dict) -> None:
        """Print a summary of benchmark results"""
        print("\n" + "=" * 60)
        print("📈 BENCHMARK SUMMARY")
        print("=" * 60)
        
        # Discovery summary
        print("\n🔍 Test Discovery Performance:")
        print("Size     Fastest   Pytest    Speedup")
        print("-" * 40)
        
        for size in sorted(results['discovery'].keys()):
            disc_data = results['discovery'][size]
            if 'fastest' in disc_data and 'pytest' in disc_data:
                speedup = disc_data['pytest'] / disc_data['fastest']
                print(f"{size:4d}     {disc_data['fastest']:.3f}s    {disc_data['pytest']:.3f}s    {speedup:.1f}x")
        
        # Execution summary
        print("\n⚡ Test Execution Performance:")
        print("Size     Fastest   Pytest    Speedup")
        print("-" * 40)
        
        for size in sorted(results['execution'].keys()):
            exec_data = results['execution'][size]
            if 'fastest' in exec_data and 'pytest' in exec_data:
                speedup = exec_data['pytest'] / exec_data['fastest']
                print(f"{size:4d}     {exec_data['fastest']:.3f}s    {exec_data['pytest']:.3f}s    {speedup:.1f}x")
    
    def cleanup(self):
        """Clean up temporary directories"""
        import shutil
        for temp_dir in self.temp_dirs:
            if temp_dir.exists():
                shutil.rmtree(temp_dir)


def main():
    """Run the main benchmark"""
    benchmark = FastestBenchmark()
    
    try:
        # Run with default test sizes
        results = benchmark.run_benchmark_suite()
        
        # Print summary
        benchmark.print_summary(results)
        
        # Save results to file
        results_file = Path("benchmarks/latest_benchmark_results.json")
        with open(results_file, 'w') as f:
            json.dump(results, f, indent=2)
        
        print(f"\n💾 Results saved to {results_file}")
        
    finally:
        benchmark.cleanup()


if __name__ == "__main__":
    main()