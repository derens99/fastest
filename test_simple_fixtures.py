"""Test simple fixture functionality."""
import pytest

# Simple function fixture
@pytest.fixture
def simple_value():
    """Simple fixture that returns a value."""
    return 42

# Built-in fixture test
def test_tmp_path(tmp_path):
    """Test built-in tmp_path fixture."""
    assert tmp_path.exists()
    test_file = tmp_path / "test.txt"
    test_file.write_text("hello")
    assert test_file.read_text() == "hello"

def test_capsys(capsys):
    """Test built-in capsys fixture."""
    print("Hello, World!")
    captured = capsys.readouterr()
    assert captured.out == "Hello, World!\n"

def test_simple_fixture(simple_value):
    """Test simple pytest fixture."""
    assert simple_value == 42

def test_multiple_fixtures(simple_value, tmp_path):
    """Test multiple fixtures in one test."""
    assert simple_value == 42
    assert tmp_path.exists()

# Non-decorated fixture (just a function)
def regular_function():
    return 100

# Test with a non-fixture dependency (should fail)
def test_non_fixture(regular_function):
    """This should fail - regular_function is not a fixture."""
    assert regular_function == 100