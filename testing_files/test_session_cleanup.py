import pytest
import tempfile
import os
import time

# Global variable to track cleanup
cleanup_called = False
cleanup_time = None
test_end_time = None

@pytest.fixture(scope="session")
def session_resource():
    """Session fixture that creates a temporary file"""
    # Create a temp file
    fd, path = tempfile.mkstemp(suffix=".session")
    os.write(fd, b"session data")
    os.close(fd)
    
    # Store the path for verification
    session_resource._path = path
    
    yield path
    
    # Cleanup - should only happen after ALL tests
    global cleanup_called, cleanup_time
    cleanup_called = True
    cleanup_time = time.time()
    os.unlink(path)

def test_first(session_resource):
    """First test using session fixture"""
    assert os.path.exists(session_resource)
    assert os.path.getsize(session_resource) == 12  # "session data"

def test_second(session_resource):
    """Second test using same session fixture"""
    assert os.path.exists(session_resource)
    # Should be the same file
    assert session_resource.endswith(".session")

def test_third(session_resource):
    """Third test to ensure fixture is still alive"""
    assert os.path.exists(session_resource)
    with open(session_resource, 'rb') as f:
        assert f.read() == b"session data"

def test_cleanup_check():
    """Test that runs last to check cleanup hasn't happened yet"""
    global test_end_time, cleanup_called
    test_end_time = time.time()
    # Session cleanup should NOT have been called yet
    assert not cleanup_called, "Session fixture was cleaned up too early!"

# This test would fail if session cleanup happens too early
def test_final_verification():
    """Final test to ensure proper ordering"""
    global cleanup_called
    assert not cleanup_called, "Session cleanup happened before all tests completed"