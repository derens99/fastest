# Fastest Roadmap v3

> Comprehensive roadmap based on a full codebase audit of v2.3.0 (March 2026).
> Organized into 6 phases, ordered by impact and dependency.

---

## Current State (v2.3.0)

- **9,282 lines of Rust** across 3 crates (core, execution, cli)
- **~53K lines of Python** (harness, testing files, benchmarks, scripts)
- **144 passing tests** (Rust unit + integration), 0 failures
- **~91% pytest compatibility** (claimed), real-world ~80-85% due to execution gaps
- CI: fmt + clippy + test (3 platforms) + security audit + release pipeline
- Semantic-release to PyPI with 5-target binary matrix

### What Works Well
- AST-based test discovery (rustpython-parser) — fast and reliable
- Parallel subprocess pool with crossbeam work-stealing
- Marker expression parser (recursive descent for `-m`/`-k`)
- Config loading from 4 formats (pyproject.toml, pytest.ini, setup.cfg, tox.ini)
- Parametrize expansion with cross-product and pytest.param()
- CI/CD pipeline with semantic-release and multi-platform binaries

### What Needs Work
- Execution correctness (test isolation, timeout enforcement, fixture lifecycle)
- Streaming output (spinner blocks until all results arrive)
- Integration test coverage (only 8 Rust integration tests, none for execution)
- Infrastructure (broken Makefile targets, stale docs, missing CI gates)
- Code duplication between in-process and subprocess backends (~400 lines)

---

## Phase 1: Foundation Fixes (Critical Bugs & Infrastructure)

**Goal:** Fix things that are currently broken or could cause data loss/user-facing failures.
**Effort:** ~3-5 days | **Risk:** Low (mostly fixes, no new features)

### 1.1 Fix Broken Infrastructure
- [ ] **Fix Makefile binary name** — `fastest-cli` → `fastest` in 3 targets (quick-check, test-integration, release)
- [ ] **Fix aarch64-linux cross-compile** — Add cross-compilation toolchain to `semantic-release.yml` `upload-binaries` job
- [ ] **Add CI gate before release** — Branch protection rule or `workflow_run` dependency so broken commits can't trigger PyPI publish
- [ ] **Fix `version.json`** — Update stale manifest from v2.0.0 to v2.3.0
- [ ] **Fix CHANGELOG dual-format** — Remove manual v2.0.0 section that conflicts with semantic-release auto-generation

### 1.2 Fix Critical Execution Bugs
- [ ] **`_PytestApprox.__eq__` bug** — `__builtins__['abs']` fails when `__builtins__` is a module (subprocess harness line 44). Replace with `abs()` builtin call directly
- [ ] **`pytest.approx` shim mismatch** — In-process shim returns raw value (no tolerance check). Copy the `_PytestApprox` class from `worker_harness.py` into `inprocess.rs` shim
- [ ] **`--ignore` path bug** — Uses `starts_with` string matching, not path-component matching. `--ignore tests/t` incorrectly matches `tests/test_other.py`. Fix: use `PathBuf::starts_with`

### 1.3 Clean Up Stale Documentation
- [ ] **Delete `docs/CHANGELOG.md`** — Stale v0.x changelog from removed codebase
- [ ] **Rewrite `docs/ROADMAP.md`** — Replace with this document
- [ ] **Delete `docs/NEXT_STEPS.md`** — Stale January 2025 planning doc
- [ ] **Fix `docs/DEVELOPMENT.md`** — Remove references to `fastest-python` crate and non-existent scripts
- [ ] **Fix `docs/RELEASE.md`** — Replace manual process with automated pipeline description
- [ ] **Fix `CONTRIBUTING.md`** — Python 3.8 → 3.9, remove `maturin develop` reference
- [ ] **Fix `benchmarks/README.md`** — Remove references to non-existent benchmark scripts

### 1.4 Dependency Cleanup
- [ ] **Remove `tokio` dev-dependency** from fastest-execution (unused, `features = ["full"]`)
- [ ] **Remove duplicate `crossbeam`** — Keep only `crossbeam-deque`, remove umbrella crate
- [ ] **Relax pyo3 pin** — `"=0.25.0"` → `"~0.25"` (allow patch releases)
- [ ] **Add `publish = false`** to all three `Cargo.toml` files (prevent accidental crates.io publish)

---

## Phase 2: Test Isolation & Correctness (Execution Layer)

**Goal:** Make test execution actually correct — tests should not interfere with each other.
**Effort:** ~5-8 days | **Risk:** Medium (changes execution semantics)

### 2.1 Subprocess Timeout Enforcement
- [ ] **Implement real timeout via process kill** — Spawn a watchdog thread per worker that calls `child.kill()` after deadline. Currently `read_line` blocks forever on infinite loops
- [ ] **Expose `--timeout` CLI flag** — The execution layer supports `TimeoutConfig` but CLI never surfaces it

### 2.2 Worker Crash Recovery
- [ ] **Detect worker death** — On EOF from stdout, call `child.try_wait()` to get exit status. Report signal/exit code in error message
- [ ] **Restart crashed workers** — On unexpected EOF, spawn a fresh worker and re-enqueue the current test. Cap restarts at 3 per worker
- [ ] **Capture stderr** — Replace `Stdio::null()` with a background drain thread per worker. Include last 8KB of stderr in error messages on crash

### 2.3 Protocol Robustness
- [ ] **Handle non-JSON stdout lines** — Tests that print directly to fd 1 cause result misalignment. Add framing: either length-prefix or sentinel delimiters, or communicate results on a dedicated pipe (fd 3)

### 2.4 Fix Module Reload Issues (Subprocess)
- [ ] **Cache module objects per file path** — Don't `importlib.reload()` on every test. Only reimport when switching to a new test file
- [ ] **Fix `setup_module` flag reset** — `_fastest_setup_done` attribute is lost on reload. Track in worker-level dict instead
- [ ] **Cache conftest modules** — Stop re-executing conftest.py from disk on every fixture lookup. Cache keyed by file path

### 2.5 Fix Setup/Teardown Distribution
- [ ] **Group tests by class for work distribution** — `setup_class`/`teardown_class` must run once per class, but currently each worker calls it independently. Partition tests by `(path, class_name)` and assign each partition to a single worker

### 2.6 In-Process Isolation (If Keeping In-Process Mode)
- [ ] **Snapshot and restore `sys.path`** between tests
- [ ] **Snapshot and restore `sys.modules`** between tests
- [ ] **Snapshot and restore `os.environ`** between tests
- [ ] **Pass isolated globals/locals dicts** to `py.run()` instead of sharing `__main__`
- [ ] **OR: Default to subprocess mode** — Set threshold to 0, make in-process opt-in via `--inprocess` flag

---

## Phase 3: Streaming Output & UX Polish

**Goal:** Make the user experience match or exceed pytest's output quality.
**Effort:** ~3-5 days | **Risk:** Low (output-only changes)

### 3.1 Streaming Test Results
- [ ] **Expose a channel-based API from SubprocessPool** — Send `TestResult` as each worker completes instead of collecting into a Vec
- [ ] **Live progress bar** — Replace spinner with an `indicatif` progress bar showing `[15/100] tests passed...` or pytest-style `..F...sxF.`
- [ ] **Print results as they arrive** — In verbose mode, show each test's pass/fail immediately instead of waiting for all to complete

### 3.2 Output Parity with pytest
- [ ] **Show captured stdout/stderr in FAILURES section** — Currently only shows error message, not captured output. Add `--- Captured stdout call ---` headers
- [ ] **Terminal-width-aware formatting** — Use `terminal_size` crate for separator widths instead of hardcoded 60
- [ ] **Center FAILURES header** — `format!("{:=^width$}", " FAILURES ", width = term_width)`
- [ ] **Fix `--tb=short`** — Show file + line reference + assertion, not just the last exception line
- [ ] **Add `--tb=line` mode** — Single-line failure summary
- [ ] **Header line with context** — Show Python version, platform, rootdir, config file in header (like pytest)
- [ ] **Remove redundant `(N total)`** from summary line
- [ ] **Fix `--quiet` separator lines** — Skip decorative separators when `--quiet` is active

### 3.3 Node-ID Path Syntax
- [ ] **Parse `::` in path arguments** — `fastest tests/test_math.py::TestCalc::test_add` should work like pytest. Split on `::` to separate file path from class/function selectors

### 3.4 Introspection Commands
- [ ] **`--markers`** — List all registered markers from config
- [ ] **`--fixtures`** — List all available fixtures with scope info
- [ ] **`--collect-only` tree output** — Group tests by module and class instead of flat list

---

## Phase 4: Core Correctness (Discovery & Fixtures)

**Goal:** Fix the core data model to handle real-world pytest projects correctly.
**Effort:** ~8-12 days | **Risk:** High (changes fundamental data structures)

### 4.1 Hierarchical Conftest Scoping
- [ ] **Store fixtures with directory scope** — Replace flat `HashMap<String, Fixture>` with `Vec<(PathBuf, HashMap<String, Fixture>)>` sorted by depth
- [ ] **Resolve fixtures by walking up** — From test file's directory toward root, stopping at first match
- [ ] **Add integration tests** — Two sibling directories with same-named fixtures, verify isolation

### 4.2 Yield Fixture Teardown
- [ ] **Track generator/coroutine state** — For yield fixtures, maintain a reference to the running generator
- [ ] **Run teardown on scope boundary** — When clearing scope cache, `next()` the generator to run post-yield code
- [ ] **Handle teardown errors** — Report fixture teardown failures without losing the test result

### 4.3 Config-Aware Discovery
- [ ] **Thread `Config` into parser** — Replace hardcoded `is_test_function_name` / `is_test_class_name` with `config.is_test_function()` / `config.is_test_class()`
- [ ] **Support glob patterns in `norecursedirs`** — Use `Config::matches_pattern` instead of exact-name matching
- [ ] **Deduplicate overlapping paths** — `testpaths = [".", "tests"]` should not discover tests twice

### 4.4 Parametrize Improvements
- [ ] **Eliminate double file parse** — Thread parsed AST from discovery into expansion step
- [ ] **Support `argnames` as list** — `@pytest.mark.parametrize(["x", "y"], ...)` syntax
- [ ] **Handle nested class parametrize** — Recurse into nested `ClassDef` during expansion
- [ ] **Populate `request.param`** — For parametrized fixtures, set the current parameter value

### 4.5 Incremental Testing Fixes
- [ ] **Conftest changes trigger re-run** — Add `conftest.py` to the config files that trigger full re-runs
- [ ] **Fix `paths_match` false positives** — Canonicalize paths before comparing
- [ ] **Persist `ResultCache` to disk** — Serialize to `.fastest_cache/` for cross-run incremental testing

### 4.6 Error Handling Improvements
- [ ] **Return parse warnings from `discover_tests`** — Replace `eprintln!` with structured `(Vec<TestItem>, Vec<Warning>)` return
- [ ] **Pre-compile config patterns** — Store `Vec<Regex>` instead of recompiling on every `matches_pattern` call
- [ ] **Add `tracing`/`log` integration** — Replace all `eprintln!` warnings with structured logging

---

## Phase 5: Architecture Refactoring

**Goal:** Reduce tech debt, eliminate duplication, improve maintainability.
**Effort:** ~5-8 days | **Risk:** Medium (internal refactoring, no behavior change)

### 5.1 Unify Execution Backends
- [ ] **Extract shared Python runtime module** — Create `fastest_runtime.py` that both backends import. Move pytest shim, fixture setup, marker evaluation, setup/teardown lifecycle into it
- [ ] **Eliminate `build_test_code` string generation** — The in-process executor should call functions in the runtime module, not exec generated strings
- [ ] **Share `skipif` evaluation** — Both backends duplicate the same condition eval logic

### 5.2 CLI Pipeline Refactoring
- [ ] **Extract `RunConfig` struct** — Merge `Cli` and `WatchConfig` into a shared execution config. Eliminate the `WatchConfig` clone with its missing fields
- [ ] **Extract `execute_pipeline()`** — Move the 256-line `run_tests` into composable stages: `filter_tests()`, `execute_with_policy()`, `report_results()`
- [ ] **Single-pass filter chain** — Merge `--ignore`, `--ignore-glob`, `--deselect` into one `.filter()` call

### 5.3 Plugin System Decision
- [ ] **Option A: Remove hollow plugins** — The 4 built-in plugins do nothing. Remove the plugin infrastructure and simplify
- [ ] **Option B: Make plugins real** — Wire `collection_modifyitems` to actually modify the test list, implement real fixture plugin logic, use typed `HookData` enum instead of JSON

### 5.4 Output Separation
- [ ] **Separate `JunitXml` from `OutputFormat`** — `--junit-xml` should be a side-channel, not a format variant. Allow `--output json --junit-xml out.xml` to work correctly
- [ ] **Stream output directly** — Change `format_results` to accept `&mut dyn Write` instead of returning `String`

### 5.5 Fix `apply_addopts`
- [ ] **Use shlex splitting** — Handle quoted strings in addopts properly
- [ ] **Detect user-set values** — Use clap's `value_source()` to avoid overwriting explicit CLI args with addopts
- [ ] **Support all CLI flags** — Currently only handles `-v`, `--verbose`, `--tb`, `--maxfail`, `-x`, `-k`, `-m`

---

## Phase 6: New Features & Growth

**Goal:** Features that would differentiate fastest and drive adoption.
**Effort:** ~10-15 days | **Risk:** Medium (new code, but isolated)

### 6.1 Persistent Workers (Watch Mode Performance)
- [ ] **Keep workers alive between runs** — Don't spawn new Python processes on every watch-triggered rerun
- [ ] **Send "reload" signal** — Workers invalidate module cache on file change, then accept new tests
- [ ] **Cache harness file** — Write once at startup, not per `execute()` call

### 6.2 Real-Time Test Feedback
- [ ] **Subprocess streaming protocol** — Workers send progress events (test started, test completed) in addition to results
- [ ] **Terminal UI** — Optional TUI with live test status, duration, and failure details (ratatui or similar)

### 6.3 Missing pytest Features
- [ ] **`pytest.raises` context manager** — Most-requested missing feature for real-world adoption
- [ ] **`pytest.warns` context manager**
- [ ] **`pytest.importorskip`**
- [ ] **`--stepwise` mode** — Stop on first failure, resume from there on next run
- [ ] **`--rootdir`** — Explicit root directory override
- [ ] **`filterwarnings` config/marker support**

### 6.4 CI Integration Features
- [ ] **Code coverage integration** — `--cov` flag that runs coverage.py alongside tests
- [ ] **JUnit XML improvements** — Add `<system-out>` and `<system-err>` elements, hostname, timestamp
- [ ] **GitHub Actions annotations** — `--github-actions` flag that outputs `::error file=...` annotations
- [ ] **JSON streaming output** — NDJSON format for CI pipeline consumption

### 6.5 Advanced Execution
- [ ] **`--forked` per-test isolation** — Run each test in a fork'd subprocess for complete memory isolation
- [ ] **Test ordering/dependencies** — Support `@pytest.mark.order(N)` and `@pytest.mark.depends(on=[...])`
- [ ] **Distributed execution** — Remote worker support for xdist-style parallelism across machines

### 6.6 Developer Experience
- [ ] **`--profile` flag** — Show where time is spent (discovery, fixture setup, test execution, teardown)
- [ ] **`--durations-min` threshold** — Only show tests slower than N seconds in durations report
- [ ] **Tab completion** — Shell completion scripts for bash/zsh/fish via clap_complete
- [ ] **Error diagnostic improvements** — Rich error messages with code context and suggestions

---

## CI/CD Improvements (Cross-Cutting)

These should be done alongside the phases:

- [ ] **Multi-Python CI matrix** — Test against 3.9, 3.10, 3.11, 3.12, 3.13
- [ ] **Code coverage in CI** — cargo-tarpaulin or cargo-llvm-cov with Codecov badge
- [ ] **Integration test suite in CI** — Run `fastest tests/checks/` as part of CI
- [ ] **Add musl Linux wheel** — `x86_64-unknown-linux-musl` target for Alpine/Docker
- [ ] **TestPyPI smoke test** — Install wheel in fresh venv and run `fastest --version` before publishing
- [ ] **Speed up `cargo-audit`** — Use `taiki-e/install-action@cargo-audit` (pre-built binary, ~5s vs ~3min)
- [ ] **Add dev/test profile optimization** — `opt-level = 1` for dev builds
- [ ] **Split `requirements-dev.txt`** — Core (pytest, maturin) vs benchmarks (numpy, matplotlib) vs docs (mkdocs)
- [ ] **Pre-commit hook cleanup** — Remove `cargo test` from pre-commit (too slow), update black/ruff versions
- [ ] **Organize test directories** — Separate `testing_files/` (test subjects) from `tests/` (actual test suite)
- [ ] **Add `testpaths = ["tests"]`** to pyproject.toml to prevent accidental discovery of testing_files/

---

## Priority Matrix

| Phase | Impact | Effort | Risk | When |
|-------|--------|--------|------|------|
| **Phase 1** (Foundation) | Critical | 3-5 days | Low | **Now** |
| **Phase 2** (Isolation) | Critical | 5-8 days | Medium | After Phase 1 |
| **Phase 3** (UX) | High | 3-5 days | Low | Parallel with Phase 2 |
| **Phase 4** (Correctness) | High | 8-12 days | High | After Phase 2 |
| **Phase 5** (Refactoring) | Medium | 5-8 days | Medium | After Phase 4 |
| **Phase 6** (Features) | Medium | 10-15 days | Medium | After Phase 5 |

**Total estimated effort: 34-53 days**

---

## Metrics to Track

| Metric | Current | Target |
|--------|---------|--------|
| Rust unit tests | 144 | 300+ |
| Integration tests (execution) | 0 | 50+ |
| Code coverage | Unknown | 70%+ |
| Real pytest compatibility | ~80-85% | 95%+ |
| Broken Makefile targets | 3 | 0 |
| Stale doc files | 5 | 0 |
| Clippy warnings | 0 | 0 (maintain) |

---

*Generated from full codebase audit on 2026-03-08 against v2.3.0 (commit 9b47f64)*
