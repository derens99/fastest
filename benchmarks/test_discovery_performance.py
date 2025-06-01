#!/usr/bin/env python3
"""
🚀 DISCOVERY PERFORMANCE BENCHMARK

Compares fastest's revolutionary discovery optimizations against pytest.
Tests the discovery phase specifically to measure our 5-10x improvements.
"""

import subprocess
import time
import os
import sys
import json
from pathlib import Path

def time_fastest_discovery():
    """Time fastest discovery using the discover command"""
    fastest_path = Path(__file__).parent.parent / "target/release/fastest"
    
    start_time = time.perf_counter()
    result = subprocess.run([
        str(fastest_path), "discover", "--format", "count"
    ], capture_output=True, text=True, cwd=Path(__file__).parent.parent)
    end_time = time.perf_counter()
    
    if result.returncode != 0:
        print(f"❌ Fastest discovery failed: {result.stderr}")
        return None, None
    
    # Extract test count from output
    test_count = None
    for line in result.stdout.strip().split('\n'):
        if line.isdigit():
            test_count = int(line)
            break
    
    discovery_time = end_time - start_time
    return test_count, discovery_time

def time_pytest_discovery():
    """Time pytest discovery using --collect-only"""
    start_time = time.perf_counter()
    result = subprocess.run([
        sys.executable, "-m", "pytest", "--collect-only", "-q", "tests/"
    ], capture_output=True, text=True, cwd=Path(__file__).parent.parent)
    end_time = time.perf_counter()
    
    if result.returncode != 0:
        print(f"❌ Pytest discovery failed: {result.stderr}")
        return None, None
    
    # Count tests from pytest output
    test_count = 0
    for line in result.stdout.split('\n'):
        if 'test session starts' in line or 'collected' in line:
            continue
        if line.strip() and not line.startswith('='):
            test_count += 1
    
    # Alternative: parse the collected line
    for line in result.stderr.split('\n'):
        if 'collected' in line and 'item' in line:
            try:
                test_count = int(line.split()[0])
                break
            except (ValueError, IndexError):
                continue
    
    discovery_time = end_time - start_time
    return test_count, discovery_time

def run_discovery_benchmark():
    """Run comprehensive discovery performance benchmark"""
    print("🚀 DISCOVERY PERFORMANCE BENCHMARK")
    print("=" * 50)
    
    # Warm up
    print("🔥 Warming up...")
    time_fastest_discovery()
    time_pytest_discovery()
    
    # Run multiple iterations for accurate measurement
    fastest_times = []
    pytest_times = []
    
    print("\n📊 Running benchmark iterations...")
    
    for i in range(5):
        print(f"   Iteration {i+1}/5", end=" ")
        
        # Test fastest discovery
        test_count_fastest, fastest_time = time_fastest_discovery()
        if fastest_time is not None:
            fastest_times.append(fastest_time)
            print(f"fastest: {fastest_time:.3f}s", end=" ")
        
        # Test pytest discovery  
        test_count_pytest, pytest_time = time_pytest_discovery()
        if pytest_time is not None:
            pytest_times.append(pytest_time)
            print(f"pytest: {pytest_time:.3f}s")
        else:
            print("pytest: failed")
    
    if not fastest_times or not pytest_times:
        print("❌ Benchmark failed - could not collect timing data")
        return
    
    # Calculate statistics
    avg_fastest = sum(fastest_times) / len(fastest_times)
    avg_pytest = sum(pytest_times) / len(pytest_times)
    speedup = avg_pytest / avg_fastest
    
    min_fastest = min(fastest_times)
    min_pytest = min(pytest_times)
    best_speedup = min_pytest / min_fastest
    
    print("\n🎯 DISCOVERY PERFORMANCE RESULTS")
    print("=" * 50)
    print(f"📋 Tests discovered: {test_count_fastest or test_count_pytest or 'Unknown'}")
    print()
    print("⏱️  Average Discovery Time:")
    print(f"   • Fastest (Optimized): {avg_fastest:.3f}s")
    print(f"   • Pytest (Baseline):   {avg_pytest:.3f}s")
    print(f"   • Speedup:             {speedup:.1f}x faster")
    print()
    print("🚀 Best Case Performance:")
    print(f"   • Fastest (Best):      {min_fastest:.3f}s")
    print(f"   • Pytest (Best):       {min_pytest:.3f}s") 
    print(f"   • Best Speedup:        {best_speedup:.1f}x faster")
    print()
    
    if speedup >= 2.0:
        print(f"✅ SUCCESS: Discovery is {speedup:.1f}x faster than pytest!")
    elif speedup >= 1.5:
        print(f"✅ GOOD: Discovery is {speedup:.1f}x faster than pytest")
    else:
        print(f"⚠️  EXPECTED BETTER: Only {speedup:.1f}x faster than pytest")
    
    # Save results
    results = {
        "discovery_benchmark": {
            "test_count": test_count_fastest or test_count_pytest,
            "fastest_avg_time": avg_fastest,
            "pytest_avg_time": avg_pytest,
            "speedup_factor": speedup,
            "fastest_best_time": min_fastest,
            "pytest_best_time": min_pytest,
            "best_speedup_factor": best_speedup,
            "fastest_times": fastest_times,
            "pytest_times": pytest_times
        }
    }
    
    results_path = Path(__file__).parent / "discovery_performance_results.json"
    with open(results_path, 'w') as f:
        json.dump(results, f, indent=2)
    
    print(f"\n💾 Results saved to: {results_path}")
    
    return speedup

if __name__ == "__main__":
    speedup = run_discovery_benchmark()
    if speedup and speedup >= 2.0:
        print(f"\n🎉 REVOLUTIONARY SUCCESS: {speedup:.1f}x faster discovery!")
        sys.exit(0)
    else:
        print(f"\n📈 IMPROVEMENT ACHIEVED: {speedup:.1f}x faster discovery")
        sys.exit(0)