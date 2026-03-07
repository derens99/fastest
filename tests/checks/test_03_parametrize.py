"""Check 3: Parametrized tests."""

import pytest


@pytest.mark.parametrize("x", [1, 2, 3, 4, 5])
def test_single_param(x):
    assert x > 0


@pytest.mark.parametrize("x,expected", [(1, 1), (2, 4), (3, 9), (4, 16)])
def test_square(x, expected):
    assert x**2 == expected


@pytest.mark.parametrize(
    "a,b,expected",
    [
        (1, 2, 3),
        (0, 0, 0),
        (-1, 1, 0),
        (100, 200, 300),
    ],
)
def test_addition(a, b, expected):
    assert a + b == expected


@pytest.mark.parametrize("value", [True, 1, "yes", [1]])
def test_truthy_values(value):
    assert value


@pytest.mark.parametrize("value", [False, 0, "", [], None])
def test_falsy_values(value):
    assert not value


@pytest.mark.parametrize(
    "input_str,expected",
    [
        ("hello", "HELLO"),
        ("world", "WORLD"),
        ("PyTest", "PYTEST"),
    ],
    ids=["lowercase", "lowercase2", "mixed"],
)
def test_upper_with_ids(input_str, expected):
    assert input_str.upper() == expected


# Cross-product: 3 x 2 = 6 tests
@pytest.mark.parametrize("x", [1, 2, 3])
@pytest.mark.parametrize("y", [10, 20])
def test_cross_product(x, y):
    assert x + y > 0


@pytest.mark.parametrize("n", [2, 3, 5, 7, 11, 13])
def test_primes(n):
    for i in range(2, n):
        assert n % i != 0


# Parametrize with a failing case
@pytest.mark.parametrize("x,y", [(1, 1), (2, 3)])
def test_equality_param(x, y):
    assert x == y  # second case will fail
