# Control Centre for Niri Wayland

A beautiful, macOS-style Control Centre overlay application built with Tauri (Rust) and vanilla HTML/CSS/JS, designed specifically for the Niri Wayland compositor.

![Control Centre Preview](docs/preview.png)

## Features

- 🎨 **macOS-inspired Design** - Frosted glass effect, smooth animations, modern UI
- 🔊 **Volume Control** - Slider with mute toggle (PulseAudio/PipeWire)
- 🔆 **Brightness Control** - Screen brightness adjustment (brightnessctl)
- 📶 **WiFi Toggle** - Enable/disable wireless networking (NetworkManager)
- 🔵 **Bluetooth Toggle** - Enable/disable Bluetooth (bluetoothctl)
- 🌙 **Night Light Toggle** - Color temperature adjustment (GNOME/gammastep)
- 💤 **Suspend** - Quick system suspend (loginctl)
- 🪟 **Niri Optimized** - Floating overlay behavior, proper layer handling
- ⚡ **Single Instance** - Toggle visibility with repeated clicks
- ⌨️ **Keyboard Support** - ESC to close, arrow keys for sliders

## Requirements

### System Dependencies

```bash
# Arch Linux
sudo pacman -S \
    pulseaudio-utils \      # or pipewire-pulse
    brightnessctl \
    networkmanager \
    bluez-utils \
    webkit2gtk \
    gtk3

# Optional for Night Light
sudo pacman -S gammastep     # or wlsunset for Wayland
```

### Build Dependencies

```bash
# Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default stable

# Tauri CLI
cargo install tauri-cli
```

## Installation

### Building from Source

```bash
# Clone or navigate to the project
cd ~/.config/waybar/control_centre/tauri-control-centre

# Build release binary
cargo tauri build --release

# The binary will be at:
# target/release/control-centre

# Install to local bin
mkdir -p ~/.local/bin
cp target/release/control-centre ~/.local/bin/
cp scripts/control-centre ~/.local/bin/control-centre-toggle
chmod +x ~/.local/bin/control-centre-toggle
```

### Quick Install Script

```bash
#!/bin/bash
cd ~/.config/waybar/control_centre/tauri-control-centre
cargo tauri build --release
mkdir -p ~/.local/bin
cp target/release/control-centre ~/.local/bin/
cp scripts/control-centre ~/.local/bin/
chmod +x ~/.local/bin/control-centre
```

## Niri Configuration

Add these window rules to your Niri config (`~/.config/niri/config.kdl`):

```kdl
window-rule {
    match app-id="control-centre"
    match app-id="Control Centre"
    
    open-floating true
    default-column-width { fixed 420; }
}

// Optional: Add keybinding
binds {
    Mod+C { spawn "~/.local/bin/control-centre"; }
}
```

## Waybar Integration

Add to your Waybar config (`~/.config/waybar/config`):

```json
{
    "modules-right": ["custom/control-centre", ...],
    
    "custom/control-centre": {
        "format": "󰍜",
        "on-click": "~/.local/bin/control-centre",
        "on-click-right": "~/.local/bin/control-centre kill",
        "tooltip": true,
        "tooltip-format": "Control Centre"
    }
}
```

Add styling to your Waybar CSS (`~/.config/waybar/style.css`):

```css
#custom-control-centre {
    padding: 0 12px;
    margin: 4px 2px;
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.1);
    color: #ffffff;
    font-size: 16px;
}

#custom-control-centre:hover {
    background: rgba(255, 255, 255, 0.2);
}
```

## Usage

### Toggle Script Commands

```bash
# Toggle visibility (default)
~/.local/bin/control-centre

# Explicit commands
~/.local/bin/control-centre show
~/.local/bin/control-centre hide
~/.local/bin/control-centre kill
~/.local/bin/control-centre restart
~/.local/bin/control-centre status
```

### Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `ESC` | Close Control Centre |
| `↑/↓` | Adjust focused slider by 5% |
| `Tab` | Navigate between controls |

## Development

### Running in Development Mode

```bash
cd ~/.config/waybar/control_centre/tauri-control-centre
cargo tauri dev
```

### Running Tests

```bash
# Rust tests
cargo test

# Integration tests (requires system services)
cargo test -- --ignored

# Frontend tests (requires Jest)
cd ui && npm test
```

### Project Structure

```
tauri-control-centre/
├── Cargo.toml              # Rust dependencies
├── tauri.conf.json         # Tauri configuration
├── build.rs                # Build script
├── src/
│   ├── main.rs             # Application entry point
│   ├── commands.rs         # Tauri commands (system control)
│   ├── error.rs            # Error types
│   └── state.rs            # State management
├── ui/
│   ├── index.html          # Main HTML
│   ├── styles.css          # Styling (CSS)
│   └── app.js              # Frontend logic
├── scripts/
│   └── control-centre      # Toggle script for Waybar
├── niri/
│   └── window-rules.kdl    # Niri window rules
├── waybar/
│   └── module.json         # Waybar module config
└── tests/
    └── integration_tests.rs # Rust integration tests
```

## Troubleshooting

### Window doesn't appear

1. Check if the binary exists: `ls -la ~/.local/bin/control-centre`
2. Run manually to see errors: `~/.local/bin/control-centre`
3. Check Niri window rules are applied

### Blur effect not working

Blur requires compositor support. On Niri, ensure blur is enabled in your config:

```kdl
// In niri config
prefer-no-csd
```

If blur still doesn't work, the app will fall back to a solid semi-transparent background.

### Volume/Brightness controls not working

Check that the required tools are installed and working:

```bash
# Test volume
pactl get-sink-volume @DEFAULT_SINK@
pactl set-sink-volume @DEFAULT_SINK@ 50%

# Test brightness
brightnessctl info
brightnessctl set 50%
```

### WiFi/Bluetooth toggles not working

```bash
# Test WiFi
nmcli radio wifi

# Test Bluetooth
bluetoothctl show
```

### Night Light not working

The app tries GNOME settings first, then falls back to gammastep/wlsunset:

```bash
# Install gammastep for Wayland
sudo pacman -S gammastep

# Or wlsunset
sudo pacman -S wlsunset
```

### Single instance not working

Check for stale PID/socket files:

```bash
rm -f /tmp/control-centre.sock
rm -f ${XDG_RUNTIME_DIR}/control-centre.pid
```

## Architecture

### Backend (Rust/Tauri)

- **Commands**: Each system control (volume, brightness, etc.) is a Tauri command
- **Async Execution**: Uses `tokio::process::Command` for non-blocking CLI calls
- **State Caching**: Reduces redundant system calls with time-based cache
- **Single Instance**: Uses Unix sockets for IPC between instances
- **Security**: Direct process execution (no shell), input validation

### Frontend (HTML/CSS/JS)

- **No Framework**: Vanilla JS for minimal bundle size
- **CSS Variables**: Easy theming support
- **Debounced Sliders**: Prevents excessive system calls
- **Optimistic Updates**: UI updates immediately, reverts on error
- **Keyboard Support**: Full accessibility

### IPC Flow

```
[Frontend] --invoke--> [Tauri Command] --spawn--> [System CLI]
    ↑                        ↓
    └─────── Result ─────────┘
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests: `cargo test`
5. Submit a pull request

## License

MIT License - See LICENSE file for details.

## Acknowledgments

- [Tauri](https://tauri.app/) - Cross-platform app framework
- [Niri](https://github.com/YaLTeR/niri) - Scrollable-tiling Wayland compositor
- [Waybar](https://github.com/Alexays/Waybar) - Wayland bar
- Inspired by macOS Control Centre
