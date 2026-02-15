use std::sync::{OnceLock, RwLock};

use device_query::{DeviceState, MouseState};

static DETAIL_HOTKEY_CACHE: OnceLock<RwLock<Option<i32>>> = OnceLock::new();
static DETECTION_HOTKEY_CACHE: OnceLock<RwLock<Option<i32>>> = OnceLock::new();
static CARD_DETECTION_HOTKEY_CACHE: OnceLock<RwLock<Option<i32>>> = OnceLock::new();
static TOGGLE_COLLAPSE_HOTKEY_CACHE: OnceLock<RwLock<Option<i32>>> = OnceLock::new();
static YOLO_HOTKEY_CACHE: OnceLock<RwLock<Option<i32>>> = OnceLock::new();

pub fn update_detail_hotkey_cache(val: Option<i32>) {
    let cache = DETAIL_HOTKEY_CACHE.get_or_init(|| RwLock::new(None));
    if let Ok(mut writer) = cache.write() {
        *writer = val;
    }
}

pub fn update_detection_hotkey_cache(val: Option<i32>) {
    let cache = DETECTION_HOTKEY_CACHE.get_or_init(|| RwLock::new(None));
    if let Ok(mut writer) = cache.write() {
        *writer = val;
    }
}

pub fn update_card_detection_hotkey_cache(val: Option<i32>) {
    let cache = CARD_DETECTION_HOTKEY_CACHE.get_or_init(|| RwLock::new(None));
    if let Ok(mut writer) = cache.write() {
        *writer = val;
    }
}

pub fn update_toggle_collapse_hotkey_cache(val: Option<i32>) {
    let cache = TOGGLE_COLLAPSE_HOTKEY_CACHE.get_or_init(|| RwLock::new(None));
    if let Ok(mut writer) = cache.write() {
        *writer = val;
    }
}

pub fn update_yolo_hotkey_cache(val: Option<i32>) {
    let cache = YOLO_HOTKEY_CACHE.get_or_init(|| RwLock::new(None));
    if let Ok(mut writer) = cache.write() {
        *writer = val;
    }
}

pub fn get_cached_detail_hotkey() -> Option<i32> {
    DETAIL_HOTKEY_CACHE
        .get()
        .and_then(|c| c.read().ok())
        .and_then(|r| *r)
}

pub fn get_cached_detection_hotkey() -> Option<i32> {
    DETECTION_HOTKEY_CACHE
        .get()
        .and_then(|c| c.read().ok())
        .and_then(|r| *r)
}

pub fn get_cached_card_detection_hotkey() -> Option<i32> {
    CARD_DETECTION_HOTKEY_CACHE
        .get()
        .and_then(|c| c.read().ok())
        .and_then(|r| *r)
}

pub fn get_cached_toggle_collapse_hotkey() -> Option<i32> {
    TOGGLE_COLLAPSE_HOTKEY_CACHE
        .get()
        .and_then(|c| c.read().ok())
        .and_then(|r| *r)
}

pub fn get_cached_yolo_hotkey() -> Option<i32> {
    YOLO_HOTKEY_CACHE
        .get()
        .and_then(|c| c.read().ok())
        .and_then(|r| *r)
}

pub fn is_key_pressed(key_code: i32, device_state: &DeviceState, mouse_state: &MouseState) -> bool {
    crate::platforms::hotkey::is_key_pressed(key_code, device_state, mouse_state)
}
