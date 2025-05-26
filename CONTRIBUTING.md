# Contributing to Fastest

Thank you for your interest in contributing to Fastest! This guide will help you get started.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/yourusername/fastest.git`
3. Create a feature branch: `git checkout -b feature/your-feature`

## Development Setup

### Prerequisites
- Rust 1.70+ (install via [rustup](https://rustup.rs/))
- Python 3.8+ 
- maturin (`pip install maturin`)

### Building
```bash
# Build all crates
cargo build

# Build Python bindings
maturin develop

# Run tests
cargo test
python -m pytest tests/
```

## Code Guidelines

### Rust Code
- Follow standard Rust formatting (`cargo fmt`)
- Ensure no clippy warnings (`cargo clippy`)
- Add tests for new functionality
- Document public APIs

### Python Code
- Follow PEP 8
- Use type hints where possible
- Add docstrings to public functions

## Pull Request Process

1. Ensure all tests pass
2. Update documentation if needed
3. Add entry to CHANGELOG.md
4. Create PR with clear description
5. Wait for review

## Reporting Issues

- Use issue templates
- Include minimal reproducible example
- Specify Python/Rust versions
- Attach relevant logs

## Areas for Contribution

- 🐛 Bug fixes
- 📚 Documentation improvements
- 🧪 Test coverage
- 🚀 Performance optimizations
- 🔧 Config file support
- 🔌 Plugin system design
- 🌍 Platform-specific fixes

## Questions?

Feel free to open a discussion or reach out in issues! 