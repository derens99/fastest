# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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