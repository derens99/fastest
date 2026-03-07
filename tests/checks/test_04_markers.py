"""Check 4: Markers — skip, skipif, xfail, custom."""

import sys
import pytest


@pytest.mark.skip(reason="unconditionally skipped")
def test_skip_unconditional():
    assert False, "should never run"


@pytest.mark.skipif(sys.platform == "win32", reason="skip on windows")
def test_skipif_windows():
    assert True  # skipped on windows


@pytest.mark.skipif(sys.version_info < (3, 8), reason="requires 3.8+")
def test_skipif_python_version():
    assert True  # should run on 3.8+


@pytest.mark.xfail(reason="expected to fail")
def test_xfail_expected():
    assert False  # xfail — should be reported as xfail


@pytest.mark.xfail(reason="expected to fail but passes")
def test_xfail_unexpected_pass():
    assert True  # xpass — unexpected pass


@pytest.mark.slow
def test_custom_marker_slow():
    import time
    time.sleep(0.01)
    assert True


@pytest.mark.integration
def test_custom_marker_integration():
    assert True


@pytest.mark.slow
@pytest.mark.integration
def test_multiple_markers():
    assert True


def test_no_markers():
    assert True
