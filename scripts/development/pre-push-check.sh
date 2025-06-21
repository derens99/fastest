#!/bin/bash
# Pre-push validation script to catch CI issues before pushing to GitHub
# This script runs the same checks as the CI pipeline to ensure builds will pass

set -e

echo "🔍 Running pre-push validation checks..."
echo "============================================"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if we're in the project root
if [ ! -f "Cargo.toml" ]; then
    echo -e "${RED}❌ Error: Must run from project root directory${NC}"
    exit 1
fi

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Check required tools
echo "📦 Checking required tools..."
MISSING_TOOLS=()
for tool in cargo rustc; do
    if ! command_exists $tool; then
        MISSING_TOOLS+=($tool)
    fi
done

if [ ${#MISSING_TOOLS[@]} -ne 0 ]; then
    echo -e "${RED}❌ Error: Required tools not installed: ${MISSING_TOOLS[*]}${NC}"
    echo "Please install Rust: https://rustup.rs/"
    exit 1
fi

# Check optional but recommended tools
OPTIONAL_TOOLS=()
if ! command_exists cargo-clippy && ! cargo clippy --version >/dev/null 2>&1; then
    OPTIONAL_TOOLS+=("clippy")
fi
if ! command_exists rustfmt && ! cargo fmt --version >/dev/null 2>&1; then
    OPTIONAL_TOOLS+=("rustfmt")
fi

if [ ${#OPTIONAL_TOOLS[@]} -ne 0 ]; then
    echo -e "${YELLOW}⚠️  Warning: Recommended tools not installed: ${OPTIONAL_TOOLS[*]}${NC}"
    echo ""
    echo "To install missing components:"
    for tool in "${OPTIONAL_TOOLS[@]}"; do
        echo "  rustup component add $tool"
    done
    echo ""
    echo "Installing them now..."
    for tool in "${OPTIONAL_TOOLS[@]}"; do
        rustup component add $tool || echo -e "${RED}Failed to install $tool${NC}"
    done
fi

echo -e "${GREEN}✅ All required tools found${NC}"

# 1. Check formatting
echo ""
echo "🎨 Checking code formatting..."
if cargo fmt --version >/dev/null 2>&1; then
    if ! cargo fmt -- --check; then
        echo -e "${RED}❌ Formatting issues found. Run 'cargo fmt' to fix.${NC}"
        exit 1
    fi
    echo -e "${GREEN}✅ Code formatting OK${NC}"
else
    echo -e "${YELLOW}⚠️  Skipping format check (rustfmt not available)${NC}"
fi

# 2. Run clippy with CI configuration
echo ""
echo "📎 Running clippy with strict settings..."
if cargo clippy --version >/dev/null 2>&1; then
    if ! cargo clippy --all-targets --all-features -- -D warnings; then
        echo -e "${RED}❌ Clippy warnings found. Fix them before pushing.${NC}"
        exit 1
    fi
    echo -e "${GREEN}✅ Clippy checks passed${NC}"
else
    echo -e "${YELLOW}⚠️  Skipping clippy check (clippy not available)${NC}"
    echo -e "${YELLOW}    Note: CI will still run clippy and may fail!${NC}"
fi

# 3. Build in release mode (catches different errors than debug)
echo ""
echo "🔨 Building in release mode..."
if ! cargo build --release; then
    echo -e "${RED}❌ Release build failed${NC}"
    exit 1
fi
echo -e "${GREEN}✅ Release build successful${NC}"

# 4. Run tests
echo ""
echo "🧪 Running tests..."
if ! cargo test --all-features; then
    echo -e "${RED}❌ Tests failed${NC}"
    exit 1
fi
echo -e "${GREEN}✅ All tests passed${NC}"

# 5. Check for common CI issues
echo ""
echo "🔍 Checking for common CI issues..."

# Check for unused dependencies
echo "  - Checking for unused dependencies..."
if cargo machete --help >/dev/null 2>&1; then
    cargo machete || echo -e "${YELLOW}⚠️  Warning: Some unused dependencies found${NC}"
else
    echo -e "${YELLOW}⚠️  Skipping unused dependency check (cargo-machete not installed)${NC}"
fi

# Check for security advisories
echo "  - Checking for security advisories..."
if cargo audit --help >/dev/null 2>&1; then
    cargo audit || echo -e "${YELLOW}⚠️  Warning: Security advisories found${NC}"
else
    echo -e "${YELLOW}⚠️  Skipping security audit (cargo-audit not installed)${NC}"
fi

# 6. Architecture-specific checks
echo ""
echo "🏗️  Checking architecture-specific code..."

# Check if we have arch-specific imports that might fail on different architectures
if grep -r "use std::arch::" crates/ --include="*.rs" | grep -v "#\[allow(unused_imports)\]" | grep -v "#\[cfg(" > /dev/null; then
    echo -e "${YELLOW}⚠️  Warning: Found architecture-specific imports without proper cfg guards${NC}"
    echo "    Consider adding #[cfg(target_arch = ...)] attributes"
fi

# 7. Check Cargo.lock is up to date
echo ""
echo "🔒 Checking Cargo.lock..."
if ! cargo check --locked 2>/dev/null; then
    echo -e "${YELLOW}⚠️  Warning: Cargo.lock might be out of date. Run 'cargo update' if needed.${NC}"
fi

# 8. Quick fastest functionality test
echo ""
echo "⚡ Testing fastest binary..."
if [ -f "./target/release/fastest" ]; then
    if ./target/release/fastest --version >/dev/null 2>&1; then
        echo -e "${GREEN}✅ Fastest binary works${NC}"
    else
        echo -e "${RED}❌ Fastest binary failed to run${NC}"
        exit 1
    fi
else
    echo -e "${YELLOW}⚠️  Release binary not found, skipping functional test${NC}"
fi

# Summary
echo ""
echo "============================================"
echo -e "${GREEN}✅ All pre-push checks passed!${NC}"
echo ""
echo "💡 Tips for avoiding CI failures:"
echo "   - Always use conditional compilation for arch-specific code"
echo "   - Add #[allow(dead_code)] or remove unused code"
echo "   - Use #[allow(unused_imports)] for conditional imports"
echo "   - Test on both debug and release builds"
echo "   - Keep dependencies up to date with 'cargo update'"
echo ""
echo "🚀 Ready to push to GitHub!"