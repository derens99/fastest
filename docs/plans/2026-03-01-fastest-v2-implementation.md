# Fastest v2 — Full Rewrite Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Rewrite the Fastest Python test runner from scratch with 3 clean crates, removing all dead code and stubs, keeping only proven patterns.

**Architecture:** 3-crate workspace (fastest-core, fastest-execution, fastest-cli). Hybrid execution: PyO3 in-process for <=20 tests, subprocess pool for >20. Plugin system lives in core. Incremental testing and watch mode included.

**Tech Stack:** Rust 2021 edition, PyO3 0.25, rustpython-parser 0.3, rayon, crossbeam, clap 4.5, tokio, git2, blake3, notify 6.0

**Design Doc:** `docs/plans/2026-03-01-fastest-v2-rewrite-design.md`

---

## Phase 0: Cleanup and Scaffold

### Task 1: Delete junk files and old crates

**Files:**
- Delete: `setup_test.sh`
- Delete: `test_project/` (entire directory)
- Delete: `crates/fastest-advanced/` (entire directory)
- Delete: `crates/fastest-plugins/` (entire directory)
- Delete: `crates/fastest-plugins-macros/` (entire directory)
- Delete: `crates/fastest-execution/src/experimental/` (entire directory)
- Delete: `crates/fastest-execution/src/utils/` (entire directory — contains only simd_json.rs)
- Delete: `crates/fastest-core/src/debug/` (entire directory)
- Delete: `crates/fastest-core/src/utils/` (entire directory — contains python.rs and simd_json.rs)
- Delete: `crates/fastest-core/src/plugin/` (old plugin module — we'll recreate as plugins/)
- Delete: `crates/fastest-core/src/test/parser/tree_sitter.rs`
- Delete: `crates/fastest-core/src/test/fixtures/session.rs`
- Delete: `crates/fastest-execution/src/core/lifecycle.rs`
- Delete: `crates/fastest-execution/src/core/runtime.rs`
- Delete: `crates/fastest-execution/src/core/strategies.rs`
- Delete: `crates/fastest-execution/src/core/fixture_integration.rs`
- Delete: `crates/fastest-execution/src/infrastructure/` (entire directory)
- Delete: `crates/fastest-cli/src/junit.rs` (will be rewritten as part of output.rs)

**Step 1: Delete junk files**

```bash
rm -f setup_test.sh
rm -rf test_project/
```

**Step 2: Delete old crates**

```bash
rm -rf crates/fastest-advanced/
rm -rf crates/fastest-plugins/
rm -rf crates/fastest-plugins-macros/
```

**Step 3: Gut fastest-execution (keep only crate skeleton)**

```bash
rm -rf crates/fastest-execution/src/experimental/
rm -rf crates/fastest-execution/src/utils/
rm -rf crates/fastest-execution/src/infrastructure/
rm -f crates/fastest-execution/src/core/lifecycle.rs
rm -f crates/fastest-execution/src/core/runtime.rs
rm -f crates/fastest-execution/src/core/strategies.rs
rm -f crates/fastest-execution/src/core/fixture_integration.rs
```

**Step 4: Gut fastest-core (keep only crate skeleton)**

```bash
rm -rf crates/fastest-core/src/debug/
rm -rf crates/fastest-core/src/utils/
rm -rf crates/fastest-core/src/plugin/
rm -f crates/fastest-core/src/test/parser/tree_sitter.rs
rm -f crates/fastest-core/src/test/fixtures/session.rs
rm -f crates/fastest-cli/src/junit.rs
```

**Step 5: Verify deletions**

```bash
find crates/ -name "*.rs" | sort
```

Expected: Only skeleton files remain in fastest-core, fastest-execution, fastest-cli.

---

### Task 2: Rewrite workspace Cargo.toml

**Files:**
- Modify: `Cargo.toml` (workspace root)

**Step 1: Rewrite workspace Cargo.toml**

Replace entire contents of `Cargo.toml`:

```toml
[workspace]
members = [
    "crates/fastest-core",
    "crates/fastest-execution",
    "crates/fastest-cli",
]
resolver = "2"

[workspace.package]
version = "2.0.0"
authors = ["Fastest Team <hello@fastest.dev>"]
edition = "2021"
license = "MIT"
repository = "https://github.com/derens99/fastest"

[workspace.dependencies]
anyhow = "1.0"
thiserror = "1.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
rayon = "1.10"
num_cpus = "1.16"
parking_lot = "0.12"
pyo3 = "=0.25.0"
```

**Step 2: Verify workspace parses**

```bash
cargo metadata --no-deps 2>&1 | head -5
```

Note: This will fail until we fix the crate Cargo.toml files, that's expected.

---

### Task 3: Rewrite fastest-core Cargo.toml

**Files:**
- Modify: `crates/fastest-core/Cargo.toml`

**Step 1: Rewrite fastest-core/Cargo.toml**

Replace entire contents:

```toml
[package]
name = "fastest-core"
version.workspace = true
authors.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
description = "Core types, discovery, and parsing for Fastest test runner"

[dependencies]
anyhow.workspace = true
thiserror.workspace = true
serde.workspace = true
serde_json.workspace = true
rayon.workspace = true
parking_lot.workspace = true

# Python AST parsing
rustpython-parser = "0.3"

# Fast pattern matching (SIMD-accelerated)
aho-corasick = "1.1"

# Directory walking
walkdir = "2.5"

# Pattern matching
regex = "1.11"

# Config file parsing
toml = "0.8"
glob = "0.3"
ignore = "0.4"

# Fast hashing for cache
xxhash-rust = { version = "0.8.7", features = ["xxh3"] }

# Fixture dependency ordering
topological-sort = "0.2"

# Incremental testing
git2 = { version = "0.18", features = ["vendored-openssl"] }
blake3 = "1.5"
lru = "0.12"

# File watching
notify = "6.0"

# Misc
dirs = "5.0"
once_cell = "1.17"

[dev-dependencies]
tempfile = "3.2"
```

---

### Task 4: Rewrite fastest-execution Cargo.toml

**Files:**
- Modify: `crates/fastest-execution/Cargo.toml`

**Step 1: Rewrite fastest-execution/Cargo.toml**

Replace entire contents:

```toml
[package]
name = "fastest-execution"
version.workspace = true
authors.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
description = "Hybrid test execution engine for Fastest"

[features]
default = ["mimalloc"]
mimalloc = ["dep:mimalloc"]

[dependencies]
fastest-core = { path = "../fastest-core" }
anyhow.workspace = true
thiserror.workspace = true
serde.workspace = true
serde_json.workspace = true
rayon.workspace = true
num_cpus.workspace = true
parking_lot.workspace = true

# Python integration
pyo3 = { workspace = true, features = ["auto-initialize"] }

# Parallelism
crossbeam = "0.8"
crossbeam-deque = "0.8"

# Optional allocator
mimalloc = { version = "0.1", optional = true }

# Process management
which = "7.0"
tempfile = "3.2"

[dev-dependencies]
tokio = { version = "1.0", features = ["full"] }
```

---

### Task 5: Rewrite fastest-cli Cargo.toml

**Files:**
- Modify: `crates/fastest-cli/Cargo.toml`

**Step 1: Rewrite fastest-cli/Cargo.toml**

Replace entire contents:

```toml
[package]
name = "fastest-cli"
version.workspace = true
authors.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
description = "CLI for Fastest Python test runner"

[[bin]]
name = "fastest"
path = "src/main.rs"

[dependencies]
fastest-core = { path = "../fastest-core" }
fastest-execution = { path = "../fastest-execution" }
anyhow.workspace = true
serde.workspace = true
serde_json.workspace = true

# CLI
clap = { version = "4.5", features = ["derive"] }

# Async runtime
tokio = { version = "1.0", features = ["full"] }

# Output
colored = "2.1"
indicatif = "0.17"
chrono = { version = "0.4", features = ["serde"] }
num_cpus.workspace = true

[dev-dependencies]
assert_cmd = "2.0"
predicates = "3.0"
tempfile = "3.2"
```

---

### Task 6: Create stub lib.rs and main.rs files so workspace compiles

**Files:**
- Create: `crates/fastest-core/src/lib.rs` (overwrite)
- Create: `crates/fastest-execution/src/lib.rs` (overwrite)
- Create: `crates/fastest-execution/src/core/mod.rs` (overwrite)
- Create: `crates/fastest-cli/src/main.rs` (overwrite)

**Step 1: Write fastest-core/src/lib.rs stub**

```rust
//! Core types, discovery, parsing, and configuration for Fastest test runner

pub mod config;
pub mod error;
pub mod model;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
```

**Step 2: Write fastest-core/src/error.rs minimal**

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Discovery error: {0}")]
    Discovery(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Execution error: {0}")]
    Execution(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
```

**Step 3: Write fastest-core/src/model.rs minimal**

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestItem {
    pub id: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Marker {
    pub name: String,
    pub args: Vec<serde_json::Value>,
    pub kwargs: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameters {
    pub names: Vec<String>,
    pub values: HashMap<String, serde_json::Value>,
    pub id_suffix: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TestOutcome {
    Passed,
    Failed,
    Skipped { reason: Option<String> },
    XFailed { reason: Option<String> },
    XPassed,
    Error { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub test_id: String,
    pub outcome: TestOutcome,
    pub duration: Duration,
    pub output: String,
    pub error: Option<String>,
    pub stdout: String,
    pub stderr: String,
}
```

**Step 4: Write fastest-core/src/config.rs minimal**

```rust
use crate::error::Result;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    pub testpaths: Vec<PathBuf>,
    pub python_files: Vec<String>,
    pub python_classes: Vec<String>,
    pub python_functions: Vec<String>,
    pub markers: Vec<String>,
    pub addopts: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            testpaths: vec![PathBuf::from(".")],
            python_files: vec!["test_*.py".into(), "*_test.py".into()],
            python_classes: vec!["Test*".into()],
            python_functions: vec!["test_*".into()],
            markers: Vec::new(),
            addopts: String::new(),
        }
    }
}

impl Config {
    pub fn load(path: Option<&PathBuf>) -> Result<Self> {
        // TODO: Implement config loading from pyproject.toml, pytest.ini, etc.
        let _ = path;
        Ok(Self::default())
    }
}
```

**Step 5: Remove old fastest-core test/ module files temporarily**

```bash
rm -rf crates/fastest-core/src/test/
```

**Step 6: Write fastest-execution stubs**

Write `crates/fastest-execution/src/lib.rs`:

```rust
//! Hybrid test execution engine for Fastest

pub mod core;

pub use fastest_core::model::{TestItem, TestOutcome, TestResult};
```

Write `crates/fastest-execution/src/core/mod.rs`:

```rust
//! Core execution module
```

Remove old execution files:

```bash
rm -f crates/fastest-execution/src/core/execution.rs
```

**Step 7: Write fastest-cli/src/main.rs stub**

```rust
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "fastest", version, about = "Blazing-fast Python test runner")]
struct Cli {
    /// Test path(s) to discover
    #[arg(default_value = ".")]
    paths: Vec<String>,
}

fn main() -> anyhow::Result<()> {
    let _cli = Cli::parse();
    println!("fastest v{}", fastest_core::VERSION);
    Ok(())
}
```

**Step 8: Build the workspace**

```bash
cargo build 2>&1
```

Expected: Clean build with no errors and no warnings (other than unused variable warnings for stubs).

**Step 9: Commit scaffold**

```bash
git add -A
git commit -m "feat: scaffold fastest v2 — delete dead code, 3-crate workspace"
```

---

## Phase 1: Core Model and Error Types

### Task 7: Flesh out model.rs with full types

**Files:**
- Modify: `crates/fastest-core/src/model.rs`

**Step 1: Write tests for model types**

Create `crates/fastest-core/src/model.rs` test module at the bottom of the file:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_item_id_format() {
        let item = TestItem {
            id: "tests/test_math.py::TestCalc::test_add".into(),
            path: PathBuf::from("tests/test_math.py"),
            function_name: "test_add".into(),
            line_number: Some(10),
            decorators: vec![],
            is_async: false,
            fixture_deps: vec!["tmp_path".into()],
            class_name: Some("TestCalc".into()),
            markers: vec![],
            parameters: None,
        };
        assert!(item.id.contains("::"));
        assert_eq!(item.function_name, "test_add");
    }

    #[test]
    fn test_result_serialization() {
        let result = TestResult {
            test_id: "test::id".into(),
            outcome: TestOutcome::Passed,
            duration: Duration::from_millis(42),
            output: String::new(),
            error: None,
            stdout: String::new(),
            stderr: String::new(),
        };
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: TestResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.outcome, TestOutcome::Passed);
    }

    #[test]
    fn test_outcome_variants() {
        assert_eq!(
            TestOutcome::Skipped { reason: Some("no db".into()) },
            TestOutcome::Skipped { reason: Some("no db".into()) }
        );
        assert_ne!(TestOutcome::Passed, TestOutcome::Failed);
    }
}
```

**Step 2: Run tests**

```bash
cargo test -p fastest-core
```

Expected: All 3 tests pass.

**Step 3: Commit**

```bash
git add crates/fastest-core/src/model.rs
git commit -m "feat(core): add model types with tests — TestItem, TestResult, TestOutcome"
```

---

## Phase 2: Config System

### Task 8: Implement config loading from pyproject.toml, pytest.ini, setup.cfg

**Files:**
- Modify: `crates/fastest-core/src/config.rs`

**Step 1: Write failing tests for config loading**

Add to config.rs:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.python_files, vec!["test_*.py", "*_test.py"]);
        assert_eq!(config.python_functions, vec!["test_*"]);
    }

    #[test]
    fn test_load_pyproject_toml() {
        let dir = tempfile::tempdir().unwrap();
        let pyproject = dir.path().join("pyproject.toml");
        fs::write(&pyproject, r#"
[tool.pytest.ini_options]
testpaths = ["tests", "integration"]
python_files = ["check_*.py"]
markers = ["slow: marks tests as slow"]
"#).unwrap();
        let config = Config::load_from_dir(dir.path()).unwrap();
        assert_eq!(config.testpaths.len(), 2);
        assert_eq!(config.python_files, vec!["check_*.py"]);
    }

    #[test]
    fn test_load_pytest_ini() {
        let dir = tempfile::tempdir().unwrap();
        let ini = dir.path().join("pytest.ini");
        fs::write(&ini, r#"
[pytest]
testpaths = tests
python_functions = check_*
"#).unwrap();
        let config = Config::load_from_dir(dir.path()).unwrap();
        assert_eq!(config.python_functions, vec!["check_*"]);
    }
}
```

**Step 2: Run tests to see them fail**

```bash
cargo test -p fastest-core config::tests
```

Expected: FAIL — `load_from_dir` method doesn't exist.

**Step 3: Implement config loading**

Implement `Config::load_from_dir(path: &Path)` that:
1. Looks for `pyproject.toml` → parses `[tool.pytest.ini_options]`
2. Falls back to `pytest.ini` → parses `[pytest]` section
3. Falls back to `setup.cfg` → parses `[tool:pytest]` section
4. Falls back to `tox.ini` → parses `[pytest]` section
5. Falls back to `Config::default()`

Reference the existing implementation in `crates/fastest-core/src/config.rs` for the parsing logic — port the working parts.

**Step 4: Run tests and verify pass**

```bash
cargo test -p fastest-core config::tests
```

Expected: All config tests pass.

**Step 5: Commit**

```bash
git add crates/fastest-core/src/config.rs
git commit -m "feat(core): implement config loading — pyproject.toml, pytest.ini, setup.cfg, tox.ini"
```

---

## Phase 3: Test Discovery

### Task 9: Create discovery module with AST parser

**Files:**
- Create: `crates/fastest-core/src/discovery/mod.rs`
- Create: `crates/fastest-core/src/discovery/parser.rs`
- Create: `crates/fastest-core/src/discovery/cache.rs`
- Modify: `crates/fastest-core/src/lib.rs`

**Step 1: Create the discovery directory**

```bash
mkdir -p crates/fastest-core/src/discovery
```

**Step 2: Write failing test for parser**

Create `crates/fastest-core/src/discovery/parser.rs`:

```rust
//! Python test file parser using rustpython-parser

use crate::error::Result;
use crate::model::TestItem;
use std::path::Path;

/// Parse a Python file and extract test items
pub fn parse_test_file(path: &Path, content: &str) -> Result<Vec<TestItem>> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_functions() {
        let content = r#"
def test_add():
    assert 1 + 1 == 2

def test_subtract():
    assert 2 - 1 == 1

def helper():
    pass
"#;
        let items = parse_test_file(Path::new("test_math.py"), content).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].function_name, "test_add");
        assert_eq!(items[1].function_name, "test_subtract");
    }

    #[test]
    fn test_parse_class_tests() {
        let content = r#"
class TestCalc:
    def test_add(self):
        assert 1 + 1 == 2

    def test_sub(self):
        assert 2 - 1 == 1

    def helper(self):
        pass
"#;
        let items = parse_test_file(Path::new("test_calc.py"), content).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].class_name, Some("TestCalc".into()));
        assert_eq!(items[0].function_name, "test_add");
    }

    #[test]
    fn test_parse_fixtures() {
        let content = r#"
def test_with_fixtures(tmp_path, capsys):
    pass
"#;
        let items = parse_test_file(Path::new("test_fx.py"), content).unwrap();
        assert_eq!(items[0].fixture_deps, vec!["tmp_path", "capsys"]);
    }

    #[test]
    fn test_parse_async() {
        let content = r#"
async def test_async_op():
    await something()
"#;
        let items = parse_test_file(Path::new("test_async.py"), content).unwrap();
        assert!(items[0].is_async);
    }

    #[test]
    fn test_parse_decorators() {
        let content = r#"
import pytest

@pytest.mark.skip(reason="broken")
def test_skipped():
    pass

@pytest.mark.xfail
def test_xfail():
    pass
"#;
        let items = parse_test_file(Path::new("test_dec.py"), content).unwrap();
        assert_eq!(items.len(), 2);
        assert!(!items[0].decorators.is_empty());
    }
}
```

**Step 3: Run tests to see them fail**

```bash
cargo test -p fastest-core discovery::parser::tests
```

Expected: FAIL — `todo!()` panics.

**Step 4: Implement parse_test_file**

Use `rustpython_parser::parse()` to parse the Python source into an AST, then walk the AST to extract:
- Top-level `def test_*` functions → `TestItem` with no class_name
- `class Test*` → nested `def test_*` methods → `TestItem` with class_name
- Function parameters (excluding `self`) → fixture_deps
- `async def` → is_async = true
- Decorator names → decorators vec

Reference the existing parser at `crates/fastest-core/src/test/parser/mod.rs` (before deletion) or the v1 discovery code for the AST walking logic.

**Step 5: Run tests and verify pass**

```bash
cargo test -p fastest-core discovery::parser::tests
```

Expected: All parser tests pass.

**Step 6: Commit**

```bash
git add crates/fastest-core/src/discovery/
git commit -m "feat(core): implement Python AST parser for test discovery"
```

---

### Task 10: Implement parallel test discovery

**Files:**
- Modify: `crates/fastest-core/src/discovery/mod.rs`
- Modify: `crates/fastest-core/src/lib.rs`

**Step 1: Write failing test for discovery**

Create `crates/fastest-core/src/discovery/mod.rs`:

```rust
//! Parallel test discovery

pub mod cache;
pub mod parser;

use crate::config::Config;
use crate::error::Result;
use crate::model::TestItem;
use rayon::prelude::*;
use std::path::{Path, PathBuf};

/// Discover all tests in the given paths
pub fn discover_tests(paths: &[PathBuf], config: &Config) -> Result<Vec<TestItem>> {
    let files = collect_test_files(paths, config)?;

    let items: Vec<TestItem> = files
        .par_iter()
        .filter_map(|path| {
            let content = std::fs::read_to_string(path).ok()?;
            parser::parse_test_file(path, &content).ok()
        })
        .flatten()
        .collect();

    Ok(items)
}

/// Collect all Python test files matching config patterns
fn collect_test_files(paths: &[PathBuf], config: &Config) -> Result<Vec<PathBuf>> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_collect_test_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("test_one.py"), "def test_a(): pass").unwrap();
        fs::write(dir.path().join("test_two.py"), "def test_b(): pass").unwrap();
        fs::write(dir.path().join("helper.py"), "def helper(): pass").unwrap();
        fs::create_dir(dir.path().join("subdir")).unwrap();
        fs::write(dir.path().join("subdir/test_three.py"), "def test_c(): pass").unwrap();

        let config = Config::default();
        let files = collect_test_files(&[dir.path().to_path_buf()], &config).unwrap();
        assert_eq!(files.len(), 3); // test_one.py, test_two.py, subdir/test_three.py
    }

    #[test]
    fn test_discover_tests_basic() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("test_math.py"), r#"
def test_add():
    assert 1 + 1 == 2

def test_sub():
    assert 2 - 1 == 1
"#).unwrap();

        let config = Config::default();
        let items = discover_tests(&[dir.path().to_path_buf()], &config).unwrap();
        assert_eq!(items.len(), 2);
    }
}
```

**Step 2: Run tests to see them fail**

```bash
cargo test -p fastest-core discovery::tests
```

Expected: FAIL — `collect_test_files` is `todo!()`.

**Step 3: Implement collect_test_files**

Use `walkdir` or `ignore` crate to recursively walk directories, filtering files that match `config.python_files` patterns (default: `test_*.py`, `*_test.py`). Use `aho_corasick` for fast pattern matching on filenames.

**Step 4: Run tests and verify pass**

```bash
cargo test -p fastest-core discovery
```

Expected: All discovery tests pass.

**Step 5: Update lib.rs to export discovery**

```rust
pub mod config;
pub mod discovery;
pub mod error;
pub mod model;

pub use config::Config;
pub use discovery::discover_tests;
pub use error::{Error, Result};
pub use model::{TestItem, TestOutcome, TestResult};
```

**Step 6: Verify workspace builds**

```bash
cargo build
```

**Step 7: Commit**

```bash
git add crates/fastest-core/
git commit -m "feat(core): implement parallel test discovery with file walking"
```

---

### Task 11: Implement discovery cache

**Files:**
- Modify: `crates/fastest-core/src/discovery/cache.rs`

**Step 1: Write failing test for cache**

```rust
//! Discovery cache using xxhash for fast change detection

use crate::error::Result;
use crate::model::TestItem;
use std::path::{Path, PathBuf};

pub struct DiscoveryCache {
    cache_path: PathBuf,
    // file_path -> (hash, Vec<TestItem>)
    entries: std::collections::HashMap<PathBuf, (u64, Vec<TestItem>)>,
}

impl DiscoveryCache {
    pub fn load(cache_dir: &Path) -> Result<Self> {
        todo!()
    }

    pub fn get(&self, path: &Path, content_hash: u64) -> Option<&[TestItem]> {
        todo!()
    }

    pub fn insert(&mut self, path: PathBuf, content_hash: u64, items: Vec<TestItem>) {
        todo!()
    }

    pub fn save(&self) -> Result<()> {
        todo!()
    }
}

/// Hash file content using xxhash
pub fn hash_content(content: &[u8]) -> u64 {
    xxhash_rust::xxh3::xxh3_64(content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_deterministic() {
        let h1 = hash_content(b"hello world");
        let h2 = hash_content(b"hello world");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_cache_hit_miss() {
        let dir = tempfile::tempdir().unwrap();
        let mut cache = DiscoveryCache::load(dir.path()).unwrap();

        let path = PathBuf::from("test_example.py");
        assert!(cache.get(&path, 12345).is_none());

        cache.insert(path.clone(), 12345, vec![]);
        assert!(cache.get(&path, 12345).is_some());
        assert!(cache.get(&path, 99999).is_none()); // different hash = miss
    }

    #[test]
    fn test_cache_persistence() {
        let dir = tempfile::tempdir().unwrap();
        {
            let mut cache = DiscoveryCache::load(dir.path()).unwrap();
            cache.insert(PathBuf::from("test.py"), 42, vec![]);
            cache.save().unwrap();
        }
        {
            let cache = DiscoveryCache::load(dir.path()).unwrap();
            assert!(cache.get(Path::new("test.py"), 42).is_some());
        }
    }
}
```

**Step 2: Run tests to see them fail**

```bash
cargo test -p fastest-core discovery::cache::tests
```

**Step 3: Implement the cache**

Implement DiscoveryCache using:
- `xxhash_rust::xxh3::xxh3_64` for content hashing
- `serde_json` for persistence to `{cache_dir}/discovery_cache.json`
- HashMap for in-memory lookup

**Step 4: Run tests and verify pass**

```bash
cargo test -p fastest-core discovery::cache
```

**Step 5: Integrate cache into discovery flow**

Modify `discover_tests()` in `discovery/mod.rs` to:
1. Load cache
2. For each file: hash content, check cache, skip parsing if hit
3. Save cache after discovery

**Step 6: Commit**

```bash
git add crates/fastest-core/src/discovery/
git commit -m "feat(core): add xxhash-based discovery cache"
```

---

## Phase 4: Markers

### Task 12: Implement marker system

**Files:**
- Create: `crates/fastest-core/src/markers.rs`
- Modify: `crates/fastest-core/src/lib.rs`

**Step 1: Write failing tests for markers**

```rust
//! Test marker support — skip, xfail, skipif, custom markers

use crate::error::Result;
use crate::model::{Marker, TestItem};

#[derive(Debug, Clone)]
pub enum BuiltinMarker {
    Skip { reason: Option<String> },
    Skipif { condition: String, reason: Option<String> },
    Xfail { reason: Option<String> },
    Parametrize,
    Timeout(f64),
    Custom(String),
}

/// Extract builtin marker from a Marker
pub fn classify_marker(marker: &Marker) -> BuiltinMarker {
    todo!()
}

/// Filter tests by marker expression (e.g. "-m slow")
pub fn filter_by_markers(tests: &[TestItem], expr: &str) -> Vec<TestItem> {
    todo!()
}

/// Filter tests by keyword expression (e.g. "-k test_add")
pub fn filter_by_keyword(tests: &[TestItem], expr: &str) -> Vec<TestItem> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_marker(name: &str) -> Marker {
        Marker { name: name.into(), args: vec![], kwargs: HashMap::new() }
    }

    #[test]
    fn test_classify_skip() {
        let m = make_marker("skip");
        assert!(matches!(classify_marker(&m), BuiltinMarker::Skip { .. }));
    }

    #[test]
    fn test_classify_xfail() {
        let m = make_marker("xfail");
        assert!(matches!(classify_marker(&m), BuiltinMarker::Xfail { .. }));
    }

    #[test]
    fn test_classify_custom() {
        let m = make_marker("slow");
        assert!(matches!(classify_marker(&m), BuiltinMarker::Custom(ref s) if s == "slow"));
    }

    #[test]
    fn test_filter_by_marker_expression() {
        let tests = vec![
            make_test_item("test_a", vec!["slow"]),
            make_test_item("test_b", vec!["fast"]),
            make_test_item("test_c", vec!["slow", "integration"]),
        ];
        let filtered = filter_by_markers(&tests, "slow");
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_filter_by_keyword() {
        let tests = vec![
            make_test_item("test_add", vec![]),
            make_test_item("test_subtract", vec![]),
            make_test_item("test_multiply", vec![]),
        ];
        let filtered = filter_by_keyword(&tests, "add");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].function_name, "test_add");
    }

    fn make_test_item(name: &str, marker_names: Vec<&str>) -> TestItem {
        TestItem {
            id: format!("test.py::{}", name),
            path: "test.py".into(),
            function_name: name.into(),
            line_number: None,
            decorators: vec![],
            is_async: false,
            fixture_deps: vec![],
            class_name: None,
            markers: marker_names.into_iter().map(|n| make_marker(n)).collect(),
            parameters: None,
        }
    }
}
```

**Step 2: Run tests to see them fail, implement, run again**

```bash
cargo test -p fastest-core markers::tests
```

**Step 3: Implement classify_marker, filter_by_markers, filter_by_keyword**

Port working logic from `crates/fastest-core/src/test/markers/mod.rs` (v1).

**Step 4: Update lib.rs exports**

**Step 5: Commit**

```bash
git add crates/fastest-core/src/markers.rs crates/fastest-core/src/lib.rs
git commit -m "feat(core): implement marker system — skip, xfail, skipif, keyword filtering"
```

---

## Phase 5: Parametrize

### Task 13: Implement parametrize expansion

**Files:**
- Create: `crates/fastest-core/src/parametrize.rs`
- Modify: `crates/fastest-core/src/lib.rs`

**Step 1: Write failing tests**

```rust
//! @pytest.mark.parametrize expansion

use crate::error::Result;
use crate::model::TestItem;

/// Expand parametrized tests into individual test items
pub fn expand_parametrized_tests(tests: Vec<TestItem>) -> Result<Vec<TestItem>> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_expand_simple_parametrize() {
        let item = TestItem {
            id: "test.py::test_add".into(),
            path: PathBuf::from("test.py"),
            function_name: "test_add".into(),
            line_number: Some(5),
            decorators: vec![
                r#"pytest.mark.parametrize("x,y,expected", [(1,2,3), (4,5,9)])"#.into()
            ],
            is_async: false,
            fixture_deps: vec![],
            class_name: None,
            markers: vec![],
            parameters: None,
        };

        let expanded = expand_parametrized_tests(vec![item]).unwrap();
        assert_eq!(expanded.len(), 2);
        assert!(expanded[0].id.contains("["));
        assert!(expanded[1].id.contains("["));
    }

    #[test]
    fn test_non_parametrized_passthrough() {
        let item = TestItem {
            id: "test.py::test_plain".into(),
            path: PathBuf::from("test.py"),
            function_name: "test_plain".into(),
            line_number: Some(1),
            decorators: vec![],
            is_async: false,
            fixture_deps: vec![],
            class_name: None,
            markers: vec![],
            parameters: None,
        };

        let expanded = expand_parametrized_tests(vec![item]).unwrap();
        assert_eq!(expanded.len(), 1);
    }
}
```

**Step 2: Run tests, implement, run again**

Port working logic from `crates/fastest-core/src/test/parametrize.rs` (v1). Use `rustpython_parser` to parse the decorator arguments.

**Step 3: Commit**

```bash
git add crates/fastest-core/src/parametrize.rs crates/fastest-core/src/lib.rs
git commit -m "feat(core): implement parametrize expansion"
```

---

## Phase 6: Fixtures

### Task 14: Implement fixture system

**Files:**
- Create: `crates/fastest-core/src/fixtures/mod.rs`
- Create: `crates/fastest-core/src/fixtures/builtin.rs`
- Create: `crates/fastest-core/src/fixtures/conftest.rs`
- Create: `crates/fastest-core/src/fixtures/scope.rs`
- Modify: `crates/fastest-core/src/lib.rs`

**Step 1: Create fixtures directory**

```bash
mkdir -p crates/fastest-core/src/fixtures
```

**Step 2: Write failing tests for fixture resolution**

Create `crates/fastest-core/src/fixtures/mod.rs`:

```rust
//! Fixture management with dependency resolution

pub mod builtin;
pub mod conftest;
pub mod scope;

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FixtureScope {
    Function,
    Class,
    Module,
    Session,
    Package,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fixture {
    pub name: String,
    pub scope: FixtureScope,
    pub autouse: bool,
    pub params: Vec<serde_json::Value>,
    pub func_path: PathBuf,
    pub dependencies: Vec<String>,
    pub is_yield: bool,
}

/// Resolve fixture dependencies using topological sort
pub fn resolve_fixture_order(
    required: &[String],
    available: &HashMap<String, Fixture>,
) -> Result<Vec<String>> {
    todo!()
}

/// Check if a fixture name is a built-in fixture
pub fn is_builtin(name: &str) -> bool {
    builtin::BUILTIN_FIXTURES.contains(&name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_simple_order() {
        let mut fixtures = HashMap::new();
        fixtures.insert("db".into(), Fixture {
            name: "db".into(),
            scope: FixtureScope::Function,
            autouse: false,
            params: vec![],
            func_path: "conftest.py".into(),
            dependencies: vec![],
            is_yield: false,
        });
        fixtures.insert("user".into(), Fixture {
            name: "user".into(),
            scope: FixtureScope::Function,
            autouse: false,
            params: vec![],
            func_path: "conftest.py".into(),
            dependencies: vec!["db".into()],
            is_yield: false,
        });

        let order = resolve_fixture_order(&["user".into()], &fixtures).unwrap();
        let db_pos = order.iter().position(|f| f == "db").unwrap();
        let user_pos = order.iter().position(|f| f == "user").unwrap();
        assert!(db_pos < user_pos, "db should come before user");
    }

    #[test]
    fn test_builtin_recognition() {
        assert!(is_builtin("tmp_path"));
        assert!(is_builtin("capsys"));
        assert!(is_builtin("monkeypatch"));
        assert!(!is_builtin("custom_fixture"));
    }
}
```

**Step 3: Implement fixture resolution with topological sort**

Use `topological_sort` crate to resolve fixture dependency order. Port the working fixture logic from `crates/fastest-core/src/test/fixtures/mod.rs` and `advanced.rs` (v1).

**Step 4: Implement builtin.rs**

```rust
//! Built-in pytest fixtures

pub const BUILTIN_FIXTURES: &[&str] = &[
    "tmp_path",
    "tmp_path_factory",
    "capsys",
    "capfd",
    "monkeypatch",
    "request",
    "pytestconfig",
    "cache",
];

/// Generate Python code for a built-in fixture
pub fn generate_builtin_code(name: &str) -> Option<String> {
    match name {
        "tmp_path" => Some("import tempfile; tmp_path = pathlib.Path(tempfile.mkdtemp())".into()),
        "capsys" => Some("import io, sys; _capsys_out = io.StringIO(); _capsys_err = io.StringIO()".into()),
        "monkeypatch" => Some("from _pytest.monkeypatch import MonkeyPatch; monkeypatch = MonkeyPatch()".into()),
        _ => None,
    }
}
```

**Step 5: Implement conftest.rs**

```rust
//! conftest.py discovery and fixture extraction

use crate::error::Result;
use super::Fixture;
use std::collections::HashMap;
use std::path::Path;

/// Discover conftest.py files and extract fixture definitions
pub fn discover_conftest_fixtures(root: &Path) -> Result<HashMap<String, Fixture>> {
    todo!()
}
```

**Step 6: Implement scope.rs**

```rust
//! Scope-aware fixture caching

use super::{Fixture, FixtureScope};
use std::collections::HashMap;

pub struct FixtureCache {
    function_cache: HashMap<String, serde_json::Value>,
    class_cache: HashMap<String, serde_json::Value>,
    module_cache: HashMap<String, serde_json::Value>,
    session_cache: HashMap<String, serde_json::Value>,
}

impl FixtureCache {
    pub fn new() -> Self {
        Self {
            function_cache: HashMap::new(),
            class_cache: HashMap::new(),
            module_cache: HashMap::new(),
            session_cache: HashMap::new(),
        }
    }

    pub fn get(&self, name: &str, scope: &FixtureScope) -> Option<&serde_json::Value> {
        match scope {
            FixtureScope::Function => self.function_cache.get(name),
            FixtureScope::Class => self.class_cache.get(name),
            FixtureScope::Module => self.module_cache.get(name),
            FixtureScope::Session | FixtureScope::Package => self.session_cache.get(name),
        }
    }

    pub fn clear_scope(&mut self, scope: &FixtureScope) {
        match scope {
            FixtureScope::Function => self.function_cache.clear(),
            FixtureScope::Class => self.class_cache.clear(),
            FixtureScope::Module => self.module_cache.clear(),
            FixtureScope::Session | FixtureScope::Package => self.session_cache.clear(),
        }
    }
}
```

**Step 7: Run all tests, commit**

```bash
cargo test -p fastest-core fixtures
git add crates/fastest-core/src/fixtures/ crates/fastest-core/src/lib.rs
git commit -m "feat(core): implement fixture system — resolution, built-ins, conftest, scoping"
```

---

## Phase 7: Plugin System

### Task 15: Implement plugin system

**Files:**
- Create: `crates/fastest-core/src/plugins/mod.rs`
- Create: `crates/fastest-core/src/plugins/hooks.rs`
- Create: `crates/fastest-core/src/plugins/builtin.rs`
- Create: `crates/fastest-core/src/plugins/loader.rs`
- Modify: `crates/fastest-core/src/lib.rs`

**Step 1: Create plugins directory**

```bash
mkdir -p crates/fastest-core/src/plugins
```

**Step 2: Write Plugin trait and manager (plugins/mod.rs)**

```rust
//! Plugin system — trait, registry, manager

pub mod builtin;
pub mod hooks;
pub mod loader;

use crate::error::Result;
use std::any::Any;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub priority: i32,
}

pub trait Plugin: Debug + Send + Sync {
    fn metadata(&self) -> &PluginMetadata;
    fn initialize(&mut self) -> Result<()>;
    fn shutdown(&mut self) -> Result<()>;
    fn on_hook(&mut self, hook: &str, args: &HookArgs) -> Result<Option<HookResult>>;
    fn as_any(&self) -> &dyn Any;
}

#[derive(Debug, Default)]
pub struct HookArgs {
    pub data: HashMap<String, serde_json::Value>,
}

#[derive(Debug)]
pub struct HookResult {
    pub data: HashMap<String, serde_json::Value>,
}

pub struct PluginManager {
    plugins: Vec<Box<dyn Plugin>>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self { plugins: Vec::new() }
    }

    pub fn with_builtins() -> Result<Self> {
        let mut mgr = Self::new();
        mgr.register(Box::new(builtin::FixturePlugin::new()))?;
        mgr.register(Box::new(builtin::MarkerPlugin::new()))?;
        mgr.register(Box::new(builtin::ReportingPlugin::new()))?;
        mgr.register(Box::new(builtin::CapturePlugin::new()))?;
        Ok(mgr)
    }

    pub fn register(&mut self, plugin: Box<dyn Plugin>) -> Result<()> {
        self.plugins.push(plugin);
        // Sort by priority (highest first)
        self.plugins.sort_by(|a, b| b.metadata().priority.cmp(&a.metadata().priority));
        Ok(())
    }

    pub fn call_hook(&mut self, hook: &str, args: &HookArgs) -> Result<Vec<HookResult>> {
        let mut results = Vec::new();
        for plugin in &mut self.plugins {
            if let Some(result) = plugin.on_hook(hook, args)? {
                results.push(result);
            }
        }
        Ok(results)
    }

    pub fn initialize_all(&mut self) -> Result<()> {
        for plugin in &mut self.plugins {
            plugin.initialize()?;
        }
        Ok(())
    }

    pub fn shutdown_all(&mut self) -> Result<()> {
        for plugin in &mut self.plugins {
            plugin.shutdown()?;
        }
        Ok(())
    }
}
```

**Step 3: Write hooks.rs**

Define the standard hook names as constants:

```rust
//! Hook definitions — pytest-compatible lifecycle hooks

pub const COLLECTION_START: &str = "pytest_collection_start";
pub const COLLECTION_MODIFY_ITEMS: &str = "pytest_collection_modifyitems";
pub const COLLECTION_FINISH: &str = "pytest_collection_finish";
pub const RUNTEST_SETUP: &str = "pytest_runtest_setup";
pub const RUNTEST_CALL: &str = "pytest_runtest_call";
pub const RUNTEST_TEARDOWN: &str = "pytest_runtest_teardown";
pub const RUNTEST_LOGREPORT: &str = "pytest_runtest_logreport";
```

**Step 4: Write builtin.rs with 4 built-in plugins**

Port from `crates/fastest-plugins/src/builtin.rs` (v1). Each plugin implements the Plugin trait with on_hook matching relevant hook names.

**Step 5: Write loader.rs stub**

```rust
//! Dynamic plugin loading from --plugin-dir

use crate::error::Result;
use super::Plugin;
use std::path::Path;

pub fn load_plugins_from_dir(_dir: &Path) -> Result<Vec<Box<dyn Plugin>>> {
    // External plugin loading — returns empty for now
    Ok(Vec::new())
}
```

**Step 6: Write tests, run, commit**

```bash
cargo test -p fastest-core plugins
git add crates/fastest-core/src/plugins/ crates/fastest-core/src/lib.rs
git commit -m "feat(core): implement plugin system — trait, hooks, built-in plugins, loader"
```

---

## Phase 8: Execution Engine

### Task 16: Implement InProcess executor (PyO3)

**Files:**
- Create: `crates/fastest-execution/src/inprocess.rs`
- Modify: `crates/fastest-execution/src/lib.rs`

**Step 1: Write InProcess executor**

```rust
//! In-process test execution using PyO3

use fastest_core::model::{TestItem, TestOutcome, TestResult};
use pyo3::prelude::*;
use std::time::Instant;

pub struct InProcessExecutor;

impl InProcessExecutor {
    pub fn new() -> Self {
        Self
    }

    /// Execute tests in-process using PyO3
    /// Best for small suites (<=20 tests)
    pub fn execute(&self, tests: &[TestItem]) -> Vec<TestResult> {
        Python::with_gil(|py| {
            tests.iter().map(|test| self.run_single(py, test)).collect()
        })
    }

    fn run_single(&self, py: Python<'_>, test: &TestItem) -> TestResult {
        let start = Instant::now();

        // Build Python code to execute the test
        let test_code = self.build_test_code(test);

        match py.run(&test_code, None, None) {
            Ok(_) => TestResult {
                test_id: test.id.clone(),
                outcome: TestOutcome::Passed,
                duration: start.elapsed(),
                output: String::new(),
                error: None,
                stdout: String::new(),
                stderr: String::new(),
            },
            Err(e) => TestResult {
                test_id: test.id.clone(),
                outcome: TestOutcome::Failed,
                duration: start.elapsed(),
                output: String::new(),
                error: Some(e.to_string()),
                stdout: String::new(),
                stderr: String::new(),
            },
        }
    }

    fn build_test_code(&self, test: &TestItem) -> String {
        // Import the test module and call the test function
        let module_path = test.path.to_string_lossy().replace(".py", "").replace('/', ".").replace('\\', ".");
        match &test.class_name {
            Some(class) => format!(
                "import importlib; mod = importlib.import_module('{}'); getattr(mod.{}(), '{}')()",
                module_path, class, test.function_name
            ),
            None => format!(
                "import importlib; mod = importlib.import_module('{}'); mod.{}()",
                module_path, test.function_name
            ),
        }
    }
}
```

**Step 2: Write basic test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_build_test_code_function() {
        let exec = InProcessExecutor::new();
        let item = TestItem {
            id: "tests/test_math.py::test_add".into(),
            path: PathBuf::from("tests/test_math.py"),
            function_name: "test_add".into(),
            line_number: None,
            decorators: vec![],
            is_async: false,
            fixture_deps: vec![],
            class_name: None,
            markers: vec![],
            parameters: None,
        };
        let code = exec.build_test_code(&item);
        assert!(code.contains("test_add"));
    }
}
```

**Step 3: Run tests, commit**

```bash
cargo test -p fastest-execution inprocess
git add crates/fastest-execution/src/inprocess.rs
git commit -m "feat(execution): implement InProcess executor with PyO3"
```

---

### Task 17: Implement Subprocess pool executor

**Files:**
- Create: `crates/fastest-execution/src/subprocess.rs`

**Step 1: Write SubprocessPool executor**

This is the workhorse for >20 tests. It:
1. Discovers the Python interpreter path
2. Spawns N persistent worker processes (N = CPU cores)
3. Each worker runs a Python harness that reads JSON from stdin and writes JSON to stdout
4. Distributes tests across workers using crossbeam-deque work-stealing

```rust
//! Subprocess pool execution for parallel test running

use fastest_core::model::{TestItem, TestOutcome, TestResult};
use crossbeam_deque::{Injector, Stealer, Worker};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

const WORKER_HARNESS: &str = include_str!("worker_harness.py");

pub struct SubprocessPool {
    num_workers: usize,
    python_path: String,
}

impl SubprocessPool {
    pub fn new(num_workers: Option<usize>) -> Self {
        let n = num_workers.unwrap_or_else(num_cpus::get);
        Self {
            num_workers: n,
            python_path: find_python().unwrap_or_else(|| "python3".into()),
        }
    }

    pub fn execute(&self, tests: &[TestItem]) -> Vec<TestResult> {
        todo!() // Implement work-stealing distribution
    }
}

fn find_python() -> Option<String> {
    which::which("python3")
        .or_else(|_| which::which("python"))
        .ok()
        .map(|p| p.to_string_lossy().into_owned())
}
```

**Step 2: Create the Python worker harness**

Create `crates/fastest-execution/src/worker_harness.py`:

```python
"""Worker harness for fastest subprocess execution.
Reads test items as JSON from stdin, executes them, writes results as JSON to stdout.
"""
import json
import sys
import time
import traceback
import importlib
import io

def run_test(test_item):
    """Execute a single test and return the result."""
    start = time.time()
    stdout_capture = io.StringIO()
    stderr_capture = io.StringIO()
    old_stdout, old_stderr = sys.stdout, sys.stderr

    try:
        sys.stdout, sys.stderr = stdout_capture, stderr_capture

        # Import the module
        module_path = test_item["path"].replace(".py", "").replace("/", ".").replace("\\", ".")
        mod = importlib.import_module(module_path)

        # Get the test function
        if test_item.get("class_name"):
            cls = getattr(mod, test_item["class_name"])
            instance = cls()
            func = getattr(instance, test_item["function_name"])
        else:
            func = getattr(mod, test_item["function_name"])

        # Execute
        func()

        return {
            "test_id": test_item["id"],
            "outcome": "Passed",
            "duration_ms": int((time.time() - start) * 1000),
            "output": "",
            "error": None,
            "stdout": stdout_capture.getvalue(),
            "stderr": stderr_capture.getvalue(),
        }
    except Exception as e:
        return {
            "test_id": test_item["id"],
            "outcome": "Failed",
            "duration_ms": int((time.time() - start) * 1000),
            "output": "",
            "error": traceback.format_exc(),
            "stdout": stdout_capture.getvalue(),
            "stderr": stderr_capture.getvalue(),
        }
    finally:
        sys.stdout, sys.stderr = old_stdout, old_stderr

# Main loop: read JSON lines from stdin, execute, write results to stdout
for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    if line == "EXIT":
        break
    try:
        test_item = json.loads(line)
        result = run_test(test_item)
        print(json.dumps(result), flush=True)
    except Exception as e:
        print(json.dumps({"error": str(e)}), flush=True)
```

**Step 3: Implement the work-stealing distribution in subprocess.rs**

Use `crossbeam_deque::Injector` to distribute tests to worker threads, each managing a subprocess.

**Step 4: Write tests, run, commit**

```bash
cargo test -p fastest-execution subprocess
git add crates/fastest-execution/src/subprocess.rs crates/fastest-execution/src/worker_harness.py
git commit -m "feat(execution): implement subprocess pool with work-stealing"
```

---

### Task 18: Implement HybridExecutor

**Files:**
- Create: `crates/fastest-execution/src/executor.rs`
- Modify: `crates/fastest-execution/src/lib.rs`
- Modify: `crates/fastest-execution/src/core/mod.rs`

**Step 1: Write HybridExecutor**

```rust
//! HybridExecutor — adaptive strategy selection

use fastest_core::model::{TestItem, TestResult};
use crate::inprocess::InProcessExecutor;
use crate::subprocess::SubprocessPool;

const INPROCESS_THRESHOLD: usize = 20;

pub struct HybridExecutor {
    inprocess: InProcessExecutor,
    subprocess: SubprocessPool,
}

impl HybridExecutor {
    pub fn new() -> Self {
        Self {
            inprocess: InProcessExecutor::new(),
            subprocess: SubprocessPool::new(None),
        }
    }

    pub fn execute(&self, tests: &[TestItem]) -> Vec<TestResult> {
        if tests.len() <= INPROCESS_THRESHOLD {
            self.inprocess.execute(tests)
        } else {
            self.subprocess.execute(tests)
        }
    }
}
```

**Step 2: Update lib.rs exports**

```rust
pub mod core;
pub mod executor;
pub mod inprocess;
pub mod subprocess;

pub use executor::HybridExecutor;
pub use fastest_core::model::{TestItem, TestOutcome, TestResult};
```

**Step 3: Build and test**

```bash
cargo build
cargo test -p fastest-execution
git add crates/fastest-execution/
git commit -m "feat(execution): implement HybridExecutor — PyO3 + subprocess adaptive strategy"
```

---

### Task 19: Implement capture and timeout

**Files:**
- Create: `crates/fastest-execution/src/capture.rs`
- Create: `crates/fastest-execution/src/timeout.rs`

**Step 1: Write capture module**

stdout/stderr capture for in-process execution (the subprocess harness handles its own capture).

**Step 2: Write timeout module**

Test timeout handling — kill tests that exceed the timeout.

**Step 3: Integrate into both executors, test, commit**

```bash
cargo test -p fastest-execution
git add crates/fastest-execution/src/
git commit -m "feat(execution): add stdout/stderr capture and test timeout handling"
```

---

## Phase 9: Incremental Testing

### Task 20: Implement git-based incremental testing

**Files:**
- Create: `crates/fastest-core/src/incremental/mod.rs`
- Create: `crates/fastest-core/src/incremental/cache.rs`
- Create: `crates/fastest-core/src/incremental/impact.rs`
- Modify: `crates/fastest-core/src/lib.rs`

**Step 1: Create incremental directory**

```bash
mkdir -p crates/fastest-core/src/incremental
```

**Step 2: Write failing tests for change detection**

```rust
//! Git-based incremental testing

pub mod cache;
pub mod impact;

use crate::error::Result;
use crate::model::TestItem;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub struct IncrementalTester {
    repo_root: PathBuf,
    cache: cache::ResultCache,
}

impl IncrementalTester {
    pub fn new(repo_root: &Path) -> Result<Self> {
        todo!()
    }

    /// Filter tests to only those affected by recent changes
    pub fn filter_unchanged(&self, tests: Vec<TestItem>) -> Result<Vec<TestItem>> {
        let changed_files = self.get_changed_files()?;
        let affected = impact::find_affected_tests(&tests, &changed_files);
        Ok(affected)
    }

    /// Get files changed since last test run (via git)
    fn get_changed_files(&self) -> Result<HashSet<PathBuf>> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_changed_test_file_is_affected() {
        let tests = vec![make_test("test_a.py", "test_one")];
        let changed = HashSet::from([PathBuf::from("test_a.py")]);
        let affected = impact::find_affected_tests(&tests, &changed);
        assert_eq!(affected.len(), 1);
    }

    #[test]
    fn test_unchanged_file_is_filtered() {
        let tests = vec![make_test("test_a.py", "test_one")];
        let changed = HashSet::from([PathBuf::from("other.py")]);
        let affected = impact::find_affected_tests(&tests, &changed);
        assert_eq!(affected.len(), 0);
    }

    #[test]
    fn test_config_change_affects_all() {
        let tests = vec![
            make_test("test_a.py", "test_one"),
            make_test("test_b.py", "test_two"),
        ];
        let changed = HashSet::from([PathBuf::from("pyproject.toml")]);
        let affected = impact::find_affected_tests(&tests, &changed);
        assert_eq!(affected.len(), 2); // All tests affected by config change
    }

    fn make_test(path: &str, name: &str) -> TestItem {
        TestItem {
            id: format!("{}::{}", path, name),
            path: PathBuf::from(path),
            function_name: name.into(),
            line_number: None,
            decorators: vec![],
            is_async: false,
            fixture_deps: vec![],
            class_name: None,
            markers: vec![],
            parameters: None,
        }
    }
}
```

**Step 3: Implement using git2 for change detection, blake3 for hashing**

Port working logic from `crates/fastest-advanced/src/incremental.rs` (v1), but only the parts that actually work (git status detection, file hashing, impact analysis). Drop the stubs.

**Step 4: Implement cache.rs with LRU**

```rust
//! LRU cache for test results

use crate::model::TestResult;
use lru::LruCache;
use std::num::NonZeroUsize;

pub struct ResultCache {
    cache: LruCache<String, TestResult>,
}

impl ResultCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: LruCache::new(NonZeroUsize::new(capacity).unwrap()),
        }
    }

    pub fn get(&mut self, test_id: &str) -> Option<&TestResult> {
        self.cache.get(test_id)
    }

    pub fn insert(&mut self, test_id: String, result: TestResult) {
        self.cache.put(test_id, result);
    }
}
```

**Step 5: Implement impact.rs**

```rust
//! Impact analysis — which tests are affected by file changes

use crate::model::TestItem;
use std::collections::HashSet;
use std::path::PathBuf;

const CONFIG_FILES: &[&str] = &[
    "pyproject.toml", "pytest.ini", "setup.cfg", "tox.ini",
    "setup.py", "requirements.txt", "requirements-dev.txt",
];

pub fn find_affected_tests(tests: &[TestItem], changed_files: &HashSet<PathBuf>) -> Vec<TestItem> {
    // If a config file changed, all tests are affected
    let config_changed = changed_files.iter().any(|f| {
        CONFIG_FILES.iter().any(|c| f.to_string_lossy().ends_with(c))
    });

    if config_changed {
        return tests.to_vec();
    }

    tests.iter()
        .filter(|t| changed_files.contains(&t.path))
        .cloned()
        .collect()
}
```

**Step 6: Run tests, commit**

```bash
cargo test -p fastest-core incremental
git add crates/fastest-core/src/incremental/ crates/fastest-core/src/lib.rs
git commit -m "feat(core): implement git-based incremental testing with blake3 + LRU cache"
```

---

## Phase 10: Watch Mode

### Task 21: Implement file watching

**Files:**
- Create: `crates/fastest-core/src/watch.rs`
- Modify: `crates/fastest-core/src/lib.rs`

**Step 1: Write watch module**

```rust
//! File watching with debounced re-execution

use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

pub struct TestWatcher {
    debounce_ms: u64,
}

impl TestWatcher {
    pub fn new(debounce_ms: u64) -> Self {
        Self { debounce_ms }
    }

    /// Watch for file changes and call the callback when Python files change
    pub fn watch<F>(&self, path: &Path, mut on_change: F) -> crate::error::Result<()>
    where
        F: FnMut(&[std::path::PathBuf]) + Send + 'static,
    {
        let (tx, rx) = mpsc::channel();
        let mut watcher = RecommendedWatcher::new(tx, Config::default())
            .map_err(|e| crate::error::Error::Other(e.into()))?;

        watcher.watch(path, RecursiveMode::Recursive)
            .map_err(|e| crate::error::Error::Other(e.into()))?;

        let debounce = Duration::from_millis(self.debounce_ms);
        let mut pending_changes = Vec::new();

        loop {
            match rx.recv_timeout(debounce) {
                Ok(Ok(event)) => {
                    if matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                        for path in event.paths {
                            if path.extension().map_or(false, |e| e == "py") {
                                if !pending_changes.contains(&path) {
                                    pending_changes.push(path);
                                }
                            }
                        }
                    }
                }
                Ok(Err(_)) => continue,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if !pending_changes.is_empty() {
                        let changes: Vec<_> = pending_changes.drain(..).collect();
                        on_change(&changes);
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }

        Ok(())
    }
}
```

**Step 2: Update lib.rs, build, commit**

```bash
cargo build -p fastest-core
git add crates/fastest-core/src/watch.rs crates/fastest-core/src/lib.rs
git commit -m "feat(core): implement file watching with debounced re-execution"
```

---

## Phase 11: CLI

### Task 22: Implement the CLI

**Files:**
- Modify: `crates/fastest-cli/src/main.rs`
- Create: `crates/fastest-cli/src/output.rs`
- Create: `crates/fastest-cli/src/progress.rs`

**Step 1: Write the full CLI with clap**

Port the CLI argument structure from `crates/fastest-cli/src/main.rs` (v1), but clean:
- `fastest [paths]` — run tests
- `fastest discover [paths]` — list tests without running
- `fastest --watch` — watch mode
- `fastest --incremental` — incremental mode
- `-k EXPR` — keyword filter
- `-m EXPR` — marker filter
- `--output json|pretty|count` — output format
- `--junit-xml PATH` — JUnit XML output
- `-x` — stop on first failure
- `-v` — verbose
- `--no-plugins` — disable plugins
- `--plugin-dir PATH` — external plugins directory
- `--workers N` — number of parallel workers

**Step 2: Wire up the data flow**

The main function should follow the data flow from the design doc:
1. Parse args → Config::load
2. PluginManager::with_builtins()
3. discover_tests(paths, config)
4. expand_parametrized_tests
5. filter_by_markers / filter_by_keyword
6. plugins.call_hook("collection_modifyitems")
7. Optional: IncrementalTester::filter_unchanged
8. HybridExecutor::execute
9. plugins.call_hook("runtest_logreport")
10. Format output

**Step 3: Write output.rs**

```rust
//! Output formatting — pretty, JSON, JUnit XML

use fastest_core::model::{TestOutcome, TestResult};
use colored::*;

pub enum OutputFormat {
    Pretty,
    Json,
    Count,
    JunitXml(String), // path to write XML
}

pub fn format_results(results: &[TestResult], format: &OutputFormat) -> String {
    match format {
        OutputFormat::Pretty => format_pretty(results),
        OutputFormat::Json => serde_json::to_string_pretty(results).unwrap_or_default(),
        OutputFormat::Count => format_count(results),
        OutputFormat::JunitXml(path) => {
            write_junit_xml(results, path);
            format_pretty(results)
        }
    }
}

fn format_pretty(results: &[TestResult]) -> String {
    // Implement pytest-style output
    todo!()
}

fn format_count(results: &[TestResult]) -> String {
    let passed = results.iter().filter(|r| r.outcome == TestOutcome::Passed).count();
    let failed = results.iter().filter(|r| r.outcome == TestOutcome::Failed).count();
    let skipped = results.iter().filter(|r| matches!(r.outcome, TestOutcome::Skipped { .. })).count();
    format!("{} passed, {} failed, {} skipped", passed, failed, skipped)
}

fn write_junit_xml(results: &[TestResult], path: &str) {
    // Generate JUnit XML
    todo!()
}
```

**Step 4: Write progress.rs**

```rust
//! Progress bar display

use indicatif::{ProgressBar, ProgressStyle};

pub fn create_progress_bar(total: usize) -> ProgressBar {
    let pb = ProgressBar::new(total as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
            .unwrap()
            .progress_chars("=>-"),
    );
    pb
}
```

**Step 5: Build, test manually, commit**

```bash
cargo build
cargo run -- --help
git add crates/fastest-cli/
git commit -m "feat(cli): implement CLI with pretty/JSON/JUnit output, progress bars"
```

---

## Phase 12: Integration Tests

### Task 23: Write integration tests

**Files:**
- Create: `crates/fastest-cli/tests/integration_test.rs` (overwrite)

**Step 1: Write integration tests**

```rust
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;

#[test]
fn test_version() {
    Command::cargo_bin("fastest")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("fastest"));
}

#[test]
fn test_help() {
    Command::cargo_bin("fastest")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Blazing-fast Python test runner"));
}

#[test]
fn test_discover_basic() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("test_example.py"), r#"
def test_one():
    assert True

def test_two():
    assert True
"#).unwrap();

    Command::cargo_bin("fastest")
        .unwrap()
        .arg("discover")
        .arg(dir.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("test_one"))
        .stdout(predicate::str::contains("test_two"));
}

#[test]
fn test_no_tests_found() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("helper.py"), "def helper(): pass").unwrap();

    Command::cargo_bin("fastest")
        .unwrap()
        .arg(dir.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("0"));
}
```

**Step 2: Run integration tests**

```bash
cargo test -p fastest-cli --test integration_test
```

**Step 3: Commit**

```bash
git add crates/fastest-cli/tests/
git commit -m "test: add integration tests for CLI"
```

---

## Phase 13: Final Cleanup

### Task 24: Remove any remaining dead code and verify clean build

**Step 1: Check for warnings**

```bash
cargo build 2>&1 | grep -i "warning"
```

Fix any warnings — no `#[allow(dead_code)]` permitted.

**Step 2: Check for unused dependencies**

```bash
cargo install cargo-udeps
cargo +nightly udeps --workspace 2>&1
```

Remove any unused dependencies from Cargo.toml files.

**Step 3: Run all tests**

```bash
cargo test --workspace
```

Expected: All tests pass, no warnings.

**Step 4: Verify binary works**

```bash
cargo run -- --help
cargo run -- --version
cargo run -- discover .
```

**Step 5: Format and lint**

```bash
cargo fmt --all
cargo clippy --workspace -- -D warnings
```

**Step 6: Final commit**

```bash
git add -A
git commit -m "chore: final cleanup — no dead code, no warnings, all tests pass"
```

---

### Task 25: Update Cargo.lock

**Step 1: Clean rebuild**

```bash
cargo clean
cargo build --release
```

**Step 2: Commit the lock file**

```bash
git add Cargo.lock
git commit -m "chore: update Cargo.lock for v2 dependencies"
```

---

## Summary

| Phase | Tasks | What's Built |
|-------|-------|-------------|
| Phase 0 | 1-6 | Clean scaffold, delete dead code, 3-crate workspace compiles |
| Phase 1 | 7 | Core model types (TestItem, TestResult, TestOutcome) |
| Phase 2 | 8 | Config loading (pyproject.toml, pytest.ini, setup.cfg) |
| Phase 3 | 9-11 | Test discovery (AST parser, parallel walking, cache) |
| Phase 4 | 12 | Marker system (skip, xfail, skipif, filtering) |
| Phase 5 | 13 | Parametrize expansion |
| Phase 6 | 14 | Fixture system (resolution, built-ins, conftest, scoping) |
| Phase 7 | 15 | Plugin system (trait, hooks, built-in plugins, loader) |
| Phase 8 | 16-19 | Execution engine (InProcess, SubprocessPool, HybridExecutor, capture, timeout) |
| Phase 9 | 20 | Incremental testing (git2, blake3, LRU cache) |
| Phase 10 | 21 | Watch mode (notify, debounce) |
| Phase 11 | 22 | CLI (clap, output formatting, progress bars) |
| Phase 12 | 23 | Integration tests |
| Phase 13 | 24-25 | Final cleanup and verification |

**Total: 25 tasks across 14 phases**

Each task follows TDD: write failing test → implement → verify pass → commit.
