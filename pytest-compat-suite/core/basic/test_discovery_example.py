"""Example test file to verify test discovery"""

import pytest
import asyncio


# Simple test function
def test_simple():
    assert True


# Async test function
async def test_async():
    await asyncio.sleep(0.1)
    assert True


# Test class with methods
class TestExample:
    def test_method(self):
        assert True
    
    async def test_async_method(self):
        await asyncio.sleep(0.1)
        assert True


# Parametrized test
@pytest.mark.parametrize("value", [1, 2, 3])
def test_parametrized(value):
    assert value > 0


# Test with xfail marker
@pytest.mark.xfail
def test_expected_failure():
    assert False


# Test with fixtures
def test_with_fixtures(tmp_path, monkeypatch):
    assert tmp_path.exists()
    assert monkeypatch is not None


# Test with multiple decorators
@pytest.mark.slow
@pytest.mark.parametrize("x,y", [(1, 2), (3, 4)])
def test_multiple_decorators(x, y):
    assert x < y


# Not a test (doesn't start with test_)
def helper_function():
    return 42


# Test class that should be skipped (inherits from unittest.TestCase)
import unittest

class TestCaseExample(unittest.TestCase):
    def test_should_be_skipped(self):
        self.assertTrue(True) 