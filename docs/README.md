# Fastest Documentation

Welcome to the Fastest documentation! Fastest is a blazing-fast Python test runner built in Rust that's designed to be a drop-in replacement for pytest.

## 🚀 Quick Start

```bash
# Install fastest
curl -LsSf https://raw.githubusercontent.com/yourusername/fastest/main/install.sh | sh

# Run your tests  
fastest tests/

# Or migrate from pytest
fastest --help  # See all pytest-compatible options
```

## 📚 Documentation

### Getting Started
- **[Installation Guide](getting-started/installation.md)** - All installation methods
- **[Quickstart Guide](getting-started/quickstart.md)** - Up and running in 5 minutes
- **[Migration Guide](getting-started/migration-guide.md)** - Migrate from pytest

### Features
- **[Markers](features/markers.md)** - Test markers and decorators
- **[Fixtures](features/fixtures.md)** - Fixture system and scopes
- **[Plugins](features/plugins.md)** - Plugin system and pytest compatibility
- **[Parallel Execution](features/parallel-execution.md)** - Parallel test strategies

### Performance
- **[Benchmarks](performance/benchmarks.md)** - Official performance results
- **[Optimization Guide](performance/optimization.md)** - Maximize test speed

### Development
- **[Contributing](development/contributing.md)** - Contribute to Fastest
- **[Architecture](development/architecture.md)** - Project structure and design
- **[Testing](development/testing.md)** - Testing Fastest itself
- **[Release Process](development/release-process.md)** - How releases work

### Reference
- **[Changelog](reference/changelog.md)** - Release history
- **[Roadmap](reference/roadmap.md)** - Future plans

## 🎯 Key Benefits

- **⚡ 3.9x faster** than pytest (validated on real test suites)
- **🔄 91% pytest compatibility** (validated with comprehensive tests)
- **🚀 Intelligent execution** with automatic strategy selection
- **💾 Smart caching** for instant test discovery
- **🔧 Built in Rust** for maximum performance

## 📊 Performance Highlights

**Real-world validation with 749 tests:**
- **3.9x faster** than pytest overall
- **5,700 tests/second** peak throughput
- **0.13-0.23 seconds** for full test suite

| Strategy | Test Range | Performance |
|----------|------------|-------------|
| InProcess | ≤20 tests | 45 tests/sec |
| HybridBurst | 21-100 tests | 180-250 tests/sec |
| WorkStealing | >100 tests | 5,700 tests/sec |

## 🎯 pytest Compatibility

**91% compatibility** validated with comprehensive test suite:
- ✅ All test types (functions, classes, async)
- ✅ Fixtures (all scopes, autouse, yield)
- ✅ Markers (skip, xfail, custom)
- ✅ Parametrization
- ✅ Setup/teardown methods
- ✅ Plugin system
- ✅ Most pytest plugins

## 🤝 Community

- **Issues**: [GitHub Issues](https://github.com/yourusername/fastest/issues)
- **Discussions**: [GitHub Discussions](https://github.com/yourusername/fastest/discussions)  
- **Contributing**: See [Contributing Guide](development/contributing.md)

---

*Ready to make your tests blazing fast? [Get started now!](getting-started/quickstart.md)*