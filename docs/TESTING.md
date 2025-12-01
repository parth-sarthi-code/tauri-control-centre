# Testing Guide for Control Centre

This document describes the testing strategies and procedures for the Control Centre application.

## Test Categories

### 1. Window Behavior Tests

These tests verify the window appears and behaves correctly on Niri Wayland.

#### Manual Tests

| Test | Steps | Expected Result |
|------|-------|-----------------|
| Window Appears | Run `control-centre` | Window appears with blur/transparency |
| Correct Size | Observe window | Approximately 420x400 pixels |
| Rounded Corners | Observe window | All corners are rounded |
| Transparency | Observe background | Background shows blur effect |
| No Focus Stealing | Click outside, reopen | Other windows not disturbed |
| Close on Outside Click | Click outside window | Window closes/hides |
| Close on ESC | Press ESC | Window closes/hides |
| Always on Top | Open over other windows | Window stays above |

#### Automated Tests

```bash
# Run from project directory
cargo test window -- --nocapture
```

### 2. UI Interaction Tests

These tests verify all UI elements respond correctly.

#### Slider Tests

| Test | Steps | Expected Result |
|------|-------|-----------------|
| Volume Drag | Drag volume slider | Fill updates smoothly |
| Volume Click | Click on slider track | Value jumps to position |
| Volume Keyboard | Focus slider, press arrows | Value changes by 5% |
| Brightness Min | Drag to minimum | Stops at 1% (not 0) |
| Rapid Slider | Drag quickly back/forth | No freezing or errors |

#### Tile Tests

| Test | Steps | Expected Result |
|------|-------|-----------------|
| WiFi Toggle | Click WiFi tile | Tile color changes, status updates |
| Bluetooth Toggle | Click Bluetooth tile | Tile color changes, status updates |
| Night Light Toggle | Click Night Light tile | Tile color changes, display adjusts |
| Suspend Click | Click Suspend tile | Toast appears, system suspends |
| Double Click | Click tile twice rapidly | Only one action fires |
| Hover State | Hover over tile | Tile shows hover effect |
| Click Animation | Click tile | Ripple effect appears |

#### Automated Tests (Jest)

```bash
cd ui
npm install
npm test
```

### 3. Backend Command Tests

These tests verify system commands execute correctly.

#### Volume Tests

```bash
# Manual verification
pactl get-sink-volume @DEFAULT_SINK@
pactl set-sink-volume @DEFAULT_SINK@ 50%
pactl set-sink-mute @DEFAULT_SINK@ toggle
```

```rust
// Rust test
cargo test test_volume --ignored
```

#### Brightness Tests

```bash
# Manual verification
brightnessctl info
brightnessctl set 50%
```

```rust
// Rust test
cargo test test_brightness --ignored
```

#### Network Tests

```bash
# Manual verification
nmcli radio wifi
nmcli radio wifi off
nmcli radio wifi on
```

```rust
// Rust test
cargo test test_wifi --ignored
```

#### Bluetooth Tests

```bash
# Manual verification
bluetoothctl show
bluetoothctl power off
bluetoothctl power on
```

```rust
// Rust test
cargo test test_bluetooth --ignored
```

### 4. Toggle Behavior Tests

These tests verify single-instance and toggle functionality.

| Test | Steps | Expected Result |
|------|-------|-----------------|
| First Launch | Run `control-centre` when not running | Window opens |
| Second Launch | Run `control-centre` when open | Window toggles (hides) |
| Third Launch | Run `control-centre` when hidden | Window shows |
| Kill and Restart | Kill process, run again | New instance starts |
| Socket Cleanup | Kill process, check /tmp | Socket file removed |

```bash
# Automated toggle test
./scripts/control-centre status
./scripts/control-centre        # Should open
./scripts/control-centre        # Should close
./scripts/control-centre        # Should open
./scripts/control-centre kill
./scripts/control-centre status # Should show "Not running"
```

### 5. Niri-Specific Tests

These tests verify compatibility with Niri compositor.

| Test | Steps | Expected Result |
|------|-------|-----------------|
| Floating Window | Open CC | Window is floating, not tiled |
| Layer Stacking | Open CC over tiled window | CC appears above |
| Focus Return | Close CC | Focus returns to previous window |
| Blur Effect | Open CC with Niri blur | Blur visible through window |
| Fractional Scaling | Set 1.5x scaling | Window renders correctly |
| Multi-Monitor | Open on secondary monitor | Appears on correct monitor |

### 6. IPC Safety Tests

These tests verify frontend-backend communication is safe.

| Test | Steps | Expected Result |
|------|-------|-----------------|
| Invalid Volume | Send volume > 100 | Backend rejects, returns error |
| Negative Value | Send negative brightness | Backend rejects, returns error |
| Rapid Commands | Spam slider changes | Commands debounced, no crash |
| Command Timeout | Disconnect network, toggle WiFi | Graceful timeout |
| Error Display | Trigger error condition | User-friendly error message |

```rust
// Unit tests for validation
cargo test test_validate_percentage
```

## Running All Tests

### Quick Test (Unit Tests Only)

```bash
cargo test
```

### Full Test (Including Integration)

```bash
# Requires system services
cargo test -- --include-ignored
```

### Frontend Tests

```bash
cd ui
npm test
```

### Manual Test Checklist

Run through this checklist before releases:

- [ ] Fresh install on clean system
- [ ] Build succeeds without warnings
- [ ] Window appears on first run
- [ ] All tiles toggle correctly
- [ ] Volume slider adjusts audio
- [ ] Brightness slider adjusts screen
- [ ] Mute button works
- [ ] ESC closes window
- [ ] Click outside closes window
- [ ] Toggle script works from Waybar
- [ ] No console errors in dev tools
- [ ] Memory usage stays stable over time
- [ ] No zombie processes after close

## Test Environment Setup

### Required Services

```bash
# Ensure these are running
systemctl --user status pulseaudio  # or pipewire
systemctl status NetworkManager
systemctl status bluetooth
```

### Mock Mode (No Hardware)

For testing without actual hardware, the frontend includes mock mode:

```javascript
// In app.js, if Tauri is unavailable, mockInvoke() is used
// This allows UI testing in a browser
```

Open `ui/index.html` directly in a browser to test UI without Tauri.

## Continuous Integration

For CI environments without display:

```bash
# Build only (no GUI tests)
cargo build --release

# Run unit tests only
cargo test --lib

# Skip Tauri-specific tests
cargo test --features mock
```

## Performance Testing

### Memory Usage

```bash
# Monitor memory during use
watch -n 1 'ps aux | grep control-centre'
```

### Response Time

```bash
# Measure toggle latency
time control-centre
```

Target metrics:
- Window open: < 200ms
- Slider response: < 50ms
- Toggle state: < 100ms
- Command execution: < 500ms
