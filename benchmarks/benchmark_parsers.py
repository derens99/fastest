#!/usr/bin/env python3
"""Benchmark regex vs AST parser performance."""

import subprocess
import time
import tempfile
import os
from pathlib import Path

def get_fastest_binary():
    """Get the path to the fastest binary."""
    # Check for release build first (faster)
    release_path = "./target/release/fastest"
    if os.path.exists(release_path):
        return release_path
    
    # Fall back to debug build
    debug_path = "./target/debug/fastest"
    if os.path.exists(debug_path):
        return debug_path
    
    # Try to find it anywhere in target
    for path in Path("target").rglob("fastest"):
        if path.is_file() and os.access(path, os.X_OK):
            return str(path)
    
    raise FileNotFoundError("Could not find fastest binary. Please run 'cargo build' first.")

def generate_large_test_suite(num_files=100, tests_per_file=50):
    """Generate a large test suite for benchmarking."""
    test_dir = tempfile.mkdtemp(prefix="parser_benchmark_")
    
    # Create __init__.py
    (Path(test_dir) / "__init__.py").write_text("")
    
    # Generate test files
    for i in range(num_files):
        content = f'"""Test file {i} for parser benchmarking."""\n\n'
        content += 'import pytest\nimport asyncio\n\n'
        
        # Add some decorated tests
        if i % 3 == 0:
            content += '@pytest.mark.slow\n'
            content += '@pytest.mark.integration\n'
            content += f'def test_decorated_{i}_0():\n'
            content += '    """Decorated test."""\n'
            content += '    assert True\n\n'
        
        # Track if we're in a class
        in_class = False
        class_test_count = 0
        
        # Add regular tests
        for j in range(tests_per_file):
            if j % 10 == 0:
                # Async test
                content += f'async def test_async_{i}_{j}():\n'
                content += f'    """Async test {i}_{j}."""\n'
                content += '    await asyncio.sleep(0)\n'
                content += '    assert True\n\n'
            elif j == 5:
                # Start a test class
                in_class = True
                content += f'class TestClass{i}:\n'
                content += '    """Test class."""\n\n'
            elif in_class and class_test_count < 3:
                # Add methods to class
                content += f'    def test_method_{i}_{j}(self):\n'
                content += f'        """Test method {i}_{j}."""\n'
                content += '        assert True\n\n'
                class_test_count += 1
                if class_test_count >= 3:
                    in_class = False  # Exit class after 3 methods
                    content += '\n'  # Add blank line after class
            else:
                # Regular function test
                content += f'def test_function_{i}_{j}():\n'
                content += f'    """Test function {i}_{j}."""\n'
                content += '    assert 1 + 1 == 2\n\n'
        
        # Add some parametrized tests
        content += '@pytest.mark.parametrize("x", [1, 2, 3, 4, 5])\n'
        content += f'def test_parametrized_{i}(x):\n'
        content += '    """Parametrized test."""\n'
        content += '    assert x > 0\n\n'
        
        (Path(test_dir) / f"test_file_{i}.py").write_text(content)
    
    return test_dir

def benchmark_parser(test_dir, parser_type, runs=5):
    """Benchmark a specific parser."""
    times = []
    fastest_bin = get_fastest_binary()
    
    for _ in range(runs):
        start = time.time()
        result = subprocess.run([
            fastest_bin, 
            test_dir,
            "--parser", parser_type,
            "discover",
            "--format", "count"
        ], capture_output=True, text=True)
        
        if result.returncode != 0:
            print(f"Error running {parser_type} parser: {result.stderr}")
            return None, 0
            
        elapsed = time.time() - start
        times.append(elapsed)
        
        # Extract test count from output
        test_count = int(result.stdout.strip())
    
    # Return average time and test count
    return sum(times) / len(times), test_count

def main():
    """Run the benchmark."""
    print("🔬 Parser Performance Benchmark")
    print("=" * 60)
    
    # Generate test suites of different sizes
    test_configs = [
        (10, 10),    # 100 tests
        (50, 20),    # 1,000 tests  
        (100, 50),   # 5,000 tests
    ]
    
    for num_files, tests_per_file in test_configs:
        total_tests = num_files * tests_per_file
        print(f"\n📊 Benchmarking with ~{total_tests} tests ({num_files} files)")
        print("-" * 40)
        
        # Generate test suite
        test_dir = generate_large_test_suite(num_files, tests_per_file)
        
        try:
            # Benchmark regex parser
            print("Testing regex parser...", end=" ", flush=True)
            regex_time, regex_count = benchmark_parser(test_dir, "regex", runs=3)
            if regex_time:
                print(f"✓ Found {regex_count} tests in {regex_time:.3f}s")
            
            # Benchmark AST parser  
            print("Testing AST parser...", end=" ", flush=True)
            ast_time, ast_count = benchmark_parser(test_dir, "ast", runs=3)
            if ast_time:
                print(f"✓ Found {ast_count} tests in {ast_time:.3f}s")
            
            # Compare results
            if regex_time and ast_time:
                if regex_time < ast_time:
                    speedup = ast_time / regex_time
                    print(f"\n🏁 Regex parser is {speedup:.2f}x faster")
                else:
                    speedup = regex_time / ast_time
                    print(f"\n🏁 AST parser is {speedup:.2f}x faster")
                
                if regex_count != ast_count:
                    print(f"⚠️  Test count mismatch: regex={regex_count}, ast={ast_count}")
                else:
                    print(f"✅ Both parsers found {regex_count} tests")
        
        finally:
            # Cleanup
            import shutil
            shutil.rmtree(test_dir)
    
    print("\n" + "=" * 60)
    print("🎯 Benchmark Summary:")
    print("- AST parser provides more accurate parsing")
    print("- Regex parser may be faster for simple test files")
    print("- AST parser handles complex Python syntax better")
    print("- AST parser enables future features (decorators, fixtures)")

if __name__ == "__main__":
    main() 