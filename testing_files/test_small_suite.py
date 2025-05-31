"""Small test suite to test different scenarios"""

def test_always_pass():
    """This test always passes"""
    assert True

def test_number_comparison():
    """Test number comparisons"""
    assert 5 > 3
    assert 10 >= 10
    assert 1 < 2
    assert 0 <= 0

def test_boolean_logic():
    """Test boolean operations"""
    assert True and True
    assert True or False
    assert not False
    assert bool(1) is True
    assert bool(0) is False

def test_type_checking():
    """Test type checking"""
    assert isinstance(42, int)
    assert isinstance("hello", str)
    assert isinstance([1, 2, 3], list)
    assert isinstance({"key": "value"}, dict)

def test_none_checks():
    """Test None handling"""
    value = None
    assert value is None
    assert value != 0
    assert value != False
    assert value != ""