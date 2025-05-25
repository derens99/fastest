.PHONY: help build test clean install dev benchmark lint format

help:
	@echo "Available commands:"
	@echo "  make build      - Build the Rust project in release mode"
	@echo "  make dev        - Build and install for development"
	@echo "  make test       - Run tests"
	@echo "  make benchmark  - Run performance benchmarks"
	@echo "  make clean      - Clean build artifacts and caches"
	@echo "  make lint       - Run linters"
	@echo "  make format     - Format code"

build:
	cargo build --release

dev:
	maturin develop

test:
	# Run Rust tests
	cargo test
	# Run Python test scripts
	@if [ -f "fastest.*.so" ] || [ -f "fastest.pyd" ]; then \
		python tests/test_fastest.py; \
		python tests/test_enhanced.py; \
	else \
		echo "Python bindings not built. Run 'make dev' first."; \
	fi

benchmark:
	@if [ -f "fastest.*.so" ] || [ -f "fastest.pyd" ]; then \
		python benchmarks/benchmark.py; \
	else \
		echo "Python bindings not built. Run 'make dev' first."; \
	fi

clean:
	cargo clean
	rm -rf __pycache__ .pytest_cache
	find . -type d -name "__pycache__" -exec rm -rf {} + 2>/dev/null || true
	find . -type f -name "*.pyc" -delete 2>/dev/null || true
	rm -f fastest.*.so fastest.pyd

lint:
	cargo clippy -- -D warnings
	@if command -v ruff >/dev/null 2>&1; then \
		ruff check .; \
	else \
		echo "ruff not installed. Run 'pip install -r requirements-dev.txt'"; \
	fi

format:
	cargo fmt
	@if command -v black >/dev/null 2>&1; then \
		black . --exclude "/(\.venv|target|test_project)/"; \
	else \
		echo "black not installed. Run 'pip install -r requirements-dev.txt'"; \
	fi 