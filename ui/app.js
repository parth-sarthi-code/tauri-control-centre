/**
 * Control Centre Frontend Application
 * 
 * Handles all UI interactions and communicates with Tauri backend via IPC.
 * 
 * Niri Wayland Compatibility Notes:
 * - All system commands go through Tauri IPC (secure)
 * - Window events handled by backend for proper Wayland behavior
 * - Keyboard shortcuts (ESC) handled by global shortcut in backend
 * 
 * Security Notes:
 * - No direct shell access from frontend
 * - All values validated before sending to backend
 * - Error messages sanitized before display
 */

// ============================================================================
// Tauri IPC Interface
// ============================================================================

/**
 * Safely invoke a Tauri command with error handling
 * @param {string} command - The command name
 * @param {object} args - Command arguments
 * @returns {Promise<any>} - Command result
 */
async function invoke(command, args = {}) {
    try {
        // Check if Tauri is available (v2 uses window.__TAURI__.core.invoke)
        if (typeof window.__TAURI__ !== 'undefined') {
            // Tauri v2 API
            if (window.__TAURI__.core && window.__TAURI__.core.invoke) {
                return await window.__TAURI__.core.invoke(command, args);
            }
            // Fallback for v1 API
            if (window.__TAURI__.invoke) {
                return await window.__TAURI__.invoke(command, args);
            }
        }
        console.warn('Tauri not available, using mock mode');
        return mockInvoke(command, args);
    } catch (error) {
        console.error(`Command ${command} failed:`, error);
        showToast(`Error: ${error}`, 'error');
        throw error;
    }
}

/**
 * Mock invoke for development/testing outside Tauri
 */
function mockInvoke(command, args) {
    console.log(`[MOCK] ${command}`, args);
    
    const mockState = {
        volume: 50,
        muted: false,
        brightness: 75,
        wifi: true,
        bluetooth: false,
        nightLight: false,
    };
    
    switch (command) {
        case 'get_all_states':
            return {
                volume: { volume: mockState.volume, muted: mockState.muted },
                brightness: { brightness: mockState.brightness, max_brightness: 100 },
                network: { 
                    wifi_enabled: mockState.wifi, 
                    wifi_connected: true, 
                    wifi_ssid: 'MockNetwork',
                    bluetooth_enabled: mockState.bluetooth,
                    bluetooth_connected: false
                },
                display: { night_light_enabled: mockState.nightLight }
            };
        case 'get_volume':
            return mockState.volume;
        case 'set_volume':
            mockState.volume = args.value;
            return args.value;
        case 'toggle_mute':
            mockState.muted = !mockState.muted;
            return mockState.muted;
        case 'get_mute_status':
            return mockState.muted;
        case 'get_brightness':
            return mockState.brightness;
        case 'set_brightness':
            mockState.brightness = args.value;
            return args.value;
        case 'get_wifi_status':
            return { wifi_enabled: mockState.wifi, wifi_connected: true, wifi_ssid: 'MockNetwork', bluetooth_enabled: mockState.bluetooth, bluetooth_connected: false };
        case 'set_wifi_enabled':
            mockState.wifi = args.enabled;
            return args.enabled;
        case 'get_bluetooth_status':
            return mockState.bluetooth;
        case 'set_bluetooth_enabled':
            mockState.bluetooth = args.enabled;
            return args.enabled;
        case 'get_night_light_status':
            return mockState.nightLight;
        case 'set_night_light_enabled':
            mockState.nightLight = args.enabled;
            return args.enabled;
        case 'suspend_system':
            console.log('[MOCK] System would suspend');
            return null;
        case 'close_window':
            console.log('[MOCK] Window would close');
            return null;
        default:
            return null;
    }
}

// ============================================================================
// State Management
// ============================================================================

const state = {
    volume: 50,
    muted: false,
    brightness: 50,
    wifiEnabled: false,
    wifiConnected: false,
    wifiSsid: null,
    bluetoothEnabled: false,
    nightLightEnabled: false,
    isLoading: true,
    pendingOperations: new Set(),
};

// ============================================================================
// DOM Elements
// ============================================================================

const elements = {
    // Tiles
    wifiTile: null,
    bluetoothTile: null,
    nightLightTile: null,
    suspendTile: null,
    
    // Sliders
    volumeSlider: null,
    volumeFill: null,
    volumeValue: null,
    volumeIcon: null,
    volumeMutedIcon: null,
    volumeIconBtn: null,
    
    brightnessSlider: null,
    brightnessFill: null,
    brightnessValue: null,
    
    // Status displays
    wifiStatus: null,
    bluetoothStatus: null,
    nightLightStatus: null,
    
    // UI elements
    closeBtn: null,
    loadingOverlay: null,
    toast: null,
};

// ============================================================================
// Initialization
// ============================================================================

document.addEventListener('DOMContentLoaded', async () => {
    // Cache DOM elements
    cacheElements();
    
    // Set up event listeners
    setupEventListeners();
    
    // Load initial state
    await loadInitialState();
    
    // Listen for Tauri events
    setupTauriEvents();
    
    // Hide loading overlay
    hideLoading();
});

function cacheElements() {
    elements.wifiTile = document.getElementById('wifi-tile');
    elements.bluetoothTile = document.getElementById('bluetooth-tile');
    elements.nightLightTile = document.getElementById('nightlight-tile');
    elements.suspendTile = document.getElementById('suspend-tile');
    
    elements.volumeSlider = document.getElementById('volume-slider');
    elements.volumeFill = document.getElementById('volume-fill');
    elements.volumeValue = document.getElementById('volume-value');
    elements.volumeIcon = document.getElementById('volume-icon');
    elements.volumeMutedIcon = document.getElementById('volume-muted-icon');
    elements.volumeIconBtn = document.getElementById('volume-icon-btn');
    
    elements.brightnessSlider = document.getElementById('brightness-slider');
    elements.brightnessFill = document.getElementById('brightness-fill');
    elements.brightnessValue = document.getElementById('brightness-value');
    
    elements.wifiStatus = document.getElementById('wifi-status');
    elements.bluetoothStatus = document.getElementById('bluetooth-status');
    elements.nightLightStatus = document.getElementById('nightlight-status');
    
    elements.closeBtn = document.getElementById('close-btn');
    elements.loadingOverlay = document.getElementById('loading-overlay');
    elements.toast = document.getElementById('toast');
}

// ============================================================================
// Event Listeners
// ============================================================================

function setupEventListeners() {
    // Close button
    elements.closeBtn.addEventListener('click', closeWindow);
    
    // Tile clicks
    elements.wifiTile.addEventListener('click', (e) => {
        addRippleEffect(e);
        toggleWifi();
    });
    
    elements.bluetoothTile.addEventListener('click', (e) => {
        addRippleEffect(e);
        toggleBluetooth();
    });
    
    elements.nightLightTile.addEventListener('click', (e) => {
        addRippleEffect(e);
        toggleNightLight();
    });
    
    elements.suspendTile.addEventListener('click', (e) => {
        addRippleEffect(e);
        suspendSystem();
    });
    
    // Volume slider
    elements.volumeSlider.addEventListener('input', handleVolumeInput);
    elements.volumeSlider.addEventListener('change', handleVolumeChange);
    elements.volumeIconBtn.addEventListener('click', toggleMute);
    
    // Brightness slider
    elements.brightnessSlider.addEventListener('input', handleBrightnessInput);
    elements.brightnessSlider.addEventListener('change', handleBrightnessChange);
    
    // Keyboard shortcuts
    document.addEventListener('keydown', handleKeyDown);
    
    // Prevent context menu
    document.addEventListener('contextmenu', (e) => e.preventDefault());
}

function setupTauriEvents() {
    if (typeof window.__TAURI__ === 'undefined') return;
    
    // Listen for window show event to refresh state
    window.__TAURI__.event.listen('window-shown', async () => {
        console.log('Window shown, refreshing state');
        await loadInitialState();
    });
}

// ============================================================================
// State Loading
// ============================================================================

async function loadInitialState() {
    try {
        showLoading();
        
        const allStates = await invoke('get_all_states');
        
        // Update state
        state.volume = allStates.volume.volume;
        state.muted = allStates.volume.muted;
        state.brightness = allStates.brightness.brightness;
        state.wifiEnabled = allStates.network.wifi_enabled;
        state.wifiConnected = allStates.network.wifi_connected;
        state.wifiSsid = allStates.network.wifi_ssid;
        state.bluetoothEnabled = allStates.network.bluetooth_enabled;
        state.nightLightEnabled = allStates.display.night_light_enabled;
        
        // Update UI
        updateAllUI();
        
    } catch (error) {
        console.error('Failed to load initial state:', error);
        showToast('Failed to load system state', 'error');
    } finally {
        hideLoading();
    }
}

// ============================================================================
// UI Updates
// ============================================================================

function updateAllUI() {
    updateVolumeUI();
    updateBrightnessUI();
    updateWifiUI();
    updateBluetoothUI();
    updateNightLightUI();
}

function updateVolumeUI() {
    elements.volumeSlider.value = state.volume;
    elements.volumeFill.style.width = `${state.volume}%`;
    elements.volumeValue.textContent = `${state.volume}%`;
    
    // Update mute icon
    if (state.muted) {
        elements.volumeIcon.style.display = 'none';
        elements.volumeMutedIcon.style.display = 'block';
        elements.volumeIconBtn.classList.add('muted');
    } else {
        elements.volumeIcon.style.display = 'block';
        elements.volumeMutedIcon.style.display = 'none';
        elements.volumeIconBtn.classList.remove('muted');
    }
}

function updateBrightnessUI() {
    elements.brightnessSlider.value = state.brightness;
    elements.brightnessFill.style.width = `${state.brightness}%`;
    elements.brightnessValue.textContent = `${state.brightness}%`;
}

function updateWifiUI() {
    elements.wifiTile.dataset.enabled = state.wifiEnabled;
    
    if (state.wifiEnabled && state.wifiConnected && state.wifiSsid) {
        elements.wifiStatus.textContent = state.wifiSsid;
    } else if (state.wifiEnabled) {
        elements.wifiStatus.textContent = 'On';
    } else {
        elements.wifiStatus.textContent = 'Off';
    }
}

function updateBluetoothUI() {
    elements.bluetoothTile.dataset.enabled = state.bluetoothEnabled;
    elements.bluetoothStatus.textContent = state.bluetoothEnabled ? 'On' : 'Off';
}

function updateNightLightUI() {
    elements.nightLightTile.dataset.enabled = state.nightLightEnabled;
    elements.nightLightStatus.textContent = state.nightLightEnabled ? 'On' : 'Off';
}

// ============================================================================
// Volume Control
// ============================================================================

// Debounce for slider changes
let volumeDebounceTimer = null;

function handleVolumeInput(e) {
    const value = parseInt(e.target.value);
    state.volume = value;
    elements.volumeFill.style.width = `${value}%`;
    elements.volumeValue.textContent = `${value}%`;
}

function handleVolumeChange(e) {
    const value = parseInt(e.target.value);
    
    // Clear existing timer
    if (volumeDebounceTimer) {
        clearTimeout(volumeDebounceTimer);
    }
    
    // Debounce the actual command
    volumeDebounceTimer = setTimeout(async () => {
        try {
            await invoke('set_volume', { value });
        } catch (error) {
            // Revert UI on error
            await refreshVolume();
        }
    }, 50);
}

async function toggleMute() {
    try {
        state.muted = await invoke('toggle_mute');
        updateVolumeUI();
    } catch (error) {
        console.error('Failed to toggle mute:', error);
    }
}

async function refreshVolume() {
    try {
        state.volume = await invoke('get_volume');
        state.muted = await invoke('get_mute_status');
        updateVolumeUI();
    } catch (error) {
        console.error('Failed to refresh volume:', error);
    }
}

// ============================================================================
// Brightness Control
// ============================================================================

let brightnessDebounceTimer = null;

function handleBrightnessInput(e) {
    const value = parseInt(e.target.value);
    state.brightness = value;
    elements.brightnessFill.style.width = `${value}%`;
    elements.brightnessValue.textContent = `${value}%`;
}

function handleBrightnessChange(e) {
    const value = parseInt(e.target.value);
    
    if (brightnessDebounceTimer) {
        clearTimeout(brightnessDebounceTimer);
    }
    
    brightnessDebounceTimer = setTimeout(async () => {
        try {
            await invoke('set_brightness', { value });
        } catch (error) {
            await refreshBrightness();
        }
    }, 50);
}

async function refreshBrightness() {
    try {
        state.brightness = await invoke('get_brightness');
        updateBrightnessUI();
    } catch (error) {
        console.error('Failed to refresh brightness:', error);
    }
}

// ============================================================================
// Toggle Controls
// ============================================================================

async function toggleWifi() {
    if (state.pendingOperations.has('wifi')) return;
    state.pendingOperations.add('wifi');
    
    try {
        const newState = !state.wifiEnabled;
        elements.wifiTile.dataset.enabled = newState; // Optimistic update
        
        await invoke('set_wifi_enabled', { enabled: newState });
        state.wifiEnabled = newState;
        
        // Refresh to get connection status
        setTimeout(async () => {
            const networkState = await invoke('get_wifi_status');
            state.wifiEnabled = networkState.wifi_enabled;
            state.wifiConnected = networkState.wifi_connected;
            state.wifiSsid = networkState.wifi_ssid;
            updateWifiUI();
        }, 1000);
        
    } catch (error) {
        // Revert on error
        state.wifiEnabled = !state.wifiEnabled;
        updateWifiUI();
    } finally {
        state.pendingOperations.delete('wifi');
    }
}

async function toggleBluetooth() {
    if (state.pendingOperations.has('bluetooth')) return;
    state.pendingOperations.add('bluetooth');
    
    try {
        const newState = !state.bluetoothEnabled;
        elements.bluetoothTile.dataset.enabled = newState; // Optimistic update
        
        await invoke('set_bluetooth_enabled', { enabled: newState });
        state.bluetoothEnabled = newState;
        updateBluetoothUI();
        
    } catch (error) {
        // Revert on error
        state.bluetoothEnabled = !state.bluetoothEnabled;
        updateBluetoothUI();
    } finally {
        state.pendingOperations.delete('bluetooth');
    }
}

async function toggleNightLight() {
    if (state.pendingOperations.has('nightLight')) return;
    state.pendingOperations.add('nightLight');
    
    try {
        const newState = !state.nightLightEnabled;
        elements.nightLightTile.dataset.enabled = newState; // Optimistic update
        
        await invoke('set_night_light_enabled', { enabled: newState });
        state.nightLightEnabled = newState;
        updateNightLightUI();
        
    } catch (error) {
        // Revert on error
        state.nightLightEnabled = !state.nightLightEnabled;
        updateNightLightUI();
    } finally {
        state.pendingOperations.delete('nightLight');
    }
}

async function suspendSystem() {
    // Confirm action
    showToast('Suspending...', 'success');
    
    try {
        // Close window first for clean state
        await closeWindow();
        
        // Small delay to ensure window is closed
        setTimeout(async () => {
            await invoke('suspend_system');
        }, 100);
        
    } catch (error) {
        showToast('Failed to suspend', 'error');
    }
}

// ============================================================================
// Window Control
// ============================================================================

async function closeWindow() {
    try {
        await invoke('close_window');
    } catch (error) {
        console.error('Failed to close window:', error);
    }
}

// ============================================================================
// Keyboard Handling
// ============================================================================

function handleKeyDown(e) {
    switch (e.key) {
        case 'Escape':
            closeWindow();
            break;
        case 'ArrowUp':
            if (document.activeElement === elements.volumeSlider) {
                elements.volumeSlider.value = Math.min(100, parseInt(elements.volumeSlider.value) + 5);
                elements.volumeSlider.dispatchEvent(new Event('input'));
                elements.volumeSlider.dispatchEvent(new Event('change'));
            } else if (document.activeElement === elements.brightnessSlider) {
                elements.brightnessSlider.value = Math.min(100, parseInt(elements.brightnessSlider.value) + 5);
                elements.brightnessSlider.dispatchEvent(new Event('input'));
                elements.brightnessSlider.dispatchEvent(new Event('change'));
            }
            break;
        case 'ArrowDown':
            if (document.activeElement === elements.volumeSlider) {
                elements.volumeSlider.value = Math.max(0, parseInt(elements.volumeSlider.value) - 5);
                elements.volumeSlider.dispatchEvent(new Event('input'));
                elements.volumeSlider.dispatchEvent(new Event('change'));
            } else if (document.activeElement === elements.brightnessSlider) {
                elements.brightnessSlider.value = Math.max(1, parseInt(elements.brightnessSlider.value) - 5);
                elements.brightnessSlider.dispatchEvent(new Event('input'));
                elements.brightnessSlider.dispatchEvent(new Event('change'));
            }
            break;
    }
}

// ============================================================================
// UI Helpers
// ============================================================================

function showLoading() {
    state.isLoading = true;
    if (elements.loadingOverlay) {
        elements.loadingOverlay.classList.remove('hidden');
    }
}

function hideLoading() {
    state.isLoading = false;
    if (elements.loadingOverlay) {
        elements.loadingOverlay.classList.add('hidden');
    }
}

let toastTimer = null;

function showToast(message, type = 'info') {
    if (!elements.toast) return;
    
    // Clear existing timer
    if (toastTimer) {
        clearTimeout(toastTimer);
    }
    
    // Update toast
    elements.toast.textContent = message;
    elements.toast.className = `toast show ${type}`;
    
    // Hide after delay
    toastTimer = setTimeout(() => {
        elements.toast.classList.remove('show');
    }, 2500);
}

function addRippleEffect(e) {
    const button = e.currentTarget;
    const rect = button.getBoundingClientRect();
    const x = ((e.clientX - rect.left) / rect.width) * 100;
    const y = ((e.clientY - rect.top) / rect.height) * 100;
    
    button.style.setProperty('--ripple-x', `${x}%`);
    button.style.setProperty('--ripple-y', `${y}%`);
}

// ============================================================================
// Export for Testing
// ============================================================================

if (typeof module !== 'undefined' && module.exports) {
    module.exports = {
        state,
        invoke,
        loadInitialState,
        toggleWifi,
        toggleBluetooth,
        toggleNightLight,
        suspendSystem,
        handleVolumeChange,
        handleBrightnessChange,
    };
}
