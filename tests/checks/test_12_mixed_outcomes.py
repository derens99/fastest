"""Check 12: Mixed outcomes in a single file for summary verification."""

import pytest


def test_pass_1():
    assert True


def test_pass_2():
    assert True


def test_pass_3():
    assert True


def test_fail_1():
    assert False, "failure one"


def test_fail_2():
    assert 1 == 2, "failure two"


@pytest.mark.skip(reason="skipping this one")
def test_skipped():
    assert False


@pytest.mark.xfail
def test_expected_failure():
    assert False


def test_pass_4():
    assert True


def test_error():
    raise RuntimeError("runtime error in test")
