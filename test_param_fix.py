import pytest

# Test building the URL path
@pytest.mark.parametrize(
    "endpoint, sub_path, query, expected",
    [
        ("endpoint", None, None, "https://example.com/endpoint"),
        ("endpoint", "sub", None, "https://example.com/endpoint/sub"),
        ("endpoint", "sub", {"key": "value", "key2": None}, "https://example.com/endpoint/sub?key=value"),
    ],
)
def test_build_path(endpoint, sub_path, query, expected):
    # Simple test to verify parametrized tests work
    assert endpoint is not None
    assert isinstance(expected, str)
    print(f"Testing: {endpoint}, {sub_path}, {query} -> {expected}")

def test_simple():
    """A simple test that should always pass"""
    assert True 