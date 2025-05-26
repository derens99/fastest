"""Sample test file to verify fastest executor."""

import pytest


def test_simple():
    """A simple test that should pass."""
    assert 1 + 1 == 2


def test_with_output(capsys):
    """Test that produces output."""
    print("Hello from test!")
    assert True
    captured = capsys.readouterr()
    assert "Hello" in captured.out


class TestClass:
    """Test class with multiple tests."""
    
    def test_method(self):
        """Test method in a class."""
        assert "hello".upper() == "HELLO"
    
    def test_failing(self):
        """This test should fail."""
        assert False, "This test is expected to fail"


@pytest.mark.skip(reason="Testing skip functionality")
def test_skip():
    """This test should be skipped."""
    assert False


@pytest.mark.xfail(reason="Testing xfail functionality")
def test_xfail():
    """This test is expected to fail."""
    assert False


@pytest.mark.xfail(reason="Testing unexpected pass")
def test_xpass():
    """This test is marked as xfail but will pass."""
    assert True


async def test_async():
    """An async test."""
    import asyncio
    await asyncio.sleep(0.01)
    assert True


def test_exception():
    """Test that raises an exception."""
    raise ValueError("Test exception") 