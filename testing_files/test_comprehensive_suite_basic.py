"""
Comprehensive Test Suite - Basic Functionality
Tests for core test discovery, execution, and filtering
"""

import pytest
import os
import sys


# Basic test discovery patterns
def test_simple_pass():
    """Most basic test - should always pass"""
    assert True


def test_simple_fail():
    """Basic failing test"""
    assert False, "This test is expected to fail"


def test_with_assertion():
    """Test with detailed assertion"""
    expected = [1, 2, 3]
    actual = [1, 2, 4]
    assert actual == expected, f"Lists don't match: {actual} != {expected}"


def test_with_multiple_assertions():
    """Test with multiple assertion points"""
    x = 5
    assert x > 0
    assert x < 10
    assert x == 5
    assert isinstance(x, int)


# Test name patterns
def test_CamelCase():
    """Test with camelCase naming"""
    assert True


def test_with_numbers_123():
    """Test with numbers in name"""
    assert True


def test_with_special_chars_():
    """Test with underscores"""
    assert True


# Async tests
async def test_async_simple():
    """Simple async test"""
    import asyncio
    await asyncio.sleep(0.01)
    assert True


async def test_async_with_assertion():
    """Async test with assertions"""
    import asyncio
    
    async def fetch_data():
        await asyncio.sleep(0.01)
        return {"status": "ok"}
    
    result = await fetch_data()
    assert result["status"] == "ok"


# Tests with imports
def test_with_stdlib_imports():
    """Test using standard library"""
    import json
    import datetime
    import collections
    
    data = {"key": "value"}
    json_str = json.dumps(data)
    assert json.loads(json_str) == data
    
    now = datetime.datetime.now()
    assert isinstance(now, datetime.datetime)
    
    counter = collections.Counter([1, 2, 2, 3, 3, 3])
    assert counter[3] == 3


def test_with_local_imports():
    """Test with local module imports"""
    # This tests that sys.path is correctly set up
    current_dir = os.path.dirname(__file__)
    assert os.path.exists(current_dir)


# Exception testing
def test_raises_exception():
    """Test that properly raises exception"""
    with pytest.raises(ValueError):
        raise ValueError("Expected error")


def test_raises_specific_exception():
    """Test for specific exception type"""
    with pytest.raises(ZeroDivisionError):
        1 / 0


def test_exception_message():
    """Test exception with message checking"""
    with pytest.raises(ValueError, match="invalid.*value"):
        raise ValueError("invalid value provided")


# Assertion introspection tests
def test_string_comparison():
    """Test string assertion for introspection"""
    assert "hello world" == "hello worlds"


def test_list_comparison():
    """Test list assertion for introspection"""
    assert [1, 2, 3, 4] == [1, 2, 3, 5]


def test_dict_comparison():
    """Test dict assertion for introspection"""
    expected = {"a": 1, "b": 2, "c": 3}
    actual = {"a": 1, "b": 2, "c": 4}
    assert actual == expected


def test_complex_comparison():
    """Test complex nested structure comparison"""
    expected = {
        "users": [
            {"name": "Alice", "age": 30},
            {"name": "Bob", "age": 25}
        ],
        "total": 2
    }
    actual = {
        "users": [
            {"name": "Alice", "age": 30},
            {"name": "Bob", "age": 26}  # Different age
        ],
        "total": 2
    }
    assert actual == expected


# Long running tests (for timeout testing)
def test_quick_execution():
    """Test that executes quickly"""
    import time
    time.sleep(0.01)
    assert True


def test_medium_execution():
    """Test with medium execution time"""
    import time
    time.sleep(0.1)
    assert True


# Tests with print output
def test_with_stdout():
    """Test that prints to stdout"""
    print("This is stdout output")
    print("Multiple lines")
    print("of output")
    assert True


def test_with_stderr():
    """Test that prints to stderr"""
    import sys
    print("This is stderr output", file=sys.stderr)
    print("Error information", file=sys.stderr)
    assert True


def test_with_mixed_output():
    """Test with both stdout and stderr"""
    import sys
    print("Normal output")
    print("Error output", file=sys.stderr)
    print("More normal output")
    assert True


# Module-level tests
MODULE_VAR = 42


def test_module_variable():
    """Test accessing module-level variable"""
    assert MODULE_VAR == 42


def test_modify_module_variable():
    """Test modifying module-level variable"""
    global MODULE_VAR
    old_value = MODULE_VAR
    MODULE_VAR = 100
    assert MODULE_VAR == 100
    MODULE_VAR = old_value  # Restore


# Helper function for filtering tests
def helper_not_a_test():
    """This should not be discovered as a test"""
    return True


def Test_wrong_case():
    """This should not be discovered (wrong case)"""
    assert False


class NotATestClass:
    """This should not be discovered"""
    def test_method(self):
        assert False