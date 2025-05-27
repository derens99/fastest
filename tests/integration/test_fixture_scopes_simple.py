"""Test fixture scope caching with a simpler approach."""
import pytest
import time
import os

# Use file-based tracking to handle parallel execution
TRACKING_FILE = "/tmp/fixture_scope_tracking.txt"

def record_creation(scope, name):
    """Record fixture creation to file."""
    with open(TRACKING_FILE, "a") as f:
        f.write(f"{scope}:{name}:{time.time()}\n")

def get_creation_count(scope):
    """Get creation count for a scope."""
    if not os.path.exists(TRACKING_FILE):
        return 0
    count = 0
    with open(TRACKING_FILE, "r") as f:
        for line in f:
            if line.startswith(f"{scope}:"):
                count += 1
    return count

# Clear tracking file at start
if os.path.exists(TRACKING_FILE):
    os.remove(TRACKING_FILE)

@pytest.fixture(scope='session')
def session_fixture():
    """Session-scoped fixture - should be created only once."""
    record_creation('session', 'session_fixture')
    return f"session_value_{time.time()}"

@pytest.fixture(scope='module')
def module_fixture():
    """Module-scoped fixture - should be created once per module."""
    record_creation('module', 'module_fixture')
    return f"module_value_{time.time()}"

@pytest.fixture(scope='function')
def function_fixture():
    """Function-scoped fixture - should be created for each test."""
    record_creation('function', 'function_fixture')
    return f"function_value_{time.time()}"

def test_single_process_check():
    """Check if we're running in a single process (for debugging)."""
    # This test just helps us understand the execution model
    print(f"Process ID: {os.getpid()}")
    assert True

def test_session_fixture_1(session_fixture):
    """First test using session fixture."""
    print(f"Test 1 got session fixture: {session_fixture}")
    assert session_fixture.startswith("session_value_")

def test_session_fixture_2(session_fixture):
    """Second test using session fixture."""
    print(f"Test 2 got session fixture: {session_fixture}")
    assert session_fixture.startswith("session_value_")

def test_module_fixture_1(module_fixture):
    """First test using module fixture."""
    print(f"Test 1 got module fixture: {module_fixture}")
    assert module_fixture.startswith("module_value_")

def test_module_fixture_2(module_fixture):
    """Second test using module fixture."""
    print(f"Test 2 got module fixture: {module_fixture}")
    assert module_fixture.startswith("module_value_")

def test_function_fixture_1(function_fixture):
    """First test using function fixture."""
    print(f"Test 1 got function fixture: {function_fixture}")
    assert function_fixture.startswith("function_value_")

def test_function_fixture_2(function_fixture):
    """Second test using function fixture."""
    print(f"Test 2 got function fixture: {function_fixture}")
    assert function_fixture.startswith("function_value_")