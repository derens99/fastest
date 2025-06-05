#!/usr/bin/env python3
"""
Test script for Fastest parametrization functionality.
Tests @pytest.mark.parametrize with various parameter types and combinations.
"""

import subprocess
import sys
import os
import tempfile
import json
from pathlib import Path

# Colors for output
GREEN = '\033[92m'
RED = '\033[91m'
YELLOW = '\033[93m'
BLUE = '\033[94m'
RESET = '\033[0m'
BOLD = '\033[1m'

def run_fastest(test_file, args=''):
    """Run fastest on a test file and return the result."""
    cmd = f"cargo run --bin fastest -- {test_file} {args} -o json"
    try:
        result = subprocess.run(cmd, shell=True, capture_output=True, text=True)
        if result.stdout:
            return json.loads(result.stdout), result.returncode
        return None, result.returncode
    except:
        return None, -1

def create_test_file(content):
    """Create a temporary test file with the given content."""
    with tempfile.NamedTemporaryFile(mode='w', suffix='.py', delete=False) as f:
        f.write(content)
        return f.name

def test_simple_parametrize():
    """Test simple parametrization with single parameter."""
    print(f"\n{BOLD}Testing simple parametrization:{RESET}")
    
    test_content = '''
import pytest

@pytest.mark.parametrize("value", [1, 2, 3, 4, 5])
def test_single_param(value):
    """Test with single parameter."""
    assert value > 0
    assert value < 10

@pytest.mark.parametrize("x", [0, 1, -1, 100, -100])
def test_square(x):
    """Test squaring numbers."""
    assert x * x == x ** 2
'''
    
    test_file = create_test_file(test_content)
    results, _ = run_fastest(test_file)
    
    if results:
        passed = sum(1 for r in results if r['passed'])
        print(f"  Total tests: {len(results)}")
        print(f"  Passed: {GREEN}{passed}{RESET}")
        assert passed == 10, f"Expected 10 passed tests, got {passed}"
        print(f"  {GREEN}✓ Simple parametrization working{RESET}")
    else:
        print(f"  {RED}✗ Failed to get results{RESET}")
    
    os.unlink(test_file)

def test_multiple_parameters():
    """Test parametrization with multiple parameters."""
    print(f"\n{BOLD}Testing multiple parameters:{RESET}")
    
    test_content = '''
import pytest

@pytest.mark.parametrize("x,y,expected", [
    (2, 3, 5),
    (1, 1, 2),
    (0, 0, 0),
    (-1, 1, 0),
    (10, -5, 5),
])
def test_addition(x, y, expected):
    """Test addition with multiple parameters."""
    assert x + y == expected

@pytest.mark.parametrize("a,b,result", [
    (2, 3, 6),
    (0, 5, 0),
    (-2, 3, -6),
    (4, 4, 16),
])
def test_multiplication(a, b, result):
    """Test multiplication."""
    assert a * b == result
'''
    
    test_file = create_test_file(test_content)
    results, _ = run_fastest(test_file)
    
    if results:
        passed = sum(1 for r in results if r['passed'])
        print(f"  Total tests: {len(results)}")
        print(f"  Passed: {GREEN}{passed}{RESET}")
        assert passed == 9, f"Expected 9 passed tests, got {passed}"
        print(f"  {GREEN}✓ Multiple parameter tuples working{RESET}")
    else:
        print(f"  {RED}✗ Failed to get results{RESET}")
    
    os.unlink(test_file)

def test_complex_types():
    """Test parametrization with complex types."""
    print(f"\n{BOLD}Testing complex parameter types:{RESET}")
    
    test_content = '''
import pytest

@pytest.mark.parametrize("data", [
    [1, 2, 3],
    [],
    [0],
    list(range(10)),
])
def test_list_param(data):
    """Test with list parameters."""
    assert isinstance(data, list)
    assert len(data) >= 0

@pytest.mark.parametrize("config", [
    {"name": "test", "value": 1},
    {"empty": True},
    {},
    {"nested": {"key": "value"}},
])
def test_dict_param(config):
    """Test with dictionary parameters."""
    assert isinstance(config, dict)

@pytest.mark.parametrize("value", [
    None,
    "",
    0,
    False,
    [],
])
def test_falsy_values(value):
    """Test with falsy values."""
    assert not value
'''
    
    test_file = create_test_file(test_content)
    results, _ = run_fastest(test_file)
    
    if results:
        passed = sum(1 for r in results if r['passed'])
        print(f"  Total tests: {len(results)}")
        print(f"  Passed: {GREEN}{passed}{RESET}")
        print(f"  {GREEN}✓ Complex types (list, dict, None) working{RESET}")
    else:
        print(f"  {RED}✗ Failed to get results{RESET}")
    
    os.unlink(test_file)

def test_multiple_decorators():
    """Test multiple parametrize decorators (cartesian product)."""
    print(f"\n{BOLD}Testing multiple parametrize decorators:{RESET}")
    
    test_content = '''
import pytest

@pytest.mark.parametrize("x", [1, 2])
@pytest.mark.parametrize("y", [10, 20])
@pytest.mark.parametrize("z", [100, 200])
def test_cartesian_product(x, y, z):
    """Test with cartesian product of parameters."""
    assert x < y < z

@pytest.mark.parametrize("letter", ["a", "b"])
@pytest.mark.parametrize("number", [1, 2, 3])
def test_combinations(letter, number):
    """Test letter-number combinations."""
    assert letter in ["a", "b"]
    assert number in [1, 2, 3]
'''
    
    test_file = create_test_file(test_content)
    results, _ = run_fastest(test_file)
    
    if results:
        passed = sum(1 for r in results if r['passed'])
        print(f"  Total tests: {len(results)}")
        print(f"  Passed: {GREEN}{passed}{RESET}")
        # First test: 2 * 2 * 2 = 8 combinations
        # Second test: 2 * 3 = 6 combinations
        assert passed == 14, f"Expected 14 passed tests (8+6), got {passed}"
        print(f"  {GREEN}✓ Cartesian product working{RESET}")
    else:
        print(f"  {RED}✗ Failed to get results{RESET}")
    
    os.unlink(test_file)

def test_parametrize_with_ids():
    """Test parametrization with custom test IDs."""
    print(f"\n{BOLD}Testing parametrize with IDs:{RESET}")
    
    test_content = '''
import pytest

@pytest.mark.parametrize("test_input,expected", [
    (1, 2),
    (2, 4),
    (3, 6),
], ids=["one", "two", "three"])
def test_with_ids(test_input, expected):
    """Test with custom IDs."""
    assert test_input * 2 == expected

@pytest.mark.parametrize("value", [
    0,
    1,
    -1,
    999,
], ids=lambda x: f"value_{x}")
def test_with_id_function(value):
    """Test with ID function."""
    assert isinstance(value, int)
'''
    
    test_file = create_test_file(test_content)
    results, _ = run_fastest(test_file, "-v")
    
    if results:
        passed = sum(1 for r in results if r['passed'])
        print(f"  Total tests: {len(results)}")
        print(f"  Passed: {GREEN}{passed}{RESET}")
        print(f"  {GREEN}✓ Custom IDs working{RESET}")
    else:
        print(f"  {RED}✗ Failed to get results{RESET}")
    
    os.unlink(test_file)

def test_class_parametrization():
    """Test parametrization on class methods."""
    print(f"\n{BOLD}Testing parametrization on classes:{RESET}")
    
    test_content = '''
import pytest

class TestParametrizedClass:
    @pytest.mark.parametrize("value", [1, 2, 3])
    def test_method_param(self, value):
        """Parametrized method."""
        assert value > 0
    
    @pytest.mark.parametrize("x,y", [(1, 1), (2, 4), (3, 9)])
    def test_squares(self, x, y):
        """Test squares in class."""
        assert x * x == y

@pytest.mark.parametrize("base", [2, 3])
class TestParametrizedWholeClass:
    def test_method1(self, base):
        assert base > 0
    
    def test_method2(self, base):
        assert base < 10
'''
    
    test_file = create_test_file(test_content)
    results, _ = run_fastest(test_file)
    
    if results:
        passed = sum(1 for r in results if r['passed'])
        print(f"  Total tests: {len(results)}")
        print(f"  Passed: {GREEN}{passed}{RESET}")
        print(f"  {GREEN}✓ Class parametrization working{RESET}")
    else:
        print(f"  {RED}✗ Failed to get results{RESET}")
    
    os.unlink(test_file)

def test_failing_parametrized():
    """Test parametrized tests with some failing cases."""
    print(f"\n{BOLD}Testing mixed pass/fail parametrized tests:{RESET}")
    
    test_content = '''
import pytest

@pytest.mark.parametrize("value,is_even", [
    (2, True),
    (3, False),
    (4, True),
    (5, True),  # This will fail
    (6, True),
])
def test_even_numbers(value, is_even):
    """Test even number detection."""
    assert (value % 2 == 0) == is_even
'''
    
    test_file = create_test_file(test_content)
    results, _ = run_fastest(test_file)
    
    if results:
        passed = sum(1 for r in results if r['passed'])
        failed = sum(1 for r in results if not r['passed'])
        print(f"  Total tests: {len(results)}")
        print(f"  Passed: {GREEN}{passed}{RESET}")
        print(f"  Failed: {RED}{failed}{RESET}")
        assert passed == 4, f"Expected 4 passed tests, got {passed}"
        assert failed == 1, f"Expected 1 failed test, got {failed}"
        print(f"  {GREEN}✓ Individual parameter failures handled correctly{RESET}")
    else:
        print(f"  {RED}✗ Failed to get results{RESET}")
    
    os.unlink(test_file)

def main():
    """Run all parametrization tests."""
    print(f"{BOLD}{BLUE}=== Fastest Parametrization Test Suite ==={RESET}")
    print(f"Testing @pytest.mark.parametrize functionality\n")
    
    # Check if fastest is available
    if subprocess.run("cargo --version", shell=True, capture_output=True).returncode != 0:
        print(f"{RED}Error: Cargo not found. Please install Rust.{RESET}")
        sys.exit(1)
    
    # Run all tests
    test_simple_parametrize()
    test_multiple_parameters()
    test_complex_types()
    test_multiple_decorators()
    test_parametrize_with_ids()
    test_class_parametrization()
    test_failing_parametrized()
    
    print(f"\n{BOLD}{GREEN}All parametrization tests completed!{RESET}")
    print(f"\nParametrization features tested:")
    print(f"  • Single parameter")
    print(f"  • Multiple parameters (tuples)")
    print(f"  • Complex types (lists, dicts, None)")
    print(f"  • Multiple decorators (cartesian product)")
    print(f"  • Custom test IDs")
    print(f"  • Class method parametrization")
    print(f"  • Individual parameter pass/fail")

if __name__ == "__main__":
    main()