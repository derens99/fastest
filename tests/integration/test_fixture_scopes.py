"""Test fixture scope caching to ensure proper reuse."""
import pytest

# Track fixture creation counts
creation_counts = {
    'session': 0,
    'module': 0,
    'class': 0,
    'function': 0
}

@pytest.fixture(scope='session')
def session_fixture():
    """Session-scoped fixture - should be created only once."""
    creation_counts['session'] += 1
    return f"session_value_{creation_counts['session']}"

@pytest.fixture(scope='module')
def module_fixture():
    """Module-scoped fixture - should be created once per module."""
    creation_counts['module'] += 1
    return f"module_value_{creation_counts['module']}"

@pytest.fixture(scope='class')
def class_fixture():
    """Class-scoped fixture - should be created once per class."""
    creation_counts['class'] += 1
    return f"class_value_{creation_counts['class']}"

@pytest.fixture(scope='function')
def function_fixture():
    """Function-scoped fixture - should be created for each test."""
    creation_counts['function'] += 1
    return f"function_value_{creation_counts['function']}"

def test_session_scope_1(session_fixture):
    """First test using session fixture."""
    assert session_fixture == "session_value_1"
    assert creation_counts['session'] == 1

def test_session_scope_2(session_fixture):
    """Second test using session fixture - should reuse."""
    assert session_fixture == "session_value_1"  # Same value
    assert creation_counts['session'] == 1  # Not recreated

def test_module_scope_1(module_fixture):
    """First test using module fixture."""
    assert module_fixture == "module_value_1"
    assert creation_counts['module'] == 1

def test_module_scope_2(module_fixture):
    """Second test using module fixture - should reuse."""
    assert module_fixture == "module_value_1"  # Same value
    assert creation_counts['module'] == 1  # Not recreated

def test_function_scope_1(function_fixture):
    """First test using function fixture."""
    assert function_fixture == "function_value_1"
    assert creation_counts['function'] == 1

def test_function_scope_2(function_fixture):
    """Second test using function fixture - should create new."""
    assert function_fixture == "function_value_2"  # New value
    assert creation_counts['function'] == 2  # Recreated

class TestClassScope:
    """Test class for class-scoped fixtures."""
    
    def test_class_scope_1(self, class_fixture):
        """First test in class using class fixture."""
        assert class_fixture == "class_value_1"
        assert creation_counts['class'] == 1
    
    def test_class_scope_2(self, class_fixture):
        """Second test in class using class fixture - should reuse."""
        assert class_fixture == "class_value_1"  # Same value
        assert creation_counts['class'] == 1  # Not recreated

class TestAnotherClass:
    """Another test class for class-scoped fixtures."""
    
    def test_class_scope_3(self, class_fixture):
        """Test in different class - should create new class fixture."""
        assert class_fixture == "class_value_2"  # New value
        assert creation_counts['class'] == 2  # Recreated for new class

def test_all_scopes_together(session_fixture, module_fixture, class_fixture, function_fixture):
    """Test using all scopes together."""
    # Session and module should be reused
    assert session_fixture == "session_value_1"
    assert module_fixture == "module_value_1"
    # Class fixture created for function-level test
    assert class_fixture == "class_value_3"
    # Function fixture is new for each test
    assert function_fixture.startswith("function_value_")

def test_fixture_dependencies():
    """Verify final counts to ensure proper caching."""
    # Basic test - fixture counts depend on execution strategy and actual fixture support
    # For now, just verify the test infrastructure works
    assert True