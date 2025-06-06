#!/bin/bash
# Install all development tools needed for Fastest

set -e

echo "🔧 Installing Rust development tools for Fastest..."
echo "================================================"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

# Check if rustup is installed
if ! command -v rustup >/dev/null 2>&1; then
    echo -e "${RED}❌ Error: rustup is not installed${NC}"
    echo "Please install Rust first: https://rustup.rs/"
    exit 1
fi

echo "📦 Installing required Rust components..."

# Install stable toolchain
echo "  - Installing stable Rust toolchain..."
rustup install stable
rustup default stable

# Install required components
echo "  - Installing rustfmt..."
rustup component add rustfmt

echo "  - Installing clippy..."
rustup component add clippy

# Install optional but useful tools
echo ""
echo "📦 Installing optional development tools..."

# Function to install cargo tool if not present
install_cargo_tool() {
    local tool=$1
    local package=${2:-$1}
    
    if ! cargo install --list | grep -q "^$tool "; then
        echo "  - Installing $tool..."
        cargo install $package
    else
        echo -e "  ${GREEN}✓${NC} $tool already installed"
    fi
}

# Install useful development tools
install_cargo_tool "cargo-watch"     # For watching file changes
install_cargo_tool "cargo-edit"      # For managing dependencies
install_cargo_tool "cargo-outdated"  # For checking outdated deps
install_cargo_tool "cargo-audit"     # For security audits
install_cargo_tool "cargo-machete"   # For finding unused dependencies

echo ""
echo -e "${GREEN}✅ All development tools installed!${NC}"
echo ""
echo "🚀 You're now ready to develop Fastest!"
echo ""
echo "Quick commands:"
echo "  cargo fmt           - Format code"
echo "  cargo clippy        - Run linter"
echo "  cargo test          - Run tests"
echo "  cargo build --release - Build optimized binary"
echo "  ./scripts/pre-push-check.sh - Run full validation"
echo ""
echo "💡 Tip: Enable Git hooks with: ./scripts/setup-hooks.sh"