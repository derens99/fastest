#!/usr/bin/env python3
"""
Micro-benchmark to validate performance optimizations
Tests the actual optimizations without benchmark suite bugs
"""

import subprocess
import time
import tempfile
from pathlib import Path

def create_test_files(count: int) -> Path:
    """Create exactly the specified number of tests"""
    temp_dir = Path(tempfile.mkdtemp(prefix=f"opt_test_{count}_"))
    
    test_content = "\n".join([
        f"def test_simple_{i}():\n    assert {i} + 1 == {i + 1}"
        for i in range(count)
    ])
    
    test_file = temp_dir / "test_performance.py"
    test_file.write_text(test_content)
    
    return temp_dir

def benchmark_runner(test_dir: Path, runner: str) -> float:
    """Benchmark a test runner and return execution time"""
    if runner == "pytest":
        cmd = ["python", "-m", "pytest", str(test_dir), "-v", "--tb=short"]
    else:  # fastest
        cmd = ["/Users/derensnonwork/Desktop/Files/Coding/fastest/target/release/fastest", str(test_dir), "-v"]
    
    # Warm up
    subprocess.run(cmd, capture_output=True, cwd=test_dir.parent)
    
    # Actual benchmark (3 runs, take best)
    times = []
    for _ in range(3):
        start = time.perf_counter()
        result = subprocess.run(cmd, capture_output=True, cwd=test_dir.parent)
        end = time.perf_counter()
        
        if result.returncode == 0:
            times.append(end - start)
    
    return min(times) if times else float('inf')

def validate_optimizations():
    """Validate the performance optimizations"""
    test_cases = [
        (1, "InProcess - Single test"),
        (5, "InProcess - Small suite"),  
        (15, "InProcess - Edge case"),
        (25, "WarmWorkers - Small batch"),
        (50, "WarmWorkers - Medium batch"),
        (120, "FullParallel - Large suite"),
    ]
    
    print("🚀 Performance Optimization Validation")
    print("=" * 50)
    print()
    
    results = []
    
    for test_count, description in test_cases:
        print(f"📊 Testing {description}: {test_count} tests")
        
        test_dir = create_test_files(test_count)
        
        try:
            # Benchmark both runners
            pytest_time = benchmark_runner(test_dir, "pytest")
            fastest_time = benchmark_runner(test_dir, "fastest")
            
            speedup = pytest_time / fastest_time if fastest_time > 0 else 0
            status = "✅" if speedup > 1.0 else "❌"
            
            print(f"   pytest:  {pytest_time:.3f}s")
            print(f"   fastest: {fastest_time:.3f}s")
            print(f"   speedup: {speedup:.2f}x {status}")
            print()
            
            results.append({
                'description': description,
                'test_count': test_count,
                'pytest_time': pytest_time,
                'fastest_time': fastest_time,
                'speedup': speedup,
                'faster': speedup > 1.0
            })
            
        finally:
            # Cleanup
            import shutil
            shutil.rmtree(test_dir)
    
    # Summary
    successful = sum(1 for r in results if r['faster'])
    total = len(results)
    avg_speedup = sum(r['speedup'] for r in results) / len(results)
    
    print("📈 **OPTIMIZATION VALIDATION SUMMARY**")
    print(f"   Success rate: {successful}/{total} scenarios faster")
    print(f"   Average speedup: {avg_speedup:.2f}x")
    print()
    
    # Strategy-specific analysis
    inprocess_results = [r for r in results if r['test_count'] <= 20]
    if inprocess_results:
        inprocess_avg = sum(r['speedup'] for r in inprocess_results) / len(inprocess_results)
        print(f"   InProcess strategy average: {inprocess_avg:.2f}x")
    
    warmworkers_results = [r for r in results if 20 < r['test_count'] <= 100]
    if warmworkers_results:
        warmworkers_avg = sum(r['speedup'] for r in warmworkers_results) / len(warmworkers_results)
        print(f"   WarmWorkers strategy average: {warmworkers_avg:.2f}x")
    
    fullparallel_results = [r for r in results if r['test_count'] > 100]
    if fullparallel_results:
        fullparallel_avg = sum(r['speedup'] for r in fullparallel_results) / len(fullparallel_results)
        print(f"   FullParallel strategy average: {fullparallel_avg:.2f}x")

if __name__ == "__main__":
    validate_optimizations()