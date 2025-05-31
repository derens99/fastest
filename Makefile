# Fastest Development Makefile
# Provides easy commands for development workflow

# Variables
FASTEST_BINARY ?= ./target/release/fastest
PYTEST_BINARY ?= pytest
PYTHON ?= python3
TEST_DIR ?= tests/compatibility
COMPARISON_RUNS ?= 3

# Colors for output
GREEN = \033[0;32m
YELLOW = \033[1;33m
RED = \033[0;31m
NC = \033[0m # No Color

.PHONY: all build test clean install dev-setup lint format coverage bench release compare dashboard perf-track

# Default target
all: build test

# Build the project
build:
	@echo "$(GREEN)Building fastest binary...$(NC)"
	cargo build --release

# Run all tests
test:
	cargo test --all-features
	cargo test --test '*' --all-features

# Run integration tests with Python
test-integration: build
	./target/release/fastest-cli tests/ -v

# Compare performance with pytest
compare:
	@echo "$(GREEN)Comparing fastest vs pytest...$(NC)"
	@if [ -f "scripts/compare_with_pytest.py" ]; then \
		$(PYTHON) scripts/compare_with_pytest.py $(TEST_DIR) --fastest-binary $(FASTEST_BINARY); \
	else \
		echo "$(RED)❌ Comparison script not found$(NC)"; \
		exit 1; \
	fi

# Show development dashboard
dashboard:
	@echo "$(GREEN)Showing development dashboard...$(NC)"
	@if [ -f "scripts/development_dashboard.py" ]; then \
		$(PYTHON) scripts/development_dashboard.py --fastest-binary $(FASTEST_BINARY); \
	else \
		echo "$(RED)❌ Dashboard script not found$(NC)"; \
		exit 1; \
	fi

# Track performance metrics
perf-track:
	@echo "$(GREEN)Tracking performance metrics...$(NC)"
	@if [ -f "scripts/track_performance_regression.py" ]; then \
		$(PYTHON) scripts/track_performance_regression.py --binary $(FASTEST_BINARY) --test-dir $(TEST_DIR) --runs $(COMPARISON_RUNS); \
	else \
		echo "$(RED)❌ Performance tracking script not found$(NC)"; \
		exit 1; \
	fi

# Generate performance report
perf-report:
	@echo "$(GREEN)Generating performance report...$(NC)"
	@if [ -f "scripts/track_performance_regression.py" ]; then \
		$(PYTHON) scripts/track_performance_regression.py --report; \
	else \
		echo "$(RED)❌ Performance tracking script not found$(NC)"; \
		exit 1; \
	fi

# Quick development status check
quick-check: build
	@echo "$(GREEN)Quick development status check...$(NC)"
	@echo "🔍 Git status:"
	@git status --short || echo "$(YELLOW)⚠️  Not a git repository$(NC)"
	@echo ""
	@echo "🏗️  Binary status:"
	@if [ -f "target/release/fastest-cli" ]; then \
		echo "$(GREEN)✅ fastest-cli binary exists$(NC)"; \
		./target/release/fastest-cli --version 2>/dev/null || echo "$(YELLOW)⚠️  Binary version check failed$(NC)"; \
	else \
		echo "$(RED)❌ fastest-cli binary not found$(NC)"; \
	fi

# Watch mode dashboard
watch-dashboard:
	@echo "$(GREEN)Starting watch mode dashboard...$(NC)"
	@if [ -f "scripts/development_dashboard.py" ]; then \
		$(PYTHON) scripts/development_dashboard.py --fastest-binary $(FASTEST_BINARY) --watch; \
	else \
		echo "$(RED)❌ Dashboard script not found$(NC)"; \
		exit 1; \
	fi

# Run all compatibility tests
test-compatibility:
	@echo "$(GREEN)Running all compatibility tests...$(NC)"
	@if [ -d "tests/compatibility" ]; then \
		$(FASTEST_BINARY) tests/compatibility/; \
	else \
		echo "$(YELLOW)⚠️  Compatibility tests not found, creating sample...$(NC)"; \
		$(PYTHON) scripts/compare_with_pytest.py --create-sample 20; \
	fi

# Full validation suite
validate: build compare perf-track test-compatibility
	@echo "$(GREEN)Running full validation suite...$(NC)"
	@echo "$(GREEN)✅ Full validation complete$(NC)"

# Development cycle: build, test, compare
dev-cycle: build test compare dashboard
	@echo "$(GREEN)✅ Development cycle complete$(NC)"

# Clean build artifacts
clean:
	cargo clean
	rm -rf target/
	rm -rf htmlcov/
	rm -rf .coverage*
	rm -rf tarpaulin-report.html
	find . -type d -name __pycache__ -exec rm -rf {} +
	find . -type f -name "*.pyc" -delete

# Clean performance and comparison data
clean-data:
	@echo "$(GREEN)Cleaning performance and comparison data...$(NC)"
	@rm -rf comparison_results/ performance_data/
	@rm -f *.png *.json *.log
	@echo "$(GREEN)✅ Data cleaned$(NC)"

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
	@echo "📁 Creating directories..."
	@mkdir -p comparison_results performance_data tests/compatibility
	@echo "$(GREEN)✅ Development environment ready$(NC)"

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
	strip target/release/fastest-cli
	ls -lh target/release/fastest-cli

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
	cargo flamegraph --bin fastest-cli -- tests/

# Update dependencies
update:
	cargo update
	cargo outdated

# Clean and rebuild everything
rebuild: clean build test

# Show project status
status:
	@echo "$(GREEN)Fastest Project Status$(NC)"
	@echo "======================"
	@echo "📁 Project: $(PWD)"
	@echo "🦀 Rust: $(shell rustc --version 2>/dev/null || echo 'Not installed')"
	@echo "🐍 Python: $(shell $(PYTHON) --version 2>/dev/null || echo 'Not installed')"
	@echo "📦 Cargo: $(shell cargo --version 2>/dev/null || echo 'Not installed')"
	@echo "🧪 pytest: $(shell $(PYTEST_BINARY) --version 2>/dev/null | head -1 || echo 'Not installed')"

# Help target
help:
	@echo "$(GREEN)Fastest Development Commands$(NC)"
	@echo "=============================="
	@echo ""
	@echo "$(YELLOW)Core Development:$(NC)"
	@echo "  make build            - Build the fastest binary"
	@echo "  make test             - Run all tests"
	@echo "  make compare          - Compare fastest vs pytest performance"
	@echo "  make dashboard        - Show development dashboard"
	@echo ""
	@echo "$(YELLOW)Performance Tracking:$(NC)"
	@echo "  make perf-track       - Track performance metrics"
	@echo "  make perf-report      - Generate performance report"
	@echo ""
	@echo "$(YELLOW)Development Workflow:$(NC)"
	@echo "  make dev-setup        - Set up development environment"
	@echo "  make quick-check      - Quick development status check"
	@echo "  make watch-dashboard  - Watch mode dashboard"
	@echo "  make dev-cycle        - Build, test, compare, dashboard"
	@echo ""
	@echo "$(YELLOW)Testing & Validation:$(NC)"
	@echo "  make test-compatibility - Run all compatibility tests"
	@echo "  make validate           - Full validation suite"
	@echo ""
	@echo "$(YELLOW)Original Commands:$(NC)"
	@echo "  make install      - Install the binary"
	@echo "  make lint         - Run linters"
	@echo "  make format       - Format code"
	@echo "  make coverage     - Generate coverage report"
	@echo "  make bench        - Run benchmarks"
	@echo "  make audit        - Run security audit"
	@echo "  make release      - Create optimized release build"
	@echo "  make clean        - Clean build artifacts"
	@echo "  make clean-data   - Clean performance and comparison data"
	@echo "  make status       - Show project status" 