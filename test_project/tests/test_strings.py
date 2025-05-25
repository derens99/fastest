"""String manipulation tests"""

def test_concatenation():
    assert "hello" + " " + "world" == "hello world"

def test_upper():
    assert "python".upper() == "PYTHON"

def test_split():
    parts = "a,b,c".split(",")
    assert len(parts) == 3
    assert parts[1] == "b"