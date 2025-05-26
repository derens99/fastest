"""Test file to demonstrate fastest performance features."""

import time
import pytest


def test_fast_1():
    """A fast test that completes quickly."""
    assert 1 + 1 == 2


def test_fast_2():
    """Another fast test."""
    assert "hello".upper() == "HELLO"


def test_fast_3():
    """Yet another fast test."""
    assert len([1, 2, 3]) == 3


@pytest.mark.skip(reason="Demonstrating skip functionality")
def test_skip_me():
    """This test should be skipped."""
    assert False


@pytest.mark.xfail
def test_expected_fail():
    """This test is expected to fail."""
    assert 1 == 2


@pytest.mark.xfail
def test_unexpected_pass():
    """This test is marked as xfail but will pass."""
    assert True


@pytest.mark.parametrize("n", range(10))
def test_parametrized_performance(n):
    """Parametrized test to show parallel execution."""
    # Simulate some work
    result = n * n
    assert result >= 0


class TestPerformanceClass:
    """Class-based tests to demonstrate batching."""
    
    def test_method_1(self):
        assert True
    
    def test_method_2(self):
        assert True
    
    def test_method_3(self):
        assert True
    
    @pytest.mark.parametrize("x,y", [(1, 2), (3, 4), (5, 6)])
    def test_parametrized_method(self, x, y):
        assert x < y


# Generate many tests to show performance
for i in range(20):
    exec(f"""
def test_generated_{i}():
    '''Generated test {i}'''
    assert {i} >= 0
""")


async def test_async_performance():
    """Async test to show async support."""
    await asyncio.sleep(0.001)
    assert True


# Import asyncio only if needed
try:
    import asyncio
except ImportError:
    pass 