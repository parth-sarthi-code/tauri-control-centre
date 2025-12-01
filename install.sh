#!/bin/bash
#
# Control Centre Installation Script
# 
# This script builds and installs the Control Centre application
# for Niri Wayland compositor.
#
# Usage: ./install.sh
#

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log() { echo -e "${GREEN}[✓]${NC} $1"; }
warn() { echo -e "${YELLOW}[!]${NC} $1"; }
error() { echo -e "${RED}[✗]${NC} $1"; }
info() { echo -e "${BLUE}[i]${NC} $1"; }

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "${SCRIPT_DIR}"

echo ""
echo -e "${BLUE}╔════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║     Control Centre Installation Script     ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════╝${NC}"
echo ""

# Check for Rust
if ! command -v cargo &> /dev/null; then
    error "Rust/Cargo not found!"
    info "Install Rust: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi
log "Rust/Cargo found: $(cargo --version)"

# Check for Tauri CLI
if ! command -v cargo-tauri &> /dev/null; then
    warn "Tauri CLI not found, installing..."
    cargo install tauri-cli
fi
log "Tauri CLI available"

# Check system dependencies
info "Checking system dependencies..."

check_dep() {
    if command -v "$1" &> /dev/null; then
        log "$1 found"
        return 0
    else
        warn "$1 not found - $2"
        return 1
    fi
}

MISSING_DEPS=0

check_dep "pactl" "Volume control won't work" || MISSING_DEPS=1
check_dep "brightnessctl" "Brightness control won't work" || MISSING_DEPS=1
check_dep "nmcli" "WiFi toggle won't work" || MISSING_DEPS=1
check_dep "bluetoothctl" "Bluetooth toggle won't work" || MISSING_DEPS=1
check_dep "loginctl" "Suspend won't work" || MISSING_DEPS=1

if [[ $MISSING_DEPS -eq 1 ]]; then
    warn "Some dependencies are missing. The app will still build but some features won't work."
    echo ""
    info "Install on Arch Linux:"
    echo "  sudo pacman -S pulseaudio-utils brightnessctl networkmanager bluez-utils"
    echo ""
    read -p "Continue anyway? [y/N] " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Build
info "Building Control Centre (this may take a few minutes)..."
echo ""

# Tauri v2 uses different syntax - no --release flag, it builds release by default
if cargo tauri build 2>&1 | tee /tmp/control-centre-build.log; then
    log "Build successful!"
else
    error "Build failed! Check /tmp/control-centre-build.log for details"
    exit 1
fi

# Install binary
info "Installing to ~/.local/bin..."
mkdir -p ~/.local/bin

BINARY_PATH=""
if [[ -f "target/release/control-centre" ]]; then
    BINARY_PATH="target/release/control-centre"
elif [[ -f "src-tauri/target/release/control-centre" ]]; then
    BINARY_PATH="src-tauri/target/release/control-centre"
fi

if [[ -n "$BINARY_PATH" ]]; then
    cp "$BINARY_PATH" ~/.local/bin/control-centre
    chmod +x ~/.local/bin/control-centre
    log "Binary installed: ~/.local/bin/control-centre"
else
    error "Could not find built binary!"
    exit 1
fi

# Install toggle script
cp scripts/control-centre ~/.local/bin/control-centre-toggle 2>/dev/null || \
    cp scripts/control-centre ~/.local/bin/ 2>/dev/null || true
chmod +x ~/.local/bin/control-centre* 2>/dev/null || true
log "Toggle script installed"

# Check PATH
if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
    warn "~/.local/bin is not in your PATH"
    info "Add to your shell config: export PATH=\"\$HOME/.local/bin:\$PATH\""
fi

# Print Niri config
echo ""
echo -e "${BLUE}╔════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║           Configuration Required           ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════╝${NC}"
echo ""
info "Add to your Niri config (~/.config/niri/config.kdl):"
echo ""
cat << 'EOF'
window-rule {
    match app-id="control-centre"
    open-floating true
    default-column-width { fixed 420; }
}

// Optional keybinding
binds {
    Mod+C { spawn "~/.local/bin/control-centre"; }
}
EOF
echo ""

info "Add to your Waybar config:"
echo ""
cat << 'EOF'
"custom/control-centre": {
    "format": "󰍜",
    "on-click": "~/.local/bin/control-centre",
    "tooltip": "Control Centre"
}
EOF
echo ""

echo -e "${GREEN}╔════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║         Installation Complete! 🎉          ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════════╝${NC}"
echo ""
log "Run 'control-centre' to test the application"
echo ""
