//! Application state management
//! 
//! Manages cached state for system settings to reduce redundant CLI calls.

use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Cache duration for system state (prevents excessive CLI calls)
const CACHE_DURATION: Duration = Duration::from_millis(500);

/// State entry with timestamp for cache invalidation
#[derive(Debug)]
struct CacheEntry<T> {
    value: T,
    timestamp: Instant,
}

impl<T: Clone> CacheEntry<T> {
    fn new(value: T) -> Self {
        Self {
            value,
            timestamp: Instant::now(),
        }
    }
    
    fn is_valid(&self) -> bool {
        self.timestamp.elapsed() < CACHE_DURATION
    }
    
    fn get(&self) -> Option<T> {
        if self.is_valid() {
            Some(self.value.clone())
        } else {
            None
        }
    }
}

/// Application state container
pub struct AppState {
    volume: Mutex<Option<CacheEntry<u8>>>,
    muted: Mutex<Option<CacheEntry<bool>>>,
    brightness: Mutex<Option<CacheEntry<u8>>>,
    wifi_enabled: Mutex<Option<CacheEntry<bool>>>,
    bluetooth_enabled: Mutex<Option<CacheEntry<bool>>>,
    night_light_enabled: Mutex<Option<CacheEntry<bool>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            volume: Mutex::new(None),
            muted: Mutex::new(None),
            brightness: Mutex::new(None),
            wifi_enabled: Mutex::new(None),
            bluetooth_enabled: Mutex::new(None),
            night_light_enabled: Mutex::new(None),
        }
    }
    
    pub fn get_cached_volume(&self) -> Option<u8> {
        self.volume.lock().ok()?.as_ref()?.get()
    }
    
    pub fn set_cached_volume(&self, value: u8) {
        if let Ok(mut cache) = self.volume.lock() {
            *cache = Some(CacheEntry::new(value));
        }
    }
    
    pub fn get_cached_muted(&self) -> Option<bool> {
        self.muted.lock().ok()?.as_ref()?.get()
    }
    
    pub fn set_cached_muted(&self, value: bool) {
        if let Ok(mut cache) = self.muted.lock() {
            *cache = Some(CacheEntry::new(value));
        }
    }
    
    pub fn get_cached_brightness(&self) -> Option<u8> {
        self.brightness.lock().ok()?.as_ref()?.get()
    }
    
    pub fn set_cached_brightness(&self, value: u8) {
        if let Ok(mut cache) = self.brightness.lock() {
            *cache = Some(CacheEntry::new(value));
        }
    }
    
    pub fn get_cached_wifi(&self) -> Option<bool> {
        self.wifi_enabled.lock().ok()?.as_ref()?.get()
    }
    
    pub fn set_cached_wifi(&self, value: bool) {
        if let Ok(mut cache) = self.wifi_enabled.lock() {
            *cache = Some(CacheEntry::new(value));
        }
    }
    
    pub fn get_cached_bluetooth(&self) -> Option<bool> {
        self.bluetooth_enabled.lock().ok()?.as_ref()?.get()
    }
    
    pub fn set_cached_bluetooth(&self, value: bool) {
        if let Ok(mut cache) = self.bluetooth_enabled.lock() {
            *cache = Some(CacheEntry::new(value));
        }
    }
    
    pub fn get_cached_night_light(&self) -> Option<bool> {
        self.night_light_enabled.lock().ok()?.as_ref()?.get()
    }
    
    pub fn set_cached_night_light(&self, value: bool) {
        if let Ok(mut cache) = self.night_light_enabled.lock() {
            *cache = Some(CacheEntry::new(value));
        }
    }
    
    pub fn invalidate_all(&self) {
        if let Ok(mut v) = self.volume.lock() { *v = None; }
        if let Ok(mut v) = self.muted.lock() { *v = None; }
        if let Ok(mut v) = self.brightness.lock() { *v = None; }
        if let Ok(mut v) = self.wifi_enabled.lock() { *v = None; }
        if let Ok(mut v) = self.bluetooth_enabled.lock() { *v = None; }
        if let Ok(mut v) = self.night_light_enabled.lock() { *v = None; }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
