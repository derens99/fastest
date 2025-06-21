import pytest

@pytest.fixture
def double(request):
    return request.param * 2

@pytest.mark.parametrize("double", [1, 2, 3], indirect=True)
def test_simple_indirect(double):
    assert double in [2, 4, 6]

# Regular parametrize for comparison
@pytest.mark.parametrize("x", [1, 2, 3])
def test_regular(x):
    assert x in [1, 2, 3]