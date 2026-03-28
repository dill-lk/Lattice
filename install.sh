#!/bin/bash
# Lattice Blockchain - One-Click Installer for Linux/macOS
# This script installs Lattice blockchain and all its dependencies

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration
INSTALL_DIR="${INSTALL_DIR:-$HOME/.lattice}"
BIN_DIR="${BIN_DIR:-$HOME/.local/bin}"
REPO_URL="https://github.com/lattice-chain/lattice"
MIN_RUST_VERSION="1.70.0"

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

# Version comparison
version_ge() {
    [ "$(printf '%s\n' "$1" "$2" | sort -V | head -n1)" = "$2" ]
}

# Detect OS
detect_os() {
    case "$(uname -s)" in
        Linux*)     OS=Linux;;
        Darwin*)    OS=Mac;;
        *)          OS="UNKNOWN";;
    esac
    print_info "Detected OS: $OS"
}

# Check Rust installation
check_rust() {
    print_step "Checking Rust installation..."
    
    if ! command_exists rustc; then
        print_warning "Rust is not installed"
        read -p "Install Rust now? (y/n) " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            install_rust
        else
            print_error "Rust is required. Exiting."
            exit 1
        fi
    else
        RUST_VERSION=$(rustc --version | awk '{print $2}')
        print_success "Rust $RUST_VERSION is installed"
        
        if ! version_ge "$RUST_VERSION" "$MIN_RUST_VERSION"; then
            print_warning "Rust version $RUST_VERSION is below minimum $MIN_RUST_VERSION"
            print_info "Updating Rust..."
            rustup update stable
        fi
    fi
}

# Install Rust
install_rust() {
    print_step "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    print_success "Rust installed successfully"
}

# Check build dependencies
check_dependencies() {
    print_step "Checking build dependencies..."
    
    local missing_deps=()
    
    # Required dependencies
    if ! command_exists git; then
        missing_deps+=("git")
    fi
    
    if ! command_exists pkg-config; then
        missing_deps+=("pkg-config")
    fi
    
    if ! command_exists gcc || ! command_exists clang; then
        missing_deps+=("gcc or clang")
    fi
    
    if [ ${#missing_deps[@]} -eq 0 ]; then
        print_success "All dependencies are installed"
    else
        print_warning "Missing dependencies: ${missing_deps[*]}"
        install_dependencies "${missing_deps[@]}"
    fi
}

# Install dependencies
install_dependencies() {
    print_step "Installing dependencies..."
    
    if command_exists apt-get; then
        # Debian/Ubuntu
        sudo apt-get update
        sudo apt-get install -y build-essential git pkg-config libssl-dev
    elif command_exists dnf; then
        # Fedora
        sudo dnf install -y gcc git pkg-config openssl-devel
    elif command_exists pacman; then
        # Arch Linux
        sudo pacman -Sy --noconfirm base-devel git pkg-config openssl
    elif command_exists brew; then
        # macOS
        brew install git pkg-config openssl
    else
        print_error "Could not detect package manager. Please install dependencies manually:"
        print_info "  - git"
        print_info "  - pkg-config"
        print_info "  - gcc/clang"
        print_info "  - OpenSSL development headers"
        exit 1
    fi
    
    print_success "Dependencies installed"
}

# Clone or update repository
setup_repo() {
    print_step "Setting up Lattice repository..."
    
    if [ -d "$INSTALL_DIR" ]; then
        print_info "Repository exists. Updating..."
        cd "$INSTALL_DIR"
        git pull origin main
    else
        print_info "Cloning repository..."
        git clone "$REPO_URL" "$INSTALL_DIR"
        cd "$INSTALL_DIR"
    fi
    
    print_success "Repository ready"
}

# Build Lattice
build_lattice() {
    print_step "Building Lattice (this may take a while)..."
    
    cd "$INSTALL_DIR"
    
    # Show progress
    cargo build --release --bins 2>&1 | while IFS= read -r line; do
        if [[ $line == *"Compiling"* ]]; then
            echo -e "${CYAN}  → ${line}${NC}"
        fi
    done
    
    print_success "Build completed"
}

# Install binaries
install_binaries() {
    print_step "Installing binaries..."
    
    mkdir -p "$BIN_DIR"
    
    # Copy binaries
    cp "$INSTALL_DIR/target/release/lattice-node" "$BIN_DIR/"
    cp "$INSTALL_DIR/target/release/lattice-cli" "$BIN_DIR/"
    cp "$INSTALL_DIR/target/release/lattice-miner" "$BIN_DIR/"
    
    # Make executable
    chmod +x "$BIN_DIR/lattice-node"
    chmod +x "$BIN_DIR/lattice-cli"
    chmod +x "$BIN_DIR/lattice-miner"
    
    print_success "Binaries installed to $BIN_DIR"
}

# Add to PATH
setup_path() {
    print_step "Setting up PATH..."
    
    # Check if BIN_DIR is in PATH
    if [[ ":$PATH:" == *":$BIN_DIR:"* ]]; then
        print_success "PATH already configured"
        return
    fi
    
    # Detect shell
    if [ -n "$BASH_VERSION" ]; then
        SHELL_RC="$HOME/.bashrc"
    elif [ -n "$ZSH_VERSION" ]; then
        SHELL_RC="$HOME/.zshrc"
    else
        SHELL_RC="$HOME/.profile"
    fi
    
    # Add to PATH
    echo "" >> "$SHELL_RC"
    echo "# Lattice Blockchain" >> "$SHELL_RC"
    echo "export PATH=\"$BIN_DIR:\$PATH\"" >> "$SHELL_RC"
    
    print_success "Added to PATH in $SHELL_RC"
    print_warning "Please restart your shell or run: source $SHELL_RC"
}

# Create configuration
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

# Run tests
run_tests() {
    print_step "Running tests (optional)..."
    
    read -p "Run test suite? (y/n) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        cd "$INSTALL_DIR"
        cargo test --all --release
        print_success "All tests passed!"
    else
        print_info "Skipping tests"
    fi
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
    echo -e "   ${BLUE}•${NC} README: $INSTALL_DIR/README.md"
    echo -e "   ${BLUE}•${NC} Docs:   https://docs.latticechain.io"
    echo ""
    echo -e "${CYAN}💡 Need help?${NC}"
    echo -e "   ${BLUE}•${NC} GitHub:  https://github.com/lattice-chain/lattice"
    echo -e "   ${BLUE}•${NC} Discord: https://discord.gg/lattice"
    echo ""
}

# Main installation flow
main() {
    print_header
    
    # Check requirements
    detect_os
    check_rust
    check_dependencies
    
    # Install
    setup_repo
    build_lattice
    install_binaries
    setup_path
    create_config
    
    # Optional tests
    run_tests
    
    # Done!
    print_completion
}

# Run main function
main "$@"
