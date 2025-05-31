#!/usr/bin/env python3
"""
Debug script to analyze discovery performance bottlenecks
"""

import time
import subprocess
import json
from pathlib import Path

def time_command(cmd, description):
    """Time a command and return duration and output"""
    print(f"⏱️  Timing: {description}")
    start = time.time()
    result = subprocess.run(cmd, shell=True, capture_output=True, text=True)
    duration = time.time() - start
    print(f"   Duration: {duration:.3f}s")
    print(f"   Output: {result.stdout.strip()[:100]}...")
    return duration, result

def main():
    test_dir = "tests/compatibility"
    
    print("🔍 DISCOVERY PERFORMANCE ANALYSIS")
    print("=" * 50)
    
    # Test fastest discovery only
    fastest_disc_time, fastest_result = time_command(
        f"./target/release/fastest {test_dir} --collect-only -q",
        "Fastest discovery only"
    )
    
    # Test pytest discovery only  
    pytest_disc_time, pytest_result = time_command(
        f"pytest {test_dir} --collect-only -q",
        "Pytest discovery only"
    )
    
    # Test with smaller subset
    small_test_time, small_result = time_command(
        f"./target/release/fastest {test_dir}/test_real_world_patterns.py --collect-only -q",
        "Fastest single file discovery"
    )
    
    print(f"\n📊 RESULTS:")
    print(f"Fastest full discovery:  {fastest_disc_time:.3f}s")
    print(f"Pytest full discovery:   {pytest_disc_time:.3f}s")
    print(f"Fastest single file:     {small_test_time:.3f}s")
    print(f"Ratio (fastest/pytest): {fastest_disc_time/pytest_disc_time:.2f}x")
    
    if fastest_disc_time > pytest_disc_time:
        print(f"❌ We're {fastest_disc_time/pytest_disc_time:.2f}x slower than pytest!")
        print("\n🔍 Potential bottlenecks:")
        print("- Parser initialization overhead")
        print("- Parallel processing overhead for small test suites")
        print("- File I/O patterns")
        print("- Tree-sitter fallback complexity")
    else:
        print(f"✅ We're {pytest_disc_time/fastest_disc_time:.2f}x faster than pytest!")

if __name__ == "__main__":
    main()