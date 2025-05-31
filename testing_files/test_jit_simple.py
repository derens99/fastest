"""Simple tests optimized for native JIT compilation"""

def test_simple_true():
    assert True

def test_simple_false():
    assert not False

def test_arithmetic_basic():
    assert 2 + 2 == 4

def test_arithmetic_multiplication():
    assert 3 * 4 == 12

def test_comparison_equal():
    assert 1 == 1

def test_comparison_not_equal():
    assert 5 != 3

def test_boolean_and():
    assert True and True

def test_boolean_or():
    assert True or False

def test_arithmetic_complex():
    assert (10 + 5) * 2 == 30

def test_simple_assertion():
    x = 42
    assert x == 42