#!/usr/bin/env bash
# Test version of the Fastest installer script
# This version works with local binary instead of GitHub releases

set -e

# Configuration
INSTALL_DIR="${FASTEST_INSTALL_DIR:-$HOME/.local/bin}"
BINARY_PATH="./target/release/fastest"

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

# Main installation flow
main() {
    echo "🚀 Installing fastest - The blazing fast Python test runner (TEST VERSION)"
    echo ""
    
    # Check if binary exists
    if [[ ! -f "$BINARY_PATH" ]]; then
        error "Binary not found at $BINARY_PATH"
        error "Please run 'cargo build --release' first"
        exit 1
    fi
    
    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --install-dir)
                INSTALL_DIR="$2"
                shift 2
                ;;
            --help)
                echo "Usage: $0 [OPTIONS]"
                echo ""
                echo "Options:"
                echo "  --install-dir DIR      Installation directory (default: ~/.local/bin)"
                echo "  --help                 Show this help message"
                exit 0
                ;;
            *)
                error "Unknown option: $1"
                exit 1
                ;;
        esac
    done
    
    # Create install directory if it doesn't exist
    info "Creating directory: $INSTALL_DIR"
    mkdir -p "$INSTALL_DIR"
    
    # Install binary
    info "Installing to ${INSTALL_DIR}/fastest..."
    cp "$BINARY_PATH" "${INSTALL_DIR}/fastest"
    chmod +x "${INSTALL_DIR}/fastest"
    
    # Verify installation
    if "${INSTALL_DIR}/fastest" --help >/dev/null 2>&1; then
        success "fastest installed successfully!"
    else
        error "Installation verification failed"
        exit 1
    fi
    
    # Check PATH
    if ! check_path "$INSTALL_DIR"; then
        warning "${INSTALL_DIR} is not in your PATH"
        add_to_path "$INSTALL_DIR"
    fi
    
    echo ""
    success "Installation complete! 🎉"
    echo ""
    echo "To get started, try:"
    echo "  fastest --help"
    echo "  fastest tests/"
    echo ""
}

# Run main function
main "$@" 