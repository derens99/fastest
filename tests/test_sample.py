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
    






@pytest.mark.xfail(reason="Testing unexpected pass")
def test_xpass():
    """This test is marked as xfail but will pass."""
    assert True


async def test_async():
    """An async test."""
    import asyncio
    await asyncio.sleep(0.01)
    assert True


 