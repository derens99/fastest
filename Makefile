.PHONY: all build test clean install dev-setup lint format coverage bench release

# Default target
all: build test

# Build the project
build:
	cargo build --release

# Run all tests
test:
	cargo test --all-features
	cargo test --test '*' --all-features

# Run integration tests with Python
test-integration: build
	./target/release/fastest tests/ -v

# Clean build artifacts
clean:
	cargo clean
	rm -rf target/
	rm -rf htmlcov/
	rm -rf .coverage*
	rm -rf tarpaulin-report.html
	find . -type d -name __pycache__ -exec rm -rf {} +
	find . -type f -name "*.pyc" -delete

# Install the binary
install: build
	cargo install --path crates/fastest-cli

# Set up development environment
dev-setup:
	rustup component add rustfmt clippy
	cargo install cargo-watch cargo-tarpaulin cargo-audit
	python -m venv venv
	./venv/bin/pip install -r requirements-dev.txt
	pre-commit install

# Run linters
lint:
	cargo fmt -- --check
	cargo clippy --all-targets --all-features -- -D warnings
	./venv/bin/python -m ruff check .
	./venv/bin/python -m black --check .

# Format code
format:
	cargo fmt
	./venv/bin/python -m ruff check --fix .
	./venv/bin/python -m black .

# Run coverage
coverage:
	cargo tarpaulin --out Html --all-features
	open tarpaulin-report.html

# Run benchmarks
bench: build
	python benchmarks/bench_discovery.py
	python benchmarks/bench_performance.py

# Run security audit
audit:
	cargo audit

# Create a release build
release:
	cargo build --release --all-features
	strip target/release/fastest
	ls -lh target/release/fastest

# Docker operations
docker-build:
	docker build -t fastest:latest .

docker-run:
	docker run --rm -v $(PWD):/workspace fastest:latest tests/

# Documentation
docs:
	cargo doc --no-deps --open

# Watch for changes and run tests
watch:
	cargo watch -x test -x clippy

# Run CI checks locally
ci-local:
	act -j test

# Quick check before committing
check: format lint test

# Performance profiling
profile: build
	cargo flamegraph --bin fastest -- tests/

# Update dependencies
update:
	cargo update
	cargo outdated

# Clean and rebuild everything
rebuild: clean build test

# Help target
help:
	@echo "Available targets:"
	@echo "  all          - Build and test (default)"
	@echo "  build        - Build the project in release mode"
	@echo "  test         - Run all tests"
	@echo "  test-integration - Run integration tests with Python"
	@echo "  clean        - Clean build artifacts"
	@echo "  install      - Install the binary"
	@echo "  dev-setup    - Set up development environment"
	@echo "  lint         - Run linters"
	@echo "  format       - Format code"
	@echo "  coverage     - Generate coverage report"
	@echo "  bench        - Run benchmarks"
	@echo "  audit        - Run security audit"
	@echo "  release      - Create optimized release build"
	@echo "  docker-build - Build Docker image"
	@echo "  docker-run   - Run tests in Docker"
	@echo "  docs         - Generate and open documentation"
	@echo "  watch        - Watch for changes and run tests"
	@echo "  check        - Format, lint, and test"
	@echo "  profile      - Run performance profiling"
	@echo "  update       - Update dependencies"
	@echo "  rebuild      - Clean and rebuild everything" 