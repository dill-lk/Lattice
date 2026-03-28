#!/bin/bash
# Lattice Blockchain - One-Click Installer for Linux/macOS
# Downloads pre-built binaries from GitHub Releases

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration
BIN_DIR="${BIN_DIR:-$HOME/.local/bin}"
GITHUB_REPO="dill-lk/Lattice"
GITHUB_API="https://api.github.com/repos/${GITHUB_REPO}/releases/latest"

# Helper functions
print_header() {
    echo -e "${CYAN}"
    echo "╔══════════════════════════════════════════════════════════╗"
    echo "║                                                          ║"
    echo "║          🚀 LATTICE BLOCKCHAIN INSTALLER 🚀             ║"
    echo "║                                                          ║"
    echo "║     Quantum-Resistant Blockchain with Advanced Features ║"
    echo "║                                                          ║"
    echo "╚══════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
}

print_step() {
    echo -e "${BLUE}==>${NC} ${GREEN}$1${NC}"
}

print_info() {
    echo -e "${CYAN}ℹ${NC}  $1"
}

print_success() {
    echo -e "${GREEN}✓${NC}  $1"
}

print_warning() {
    echo -e "${YELLOW}⚠${NC}  $1"
}

print_error() {
    echo -e "${RED}✗${NC}  $1"
}

# Check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Detect OS and architecture, set ASSET_NAME
detect_platform() {
    print_step "Detecting platform..."

    local os arch

    os=$(uname -s)
    arch=$(uname -m)

    case "$os" in
        Linux*)
            case "$arch" in
                x86_64) ASSET_NAME="lattice-linux-amd64.tar.gz" ;;
                *) print_error "Unsupported architecture: $arch (only x86_64 is supported)"; exit 1 ;;
            esac
            ;;
        Darwin*)
            # x86_64 binary runs on Apple Silicon via Rosetta 2
            ASSET_NAME="lattice-macos-amd64.tar.gz"
            if [ "$arch" = "arm64" ]; then
                print_warning "Apple Silicon detected — using x86_64 binary via Rosetta 2"
            fi
            ;;
        *)
            print_error "Unsupported OS: $os"; exit 1 ;;
    esac

    print_success "Platform: $os $arch → $ASSET_NAME"
}

# Fetch latest release tag and download URL
fetch_release() {
    print_step "Fetching latest release from GitHub..."

    local release_json

    if command_exists curl; then
        release_json=$(curl -fsSL "$GITHUB_API")
    elif command_exists wget; then
        release_json=$(wget -qO- "$GITHUB_API")
    else
        print_error "curl or wget is required to download Lattice"; exit 1
    fi

    RELEASE_TAG=$(echo "$release_json" | grep '"tag_name"' | head -1 \
        | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')

    DOWNLOAD_URL=$(echo "$release_json" | grep '"browser_download_url"' \
        | grep "$ASSET_NAME" | head -1 \
        | sed 's/.*"browser_download_url": *"\([^"]*\)".*/\1/')

    if [ -z "$RELEASE_TAG" ]; then
        print_error "Could not determine latest release tag. Check your internet connection."
        exit 1
    fi

    if [ -z "$DOWNLOAD_URL" ]; then
        print_error "No pre-built binary found for $ASSET_NAME in release $RELEASE_TAG."
        print_info "See https://github.com/${GITHUB_REPO}/releases for available assets."
        exit 1
    fi

    print_success "Latest release: $RELEASE_TAG"
}

# Download archive and install binaries
download_and_install() {
    print_step "Downloading $ASSET_NAME..."

    local tmp_dir tmp_archive
    tmp_dir=$(mktemp -d)
    tmp_archive="$tmp_dir/$ASSET_NAME"

    if command_exists curl; then
        curl -fsSL --progress-bar -o "$tmp_archive" "$DOWNLOAD_URL"
    else
        wget -q --show-progress -O "$tmp_archive" "$DOWNLOAD_URL"
    fi

    print_success "Download complete"
    print_step "Installing binaries to $BIN_DIR..."

    mkdir -p "$BIN_DIR"
    tar xzf "$tmp_archive" -C "$tmp_dir"

    local installed=0
    for bin in lattice-node lattice-cli lattice-miner; do
        if [ -f "$tmp_dir/$bin" ]; then
            cp "$tmp_dir/$bin" "$BIN_DIR/$bin"
            chmod +x "$BIN_DIR/$bin"
            installed=$((installed + 1))
        fi
    done

    rm -rf "$tmp_dir"

    if [ "$installed" -eq 0 ]; then
        print_error "No binaries were found in the archive. The release may be incomplete."
        exit 1
    fi

    print_success "Installed $installed binaries to $BIN_DIR"
}

# Add BIN_DIR to PATH if needed
setup_path() {
    print_step "Setting up PATH..."

    if [[ ":$PATH:" == *":$BIN_DIR:"* ]]; then
        print_success "PATH already configured"
        return
    fi

    local shell_rc
    if [ -n "$ZSH_VERSION" ]; then
        shell_rc="$HOME/.zshrc"
    elif [ -n "$BASH_VERSION" ]; then
        shell_rc="$HOME/.bashrc"
    else
        shell_rc="$HOME/.profile"
    fi

    printf '\n# Lattice Blockchain\nexport PATH="%s:$PATH"\n' "$BIN_DIR" >> "$shell_rc"

    print_success "Added $BIN_DIR to PATH in $shell_rc"
    print_warning "Restart your shell or run: source $shell_rc"
}

# Create default configuration
create_config() {
    print_step "Creating default configuration..."

    mkdir -p "$HOME/.lattice/config"

    cat > "$HOME/.lattice/config/node.toml" <<EOF
# Lattice Node Configuration

[network]
# P2P listen address
listen_addr = "/ip4/0.0.0.0/tcp/30333"
# Bootstrap nodes (empty for standalone)
bootstrap_nodes = []
# Maximum number of peers
max_peers = 50

[consensus]
# Mining threads (0 = auto-detect)
mining_threads = 0
# Mining difficulty (auto-adjust)
difficulty = 1000000

[rpc]
# RPC listen address
listen_addr = "127.0.0.1:8545"
# Enable RPC server
enabled = true

[storage]
# Database path
db_path = "$HOME/.lattice/data"
# Cache size in MB
cache_size = 256
EOF

    print_success "Configuration created at $HOME/.lattice/config/node.toml"
}

# Print completion message
print_completion() {
    echo ""
    echo -e "${GREEN}╔══════════════════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║                                                          ║${NC}"
    echo -e "${GREEN}║        ✅ LATTICE INSTALLATION COMPLETE! ✅             ║${NC}"
    echo -e "${GREEN}║                                                          ║${NC}"
    echo -e "${GREEN}╚══════════════════════════════════════════════════════════╝${NC}"
    echo ""
    echo -e "${CYAN}📍 Installation Details:${NC}"
    echo -e "   ${BLUE}•${NC} Version:  $RELEASE_TAG"
    echo -e "   ${BLUE}•${NC} Binaries: $BIN_DIR"
    echo -e "   ${BLUE}•${NC} Config:   $HOME/.lattice/config"
    echo -e "   ${BLUE}•${NC} Data:     $HOME/.lattice/data"
    echo ""
    echo -e "${CYAN}🚀 Quick Start:${NC}"
    echo ""
    echo -e "   ${YELLOW}1.${NC} Create a wallet:"
    echo -e "      ${BLUE}lattice-cli wallet create${NC}"
    echo ""
    echo -e "   ${YELLOW}2.${NC} Start a node:"
    echo -e "      ${BLUE}lattice-node --config $HOME/.lattice/config/node.toml${NC}"
    echo ""
    echo -e "   ${YELLOW}3.${NC} Check node status:"
    echo -e "      ${BLUE}lattice-cli node status${NC}"
    echo ""
    echo -e "   ${YELLOW}4.${NC} Start mining:"
    echo -e "      ${BLUE}lattice-miner --threads 4${NC}"
    echo ""
    echo -e "${CYAN}📚 Documentation:${NC}"
    echo -e "   ${BLUE}•${NC} Docs:   https://github.com/${GITHUB_REPO}"
    echo ""
    echo -e "${CYAN}💡 Need help?${NC}"
    echo -e "   ${BLUE}•${NC} GitHub: https://github.com/${GITHUB_REPO}"
    echo ""
}

# Main installation flow
main() {
    print_header
    detect_platform
    fetch_release
    download_and_install
    setup_path
    create_config
    print_completion
}

main "$@"

