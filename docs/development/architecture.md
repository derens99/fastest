# Fastest Architecture Documentation

## Overview

Fastest is built as a modular Rust workspace with 5 specialized crates, each serving a specific purpose in the test execution pipeline. This architecture enables high performance through separation of concerns and allows for independent optimization of each component.

## Crate Structure

### 1. fastest-core - Foundation Layer

The core crate provides fundamental building blocks used by all other crates.

#### Key Components:

**Test Discovery Engine**
- Multi-threaded file walking with `.gitignore` respect
- Dual parsing strategy: Regex for small files (<1000 lines), AST for large files
- Content-based caching with xxHash validation

**Test Data Structures**
```rust
TestItem {
    id: String,              // Unique identifier
    path: PathBuf,           // File location
    function_name: String,   // Original function name
    decorators: Vec<String>, // Python decorators
    fixtures: Vec<String>,   // Required fixtures
    markers: Vec<Marker>,    // Test markers
    class_name: Option<String>, // Parent class if any
}
```

**Configuration System**
- Multi-source config loading (pyproject.toml, pytest.ini, setup.cfg, tox.ini)
- Environment variable support
- CLI argument override precedence

**Fixture Management**
- Full pytest-compatible fixture system
- All scopes: function, class, module, session, package
- Dependency resolution with cycle detection
- Yield fixtures with proper teardown

**Parser Infrastructure**
- Tree-sitter based Python AST parsing
- Thread-local parser instances for zero allocation
- Support for all Python test patterns

### 2. fastest-execution - Execution Engine

The execution crate implements multiple strategies for running tests, automatically selecting the optimal approach based on test count.

#### Execution Strategies:

**InProcess Strategy (≤20 tests)**
- Direct PyO3 execution in Rust process
- Minimal overhead for small test suites
- ~45 tests/second

**HybridBurst Strategy (21-100 tests)**
- Intelligent threading with work queue
- Optimized batch processing
- 180-250 tests/second

**WorkStealing Strategy (>100 tests)**
- Lock-free parallel execution
- Zero-contention work distribution
- Up to 5,700 tests/second

#### Key Classes:

**UltraFastExecutor**
- Automatic strategy selection
- Plugin system integration
- Performance monitoring

**PythonRuntime**
- Virtual environment detection
- Process pool management
- PyO3 integration for in-process execution

**Experimental Features**
- JIT compilation for simple assertions (disabled for security)
- Zero-copy IPC using shared memory
- Arena allocation for reduced memory overhead

### 3. fastest-advanced - Power Features

Advanced features for enterprise use cases and power users.

#### Components:

**SmartCoverage**
- Integration with coverage.py
- Multiple report formats (HTML, XML, JSON, LCOV)
- Incremental coverage tracking
- Content-based caching with BLAKE3

**IncrementalTester**
- Git-based change detection
- File dependency analysis
- Test impact analysis
- LRU cache for test results

**TestPrioritizer**
- ML-based test ordering
- Multi-factor scoring:
  - Failure rate (40% weight)
  - Execution time (10% weight)
  - Recency (20% weight)
  - Code modifications (20% weight)
  - Dependencies (10% weight)

**DependencyTracker**
- Graph-based dependency analysis using petgraph
- Import analysis with tree-sitter
- Fixture dependency tracking
- Topological sorting for execution order

**TestWatcher**
- File system monitoring with debouncing
- Intelligent test re-execution
- Integration with incremental testing

### 4. fastest-plugins - Extensibility

Complete plugin system with pytest compatibility.

#### Architecture:

**Plugin Interface**
```rust
trait Plugin {
    fn metadata() -> &PluginMetadata;
    fn initialize() -> Result<()>;
    fn shutdown() -> Result<()>;
}
```

**Hook System**
- Type-safe hook definitions
- Priority-based execution
- Support for sync and async hooks
- Builder pattern for hook calls

**Plugin Types**
1. Built-in plugins (fixtures, markers, reporting, capture)
2. Python plugins via PyO3
3. Native Rust plugins via dynamic loading
4. Conftest.py plugins

**pytest Compatibility**
- pytest-mock: Mocker fixture implementation
- pytest-cov: Coverage integration
- Hook compatibility layer

### 5. fastest-cli - User Interface

Command-line interface that orchestrates all functionality.

#### Features:

**Commands**
- Default: Run tests
- Discover: List tests without execution
- Version: Show version information
- Update: Self-update functionality
- Benchmark: Performance testing

**Advanced Options**
- Coverage collection
- Incremental testing
- Watch mode
- Smart prioritization
- Plugin configuration

## Data Flow

### 1. Discovery Phase
```
CLI Arguments → Config Loading → Test Discovery → Parser Selection → 
Test Extraction → Fixture Analysis → Marker Processing → Cache Update
```

### 2. Execution Phase
```
Test Selection → Strategy Selection → Plugin Initialization →
Test Distribution → Parallel Execution → Result Collection → 
Plugin Hooks → Output Formatting
```

### 3. Advanced Features
```
Git Analysis → Change Detection → Impact Analysis → 
Test Prioritization → Coverage Collection → Report Generation
```

## Performance Optimizations

### SIMD Acceleration
- JSON parsing with simd-json (2-3x faster)
- Pattern matching optimization
- Supported on x86_64 (AVX2) and aarch64

### Memory Optimizations
- Arena allocation in hot paths
- String interning for deduplication
- Zero-copy IPC for process communication
- Thread-local storage for parsers

### Caching Strategy
- Content-based invalidation
- Compressed cache storage
- Memory-mapped file access
- Atomic file operations

### Parallelism
- Work-stealing for optimal distribution
- CPU affinity optimization
- Lock-free data structures
- Adaptive worker count

## Design Principles

1. **Performance First**: Every feature must improve or maintain performance
2. **Modular Architecture**: Clear separation of concerns
3. **pytest Compatibility**: Drop-in replacement goal
4. **Progressive Enhancement**: Advanced features are optional
5. **Fail Gracefully**: Degradation when features unavailable

## Integration Points

### Python Integration
- PyO3 for in-process execution
- Subprocess pools for isolation
- Virtual environment detection
- Multiple Python version support

### External Tools
- Git for change detection
- coverage.py for code coverage
- Tree-sitter for parsing
- OS-specific optimizations

### Plugin System
- Hook-based architecture
- Priority ordering
- Dependency resolution
- Lazy loading

## Future Architecture Considerations

### Planned Improvements
1. Distributed execution across machines
2. Cloud-native test execution
3. Real-time test result streaming
4. Advanced caching strategies
5. GPU acceleration for suitable workloads

### Scalability
- Designed for millions of tests
- Efficient resource utilization
- Minimal memory footprint
- Linear scaling with cores

This architecture enables Fastest to achieve its 3.9x performance improvement over pytest while maintaining high compatibility and extensibility.