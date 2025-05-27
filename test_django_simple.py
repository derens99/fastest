"""
Simple Django-style tests that don't require Django setup
"""

class TestStringMethods:
    """Test string methods Django style"""
    
    def test_upper(self):
        assert 'foo'.upper() == 'FOO'
    
    def test_isupper(self):
        assert 'FOO'.isupper()
        assert not 'Foo'.isupper()
    
    def test_split(self):
        s = 'hello world'
        assert s.split() == ['hello', 'world']
        # check that s.split fails when the separator is not a string
        try:
            s.split(2)
            assert False, "Should have raised TypeError"
        except TypeError:
            pass


def test_django_style_function():
    """A simple function test"""
    assert 1 + 1 == 2


def test_django_imports():
    """Test we can handle common Django imports patterns"""
    import os
    import sys
    assert os.path.exists('.')
    assert sys.version_info >= (3, 8)


def test_lists():
    """Test list operations"""
    lst = [1, 2, 3]
    lst.append(4)
    assert len(lst) == 4
    assert lst[-1] == 4


def test_dicts():
    """Test dictionary operations"""
    d = {'a': 1, 'b': 2}
    d['c'] = 3
    assert len(d) == 3
    assert 'c' in d


def test_exceptions():
    """Test exception handling"""
    try:
        1 / 0
    except ZeroDivisionError:
        pass
    else:
        assert False, "Should have raised ZeroDivisionError"


def test_comprehensions():
    """Test list/dict comprehensions"""
    squares = [x**2 for x in range(5)]
    assert squares == [0, 1, 4, 9, 16]
    
    square_dict = {x: x**2 for x in range(3)}
    assert square_dict == {0: 0, 1: 1, 2: 4}


def test_generators():
    """Test generator expressions"""
    gen = (x**2 for x in range(3))
    assert list(gen) == [0, 1, 4]


def test_string_formatting():
    """Test string formatting"""
    name = "Django"
    version = 5.0
    assert f"{name} {version}" == "Django 5.0"
    assert "{} {}".format(name, version) == "Django 5.0"


def test_boolean_logic():
    """Test boolean operations"""
    assert True and True
    assert not (True and False)
    assert True or False
    assert not (False or False)