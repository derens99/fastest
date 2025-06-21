"""Root conftest.py with shared fixtures"""
import pytest
import json
import time
from pathlib import Path


# Session-scoped fixture available to all tests
@pytest.fixture(scope="session")
def test_config():
    """Global test configuration"""
    return {
        "api_url": "http://localhost:8000",
        "timeout": 30,
        "retry_count": 3,
        "environment": "test"
    }


# Module-scoped fixture
@pytest.fixture(scope="module")
def api_client(test_config):
    """Simulated API client"""
    class APIClient:
        def __init__(self, config):
            self.config = config
            self.base_url = config["api_url"]
            self.session = {"token": None}
        
        def login(self, username, password):
            # Simulate login
            self.session["token"] = f"token_{username}_{int(time.time())}"
            return True
        
        def get(self, endpoint):
            if not self.session["token"]:
                raise Exception("Not authenticated")
            return {"endpoint": endpoint, "data": "mock_response"}
    
    client = APIClient(test_config)
    yield client
    # Cleanup
    client.session["token"] = None


# Fixture for testing conftest loading
@pytest.fixture
def conftest_fixture():
    """Fixture defined in conftest for testing"""
    return "conftest_value"


# Plugin fixture (simulating a plugin-provided fixture)
@pytest.fixture
def plugin_fixture():
    """Simulating a fixture that would be provided by a plugin"""
    return "plugin_value"


# Autouse fixture for test isolation
@pytest.fixture(autouse=True)
def reset_environment(monkeypatch):
    """Reset environment for each test"""
    # Clear any test environment variables
    test_env_vars = [k for k in os.environ.keys() if k.startswith("TEST_")]
    for var in test_env_vars:
        monkeypatch.delenv(var, raising=False)
    
    yield
    
    # Cleanup is automatic with monkeypatch


# Fixture factory
@pytest.fixture
def user_factory():
    """Factory for creating test users"""
    created_users = []
    
    def make_user(name, email=None, age=25):
        user = {
            "id": len(created_users) + 1,
            "name": name,
            "email": email or f"{name.lower()}@test.com",
            "age": age,
            "created_at": time.time()
        }
        created_users.append(user)
        return user
    
    yield make_user
    
    # Cleanup - in real scenario might delete from database
    created_users.clear()


# Parametrized fixture with custom ids
@pytest.fixture(params=[
    {"format": "json", "extension": ".json"},
    {"format": "yaml", "extension": ".yml"},
    {"format": "toml", "extension": ".toml"}
], ids=lambda p: p["format"])
def file_format(request):
    """Different file formats for testing"""
    return request.param


# Fixture using pytest cache
@pytest.fixture
def cached_data(request):
    """Fixture that uses pytest's cache"""
    cache = request.config.cache
    
    # Try to get cached value
    cached_value = cache.get("test_data", None)
    if cached_value is None:
        # Generate new value
        cached_value = {"generated_at": time.time(), "data": "test"}
        cache.set("test_data", cached_value)
    
    return cached_value


# Conditional fixture
@pytest.fixture
def slow_resource(request):
    """Fixture that's only created for tests marked as 'slow'"""
    if "slow" not in [m.name for m in request.node.iter_markers()]:
        pytest.skip("Slow resource only available for slow tests")
    
    # Simulate slow resource creation
    time.sleep(0.1)
    return {"resource": "slow", "ready": True}


# Fixture with finalizer
@pytest.fixture
def resource_with_finalizer(request):
    """Fixture demonstrating finalizer usage"""
    resource = {"name": "resource", "active": True}
    
    def fin():
        # This runs after the test
        resource["active"] = False
        print(f"Finalized resource: {resource}")
    
    request.addfinalizer(fin)
    return resource


# Error handling fixture
@pytest.fixture
def safe_temp_dir(tmp_path):
    """Fixture that ensures cleanup even on test failure"""
    work_dir = tmp_path / "work"
    work_dir.mkdir()
    
    # Track created files for cleanup
    created_files = []
    
    class SafeDir:
        def __init__(self, path):
            self.path = path
        
        def create_file(self, name, content=""):
            filepath = self.path / name
            filepath.write_text(content)
            created_files.append(filepath)
            return filepath
        
        def __enter__(self):
            return self
        
        def __exit__(self, exc_type, exc_val, exc_tb):
            # Cleanup even if test fails
            for filepath in created_files:
                try:
                    filepath.unlink()
                except:
                    pass
    
    return SafeDir(work_dir)


# Dynamic fixture
@pytest.fixture
def dynamic_fixture(request):
    """Fixture that adapts based on test requirements"""
    # Get test function
    test_func = request.node.function
    
    # Check for specific attributes or markers
    if hasattr(test_func, "use_mock"):
        return {"type": "mock", "data": "mocked"}
    elif "integration" in request.node.keywords:
        return {"type": "real", "data": "real_data"}
    else:
        return {"type": "default", "data": "default_data"}


# Fixture with indirect parametrization support
@pytest.fixture
def data_source(request):
    """Fixture that can be parametrized indirectly"""
    if hasattr(request, "param"):
        source_type = request.param
        if source_type == "file":
            return {"type": "file", "path": "/tmp/data.txt"}
        elif source_type == "database":
            return {"type": "database", "connection": "db://localhost"}
        elif source_type == "api":
            return {"type": "api", "url": "http://api.test.com"}
    
    # Default
    return {"type": "memory", "data": {}}


# Helper fixture for assertions
@pytest.fixture
def assert_helpers():
    """Common assertion helpers"""
    class Helpers:
        @staticmethod
        def assert_between(value, min_val, max_val):
            assert min_val <= value <= max_val, f"{value} not between {min_val} and {max_val}"
        
        @staticmethod
        def assert_dict_subset(subset, superset):
            for key, value in subset.items():
                assert key in superset, f"Key {key} not in dict"
                assert superset[key] == value, f"Value mismatch for key {key}"
        
        @staticmethod
        def assert_raises_with_message(exc_class, message, func, *args, **kwargs):
            with pytest.raises(exc_class) as exc_info:
                func(*args, **kwargs)
            assert message in str(exc_info.value)
    
    return Helpers()


# Import os for reset_environment fixture
import os