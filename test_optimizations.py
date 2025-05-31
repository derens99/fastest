#!/usr/bin/env python3
"""Quick test of the core optimizations we implemented."""

import subprocess
import time
import sys

def run_command(cmd):
    """Run a command and measure timing."""
    start = time.time()
    try:
        result = subprocess.run(cmd, shell=True, capture_output=True, text=True, timeout=30)
        end = time.time()
        return end - start, result.returncode == 0, result.stdout, result.stderr
    except subprocess.TimeoutExpired:
        return 30.0, False, "", "Timeout"

def main():
    print("🚀 Testing Core Optimizations")
    print("=" * 50)
    
    # Test discovery performance
    print("\n📊 Discovery Performance Test")
    print("-" * 30)
    
    discovery_cmd = "./target/release/fastest discover --format count"
    duration, success, stdout, stderr = run_command(discovery_cmd)
    
    if success:
        test_count = stdout.strip().split('\n')[-1]
        try:
            count = int(test_count)
            rate = count / duration if duration > 0 else float('inf')
            print(f"✅ Discovered {count:,} tests in {duration:.3f}s")
            print(f"⚡ Discovery rate: {rate:,.0f} tests/second")
        except ValueError:
            print(f"✅ Discovery completed in {duration:.3f}s")
    else:
        print(f"❌ Discovery failed in {duration:.3f}s")
        if stderr:
            print(f"Error: {stderr[:200]}...")
    
    # Test simple execution performance
    print("\n🎯 Simple Execution Test")
    print("-" * 30)
    
    exec_cmd = "./target/release/fastest testing_files/test_simple_working.py::test_basic_math"
    duration, success, stdout, stderr = run_command(exec_cmd)
    
    print(f"{'✅' if success else '❌'} Simple test execution: {duration:.3f}s")
    
    # Compare with baseline
    print("\n📈 Optimization Impact Analysis")
    print("-" * 30)
    
    # Discovery rate analysis
    if 'count' in locals():
        if rate > 30000:
            print("🚀 EXCELLENT: Discovery rate > 30k tests/sec")
        elif rate > 20000:
            print("✅ GOOD: Discovery rate > 20k tests/sec")
        elif rate > 10000:
            print("⚠️  OK: Discovery rate > 10k tests/sec")
        else:
            print("❌ SLOW: Discovery rate < 10k tests/sec")
    
    # Overall assessment
    print(f"\n🎯 Overall Performance Assessment")
    print(f"Discovery latency: {duration:.3f}s")
    if duration < 0.1:
        print("🚀 ULTRA-FAST: Sub-100ms discovery")
    elif duration < 0.5:
        print("⚡ FAST: Sub-500ms discovery")
    elif duration < 1.0:
        print("✅ GOOD: Sub-1s discovery")
    else:
        print("⚠️  NEEDS IMPROVEMENT: >1s discovery")

if __name__ == "__main__":
    main()