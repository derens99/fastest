# Fastest Project Tests

This directory contains tests for the Fastest project itself. Files here should
pass under normal pytest. Pytest compatibility fixtures, including intentional
failure cases and runner input suites, live separately under
`pytest-compat-suite/`.

## Layout

- `tests/compatibility/` - small compatibility checks used by project-level workflows
- `tests/integration/` - integration coverage for discovery, execution, fixtures, markers, plugins, parsing, and current runner behavior
- `tests/performance/` - Python suites used to exercise performance-sensitive paths
- `tests/large_suite/` - generated large-suite inputs for scaling checks
- `tests/test_*.py` - focused smoke and regression checks

## Common Commands

Run Rust workspace checks:

```bash
cargo test --workspace
```

Run Fastest against project-level Python tests:

```bash
cargo build --release
./target/release/fastest tests/
```

Run pytest as a reference runner:

```bash
uv run pytest tests -q
```

## Compatibility Suites

Use `pytest-compat-suite/` for broader pytest feature coverage:

```bash
./target/release/fastest pytest-compat-suite/core/basic/
./target/release/fastest pytest-compat-suite/features/fixtures/
./target/release/fastest pytest-compat-suite/edge-cases/
```

When adding new compatibility examples, put them in `pytest-compat-suite/` rather than this directory unless they directly test Fastest's own tooling.
