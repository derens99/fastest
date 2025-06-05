"""Test file demonstrating all marker functionality"""
import pytest
import sys


@pytest.mark.skip
def test_basic_skip():
    """This test should be skipped without a reason"""
    assert False, "This should not run"


@pytest.mark.skip(reason="This feature is not implemented yet")
def test_skip_with_reason():
    """This test should be skipped with a reason"""
    assert False, "This should not run"


def test_runtime_skip():
    """This test uses pytest.skip() at runtime"""
    if True:  # Some condition
        pytest.skip("Skipping at runtime")
    assert False, "This should not run"


@pytest.mark.skipif(sys.platform == "win32", reason="Does not work on Windows")
def test_skipif_windows():
    """This test is skipped on Windows"""
    assert True


@pytest.mark.skipif(sys.platform == "darwin", reason="Does not work on macOS")
def test_skipif_macos():
    """This test is skipped on macOS"""
    assert True


@pytest.mark.skipif(sys.platform == "linux", reason="Does not work on Linux")
def test_skipif_linux():
    """This test is skipped on Linux"""
    assert True


@pytest.mark.skipif(sys.version_info < (3, 9), reason="Requires Python 3.9+")
def test_skipif_python_version():
    """This test requires Python 3.9 or higher"""
    assert True


@pytest.mark.skipif(True, reason="Always skip this one")
def test_skipif_always():
    """This test is always skipped"""
    assert False, "This should not run"


@pytest.mark.skipif(False, reason="Never skip this one")
def test_skipif_never():
    """This test is never skipped"""
    assert True


@pytest.mark.xfail
def test_basic_xfail():
    """This test is expected to fail"""
    assert False, "This is expected to fail"


@pytest.mark.xfail(reason="Known bug in feature X")
def test_xfail_with_reason():
    """This test is expected to fail with a reason"""
    assert 1 == 2, "Math is broken"


@pytest.mark.xfail
def test_xpass():
    """This test is expected to fail but will pass (XPASS)"""
    assert True  # This will cause an XPASS


def test_runtime_xfail():
    """This test uses pytest.xfail() at runtime"""
    if True:  # Some condition
        pytest.xfail("Expected to fail due to condition")
    assert False


@pytest.mark.xfail(strict=True)
def test_xfail_strict():
    """Strict xfail - XPASS will be considered a failure"""
    assert True  # This will fail the test suite due to strict=True


# Combining markers
@pytest.mark.skip
@pytest.mark.xfail
def test_multiple_markers():
    """Test with multiple markers - skip takes precedence"""
    assert False


# Custom markers
@pytest.mark.slow
def test_custom_marker_slow():
    """Test with custom marker 'slow'"""
    import time
    time.sleep(0.1)
    assert True


@pytest.mark.integration
def test_custom_marker_integration():
    """Test with custom marker 'integration'"""
    assert True


# Marker with arguments
@pytest.mark.timeout(5)
def test_with_timeout():
    """Test with timeout marker"""
    assert True


# Class-based tests with markers
@pytest.mark.skip(reason="Entire class is skipped")
class TestSkippedClass:
    def test_method1(self):
        assert False
    
    def test_method2(self):
        assert False


class TestMixedMarkers:
    @pytest.mark.skip
    def test_skip_method(self):
        assert False
    
    @pytest.mark.xfail
    def test_xfail_method(self):
        assert False
    
    def test_normal_method(self):
        assert True


# Parametrized tests with markers
@pytest.mark.parametrize("value", [1, 2, 3])
def test_parametrized_normal(value):
    assert value > 0


@pytest.mark.skip
@pytest.mark.parametrize("value", [1, 2, 3])
def test_parametrized_skip(value):
    assert False


@pytest.mark.xfail
@pytest.mark.parametrize("value", [1, 2, 3])
def test_parametrized_xfail(value):
    assert value == 2  # Only one will xpass


# Conditional skip with complex expression
@pytest.mark.skipif(
    sys.platform == "win32" and sys.version_info < (3, 8),
    reason="Does not work on Windows with Python < 3.8"
)
def test_complex_skipif():
    assert True