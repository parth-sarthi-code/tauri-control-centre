//! Integration tests for Control Centre
//!
//! These tests verify the system command integrations work correctly.

use std::process::Command;

fn command_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
fn test_pactl_available() {
    assert!(
        command_exists("pactl"),
        "pactl is required for volume control"
    );
}

#[test]
fn test_brightnessctl_available() {
    assert!(
        command_exists("brightnessctl"),
        "brightnessctl is required for brightness control"
    );
}

#[test]
fn test_nmcli_available() {
    assert!(
        command_exists("nmcli"),
        "nmcli is required for WiFi control"
    );
}

#[test]
fn test_bluetoothctl_available() {
    assert!(
        command_exists("bluetoothctl"),
        "bluetoothctl is required for Bluetooth control"
    );
}

#[test]
fn test_loginctl_available() {
    assert!(
        command_exists("loginctl"),
        "loginctl is required for suspend"
    );
}

#[test]
fn test_unix_socket_creation() {
    use std::os::unix::net::UnixListener;
    
    let socket_path = "/tmp/test-control-centre.sock";
    let _ = std::fs::remove_file(socket_path);
    
    let listener = UnixListener::bind(socket_path);
    assert!(listener.is_ok(), "Failed to create Unix socket");
    
    let _ = std::fs::remove_file(socket_path);
}

#[test]
fn test_volume_regex() {
    let re = regex::Regex::new(r"(\d+)%").unwrap();
    
    let test_cases = vec![
        ("Volume: front-left: 65536 / 100% / 0.00 dB", Some(100)),
        ("Volume: front-left: 32768 /  50% / -12.00 dB", Some(50)),
        ("No percentage here", None),
    ];
    
    for (input, expected) in test_cases {
        let result = re
            .captures(input)
            .and_then(|cap| cap.get(1))
            .and_then(|m| m.as_str().parse::<u8>().ok());
        
        assert_eq!(result, expected, "Failed for input: {}", input);
    }
}

#[test]
fn test_brightness_csv_parsing() {
    let test_cases = vec![
        ("intel_backlight,backlight,1000,39%,2550", Some(39)),
        ("amdgpu_bl0,backlight,255,100%,255", Some(100)),
        ("invalid", None),
    ];
    
    for (input, expected) in test_cases {
        let result = input
            .split(',')
            .nth(3)
            .and_then(|s| s.trim_end_matches('%').parse::<u8>().ok());
        
        assert_eq!(result, expected, "Failed for input: {}", input);
    }
}
