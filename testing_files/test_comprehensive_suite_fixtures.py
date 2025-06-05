"""
Comprehensive Test Suite - Fixtures
Tests for all fixture scopes, dependencies, yield fixtures, and autouse
"""

import pytest
import tempfile
import os


# Function-scoped fixtures (default)
@pytest.fixture
def simple_fixture():
    """Basic function-scoped fixture"""
    return {"data": "test_value"}


@pytest.fixture
def dependent_fixture(simple_fixture):
    """Fixture that depends on another fixture"""
    return {**simple_fixture, "extra": "dependent_value"}


@pytest.fixture
def nested_dependent(dependent_fixture):
    """Deeply nested fixture dependency"""
    return {**dependent_fixture, "nested": True}


# Class-scoped fixture
@pytest.fixture(scope="class")
def class_fixture():
    """Class-scoped fixture - shared across all methods in a class"""
    return {"class_data": "shared_value", "counter": 0}


# Module-scoped fixture
@pytest.fixture(scope="module")
def module_fixture():
    """Module-scoped fixture - shared across entire module"""
    return {"module_data": "module_value", "initialized": True}


# Session-scoped fixture
@pytest.fixture(scope="session")
def session_fixture():
    """Session-scoped fixture - shared across entire test session"""
    return {"session_id": "test_session_123"}


# Yield fixtures with teardown
@pytest.fixture
def yield_fixture():
    """Fixture with setup and teardown using yield"""
    # Setup
    resource = {"status": "initialized"}
    print("Setting up yield_fixture")
    
    yield resource
    
    # Teardown
    print("Tearing down yield_fixture")
    resource["status"] = "cleaned"


@pytest.fixture
def yield_with_dependency(simple_fixture):
    """Yield fixture that depends on another fixture"""
    # Setup
    data = {**simple_fixture, "yield": True}
    
    yield data
    
    # Teardown
    data["cleaned"] = True


# Autouse fixtures
@pytest.fixture(autouse=True)
def autouse_function_fixture():
    """Automatically used for every test function"""
    print("Autouse setup for function")
    yield
    print("Autouse teardown for function")


@pytest.fixture(scope="module", autouse=True)
def autouse_module_fixture():
    """Automatically used once per module"""
    print("Autouse setup for module")
    yield
    print("Autouse teardown for module")


# Parametrized fixtures
@pytest.fixture(params=[1, 2, 3])
def parametrized_fixture(request):
    """Fixture that provides multiple values"""
    return request.param * 10


@pytest.fixture(params=["a", "b"], ids=["first", "second"])
def parametrized_with_ids(request):
    """Parametrized fixture with custom IDs"""
    return request.param.upper()


# Request object usage
@pytest.fixture
def fixture_with_request(request):
    """Fixture that uses the request object"""
    return {
        "test_name": request.node.name,
        "fixture_name": request.fixturename,
        "scope": request.scope,
    }


# Fixture with finalizer
@pytest.fixture
def fixture_with_finalizer(request):
    """Fixture using request.addfinalizer"""
    data = {"finalized": False}
    
    def cleanup():
        data["finalized"] = True
        print("Finalizer called")
    
    request.addfinalizer(cleanup)
    return data


# Complex fixture scenarios
@pytest.fixture
def database_fixture():
    """Simulated database connection fixture"""
    class MockDB:
        def __init__(self):
            self.connected = True
            self.data = {}
        
        def insert(self, key, value):
            self.data[key] = value
        
        def get(self, key):
            return self.data.get(key)
        
        def close(self):
            self.connected = False
    
    db = MockDB()
    yield db
    db.close()


@pytest.fixture
def user_fixture(database_fixture):
    """Fixture that creates a user in the database"""
    user = {"id": 1, "name": "Test User"}
    database_fixture.insert("user_1", user)
    return user


# Error handling in fixtures
@pytest.fixture
def failing_fixture():
    """Fixture that fails during setup"""
    raise ValueError("Fixture setup failed")


@pytest.fixture
def failing_teardown_fixture():
    """Fixture that fails during teardown"""
    yield "data"
    raise ValueError("Fixture teardown failed")


# Tests using the fixtures
def test_simple_fixture(simple_fixture):
    """Test using basic fixture"""
    assert simple_fixture["data"] == "test_value"


def test_dependent_fixtures(nested_dependent):
    """Test using nested fixture dependencies"""
    assert nested_dependent["data"] == "test_value"
    assert nested_dependent["extra"] == "dependent_value"
    assert nested_dependent["nested"] is True


def test_yield_fixture(yield_fixture):
    """Test using yield fixture"""
    assert yield_fixture["status"] == "initialized"
    yield_fixture["modified"] = True


def test_multiple_fixtures(simple_fixture, dependent_fixture, yield_fixture):
    """Test using multiple fixtures"""
    assert simple_fixture["data"] == "test_value"
    assert dependent_fixture["extra"] == "dependent_value"
    assert yield_fixture["status"] == "initialized"


class TestClassWithFixtures:
    """Test class using class-scoped fixtures"""
    
    def test_class_fixture_1(self, class_fixture):
        """First test using class fixture"""
        assert class_fixture["class_data"] == "shared_value"
        class_fixture["counter"] += 1
    
    def test_class_fixture_2(self, class_fixture):
        """Second test using same class fixture instance"""
        # Should see the counter incremented if scope is working
        assert class_fixture["counter"] >= 1
        class_fixture["counter"] += 1
    
    def test_mixed_scopes(self, class_fixture, simple_fixture):
        """Test mixing different fixture scopes"""
        assert class_fixture["class_data"] == "shared_value"
        assert simple_fixture["data"] == "test_value"


def test_module_fixture(module_fixture):
    """Test using module-scoped fixture"""
    assert module_fixture["module_data"] == "module_value"
    assert module_fixture["initialized"] is True


def test_session_fixture(session_fixture):
    """Test using session-scoped fixture"""
    assert session_fixture["session_id"] == "test_session_123"


def test_parametrized_fixture(parametrized_fixture):
    """Test will run multiple times with different fixture values"""
    assert parametrized_fixture in [10, 20, 30]


def test_fixture_with_request(fixture_with_request):
    """Test fixture that uses request object"""
    assert "test_fixture_with_request" in fixture_with_request["test_name"]
    assert fixture_with_request["fixture_name"] == "fixture_with_request"


def test_database_operations(database_fixture, user_fixture):
    """Test complex fixture interaction"""
    assert database_fixture.connected is True
    assert database_fixture.get("user_1") == user_fixture
    
    # Add more data
    database_fixture.insert("user_2", {"id": 2, "name": "Another User"})
    assert database_fixture.get("user_2")["name"] == "Another User"


# Tests for built-in fixtures
def test_tmp_path(tmp_path):
    """Test built-in tmp_path fixture"""
    assert tmp_path.exists()
    assert tmp_path.is_dir()
    
    # Create a file in tmp directory
    test_file = tmp_path / "test.txt"
    test_file.write_text("Hello, World!")
    
    assert test_file.exists()
    assert test_file.read_text() == "Hello, World!"


def test_capsys(capsys):
    """Test built-in capsys fixture"""
    print("Hello stdout")
    print("Hello stderr", file=sys.stderr)
    
    captured = capsys.readouterr()
    assert "Hello stdout" in captured.out
    assert "Hello stderr" in captured.err
    
    # Test that capture is cleared
    print("Second output")
    captured2 = capsys.readouterr()
    assert "Second output" in captured2.out
    assert "Hello stdout" not in captured2.out


def test_monkeypatch(monkeypatch):
    """Test built-in monkeypatch fixture"""
    # Patch an attribute
    import os
    monkeypatch.setattr(os, "custom_attr", "patched_value")
    assert os.custom_attr == "patched_value"
    
    # Patch environment variable
    monkeypatch.setenv("TEST_ENV_VAR", "test_value")
    assert os.environ["TEST_ENV_VAR"] == "test_value"
    
    # Delete environment variable
    monkeypatch.setenv("TO_DELETE", "value")
    assert "TO_DELETE" in os.environ
    monkeypatch.delenv("TO_DELETE")
    assert "TO_DELETE" not in os.environ


# Fixture inheritance and override scenarios
@pytest.fixture
def base_fixture():
    """Base fixture that might be overridden"""
    return {"type": "base"}


def test_fixture_override(base_fixture):
    """Test that uses base fixture"""
    assert base_fixture["type"] == "base"


# Edge cases
def test_unused_fixture(simple_fixture):
    """Test that requests fixture but doesn't use it"""
    # This tests that fixture execution happens even if not used
    assert True


def test_no_fixtures():
    """Test without any fixtures"""
    assert 1 + 1 == 2


@pytest.fixture
def recursive_fixture_a(recursive_fixture_b):
    """Part of a circular dependency - should be detected"""
    return "a"


@pytest.fixture
def recursive_fixture_b(recursive_fixture_a):
    """Part of a circular dependency - should be detected"""
    return "b"


# This test should fail due to circular dependency
# def test_circular_dependency(recursive_fixture_a):
#     """Test that would cause circular dependency error"""
#     assert False