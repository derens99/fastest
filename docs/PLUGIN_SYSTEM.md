# Fastest Plugin System

The Fastest plugin system provides a powerful and extensible architecture that maintains compatibility with pytest plugins while offering superior performance and type safety.

## Overview

Fastest implements a **hook-based plugin architecture** similar to pytest, allowing you to:

- Customize test collection and execution
- Add new fixtures and markers
- Integrate with external tools
- Generate custom reports
- Extend functionality without modifying core code

## Key Features

### 🚀 Performance First
- **Zero-cost abstractions**: Plugins have minimal overhead
- **Type-safe hooks**: Compile-time guarantees prevent runtime errors
- **Lazy loading**: Plugins are only loaded when needed
- **Parallel-safe**: All plugins work correctly with parallel execution

### 🔧 Pytest Compatible
- Familiar hook names and semantics
- Support for conftest.py files
- Compatible with many pytest plugins
- Easy migration path

### 🏗️ Extensible Architecture
- Multiple plugin types (Python, Rust, conftest)
- Dependency resolution
- Priority-based execution
- Dynamic discovery

## Plugin Types

### 1. Built-in Plugins

Fastest's core functionality is implemented as plugins:

```rust
use fastest_plugins::{Plugin, PluginMetadata, PluginBuilder};

pub struct FixturePlugin {
    metadata: PluginMetadata,
    // ... plugin state
}

impl Plugin for FixturePlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }
    
    fn initialize(&mut self) -> PluginResult<()> {
        // Initialize plugin
        Ok(())
    }
}
```

### 2. Python Plugins

Create pytest-compatible plugins in Python:

```python
# conftest.py or separate plugin file

def pytest_collection_modifyitems(items):
    """Modify collected test items."""
    # Sort tests by name
    items.sort(key=lambda item: item.name)

def pytest_runtest_setup(item):
    """Called before each test."""
    print(f"Setting up {item.name}")

@pytest.fixture
def my_fixture():
    """Custom fixture."""
    return {"data": "value"}
```

### 3. Native Rust Plugins

High-performance plugins written in Rust:

```rust
use fastest_plugins::*;

#[derive(Debug)]
pub struct TimingPlugin {
    metadata: PluginMetadata,
}

impl Plugin for TimingPlugin {
    // ... implementation
}

// Export for dynamic loading
#[no_mangle]
pub extern "C" fn fastest_plugin_entry() -> *mut dyn Plugin {
    Box::into_raw(Box::new(TimingPlugin::new()))
}
```

## Available Hooks

### Session Hooks

| Hook | Description | When Called |
|------|-------------|-------------|
| `pytest_configure` | Configure test session | At startup |
| `pytest_sessionstart` | Session is starting | Before collection |
| `pytest_sessionfinish` | Session is ending | After all tests |
| `pytest_unconfigure` | Unconfigure session | At shutdown |

### Collection Hooks

| Hook | Description | When Called |
|------|-------------|-------------|
| `pytest_collection_start` | Collection starting | Before discovery |
| `pytest_collection_modifyitems` | Modify test items | After collection |
| `pytest_collection_finish` | Collection complete | After modification |

### Test Execution Hooks

| Hook | Description | When Called |
|------|-------------|-------------|
| `pytest_runtest_protocol` | Customize test protocol | For each test |
| `pytest_runtest_setup` | Test setup | Before test |
| `pytest_runtest_call` | Test execution | During test |
| `pytest_runtest_teardown` | Test teardown | After test |
| `pytest_runtest_makereport` | Generate report | After each phase |

### Fixture Hooks

| Hook | Description | When Called |
|------|-------------|-------------|
| `pytest_fixture_setup` | Fixture setup | Before fixture |
| `pytest_fixture_post_finalizer` | After finalizer | After teardown |

## Creating Plugins

### Simple Python Plugin

```python
# my_plugin.py
import time

class TimerPlugin:
    def __init__(self):
        self.start_times = {}
    
    def pytest_runtest_setup(self, item):
        self.start_times[item.nodeid] = time.time()
    
    def pytest_runtest_teardown(self, item):
        duration = time.time() - self.start_times.get(item.nodeid, 0)
        print(f"{item.nodeid}: {duration:.3f}s")

# Register in conftest.py
def pytest_configure(config):
    config.pluginmanager.register(TimerPlugin(), "timer")
```

### Rust Plugin with Hooks

```rust
use fastest_plugins::*;

pub struct CoveragePlugin {
    metadata: PluginMetadata,
    coverage_data: HashMap<String, f64>,
}

impl CoveragePlugin {
    pub fn new() -> Self {
        Self {
            metadata: PluginBuilder::new("coverage")
                .version("0.1.0")
                .description("Code coverage plugin")
                .build(),
            coverage_data: HashMap::new(),
        }
    }
}

impl Plugin for CoveragePlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }
}

// Hook implementation
struct CoverageReportHook {
    plugin: Arc<CoveragePlugin>,
}

impl Hook for CoverageReportHook {
    fn name(&self) -> &str {
        "pytest_sessionfinish"
    }
    
    fn execute(&self, args: HookArgs) -> HookResult<HookReturn> {
        // Generate coverage report
        println!("Coverage: {:.1}%", self.calculate_coverage());
        Ok(HookReturn::None)
    }
}
```

## pytest Plugin Compatibility

### Supported pytest Plugins

| Plugin | Status | Notes |
|--------|--------|-------|
| pytest-mock | ✅ Supported | Full mocker fixture |
| pytest-cov | ✅ Supported | Coverage collection |
| pytest-xdist | 🚧 Partial | Basic distribution |
| pytest-asyncio | 🚧 Planned | Async test support |
| pytest-timeout | 🚧 Planned | Test timeouts |

### Using pytest-mock

```python
def test_mock_example(mocker):
    # Mock a function
    mock_func = mocker.patch('module.function')
    mock_func.return_value = 42
    
    # Mock an object method
    mock_obj = mocker.patch.object(MyClass, 'method')
    
    # Create a spy
    spy = mocker.spy(obj, 'method')
```

### Using pytest-cov

```bash
# Run with coverage
fastest --cov=src --cov-report=html

# With minimum coverage
fastest --cov=src --cov-min=80
```

## Plugin Configuration

### Via CLI

```bash
# Load specific plugins
fastest --plugins timer,coverage

# Disable plugins
fastest --no-plugins fixture,marker

# Plugin directory
fastest --plugin-dir ./my_plugins
```

### Via Configuration File

```toml
# pyproject.toml
[tool.fastest.plugins]
enabled = ["timer", "coverage"]
disabled = ["verbose"]
directories = ["./plugins"]

[tool.fastest.plugins.coverage]
source = ["src"]
min_coverage = 80
report = ["term", "html"]
```

### Via Code

```rust
use fastest_plugins::{PluginManager, PluginManagerBuilder};

let manager = PluginManagerBuilder::new()
    .discover_installed(true)
    .load_conftest(true)
    .plugin_dir("./plugins")
    .with_plugin(Box::new(MyPlugin::new()))
    .build()?;
```

## Advanced Features

### Plugin Dependencies

```rust
let metadata = PluginBuilder::new("my_plugin")
    .requires("fastest.fixtures")  // Requires fixture plugin
    .conflicts("old_plugin")        // Conflicts with old_plugin
    .priority(100)                  // Higher priority
    .build();
```

### Async Hooks

```rust
#[async_trait]
impl AsyncHook for MyAsyncHook {
    fn name(&self) -> &str {
        "pytest_async_setup"
    }
    
    async fn execute_async(&self, args: HookArgs) -> HookResult<HookReturn> {
        // Async operations
        Ok(HookReturn::None)
    }
}
```

### Hook Ordering

Hooks are executed based on:
1. Plugin priority (higher first)
2. Registration order (first registered first)
3. Dependencies (required plugins first)

### Plugin Discovery

Fastest discovers plugins from:
1. Built-in plugins (always loaded)
2. Entry points (`pytest11` and `fastest.plugins`)
3. conftest.py files (in directory hierarchy)
4. Plugin directories (specified via CLI/config)
5. Manually registered plugins

## Best Practices

### 1. Keep Plugins Focused
Each plugin should have a single responsibility.

### 2. Use Type-Safe APIs
Leverage Rust's type system for compile-time guarantees.

### 3. Handle Errors Gracefully
Plugins shouldn't crash the test runner.

### 4. Document Hook Behavior
Clearly document what your hooks do and when they run.

### 5. Test Your Plugins
Write tests for plugin functionality.

## Example: Complete Plugin

Here's a complete example of a performance monitoring plugin:

```rust
use fastest_plugins::*;
use std::time::{Duration, Instant};
use std::collections::HashMap;

/// Performance monitoring plugin
#[derive(Debug)]
pub struct PerfPlugin {
    metadata: PluginMetadata,
    timings: Arc<Mutex<HashMap<String, Duration>>>,
    thresholds: HashMap<String, Duration>,
}

impl PerfPlugin {
    pub fn new() -> Self {
        Self {
            metadata: PluginBuilder::new("perf_monitor")
                .version("0.1.0")
                .description("Monitor test performance")
                .author("Fastest Team")
                .build(),
            timings: Arc::new(Mutex::new(HashMap::new())),
            thresholds: HashMap::new(),
        }
    }
    
    pub fn set_threshold(&mut self, test: &str, threshold: Duration) {
        self.thresholds.insert(test.to_string(), threshold);
    }
}

impl Plugin for PerfPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }
    
    fn initialize(&mut self) -> PluginResult<()> {
        println!("Performance monitoring enabled");
        Ok(())
    }
}

// Timing collection hook
struct PerfTimingHook {
    timings: Arc<Mutex<HashMap<String, Duration>>>,
}

impl Hook for PerfTimingHook {
    fn name(&self) -> &str {
        "pytest_runtest_makereport"
    }
    
    fn execute(&self, args: HookArgs) -> HookResult<HookReturn> {
        if let Some(test_id) = args.get::<String>("test_id") {
            if let Some(duration) = args.get::<Duration>("duration") {
                self.timings.lock().unwrap()
                    .insert(test_id.clone(), *duration);
            }
        }
        Ok(HookReturn::None)
    }
}

// Performance report hook
struct PerfReportHook {
    plugin: Arc<PerfPlugin>,
}

impl Hook for PerfReportHook {
    fn name(&self) -> &str {
        "pytest_sessionfinish"
    }
    
    fn execute(&self, _args: HookArgs) -> HookResult<HookReturn> {
        let timings = self.plugin.timings.lock().unwrap();
        
        // Find slow tests
        let mut slow_tests = Vec::new();
        for (test, duration) in timings.iter() {
            if let Some(threshold) = self.plugin.thresholds.get(test) {
                if duration > threshold {
                    slow_tests.push((test, duration, threshold));
                }
            }
        }
        
        // Report slow tests
        if !slow_tests.is_empty() {
            println!("\n⚠️  Slow Tests Detected:");
            for (test, duration, threshold) in slow_tests {
                println!("  {} took {:.3}s (threshold: {:.3}s)",
                    test,
                    duration.as_secs_f64(),
                    threshold.as_secs_f64()
                );
            }
        }
        
        // Summary statistics
        let total_time: Duration = timings.values().sum();
        let avg_time = total_time / timings.len() as u32;
        
        println!("\n📊 Performance Summary:");
        println!("  Total time: {:.3}s", total_time.as_secs_f64());
        println!("  Average time: {:.3}s", avg_time.as_secs_f64());
        println!("  Test count: {}", timings.len());
        
        Ok(HookReturn::None)
    }
}
```

## Debugging Plugins

### Enable Debug Output

```bash
RUST_LOG=fastest_plugins=debug fastest
```

### Hook Execution Trace

```rust
let registry = manager.hooks();
let history = registry.history();
println!("Hook execution order: {:?}", history);
```

### Plugin List

```bash
fastest --list-plugins
```

## Future Enhancements

- **Hot Reloading**: Reload plugins without restarting
- **Plugin Marketplace**: Share and discover plugins
- **WASM Plugins**: WebAssembly plugin support
- **Plugin Profiling**: Performance analysis for plugins
- **Enhanced pytest Compatibility**: More pytest plugins supported

## Contributing

To contribute a plugin:

1. Follow the plugin template
2. Add comprehensive tests
3. Document all hooks and features
4. Submit a PR with examples

See [CONTRIBUTING.md](../CONTRIBUTING.md) for details.