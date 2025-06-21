# Plugin System

Fastest provides a powerful plugin architecture that maintains pytest compatibility while offering superior performance and type safety.

## Quick Start

### Using Plugins

```bash
# Run with default plugins (fixtures, markers, reporting, capture)
fastest tests/

# Disable all plugins for maximum performance
fastest --no-plugins tests/

# Use specific plugin directory
fastest --plugin-dir ./my_plugins tests/

# With pytest-compatible plugins
fastest --cov=src --cov-report=html tests/  # Coverage support
```

### Writing a Simple Plugin

```python
# conftest.py
import time

def pytest_runtest_setup(item):
    """Called before each test"""
    item.start_time = time.time()

def pytest_runtest_teardown(item):
    """Called after each test"""
    duration = time.time() - item.start_time
    print(f"{item.name}: {duration:.3f}s")

@pytest.fixture
def my_fixture():
    """Custom fixture provided by plugin"""
    return {"data": "value"}
```

## Plugin Architecture

### Hook-Based System

Fastest uses a hook-based architecture similar to pytest:

- **Collection Hooks**: Modify test discovery
- **Session Hooks**: Setup/teardown test sessions
- **Test Hooks**: Customize test execution
- **Fixture Hooks**: Extend fixture system
- **Reporting Hooks**: Custom output formats

### Plugin Types

1. **Built-in Plugins** - Core functionality as plugins
2. **Python Plugins** - pytest-compatible Python code
3. **Native Rust Plugins** - High-performance plugins
4. **Conftest Plugins** - Project-specific customization

## Core Hooks

### Session Lifecycle

```python
def pytest_sessionstart(session):
    """Called after Session object created, before collection"""
    print("Testing session started")

def pytest_sessionfinish(session, exitstatus):
    """Called after all tests completed"""
    print(f"Testing finished with status: {exitstatus}")
```

### Test Collection

```python
def pytest_collection_modifyitems(items):
    """Modify collected test items"""
    # Sort tests by name
    items.sort(key=lambda x: x.name)
    
    # Skip slow tests in CI
    if os.environ.get('CI'):
        skip_slow = pytest.mark.skip(reason="Slow test skipped in CI")
        for item in items:
            if "slow" in item.keywords:
                item.add_marker(skip_slow)
```

### Test Execution

```python
def pytest_runtest_setup(item):
    """Setup before test execution"""
    if "database" in item.fixturenames:
        setup_test_database()

def pytest_runtest_teardown(item):
    """Cleanup after test execution"""
    if "database" in item.fixturenames:
        cleanup_test_database()

def pytest_runtest_makereport(item, call):
    """Create test report"""
    if call.when == "call" and call.excinfo is not None:
        # Custom failure handling
        save_failure_screenshot(item.name)
```

## pytest Plugin Compatibility

### Supported Plugins

| Plugin | Status | Usage |
|--------|--------|-------|
| pytest-mock | ✅ Supported | `mocker` fixture available |
| pytest-cov | ✅ Supported | `--cov` options work |
| pytest-xdist | 😧 Partial | Basic distribution |
| pytest-asyncio | 📋 Planned | Coming soon |
| pytest-timeout | 📋 Planned | Coming soon |

### Using pytest-mock

```python
def test_with_mock(mocker):
    # Mock a module function
    mock = mocker.patch('requests.get')
    mock.return_value.json.return_value = {"status": "ok"}
    
    # Mock object method
    obj = MyClass()
    mocker.patch.object(obj, 'method', return_value=42)
    
    # Spy on calls
    spy = mocker.spy(obj, 'other_method')
    obj.other_method()
    spy.assert_called_once()
```

### Using Coverage

```bash
# Basic coverage
fastest --cov=src tests/

# With HTML report
fastest --cov=src --cov-report=html tests/

# With minimum coverage requirement
fastest --cov=src --cov-fail-under=80 tests/
```

## Creating Custom Plugins

### Python Plugin Example

```python
# performance_plugin.py
import time
from typing import Dict

class PerformancePlugin:
    """Track and report slow tests"""
    
    def __init__(self, threshold: float = 1.0):
        self.threshold = threshold
        self.timings: Dict[str, float] = {}
    
    def pytest_runtest_setup(self, item):
        """Start timing"""
        self.timings[item.nodeid] = time.time()
    
    def pytest_runtest_teardown(self, item):
        """Check test duration"""
        duration = time.time() - self.timings[item.nodeid]
        if duration > self.threshold:
            print(f"\n⚠️  SLOW: {item.nodeid} took {duration:.2f}s")
    
    def pytest_sessionfinish(self, session):
        """Report summary"""
        slow_tests = [
            (name, end - start) 
            for name, start in self.timings.items()
            if (end := time.time()) - start > self.threshold
        ]
        
        if slow_tests:
            print(f"\n{len(slow_tests)} slow tests found!")
            for name, duration in sorted(slow_tests, key=lambda x: x[1], reverse=True)[:5]:
                print(f"  {duration:.2f}s - {name}")

# Register in conftest.py
def pytest_configure(config):
    config.pluginmanager.register(PerformancePlugin(threshold=0.5))
```

### Rust Plugin Example

```rust
use fastest_plugins::{Plugin, PluginMetadata, PluginResult};
use std::any::Any;

#[derive(Debug)]
pub struct MetricsPlugin {
    metadata: PluginMetadata,
    test_count: usize,
    pass_count: usize,
}

impl MetricsPlugin {
    pub fn new() -> Self {
        Self {
            metadata: PluginMetadata {
                name: "metrics".to_string(),
                version: "0.1.0".to_string(),
                description: Some("Test metrics collector".to_string()),
                ..Default::default()
            },
            test_count: 0,
            pass_count: 0,
        }
    }
}

impl Plugin for MetricsPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }
    
    fn initialize(&mut self) -> PluginResult<()> {
        println!("Metrics plugin initialized");
        Ok(())
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
}
```

## Configuration

### CLI Options

```bash
# Disable all plugins
fastest --no-plugins tests/

# Disable specific plugin
fastest --disable-plugin verbose tests/

# Add plugin search path
fastest --plugin-dir ./plugins tests/

# Plugin-specific options
fastest --plugin-opt coverage.min=80 tests/
```

### Configuration File

```toml
# pyproject.toml
[tool.fastest.plugins]
enabled = ["timer", "coverage", "custom"]
disabled = ["verbose"]
directories = ["./project_plugins"]

[tool.fastest.plugins.coverage]
source = ["src"]
min_coverage = 80
report = ["term", "html"]

[tool.fastest.plugins.custom]
threshold = 0.5
output = "metrics.json"
```

## Built-in Plugins

### FixturePlugin
Manages test fixtures, scopes, and dependencies.

### MarkerPlugin  
Handles test markers like skip, xfail, and custom markers.

### ReportingPlugin
Controls test output formatting and reporting.

### CapturePlugin
Manages stdout/stderr capture during tests.

## Best Practices

### 1. Keep Plugins Focused

```python
# Good: Single responsibility
class TimingPlugin:
    """Only handles test timing"""
    
def pytest_runtest_setup(self, item):
        item.start_time = time.time()

# Avoid: Multiple responsibilities
class EverythingPlugin:
    """Timing, coverage, mocking, reporting..."""
```

### 2. Handle Errors Gracefully

```python
def pytest_runtest_setup(item):
    try:
        expensive_setup()
    except Exception as e:
        # Don't crash the test runner
        item.add_marker(pytest.mark.skip(f"Setup failed: {e}"))
```

### 3. Use Appropriate Hooks

```python
# Good: Use sessionfinish for summary
def pytest_sessionfinish(session):
    print_test_summary()

# Avoid: Don't use runtest hooks for session-level work
def pytest_runtest_teardown(item):
    print_entire_test_summary()  # Called for every test!
```

### 4. Performance Considerations

```python
# Cache expensive operations
class CachedPlugin:
    def __init__(self):
        self._cache = {}
    
    def pytest_runtest_setup(self, item):
        if item.module not in self._cache:
            self._cache[item.module] = expensive_analysis(item.module)
```

## Debugging Plugins

### Enable Debug Output

```bash
# See all hook calls
FASTEST_DEBUG=1 fastest -v tests/

# Trace specific plugin
RUST_LOG=fastest_plugins=debug fastest tests/
```

### List Active Plugins

```bash
fastest --list-plugins
```

### Plugin Load Order

1. Built-in plugins (always first)
2. Installed packages (pytest11 entry point)
3. conftest.py files (hierarchical)
4. Plugin directories (--plugin-dir)
5. Manually specified (--plugins)

## Future Enhancements

- **Hot Reloading** - Reload plugins without restart
- **WASM Plugins** - WebAssembly plugin support
- **Plugin Marketplace** - Share and discover plugins
- **More pytest Plugins** - Broader compatibility
- **Plugin Profiling** - Performance analysis

## Next Steps

- [Fixtures](fixtures.md) - Learn about the fixture system
- [Markers](markers.md) - Use test markers
- [Development Guide](../development/contributing.md) - Create plugins
