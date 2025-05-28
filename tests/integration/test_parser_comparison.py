"""Comprehensive test file to compare parser performance and accuracy."""
import pytest
import asyncio
from typing import List, Dict, Any

# Test basic functions
def test_simple_function():
    """A simple test function."""
    assert 1 + 1 == 2

def test_with_fixtures(tmp_path, capsys):
    """Test with built-in fixtures."""
    print("Hello")
    assert tmp_path.exists()

# Test async functions
async def test_async_function():
    """An async test function."""
    await asyncio.sleep(0.01)
    assert True

# Test parametrized tests
@pytest.mark.parametrize("x,y,expected", [
    (1, 2, 3),
    (2, 3, 5),
    (5, 5, 10),
])
def test_parametrized(x, y, expected):
    """Test with parametrization."""
    assert x + y == expected

# Test with multiple decorators

# Test fixtures
@pytest.fixture
def simple_fixture():
    """A simple fixture."""
    return 42

@pytest.fixture(scope="module")
def module_fixture():
    """Module-scoped fixture."""
    return "module_data"

@pytest.fixture(scope="session", autouse=True)
def session_setup():
    """Session-scoped autouse fixture."""
    print("Setting up session")
    yield
    print("Tearing down session")

@pytest.fixture(params=[1, 2, 3])
def parametrized_fixture(request):
    """Parametrized fixture."""
    return request.param * 10

# Test with custom fixtures
def test_with_custom_fixture(simple_fixture):
    """Test using custom fixture."""
    assert simple_fixture == 42

def test_with_multiple_fixtures(simple_fixture, module_fixture, parametrized_fixture):
    """Test with multiple fixtures."""
    assert simple_fixture == 42
    assert module_fixture == "module_data"
    assert parametrized_fixture in [10, 20, 30]

# Test classes
class TestSimpleClass:
    """Simple test class."""
    
    def test_method_1(self):
        """First method."""
        assert True
    
    def test_method_2(self, simple_fixture):
        """Method with fixture."""
        assert simple_fixture == 42

class TestComplexClass:
    """Complex test class with fixtures."""
    
    def setup_method(self):
        """Setup method called before each test."""
        self.value = 100
    
    @pytest.fixture
    def class_fixture(self):
        """Class-level fixture."""
        return self.value * 2
    
    def test_with_class_fixture(self):
        """Test using class fixture."""
        self.setup_method()  # Manual setup for now
        class_fixture = self.value * 2
        assert class_fixture == 200
    
    @pytest.mark.parametrize("x", [1, 2, 3])
    def test_parametrized_in_class(self, x):
        """Parametrized test in class."""
        self.setup_method()  # Manual setup for now
        assert x * self.value > 0

# Edge cases
def test_function_with_unusual_name_test():
    """Function with 'test' in middle of name."""
    pass

def not_a_test_function():
    """This should not be detected as a test."""
    pass

def test_with_nested_function():
    """Test with nested function definition."""
    def inner_test():
        return 42
    assert inner_test() == 42

# Complex decorators
@pytest.mark.parametrize("a,b", [
    pytest.param(1, 2, id="first"),
    pytest.param(3, 4, id="second", marks=pytest.mark.xfail),
])
def test_complex_parametrize(a, b):
    """Test with complex parametrization."""
    assert a < b

# Type annotations
def test_with_type_annotations(x: int = 5) -> None:
    """Test with type annotations."""
    assert isinstance(x, int)

async def test_async_with_fixtures(tmp_path: str, capsys) -> None:
    """Async test with type-annotated fixtures."""
    await asyncio.sleep(0.01)
    assert tmp_path.exists()

# Multiline decorators
@pytest.mark.parametrize(
    "input,expected",
    [
        ("hello", 5),
        ("world", 5),
        ("python", 6),
    ]
)
def test_multiline_decorator(input, expected):
    """Test with multiline decorator."""
    assert len(input) == expected

# Generator fixture
@pytest.fixture
def generator_fixture():
    """Fixture using generator for setup/teardown."""
    print("Setup")
    yield 123
    print("Teardown")

def test_with_generator_fixture(generator_fixture):
    """Test using generator fixture."""
    assert generator_fixture == 123

# Unicode and special characters
def test_unicode_αβγ():
    """Test with unicode in name."""
    assert True

def test_special_chars_123():
    """Test with numbers in name."""
    assert True

# Empty test
def test_empty():
    pass

# Test with docstring variations
def test_single_line_docstring(): """Single line docstring."""

def test_no_docstring():
    assert True

def test_multiline_docstring():
    """
    This is a
    multiline
    docstring.
    """
    assert True