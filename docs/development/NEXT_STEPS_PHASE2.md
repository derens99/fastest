# Phase 2 Completion: Next Steps

## 1. Tree-sitter Python Parsing (Week 1-2)

**Why**: Current regex parsing misses edge cases and can't handle complex Python syntax properly.

**Implementation Plan:**

### Step 1: Add Dependencies
```toml
# crates/fastest-core/Cargo.toml
tree-sitter = "0.22"
tree-sitter-python = "0.21"
```

### Step 2: Create AST-based Parser
```rust
// crates/fastest-core/src/parser/ast.rs
pub struct AstParser {
    parser: Parser,
    python_language: Language,
}

impl AstParser {
    pub fn parse_file(content: &str) -> Result<Vec<TestFunction>> {
        // Parse Python AST
        // Walk tree to find test functions
        // Handle decorators, async, classes properly
    }
}
```

### Step 3: Benchmark & Compare
- Parse test_project with both parsers
- Measure speed and accuracy
- Switch to tree-sitter if better

### Expected Benefits:
- More accurate test discovery
- Support for decorated tests
- Better handling of nested classes
- Foundation for future features (fixtures, parametrize)

## 2. Automated Benchmarking System (Week 3)

**Why**: Need to prevent performance regressions and track improvements.

### Step 1: Create Benchmark Suite
```yaml
# .github/workflows/benchmark.yml
name: Benchmark
on: [push, pull_request]
jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Run benchmarks
        run: cargo bench
      - name: Store benchmark result
        uses: benchmark-action/github-action-benchmark@v1
```

### Step 2: Criterion Benchmarks
```rust
// benches/discovery_bench.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn discovery_benchmark(c: &mut Criterion) {
    c.bench_function("discover 1000 tests", |b| {
        b.iter(|| discover_tests(black_box("large_test_dir")))
    });
}
```

### Step 3: Python Benchmarks
```python
# benchmarks/automated_benchmark.py
def benchmark_against_pytest():
    """Compare fastest vs pytest on various workloads"""
    # Small suite (10 tests)
    # Medium suite (100 tests)
    # Large suite (1000 tests)
    # Generate performance report
```

## 3. Performance Optimizations (Week 4)

### Memory-mapped Files
```rust
// For large Python files
use memmap2::Mmap;

pub fn parse_large_file(path: &Path) -> Result<Vec<TestFunction>> {
    let file = File::open(path)?;
    let mmap = unsafe { Mmap::map(&file)? };
    // Parse from memory-mapped data
}
```

### Lazy Module Imports
```python
# Optimize batch executor to delay imports
def lazy_import(module_name):
    """Import module only when first test runs"""
```

### Smart Test Scheduling
```rust
// Schedule slow tests first for better parallelization
pub fn schedule_tests(tests: Vec<TestItem>, history: &TestHistory) -> Vec<TestItem> {
    tests.sort_by_key(|t| history.avg_duration(t).unwrap_or(Duration::ZERO));
    tests.reverse() // Slowest first
}
```

## Timeline

- **Week 1-2**: Tree-sitter integration
- **Week 3**: Automated benchmarking
- **Week 4**: Performance optimizations

## Success Criteria

- [ ] Tree-sitter parser handles all pytest patterns correctly
- [ ] Automated benchmarks run on every PR
- [ ] Performance dashboard shows trends over time
- [ ] Memory usage reduced by 20%+ for large files
- [ ] No performance regression vs current implementation

## Next After Phase 2

Once Phase 2 is complete, we'll move to Phase 3 (Compatibility):
- Basic fixture support
- Test markers (@pytest.mark.skip, @pytest.mark.parametrize)
- Configuration file support
- Plugin system foundation 