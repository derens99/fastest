# Fastest Feasibility Assessment And Roadmap

Last updated: 2026-06-20

## Executive Verdict

Fastest is feasible as a Rust-backed Python test runner, but it is not currently practical as a drop-in pytest replacement. The project has a plausible technical core: Rust CLI, PyO3 integration, test discovery, execution strategies, plugin scaffolding, and a maintained compatibility suite. The current implementation can run simple module, class, async, filtered, and failing tests through the CLI.

The practical gap is correctness and product honesty. Current documentation and CLI copy overstate compatibility, performance validation, and advanced features. The next roadmap should exist, but it should be a stabilization roadmap, not a feature-expansion roadmap.

## Current Evidence

Commands run against the current worktree:

| Area | Evidence | Result |
|------|----------|--------|
| Lint and formatting | `make lint` | Passed after the discovery/cache/test-layout/roadmap changes in this assessment pass. |
| Core discovery regression | `PYO3_PYTHON=$(command -v python3.12 || command -v python3) cargo test -p fastest-core test_discover_simple_tests` | Passed after fixing Python discovery namespace and class-method double counting. |
| CLI integration | `PYO3_PYTHON=$(command -v python3.12 || command -v python3) cargo test -p fastest-cli --test integration_test` | Passed: 25 passed, including regressions for same-stem files, pytest import fallback, `pytest.param` fallback support, `pytest.raises(match=...)`, conftest autouse `request.instance`, xfailed setup handling, skipif expression evaluation, XPASS/xfail exit semantics, static parametrized expressions, pytest hook decorators, mocker helpers, request/config/cache helpers, event loop fixture support, unittest.mock fixture scanning, and class teardown before following module tests. |
| Full Rust tests | `PYO3_PYTHON=$(command -v python3.12 || command -v python3) cargo test --workspace` | Passed after fixing conftest parsing, fixture dependency ordering, cache compression testing, timeout parsing, non-x86 JIT gating, and brittle debug performance assertions. |
| Python package tests | `uv run pytest tests -q` | Passed: 13,888 passed, 14 skipped, 2 xfailed, 4 xpassed. Stale extension demos and intentional failing input suites were moved out of pytest's project test tree. |
| Compatibility report harness | `make compat-report COMPAT_SUITES="core/basic features/fixtures"` | Generates a text report and machine-readable JSON at `target/compatibility-report.json`. |
| Full compatibility baseline | `make compat-report-all` | Generates `target/compatibility-report-all.json` without failing on known gaps and with a bounded per-suite timeout. Passing categories: `comprehensive`, `core/basic`, `core/classes`, `edge-cases`, `examples`, `features/assertion-introspection`, `features/fixtures`, `features/lifecycle`, `features/markers`, `features/parametrization`, `features/parametrize`, `features/plugins`, `features/setup-teardown`, and `third-party/plugins`. |
| Basic compatibility suite | `PYO3_PYTHON=$(command -v python3.12 || command -v python3) cargo run -p fastest-cli -- pytest-compat-suite/core/basic --no-cache` | Passed: 39 passed, 1 xfailed after exact-path module loading, pytest fallback import support, and parametrized discovery expansion. |
| Fixture compatibility suite | `PYO3_PYTHON=$(command -v python3.12 || command -v python3) cargo run -p fastest-cli -- pytest-compat-suite/features/fixtures --no-cache` | Passed: 80 passed, 3 xfailed after exact-path loading, class teardown transition fixes, and real plugin package availability. |
| Comprehensive compatibility suite | `PYO3_PYTHON=$(command -v python3.12 || command -v python3) cargo run -p fastest-cli -- pytest-compat-suite/comprehensive --no-cache` | Passed: 309 passed, 4 skipped, 37 xfailed, 1 xpassed after suite hygiene, `pytest.raises(match=...)` support, and conftest autouse `request.instance` support. |
| Edge-case compatibility suite | `PYO3_PYTHON=$(command -v python3.12 || command -v python3) cargo run -p fastest-cli -- pytest-compat-suite/edge-cases --no-cache` | Passed: 132 passed, 5 skipped, 34 xfailed, 5 xpassed after moving intentional failure-reporting examples out of the normal pass/fail signal. |
| Assertion introspection compatibility suite | `PYO3_PYTHON=$(command -v python3.12 || command -v python3) cargo run -p fastest-cli -- pytest-compat-suite/features/assertion-introspection --no-cache` | Passed as an intentional output-inspection suite: 26 xfailed, 1 xpassed after module-level `pytestmark` propagation. |
| Marker compatibility suite | `PYO3_PYTHON=$(command -v python3.12 || command -v python3) cargo run -p fastest-cli -- pytest-compat-suite/features/markers --no-cache` | Passed: 63 passed, 30 skipped, 18 xfailed, 6 xpassed after skipif expression, strict XPASS, and xfail `raises` handling fixes. |
| Parametrize compatibility suite | `PYO3_PYTHON=$(command -v python3.12 || command -v python3) cargo run -p fastest-cli -- pytest-compat-suite/features/parametrize --no-cache` | Passed: 240 passed, 1 skipped, 1 xfailed, 1 xpassed after static `range`, `float`, `set`, and string repetition support plus indirect fixture cleanup. |
| Plugin compatibility suite | `PYO3_PYTHON=$(command -v python3.12 || command -v python3) cargo run -p fastest-cli -- pytest-compat-suite/features/plugins --no-cache` | Passed: 266 passed, 2 skipped, 7 xfailed after fixture-scanner hardening, pytest hook decorator shims, mocker helper support, request/config/cache helpers, event loop fixture support, and real pytest-wrapped async fixture support. |
| Third-party plugin smoke | `make plugin-smoke` | Passed: Fastest reports 266 passed, 2 skipped, 7 xfailed for `features/plugins`; Fastest reports 4 passed for `third-party/plugins`; pytest reports 4 passed for the third-party smoke suite. This validates installed `pytest-asyncio`, `pytest-cov`, `pytest-mock`, `pytest-timeout`, and `pytest-xdist` packages plus the supported shim subset. |
| Coverage flag | `PYO3_PYTHON=$(command -v python3.12 || command -v python3) cargo run -p fastest-cli -- --coverage pytest-compat-suite/core/basic --no-cache` | Test execution passed: 39 passed, 1 xfailed. Output still only labels the run as `with coverage framework`, without proving useful coverage reporting. |
| PyO3 linking | `cargo run -p fastest-cli -- --help` without `PYO3_PYTHON` | Failed on this machine trying to link an unavailable system Python. With `PYO3_PYTHON=$(command -v python3.12 || command -v python3)`, CLI help runs. |
| Strategy behavior | `uv run pytest tests/integration/test_execution_strategies.py -q` after building `target/debug/fastest` | Passed after updating the test to current behavior: all suite sizes use the compatibility-first `UltraInProcess` path. Threshold-based `HybridBurst` and `WorkStealing` behavior is not the active verified path. |
| Maintainability | Rust source line count scan | `core/execution.rs` is about 4,300 lines, `core/strategies.rs` about 2,500, `fixture_integration.rs` about 2,000. These are major ownership and review risks. |

## Feasibility

### Feasible

Fastest can be built into a useful tool if the scope is narrowed and verified incrementally. The strongest feasible product is:

- A fast runner for straightforward pytest-style tests.
- A discovery and execution engine that handles functions, classes, async tests, skip/xfail, and common builtin fixtures.
- A performance-focused alternative for projects willing to validate compatibility suite by suite.

### Not Yet Feasible As Marketed

The current state does not support claims like "drop-in pytest replacement", "91% pytest compatible", "coverage ready", or "plugin system ready" without stronger evidence from every compatibility category.

The hard parts are not Rust speed. They are pytest semantics:

- Import/module identity and execution context.
- Fixture scope, fixture dependency, parametrized fixture, and teardown semantics.
- Conftest discovery and hook behavior.
- Assertion rewriting and traceback fidelity.
- Plugin compatibility.
- Configuration compatibility.

## Practicality

Fastest is currently practical for local experimentation and targeted development. It is not practical for production migration or broad user adoption yet.

Reasons:

- The project-level Rust and Python gates now pass, and every compatibility category is green in the generated Fastest report.
- Plugin compatibility has a passing suite-level gate and a narrow third-party package smoke gate, but broader plugin ecosystem claims still need more real package validation.
- Current verified execution behavior is compatibility-first `UltraInProcess` across suite sizes; threshold-based parallel strategy claims need to be rebuilt and revalidated.
- Advanced feature flags are exposed before they are end-to-end complete.
- Build behavior depends on setting a linkable `PYO3_PYTHON` in this environment.
- Older docs outside the current README, CLI help, docs landing page, workplan, and roadmap may still overpromise relative to verified behavior.

## Product Decision

Continue the project, but reset the public positioning:

- Do not call it production ready.
- Do not advertise percentage compatibility until generated from a reproducible compatibility harness.
- Do not market coverage, incremental, watch, prioritization, or plugin compatibility as complete until each has a passing end-to-end gate.
- Treat "pytest-compatible core runner" as the only near-term product.

The roadmap should exist because the project is still feasible, but the roadmap should be evidence-gated.

## Roadmap

### Phase 0: Make The Project Honest And Reproducible

Goal: establish trust in the repo before expanding behavior.

Required work:

- Update README, docs, and CLI help to distinguish working features from scaffolding. Current status: completed for the main README, CLI, docs landing page, development workplan, benchmark docs, plugin docs, and architecture/status docs.
- Replace stale "91% compatible" claims with current generated compatibility numbers.
- Add a single `make verify` target that runs the accepted gate set. Current status: implemented.
- Standardize PyO3 linking in Makefile, docs, scripts, and CI using a known Python. Current status: Makefile defaults to `python3.12` then `python3`, docs use the same portable export, development scripts export `PYO3_PYTHON`, and CI exports the setup-python interpreter.
- Decide whether the Python import surface is `fastest`, `fastest_runner`, or both; current collected tests now align with `fastest_runner`, while legacy `fastest` API demos live under `examples/legacy-python-api/`.
- Ensure all required source files, including `crates/fastest-core/src/test/discovery/python_introspection.rs`, are tracked.

Exit gates:

- `cargo check --workspace` passes.
- `make lint` passes. Current status: passing.
- `PYO3_PYTHON=$(command -v python3.12 || command -v python3) cargo test --workspace` passes. Current status: passing.
- `uv run pytest tests -q` passes. Current status: passing after moving stale demos and intentional compatibility input suites out of collected tests.
- `make compat-report COMPAT_SUITES="core/basic features/fixtures"` produces `target/compatibility-report.json`. Current status: implemented and passing.
- `make compat-report-all` produces `target/compatibility-report-all.json`. Current status: implemented; every discovered compatibility category has a passing Fastest summary.
- README and CLI help no longer advertise unverified feature completion. Current status: completed for the main README, CLI, docs landing page, benchmark docs, plugin docs, and architecture/status docs.

### Phase 1: Fix Discovery-To-Execution Correctness

Goal: make every discovered test resolvable and runnable in its real module context.

Required work:

- Fix module loading so tests under nested paths execute from the discovered file, not an ambiguous module name.
- Add regression tests for same-stem files in different directories.
- Add regression tests for relative imports, package tests, and conftest-adjacent tests.
- Ensure class method IDs map to actual class objects and methods.
- Keep Python discovery and Rust execution in one explicit contract with tests.

Exit gates:

- `pytest-compat-suite/core/basic` has no "module has no attribute" failures. Current status: passing.
- CLI integration tests include nested same-stem files, classes, async functions, filtered tests, pytest import fallback, and class teardown transition coverage.
- Discovery output and execution input use the same canonical test ID format.

### Phase 2: Establish Compatibility Harness And Baseline Metrics

Goal: replace hand-written compatibility claims with generated evidence.

Required work:

- Add a compatibility runner that executes every category in `pytest-compat-suite/`.
- Store machine-readable results under an ignored artifacts directory.
- Categorize failures by feature: import, fixture, marker, parametrization, async, class, config, plugin.
- Add a command that prints current compatibility by category.
- Track pytest baseline behavior for the same suite.

Exit gates:

- Compatibility report is reproducible from one command.
- Docs cite the generated report, not hand-maintained percentages.
- Basic category reaches at least 95% pass rate before any production-readiness claim.

### Phase 3: Fixtures Before Plugins

Goal: make common pytest fixture behavior reliable before claiming plugin compatibility.

Required work:

- Fix conftest parsing query errors.
- Fix fixture dependency graph behavior and test expectations.
- Implement accurate function, class, module, session, and package scope cache keys.
- Complete yield fixture teardown with verified ordering.
- Complete `request`, `tmp_path`, `capsys`, and `monkeypatch` behavior.
- Add focused tests for parametrized fixtures and indirect parametrization.

Exit gates:

- `pytest-compat-suite/features/fixtures` reaches at least 90% pass rate. Current status: passing with 80 passed and 3 xfailed.
- All fixture lifecycle tests pass in `cargo test --workspace`. Current status: passing.
- Fixture failures are structured and actionable.

### Phase 4: Configuration, Markers, And Pytest Semantics

Goal: support the pytest behavior users expect before adding new product features.

Required work:

- Implement pytest config precedence for `pyproject.toml`, `pytest.ini`, `setup.cfg`, and `tox.ini`.
- Implement `addopts` or explicitly document unsupported behavior.
- Replace placeholder skipif evaluation with safe, correct behavior or clear unsupported diagnostics.
- Validate marker selection semantics against pytest.
- Improve assertion failure output without compromising correctness.

Exit gates:

- Marker and config compatibility categories pass at an agreed threshold.
- Unsupported pytest features fail loudly with a clear reason.

### Phase 5: Performance Revalidation

Goal: measure speed only after correctness is credible.

Required work:

- Rebuild the benchmark suite around verified compatibility categories.
- Compare against pytest on the same test files and same Python environment.
- Separate discovery time, execution time, and total wall time.
- Remove hard-coded "3.9x faster" output from normal test runs unless backed by the current benchmark artifact.
- Publish benchmark scripts and raw outputs.

Exit gates:

- `uv run --extra dev python scripts/benchmarks/official.py --quick --output-dir target/benchmark-artifacts/quick` produces `benchmark_results.{json,md}`.
- README avoids fixed performance numbers unless they link to current generated benchmark data.
- Benchmark methodology includes hardware, Python version, pytest version, warm/cold cache notes, and command lines.

Current status: the quick artifact path has been verified locally. Treat the
numbers as run-specific evidence, not as reusable product claims.

### Phase 6: Advanced Features Only After Core Stability

Goal: prevent scaffolding from becoming user-facing promises.

Required work:

- Decide which advanced features belong in the product: coverage, incremental, watch, prioritization, plugins.
- For each kept feature, add one end-to-end user story and one pass/fail gate.
- Hide or mark incomplete CLI options until their gates pass.
- Remove experimental/JIT/native-transpiler code from default builds if it is not part of the near-term product.

Exit gates:

- `--coverage` produces a real report and has tests.
- `--watch` runs a verified watch loop or is hidden.
- `--incremental` demonstrably changes selected tests from a git diff.
- Plugin claims are limited to plugins and behaviors with passing compatibility or smoke gates.

## Near-Term Priority List

1. Keep stale claims out of older topic docs and generated benchmark artifacts.
2. Keep PyO3 linking standardized in docs, scripts, Makefile, and CI.
3. Rebuild and revalidate threshold-based execution strategies after correctness stabilizes.
4. Keep experimental advanced features explicitly labeled in CLI help until each has an end-to-end gate.
5. Expand real third-party plugin smoke tests before broad plugin ecosystem claims.

## Non-Goals For Now

- AI-powered test selection.
- GPU acceleration.
- Distributed execution.
- Enterprise dashboards.
- Broad plugin marketplace support.
- More execution strategies before correctness is stable.

These ideas may be interesting later, but they distract from the current blocker: pytest-compatible correctness.

## Success Definition

Fastest becomes practical when a new contributor can run one documented verification command, see a passing core test suite, see generated compatibility metrics, and use the CLI on a real small pytest project without unexplained import, fixture, or configuration failures.
