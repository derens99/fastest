#!/usr/bin/env bash
# Fastest installer script
# Inspired by Astral's UV installer

set -euo pipefail

# Configuration
REPO="derens99/fastest"
BASE_URL="https://github.com/${REPO}/releases"
INSTALL_DIR="${FASTEST_INSTALL_DIR:-$HOME/.fastest}"
BIN_DIR="${INSTALL_DIR}/bin"
EXECUTABLE_NAME="fastest"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Helper functions
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

# Detect OS and architecture
detect_platform() {
    local os
    local arch
    
    # Detect OS
    case "$(uname -s)" in
        Linux*)     os="linux";;
        Darwin*)    os="macos";;
        CYGWIN*|MINGW*|MSYS*) os="windows";;
        *)          error "Unsupported operating system"; exit 1;;
    esac
    
    # Detect architecture
    case "$(uname -m)" in
        x86_64|amd64)  arch="x86_64";;
        aarch64|arm64) arch="aarch64";;
        *)             error "Unsupported architecture"; exit 1;;
    esac
    
    echo "${os}-${arch}"
}

# Download the latest release
download_release() {
    local platform=$1
    local temp_file=$(mktemp)
    
    # Get latest release URL
    info "Finding latest release..."
    local latest_url="${BASE_URL}/latest"
    local download_url="${BASE_URL}/download/latest/${EXECUTABLE_NAME}-${platform}.tar.gz"
    
    # Download the file
    info "Downloading Fastest for ${platform}..."
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL -o "$temp_file" "$download_url"
    elif command -v wget >/dev/null 2>&1; then
        wget -qO "$temp_file" "$download_url"
    else
        error "Neither curl nor wget found. Please install one of them."
        exit 1
    fi
    
    echo "$temp_file"
}

# Install the binary
install_binary() {
    local archive_file=$1
    
    # Create installation directory
    info "Installing to ${INSTALL_DIR}..."
    mkdir -p "$BIN_DIR"
    
    # Extract the archive
    tar -xzf "$archive_file" -C "$BIN_DIR"
    
    # Make executable
    chmod +x "${BIN_DIR}/${EXECUTABLE_NAME}"
    
    # Clean up
    rm -f "$archive_file"
}

# Setup shell integration
setup_shell() {
    local shell_name=$(basename "$SHELL")
    local added_path=false
    
    # Function to add path to shell config
    add_to_path() {
        local config_file=$1
        local path_line="export PATH=\"${BIN_DIR}:\$PATH\""
        
        if [ -f "$config_file" ]; then
            if ! grep -q "${BIN_DIR}" "$config_file"; then
                echo "" >> "$config_file"
                echo "# Added by Fastest installer" >> "$config_file"
                echo "$path_line" >> "$config_file"
                added_path=true
                info "Added ${BIN_DIR} to PATH in ${config_file}"
            else
                info "PATH already contains ${BIN_DIR}"
            fi
        fi
    }
    
    case "$shell_name" in
        bash)
            add_to_path "$HOME/.bashrc"
            [ -f "$HOME/.bash_profile" ] && add_to_path "$HOME/.bash_profile"
            ;;
        zsh)
            add_to_path "$HOME/.zshrc"
            ;;
        fish)
            local fish_config="$HOME/.config/fish/config.fish"
            if [ -f "$fish_config" ]; then
                if ! grep -q "${BIN_DIR}" "$fish_config"; then
                    echo "" >> "$fish_config"
                    echo "# Added by Fastest installer" >> "$fish_config"
                    echo "set -gx PATH ${BIN_DIR} \$PATH" >> "$fish_config"
                    added_path=true
                    info "Added ${BIN_DIR} to PATH in ${fish_config}"
                fi
            fi
            ;;
        *)
            warning "Unknown shell: $shell_name. Please manually add ${BIN_DIR} to your PATH."
            ;;
    esac
    
    if [ "$added_path" = true ]; then
        info "Run 'source ~/.$shell_name*rc' or start a new shell to use fastest"
    fi
}

# Verify installation
verify_installation() {
    if "${BIN_DIR}/${EXECUTABLE_NAME}" --version >/dev/null 2>&1; then
        local version=$("${BIN_DIR}/${EXECUTABLE_NAME}" --version | cut -d' ' -f2)
        success "Fastest ${version} installed successfully!"
    else
        error "Installation verification failed"
        exit 1
    fi
}

# Main installation flow
main() {
    echo "🚀 Installing Fastest - The blazing fast Python test runner"
    echo ""
    
    # Check if already installed
    if [ -f "${BIN_DIR}/${EXECUTABLE_NAME}" ]; then
        warning "Fastest is already installed at ${BIN_DIR}/${EXECUTABLE_NAME}"
        read -p "Do you want to reinstall? [y/N] " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            exit 0
        fi
    fi
    
    # Detect platform
    local platform=$(detect_platform)
    info "Detected platform: ${platform}"
    
    # Download release
    local temp_archive=$(download_release "$platform")
    
    # Install binary
    install_binary "$temp_archive"
    
    # Setup shell
    setup_shell
    
    # Verify installation
    verify_installation
    
    echo ""
    echo "📚 Get started with:"
    echo "    ${EXECUTABLE_NAME} --help"
    echo ""
    echo "📖 Documentation: https://github.com/${REPO}"
    echo "🐛 Report issues: https://github.com/${REPO}/issues"
}

# Run main function
main "$@" 