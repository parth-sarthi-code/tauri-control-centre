//! Control Centre - A macOS-style Control Centre for Niri Wayland
//! 
//! This application provides a beautiful, modern control panel overlay
//! designed specifically for the Niri Wayland compositor.
//! 
//! ## Niri Compatibility Notes
//! - Uses transparent, frameless window for overlay behavior
//! - Configured as always-on-top to appear above tiled windows
//! - Uses skip_taskbar to avoid appearing in window lists
//! - Window positioning handled via Niri's window rules
//! 
//! ## Security Notes
//! - All system commands use strict argument validation
//! - No shell expansion - direct command execution only
//! - All outputs are sanitized before returning to frontend

#![cfg_attr(
    all(not(debug_assertions), target_os = "linux"),
    windows_subsystem = "windows"
)]

mod commands;
mod error;
mod state;

use anyhow::Result;
use log::{info, warn, error};
use single_instance::SingleInstance;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{
    Manager, RunEvent, Window, WindowEvent, GlobalShortcutManager,
};

use commands::*;
use state::AppState;

/// Global flag for window visibility state
static WINDOW_VISIBLE: AtomicBool = AtomicBool::new(false);

/// Application name for single-instance detection
const APP_INSTANCE_NAME: &str = "com.niri.control-centre";

/// Socket path for IPC between instances
const SOCKET_PATH: &str = "/tmp/control-centre.sock";

fn main() -> Result<()> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    
    info!("Starting Control Centre for Niri Wayland");
    
    // Check for single instance
    let instance = SingleInstance::new(APP_INSTANCE_NAME)
        .expect("Failed to create single instance lock");
    
    if !instance.is_single() {
        info!("Another instance is running, sending toggle signal");
        send_toggle_signal();
        return Ok(());
    }
    
    // Build Tauri application
    let app = tauri::Builder::default()
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
            let window = app.get_window("main").expect("Failed to get main window");
            
            // Configure window for Niri Wayland
            setup_window_for_niri(&window);
            
            // Register global shortcut for ESC to close
            let window_clone = window.clone();
            app.global_shortcut_manager()
                .register("Escape", move || {
                    if WINDOW_VISIBLE.load(Ordering::SeqCst) {
                        let _ = window_clone.hide();
                        WINDOW_VISIBLE.store(false, Ordering::SeqCst);
                    }
                })
                .ok();
            
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
                        WindowEvent::Focused(focused) => {
                            // On Niri, losing focus typically means user clicked outside
                            // We hide the window in this case for overlay behavior
                            if !focused && WINDOW_VISIBLE.load(Ordering::SeqCst) {
                                if let Some(window) = app_handle.get_window("main") {
                                    // Small delay to prevent race conditions
                                    std::thread::spawn(move || {
                                        std::thread::sleep(std::time::Duration::from_millis(100));
                                        if !window.is_focused().unwrap_or(false) {
                                            let _ = window.hide();
                                            WINDOW_VISIBLE.store(false, Ordering::SeqCst);
                                        }
                                    });
                                }
                            }
                        }
                        WindowEvent::CloseRequested { api, .. } => {
                            // Prevent actual close, just hide
                            api.prevent_close();
                            if let Some(window) = app_handle.get_window("main") {
                                let _ = window.hide();
                                WINDOW_VISIBLE.store(false, Ordering::SeqCst);
                            }
                        }
                        _ => {}
                    }
                }
            }
            RunEvent::ExitRequested { .. } => {
                // Clean up socket file on exit
                let _ = std::fs::remove_file(SOCKET_PATH);
            }
            _ => {}
        }
    });
    
    Ok(())
}

/// Configure window for Niri Wayland compositor
/// 
/// Niri-specific notes:
/// - Niri handles layer-shell differently than other compositors
/// - For overlay windows, use window rules in niri config
/// - Transparent windows work best with decorations=false
fn setup_window_for_niri(window: &Window) {
    // Set window type hints for Wayland
    // Note: On Niri, additional configuration in niri.kdl may be needed:
    // 
    // window-rule {
    //     match app-id="control-centre"
    //     open-floating true
    //     default-column-width { proportion 0.3; }
    // }
    
    info!("Configuring window for Niri Wayland compatibility");
    
    // The window is configured in tauri.conf.json with:
    // - transparent: true (for blur effect)
    // - decorations: false (frameless)
    // - always_on_top: true (overlay behavior)
    // - skip_taskbar: true (no taskbar entry)
    // - resizable: false (fixed size overlay)
}

/// Show and focus the window
fn show_window(window: &Window) {
    let _ = window.show();
    let _ = window.set_focus();
    WINDOW_VISIBLE.store(true, Ordering::SeqCst);
    
    // Emit event to frontend to refresh state
    let _ = window.emit("window-shown", ());
}

/// Toggle window visibility
fn toggle_window_visibility(window: &Window) {
    if WINDOW_VISIBLE.load(Ordering::SeqCst) {
        let _ = window.hide();
        WINDOW_VISIBLE.store(false, Ordering::SeqCst);
    } else {
        show_window(window);
    }
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
fn start_socket_listener(window: Window) {
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
// Tauri Commands - Window Management
// ============================================================================

/// Toggle window visibility from frontend
#[tauri::command]
async fn toggle_window(window: Window) -> Result<bool, String> {
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

/// Close window from frontend (actually hides)
#[tauri::command]
async fn close_window(window: Window) -> Result<(), String> {
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
async fn position_window(window: Window, x: i32, y: i32) -> Result<(), String> {
    // On Wayland/Niri, direct positioning may not work
    // The compositor handles window placement based on rules
    // This is a best-effort attempt
    window.set_position(tauri::Position::Logical(tauri::LogicalPosition::new(x as f64, y as f64)))
        .map_err(|e| e.to_string())?;
    Ok(())
}
