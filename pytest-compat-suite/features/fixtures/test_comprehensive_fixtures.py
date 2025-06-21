"""
Comprehensive test suite for the complete fixture system.
Tests all scopes, dependencies, autouse, yield fixtures, and parametrization.
"""

import pytest
import os
import tempfile


# Function-scoped fixtures
@pytest.fixture
def simple_fixture():
    """Simple function-scoped fixture"""
    return "simple_value"


@pytest.fixture
def dependent_fixture(simple_fixture):
    """Fixture that depends on another fixture"""
    return f"dependent_{simple_fixture}"


@pytest.fixture
def nested_dependent(dependent_fixture, simple_fixture):
    """Fixture with multiple dependencies"""
    return f"nested_{dependent_fixture}_{simple_fixture}"


# Class-scoped fixture
@pytest.fixture(scope="class")
def class_fixture():
    """Class-scoped fixture"""
    return {"class_data": "shared_across_class"}


# Module-scoped fixture
@pytest.fixture(scope="module")
def module_fixture():
    """Module-scoped fixture"""
    return {"module_data": "shared_across_module"}


# Session-scoped fixture
@pytest.fixture(scope="session")
def session_fixture():
    """Session-scoped fixture"""
    return {"session_data": "shared_across_session"}


# Autouse fixtures
@pytest.fixture(autouse=True)
def autouse_function_fixture():
    """Autouse fixture that runs for every test"""
    # This should run automatically
    return "autouse_active"


@pytest.fixture(scope="class", autouse=True)
def autouse_class_fixture():
    """Autouse fixture at class scope"""
    return "class_autouse_active"


# Yield fixtures with teardown
@pytest.fixture
def yield_fixture():
    """Yield fixture with setup and teardown"""
    # Setup
    resource = {"setup": True}
    yield resource
    # Teardown
    resource["teardown"] = True


@pytest.fixture
def yield_with_dependency(simple_fixture):
    """Yield fixture that depends on another fixture"""
    # Setup
    value = f"yield_{simple_fixture}"
    yield value
    # Teardown - can use the fixture value
    assert simple_fixture == "simple_value"


# Parametrized fixtures
@pytest.fixture(params=[1, 2, 3])
def parametrized_fixture(request):
    """Parametrized fixture with multiple values"""
    return request.param * 10


@pytest.fixture(params=["a", "b"], ids=["case_a", "case_b"])
def parametrized_with_ids(request):
    """Parametrized fixture with custom IDs"""
    return f"param_{request.param}"


# Built-in fixture tests
def test_tmp_path(tmp_path):
    """Test built-in tmp_path fixture"""
    assert tmp_path.exists()
    assert tmp_path.is_dir()
    
    # Create a file in tmp directory
    test_file = tmp_path / "test.txt"
    test_file.write_text("hello")
    assert test_file.read_text() == "hello"


def test_capsys(capsys):
    """Test built-in capsys fixture"""
    print("Hello stdout")
    print("Hello stderr", file=os.sys.stderr)
    
    captured = capsys.readouterr()
    assert captured.out == "Hello stdout\n"
    assert captured.err == "Hello stderr\n"


def test_monkeypatch(monkeypatch):
    """Test built-in monkeypatch fixture"""
    # Test setattr
    class MyClass:
        value = "original"
    
    monkeypatch.setattr(MyClass, "value", "patched")
    assert MyClass.value == "patched"
    
    # Test setenv
    monkeypatch.setenv("TEST_ENV", "test_value")
    assert os.environ["TEST_ENV"] == "test_value"


# Simple fixture dependency tests
def test_simple_fixture(simple_fixture):
    """Test simple fixture"""
    assert simple_fixture == "simple_value"


def test_dependent_fixture(dependent_fixture):
    """Test fixture with dependency"""
    assert dependent_fixture == "dependent_simple_value"


def test_nested_dependencies(nested_dependent):
    """Test fixture with nested dependencies"""
    assert nested_dependent == "nested_dependent_simple_value_simple_value"


# Scope tests
class TestClassScope:
    """Test class-scoped fixtures"""
    
    def test_class_fixture_1(self, class_fixture):
        """First test using class fixture"""
        assert class_fixture["class_data"] == "shared_across_class"
        # Modify the fixture
        class_fixture["test1"] = True
    
    def test_class_fixture_2(self, class_fixture):
        """Second test should see modifications from first test"""
        assert class_fixture["class_data"] == "shared_across_class"
        # This should be present if fixture is properly shared
        assert class_fixture.get("test1") == True


def test_module_fixture(module_fixture):
    """Test module-scoped fixture"""
    assert module_fixture["module_data"] == "shared_across_module"
    module_fixture["test_added"] = True


def test_module_fixture_persistence(module_fixture):
    """Test that module fixture persists across tests"""
    assert module_fixture["module_data"] == "shared_across_module"
    # This should be present if fixture is properly shared
    assert module_fixture.get("test_added") == True


def test_session_fixture(session_fixture):
    """Test session-scoped fixture"""
    assert session_fixture["session_data"] == "shared_across_session"


# Yield fixture tests
def test_yield_fixture(yield_fixture):
    """Test yield fixture with setup/teardown"""
    assert yield_fixture["setup"] == True
    assert "teardown" not in yield_fixture  # Teardown hasn't run yet


def test_yield_with_dependency(yield_with_dependency):
    """Test yield fixture that depends on another fixture"""
    assert yield_with_dependency == "yield_simple_value"


# Parametrized fixture tests
def test_parametrized_fixture(parametrized_fixture):
    """Test parametrized fixture - will run 3 times"""
    assert parametrized_fixture in [10, 20, 30]


def test_parametrized_with_ids(parametrized_with_ids):
    """Test parametrized fixture with IDs - will run 2 times"""
    assert parametrized_with_ids in ["param_a", "param_b"]


# Complex fixture combination tests
def test_multiple_fixtures(simple_fixture, dependent_fixture, tmp_path):
    """Test using multiple fixtures together"""
    assert simple_fixture == "simple_value"
    assert dependent_fixture == "dependent_simple_value"
    assert tmp_path.exists()


def test_all_scopes(simple_fixture, class_fixture, module_fixture, session_fixture):
    """Test using fixtures from all scopes"""
    assert simple_fixture == "simple_value"
    assert class_fixture["class_data"] == "shared_across_class"
    assert module_fixture["module_data"] == "shared_across_module"
    assert session_fixture["session_data"] == "shared_across_session"


# Request object tests
@pytest.fixture
def fixture_with_request(request):
    """Fixture that uses the request object"""
    return {
        "node_id": request.node.nodeid,
        "fixture_name": request.fixturename,
        "scope": request.scope,
    }


def test_request_object(fixture_with_request):
    """Test fixture request object"""
    assert "node_id" in fixture_with_request
    assert fixture_with_request["fixture_name"] == "fixture_with_request"
    assert fixture_with_request["scope"] == "function"


# Fixture finalization tests
@pytest.fixture
def fixture_with_finalizer(request):
    """Fixture with finalizer"""
    data = {"finalized": False}
    
    def finalizer():
        data["finalized"] = True
    
    request.addfinalizer(finalizer)
    return data


def test_finalizer(fixture_with_finalizer):
    """Test fixture finalizer"""
    assert fixture_with_finalizer["finalized"] == False
    # Finalizer will run after this test


# Error handling tests
@pytest.fixture
def failing_fixture():
    """Fixture that fails during setup"""
    raise ValueError("Fixture setup failed")


@pytest.mark.xfail(reason="Fixture intentionally fails")
def test_failing_fixture(failing_fixture):
    """Test handling of failing fixtures"""
    # This test should be marked as xfail
    pass


# Circular dependency detection (should fail)
@pytest.fixture
def circular_a(circular_b):
    return "a"


@pytest.fixture
def circular_b(circular_a):
    return "b"


@pytest.mark.xfail(reason="Circular fixture dependency")
def test_circular_dependency(circular_a):
    """Test circular dependency detection"""
    # This should fail with circular dependency error
    pass