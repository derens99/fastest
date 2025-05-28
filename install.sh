#!/usr/bin/env bash
# ⚡ Fastest installer script
# The blazing fast Python test runner with intelligent performance optimization
# 
# Usage: 
#   curl -LsSf https://raw.githubusercontent.com/derens99/fastest/main/install.sh | sh
#   curl -LsSf https://raw.githubusercontent.com/derens99/fastest/main/install.sh | sh -s -- --version v0.1.0
#
# Options:
#   --version VERSION      Install a specific version (default: latest)
#   --install-dir DIR      Installation directory (default: ~/.local/bin)
#   --help                 Show help message

set -e

# Configuration
REPO="derens99/fastest"
INSTALL_DIR="${FASTEST_INSTALL_DIR:-$HOME/.local/bin}"
TEMP_DIR=$(mktemp -d)
VERSION="${FASTEST_VERSION:-}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
info() {
    printf "${BLUE}info${NC}: %s\n" "$1"
}

success() {
    printf "${GREEN}success${NC}: %s\n" "$1"
}

warning() {
    printf "${YELLOW}warning${NC}: %s\n" "$1"
}

error() {
    printf "${RED}error${NC}: %s\n" "$1" >&2
}

# Cleanup on exit
cleanup() {
    rm -rf "$TEMP_DIR"
}
trap cleanup EXIT

# Detect OS and architecture
detect_platform() {
    local os
    local arch
    
    # Detect OS
    case "$(uname -s)" in
        Linux*)     os="unknown-linux-gnu";;
        Darwin*)    os="apple-darwin";;
        CYGWIN*|MINGW*|MSYS*) os="pc-windows-msvc";;
        *)          error "Unsupported OS: $(uname -s)"; exit 1;;
    esac
    
    # Detect architecture
    case "$(uname -m)" in
        x86_64|amd64) arch="x86_64";;
        aarch64|arm64) arch="aarch64";;
        *)          error "Unsupported architecture: $(uname -m)"; exit 1;;
    esac
    
    echo "${arch}-${os}"
}

# Get the latest release version from version manifest
get_latest_version() {
    # Try version manifest first (faster and more reliable)
    local manifest_version=$(curl -s "https://raw.githubusercontent.com/${REPO}/main/.github/version.json" 2>/dev/null | \
        grep '"latest"' | head -1 | \
        sed -E 's/.*"latest"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/')
    
    if [[ -n "$manifest_version" ]]; then
        echo "v${manifest_version}"
    else
        # Fallback to GitHub API
        curl -s "https://api.github.com/repos/${REPO}/releases/latest" | \
            grep '"tag_name"' | \
            sed -E 's/.*"([^"]+)".*/\1/'
    fi
}

# Download and install fastest
install_fastest() {
    local platform="$1"
    local version="${2:-$(get_latest_version)}"
    
    info "Installing fastest ${version} for ${platform}..."
    
    # Convert platform to asset naming convention
    local asset_platform=""
    case "$platform" in
        x86_64-unknown-linux-gnu) asset_platform="linux-amd64";;
        aarch64-unknown-linux-gnu) asset_platform="linux-arm64";;
        x86_64-apple-darwin) asset_platform="darwin-amd64";;
        aarch64-apple-darwin) asset_platform="darwin-arm64";;
        x86_64-pc-windows-msvc) asset_platform="windows-amd64";;
        *) error "Unsupported platform: $platform"; exit 1;;
    esac
    
    # Construct download URL
    local binary_name="fastest"
    local archive_ext="tar.gz"
    if [[ "$platform" == *"windows"* ]]; then
        binary_name="fastest.exe"
        archive_ext="zip"
    fi
    
    local url="https://github.com/${REPO}/releases/download/${version}/fastest-${asset_platform}.${archive_ext}"
    local archive_path="${TEMP_DIR}/fastest.${archive_ext}"
    
    # Download
    info "Downloading from ${url}..."
    if ! curl -LsSf "$url" -o "$archive_path"; then
        error "Failed to download fastest"
        error "URL: ${url}"
        exit 1
    fi
    
    # Extract
    info "Extracting archive..."
    cd "$TEMP_DIR"
    if [[ "$archive_ext" == "zip" ]]; then
        unzip -q "$archive_path"
    else
        tar -xzf "$archive_path"
    fi
    
    # Create install directory if it doesn't exist
    mkdir -p "$INSTALL_DIR"
    
    # Install binary
    info "Installing to ${INSTALL_DIR}/${binary_name}..."
    mv "$binary_name" "${INSTALL_DIR}/"
    chmod +x "${INSTALL_DIR}/${binary_name}"
    
    # Verify installation
    if "${INSTALL_DIR}/${binary_name}" --help >/dev/null 2>&1; then
        success "fastest installed successfully!"
    else
        error "Installation verification failed"
        exit 1
    fi
}

# Check if directory is in PATH
check_path() {
    local dir="$1"
    case ":$PATH:" in
        *":$dir:"*) return 0;;
        *) return 1;;
    esac
}

# Add directory to shell configuration
add_to_path() {
    local dir="$1"
    local shell_config=""
    
    # Detect shell configuration file
    if [[ -n "$BASH_VERSION" ]]; then
        if [[ -f "$HOME/.bashrc" ]]; then
            shell_config="$HOME/.bashrc"
        elif [[ -f "$HOME/.bash_profile" ]]; then
            shell_config="$HOME/.bash_profile"
        fi
    elif [[ -n "$ZSH_VERSION" ]]; then
        shell_config="$HOME/.zshrc"
    elif [[ -f "$HOME/.profile" ]]; then
        shell_config="$HOME/.profile"
    fi
    
    if [[ -n "$shell_config" ]]; then
        echo "" >> "$shell_config"
        echo "# Added by fastest installer" >> "$shell_config"
        echo "export PATH=\"$dir:\$PATH\"" >> "$shell_config"
        info "Added $dir to PATH in $shell_config"
        warning "Please restart your shell or run: source $shell_config"
    else
        warning "Could not detect shell configuration file"
        warning "Please add $dir to your PATH manually"
    fi
}

# Show installation banner
show_banner() {
    echo "⚡ Installing Fastest - The blazing fast Python test runner"
    echo ""
    echo "🎯 Features:"
    echo "  • Intelligent execution strategy (adapts to test suite size)"
    echo "  • In-process execution for small suites (≤20 tests) - 10x faster"
    echo "  • Optimized workers for medium suites (21-100 tests) - 5x faster"
    echo "  • Full parallel execution for large suites (>100 tests) - 3x faster"
    echo "  • Drop-in replacement for pytest"
    echo ""
}

# Verify installation
verify_installation() {
    local binary_path="$1"
    
    info "Verifying installation..."
    
    # Test basic functionality
    if ! "$binary_path" --version >/dev/null 2>&1; then
        error "Installation verification failed: --version command failed"
        return 1
    fi
    
    # Test help command
    if ! "$binary_path" --help >/dev/null 2>&1; then
        error "Installation verification failed: --help command failed"
        return 1
    fi
    
    success "Installation verified successfully!"
    return 0
}

# Show post-installation instructions
show_post_install() {
    echo ""
    success "🎉 Installation complete!"
    echo ""
    echo "🚀 Quick Start:"
    echo "  fastest --help                    # Show help"
    echo "  fastest tests/                    # Run all tests in tests/ directory"
    echo "  fastest test_file.py              # Run specific test file"
    echo "  fastest tests/ --verbose          # Run with detailed output"
    echo ""
    echo "🔥 Performance Tips:"
    echo "  • Small test suites (≤20 tests): Automatically uses in-process execution"
    echo "  • Large test suites (>100 tests): Automatically uses parallel execution"
    echo "  • Use --verbose to see which strategy is selected"
    echo ""
    echo "📚 Learn More:"
    echo "  • GitHub: https://github.com/${REPO}"
    echo "  • Roadmap: https://github.com/${REPO}/blob/main/ROADMAP.md"
    echo "  • Benchmarks: https://github.com/${REPO}/tree/main/benchmarks"
}

# Main installation flow
main() {
    show_banner
    
    # Check for curl
    if ! command -v curl >/dev/null 2>&1; then
        error "curl is required but not installed"
        error "Please install curl and try again"
        exit 1
    fi
    
    # Detect platform
    local platform
    platform=$(detect_platform)
    info "Detected platform: ${platform}"
    
    # Parse arguments
    local version="${VERSION}"
    while [[ $# -gt 0 ]]; do
        case $1 in
            --version)
                version="$2"
                shift 2
                ;;
            --install-dir)
                INSTALL_DIR="$2"
                shift 2
                ;;
            --help)
                echo "⚡ Fastest Installation Script"
                echo ""
                echo "Usage: $0 [OPTIONS]"
                echo ""
                echo "Options:"
                echo "  --version VERSION      Install a specific version (default: latest)"
                echo "  --install-dir DIR      Installation directory (default: ~/.local/bin)"
                echo "  --help                 Show this help message"
                echo ""
                echo "Environment Variables:"
                echo "  FASTEST_INSTALL_DIR    Alternative to --install-dir"
                echo "  FASTEST_VERSION        Alternative to --version"
                echo ""
                echo "Examples:"
                echo "  # Install latest version"
                echo "  curl -LsSf https://raw.githubusercontent.com/${REPO}/main/install.sh | sh"
                echo ""
                echo "  # Install specific version"
                echo "  curl -LsSf https://raw.githubusercontent.com/${REPO}/main/install.sh | sh -s -- --version v0.1.0"
                echo ""
                echo "  # Install to custom directory"
                echo "  curl -LsSf https://raw.githubusercontent.com/${REPO}/main/install.sh | sh -s -- --install-dir /usr/local/bin"
                exit 0
                ;;
            *)
                error "Unknown option: $1"
                echo "Use --help for usage information"
                exit 1
                ;;
        esac
    done
    
    # Install
    install_fastest "$platform" "$version"
    
    # Verify installation
    local binary_name="fastest"
    if [[ "$platform" == *"windows"* ]]; then
        binary_name="fastest.exe"
    fi
    
    if ! verify_installation "${INSTALL_DIR}/${binary_name}"; then
        exit 1
    fi
    
    # Check PATH
    if ! check_path "$INSTALL_DIR"; then
        warning "${INSTALL_DIR} is not in your PATH"
        add_to_path "$INSTALL_DIR"
    fi
    
    show_post_install
}

# Run main function
main "$@" 