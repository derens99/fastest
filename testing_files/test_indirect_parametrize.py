"""Test indirect parametrization functionality"""
import pytest


# Simple test without parametrization
def test_simple():
    """Simple test to verify discovery"""
    assert True


# Fixture that will be parametrized indirectly
@pytest.fixture
def number_fixture(request):
    """Fixture that uses request.param for indirect parametrization"""
    if hasattr(request, 'param'):
        # Double the parameter value
        return request.param * 2
    return 0


# Another fixture for testing
@pytest.fixture
def string_fixture(request):
    """Fixture that converts param to string"""
    if hasattr(request, 'param'):
        return f"value_{request.param}"
    return "default"


# Test with indirect parametrization
@pytest.mark.parametrize("number_fixture", [1, 2, 3], indirect=True)
def test_indirect_single(number_fixture):
    """Test single indirect parametrization"""
    # number_fixture should be 2, 4, 6 (doubled)
    assert number_fixture in [2, 4, 6]
    assert number_fixture % 2 == 0


# Test with multiple indirect parameters
@pytest.mark.parametrize("number_fixture,string_fixture", [
    (10, "a"),
    (20, "b"),
    (30, "c"),
], indirect=True)
def test_indirect_multiple(number_fixture, string_fixture):
    """Test multiple indirect parametrization"""
    # number_fixture should be doubled (20, 40, 60)
    # string_fixture should be prefixed (value_a, value_b, value_c)
    assert number_fixture in [20, 40, 60]
    assert string_fixture.startswith("value_")


# Test with partial indirect parametrization
@pytest.mark.parametrize("number_fixture,direct_param", [
    (5, "x"),
    (10, "y"),
], indirect=["number_fixture"])
def test_indirect_partial(number_fixture, direct_param):
    """Test partial indirect parametrization"""
    # number_fixture is indirect (doubled: 10, 20)
    # direct_param is direct ("x", "y")
    assert number_fixture in [10, 20]
    assert direct_param in ["x", "y"]


# Test with existing conftest fixture
@pytest.mark.parametrize("data_source", ["file", "database", "api"], indirect=True)
def test_indirect_conftest_fixture(data_source):
    """Test indirect parametrization with conftest fixture"""
    assert data_source["type"] in ["file", "database", "api"]
    if data_source["type"] == "file":
        assert data_source["path"] == "/tmp/data.txt"
    elif data_source["type"] == "database":
        assert data_source["connection"] == "db://localhost"
    elif data_source["type"] == "api":
        assert data_source["url"] == "http://api.test.com"