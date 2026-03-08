# Contributing to Fastest

Thank you for your interest in contributing to Fastest! This guide will help you get started.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/your-username/fastest.git`
3. Create a feature branch: `git checkout -b feature/your-feature`

## Development Setup

### Prerequisites
- Rust stable toolchain (install via [rustup](https://rustup.rs/))
- Python 3.9+
- Git

### Building
```bash
# Build all crates
cargo build --workspace

# Run tests
cargo test --workspace

# Check formatting and lints
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

### Testing with Python files
```bash
# Build release binary
cargo build --release

# Run against example test files
./target/release/fastest tests/checks/ -v

# Compare with pytest
pytest tests/checks/ -v
```

## Code Guidelines

### Rust Code
- Run `cargo fmt` before committing
- Ensure `cargo clippy` passes with zero warnings
- Add tests for new functionality
- Document public APIs

### Commit Messages
Use [Conventional Commits](https://www.conventionalcommits.org/) — they drive automated releases:
- `feat:` — new feature (minor version bump)
- `fix:` — bug fix (patch version bump)
- `docs:` — documentation only
- `chore:` — maintenance (no release)

## Pull Request Process

1. Ensure all tests pass (`cargo test --workspace`)
2. Ensure no clippy warnings (`cargo clippy --workspace --all-targets -- -D warnings`)
3. Update documentation if needed
4. Create PR with clear description
5. Wait for review

## Reporting Issues

- Include a minimal reproducible example
- Specify Python and OS versions
- Attach relevant error output

## Areas for Contribution

- Bug fixes
- Documentation improvements
- Test coverage
- Performance optimizations
- Pytest compatibility improvements
- Platform-specific fixes

## Questions?

Feel free to open a discussion or reach out in issues!
