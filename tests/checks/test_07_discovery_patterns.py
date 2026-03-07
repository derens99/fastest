"""Check 7: Discovery patterns — what gets discovered vs what doesn't."""


# Should be discovered: test_ prefix functions
def test_discovered_function():
    assert True


# Should NOT be discovered: no test_ prefix
def helper_function():
    return 42


def check_something():
    """Not a test — no test_ prefix."""
    return True


# Should be discovered: Test prefix class
class TestDiscoveredClass:
    def test_method(self):
        assert True

    def helper_method(self):
        """Not a test — no test_ prefix."""
        return True


# Should NOT be discovered: no Test prefix
class MyHelper:
    def test_should_not_run(self):
        raise RuntimeError("should not be discovered")


# Underscore-only function name
def test_(self=None):
    """Minimal valid test name."""
    assert True


# Long test name
def test_this_is_a_very_long_test_name_that_should_still_work_correctly():
    assert True


# Test with default arguments
def test_with_defaults(x=1, y=2):
    assert x + y == 3


# Test with *args/**kwargs
def test_with_star_args(*args, **kwargs):
    assert True
