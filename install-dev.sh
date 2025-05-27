#!/usr/bin/env bash
# Development installation script for fastest
# This installs fastest from the local build for testing

set -e

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo "🚀 Installing fastest from local build..."

# Check if we're in the fastest directory
if [ ! -f "Cargo.toml" ]; then
    echo -e "${RED}Error: Not in fastest directory. Please run from the project root.${NC}"
    exit 1
fi

# Build the project
echo "📦 Building fastest in release mode..."
cargo build --release --package fastest-cli

# Get the target binary
BINARY="target/release/fastest"
if [ ! -f "$BINARY" ]; then
    echo -e "${RED}Error: Binary not found at $BINARY${NC}"
    exit 1
fi

# Determine installation directory
INSTALL_DIR="${FASTEST_INSTALL_DIR:-${HOME}/.local/bin}"

# Create install directory with proper permissions
if [ ! -d "$INSTALL_DIR" ]; then
    echo "Creating installation directory: $INSTALL_DIR"
    mkdir -p "$INSTALL_DIR" || {
        echo -e "${YELLOW}Warning: Cannot create $INSTALL_DIR, trying alternative...${NC}"
        INSTALL_DIR="${HOME}/bin"
        mkdir -p "$INSTALL_DIR" || {
            echo -e "${RED}Error: Cannot create installation directory${NC}"
            echo "Please create it manually or set FASTEST_INSTALL_DIR"
            exit 1
        }
    }
fi

# Copy binary
echo "📋 Installing to $INSTALL_DIR/fastest..."
cp "$BINARY" "$INSTALL_DIR/fastest"
chmod +x "$INSTALL_DIR/fastest"

# Check if directory is in PATH
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo -e "${YELLOW}Warning: $INSTALL_DIR is not in your PATH${NC}"
    echo ""
    echo "Add this line to your ~/.bashrc or ~/.zshrc:"
    echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
    echo ""
    echo "Then reload your shell:"
    echo "  source ~/.bashrc  # or ~/.zshrc"
else
    echo -e "${GREEN}✓ $INSTALL_DIR is already in PATH${NC}"
fi

# Test installation
echo ""
echo "Testing installation..."
if command -v fastest &> /dev/null; then
    echo -e "${GREEN}✓ fastest is available in PATH${NC}"
    fastest --help | head -1
else
    echo -e "${YELLOW}! fastest not found in PATH, trying direct execution...${NC}"
    "$INSTALL_DIR/fastest" --help | head -1
fi

echo ""
echo -e "${GREEN}✅ Installation complete!${NC}"
echo ""
echo "Usage examples:"
echo "  fastest test_file.py"
echo "  fastest tests/ --optimizer lightning"
echo "  fastest --help"