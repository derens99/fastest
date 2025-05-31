#!/usr/bin/env python3
"""
Debug script to analyze test counting discrepancies
"""

import subprocess
import re

def count_pytest_tests():
    """Count tests found by pytest"""
    result = subprocess.run([
        "pytest", "tests/compatibility/test_real_world_patterns.py", 
        "--collect-only", "-q"
    ], capture_output=True, text=True)
    
    test_lines = [line for line in result.stdout.split('\n') 
                  if line.strip() and '::' in line and not line.startswith('=')]
    
    print(f"🔍 Pytest found {len(test_lines)} tests:")
    for i, test in enumerate(test_lines, 1):
        print(f"  {i:2}. {test}")
    
    return len(test_lines)

def count_manual_tests():
    """Manually count expected tests from source"""
    
    # Read the test file
    with open("tests/compatibility/test_real_world_patterns.py") as f:
        content = f.read()
    
    # Find all test functions
    test_functions = re.findall(r'def (test_[^(]+)', content)
    async_test_functions = re.findall(r'async def (test_[^(]+)', content)
    
    print(f"\n🔍 Manual analysis:")
    print(f"Regular test functions: {len(test_functions)}")
    print(f"Async test functions: {len(async_test_functions)}")
    
    # Count parametrized decorators and their parameters
    parametrize_pattern = r'@pytest\.mark\.parametrize\([^)]+\[([^\]]+)\]'
    parametrize_matches = re.findall(parametrize_pattern, content, re.DOTALL)
    
    total_param_tests = 0
    for i, match in enumerate(parametrize_matches, 1):
        # Count commas at top level to get parameter count
        depth = 0
        comma_count = 0
        for char in match:
            if char in '([{':
                depth += 1
            elif char in ')]}':
                depth -= 1
            elif char == ',' and depth == 0:
                comma_count += 1
        
        param_count = comma_count + 1
        total_param_tests += param_count
        print(f"  Parametrize {i}: {param_count} test cases")
    
    print(f"Total parametrized test cases: {total_param_tests}")
    
    regular_tests = len(test_functions) + len(async_test_functions)
    expected_total = regular_tests + total_param_tests - len(parametrize_matches)  # Subtract base functions
    
    print(f"Expected total tests: {expected_total}")
    
    return expected_total

def main():
    print("🔍 DEBUG: Test Counting Analysis")
    print("=" * 50)
    
    pytest_count = count_pytest_tests()
    manual_count = count_manual_tests()
    
    print(f"\n📊 Summary:")
    print(f"Pytest found: {pytest_count} tests")
    print(f"Manual count: {manual_count} tests")
    print(f"Difference: {abs(pytest_count - manual_count)}")
    
    # Now test fastest
    print(f"\n🚀 Fastest result:")
    result = subprocess.run([
        "./target/release/fastest", 
        "tests/compatibility/test_real_world_patterns.py", 
        "--dry-run"
    ], capture_output=True, text=True)
    
    print(result.stdout)
    
    # Extract test count from fastest output
    fastest_match = re.search(r'Running (\d+) tests', result.stdout)
    if fastest_match:
        fastest_count = int(fastest_match.group(1))
        print(f"Fastest found: {fastest_count} tests")
        print(f"Pytest vs Fastest difference: {abs(pytest_count - fastest_count)}")

if __name__ == "__main__":
    main()