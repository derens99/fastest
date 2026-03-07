# Fastest v2 — Full Rewrite Design

**Date:** 2026-03-01
**Status:** Approved
**Goal:** Rewrite Fastest from scratch with clean architecture, removing all dead code, stubs, and unnecessary complexity while keeping proven patterns.

---

## Context

Fastest v1 accumulated significant technical debt:
- 392 `#[allow(dead_code)]` instances
- 3 experimental modules that don't work (JIT transpiler, zero-copy, work-stealing)
- ~10 unused heavy dependencies (Cranelift, bumpalo, string-interner, etc.)
- Stub implementations masquerading as features (SmartCoverage, Phase3Manager, smart test selection)
- 6 crates where 3 suffice
- Commented-out plugin system (6 of 9 modules disabled)
- Duplicate type definitions across crates

The core approach (Rust-based discovery + parallel execution) is sound and proven. The rewrite preserves what works and deletes everything else.

---

## Architecture

### 3 Crates

```
fastest/
├── crates/
│   ├── fastest-cli/          # Binary, CLI args, output formatting
│   ├── fastest-core/         # Discovery, parsing, config, fixtures, markers,
│   │                         # parametrize, plugins, incremental, watch
│   └── fastest-execution/    # Hybrid executor (PyO3 + subprocess), parallelism
├── tests/                    # Integration tests
└── Cargo.toml                # Workspace
```

### fastest-core

```
src/
├── lib.rs
├── config.rs              # Config loading (pyproject.toml, pytest.ini, setup.cfg, tox.ini)
├── error.rs               # Unified error types
├── model.rs               # TestItem, TestResult, TestOutcome (SINGLE source of truth)
├── discovery/
│   ├── mod.rs             # Parallel test discovery (rayon + aho-corasick)
│   ├── parser.rs          # AST parsing (rustpython-parser only)
│   └── cache.rs           # Discovery cache (xxhash + file mod times)
├── fixtures/
│   ├── mod.rs             # Fixture manager + dependency resolution
│   ├── builtin.rs         # tmp_path, capsys, monkeypatch
│   ├── conftest.rs        # conftest.py discovery + loading
│   └── scope.rs           # Scope-aware caching and lifecycle
├── markers.rs             # skip, xfail, skipif, custom markers
├── parametrize.rs         # @pytest.mark.parametrize expansion
├── plugins/
│   ├── mod.rs             # Plugin trait, registry, manager
│   ├── hooks.rs           # Hook system (pytest-compatible lifecycle hooks)
│   ├── builtin.rs         # Built-in plugins (fixture, marker, reporting, capture)
│   └── loader.rs          # Dynamic plugin loading
├── incremental/
│   ├── mod.rs             # Git-based change detection (git2 + blake3)
│   ├── cache.rs           # Test result caching (LRU)
│   └── impact.rs          # Impact analysis (file changes -> affected tests)
└── watch.rs               # File watching (notify) + debounced re-execution
```

### fastest-execution

```
src/
├── lib.rs
├── executor.rs            # HybridExecutor (the ONE executor)
├── inprocess.rs           # PyO3 in-process execution (<=20 tests)
├── subprocess.rs          # Subprocess pool execution (>20 tests)
├── parallel.rs            # Parallelism manager (rayon + crossbeam)
├── capture.rs             # stdout/stderr capture
├── fixtures.rs            # Fixture setup/teardown during execution
└── timeout.rs             # Test timeout handling
```

### fastest-cli

```
src/
├── main.rs                # Entry point, clap CLI, orchestration
├── output.rs              # Output formatting (pretty, JSON, JUnit XML)
└── progress.rs            # Progress bars (indicatif)
```

---

## Execution Strategy: Hybrid

```
HybridExecutor
  │
  ├── test_count <= 20  ──→  InProcess (PyO3)
  │                          Sequential or small thread pool
  │                          Fast startup, no subprocess overhead
  │
  └── test_count > 20   ──→  SubprocessPool (N workers)
                             N = CPU cores
                             JSON protocol over stdin/stdout
                             Work-stealing via crossbeam-deque
```

### InProcess (PyO3)
- Embeds Python interpreter via PyO3
- Executes tests by importing modules and calling test functions
- Best for small suites: avoids subprocess spawn overhead
- Limited by GIL for true parallelism, but fine for <=20 tests

### SubprocessPool
- Spawns N persistent Python worker processes
- Each worker runs a small Python harness that:
  1. Receives test items as JSON via stdin
  2. Executes the test (with fixture setup/teardown)
  3. Returns TestResult as JSON via stdout
- Work distribution via crossbeam-deque (work-stealing)
- Workers are reused across tests (no per-test spawn overhead)

---

## Data Flow

```
CLI args
  │
  ▼
Config::load()  <── pyproject.toml / pytest.ini / setup.cfg
  │
  ▼
PluginManager::new() → register built-in + load external
  │
  ▼
discovery::discover_tests(paths, config)
  ├── Walk directories (rayon parallel)
  ├── Parse Python AST (rustpython-parser)
  ├── Extract test items, fixtures, markers
  ├── Expand parametrize
  └── Cache results (xxhash)
  │
  ▼
plugins.call_hook("collection_modifyitems", &mut tests)
  │
  ▼
incremental::filter_unchanged(tests, git_status)  [if --incremental]
  │
  ▼
HybridExecutor::execute(tests)
  ├── <= 20 tests → InProcess (PyO3)
  └── > 20 tests → SubprocessPool (N workers)
     ├── Fixture setup per worker
     ├── Execute test + capture output
     ├── Fixture teardown
     └── Collect TestResult
  │
  ▼
plugins.call_hook("runtest_logreport", &results)
  │
  ▼
Output formatting → Pretty / JSON / JUnit XML
```

---

## Plugin System

### Plugin Trait
```rust
pub trait Plugin: Debug + Send + Sync {
    fn metadata(&self) -> &PluginMetadata;
    fn initialize(&mut self) -> Result<()>;
    fn shutdown(&mut self) -> Result<()>;
    fn on_hook(&mut self, hook: &str, args: &HookArgs) -> Result<Option<HookResult>>;
}
```

### Hooks (pytest-compatible)
- `pytest_collection_start`
- `pytest_collection_modifyitems`
- `pytest_collection_finish`
- `pytest_runtest_setup`
- `pytest_runtest_call`
- `pytest_runtest_teardown`
- `pytest_runtest_logreport`

### Built-in Plugins
1. FixturePlugin (priority 100): Fixture management
2. MarkerPlugin (priority 90): Test marker support
3. ReportingPlugin (priority 80): Test result reporting
4. CapturePlugin (priority 70): Output capture

### Loading
- Built-in plugins registered automatically
- External plugins loaded from `--plugin-dir` or conftest.py
- No proc macros needed — trait objects + simple registry

---

## Model Types (Single Source of Truth)

All in `fastest-core/src/model.rs`:

```rust
pub struct TestItem {
    pub id: String,               // "path::class::func[params]"
    pub path: PathBuf,
    pub function_name: String,
    pub line_number: Option<usize>,
    pub decorators: Vec<String>,
    pub is_async: bool,
    pub fixture_deps: Vec<String>,
    pub class_name: Option<String>,
    pub markers: Vec<Marker>,
    pub parameters: Option<Parameters>,
}

pub struct TestResult {
    pub test_id: String,
    pub outcome: TestOutcome,
    pub duration: Duration,
    pub output: String,
    pub error: Option<String>,
    pub stdout: String,
    pub stderr: String,
}

pub enum TestOutcome {
    Passed,
    Failed,
    Skipped { reason: Option<String> },
    XFailed { reason: Option<String> },
    XPassed,
    Error { message: String },
}
```

---

## Dependencies

### fastest-core
```toml
rustpython-parser = "0.3"
aho-corasick = "1.1"
rayon = "1.10"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
xxhash-rust = { version = "0.8", features = ["xxh3"] }
topological-sort = "0.2"
parking_lot = "0.12"
git2 = { version = "0.18", features = ["vendored-openssl"] }
blake3 = "1.5"
lru = "0.12"
notify = "6.0"
walkdir = "2.5"
regex = "1.11"
thiserror = "1.0"
anyhow = "1.0"
```

### fastest-execution
```toml
pyo3 = "=0.25.0"
crossbeam = "0.8"
crossbeam-deque = "0.8"
rayon = "1.10"
num_cpus = "1.16"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

### fastest-cli
```toml
clap = { version = "4.5", features = ["derive"] }
tokio = { version = "1.0", features = ["full"] }
colored = "2.1"
indicatif = "0.17"
anyhow = "1.0"
chrono = "0.4"
```

### Workspace-level
```toml
[features]
default = ["mimalloc"]
mimalloc = ["dep:mimalloc"]
```

### Deleted Dependencies (~15-20 crates removed)
- cranelift, cranelift-jit, cranelift-module, cranelift-codegen, target-lexicon
- bumpalo, string-interner, dashmap, rmp-serde, simd-json
- tree-sitter, tree-sitter-python (from core)
- petgraph, priority-queue (from advanced)
- tar, zip, sha2, ureq, semver, flate2 (self-update features)
- uuid (use path-based IDs instead)
- memmap2, smallvec

---

## Files/Directories to Delete

### Junk files
- `setup_test.sh` — one-off test setup script
- `test_project/` — ad-hoc test examples (replace with proper integration tests)

### Entire crates to delete
- `crates/fastest-advanced/` — incremental/watch move to core, rest deleted
- `crates/fastest-plugins/` — moves into fastest-core/plugins/
- `crates/fastest-plugins-macros/` — no longer needed

### Files within remaining crates to delete
- `crates/fastest-execution/src/experimental/` — entire directory (native_transpiler, zero_copy, work_stealing)
- `crates/fastest-execution/src/utils/simd_json.rs` — unnecessary
- Any file with primarily `#[allow(dead_code)]` content

### Git cleanup
- `crates/fastest-advanced/src/mod.rs` — already deleted but needs git commit

---

## What's Preserved (Proven Patterns)

1. AST-based test discovery with rustpython-parser
2. Parallel discovery with rayon
3. Aho-corasick SIMD pattern matching for fast file scanning
4. xxhash-based discovery caching
5. Topological sort for fixture dependencies
6. Complete fixture system (all scopes, autouse, yield, parametrize)
7. Full marker support (skip, xfail, skipif, custom)
8. Parametrize expansion with cross-product
9. Git-based incremental testing with blake3
10. LRU caching for test results
11. File watching with notify
12. PyO3 for Python integration
13. Crossbeam work-stealing for parallelism
14. Config priority chain (CLI > pyproject.toml > pytest.ini > ...)
15. mimalloc optional allocator
