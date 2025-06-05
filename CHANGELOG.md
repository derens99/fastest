## [0.3.16](https://github.com/derens99/fastest/compare/v0.3.15...v0.3.16) (2025-06-05)


### Bug Fixes

* cargo2 ([4c5e444](https://github.com/derens99/fastest/commit/4c5e4449ba54c36df298f2cda1445bf20f03e47f))

## [0.3.15](https://github.com/derens99/fastest/compare/v0.3.14...v0.3.15) (2025-06-05)


### Bug Fixes

* cargo ([d06614e](https://github.com/derens99/fastest/commit/d06614e83dac41057d22081b2f81318ef5f60f69))

## [0.3.14](https://github.com/derens99/fastest/compare/v0.3.13...v0.3.14) (2025-06-05)


### Bug Fixes

* pain ([ac16d89](https://github.com/derens99/fastest/commit/ac16d891631853cde53bb7989e9da050d9172ca1))

## [0.3.13](https://github.com/derens99/fastest/compare/v0.3.12...v0.3.13) (2025-06-05)


### Bug Fixes

* dist version ([123a6f0](https://github.com/derens99/fastest/commit/123a6f0d7f17fc815c1bb6df230e22607e1e41e5))

## [0.3.12](https://github.com/derens99/fastest/compare/v0.3.11...v0.3.12) (2025-06-05)


### Bug Fixes

* semver11 ([dc17b63](https://github.com/derens99/fastest/commit/dc17b63a34f01e3e27a5625946342fa2d6d66181))

## [0.3.11](https://github.com/derens99/fastest/compare/v0.3.10...v0.3.11) (2025-06-05)


### Bug Fixes

* json fix ([c873246](https://github.com/derens99/fastest/commit/c873246c4769ff29081cda2630a830054ba3b9c1))

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Class-Based Test Support** ⭐⭐⭐
  - Full discovery of test classes matching `TestClass` pattern
  - Proper method execution with `self` parameter handling
  - Support for `setUp` and `tearDown` methods
  - Async test methods in classes
  - Test class inheritance
  - Nested test discovery within class bodies
- **Enhanced Test Discovery**
  - Improved tree-sitter parser with `collect_functions_in_class()`
  - Better handling of decorated methods in classes
  - Support for `testMethodName` pattern (without underscore)

### Fixed
- Class methods not being discovered in test classes
- Test methods requiring proper class instantiation
- Async class methods not being detected

### Known Issues
- Parametrized tests incorrectly map index as value instead of actual parameter values
- Markers like `@pytest.mark.skip` and `@pytest.mark.xfail` not implemented
- Fixture scopes limited to function scope only
- `capsys` fixture not fully implemented

## [0.2.0] - 2024-05-28

### Added
- **Intelligent Execution Strategy**: Automatically selects optimal execution mode based on test suite size
  - InProcess (PyO3): **47x faster** for ≤20 tests
  - WarmWorkers: **5x faster** for 21-100 tests  
  - FullParallel: **3x faster** for >100 tests
- **Self-Update System**: `fastest update` command for easy updates
- **Comprehensive Release Workflow**: Multi-platform binary releases (Linux, macOS, Windows)
- **Enhanced Fixture System**: Lock-free caching, graph-based dependencies, parallel execution
- **Binary Protocol Support**: MessagePack for 2-3x faster fixture serialization
- **Version Management**: Automated version bumping script
- **Plugin Architecture**: Extensible hook system for pytest compatibility
- **Coverage Integration**: Built-in coverage.py support
- **Watch Mode**: Auto-rerun tests on file changes
- **IDE Integration**: VS Code diagnostics and code execution

### Changed
- **Fixture System Optimization**: 100x performance improvement with:
  - Lock-free concurrent caching (DashMap)
  - Graph-based dependency resolution (O(V+E))
  - Parallel fixture execution with CPU-aware batching
  - Smart code template caching
  - LRU cache eviction for memory efficiency
- **Test Discovery**: Tree-sitter based parsing now 10x faster
- **Error Handling**: Better error messages and recovery
- **CLI**: More intuitive command structure and options

### Performance
- **Small suites (≤20 tests)**: 47x faster than pytest
- **Medium suites (21-100 tests)**: 5x faster than pytest
- **Large suites (>100 tests)**: 3x faster than pytest
- **Fixture execution**: 100x faster cache hits
- **Discovery**: 10x faster with tree-sitter optimizations
- **Memory**: Constant memory usage with LRU eviction

### Documentation
- Updated CLAUDE.md with complete project context
- Comprehensive release workflow documentation
- Installation methods for all platforms
- Performance benchmark results

## [0.1.0] - 2024-11-01

Initial release of Fastest - A blazing fast Python test runner built with Rust.

### Added
- Basic test discovery and execution
- Tree-sitter based Python parsing
- Parallel test execution
- Basic pytest compatibility
- Cross-platform support
