# Fastest Development Workplan

Last updated: 2026-06-20

This workplan tracks the cleanup branch at a high level. The detailed source of
truth is [the roadmap](../reference/roadmap.md), which includes current command
evidence and compatibility-suite results.

## Current Status

- Rust workspace tests pass with `PYO3_PYTHON=$(command -v python3.12 || command -v python3) cargo test --workspace`.
- Python project tests pass with `uv run pytest tests -q`.
- Lint and formatting pass with `make lint`.
- The accepted local gate is `make verify`.
- Compatibility reports are generated with `make compat-report`.
- Full baseline reports are generated with `make compat-report-all`.
- Comprehensive compatibility passes: `pytest-compat-suite/comprehensive` reports 309 passed, 4 skipped, 37 xfailed, and 1 xpassed under Fastest.
- Basic compatibility passes: `pytest-compat-suite/core/basic` reports 39 passed and 1 xfailed.
- Class compatibility passes: `pytest-compat-suite/core/classes` reports 102 passed, 1 skipped, and 2 xfailed.
- Edge-case compatibility passes: `pytest-compat-suite/edge-cases` reports 132 passed, 5 skipped, 34 xfailed, and 5 xpassed under Fastest.
- Assertion introspection compatibility passes as an intentional output-inspection suite: `pytest-compat-suite/features/assertion-introspection` reports 26 xfailed and 1 xpassed.
- Fixture compatibility passes: `pytest-compat-suite/features/fixtures` reports 80 passed and 3 xfailed.
- Marker compatibility passes: `pytest-compat-suite/features/markers` reports 63 passed, 30 skipped, 18 xfailed, and 6 xpassed.
- Simple legacy parametrization compatibility passes: `pytest-compat-suite/features/parametrization` reports 14 passed.
- Expanded parametrization compatibility passes: `pytest-compat-suite/features/parametrize` reports 240 passed, 1 skipped, 1 xfailed, and 1 xpassed.
- Plugin compatibility passes: `pytest-compat-suite/features/plugins` reports 266 passed, 2 skipped, and 7 xfailed under Fastest.
- Third-party plugin smoke passes: `pytest-compat-suite/third-party/plugins` reports 4 passed under Fastest, and `uv run pytest pytest-compat-suite/third-party/plugins -q` reports 4 passed.

Fastest is still experimental. Do not treat it as a production-ready drop-in
pytest replacement until compatibility and benchmark reports are generated from
repeatable harnesses.

## Near-Term Priorities

1. Keep stale performance and compatibility claims out of docs and generated benchmark artifacts. Current status: stale fixed-speed and fixed-percent claims have been replaced with evidence-gated language.
2. Keep PyO3 linking standardized in docs, scripts, Makefile, and CI. Current status: examples and scripts use `PYO3_PYTHON=$(command -v python3.12 || command -v python3)`, and CI exports the setup-python interpreter.
3. Revalidate performance strategies only after correctness stays green.
4. Keep experimental advanced features visible only with explicit experimental labeling in CLI help and docs. Current status: CLI advanced flags are labeled experimental.
5. Expand third-party plugin smoke coverage before broad plugin ecosystem claims. Current status: a narrow smoke gate covers installed `pytest-asyncio`, `pytest-cov`, `pytest-mock`, `pytest-timeout`, and `pytest-xdist` package availability plus supported shim behavior.

## Development Rules

- Keep project tests that must pass under pytest in `tests/`.
- Put pytest compatibility input suites, including intentional failures, under `pytest-compat-suite/`.
- Put user and developer docs under `docs/`.
- Use `uv` for Python commands.
- Treat advanced features such as coverage, incremental mode, watch mode, prioritization, and plugins as experimental unless a passing end-to-end gate says otherwise.
