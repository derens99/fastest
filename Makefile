# Fastest Development Makefile
# Provides easy commands for development workflow

# Variables
FASTEST_BINARY ?= ./target/release/fastest
DEV_FASTEST_BINARY ?= ./target/debug/fastest
PYTEST_BINARY ?= uv run pytest
PYTHON ?= uv run python
PYO3_PYTHON ?= $(shell command -v python3.12 2>/dev/null || command -v python3)
TEST_DIR ?= pytest-compat-suite/core/basic
COMPAT_SUITES ?= core/basic features/fixtures
COMPAT_REPORT ?= target/compatibility-report.json
COMPAT_REPORT_ALL ?= target/compatibility-report-all.json
COMPAT_SUITE_TIMEOUT ?= 30
COMPARISON_RUNS ?= 3

# Colors for output
GREEN = \033[0;32m
YELLOW = \033[1;33m
RED = \033[0;31m
NC = \033[0m # No Color

.PHONY: all build test verify plugin-smoke compat-report compat-report-all clean install dev-setup lint format coverage bench release compare

# Default target
all: build test

# Build the project
build:
	@echo "$(GREEN)Building fastest binary...$(NC)"
	PYO3_PYTHON=$(PYO3_PYTHON) cargo build --release

# Run all tests
test:
	PYO3_PYTHON=$(PYO3_PYTHON) cargo test --all-features
	PYO3_PYTHON=$(PYO3_PYTHON) cargo test --test '*' --all-features

# Run integration tests with Python
test-integration: build
	$(FASTEST_BINARY) tests/ -v

# Compare performance with pytest
compare:
	@echo "$(GREEN)Comparing fastest vs pytest...$(NC)"
	@if [ -f "scripts/benchmarks/compare.py" ]; then \
		$(PYTHON) scripts/benchmarks/compare.py $(TEST_DIR) --fastest-binary $(FASTEST_BINARY); \
	else \
		echo "$(RED)❌ Comparison script not found$(NC)"; \
		exit 1; \
	fi

# Quick development status check
quick-check: build
	@echo "$(GREEN)Quick development status check...$(NC)"
	@echo "🔍 Git status:"
	@git status --short || echo "$(YELLOW)⚠️  Not a git repository$(NC)"
	@echo ""
	@echo "🏗️  Binary status:"
	@if [ -f "$(FASTEST_BINARY)" ]; then \
		echo "$(GREEN)✅ fastest binary exists$(NC)"; \
		$(FASTEST_BINARY) version 2>/dev/null || echo "$(YELLOW)⚠️  Binary version check failed$(NC)"; \
	else \
		echo "$(RED)❌ fastest binary not found$(NC)"; \
	fi


# Run all compatibility tests
test-compatibility:
	@echo "$(GREEN)Running all compatibility tests...$(NC)"
	@if [ -d "pytest-compat-suite" ]; then \
		$(FASTEST_BINARY) pytest-compat-suite/; \
	else \
		echo "$(YELLOW)⚠️  Compatibility tests not found at pytest-compat-suite$(NC)"; \
	fi

# Generate compatibility report for selected suites
compat-report:
	@echo "$(GREEN)Generating compatibility report...$(NC)"
	PYO3_PYTHON=$(PYO3_PYTHON) cargo build -p fastest-cli
		$(PYTHON) scripts/development/compatibility_report.py \
			--fastest-binary $(DEV_FASTEST_BINARY) \
			--json-output $(COMPAT_REPORT) \
			--suite-timeout $(COMPAT_SUITE_TIMEOUT) \
			$(COMPAT_SUITES)
	@echo "$(GREEN)✅ Compatibility report written to $(COMPAT_REPORT)$(NC)"

# Generate report for every compatibility suite without failing on known gaps
compat-report-all:
	@echo "$(GREEN)Generating full compatibility baseline report...$(NC)"
	PYO3_PYTHON=$(PYO3_PYTHON) cargo build -p fastest-cli
		$(PYTHON) scripts/development/compatibility_report.py \
			--fastest-binary $(DEV_FASTEST_BINARY) \
			--json-output $(COMPAT_REPORT_ALL) \
			--suite-timeout $(COMPAT_SUITE_TIMEOUT) \
			--allow-failures
	@echo "$(GREEN)✅ Full compatibility report written to $(COMPAT_REPORT_ALL)$(NC)"

# Smoke real third-party pytest plugin packages against the supported shim surface
plugin-smoke:
	@echo "$(GREEN)Running third-party plugin smoke tests...$(NC)"
	PYO3_PYTHON=$(PYO3_PYTHON) cargo build -p fastest-cli
		$(PYTHON) scripts/development/compatibility_report.py \
			--fastest-binary $(DEV_FASTEST_BINARY) \
			--suite-timeout $(COMPAT_SUITE_TIMEOUT) \
			features/plugins \
			third-party/plugins
	uv run pytest pytest-compat-suite/third-party/plugins -q

# Accepted local verification gate
verify: lint
	PYO3_PYTHON=$(PYO3_PYTHON) cargo test --workspace
	uv run pytest tests -q
	$(MAKE) compat-report
	$(MAKE) plugin-smoke

# Full validation suite
validate: build compare test-compatibility
	@echo "$(GREEN)Running full validation suite...$(NC)"
	@echo "$(GREEN)✅ Full validation complete$(NC)"

# Development cycle: build, test, compare
dev-cycle: build test compare
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
	uv venv .venv
	uv pip install -r requirements-dev.txt
	uv run pre-commit install
	@echo "📁 Creating directories..."
	@mkdir -p comparison_results performance_data
	@echo "$(GREEN)✅ Development environment ready$(NC)"

# Run linters
lint:
	cargo fmt -- --check
	PYO3_PYTHON=$(PYO3_PYTHON) cargo clippy --all-targets --all-features
	uv run --extra dev ruff check python scripts benchmarks examples
	uv run --extra dev black --check python scripts benchmarks examples

# Format code
format:
	cargo fmt
	uv run --extra dev ruff check --fix python scripts benchmarks examples
	uv run --extra dev black python scripts benchmarks examples

# Run coverage
coverage:
	cargo tarpaulin --out Html --all-features
	open tarpaulin-report.html

# Run benchmarks
bench: build
	$(PYTHON) scripts/benchmarks/official.py --quick --output-dir target/benchmark-artifacts/quick

# Run security audit
audit:
	cargo audit

# Create a release build
release:
	PYO3_PYTHON=$(PYO3_PYTHON) cargo build --release --all-features
	strip $(FASTEST_BINARY)
	ls -lh $(FASTEST_BINARY)

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

# Show project status
status:
	@echo "$(GREEN)Fastest Project Status$(NC)"
	@echo "======================"
	@echo "📁 Project: $(PWD)"
	@echo "🦀 Rust: $(shell rustc --version 2>/dev/null || echo 'Not installed')"
	@echo "🐍 Python: $(shell $(PYTHON) --version 2>/dev/null || echo 'Not installed')"
	@echo "🐍 PyO3 Python: $(PYO3_PYTHON)"
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
	@echo ""
	@echo "$(YELLOW)Development Workflow:$(NC)"
	@echo "  make dev-setup        - Set up development environment"
	@echo "  make quick-check      - Quick development status check"
	@echo "  make dev-cycle        - Build, test, and compare"
	@echo ""
	@echo "$(YELLOW)Testing & Validation:$(NC)"
	@echo "  make test-compatibility - Run all compatibility tests"
	@echo "  make compat-report      - Generate compatibility report for selected suites"
	@echo "  make compat-report-all  - Generate full compatibility baseline report"
	@echo "  make plugin-smoke       - Run third-party plugin smoke tests"
	@echo "  make validate           - Full validation suite"
	@echo "  make verify             - Run accepted local verification gates"
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
