def test_add():
    assert 1 + 1 == 2

def test_subtract():
    assert 5 - 3 == 2

def test_multiply():
    assert 3 * 4 == 12

def test_divide():
    assert 10 / 2 == 5

def test_string_upper():
    assert "hello".upper() == "HELLO"

def test_string_lower():
    assert "WORLD".lower() == "world"

def test_list_append():
    lst = [1, 2, 3]
    lst.append(4)
    assert lst == [1, 2, 3, 4]

def test_dict_get():
    d = {"key": "value"}
    assert d.get("key") == "value"

def test_bool_true():
    assert True

def test_bool_false():
    assert not False
