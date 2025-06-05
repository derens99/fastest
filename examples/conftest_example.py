"""Example conftest.py plugin for Fastest

This demonstrates how to write pytest-compatible plugins for Fastest.
"""

import time
from typing import List, Optional


# Hook: Modify test collection
def pytest_collection_modifyitems(items: List):
    """Modify collected test items.
    
    This hook is called after test collection and can be used to:
    - Filter tests
    - Reorder tests
    - Add markers
    """
    # Example: Sort tests alphabetically
    items.sort(key=lambda item: item.name)
    
    # Example: Add marker to slow tests
    for item in items:
        if "slow" in item.name.lower():
            item.add_marker("slow")


# Hook: Test session start
def pytest_sessionstart(session):
    """Called at the start of a test session."""
    print("\n🚀 Starting Fastest test session!")
    session.start_time = time.time()


# Hook: Test session finish
def pytest_sessionfinish(session, exitstatus):
    """Called at the end of a test session."""
    duration = time.time() - getattr(session, 'start_time', time.time())
    print(f"\n✅ Test session finished in {duration:.2f}s")


# Hook: Before test setup
def pytest_runtest_setup(item):
    """Called before test setup."""
    print(f"  Setting up: {item.name}")


# Hook: After test call
def pytest_runtest_teardown(item):
    """Called after test execution."""
    print(f"  Tearing down: {item.name}")


# Hook: Customize test execution
def pytest_runtest_protocol(item):
    """Customize how tests are executed.
    
    Return True to prevent default execution.
    """
    # Example: Skip tests with specific names
    if "skip_me" in item.name:
        print(f"  Skipping test: {item.name}")
        return True
    
    # Use default execution
    return None


# Hook: Generate test report
def pytest_runtest_makereport(item, call):
    """Generate a test report after execution."""
    if call.when == "call":
        # Custom reporting logic
        if call.excinfo is None:
            print(f"  ✓ {item.name} passed")
        else:
            print(f"  ✗ {item.name} failed: {call.excinfo.type.__name__}")


# Custom fixtures
import pytest


@pytest.fixture(scope="session")
def session_data():
    """Session-scoped fixture providing shared data."""
    return {
        "api_url": "https://api.example.com",
        "timeout": 30,
        "retries": 3,
    }


@pytest.fixture
def timer():
    """Function-scoped fixture for timing tests."""
    start = time.time()
    yield
    duration = time.time() - start
    print(f"  Test took {duration:.3f}s")


@pytest.fixture(autouse=True)
def reset_environment():
    """Automatically used fixture to reset environment."""
    # Setup
    original_env = os.environ.copy()
    
    yield
    
    # Teardown - restore original environment
    os.environ.clear()
    os.environ.update(original_env)


# Custom markers
def pytest_configure(config):
    """Register custom markers."""
    config.addinivalue_line(
        "markers", "slow: marks tests as slow (deselect with '-m \"not slow\"')"
    )
    config.addinivalue_line(
        "markers", "integration: marks tests as integration tests"
    )
    config.addinivalue_line(
        "markers", "unit: marks tests as unit tests"
    )


# Test generation
def pytest_generate_tests(metafunc):
    """Generate tests dynamically.
    
    This is called for each test function to generate parameters.
    """
    # Example: Generate tests for different API versions
    if "api_version" in metafunc.fixturenames:
        metafunc.parametrize("api_version", ["v1", "v2", "v3"])
    
    # Example: Generate tests from external data
    if "test_case" in metafunc.fixturenames:
        test_cases = load_test_cases()  # Load from file/database
        metafunc.parametrize("test_case", test_cases)


def load_test_cases():
    """Load test cases from external source."""
    return [
        {"input": 1, "expected": 2},
        {"input": 2, "expected": 4},
        {"input": 3, "expected": 6},
    ]


# Plugin class example
class VerbosePlugin:
    """Example plugin class for Fastest."""
    
    def __init__(self, config):
        self.config = config
        self.test_count = 0
    
    def pytest_runtest_protocol(self, item):
        """Add verbose output for each test."""
        self.test_count += 1
        print(f"\n[Test {self.test_count}] Running: {item.nodeid}")
        
        # Continue with default protocol
        return None
    
    def pytest_sessionfinish(self):
        """Print summary at end."""
        print(f"\nTotal tests executed: {self.test_count}")


# Register the plugin
def pytest_configure(config):
    """Configure and register plugins."""
    config.pluginmanager.register(VerbosePlugin(config), "verbose_plugin")