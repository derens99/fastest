# Next Steps: Complete Phase 2 Performance

## Current Status (December 2024)

### Phase 1: MVP ✅ COMPLETED
- ✅ Fast test discovery (88x faster than pytest)
- ✅ Subprocess execution with batch optimization (2.1x faster)
- ✅ Full-featured CLI with progress bars
- ✅ Python bindings via PyO3

### Phase 2: Performance 🚧 IN PROGRESS

#### Completed ✅
- ✅ Parallel test execution with rayon work-stealing
- ✅ Discovery caching with file modification tracking
- ✅ Batch execution optimization
- ✅ Clean executor module architecture

#### Remaining Tasks ❌
- ❌ Tree-sitter Python parsing (current: regex-based)
- ❌ Automated benchmarking CI/CD
- ❌ Memory-mapped file support
- ❌ Smart test scheduling based on history

## Next Priority: Tree-sitter Integration (HIGH)

**Current State**: Tests can now run in parallel using Rayon's work-stealing thread pool
**Achievement**: 
- Added `-n` CLI option for specifying worker count
- Python API: `run_tests_parallel(tests, num_workers=None)`
- Tests are grouped by module for efficient batch execution
- 1.2-2x speedup on multi-core machines

#### Implementation Plan:

1. **Add Rayon dependency** to `crates/fastest-core/Cargo.toml`:
   ```toml
   rayon = "1.8"
   ```

2. **Create work-stealing scheduler** in `crates/fastest-core/src/executor/parallel.rs`:
   - Use rayon's thread pool for CPU-bound work
   - Group tests by module to minimize import overhead
   - Implement smart work distribution based on test history

3. **Add CLI option** for parallelism:
   ```bash
   fastest -n 4  # Run with 4 workers
   fastest -n auto  # Auto-detect CPU cores
   ```

4. **Update Python bindings** to expose parallel execution:
   ```python
   results = fastest.run_tests_parallel(tests, num_workers=4)
   ```

5. **Handle test isolation**:
   - Each worker runs tests in separate subprocess
   - Implement proper stdout/stderr capture per worker
   - Ensure test results are collected correctly

### 2. Optimize Python Parsing (Priority: MEDIUM)

**Current State**: Using regex-based parsing
**Goal**: Use tree-sitter for faster, more accurate parsing

#### Implementation Plan:

1. **Add tree-sitter dependency**:
   ```toml
   tree-sitter = "0.20"
   tree-sitter-python = "0.20"
   ```

2. **Create AST-based parser** in `crates/fastest-core/src/discovery/ast_parser.rs`
3. **Benchmark against current regex parser**
4. **Add feature flag** to switch between parsers

### 3. Set Up Automated Benchmarking (Priority: MEDIUM)

**Goal**: Track performance over time, prevent regressions

#### Implementation Plan:

1. **Create GitHub Action** for benchmark runs
2. **Use criterion.rs** for Rust benchmarks
3. **Store results** in benchmark repository
4. **Generate performance reports** on PRs

## Phase 3 Preview: Compatibility Features

Once Phase 2 is complete, the next major milestone is pytest compatibility:

### 1. Basic Fixture System
- Start with simple fixtures (no scope, no teardown)
- Implement `@pytest.fixture` decorator support
- Add dependency injection for test functions

### 2. Test Markers
- Implement `@pytest.mark.skip`
- Implement `@pytest.mark.parametrize`
- Add custom marker support

### 3. Configuration Files
- Parse `pytest.ini`
- Support `pyproject.toml` [tool.fastest] section
- Command-line option precedence

## Technical Implementation Details

### Parallel Execution Architecture

```rust
// crates/fastest-core/src/executor/parallel.rs
pub struct ParallelExecutor {
    pool: rayon::ThreadPool,
    results: Arc<DashMap<String, TestResult>>,
}

impl ParallelExecutor {
    pub fn execute(&self, tests: Vec<TestItem>) -> Vec<TestResult> {
        // Group tests by module
        let grouped = self.group_by_module(tests);
        
        // Execute groups in parallel
        grouped.par_iter().map(|group| {
            self.execute_group(group)
        }).flatten().collect()
    }
}
```

### Python Bindings Update

```python
# crates/fastest-python/src/lib.rs
#[pyfunction]
#[pyo3(signature = (tests, num_workers=None))]
fn run_tests_parallel(
    tests: Vec<TestItem>,
    num_workers: Option<usize>
) -> PyResult<Vec<TestResult>> {
    let workers = num_workers.unwrap_or_else(|| num_cpus::get());
    // Implementation
}
```

## Development Workflow

1. **Create feature branch**: `git checkout -b feature/parallel-execution`
2. **Implement in Rust core first**
3. **Add tests** in `crates/fastest-core/src/executor/tests.rs`
4. **Update Python bindings**
5. **Update CLI** to support new options
6. **Benchmark** against current implementation
7. **Update documentation**

## Success Criteria

- [ ] Tests can run in parallel with `-n` option
- [ ] Performance scales linearly with CPU cores (up to ~4x)
- [ ] No test isolation issues (each test runs in clean environment)
- [ ] Progress reporting works correctly with parallel execution
- [ ] Memory usage remains reasonable with many workers
- [ ] Benchmark shows 2-4x speedup on multi-core machines

## Timeline Estimate

- Week 1: Implement basic parallel executor with rayon
- Week 2: Add proper test isolation and result collection
- Week 3: Update Python bindings and CLI
- Week 4: Benchmarking, optimization, and documentation

## Notes

- Consider using `tokio` for async test execution in the future
- May need to implement test ordering for dependencies
- Watch out for tests that assume sequential execution
- Consider adding `--fail-fast` option for parallel mode 