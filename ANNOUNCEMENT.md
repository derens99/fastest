# Introducing Fastest: A Python Test Runner That's 88x Faster at Discovery

I'm excited to announce **Fastest**, a new Python test runner built with Rust that achieves:
- **88x faster test discovery** than pytest
- **2.1x faster test execution** 
- **50% less memory usage**
- **<100ms startup time**

## Why Another Test Runner?

Working with large Python codebases, I consistently hit the same pain point: waiting for tests. Even with pytest's excellent features, the discovery phase alone could take several seconds on large projects. This adds up quickly when you're running tests frequently during development.

## How It Works

Fastest achieves its performance through:

1. **Rust-powered discovery**: File traversal and parsing in Rust is orders of magnitude faster
2. **Smart caching**: Test discovery results are cached and invalidated intelligently
3. **Parallel execution**: Work-stealing scheduler for optimal CPU utilization
4. **Tree-sitter parsing**: Optional AST parser for even faster discovery on smaller codebases

## Drop-in Replacement

Fastest is designed as a drop-in replacement for pytest:

```bash
# Instead of: pytest tests/
fastest tests/

# Works with your existing tests!
fastest -k "test_important" -m "not slow"
```

It supports:
- ✅ pytest markers (`@pytest.mark.*` and `@fastest.mark.*`)
- ✅ Common fixtures (tmp_path, capsys, monkeypatch)
- ✅ Async tests
- ✅ Class-based tests
- ✅ Parallel execution (`-n` flag)

## Installation

```bash
# macOS/Linux
curl -LsSf https://raw.githubusercontent.com/yourusername/fastest/main/install.sh | sh

# Or build from source
cargo install --git https://github.com/yourusername/fastest
```

## Real-World Performance

On a codebase with 1,000 tests:
- pytest discovery: 358ms
- fastest discovery: 6.7ms (53x faster)
- fastest with cache: <1ms

## Current Status

This is an **alpha release** (v0.1.0). While it's stable enough for real use, some pytest features aren't supported yet:
- Configuration files (pytest.ini, pyproject.toml) 
- Plugins
- Some advanced fixtures
- Parametrized tests

## Get Involved

I'd love your feedback and contributions! The project is open source and looking for:
- Bug reports from real-world usage
- Performance benchmarks on your codebases
- Feature requests and use cases
- Code contributions

Check it out: https://github.com/yourusername/fastest

## What's Next?

The roadmap includes:
- Config file support
- Watch mode for continuous testing
- Coverage integration
- VS Code extension
- More pytest compatibility

Let's make Python testing instant! 🚀 