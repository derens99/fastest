import pytest

# Simple indirect parametrization test cases

@pytest.fixture
def number_fixture(request):
    """Fixture that doubles the parameter value"""
    return request.param * 2

@pytest.fixture
def string_fixture(request):
    """Fixture that uppercases the parameter value"""
    return request.param.upper()

@pytest.fixture
def user_data(request):
    """Fixture that creates a user dict from parameter"""
    return {"name": request.param, "id": len(request.param)}

# Test with single indirect parameter
@pytest.mark.parametrize("number_fixture", [1, 2, 3], indirect=True)
def test_indirect_single(number_fixture):
    """Test with single indirect parameter"""
    assert number_fixture in [2, 4, 6]

# Test with multiple indirect parameters
@pytest.mark.parametrize(
    "number_fixture,string_fixture", 
    [(1, "hello"), (2, "world")], 
    indirect=True
)
def test_indirect_multiple(number_fixture, string_fixture):
    """Test with multiple indirect parameters"""
    assert number_fixture in [2, 4]
    assert string_fixture in ["HELLO", "WORLD"]

# Test with mixed indirect and direct parameters
@pytest.mark.parametrize(
    "number_fixture,regular_param", 
    [(1, "a"), (2, "b")], 
    indirect=["number_fixture"]  # Only number_fixture is indirect
)
def test_mixed_indirect(number_fixture, regular_param):
    """Test with mixed indirect and direct parameters"""
    assert number_fixture in [2, 4]
    assert regular_param in ["a", "b"]

# Test with all parameters indirect using indirect=True
@pytest.mark.parametrize(
    "user_data,string_fixture", 
    [("alice", "hello"), ("bob", "world")], 
    indirect=True
)
def test_all_indirect(user_data, string_fixture):
    """Test with all parameters indirect"""
    assert user_data["name"] in ["alice", "bob"]
    assert user_data["id"] in [5, 3]
    assert string_fixture in ["HELLO", "WORLD"]

# Test indirect with string parameter name
@pytest.mark.parametrize("user_data", ["charlie", "dave"], indirect=["user_data"])
def test_indirect_string_form(user_data):
    """Test indirect parameter specified as string"""
    assert user_data["name"] in ["charlie", "dave"]
    assert user_data["id"] in [7, 4]

# Edge case: indirect parameter not in fixture list
@pytest.mark.parametrize(
    "number_fixture,missing_fixture", 
    [(1, 10), (2, 20)], 
    indirect=["number_fixture"]
)
def test_indirect_with_missing(number_fixture, missing_fixture):
    """Test where indirect parameter is actually a direct parameter"""
    assert number_fixture in [2, 4]
    assert missing_fixture in [10, 20]  # Should be passed directly