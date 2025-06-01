#!/usr/bin/env python3
"""
Simple validation script for revolutionary optimizations
Tests the actual performance improvements without requiring special CLI flags
"""

import os
import sys
import time
import subprocess
import tempfile
import statistics
from pathlib import Path


def create_simple_tests(count: int, test_dir: str) -> str:
    """Create simple tests optimized for JIT compilation"""
    test_file = os.path.join(test_dir, f"test_simple_{count}.py")
    
    with open(test_file, 'w') as f:
        f.write('"""Simple tests for performance validation"""\n\n')
        
        for i in range(count):
            if i % 4 == 0:
                f.write(f'def test_simple_true_{i}():\n    assert True\n\n')
            elif i % 4 == 1:
                f.write(f'def test_arithmetic_{i}():\n    assert 2 + 2 == 4\n\n')
            elif i % 4 == 2:
                f.write(f'def test_comparison_{i}():\n    assert 1 == 1\n\n')
            else:
                f.write(f'def test_boolean_{i}():\n    assert not False\n\n')
    
    return test_file


def create_parallel_tests(count: int, test_dir: str) -> str:
    """Create tests for parallel execution"""
    test_file = os.path.join(test_dir, f"test_parallel_{count}.py")
    
    with open(test_file, 'w') as f:
        f.write('"""Tests for parallel execution validation"""\n\n')
        
        for i in range(count):
            f.write(f'''def test_parallel_{i}():
    """Test case {i}"""
    data = [x for x in range({i % 10 + 1})]
    assert len(data) == {i % 10 + 1}
    assert sum(data) >= 0

''')
    
    return test_file


def run_fastest(test_file: str, iterations: int = 3) -> tuple:
    """Run fastest and measure execution time"""
    times = []
    fastest_binary = "./target/release/fastest"
    
    if not os.path.exists(fastest_binary):
        print("Building fastest binary...")
        result = subprocess.run(["cargo", "build", "--release"], capture_output=True)
        if result.returncode != 0:
            print("Failed to build fastest")
            return float('inf'), ""
    
    for i in range(iterations):
        start_time = time.perf_counter()
        
        try:
            result = subprocess.run([fastest_binary, test_file], 
                                  capture_output=True, text=True, timeout=30)
            end_time = time.perf_counter()
            
            if result.returncode == 0:
                times.append(end_time - start_time)
            else:
                print(f"Fastest failed: {result.stderr}")
                return float('inf'), result.stderr
                
        except subprocess.TimeoutExpired:
            print("Fastest timed out")
            return float('inf'), "timeout"
        except Exception as e:
            print(f"Error running fastest: {e}")
            return float('inf'), str(e)
    
    if times:
        avg_time = statistics.mean(times)
        output = result.stdout if 'result' in locals() else ""
        return avg_time, output
    else:
        return float('inf'), "no successful runs"


def run_pytest(test_file: str, iterations: int = 3) -> float:
    """Run pytest and measure execution time"""
    times = []
    
    for i in range(iterations):
        start_time = time.perf_counter()
        
        try:
            result = subprocess.run(["python", "-m", "pytest", test_file, "-v", "--tb=no"], 
                                  capture_output=True, text=True, timeout=30)
            end_time = time.perf_counter()
            
            if result.returncode in [0, 1]:  # 0 = pass, 1 = failures but ran
                times.append(end_time - start_time)
            else:
                print(f"pytest failed: {result.stderr}")
                return float('inf')
                
        except subprocess.TimeoutExpired:
            print("pytest timed out")
            return float('inf')
        except Exception as e:
            print(f"Error running pytest: {e}")
            return float('inf')
    
    return statistics.mean(times) if times else float('inf')


def validate_optimizations():
    """Run validation tests for all optimizations"""
    print("🚀 Validating Revolutionary Optimizations...")
    print("=" * 50)
    
    # Create temporary directory
    with tempfile.TemporaryDirectory() as temp_dir:
        
        # Test 1: Simple tests (should trigger JIT-like optimizations)
        print("\n🔥 Testing Simple Assertions (JIT-optimized patterns)...")
        
        for count in [5, 10, 15, 20]:
            print(f"  📊 Testing {count} simple tests...")
            
            test_file = create_simple_tests(count, temp_dir)
            
            # Run fastest
            fastest_time, fastest_output = run_fastest(test_file)
            
            # Run pytest
            pytest_time = run_pytest(test_file)
            
            if fastest_time != float('inf') and pytest_time != float('inf'):
                speedup = pytest_time / fastest_time
                print(f"    ⚡ Fastest: {fastest_time:.4f}s | pytest: {pytest_time:.4f}s | Speedup: {speedup:.1f}x")
                
                # Check for optimization indicators in output
                if "NATIVE" in fastest_output or "JIT" in fastest_output:
                    print(f"    🎯 JIT compilation detected!")
                elif fastest_time < 0.1:
                    print(f"    🎯 Ultra-fast execution (likely optimized)")
                
            else:
                print(f"    ❌ Failed - Fastest: {fastest_time}s, pytest: {pytest_time}s")
        
        # Test 2: Parallel tests (should trigger work-stealing)
        print("\n⚡ Testing Parallel Execution (SIMD/Work-stealing)...")
        
        for count in [25, 50, 100]:
            print(f"  📊 Testing {count} parallel tests...")
            
            test_file = create_parallel_tests(count, temp_dir)
            
            fastest_time, fastest_output = run_fastest(test_file)
            pytest_time = run_pytest(test_file)
            
            if fastest_time != float('inf') and pytest_time != float('inf'):
                speedup = pytest_time / fastest_time
                print(f"    ⚡ Fastest: {fastest_time:.4f}s | pytest: {pytest_time:.4f}s | Speedup: {speedup:.1f}x")
                
                # Check for parallel execution indicators
                if "SIMD" in fastest_output or "parallel" in fastest_output.lower():
                    print(f"    🎯 SIMD/Parallel execution detected!")
                elif speedup > 2.0:
                    print(f"    🎯 Significant speedup achieved!")
                
            else:
                print(f"    ❌ Failed - Fastest: {fastest_time}s, pytest: {pytest_time}s")
        
        print("\n" + "=" * 50)
        print("✅ Optimization validation complete!")
        
        # Test existing test files
        print("\n🧪 Testing existing test files...")
        
        test_files = [
            "testing_files/test_jit_simple.py",
            "testing_files/test_simd_parallel.py",
        ]
        
        for test_file in test_files:
            if os.path.exists(test_file):
                print(f"  📊 Testing {test_file}...")
                
                fastest_time, fastest_output = run_fastest(test_file, iterations=1)
                pytest_time = run_pytest(test_file, iterations=1)
                
                if fastest_time != float('inf') and pytest_time != float('inf'):
                    speedup = pytest_time / fastest_time
                    print(f"    ⚡ Fastest: {fastest_time:.4f}s | pytest: {pytest_time:.4f}s | Speedup: {speedup:.1f}x")
                    
                    # Analyze output for optimization patterns
                    if any(keyword in fastest_output for keyword in ["NATIVE", "JIT", "SIMD", "ZC-", "WS-"]):
                        print(f"    🎯 Advanced optimizations detected in output!")
                    
                else:
                    print(f"    ❌ Failed - Fastest: {fastest_time}s, pytest: {pytest_time}s")
            else:
                print(f"  ⚠️ Test file not found: {test_file}")


if __name__ == "__main__":
    validate_optimizations()