# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Nothing yet

### Changed
- Nothing yet

### Fixed
- Nothing yet

## [0.1.1] - 2024-01-09

### Added
- **Parametrized Test Support** 🎭
  - Full support for `@pytest.mark.parametrize` decorator
  - Handles single and multiple parameters
  - Supports multiple decorators (cartesian product)
  - Complex parameter values (tuples, lists, strings, numbers)
  - Proper test ID generation with parameter values
  - Works with both regex and AST parsers

### Fixed
- Multi-line decorator parsing in AST parser
- Parameter value parsing for various Python types

### Technical Details
- New `parametrize` module for handling test expansion
- Updated discovery to automatically expand parametrized tests
- Enhanced optimized executor to inject parameters at runtime

## [0.1.0] - 2024-01-08

### Added
- ⚡ Ultra-fast test discovery (88x faster than pytest)
- 🏃 Fast test execution (2.1x faster than pytest)
- 🔄 Smart caching system for test discovery
- 🎯 Parallel test execution with automatic CPU detection
- 🏃 Optimized executor with batching and pre-compilation
- 🔍 Multiple parsers: regex (fast) and AST (accurate)
- 🎨 Beautiful terminal output with progress bars
- 📦 Zero configuration required
- 🔧 Basic pytest compatibility:
  - Test discovery (`def test_*`, `class Test*`)
  - Async tests
  - Basic markers (`@pytest.mark.skip`, `@pytest.mark.xfail`)
  - Basic fixtures (`capsys`, `tmp_path`, `monkeypatch`)
  - Test filtering (`-k` pattern)
  - Marker expressions (`-m "not slow"`)
  - Fail fast (`-x` flag)

### Performance Features
- Intelligent batching by module
- Pre-compiled Python code generation
- Minimal subprocess overhead
- Persistent discovery cache

### Documentation
- Comprehensive README with benchmarks
- Migration guide from pytest
- Fixture system documentation
- Marker documentation
- Project structure guide

### Experimental
- Persistent worker pool (disabled by default)

## [0.1.0] - 2024-01-20

### Added
- Initial release with core functionality
- Blazing fast test discovery (88x faster than pytest)
- Fast test execution (2.1x faster than pytest)
- Smart discovery caching for repeated runs
- Parallel test execution with customizable workers
- Beautiful CLI with progress bars and colored output
- Full marker support (pytest.mark.* and fastest.mark.*)
- Multiple discovery parsers (regex and AST)
- Python bindings via PyO3
- Basic fixture support

### Known Limitations
- Limited fixture support compared to pytest
- No configuration file support yet
- No plugin system
- JUnit XML output not implemented

## [0.1.0] - 2024-01-XX (Coming Soon)

### Added
- Initial release of Fastest test runner
- 88x faster test discovery than pytest
- 2.1x faster test execution than pytest
- Parallel test execution with `-n` flag
- Smart caching for repeated runs
- Tree-sitter AST parser option with `--parser ast`
- Full marker support (`-m` flag) for both pytest and fastest markers
- Fixture system with discovery and dependency resolution
- Built-in fixtures: tmp_path, capsys, monkeypatch
- Python API via PyO3 bindings
- Colored CLI output with progress bars
- `-k` pattern filtering
- Verbose mode with `-v`
- Cross-platform installers for macOS, Linux, and Windows

### Performance
- Test discovery: Up to 88x faster than pytest
- Test execution: 2.1x faster than pytest  
- Memory usage: ~50% less than pytest
- Startup time: <100ms for small test suites

### Known Limitations
- Config file support (pytest.ini, pyproject.toml) not yet implemented
- Plugin system not yet available
- Some advanced pytest features not supported
- Parametrized tests coming in next release 

[0.1.1]: https://github.com/derens99/fastest/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/derens99/fastest/releases/tag/v0.1.0 