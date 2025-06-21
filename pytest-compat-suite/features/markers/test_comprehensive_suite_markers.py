"""
Comprehensive Test Suite - Markers
Tests for skip, xfail, skipif, and custom markers
"""

import pytest
import sys
import platform


# Skip markers
@pytest.mark.skip
def test_skip_no_reason():
    """Test that is always skipped without reason"""
    assert False  # Should never execute


@pytest.mark.skip(reason="Not implemented yet")
def test_skip_with_reason():
    """Test that is skipped with a reason"""
    assert False  # Should never execute


@pytest.mark.skip(reason="Feature temporarily disabled")
def test_skip_feature_disabled():
    """Test for disabled feature"""
    assert False  # Should never execute


# Conditional skip markers
@pytest.mark.skipif(sys.version_info < (3, 8), reason="Requires Python 3.8+")
def test_skipif_python_version():
    """Test that requires specific Python version"""
    # This uses features from Python 3.8
    assert (x := 5) == 5  # Walrus operator


@pytest.mark.skipif(platform.system() == "Windows", reason="Not supported on Windows")
def test_skipif_not_windows():
    """Test that doesn't run on Windows"""
    assert platform.system() != "Windows"


@pytest.mark.skipif(platform.system() == "Darwin", reason="Not supported on macOS")
def test_skipif_not_macos():
    """Test that doesn't run on macOS"""
    assert platform.system() != "Darwin"


@pytest.mark.skipif(platform.system() == "Linux", reason="Not supported on Linux")
def test_skipif_not_linux():
    """Test that doesn't run on Linux"""
    assert platform.system() != "Linux"


@pytest.mark.skipif(True, reason="Always skip this test")
def test_skipif_always_true():
    """Test with condition that's always True"""
    assert False  # Should never execute


@pytest.mark.skipif(False, reason="Never skip this test")
def test_skipif_always_false():
    """Test with condition that's always False"""
    assert True  # Should always execute


# XFail markers
@pytest.mark.xfail
def test_xfail_no_reason():
    """Test expected to fail without reason"""
    assert False  # This failure is expected


@pytest.mark.xfail(reason="Known bug in algorithm")
def test_xfail_with_reason():
    """Test expected to fail with reason"""
    assert 1 + 1 == 3  # Obviously wrong


@pytest.mark.xfail(reason="Flaky test")
def test_xfail_sometimes_passes():
    """Test that might pass even though marked xfail"""
    import random
    # This might pass, resulting in XPASS
    assert random.random() > 0.5


@pytest.mark.xfail(reason="Expected to pass now")
def test_xfail_but_passes():
    """Test marked as xfail but actually passes (XPASS)"""
    assert True  # This will be an XPASS


@pytest.mark.xfail(sys.platform == "win32", reason="Fails on Windows")
def test_xfail_conditional():
    """Test that's expected to fail on certain platforms"""
    assert sys.platform != "win32"


@pytest.mark.xfail(raises=ValueError)
def test_xfail_specific_exception():
    """Test expected to fail with specific exception"""
    raise ValueError("Expected error")


@pytest.mark.xfail(raises=TypeError)
def test_xfail_wrong_exception():
    """Test that fails with unexpected exception type"""
    raise ValueError("Wrong exception type")


# Custom markers
@pytest.mark.slow
def test_custom_marker_slow():
    """Test marked as slow"""
    import time
    time.sleep(0.1)
    assert True


@pytest.mark.integration
def test_custom_marker_integration():
    """Test marked as integration test"""
    # Simulate integration test
    assert True


@pytest.mark.unit
def test_custom_marker_unit():
    """Test marked as unit test"""
    assert 2 + 2 == 4


@pytest.mark.smoke
def test_custom_marker_smoke():
    """Test marked as smoke test"""
    assert True


@pytest.mark.regression
def test_custom_marker_regression():
    """Test marked as regression test"""
    assert True


# Multiple markers on same test
@pytest.mark.slow
@pytest.mark.integration
def test_multiple_custom_markers():
    """Test with multiple custom markers"""
    import time
    time.sleep(0.05)
    assert True


@pytest.mark.skip(reason="Temporarily disabled")
@pytest.mark.slow
def test_skip_and_custom_marker():
    """Test with both skip and custom markers"""
    assert False  # Should not execute


@pytest.mark.xfail(reason="Work in progress")
@pytest.mark.integration
def test_xfail_and_custom_marker():
    """Test with both xfail and custom markers"""
    assert False  # Expected to fail


# Marker with parameters
@pytest.mark.timeout(5)
def test_marker_with_parameter():
    """Test with marker that has parameters"""
    import time
    time.sleep(0.01)
    assert True


@pytest.mark.parametrize("value", [1, 2, 3])
@pytest.mark.slow
def test_parametrize_with_marker(value):
    """Parametrized test with custom marker"""
    assert value in [1, 2, 3]


# Runtime skip and xfail
def test_runtime_skip():
    """Test that skips during execution"""
    if platform.system() == "Windows":
        pytest.skip("Skipping on Windows at runtime")
    assert True


def test_runtime_skip_with_condition():
    """Test with conditional runtime skip"""
    import os
    if os.environ.get("SKIP_THIS_TEST"):
        pytest.skip("Skipped due to environment variable")
    assert True


def test_runtime_xfail():
    """Test that marks itself as xfail during execution"""
    if sys.version_info < (3, 9):
        pytest.xfail("Expected to fail on Python < 3.9")
    assert sys.version_info >= (3, 9)


# Class-based tests with markers
@pytest.mark.integration
class TestMarkedClass:
    """Entire class marked with custom marker"""
    
    def test_method_inherits_marker(self):
        """Method inherits class marker"""
        assert True
    
    @pytest.mark.slow
    def test_method_additional_marker(self):
        """Method with additional marker"""
        assert True
    
    @pytest.mark.skip(reason="Skip this specific method")
    def test_method_skip(self):
        """Method with skip marker overrides class marker"""
        assert False


# Complex marker expressions (for -m option testing)
@pytest.mark.slow
@pytest.mark.unit
def test_marker_expression_and():
    """Test for 'slow and unit' marker expression"""
    assert True


@pytest.mark.integration
def test_marker_expression_or_1():
    """Test for 'integration or unit' marker expression (integration)"""
    assert True


@pytest.mark.unit
def test_marker_expression_or_2():
    """Test for 'integration or unit' marker expression (unit)"""
    assert True


@pytest.mark.regression
def test_marker_expression_not():
    """Test for 'not slow' marker expression"""
    assert True


# Edge cases
def test_no_markers():
    """Test without any markers for comparison"""
    assert True


@pytest.mark.skip
@pytest.mark.xfail
def test_conflicting_markers():
    """Test with conflicting markers (skip should take precedence)"""
    assert False  # Should be skipped, not xfailed


@pytest.mark.skipif(True, reason="Skip condition true")
@pytest.mark.xfail(reason="Also marked xfail")
def test_skipif_and_xfail():
    """Test with both skipif and xfail (skipif should win)"""
    assert False


# Markers with special characters
@pytest.mark.feature_123
def test_marker_with_numbers():
    """Test with marker containing numbers"""
    assert True


@pytest.mark.feature_abc_def
def test_marker_with_underscores():
    """Test with marker containing underscores"""
    assert True


# Tests for marker introspection
def test_check_own_markers(request):
    """Test that can introspect its own markers"""
    # This test has no markers
    markers = request.node.iter_markers()
    marker_names = [m.name for m in markers]
    assert len(marker_names) == 0


@pytest.mark.introspection_test
@pytest.mark.slow
def test_check_multiple_markers(request):
    """Test that can see its multiple markers"""
    markers = request.node.iter_markers()
    marker_names = [m.name for m in markers]
    assert "introspection_test" in marker_names
    assert "slow" in marker_names