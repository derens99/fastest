"""Check 1: Basic pass/fail/error tests."""


def test_pass():
    assert True


def test_fail():
    assert 1 == 2, "expected failure"


def test_arithmetic():
    assert 2 + 2 == 4


def test_string_equality():
    assert "hello" == "hello"


def test_list_equality():
    assert [1, 2, 3] == [1, 2, 3]


def test_dict_equality():
    assert {"a": 1, "b": 2} == {"a": 1, "b": 2}


def test_none_check():
    assert None is None


def test_truthy():
    assert [1]
    assert "nonempty"
    assert 42


def test_falsy_fail():
    assert [] == [1], "empty list is not [1]"


def test_exception_expected():
    try:
        1 / 0
    except ZeroDivisionError:
        pass  # expected


def test_type_error():
    """This test should ERROR (unhandled exception)."""
    raise TypeError("intentional type error")
