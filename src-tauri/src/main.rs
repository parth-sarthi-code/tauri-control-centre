//! Control Centre - A macOS-style Control Centre for Niri Wayland
//! 
//! This application provides a beautiful, modern control panel overlay
//! designed specifically for the Niri Wayland compositor.
//! 
//! ## Tauri v2 Compatibility
//! - Uses WebviewWindow instead of Window
//! - Uses tauri-plugin-shell for shell commands
//! - Escape key handling done via frontend JavaScript
//! 
//! ## Niri Compatibility Notes
//! - Uses transparent, frameless window for overlay behavior
//! - Configured as always-on-top to appear above tiled windows
//! - Uses skip_taskbar to avoid appearing in window lists
//! - Window positioning handled via Niri's window rules

#![cfg_attr(
    all(not(debug_assertions), target_os = "linux"),
    windows_subsystem = "windows"
)]

mod commands;
mod error;
mod state;

use log::{debug, error, info, warn};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{Emitter, Manager, RunEvent, WebviewWindow, WindowEvent};

use commands::*;
use state::AppState;

/// Global flag for window visibility state
static WINDOW_VISIBLE: AtomicBool = AtomicBool::new(false);

/// Socket path for IPC between instances
const SOCKET_PATH: &str = "/tmp/control-centre.sock";

fn main() {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!("Starting Control Centre for Niri Wayland");

    // Check for existing instance via socket
    if is_instance_running() {
        info!("Another instance is running, sending toggle signal");
        send_toggle_signal();
        return;
    }

    // Build Tauri application
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            // Audio commands
            get_volume,
            set_volume,
            toggle_mute,
            get_mute_status,
            // Brightness commands
            get_brightness,
            set_brightness,
            get_max_brightness,
            // Network commands
            get_wifi_status,
            set_wifi_enabled,
            get_bluetooth_status,
            set_bluetooth_enabled,
            // Display commands
            get_night_light_status,
            set_night_light_enabled,
            // Power commands
            suspend_system,
            // State commands
            get_all_states,
            // Window commands
            toggle_window,
            close_window,
            position_window,
        ])
        .setup(|app| {
            let window = app
                .get_webview_window("main")
                .expect("Failed to get main window");

            // Configure window for Niri Wayland
            setup_window_for_niri(&window);

            // Start socket listener for toggle signals from other instances
            let window_for_socket = window.clone();
            std::thread::spawn(move || {
                start_socket_listener(window_for_socket);
            });

            // Default: show window on startup
            show_window(&window);

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("Error building Tauri application");

    // Run the application
    app.run(|app_handle, event| {
        match event {
            RunEvent::WindowEvent { label, event, .. } => {
                if label == "main" {
                    match event {
                        WindowEvent::CloseRequested { api, .. } => {
                            // Prevent actual close, just hide
                            api.prevent_close();
                            if let Some(window) = app_handle.get_webview_window("main") {
                                let _ = window.hide();
                                WINDOW_VISIBLE.store(false, Ordering::SeqCst);
                            }
                        }
                        _ => {}
                    }
                }
            }
            RunEvent::ExitRequested { .. } => {
                // Clean up socket on exit
                let _ = std::fs::remove_file(SOCKET_PATH);
                info!("Application exiting, socket cleaned up");
            }
            _ => {}
        }
    });
}

// ============================================================================
// Instance Management
// ============================================================================

/// Check if another instance is already running
fn is_instance_running() -> bool {
    use std::os::unix::net::UnixStream;
    UnixStream::connect(SOCKET_PATH).is_ok()
}

/// Send toggle signal to running instance via Unix socket
fn send_toggle_signal() {
    use std::io::Write;
    use std::os::unix::net::UnixStream;

    match UnixStream::connect(SOCKET_PATH) {
        Ok(mut stream) => {
            let _ = stream.write_all(b"toggle");
            info!("Sent toggle signal to running instance");
        }
        Err(e) => {
            warn!("Failed to connect to socket: {}", e);
        }
    }
}

/// Start Unix socket listener for IPC
fn start_socket_listener(window: WebviewWindow) {
    use std::io::Read;
    use std::os::unix::net::UnixListener;

    // Remove old socket if exists
    let _ = std::fs::remove_file(SOCKET_PATH);

    match UnixListener::bind(SOCKET_PATH) {
        Ok(listener) => {
            info!("Socket listener started at {}", SOCKET_PATH);

            for stream in listener.incoming() {
                match stream {
                    Ok(mut stream) => {
                        let mut buf = [0u8; 32];
                        if let Ok(n) = stream.read(&mut buf) {
                            let msg = String::from_utf8_lossy(&buf[..n]);
                            if msg.trim() == "toggle" {
                                toggle_window_visibility(&window);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Socket accept error: {}", e);
                    }
                }
            }
        }
        Err(e) => {
            error!("Failed to bind socket: {}", e);
        }
    }
}

// ============================================================================
// Window Management
// ============================================================================

/// Configure window appearance for Niri Wayland overlay behavior
fn setup_window_for_niri(window: &WebviewWindow) {
    // Set window to be transparent and frameless (configured in tauri.conf.json)
    // These are additional runtime configurations

    // Attempt to set always on top (may not work on all Wayland compositors)
    if let Err(e) = window.set_always_on_top(true) {
        warn!("Failed to set always on top: {}", e);
    }

    // Skip taskbar/dock
    if let Err(e) = window.set_skip_taskbar(true) {
        warn!("Failed to set skip taskbar: {}", e);
    }

    info!("Window configured for Niri Wayland");
}

/// Show window and update visibility state
fn show_window(window: &WebviewWindow) {
    // Position window in top-right corner (with offset for waybar and margins)
    // Get the primary monitor's size
    if let Some(monitor) = window.primary_monitor().ok().flatten() {
        let screen_size = monitor.size();
        let scale = monitor.scale_factor();
        
        // Window dimensions (from tauri.conf.json)
        let window_width = 420.0;
        let window_height = 400.0;
        
        // Calculate top-right position with margins
        // Account for waybar height (~48px) and some padding (~10px)
        let margin_top = 58.0;  // waybar height + padding
        let margin_right = 10.0;
        
        let x = (screen_size.width as f64 / scale) - window_width - margin_right;
        let y = margin_top;
        
        let _ = window.set_position(tauri::Position::Logical(
            tauri::LogicalPosition::new(x, y)
        ));
        debug!("Window positioned at ({}, {})", x, y);
    }
    
    let _ = window.show();
    let _ = window.set_focus();
    WINDOW_VISIBLE.store(true, Ordering::SeqCst);

    // Emit event to frontend to refresh state
    let _ = window.emit("window-shown", ());
    debug!("Window shown");
}

/// Toggle window visibility
fn toggle_window_visibility(window: &WebviewWindow) {
    if WINDOW_VISIBLE.load(Ordering::SeqCst) {
        let _ = window.hide();
        WINDOW_VISIBLE.store(false, Ordering::SeqCst);
        debug!("Window hidden via toggle");
    } else {
        show_window(window);
    }
}

// ============================================================================
// Tauri Commands - Window Management
// ============================================================================

/// Toggle window visibility from frontend
#[tauri::command]
async fn toggle_window(window: WebviewWindow) -> Result<bool, String> {
    let visible = WINDOW_VISIBLE.load(Ordering::SeqCst);
    if visible {
        window.hide().map_err(|e| e.to_string())?;
        WINDOW_VISIBLE.store(false, Ordering::SeqCst);
    } else {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        WINDOW_VISIBLE.store(true, Ordering::SeqCst);
    }
    Ok(!visible)
}

/// Close/hide window from frontend (ESC key handler)
#[tauri::command]
async fn close_window(window: WebviewWindow) -> Result<(), String> {
    window.hide().map_err(|e| e.to_string())?;
    WINDOW_VISIBLE.store(false, Ordering::SeqCst);
    Ok(())
}

/// Position window on screen
///
/// Note: On Niri Wayland, window positioning is handled by the compositor.
/// This command calculates desired position but actual placement depends
/// on Niri window rules.
#[tauri::command]
async fn position_window(window: WebviewWindow, x: i32, y: i32) -> Result<(), String> {
    // On Wayland/Niri, direct positioning may not work
    // The compositor handles window placement based on rules
    // This is a best-effort attempt
    window
        .set_position(tauri::Position::Logical(tauri::LogicalPosition::new(
            x as f64, y as f64,
        )))
        .map_err(|e| e.to_string())?;
    Ok(())
}
