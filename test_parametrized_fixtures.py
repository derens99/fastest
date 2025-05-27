"""Test parametrized fixtures support."""
import pytest

@pytest.fixture(params=[1, 2, 3])
def number_fixture(request):
    """Parametrized fixture that provides different numbers."""
    return request.param

@pytest.fixture(params=['a', 'b'])
def letter_fixture(request):
    """Parametrized fixture that provides different letters."""
    return request.param

def test_with_parametrized_fixture(number_fixture):
    """Test using a parametrized fixture - should run 3 times."""
    assert number_fixture in [1, 2, 3]
    print(f"Running with number: {number_fixture}")

def test_with_multiple_parametrized_fixtures(number_fixture, letter_fixture):
    """Test using multiple parametrized fixtures - should run 6 times (3 * 2)."""
    assert number_fixture in [1, 2, 3]
    assert letter_fixture in ['a', 'b']
    print(f"Running with number: {number_fixture}, letter: {letter_fixture}")

class TestParametrizedInClass:
    """Test class with parametrized fixtures."""
    
    def test_class_with_param_fixture(self, number_fixture):
        """Class method using parametrized fixture."""
        assert number_fixture in [1, 2, 3]
        print(f"Class test with number: {number_fixture}")

@pytest.fixture(params=[10, 20], ids=['ten', 'twenty'])
def named_fixture(request):
    """Parametrized fixture with custom IDs."""
    return request.param

def test_with_named_params(named_fixture):
    """Test using parametrized fixture with custom IDs."""
    assert named_fixture in [10, 20]
    print(f"Running with named param: {named_fixture}")

# Test indirect parametrization
@pytest.mark.parametrize("number_fixture", [100, 200], indirect=True)
def test_indirect_param(number_fixture):
    """Test indirect parametrization of fixtures."""
    # This should override the fixture's default params
    assert number_fixture in [100, 200]
    print(f"Indirect param: {number_fixture}")