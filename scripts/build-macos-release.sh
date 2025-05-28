#!/bin/bash
# Build macOS release binaries locally

set -e

VERSION="${1:-0.2.0}"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo "Building Fastest v$VERSION for macOS..."
echo "Repository root: $REPO_ROOT"

cd "$REPO_ROOT"

# Detect current architecture
ARCH=$(uname -m)
if [[ "$ARCH" == "arm64" ]]; then
    TARGET="aarch64-apple-darwin"
    ASSET_NAME="fastest-darwin-arm64"
else
    TARGET="x86_64-apple-darwin"
    ASSET_NAME="fastest-darwin-amd64"
fi

echo "Building for $TARGET..."

# Build the binary
cargo build --release --target $TARGET

# Create the archive
echo "Creating archive..."
cd target/$TARGET/release
tar -czf ../../../$ASSET_NAME.tar.gz fastest
cd ../../..

# Generate checksum
echo "Generating checksum..."
shasum -a 256 $ASSET_NAME.tar.gz > $ASSET_NAME.tar.gz.sha256

echo ""
echo "✅ Build complete!"
echo ""
echo "Files created:"
echo "  - $ASSET_NAME.tar.gz"
echo "  - $ASSET_NAME.tar.gz.sha256"
echo ""
echo "To upload to GitHub release:"
echo "1. Go to https://github.com/derens99/fastest/releases/tag/v$VERSION"
echo "2. Click 'Edit release'"
echo "3. Upload both files"
echo ""
echo "To test the binary locally:"
echo "tar -xzf $ASSET_NAME.tar.gz"
echo "./fastest --version"