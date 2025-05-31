"""Advanced test file with fixtures and parametrization"""
import pytest

@pytest.fixture
def sample_data():
    """Sample data fixture"""
    return {"users": ["alice", "bob", "charlie"], "count": 3}

def test_fixture_usage(sample_data):
    """Test using a fixture"""
    assert len(sample_data["users"]) == sample_data["count"]
    assert "alice" in sample_data["users"]

@pytest.mark.parametrize("input,expected", [
    (1, 2),
    (2, 4), 
    (3, 6),
    (4, 8),
])
def test_parametrized(input, expected):
    """Parametrized test"""
    assert input * 2 == expected

@pytest.mark.parametrize("text,length", [
    ("hello", 5),
    ("world", 5),
    ("pytest", 6),
    ("fastest", 7),
])
def test_string_lengths(text, length):
    """Test string lengths"""
    assert len(text) == length

def test_exception_handling():
    """Test exception handling"""
    with pytest.raises(ZeroDivisionError):
        result = 1 / 0
    
    with pytest.raises(KeyError):
        data = {}
        value = data["missing_key"]