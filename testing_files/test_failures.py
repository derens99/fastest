"""Test various failure scenarios."""

def test_assertion_failure():
    """Test that fails with assertion."""
    assert 1 + 1 == 3, "Math is broken!"


def test_exception():
    """Test that raises an exception."""
    raise ValueError("Something went wrong")


def test_import_error():
    """Test that fails due to import."""
    import nonexistent_module


def test_syntax_error():
    """Test with syntax error in execution."""
    # This will cause a runtime error
    exec("invalid python syntax !!!")


def test_passes():
    """This test should pass."""
    assert True


def test_timeout():
    """Test that takes too long."""
    import time
    time.sleep(10)  # This might timeout depending on settings


class TestFailureClass:
    """Test class with failures."""
    
    def test_class_failure(self):
        """Test that fails in a class."""
        assert False, "Class test failed"
    
    def test_class_success(self):
        """Test that passes in a class."""
        assert True