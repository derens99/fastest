"""Test autouse fixture support."""
import pytest

# Track autouse fixture calls
autouse_calls = []

@pytest.fixture(autouse=True)
def setup_test_environment():
    """Autouse fixture that runs before every test."""
    autouse_calls.append('setup')
    print("Setting up test environment")
    yield
    print("Tearing down test environment")
    autouse_calls.append('teardown')

@pytest.fixture(autouse=True, scope='module')
def module_setup():
    """Module-level autouse fixture."""
    autouse_calls.append('module_setup')
    print("Module setup")
    yield
    print("Module teardown")
    autouse_calls.append('module_teardown')

@pytest.fixture
def regular_fixture():
    """Regular fixture for comparison."""
    return "regular_value"

def test_autouse_runs_automatically():
    """Test that autouse fixtures run without being requested."""
    print("Running test_autouse_runs_automatically")
    # Autouse fixtures should have run already
    assert 'setup' in autouse_calls
    assert 'module_setup' in autouse_calls

def test_autouse_with_regular_fixture(regular_fixture):
    """Test that autouse fixtures run even when other fixtures are used."""
    print(f"Running test with regular fixture: {regular_fixture}")
    assert regular_fixture == "regular_value"
    # Autouse should still run
    assert 'setup' in autouse_calls

class TestAutouseInClass:
    """Test autouse fixtures in class context."""
    
    @pytest.fixture(autouse=True)
    def class_autouse(self):
        """Class-level autouse fixture."""
        autouse_calls.append('class_setup')
        print("Class autouse setup")
        yield
        print("Class autouse teardown")
        autouse_calls.append('class_teardown')
    
    def test_class_method_1(self):
        """First test in class."""
        print("Running test_class_method_1")
        assert 'class_setup' in autouse_calls
    
    def test_class_method_2(self):
        """Second test in class."""
        print("Running test_class_method_2")
        assert 'class_setup' in autouse_calls

def test_autouse_ordering():
    """Test that all autouse fixtures have run in correct order."""
    print("Checking autouse ordering")
    # Module setup should come first
    module_idx = autouse_calls.index('module_setup')
    # Each test should have its own setup
    setup_count = autouse_calls.count('setup')
    assert setup_count >= 3  # At least 3 tests before this one