# [2.3.0](https://github.com/derens99/fastest/compare/v2.2.0...v2.3.0) (2026-03-08)


### Features

* phase 5 full pytest compatibility - setup/teardown, capfd, scope caching, CLI flags ([#33](https://github.com/derens99/fastest/issues/33)) ([6fd83af](https://github.com/derens99/fastest/commit/6fd83aff1efbc52c9a09740c45160d817b1f0650))

# [2.2.0](https://github.com/derens99/fastest/compare/v2.1.1...v2.2.0) (2026-03-08)


### Features

* phase 3 compatibility improvements ([#30](https://github.com/derens99/fastest/issues/30)) ([704c82b](https://github.com/derens99/fastest/commit/704c82b8ccd2640e242b127525a39403eed56191))
* phase 4 pytest compatibility - nested classes, --lf/--ff, autouse, pytest.param, caplog ([#31](https://github.com/derens99/fastest/issues/31)) ([1df691c](https://github.com/derens99/fastest/commit/1df691cf0786bbc86dca97bcc48533302c16a2f1))

## [2.1.1](https://github.com/derens99/fastest/compare/v2.1.0...v2.1.1) (2026-03-08)


### Bug Fixes

* critical bug fixes, security hardening, and new features ([#21](https://github.com/derens99/fastest/issues/21)) ([0b5d569](https://github.com/derens99/fastest/commit/0b5d5690c2fd309924c0100ffae32be5fd8f1c2d))

# [2.1.0](https://github.com/derens99/fastest/compare/v2.0.0...v2.1.0) (2026-03-08)


### Features

* add CLI flags, fixture improvements, and better error reporting ([#20](https://github.com/derens99/fastest/issues/20)) ([2ee3ef4](https://github.com/derens99/fastest/commit/2ee3ef4cb13d66bed8318a9a0bddb308964ccbd5))

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.0.0] - 2026-03-02

### Added
- **Complete rewrite** of the test runner from scratch
- **3-crate architecture**: fastest-core, fastest-execution, fastest-cli
- **AST-based discovery**: rustpython-parser for reliable Python test parsing
- **Hybrid execution engine**: PyO3 in-process for small suites (<=20 tests), subprocess pool with crossbeam work-stealing for larger suites
- **Plugin system**: Trait-based with 4 built-in plugins (fixture, marker, reporting, capture) and pytest-compatible hooks
- **Fixture system**: Topological sort dependency resolution, conftest.py discovery, scope-aware caching, 8 built-in fixtures
- **Marker system**: Recursive descent expression parser for `-m "slow and not integration"`
- **Keyword filtering**: Boolean expression support for `-k "test_add or test_sub"`
- **Parametrize expansion**: Full `@pytest.mark.parametrize` support with cross-product
- **Incremental testing**: git2-based change detection with blake3 hashing and LRU result cache
- **Watch mode**: File system monitoring via notify with debounced re-execution
- **Full CLI**: clap-based with discover subcommand, pretty/JSON/count/JUnit XML output, progress bars
- **CI/CD**: GitHub Actions for CI (check/test/build) and semantic-release for automated versioning and binary releases
- **Cross-platform binaries**: Linux (x86_64, aarch64), macOS (x86_64, aarch64, universal), Windows (x86_64)

### Changed
- Reduced from 6 crates to 3 (removed fastest-advanced, fastest-plugins, fastest-plugins-macros)
- Replaced tree-sitter with rustpython-parser for more reliable AST parsing
- Removed ~30K lines of dead code, stubs, and unused dependencies
- Removed disabled JIT transpiler (Cranelift), simulated zero-copy, and experimental modules

### Removed
- PyPI publishing (binary-only distribution for now)
- Python wrapper package (fastest-runner)
- 15+ unused heavy dependencies (Cranelift, bumpalo, string-interner, dashmap, etc.)
