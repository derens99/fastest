"""Simple tests that should work perfectly with Fastest."""

import pytest

def test_basic_math():
    """Test basic mathematical operations."""
    assert 2 + 2 == 4
    assert 3 * 4 == 12
    assert 10 / 2 == 5


def test_string_operations():
    """Test string operations."""
    text = "hello world"
    assert text.upper() == "HELLO WORLD"
    assert text.title() == "Hello World"
    assert "world" in text


def test_list_operations():
    """Test list operations."""
    items = [1, 2, 3, 4, 5]
    assert len(items) == 5
    assert sum(items) == 15
    assert max(items) == 5


async def test_async_function():
    """Test async functionality."""
    import asyncio
    await asyncio.sleep(0.001)  # Very short sleep
    assert True


def test_tmp_path_fixture(tmp_path):
    """Test tmp_path fixture functionality."""
    # Create a test file
    test_file = tmp_path / "example.txt"
    test_content = "Hello, Fastest!"
    
    # Write and read back
    test_file.write_text(test_content)
    assert test_file.read_text() == test_content
    assert test_file.exists()


def test_capsys_fixture(capsys):
    """Test capsys fixture functionality."""
    print("This goes to stdout")
    print("Error message", file=__import__('sys').stderr)
    
    captured = capsys.readouterr()
    assert "This goes to stdout" in captured.out
    assert "Error message" in captured.err


@pytest.mark.parametrize("number", [1, 2, 3, 4, 5])
def test_simple_parametrize(number):
    """Test simple parametrization."""
    assert number > 0
    assert isinstance(number, int)


@pytest.mark.parametrize("text,expected", [
    ("hello", 5),
    ("world", 5),
    ("test", 4),
])
def test_string_length_parametrize(text, expected):
    """Test parametrized string length."""
    assert len(text) == expected


def test_exception_handling():
    """Test that we can handle exceptions properly."""
    with pytest.raises(ValueError):
        raise ValueError("Expected error")


def test_boolean_logic():
    """Test boolean operations."""
    assert True and True
    assert not (True and False)
    assert True or False
    assert not False