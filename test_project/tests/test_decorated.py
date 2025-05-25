"""Tests with decorators to test AST parser capabilities."""
import pytest
import asyncio


@pytest.mark.skip(reason="Testing skip decorator")
def test_skipped():
    """This test should be skipped."""
    assert False


@pytest.mark.parametrize("x,y,expected", [
    (1, 2, 3),
    (2, 3, 5),
    (10, 5, 15),
])
def test_parametrized(x, y, expected):
    """Test with parametrize decorator."""
    assert x + y == expected


@pytest.mark.timeout(5)
@pytest.mark.slow
def test_multiple_decorators():
    """Test with multiple decorators."""
    import time
    time.sleep(0.1)
    assert True


class TestDecoratedClass:
    """Test class with decorated methods."""
    
    @pytest.fixture
    def setup_data(self):
        """Fixture method."""
        return {"key": "value"}
    
    @pytest.mark.xfail(reason="Expected to fail")
    def test_expected_failure(self):
        """This test is expected to fail."""
        assert 1 == 2
    
    @pytest.mark.skipif(True, reason="Conditional skip")
    def test_conditional_skip(self):
        """Conditionally skipped test."""
        pass


@pytest.mark.asyncio
async def test_async_with_decorator():
    """Async test with decorator."""
    await asyncio.sleep(0.01)
    assert True


# Nested decorators
@pytest.mark.slow
@pytest.mark.integration
@pytest.mark.parametrize("n", [1, 2, 3])
def test_nested_decorators(n):
    """Test with nested decorators."""
    assert n > 0 