//! Tauri Commands for System Control
//! 
//! This module provides all system control commands for the Control Centre.
//! Each command wraps a CLI tool and handles errors appropriately.
//! 
//! ## Security Notes
//! - All commands use direct process execution (no shell)
//! - Input values are validated before use
//! - No user input is passed to shell expansion
//! 
//! ## Niri Compatibility
//! - Commands are non-blocking to prevent UI freezing
//! - Uses tokio::process for async execution
//! - All outputs are sanitized for IPC safety

use crate::error::{CCResult, ControlCentreError};
use crate::state::AppState;
use log::{debug, error, info, warn};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tauri::State;
use tokio::process::Command;

// ============================================================================
// Response Types
// ============================================================================

/// Volume state response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeState {
    pub volume: u8,
    pub muted: bool,
}

/// Brightness state response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrightnessState {
    pub brightness: u8,
    pub max_brightness: u32,
}

/// Network state response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkState {
    pub wifi_enabled: bool,
    pub wifi_connected: bool,
    pub wifi_ssid: Option<String>,
    pub bluetooth_enabled: bool,
    pub bluetooth_connected: bool,
}

/// Display state response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayState {
    pub night_light_enabled: bool,
}

/// Complete system state response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllStates {
    pub volume: VolumeState,
    pub brightness: BrightnessState,
    pub network: NetworkState,
    pub display: DisplayState,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Execute a command and return stdout as string
async fn run_command(cmd: &str, args: &[&str]) -> CCResult<String> {
    debug!("Running command: {} {:?}", cmd, args);
    
    let output = Command::new(cmd)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| {
            error!("Failed to execute {}: {}", cmd, e);
            ControlCentreError::from(e)
        })?;
    
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        debug!("Command output: {}", stdout);
        Ok(stdout)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        error!("Command {} failed: {}", cmd, stderr);
        Err(ControlCentreError::CommandFailed(stderr))
    }
}

/// Execute a command without capturing output
async fn run_command_no_output(cmd: &str, args: &[&str]) -> CCResult<()> {
    debug!("Running command (no output): {} {:?}", cmd, args);
    
    let status = Command::new(cmd)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .status()
        .await
        .map_err(|e| {
            error!("Failed to execute {}: {}", cmd, e);
            ControlCentreError::from(e)
        })?;
    
    if status.success() {
        Ok(())
    } else {
        Err(ControlCentreError::CommandFailed(format!(
            "Command {} exited with status {}",
            cmd,
            status.code().unwrap_or(-1)
        )))
    }
}

/// Validate percentage value (0-100)
fn validate_percentage(value: u8) -> CCResult<u8> {
    if value > 100 {
        Err(ControlCentreError::InvalidArgument(format!(
            "Percentage must be 0-100, got {}",
            value
        )))
    } else {
        Ok(value)
    }
}

// ============================================================================
// Audio Commands (PulseAudio/PipeWire via pactl)
// ============================================================================

/// Get current volume level (0-100)
#[tauri::command]
pub async fn get_volume(state: State<'_, AppState>) -> Result<u8, String> {
    if let Some(cached) = state.get_cached_volume() {
        return Ok(cached);
    }
    
    let output = run_command("pactl", &["get-sink-volume", "@DEFAULT_SINK@"])
        .await
        .map_err(|e| e.to_string())?;
    
    let re = Regex::new(r"(\d+)%").map_err(|e| e.to_string())?;
    
    let volume = re
        .captures(&output)
        .and_then(|cap| cap.get(1))
        .and_then(|m| m.as_str().parse::<u8>().ok())
        .unwrap_or(50);
    
    state.set_cached_volume(volume);
    Ok(volume)
}

/// Set volume level (0-100)
#[tauri::command]
pub async fn set_volume(state: State<'_, AppState>, value: u8) -> Result<u8, String> {
    let value = validate_percentage(value).map_err(|e| e.to_string())?;
    
    run_command_no_output("pactl", &["set-sink-volume", "@DEFAULT_SINK@", &format!("{}%", value)])
        .await
        .map_err(|e| e.to_string())?;
    
    state.set_cached_volume(value);
    info!("Volume set to {}%", value);
    
    Ok(value)
}

/// Toggle mute state
#[tauri::command]
pub async fn toggle_mute(state: State<'_, AppState>) -> Result<bool, String> {
    run_command_no_output("pactl", &["set-sink-mute", "@DEFAULT_SINK@", "toggle"])
        .await
        .map_err(|e| e.to_string())?;
    
    let muted = get_mute_status_internal().await.map_err(|e| e.to_string())?;
    state.set_cached_muted(muted);
    
    info!("Mute toggled, now: {}", muted);
    Ok(muted)
}

/// Get mute status
#[tauri::command]
pub async fn get_mute_status(state: State<'_, AppState>) -> Result<bool, String> {
    if let Some(cached) = state.get_cached_muted() {
        return Ok(cached);
    }
    
    let muted = get_mute_status_internal().await.map_err(|e| e.to_string())?;
    state.set_cached_muted(muted);
    Ok(muted)
}

async fn get_mute_status_internal() -> CCResult<bool> {
    let output = run_command("pactl", &["get-sink-mute", "@DEFAULT_SINK@"]).await?;
    Ok(output.to_lowercase().contains("yes"))
}

// ============================================================================
// Brightness Commands (brightnessctl)
// ============================================================================

/// Get current brightness percentage (0-100)
#[tauri::command]
pub async fn get_brightness(state: State<'_, AppState>) -> Result<u8, String> {
    if let Some(cached) = state.get_cached_brightness() {
        return Ok(cached);
    }
    
    let output = run_command("brightnessctl", &["-m", "info"])
        .await
        .map_err(|e| e.to_string())?;
    
    let brightness = output
        .split(',')
        .nth(3)
        .and_then(|s| s.trim_end_matches('%').parse::<u8>().ok())
        .unwrap_or(50);
    
    state.set_cached_brightness(brightness);
    Ok(brightness)
}

/// Set brightness percentage (0-100)
#[tauri::command]
pub async fn set_brightness(state: State<'_, AppState>, value: u8) -> Result<u8, String> {
    let value = validate_percentage(value).map_err(|e| e.to_string())?;
    let safe_value = value.max(1);
    
    run_command_no_output("brightnessctl", &["set", &format!("{}%", safe_value)])
        .await
        .map_err(|e| e.to_string())?;
    
    state.set_cached_brightness(safe_value);
    info!("Brightness set to {}%", safe_value);
    
    Ok(safe_value)
}

/// Get maximum brightness value
#[tauri::command]
pub async fn get_max_brightness() -> Result<u32, String> {
    let output = run_command("brightnessctl", &["max"])
        .await
        .map_err(|e| e.to_string())?;
    
    output
        .trim()
        .parse::<u32>()
        .map_err(|e| format!("Failed to parse max brightness: {}", e))
}

// ============================================================================
// WiFi Commands (NetworkManager via nmcli)
// ============================================================================

/// Get WiFi enabled status
#[tauri::command]
pub async fn get_wifi_status(state: State<'_, AppState>) -> Result<NetworkState, String> {
    let wifi_output = run_command("nmcli", &["radio", "wifi"])
        .await
        .map_err(|e| e.to_string())?;
    
    let wifi_enabled = wifi_output.to_lowercase().trim() == "enabled";
    state.set_cached_wifi(wifi_enabled);
    
    let mut wifi_connected = false;
    let mut wifi_ssid = None;
    
    if wifi_enabled {
        if let Ok(conn_output) = run_command("nmcli", &["-t", "-f", "ACTIVE,SSID", "device", "wifi"]).await {
            for line in conn_output.lines() {
                if line.starts_with("yes:") {
                    wifi_connected = true;
                    wifi_ssid = line.strip_prefix("yes:").map(|s| s.to_string());
                    break;
                }
            }
        }
    }
    
    let bt_enabled = get_bluetooth_status_internal().await.unwrap_or(false);
    state.set_cached_bluetooth(bt_enabled);
    
    Ok(NetworkState {
        wifi_enabled,
        wifi_connected,
        wifi_ssid,
        bluetooth_enabled: bt_enabled,
        bluetooth_connected: false,
    })
}

/// Enable or disable WiFi
#[tauri::command]
pub async fn set_wifi_enabled(state: State<'_, AppState>, enabled: bool) -> Result<bool, String> {
    let arg = if enabled { "on" } else { "off" };
    
    run_command_no_output("nmcli", &["radio", "wifi", arg])
        .await
        .map_err(|e| e.to_string())?;
    
    state.set_cached_wifi(enabled);
    info!("WiFi set to {}", arg);
    
    Ok(enabled)
}

// ============================================================================
// Bluetooth Commands (bluetoothctl)
// ============================================================================

/// Get Bluetooth enabled status
#[tauri::command]
pub async fn get_bluetooth_status(state: State<'_, AppState>) -> Result<bool, String> {
    if let Some(cached) = state.get_cached_bluetooth() {
        return Ok(cached);
    }
    
    let enabled = get_bluetooth_status_internal()
        .await
        .map_err(|e| e.to_string())?;
    
    state.set_cached_bluetooth(enabled);
    Ok(enabled)
}

async fn get_bluetooth_status_internal() -> CCResult<bool> {
    let output = run_command("bluetoothctl", &["show"]).await?;
    Ok(output.contains("Powered: yes"))
}

/// Enable or disable Bluetooth
#[tauri::command]
pub async fn set_bluetooth_enabled(state: State<'_, AppState>, enabled: bool) -> Result<bool, String> {
    let arg = if enabled { "on" } else { "off" };
    
    run_command_no_output("bluetoothctl", &["power", arg])
        .await
        .map_err(|e| e.to_string())?;
    
    state.set_cached_bluetooth(enabled);
    info!("Bluetooth set to {}", arg);
    
    Ok(enabled)
}

// ============================================================================
// Night Light Commands
// ============================================================================

/// Get Night Light enabled status
#[tauri::command]
pub async fn get_night_light_status(state: State<'_, AppState>) -> Result<bool, String> {
    if let Some(cached) = state.get_cached_night_light() {
        return Ok(cached);
    }
    
    let result = run_command(
        "gsettings",
        &["get", "org.gnome.settings-daemon.plugins.color", "night-light-enabled"],
    )
    .await;
    
    let enabled = match result {
        Ok(output) => output.trim() == "true",
        Err(_) => {
            warn!("GNOME settings not available, trying alternatives");
            if let Ok(output) = run_command("pgrep", &["-x", "gammastep"]).await {
                !output.is_empty()
            } else if let Ok(output) = run_command("pgrep", &["-x", "redshift"]).await {
                !output.is_empty()
            } else {
                false
            }
        }
    };
    
    state.set_cached_night_light(enabled);
    Ok(enabled)
}

/// Enable or disable Night Light
#[tauri::command]
pub async fn set_night_light_enabled(state: State<'_, AppState>, enabled: bool) -> Result<bool, String> {
    let value = if enabled { "true" } else { "false" };
    
    let result = run_command_no_output(
        "gsettings",
        &["set", "org.gnome.settings-daemon.plugins.color", "night-light-enabled", value],
    )
    .await;
    
    match result {
        Ok(_) => {
            state.set_cached_night_light(enabled);
            info!("Night Light set to {}", value);
            Ok(enabled)
        }
        Err(_) => {
            warn!("GNOME settings not available, using gammastep fallback");
            
            if enabled {
                let _ = run_command_no_output("gammastep", &["-O", "4500"]).await;
            } else {
                let _ = run_command_no_output("pkill", &["-x", "gammastep"]).await;
                let _ = run_command_no_output("pkill", &["-x", "wlsunset"]).await;
            }
            
            state.set_cached_night_light(enabled);
            Ok(enabled)
        }
    }
}

// ============================================================================
// Power Commands
// ============================================================================

/// Suspend the system
#[tauri::command]
pub async fn suspend_system() -> Result<(), String> {
    info!("Suspending system...");
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    
    run_command_no_output("loginctl", &["suspend"])
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(())
}

// ============================================================================
// Aggregate State Command
// ============================================================================

/// Get all system states at once
#[tauri::command]
pub async fn get_all_states(state: State<'_, AppState>) -> Result<AllStates, String> {
    state.invalidate_all();
    
    let (volume, muted, brightness, max_brightness, network, night_light) = tokio::join!(
        async { get_volume_internal().await.unwrap_or(50) },
        async { get_mute_status_internal().await.unwrap_or(false) },
        async { get_brightness_internal().await.unwrap_or(50) },
        async { get_max_brightness_internal().await.unwrap_or(100) },
        async { get_network_state_internal().await },
        async { get_night_light_internal().await.unwrap_or(false) },
    );
    
    state.set_cached_volume(volume);
    state.set_cached_muted(muted);
    state.set_cached_brightness(brightness);
    state.set_cached_wifi(network.wifi_enabled);
    state.set_cached_bluetooth(network.bluetooth_enabled);
    state.set_cached_night_light(night_light);
    
    Ok(AllStates {
        volume: VolumeState { volume, muted },
        brightness: BrightnessState { brightness, max_brightness },
        network,
        display: DisplayState { night_light_enabled: night_light },
    })
}

// Internal helpers
async fn get_volume_internal() -> CCResult<u8> {
    let output = run_command("pactl", &["get-sink-volume", "@DEFAULT_SINK@"]).await?;
    let re = Regex::new(r"(\d+)%").map_err(|e| ControlCentreError::ParseError(e.to_string()))?;
    Ok(re.captures(&output)
        .and_then(|cap| cap.get(1))
        .and_then(|m| m.as_str().parse::<u8>().ok())
        .unwrap_or(50))
}

async fn get_brightness_internal() -> CCResult<u8> {
    let output = run_command("brightnessctl", &["-m", "info"]).await?;
    Ok(output.split(',').nth(3)
        .and_then(|s| s.trim_end_matches('%').parse::<u8>().ok())
        .unwrap_or(50))
}

async fn get_max_brightness_internal() -> CCResult<u32> {
    let output = run_command("brightnessctl", &["max"]).await?;
    output.trim().parse::<u32>()
        .map_err(|e| ControlCentreError::ParseError(e.to_string()))
}

async fn get_network_state_internal() -> NetworkState {
    let wifi_enabled = run_command("nmcli", &["radio", "wifi"])
        .await
        .map(|o| o.to_lowercase().trim() == "enabled")
        .unwrap_or(false);
    
    let mut wifi_connected = false;
    let mut wifi_ssid = None;
    
    if wifi_enabled {
        if let Ok(conn_output) = run_command("nmcli", &["-t", "-f", "ACTIVE,SSID", "device", "wifi"]).await {
            for line in conn_output.lines() {
                if line.starts_with("yes:") {
                    wifi_connected = true;
                    wifi_ssid = line.strip_prefix("yes:").map(|s| s.to_string());
                    break;
                }
            }
        }
    }
    
    let bluetooth_enabled = get_bluetooth_status_internal().await.unwrap_or(false);
    
    NetworkState {
        wifi_enabled,
        wifi_connected,
        wifi_ssid,
        bluetooth_enabled,
        bluetooth_connected: false,
    }
}

async fn get_night_light_internal() -> CCResult<bool> {
    let result = run_command(
        "gsettings",
        &["get", "org.gnome.settings-daemon.plugins.color", "night-light-enabled"],
    ).await;
    
    match result {
        Ok(output) => Ok(output.trim() == "true"),
        Err(_) => {
            if let Ok(output) = run_command("pgrep", &["-x", "gammastep"]).await {
                Ok(!output.is_empty())
            } else {
                Ok(false)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_validate_percentage() {
        assert!(validate_percentage(0).is_ok());
        assert!(validate_percentage(50).is_ok());
        assert!(validate_percentage(100).is_ok());
        assert!(validate_percentage(101).is_err());
    }
    
    #[test]
    fn test_volume_parsing() {
        let test_output = "Volume: front-left: 65536 / 100% / 0.00 dB";
        let re = Regex::new(r"(\d+)%").unwrap();
        let volume = re.captures(test_output)
            .and_then(|cap| cap.get(1))
            .and_then(|m| m.as_str().parse::<u8>().ok())
            .unwrap_or(0);
        assert_eq!(volume, 100);
    }
    
    #[test]
    fn test_brightness_parsing() {
        let test_output = "intel_backlight,backlight,1000,39%,2550";
        let brightness = test_output.split(',').nth(3)
            .and_then(|s| s.trim_end_matches('%').parse::<u8>().ok())
            .unwrap_or(0);
        assert_eq!(brightness, 39);
    }
}
