# Fastest Documentation

Welcome to the Fastest documentation. Fastest is a Rust-backed Python test
runner focused on pytest-style discovery and execution. It is under active
compatibility work and should be validated suite by suite before replacing
pytest in a project.

## 🚀 Quick Start

```bash
# Install fastest
curl -LsSf https://raw.githubusercontent.com/derens99/fastest/main/install.sh | sh

# Run your tests  
fastest tests/

# Inspect supported and experimental options
fastest --help
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

## 🎯 Current Strengths

- **Rust CLI and execution engine**
- **Discovery for functions, classes, async tests, and parametrized tests**
- **Common fixture and marker support**
- **Passing project Rust and Python test gates**
- **Compatibility suites** for feature-by-feature validation
- **Rust implementation** with performance work tracked through benchmark artifacts

## 📊 Current Verification

Current local gates include:

| Gate | Result |
|------|--------|
| Rust workspace tests | Passing |
| Python project tests | Passing |
| `make compat-report-all` | All discovered compatibility categories pass with expected skips/xfails |

Older fixed-speedup and percentage-compatibility claims are being replaced with
generated compatibility and benchmark reports. The source of truth is the
[roadmap](reference/roadmap.md).

## 🤝 Community

- **Issues**: [GitHub Issues](https://github.com/derens99/fastest/issues)
- **Discussions**: [GitHub Discussions](https://github.com/derens99/fastest/discussions)
- **Contributing**: See [Contributing Guide](development/contributing.md)

---

Start with the [quickstart](getting-started/quickstart.md), then validate your
suite against the compatibility notes in the [roadmap](reference/roadmap.md).
