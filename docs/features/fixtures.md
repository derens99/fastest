# Fixture System in Fastest

Fastest provides a fixture system compatible with pytest, allowing you to share setup code between tests.

## Quick Start

```python
import fastest

@fastest.fixture
def sample_data():
    """Provide test data."""
    return {"name": "test", "value": 42}

def test_with_fixture(sample_data):
    assert sample_data["name"] == "test"
    assert sample_data["value"] == 42
```

## Built-in Fixtures

### tmp_path
Creates a temporary directory unique to each test.

```python
def test_file_operations(tmp_path):
    # tmp_path is a pathlib.Path object
    test_file = tmp_path / "test.txt"
    test_file.write_text("Hello, World!")
    
    assert test_file.exists()
    assert test_file.read_text() == "Hello, World!"
    # Directory is automatically cleaned up after test
```

### capsys
Captures stdout and stderr output.

```python
def test_output(capsys):
    print("Hello from test")
    print("Error message", file=sys.stderr)
    
    captured = capsys.readouterr()
    assert captured.out == "Hello from test\n"
    assert captured.err == "Error message\n"
```

### monkeypatch
Dynamically modify objects, dict items, and environment variables.

```python
def test_with_monkeypatch(monkeypatch):
    # Patch an attribute
    monkeypatch.setattr("os.getcwd", lambda: "/fake/path")
    assert os.getcwd() == "/fake/path"
    
    # Set environment variable
    monkeypatch.setenv("API_KEY", "test-key")
    assert os.environ["API_KEY"] == "test-key"
    
    # Patch dict items
    config = {"debug": False}
    monkeypatch.setitem(config, "debug", True)
    assert config["debug"] is True
```

## Creating Custom Fixtures

### Basic Fixtures

```python
@fastest.fixture
def database_url():
    """Provide database connection string."""
    return "postgresql://localhost/testdb"

@fastest.fixture
def api_client(database_url):
    """Create API client with database."""
    client = APIClient(db_url=database_url)
    return client

def test_api(api_client):
    response = api_client.get("/users")
    assert response.status_code == 200
```

### Fixtures with Cleanup

Use `yield` to provide teardown code:

```python
@fastest.fixture
def database():
    """Setup and teardown database."""
    db = Database("test.db")
    db.create_tables()
    
    yield db  # Test runs here
    
    # Cleanup after test
    db.drop_tables()
    db.close()

def test_database_operations(database):
    user = database.create_user("Alice")
    assert user.name == "Alice"
    # Database is automatically cleaned up
```

## Fixture Scopes

Control how often fixtures are created:

```python
@fastest.fixture(scope="function")  # Default - new for each test
def transaction():
    return db.begin_transaction()

@fastest.fixture(scope="module")  # Once per module
def database_connection():
    return db.connect()

@fastest.fixture(scope="session")  # Once per test run
def app_config():
    return load_config()
```

### Scope Hierarchy
- `function` - Created fresh for each test (default)
- `class` - Shared by all methods in a test class
- `module` - Shared by all tests in a module
- `session` - Created once, shared by all tests

## Fixture Dependencies

Fixtures can depend on other fixtures:

```python
@fastest.fixture
def config():
    return {"api_url": "http://localhost:8000"}

@fastest.fixture
def headers(config):
    return {"Host": config["api_url"]}

@fastest.fixture
def client(config, headers):
    return HTTPClient(base_url=config["api_url"], headers=headers)

def test_api_call(client):
    # client fixture automatically gets config and headers
    response = client.get("/health")
    assert response.ok
```

## Autouse Fixtures

Automatically use fixtures without declaring them:

```python
@fastest.fixture(autouse=True)
def reset_environment():
    """Reset environment before each test."""
    os.environ.clear()
    os.environ.update(DEFAULT_ENV)
    yield
    # Cleanup if needed

def test_environment():
    # reset_environment runs automatically
    os.environ["TEST"] = "value"
    assert os.environ["TEST"] == "value"
```

## Best Practices

### 1. Keep Fixtures Simple
```python
# Good - focused fixture
@fastest.fixture
def user():
    return User(name="test", email="test@example.com")

# Avoid - too much setup
@fastest.fixture
def everything():
    setup_database()
    create_users()
    configure_api()
    # ... too much
```

### 2. Use Descriptive Names
```python
# Good
@fastest.fixture
def authenticated_user():
    return User.create_authenticated()

# Less clear
@fastest.fixture
def u():
    return User()
```

### 3. Appropriate Scopes
```python
# Good - expensive setup at module level
@fastest.fixture(scope="module")
def database_schema():
    db.create_all_tables()
    yield
    db.drop_all_tables()

# Good - cheap setup at function level
@fastest.fixture
def mock_time():
    return Mock(return_value=1234567890)
```

## Compatibility with pytest

Fastest fixtures work alongside pytest fixtures:

```python
import pytest
import fastest

# Both decorators work
@pytest.fixture
def pytest_style():
    return "from pytest"

@fastest.fixture
def fastest_style():
    return "from fastest"

def test_both_fixtures(pytest_style, fastest_style):
    assert pytest_style == "from pytest"
    assert fastest_style == "from fastest"
```

## Current Limitations

- Parametrized fixtures are not yet supported
- Some advanced pytest fixtures may not be available
- Fixture discovery in conftest.py files is limited

For the latest updates on fixture support, see the [Roadmap](ROADMAP.md). 