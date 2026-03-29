#!/usr/bin/env bash
# Lattice Blockchain — One-Click Installer for Linux / macOS
# Source: https://github.com/dill-lk/Lattice/releases
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/dill-lk/Lattice/main/install.sh | bash
#   bash install.sh [--dir <path>] [--uninstall] [--help]
#
# Environment overrides:
#   LATTICE_BIN_DIR   — install directory (default: $HOME/.local/bin)

set -euo pipefail

# ── Configuration ────────────────────────────────────────────────────────────
GITHUB_REPO="dill-lk/Lattice"
GITHUB_API="https://api.github.com/repos/${GITHUB_REPO}/releases/latest"
BIN_DIR="${LATTICE_BIN_DIR:-$HOME/.local/bin}"
CONFIG_DIR="$HOME/.lattice/config"
DATA_DIR="$HOME/.lattice/data"
BINARIES=(lattice-node lattice-cli lattice-miner)

# ── Colours ──────────────────────────────────────────────────────────────────
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
BLUE='\033[0;34m'; CYAN='\033[0;36m'; NC='\033[0m'

# ── Helpers ───────────────────────────────────────────────────────────────────
print_header() {
    echo -e "${CYAN}"
    echo "╔══════════════════════════════════════════════════════════╗"
    echo "║                                                          ║"
    echo "║          🚀 LATTICE BLOCKCHAIN INSTALLER 🚀             ║"
    echo "║                                                          ║"
    echo "║     Quantum-Resistant Blockchain · GitHub Releases      ║"
    echo "║                                                          ║"
    echo "╚══════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
}

step()    { echo -e "${BLUE}==>${NC} ${GREEN}$*${NC}"; }
info()    { echo -e "${CYAN}ℹ${NC}  $*"; }
success() { echo -e "${GREEN}✓${NC}  $*"; }
warn()    { echo -e "${YELLOW}⚠${NC}  $*"; }
die()     { echo -e "${RED}✗${NC}  $*" >&2; exit 1; }

has() { command -v "$1" >/dev/null 2>&1; }

# ── Parse flags ───────────────────────────────────────────────────────────────
UNINSTALL=false
while [[ $# -gt 0 ]]; do
    case "$1" in
        --dir)    BIN_DIR="$2"; shift 2 ;;
        --uninstall) UNINSTALL=true; shift ;;
        --help|-h)
            echo "Usage: $0 [--dir <install-dir>] [--uninstall]"
            exit 0 ;;
        *) die "Unknown option: $1" ;;
    esac
done

# ── Uninstall ─────────────────────────────────────────────────────────────────
do_uninstall() {
    step "Removing Lattice binaries from $BIN_DIR..."
    local removed=0
    for bin in "${BINARIES[@]}"; do
        if [[ -f "$BIN_DIR/$bin" ]]; then
            rm -f "$BIN_DIR/$bin"
            success "Removed $bin"
            removed=$((removed + 1))
        fi
    done
    [[ $removed -eq 0 ]] && warn "No Lattice binaries found in $BIN_DIR" \
                          || success "Uninstall complete ($removed binaries removed)"
    info "Configuration and data in $HOME/.lattice were left intact."
    exit 0
}

$UNINSTALL && do_uninstall

# ── Detect platform ───────────────────────────────────────────────────────────
detect_platform() {
    step "Detecting platform..."
    local os arch
    os=$(uname -s)
    arch=$(uname -m)

    case "$os" in
        Linux*)
            case "$arch" in
                x86_64)         ASSET_NAME="lattice-linux-amd64.tar.gz" ;;
                aarch64|arm64)  ASSET_NAME="lattice-linux-arm64.tar.gz" ;;
                *) die "Unsupported architecture: $arch" ;;
            esac ;;
        Darwin*)
            case "$arch" in
                arm64)  ASSET_NAME="lattice-macos-arm64.tar.gz"
                        # Fall back to amd64 if arm64 asset absent (set later)
                        ASSET_FALLBACK="lattice-macos-amd64.tar.gz" ;;
                *)      ASSET_NAME="lattice-macos-amd64.tar.gz" ;;
            esac ;;
        *) die "Unsupported OS: $os" ;;
    esac

    success "Platform: $os/$arch → $ASSET_NAME"
}

# ── Fetch latest release ───────────────────────────────────────────────────────
fetch_release() {
    step "Fetching latest release from GitHub..."

    local json
    if has curl; then
        json=$(curl -fsSL "$GITHUB_API")
    elif has wget; then
        json=$(wget -qO- "$GITHUB_API")
    else
        die "curl or wget is required"
    fi

    RELEASE_TAG=$(printf '%s' "$json" | grep '"tag_name"' | head -1 \
        | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')
    [[ -z "$RELEASE_TAG" ]] && \
        die "Could not get release info. Check your internet connection and try again.\n   Releases: https://github.com/${GITHUB_REPO}/releases"

    # Try preferred asset name; fall back to amd64 on Apple Silicon if needed
    DOWNLOAD_URL=$(printf '%s' "$json" | grep '"browser_download_url"' \
        | grep "\"${ASSET_NAME}\"" | head -1 \
        | sed 's/.*"browser_download_url": *"\([^"]*\)".*/\1/')

    if [[ -z "$DOWNLOAD_URL" && -n "${ASSET_FALLBACK:-}" ]]; then
        warn "Native $ASSET_NAME not found — falling back to $ASSET_FALLBACK (Rosetta 2)"
        ASSET_NAME="$ASSET_FALLBACK"
        DOWNLOAD_URL=$(printf '%s' "$json" | grep '"browser_download_url"' \
            | grep "\"${ASSET_NAME}\"" | head -1 \
            | sed 's/.*"browser_download_url": *"\([^"]*\)".*/\1/')
    fi

    [[ -z "$DOWNLOAD_URL" ]] && \
        die "No binary for $ASSET_NAME in release $RELEASE_TAG.\n   See: https://github.com/${GITHUB_REPO}/releases"

    success "Latest release: $RELEASE_TAG"
}

# ── Download and install ───────────────────────────────────────────────────────
download_and_install() {
    step "Downloading $ASSET_NAME..."

    local tmp_dir tmp_archive
    tmp_dir=$(mktemp -d)
    tmp_archive="$tmp_dir/$ASSET_NAME"

    if has curl; then
        curl -fsSL --progress-bar -o "$tmp_archive" "$DOWNLOAD_URL"
    else
        wget -q --show-progress -O "$tmp_archive" "$DOWNLOAD_URL"
    fi
    success "Download complete"

    step "Installing binaries to $BIN_DIR..."
    mkdir -p "$BIN_DIR"
    tar xzf "$tmp_archive" -C "$tmp_dir"

    local installed=0
    for bin in "${BINARIES[@]}"; do
        # Support both flat archives and archives with a top-level subdirectory
        local src
        src=$(find "$tmp_dir" -maxdepth 2 -name "$bin" -type f 2>/dev/null | head -1)
        if [[ -n "$src" ]]; then
            cp "$src" "$BIN_DIR/$bin"
            chmod +x "$BIN_DIR/$bin"
            installed=$((installed + 1))
            success "Installed $bin"
        else
            warn "$bin not found in archive (skipped)"
        fi
    done

    rm -rf "$tmp_dir"
    [[ $installed -eq 0 ]] && die "No binaries were installed. The release asset may be empty."
    success "$installed binaries installed to $BIN_DIR"
}

# ── Add BIN_DIR to PATH ────────────────────────────────────────────────────────
setup_path() {
    step "Setting up PATH..."

    # Already on PATH in this session?
    if [[ ":$PATH:" == *":$BIN_DIR:"* ]]; then
        success "PATH already includes $BIN_DIR"
        return
    fi

    # Determine the right shell RC file using $SHELL (works regardless of how
    # this script was invoked, unlike checking $ZSH_VERSION / $BASH_VERSION).
    local shell_name shell_rc
    shell_name=$(basename "${SHELL:-bash}")
    case "$shell_name" in
        zsh)  shell_rc="$HOME/.zshrc" ;;
        bash) shell_rc="${HOME}/.bashrc" ;;
        fish) shell_rc="$HOME/.config/fish/config.fish" ;;
        *)    shell_rc="$HOME/.profile" ;;
    esac

    # Only append if the directory isn't already mentioned in that file
    if grep -qF "$BIN_DIR" "$shell_rc" 2>/dev/null; then
        success "PATH already configured in $shell_rc"
    else
        printf '\n# Lattice Blockchain\nexport PATH="%s:$PATH"\n' "$BIN_DIR" >> "$shell_rc"
        success "Added $BIN_DIR to PATH in $shell_rc"
        warn "Restart your shell or run: source $shell_rc"
    fi
}

# ── Create default configuration ──────────────────────────────────────────────
create_config() {
    step "Creating default configuration..."
    mkdir -p "$CONFIG_DIR" "$DATA_DIR"

    local cfg="$CONFIG_DIR/node.toml"
    if [[ -f "$cfg" ]]; then
        info "Config already exists at $cfg — skipping"
        return
    fi

    cat > "$cfg" <<EOF
# Lattice Node Configuration
# Generated by the installer — edit as needed.

[network]
listen_addr     = "/ip4/0.0.0.0/tcp/30303"
bootstrap_nodes = []
max_peers       = 50

[consensus]
mining_threads = 0          # 0 = auto-detect CPU count
difficulty     = 1000000

[rpc]
listen_addr = "127.0.0.1:8545"
enabled     = true

[storage]
db_path    = "${DATA_DIR}"
cache_size = 256            # MB
EOF

    success "Config written to $cfg"
}

# ── Completion banner ──────────────────────────────────────────────────────────
print_completion() {
    echo ""
    echo -e "${GREEN}╔══════════════════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║        ✅  LATTICE INSTALLATION COMPLETE  ✅            ║${NC}"
    echo -e "${GREEN}╚══════════════════════════════════════════════════════════╝${NC}"
    echo ""
    echo -e "${CYAN}📍 Installation summary:${NC}"
    echo -e "   ${BLUE}•${NC} Version:  ${RELEASE_TAG}"
    echo -e "   ${BLUE}•${NC} Binaries: ${BIN_DIR}"
    echo -e "   ${BLUE}•${NC} Config:   ${CONFIG_DIR}"
    echo -e "   ${BLUE}•${NC} Data:     ${DATA_DIR}"
    echo ""
    echo -e "${CYAN}🚀 Quick start:${NC}"
    echo ""
    echo -e "   ${YELLOW}1.${NC} Create a wallet:"
    echo -e "      ${BLUE}lattice-cli wallet create${NC}"
    echo ""
    echo -e "   ${YELLOW}2.${NC} Start the node:"
    echo -e "      ${BLUE}lattice-node${NC}"
    echo ""
    echo -e "   ${YELLOW}3.${NC} Check node status:"
    echo -e "      ${BLUE}lattice-cli node status${NC}"
    echo ""
    echo -e "   ${YELLOW}4.${NC} Start mining (replace with your address):"
    echo -e "      ${BLUE}lattice-node --mine --coinbase <your-address>${NC}"
    echo ""
    echo -e "${CYAN}📦 Releases & source:${NC}"
    echo -e "   ${BLUE}https://github.com/${GITHUB_REPO}/releases${NC}"
    echo ""
}

# ── Main ──────────────────────────────────────────────────────────────────────
main() {
    print_header
    detect_platform
    fetch_release
    download_and_install
    setup_path
    create_config
    print_completion
}

main

