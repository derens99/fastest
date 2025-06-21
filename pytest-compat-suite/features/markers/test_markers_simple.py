"""Simple marker tests for verification"""
import pytest


def test_normal_pass():
    """Normal test that passes"""
    assert True


def test_normal_fail():
    """Normal test that fails"""
    assert False, "This test fails"


@pytest.mark.skip
def test_skip_no_reason():
    """Test with skip marker"""
    assert False


@pytest.mark.skip(reason="Testing skip functionality")
def test_skip_with_reason():
    """Test with skip marker and reason"""
    assert False


@pytest.mark.xfail
def test_xfail_will_fail():
    """Test expected to fail"""
    assert False


@pytest.mark.xfail
def test_xfail_will_pass():
    """Test expected to fail but passes (XPASS)"""
    assert True


@pytest.mark.skipif(True, reason="Always skip")
def test_skipif_true():
    """Test with skipif(True)"""
    assert False


@pytest.mark.skipif(False, reason="Never skip")
def test_skipif_false():
    """Test with skipif(False)"""
    assert True