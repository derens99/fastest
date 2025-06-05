"""Advanced fixture testing - all scopes and features"""
import pytest
import os
import tempfile
from pathlib import Path


# Session-scoped fixture (shared across all tests)
@pytest.fixture(scope="session")
def session_database():
    """Simulated database connection for the entire session"""
    print("SETUP: Creating session database")
    db = {"connection": "session_db_connection", "data": {}}
    yield db
    print("TEARDOWN: Closing session database")


# Package-scoped fixture
@pytest.fixture(scope="package")
def package_config():
    """Configuration for the package"""
    return {"package": "test_advanced_fixtures", "version": "1.0"}


# Module-scoped fixture (shared within module)
@pytest.fixture(scope="module")
def module_resource():
    """Resource shared within the module"""
    print("SETUP: Creating module resource")
    resource = {"type": "module_resource", "id": 42}
    yield resource
    print("TEARDOWN: Cleaning module resource")


# Class-scoped fixture
@pytest.fixture(scope="class")
def class_logger():
    """Logger for test class"""
    class Logger:
        def __init__(self):
            self.logs = []
        
        def log(self, message):
            self.logs.append(message)
        
        def get_logs(self):
            return self.logs
    
    return Logger()


# Function-scoped fixtures (default)
@pytest.fixture
def temp_workspace(tmp_path):
    """Create a temporary workspace with structure"""
    workspace = tmp_path / "workspace"
    workspace.mkdir()
    (workspace / "config.json").write_text('{"setting": "value"}')
    (workspace / "data").mkdir()
    return workspace


# Autouse fixture (automatically used by all tests)
@pytest.fixture(autouse=True)
def test_timer():
    """Automatically time each test"""
    import time
    start = time.time()
    yield
    duration = time.time() - start
    print(f"Test duration: {duration:.3f}s")


# Fixture with dependencies
@pytest.fixture
def authenticated_client(session_database, module_resource):
    """Client that depends on other fixtures"""
    client = {
        "db": session_database,
        "resource": module_resource,
        "auth_token": "secret_token_123"
    }
    return client


# Parametrized fixture
@pytest.fixture(params=[1, 2, 3], ids=["one", "two", "three"])
def number_fixture(request):
    """Fixture that provides multiple values"""
    return request.param * 10


# Fixture with teardown
@pytest.fixture
def file_creator(tmp_path):
    """Creates files and cleans them up"""
    created_files = []
    
    def create_file(name, content=""):
        filepath = tmp_path / name
        filepath.write_text(content)
        created_files.append(filepath)
        return filepath
    
    yield create_file
    
    # Teardown - remove all created files
    for filepath in created_files:
        if filepath.exists():
            filepath.unlink()


# Async fixture
@pytest.fixture
async def async_resource():
    """Async fixture example"""
    import asyncio
    await asyncio.sleep(0.001)
    return {"async": True, "data": "async_value"}


# Request fixture usage
@pytest.fixture
def request_aware_fixture(request):
    """Fixture that uses the request object"""
    return {
        "test_name": request.node.name,
        "markers": [m.name for m in request.node.iter_markers()],
        "fixture_names": request.fixturenames
    }


class TestClassScopedFixtures:
    """Test class demonstrating class-scoped fixtures"""
    
    def test_with_class_logger(self, class_logger):
        """First test using class logger"""
        class_logger.log("Test 1 started")
        assert hasattr(class_logger, 'log')
        class_logger.log("Test 1 completed")
        assert len(class_logger.get_logs()) >= 2
    
    def test_sharing_class_logger(self, class_logger):
        """Second test sharing the same logger instance"""
        # Should have logs from previous test
        existing_logs = class_logger.get_logs()
        assert len(existing_logs) >= 2  # From previous test
        
        class_logger.log("Test 2 message")
        assert len(class_logger.get_logs()) > len(existing_logs)


class TestAdvancedFixtures:
    """Test various fixture features"""
    
    def test_session_fixture(self, session_database):
        """Test session-scoped fixture"""
        assert session_database["connection"] == "session_db_connection"
        session_database["data"]["test"] = "value"
    
    def test_session_persistence(self, session_database):
        """Test that session fixture persists data"""
        # Should have data from previous test
        assert "test" in session_database["data"]
        assert session_database["data"]["test"] == "value"
    
    def test_module_fixture(self, module_resource):
        """Test module-scoped fixture"""
        assert module_resource["type"] == "module_resource"
        assert module_resource["id"] == 42
    
    def test_fixture_dependencies(self, authenticated_client):
        """Test fixture with dependencies"""
        assert authenticated_client["auth_token"] == "secret_token_123"
        assert authenticated_client["db"]["connection"] == "session_db_connection"
        assert authenticated_client["resource"]["id"] == 42
    
    def test_temp_workspace(self, temp_workspace):
        """Test complex fixture creating workspace"""
        assert temp_workspace.exists()
        assert (temp_workspace / "config.json").exists()
        assert (temp_workspace / "data").is_dir()
        
        config_content = (temp_workspace / "config.json").read_text()
        assert "setting" in config_content
    
    def test_parametrized_fixture(self, number_fixture):
        """Test parametrized fixture - runs multiple times"""
        assert number_fixture in [10, 20, 30]
        assert number_fixture % 10 == 0
    
    def test_file_creator_fixture(self, file_creator):
        """Test fixture with teardown"""
        # Create some files
        file1 = file_creator("test1.txt", "content1")
        file2 = file_creator("test2.txt", "content2")
        
        assert file1.exists()
        assert file2.exists()
        assert file1.read_text() == "content1"
        assert file2.read_text() == "content2"
        # Files will be cleaned up after test
    
    async def test_async_fixture(self, async_resource):
        """Test async fixture"""
        assert async_resource["async"] is True
        assert async_resource["data"] == "async_value"
    
    def test_request_fixture(self, request_aware_fixture):
        """Test fixture using request object"""
        assert "test_request_fixture" in request_aware_fixture["test_name"]
        assert "request_aware_fixture" in request_aware_fixture["fixture_names"]
    
    def test_multiple_fixtures(self, tmp_path, monkeypatch, capsys):
        """Test using multiple built-in fixtures"""
        # Use tmp_path
        test_file = tmp_path / "test.txt"
        test_file.write_text("hello")
        assert test_file.read_text() == "hello"
        
        # Use monkeypatch
        monkeypatch.setenv("TEST_VAR", "test_value")
        assert os.environ.get("TEST_VAR") == "test_value"
        
        # Use capsys
        print("Testing output capture")
        captured = capsys.readouterr()
        assert "Testing output capture" in captured.out


def test_package_fixture(package_config):
    """Test package-scoped fixture"""
    assert package_config["package"] == "test_advanced_fixtures"
    assert package_config["version"] == "1.0"


def test_autouse_fixture_works():
    """Test that autouse fixture (test_timer) runs automatically"""
    # The test_timer fixture should run automatically
    # We can't directly test it but it should print timing info
    assert True


# Yield fixture with complex teardown
@pytest.fixture
def database_transaction(session_database):
    """Fixture that manages database transaction"""
    # Setup - begin transaction
    transaction = {"id": "txn_123", "active": True}
    session_database["data"]["current_transaction"] = transaction
    
    yield transaction
    
    # Teardown - rollback transaction
    transaction["active"] = False
    if "current_transaction" in session_database["data"]:
        del session_database["data"]["current_transaction"]


def test_transaction_fixture(database_transaction, session_database):
    """Test fixture with yield and teardown"""
    assert database_transaction["active"] is True
    assert database_transaction["id"] == "txn_123"
    assert session_database["data"]["current_transaction"] == database_transaction