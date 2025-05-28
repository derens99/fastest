#!/bin/bash
# Test installation script locally

set -e

# Create a test installation directory
TEST_DIR="/tmp/fastest-test-install"
rm -rf "$TEST_DIR"
mkdir -p "$TEST_DIR/bin"

# Copy local binary archive
cp fastest-darwin-arm64.tar.gz "$TEST_DIR/"

# Extract and install
cd "$TEST_DIR"
tar -xzf fastest-darwin-arm64.tar.gz
mv fastest bin/

# Test the installation
echo "Testing installation..."
"$TEST_DIR/bin/fastest" version

echo ""
echo "✅ Installation test successful!"
echo ""
echo "Binary installed to: $TEST_DIR/bin/fastest"
echo ""
echo "To use it, run:"
echo "  $TEST_DIR/bin/fastest tests/"
echo ""
echo "Or add to PATH:"
echo "  export PATH=\"$TEST_DIR/bin:\$PATH\""