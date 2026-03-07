"""Check 8: Edge cases."""

import sys


def test_empty_assertion():
    """Empty assert should pass."""
    assert True


def test_multiline_assertion():
    result = (
        1 + 2 + 3 + 4 + 5
    )
    assert result == 15


def test_nested_assertions():
    data = {"users": [{"name": "alice"}, {"name": "bob"}]}
    assert data["users"][0]["name"] == "alice"
    assert len(data["users"]) == 2


def test_large_data():
    """Test with large data structures."""
    big_list = list(range(10000))
    assert len(big_list) == 10000
    assert big_list[-1] == 9999


def test_exception_message():
    """Verify exception messages work."""
    try:
        raise ValueError("custom error message")
    except ValueError as e:
        assert str(e) == "custom error message"


def test_import_stdlib():
    """Test that stdlib imports work."""
    import json
    import os
    import math
    assert math.pi > 3.14
    assert json.dumps({"a": 1}) == '{"a": 1}'
    assert os.sep in ("/", "\\")


def test_generator():
    def gen():
        yield 1
        yield 2
        yield 3
    assert list(gen()) == [1, 2, 3]


def test_context_manager():
    from contextlib import contextmanager

    @contextmanager
    def managed():
        yield 42

    with managed() as val:
        assert val == 42


def test_recursive_function():
    def factorial(n):
        if n <= 1:
            return 1
        return n * factorial(n - 1)

    assert factorial(10) == 3628800


def test_lambda():
    square = lambda x: x**2
    assert square(5) == 25


def test_walrus_operator():
    if (n := 10) > 5:
        assert n == 10


def test_fstring():
    name = "fastest"
    assert f"hello {name}" == "hello fastest"


def test_unicode_content():
    assert "cafe\u0301" != "caf\u00e9"  # different unicode representations
    assert "\u2603" == "☃"  # snowman


def test_bytes():
    assert b"hello" == b"hello"
    assert len(b"\x00\x01\x02") == 3


def test_stdout_output():
    """Test that print output is captured."""
    print("this should be captured")
    assert True


def test_stderr_output():
    """Test that stderr is captured."""
    print("stderr output", file=sys.stderr)
    assert True
