#!/usr/bin/env python3
"""
Debug script to find missing tests between fastest and pytest
"""

import subprocess
import re
from pathlib import Path

def get_pytest_tests():
    """Get all tests discovered by pytest"""
    cmd = ["pytest", "tests/compatibility/", "--collect-only", "-q"]
    result = subprocess.run(cmd, capture_output=True, text=True)
    
    tests = []
    for line in result.stdout.split('\n'):
        line = line.strip()
        if line and "::" in line and not line.startswith("="):
            # Extract test ID from pytest format
            test_id = line.strip()
            tests.append(test_id)
    
    return sorted(tests)

def get_fastest_tests_from_source():
    """Parse source file directly to see what tests should be found"""
    test_file = Path("tests/compatibility/test_real_world_patterns.py")
    
    with open(test_file) as f:
        content = f.read()
    
    # Find all test functions
    test_pattern = r'def (test_[^(]+)\('
    matches = re.findall(test_pattern, content)
    
    # Find all classes that contain tests
    class_pattern = r'class (Test\w+)'
    class_matches = re.findall(class_pattern, content)
    
    # Find parametrized tests with their parameters
    param_pattern = r'@pytest\.mark\.parametrize\([^)]+\)\s*def (test_[^(]+)\('
    param_matches = re.findall(param_pattern, content, re.MULTILINE | re.DOTALL)
    
    # Count parametrize arguments to estimate test count
    parametrize_counts = {}
    param_full_pattern = r'@pytest\.mark\.parametrize\(\s*["\']([^"\']+)["\'],\s*\[([^\]]+)\]\s*\)\s*def (test_[^(]+)\('
    param_full_matches = re.findall(param_full_pattern, content, re.MULTILINE | re.DOTALL)
    
    for param_name, param_values, func_name in param_full_matches:
        # Count comma-separated values, accounting for nested structures
        value_count = param_values.count(',') + 1
        parametrize_counts[func_name] = value_count
        print(f"  Parametrized function '{func_name}' has {value_count} test cases")
    
    print(f"Found {len(matches)} test functions")
    print(f"Found {len(class_matches)} test classes: {class_matches}")
    print(f"Found {len(param_matches)} parametrized test functions")
    print(f"Parametrization details: {parametrize_counts}")
    
    return matches, class_matches, param_matches, parametrize_counts

def main():
    print("🔍 DEBUG: Finding missing tests between fastest and pytest")
    print("=" * 60)
    
    # Get pytest tests
    pytest_tests = get_pytest_tests()
    print(f"📋 Pytest found {len(pytest_tests)} tests:")
    for test in pytest_tests[:5]:  # Show first 5
        print(f"  {test}")
    if len(pytest_tests) > 5:
        print(f"  ... and {len(pytest_tests) - 5} more")
    
    print()
    
    # Analyze source file
    functions, classes, param_functions, param_counts = get_fastest_tests_from_source()
    
    print("\n🔍 Analysis:")
    print(f"Functions found: {functions}")
    print(f"Classes found: {classes}")
    print(f"Parametrized functions: {param_functions}")
    
    # Calculate expected total test count
    expected_total = 0
    for func in functions:
        if func in param_counts:
            expected_total += param_counts[func]
        else:
            expected_total += 1
    
    print(f"\n📊 Expected test count calculation:")
    print(f"  Non-parametrized functions: {len(functions) - len(param_counts)}")
    print(f"  Parametrized functions: {len(param_counts)}")
    print(f"  Total parametrized test cases: {sum(param_counts.values()) if param_counts else 0}")
    print(f"  Expected total tests: {expected_total}")
    print(f"  Pytest found: {len(pytest_tests)}")
    print(f"  Difference: {len(pytest_tests) - expected_total}")
    
    # Check for specific patterns that might be missed
    print("\n🔍 Checking for potential issues:")
    
    # Count expected parametrized test cases
    pytest_test_names = [t.split("::")[-1] for t in pytest_tests]
    unique_functions = set()
    for name in pytest_test_names:
        # Remove parametrization part [...]
        if '[' in name:
            base_name = name.split('[')[0]
        else:
            base_name = name
        unique_functions.add(base_name)
    
    print(f"Unique function names from pytest: {len(unique_functions)}")
    print(f"Functions in source: {len(functions)}")
    
    if len(unique_functions) != len(functions):
        print("❌ Mismatch in function count!")
        print("Functions in source but not in pytest:")
        for func in functions:
            if func not in unique_functions:
                print(f"  - {func}")
        
        print("Functions in pytest but not in source:")
        for func in unique_functions:
            if func not in functions:
                print(f"  - {func}")

if __name__ == "__main__":
    main()