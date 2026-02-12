use std::sync::{Arc, RwLock, OnceLock};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{State, Manager, Emitter};

#[cfg(target_os = "macos")]
use tauri::menu::{Menu, MenuItem};
#[cfg(target_os = "macos")]
use tauri::tray::{TrayIconBuilder, TrayIconEvent, MouseButton, MouseButtonState};

use serde::{Serialize, Deserialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use regex::Regex;
use std::io::{Read, BufRead, BufReader, Seek, SeekFrom, Write};
use std::fs::File;
use std::{time, panic};
// use tokio;
use chrono::Local;

// Windows 特定导入
#[cfg(target_os = "windows")]
use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongW, SetWindowLongW, SetWindowPos,
    EnumWindows, GetWindowTextW, IsWindowVisible, SetForegroundWindow, ShowWindow,
    GWL_EXSTYLE, GWL_STYLE,
    WS_EX_TOOLWINDOW, WS_EX_APPWINDOW, WS_EX_WINDOWEDGE, WS_EX_CLIENTEDGE, WS_EX_STATICEDGE,
    WS_EX_LAYERED, WS_EX_NOACTIVATE,
    WS_CAPTION, WS_THICKFRAME, WS_POPUP, WS_SYSMENU, WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_BORDER, WS_DLGFRAME,
    WS_VISIBLE, WS_CLIPSIBLINGS, WS_CLIPCHILDREN,
    SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SWP_NOACTIVATE, SWP_FRAMECHANGED, SW_RESTORE
};
#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute,
    DWMWA_USE_IMMERSIVE_DARK_MODE, DWMWA_BORDER_COLOR, DWMWA_CAPTION_COLOR, DWMWA_TEXT_COLOR
};
#[cfg(target_os = "windows")]
use windows::core::BOOL;
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{HWND, COLORREF, LPARAM};

use opencv::core::MatTraitConst;
use device_query::{DeviceQuery, DeviceState, MouseState};

#[cfg(not(target_os = "windows"))]
use device_query::Keycode;

// macOS 原生窗口设置（用于全屏覆盖）
// 注意: cocoa crate 已弃用，但 tauri-nspanel 仍依赖它
#[cfg(target_os = "macos")]
#[allow(deprecated)]
use cocoa::appkit::{NSWindow, NSWindowCollectionBehavior};
#[cfg(target_os = "macos")]
#[allow(deprecated)]
use cocoa::base::id;
// tauri-nspanel 用于创建可以显示在全屏应用上方的 NSPanel
#[cfg(target_os = "macos")]
use tauri_nspanel::WebviewWindowExt as NSPanelExt;

// ============== 跨平台热键检测 ==============
// Windows 虚拟键码常量（用于配置兼容性）
#[cfg(not(target_os = "windows"))]
const VK_RBUTTON: i32 = 2;    // 右键
#[cfg(not(target_os = "windows"))]
const VK_MENU: i32 = 18;      // Alt 键

// Global flag to indicate if card recognition is in progress (prevents hiding overlay)
pub static IS_RECOGNIZING: AtomicBool = AtomicBool::new(false);
// Store last recognition status to implement grace period
pub static LAST_RECOG_TIME: OnceLock<RwLock<Option<std::time::Instant>>> = OnceLock::new();

fn update_last_recog_time() {
    let cache = LAST_RECOG_TIME.get_or_init(|| RwLock::new(None));
    if let Ok(mut writer) = cache.write() {
        *writer = Some(std::time::Instant::now());
    }
}

// 全局静态变量存储详细信息显示热键
static DETAIL_HOTKEY_CACHE: OnceLock<RwLock<Option<i32>>> = OnceLock::new();
static DETECTION_HOTKEY_CACHE: OnceLock<RwLock<Option<i32>>> = OnceLock::new();
static CARD_DETECTION_HOTKEY_CACHE: OnceLock<RwLock<Option<i32>>> = OnceLock::new();
static TOGGLE_COLLAPSE_HOTKEY_CACHE: OnceLock<RwLock<Option<i32>>> = OnceLock::new();
static YOLO_HOTKEY_CACHE: OnceLock<RwLock<Option<i32>>> = OnceLock::new();

fn update_detail_hotkey_cache(val: Option<i32>) {
    let cache = DETAIL_HOTKEY_CACHE.get_or_init(|| RwLock::new(None));
    if let Ok(mut writer) = cache.write() { *writer = val; }
}

fn update_detection_hotkey_cache(val: Option<i32>) {
    let cache = DETECTION_HOTKEY_CACHE.get_or_init(|| RwLock::new(None));
    if let Ok(mut writer) = cache.write() { *writer = val; }
}

fn update_card_detection_hotkey_cache(val: Option<i32>) {
    let cache = CARD_DETECTION_HOTKEY_CACHE.get_or_init(|| RwLock::new(None));
    if let Ok(mut writer) = cache.write() { *writer = val; }
}

fn update_toggle_collapse_hotkey_cache(val: Option<i32>) {
    let cache = TOGGLE_COLLAPSE_HOTKEY_CACHE.get_or_init(|| RwLock::new(None));
    if let Ok(mut writer) = cache.write() { *writer = val; }
}

fn update_yolo_hotkey_cache(val: Option<i32>) {
    let cache = YOLO_HOTKEY_CACHE.get_or_init(|| RwLock::new(None));
    if let Ok(mut writer) = cache.write() { *writer = val; }
}

fn get_cached_detail_hotkey() -> Option<i32> {
    DETAIL_HOTKEY_CACHE.get().and_then(|c| c.read().ok()).and_then(|r| *r)
}

fn get_cached_detection_hotkey() -> Option<i32> {
    DETECTION_HOTKEY_CACHE.get().and_then(|c| c.read().ok()).and_then(|r| *r)
}

fn get_cached_card_detection_hotkey() -> Option<i32> {
    CARD_DETECTION_HOTKEY_CACHE.get().and_then(|c| c.read().ok()).and_then(|r| *r)
}

fn get_cached_toggle_collapse_hotkey() -> Option<i32> {
    TOGGLE_COLLAPSE_HOTKEY_CACHE.get().and_then(|c| c.read().ok()).and_then(|r| *r)
}

fn get_cached_yolo_hotkey() -> Option<i32> {
    YOLO_HOTKEY_CACHE.get().and_then(|c| c.read().ok()).and_then(|r| *r)
}

/// 跨平台按键检测
/// key_code: Windows 虚拟键码
/// device_state: device_query 状态
/// mouse_state: 鼠标状态
fn is_key_pressed(key_code: i32, _device_state: &DeviceState, _mouse_state: &MouseState) -> bool {
    #[cfg(target_os = "windows")]
    {
        unsafe { (GetAsyncKeyState(key_code) as i16) < 0 }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let device_state = _device_state;
        let mouse_state = _mouse_state;
        // 映射 Windows 虚拟键码到 device_query
        // Windows VK codes: VK_LBUTTON=1, VK_RBUTTON=2, VK_MBUTTON=4
        // device_query indices: 0=left, 1=middle, 2=right (varies by platform)
        match key_code {
            1 => mouse_state.button_pressed.get(0).copied().unwrap_or(false), // VK_LBUTTON -> index 0 (left)
            2 => mouse_state.button_pressed.get(2).copied().unwrap_or(false), // VK_RBUTTON -> index 2 (right)
            4 => mouse_state.button_pressed.get(1).copied().unwrap_or(false), // VK_MBUTTON -> index 1 (middle)
            18 => device_state.get_keys().contains(&Keycode::LAlt) || device_state.get_keys().contains(&Keycode::RAlt), // Alt
            81 => device_state.get_keys().contains(&Keycode::Q), // Q
            192 => device_state.get_keys().contains(&Keycode::Grave), // ` (反引号)
            _ => {
                // 尝试映射其他常见按键
                let keys = device_state.get_keys();
                match key_code {
                    65..=90 => {
                        // A-Z 字母键
                        let letter = (key_code as u8 - 65 + b'A') as char;
                        let keycode = match letter {
                            'A' => Some(Keycode::A), 'B' => Some(Keycode::B), 'C' => Some(Keycode::C),
                            'D' => Some(Keycode::D), 'E' => Some(Keycode::E), 'F' => Some(Keycode::F),
                            'G' => Some(Keycode::G), 'H' => Some(Keycode::H), 'I' => Some(Keycode::I),
                            'J' => Some(Keycode::J), 'K' => Some(Keycode::K), 'L' => Some(Keycode::L),
                            'M' => Some(Keycode::M), 'N' => Some(Keycode::N), 'O' => Some(Keycode::O),
                            'P' => Some(Keycode::P), 'Q' => Some(Keycode::Q), 'R' => Some(Keycode::R),
                            'S' => Some(Keycode::S), 'T' => Some(Keycode::T), 'U' => Some(Keycode::U),
                            'V' => Some(Keycode::V), 'W' => Some(Keycode::W), 'X' => Some(Keycode::X),
                            'Y' => Some(Keycode::Y), 'Z' => Some(Keycode::Z),
                            _ => None,
                        };
                        keycode.map(|k| keys.contains(&k)).unwrap_or(false)
                    }
                    _ => false
                }
            }
        }
    }
}



// ============== Windows 特定窗口样式函数 ==============
#[cfg(target_os = "windows")]
fn apply_dark_theme(window: &tauri::WebviewWindow) {
    if let Ok(hwnd) = window.hwnd() {
        unsafe {
            let handle = HWND(hwnd.0 as _);

            // 1. 开启沉浸式暗黑模式 (Win10 1809+ / Win11)
            let use_dark_mode = 1 as i32;
            let _ = DwmSetWindowAttribute(
                handle,
                DWMWA_USE_IMMERSIVE_DARK_MODE,
                &use_dark_mode as *const _ as *const _,
                std::mem::size_of::<i32>() as u32,
            );

            // 2. [Win11 专用] 强制设置标题栏和边框颜色为纯黑
            let black_color = COLORREF(0x000000);

            let _ = DwmSetWindowAttribute(
                handle,
                DWMWA_BORDER_COLOR,
                &black_color as *const _ as *const _,
                std::mem::size_of::<COLORREF>() as u32,
            );

            let _ = DwmSetWindowAttribute(
                handle,
                DWMWA_CAPTION_COLOR,
                &black_color as *const _ as *const _,
                std::mem::size_of::<COLORREF>() as u32,
            );

            // 3. 将【标题栏文字】染成纯黑 (实现隐身)
            let _ = DwmSetWindowAttribute(
                handle,
                DWMWA_TEXT_COLOR,
                &black_color as *const _ as *const _,
                std::mem::size_of::<COLORREF>() as u32,
            );
        }
    }
}

#[cfg(target_os = "windows")]
#[allow(dead_code)]
fn apply_pure_overlay_style(window: &tauri::WebviewWindow) {
    apply_dark_theme(window);

    if let Ok(hwnd) = window.hwnd() {
        unsafe {
            let handle = HWND(hwnd.0 as _);

            let current_style = GetWindowLongW(handle, GWL_STYLE) as u32;
            let mut new_style = current_style & !(
                WS_CAPTION.0 |
                WS_THICKFRAME.0 |
                WS_MINIMIZEBOX.0 |
                WS_MAXIMIZEBOX.0 |
                WS_SYSMENU.0 |
                WS_BORDER.0 |
                WS_DLGFRAME.0
            );
            new_style |= WS_POPUP.0 | WS_VISIBLE.0 | WS_CLIPSIBLINGS.0 | WS_CLIPCHILDREN.0;
            SetWindowLongW(handle, GWL_STYLE, new_style as i32);

            let current_ex_style = GetWindowLongW(handle, GWL_EXSTYLE) as u32;
            let mut new_ex_style = current_ex_style & !(
                WS_EX_APPWINDOW.0 |
                WS_EX_WINDOWEDGE.0 |
                WS_EX_CLIENTEDGE.0 |
                WS_EX_STATICEDGE.0
            );
            new_ex_style |= WS_EX_TOOLWINDOW.0 | WS_EX_LAYERED.0;
            SetWindowLongW(handle, GWL_EXSTYLE, new_ex_style as i32);

            let _ = SetWindowPos(
                handle,
                None,
                0, 0, 0, 0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED
            );
        }
    }
}

#[cfg(target_os = "windows")]
fn apply_main_window_style(window: &tauri::WebviewWindow) {
    apply_dark_theme(window);

    if let Ok(hwnd) = window.hwnd() {
        unsafe {
            let handle = HWND(hwnd.0 as _);

            let current_style = GetWindowLongW(handle, GWL_STYLE) as u32;
            let mut new_style = current_style & !(
                WS_CAPTION.0 |
                WS_SYSMENU.0 |
                WS_MINIMIZEBOX.0 |
                WS_MAXIMIZEBOX.0
            );
            new_style |= WS_POPUP.0 | WS_THICKFRAME.0;
            SetWindowLongW(handle, GWL_STYLE, new_style as i32);

            let current_ex_style = GetWindowLongW(handle, GWL_EXSTYLE) as u32;
            // 确保主窗口显示在任务栏：移除 TOOLWINDOW，添加 APPWINDOW
            let mut new_ex_style = current_ex_style & !(WS_EX_TOOLWINDOW.0);
            new_ex_style |= WS_EX_APPWINDOW.0 | WS_EX_LAYERED.0;
            SetWindowLongW(handle, GWL_EXSTYLE, new_ex_style as i32);

            let _ = SetWindowPos(
                handle,
                None,
                0, 0, 0, 0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED
            );
        }
    }
}

// ============== macOS/其他平台的空实现 ==============
#[cfg(not(target_os = "windows"))]
#[allow(dead_code)]
fn apply_dark_theme(_window: &tauri::WebviewWindow) {
    // macOS 使用系统主题，无需特殊处理
}

#[cfg(not(target_os = "windows"))]
#[allow(dead_code)]
fn apply_pure_overlay_style(_window: &tauri::WebviewWindow) {
    // macOS 通过 Tauri 配置处理
}

#[cfg(not(target_os = "windows"))]
#[allow(dead_code)]
fn apply_main_window_style(_window: &tauri::WebviewWindow) {
    // macOS 通过 Tauri 配置处理
}

/// macOS: 使用 NSPanel 设置窗口可以覆盖在全屏应用上方
/// NSPanel 与 NSWindowStyleMaskNonActivatingPanel 可以显示在全屏应用上方
/// 这是 Discord overlay、Spotlight 等应用使用的技术
#[cfg(target_os = "macos")]
#[allow(deprecated)] // tauri-nspanel 使用已弃用的 cocoa API
fn setup_macos_fullscreen_overlay(window: &tauri::WebviewWindow) {
    // 使用 tauri-nspanel 将窗口转换为 NSPanel
    // NSPanel 可以显示在全屏应用上方，而普通 NSWindow 不行
    // 参考: https://github.com/tauri-apps/tauri/issues/9556
    match window.to_panel() {
        Ok(panel) => {
            // 关键：设置 NSWindowStyleMaskNonActivatingPanel (1 << 7 = 128)
            // 这防止面板激活所属应用，是全屏覆盖的关键
            // 参考: https://developer.apple.com/documentation/appkit/nspanel/stylemask/nonactivatingpanel
            const NS_WINDOW_STYLE_MASK_NON_ACTIVATING_PANEL: i32 = 1 << 7; // 128, NSPanel.StyleMask.nonactivatingPanel
            panel.set_style_mask(NS_WINDOW_STYLE_MASK_NON_ACTIVATING_PANEL);

            // 设置 Panel 级别为 NSMainMenuWindowLevel + 1 (24 + 1 = 25)
            // 根据 lumehq/lume 的实现，这个级别足够高以显示在全屏上方
            // 参考: https://developer.apple.com/documentation/appkit/nswindow/level/mainmenu
            const NS_MAIN_MENU_WINDOW_LEVEL: i32 = 24; // NSWindow.Level.mainMenu raw value
            panel.set_level(NS_MAIN_MENU_WINDOW_LEVEL + 1);

            // 设置 collection behavior
            // - CanJoinAllSpaces: 出现在所有桌面空间
            // - FullScreenAuxiliary: 可以与全屏窗口一起显示（关键！）
            // - Stationary: 固定位置
            panel.set_collection_behaviour(
                tauri_nspanel::cocoa::appkit::NSWindowCollectionBehavior::NSWindowCollectionBehaviorCanJoinAllSpaces
                | tauri_nspanel::cocoa::appkit::NSWindowCollectionBehavior::NSWindowCollectionBehaviorFullScreenAuxiliary
                | tauri_nspanel::cocoa::appkit::NSWindowCollectionBehavior::NSWindowCollectionBehaviorStationary
            );

            // 设置为浮动面板
            panel.set_floating_panel(true);

            // 设置窗口不隐藏在失去焦点时
            panel.set_hides_on_deactivate(false);

            println!("[macOS] Overlay converted to NSPanel with NonActivatingPanel style, level=25");
        }
        Err(e) => {
            // 如果转换失败，回退到传统 NSWindow 方法
            println!("[macOS] Failed to convert to NSPanel: {:?}, falling back to NSWindow", e);
            fallback_setup_macos_overlay(window);
        }
    }
}

/// macOS: 回退方案 - 使用传统 NSWindow 设置
#[cfg(target_os = "macos")]
#[allow(deprecated)] // cocoa crate 已弃用
#[allow(unexpected_cfgs)] // msg_send! macro uses cargo-clippy cfg
fn fallback_setup_macos_overlay(window: &tauri::WebviewWindow) {
    use objc::{msg_send, sel, sel_impl};
    use cocoa::base::BOOL;

    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGWindowLevelForKey(key: i32) -> i32;
    }
    // 参考: https://developer.apple.com/documentation/coregraphics/cgwindowlevelkey/kCGMaximumWindowLevelKey
    const K_CG_MAXIMUM_WINDOW_LEVEL_KEY: i32 = 14; // CGWindowLevelKey.maximumWindow raw value

    if let Ok(ns_window) = window.ns_window() {
        unsafe {
            let ns_win: id = ns_window as id;

            let behavior = NSWindowCollectionBehavior::NSWindowCollectionBehaviorCanJoinAllSpaces
                | NSWindowCollectionBehavior::NSWindowCollectionBehaviorStationary
                | NSWindowCollectionBehavior::NSWindowCollectionBehaviorIgnoresCycle
                | NSWindowCollectionBehavior::NSWindowCollectionBehaviorFullScreenAuxiliary;
            ns_win.setCollectionBehavior_(behavior);

            let max_level = CGWindowLevelForKey(K_CG_MAXIMUM_WINDOW_LEVEL_KEY);
            ns_win.setLevel_(max_level as i64);

            #[allow(clippy::let_unit_value)]
            let _: () = msg_send![ns_win, setHidesOnDeactivate: false as BOOL];

            println!("[macOS] Overlay window (fallback) configured with maximum level: {}", max_level);
        }
    }
}

use crate::monster_recognition::YoloDetection;

pub mod monster_recognition;

#[derive(Clone, serde::Deserialize, serde::Serialize)]
struct BoundsRect {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

struct OverlayState(Arc<std::sync::Mutex<Vec<BoundsRect>>>);

#[tauri::command]
fn update_overlay_bounds(bounds: Vec<BoundsRect>, state: State<'_, OverlayState>) {
    let mut bounds_state = state.0.lock().unwrap();
    *bounds_state = bounds.clone();
    // 减少日志输出频率
}

static YOLO_SCAN_RESULTS: OnceLock<RwLock<Vec<YoloDetection>>> = OnceLock::new();
static YOLO_SCAN_IMAGE: OnceLock<RwLock<Option<image::DynamicImage>>> = OnceLock::new();
static YOLO_WINDOW_OFFSET: OnceLock<RwLock<(i32, i32)>> = OnceLock::new();
static ABORT_YOLO: AtomicBool = AtomicBool::new(false);

fn get_yolo_scan_results() -> &'static RwLock<Vec<YoloDetection>> {
    YOLO_SCAN_RESULTS.get_or_init(|| RwLock::new(Vec::new()))
}

fn get_yolo_scan_image() -> &'static RwLock<Option<image::DynamicImage>> {
    YOLO_SCAN_IMAGE.get_or_init(|| RwLock::new(None))
}

fn get_yolo_window_offset() -> &'static RwLock<(i32, i32)> {
    YOLO_WINDOW_OFFSET.get_or_init(|| RwLock::new((0, 0)))
}

#[tauri::command]
fn abort_yolo_scan() {
    println!("[YOLO] Abort requested.");
    ABORT_YOLO.store(true, Ordering::SeqCst);
}

#[tauri::command]
fn set_show_yolo_monitor(app: tauri::AppHandle, show: bool) -> Result<(), String> {
    println!("[YOLO Monitor] Setting visibility to: {}", show);
    
    // 直接控制 yolo-monitor 窗口的显示/隐藏
    if let Some(window) = app.get_webview_window("yolo-monitor") {
        if show {
            match window.show() {
                Ok(_) => println!("[YOLO Monitor] Window shown successfully"),
                Err(e) => println!("[YOLO Monitor] Failed to show window: {}", e),
            }
        } else {
            match window.hide() {
                Ok(_) => println!("[YOLO Monitor] Window hidden successfully"),
                Err(e) => println!("[YOLO Monitor] Failed to hide window: {}", e),
            }
        }
    } else {
        println!("[YOLO Monitor] WARNING: Window not found!");
    }
    
    // Persist preference
    let mut state = load_state();
    state.show_yolo_monitor = show;
    save_state(&state);
    Ok(())
}

#[tauri::command]
#[allow(non_snake_case)]
async fn trigger_yolo_scan(app: tauri::AppHandle, useGpu: bool) -> Result<usize, String> {
    // Reset abort flag
    ABORT_YOLO.store(false, Ordering::SeqCst);
    
    // Frontend and backend now use canonical `useGpu` parameter
    let use_gpu_flag = useGpu;
    use xcap::{Window, Monitor};
    
    // Notify frontend scan started
    let _ = app.emit("yolo-scan-start", ());

    let result = (|| -> Result<usize, String> {
        let resources_path = app.path().resource_dir().map_err(|e| e.to_string())?;
        let model_path = resources_path.join("resources").join("models").join("best.onnx");

        if ABORT_YOLO.load(Ordering::SeqCst) { return Err("Aborted".into()); }

        // 1. 获取 The Bazaar 窗口截图，如果未找到则使用主屏幕截图
        let windows = Window::all().map_err(|e| e.to_string())?;
        
        if ABORT_YOLO.load(Ordering::SeqCst) { return Err("Aborted".into()); }

        // 优先寻找游戏窗口
        let target_window = windows.iter().find(|w| {
            let title = w.title().to_lowercase();
            let app_name = w.app_name().to_lowercase();
            let is_bazaar = title.contains("the bazaar") || app_name.contains("the bazaar") || 
                            title.contains("thebazaar") || app_name.contains("thebazaar");
            is_bazaar && !title.contains("bazaarhelper")
        });

        let (screenshot, window_x, window_y) = if let Some(w) = target_window {
            println!("[YOLO] Found Game Window: '{}' at ({},{})", w.title(), w.x(), w.y());
            let wx = w.x();
            let wy = w.y();
            (w.capture_image().map_err(|e| e.to_string())?, wx, wy)
        } else {
            println!("[YOLO] The Bazaar window not found, falling back to primary monitor scan.");
            let monitors = Monitor::all().map_err(|e| e.to_string())?;
            let monitor = monitors.into_iter().next().ok_or("No monitor found")?;
            (monitor.capture_image().map_err(|e| e.to_string())?, 0, 0)
        };

        if ABORT_YOLO.load(Ordering::SeqCst) { return Err("Aborted".into()); }

        let img = image::DynamicImage::ImageRgba8(screenshot);
        
        // 2. YOLO 识别
        println!("[YOLO] Starting manual scan with GPU acceleration: {}...", use_gpu_flag);
        let detections = monster_recognition::run_yolo_inference(&img, &model_path, use_gpu_flag)?;
        
        if ABORT_YOLO.load(Ordering::SeqCst) { return Err("Aborted".into()); }

        println!("[YOLO] Scan complete. Found {} objects.", detections.len());
        
        // ... (rest of the debug printing and saving)
        // (existing code)
        // 3. 保存结果和窗口偏移量
        {
            let mut results = get_yolo_scan_results().write().unwrap();
            *results = detections.clone();
        }
        {
            let mut saved_img = get_yolo_scan_image().write().unwrap();
            *saved_img = Some(img);
        }
        {
            let mut offset = get_yolo_window_offset().write().unwrap();
            *offset = (window_x, window_y);
            println!("[YOLO] Saved window offset: ({}, {})", window_x, window_y);
        }
        
        Ok(detections.len())
    })();

    match &result {
        Ok(count) => {
            println!("[YOLO] Scan succeeded with {} detections", count);
            let _ = app.emit("yolo-scan-end", ());
        }
        Err(e) if e == "Aborted" => {
            println!("[YOLO] Scan aborted by user.");
            let _ = app.emit("yolo-scan-end", ()); // Still notify end so frontend can reset if needed
        }
        Err(e) => {
            log_to_file(&format!("[YOLO Error] {}", e));
            let _ = app.emit("scan-error", e.clone());
        }
    }

    result
}

#[tauri::command]
async fn handle_overlay_right_click(app: tauri::AppHandle, x: i32, y: i32) -> Result<Option<serde_json::Value>, String> {
    use image::GenericImageView;
    let detections = get_yolo_scan_results().read().unwrap().clone();
    let img_opt = get_yolo_scan_image().read().unwrap().clone();
    
    // 动态获取游戏窗口位置，如果找不到则使用保存的偏移量
    let (window_x, window_y, window_logical_width, window_logical_height) = {
        let game_window = xcap::Window::all()
            .ok()
            .and_then(|windows| {
                windows.into_iter().find(|w| {
                    let title = w.title().to_lowercase();
                    let app_name = w.app_name().to_lowercase();
                    title.contains("the bazaar") || app_name.contains("the bazaar") || 
                    title.contains("thebazaar") || app_name.contains("thebazaar")
                })
            });
        
        if let Some(window) = game_window {
            (window.x(), window.y(), window.width(), window.height())
        } else {
            // 如果找不到游戏窗口，使用之前保存的偏移量
            let (x, y) = *get_yolo_window_offset().read().unwrap();
            (x, y, 0, 0)
        }
    };
    
    if img_opt.is_none() {
        return Ok(None);
    }
    let img = img_opt.unwrap();
    let (img_w, img_h) = img.dimensions();
    
    // 将屏幕坐标转换为相对窗口坐标
    let rel_x_logical = x - window_x;
    let rel_y_logical = y - window_y;
    
    // 跨平台 DPI 缩放修正：检测图像物理分辨率 vs 逻辑坐标
    // 截图返回物理像素，但鼠标坐标是逻辑像素
    // 通过窗口的逻辑尺寸和图像的物理尺寸计算缩放因子
    let scale_factor = if window_logical_width > 0 && window_logical_height > 0 {
        let scale_x = img_w as f32 / window_logical_width as f32;
        let scale_y = img_h as f32 / window_logical_height as f32;
        // 取平均值，通常两个方向的缩放比例应该相同
        (scale_x + scale_y) / 2.0
    } else {
        // 降级方案：根据图像大小估算
        #[cfg(target_os = "macos")]
        {
            if img_w > 1920 { 2.0 } else { 1.0 }
        }
        #[cfg(not(target_os = "macos"))]
        {
            // Windows: 常见的DPI缩放比例
            if img_w > 3000 { 1.5 } else { 1.0 }
        }
    };
    
    let rel_x = (rel_x_logical as f32 * scale_factor) as i32;
    let rel_y = (rel_y_logical as f32 * scale_factor) as i32;
    
    println!("[YOLO Click] Screen coords: ({}, {}), Window offset: ({}, {}), Window size: {}x{}, Logical relative: ({}, {}), Scale: {:.2}, Physical relative: ({}, {})", 
             x, y, window_x, window_y, window_logical_width, window_logical_height, rel_x_logical, rel_y_logical, scale_factor, rel_x, rel_y);
    println!("[DEBUG] Image dimensions: {}x{}, Total detections: {}", img_w, img_h, detections.len());
    
    for (i, d) in detections.iter().enumerate() {
        println!("[DEBUG] Detection {}: class={}, bounds=[{},{},{},{}], size={}x{}", 
                 i, d.class_id, d.x1, d.y1, d.x2, d.y2, d.x2 - d.x1, d.y2 - d.y1);
    }

    // Check for any detection hit (使用物理像素坐标)
    let target_detection = detections.iter().find(|d| {
        rel_x >= d.x1 && rel_x <= d.x2 && rel_y >= d.y1 && rel_y <= d.y2
    });

    if let Some(det) = target_detection {
        println!("[YOLO Click] Clicked on Class {} at [{}, {}, {}, {}]", det.class_id, det.x1, det.y1, det.x2, det.y2);
        
        let w = (det.x2 - det.x1).max(50) as u32;
        let h = (det.y2 - det.y1).max(50) as u32;
        let crop_x = det.x1.max(0) as u32;
        let crop_y = det.y1.max(0) as u32;
        
        let (img_w, img_h) = img.dimensions();
        let final_w = if crop_x + w > img_w { img_w - crop_x } else { w };
        let final_h = if crop_y + h > img_h { img_h - crop_y } else { h };
        
        let cropped = img.crop_imm(crop_x, crop_y, final_w, final_h);
        let scene_desc = monster_recognition::extract_features_from_dynamic_image(&cropped, 1000)
            .map_err(|e| e.to_string())?;
            
        if scene_desc.empty() {
            return Ok(None);
        }

        // names: ['day', 'event', 'item', 'monstericon', 'randomicon', 'shopicon', 'skill']
        // 0: day, 1: event, 2: item, 3: monstericon, 4: randomicon, 5: shopicon, 6: skill

        if det.class_id == 2 || det.class_id == 6 {
            // Item (2) or Skill (6) -> Card Recognition
            // 根据宽高比判断卡牌size类型
            let card_width = (det.x2 - det.x1) as f32;
            let card_height = (det.y2 - det.y1) as f32;
            let aspect_ratio = card_width / card_height;
            
            // 判断卡牌尺寸类型
            // 中型：宽高比接近1:1 (0.85 - 1.15 范围内)
            // 大型：宽明显大于高 (aspect_ratio > 1.15)
            // 小型：高明显大于宽 (aspect_ratio < 0.85)
            let card_size = if aspect_ratio >= 0.85 && aspect_ratio <= 1.15 {
                "Medium"
            } else if aspect_ratio > 1.15 {
                "Large"
            } else {
                "Small"
            };
            
            println!("[YOLO Click] Card dimensions: {}x{}, aspect_ratio: {:.2}, detected size: {}", 
                     card_width, card_height, aspect_ratio, card_size);
            
            // 使用按size分类的匹配函数
            let match_result = monster_recognition::match_card_by_size(&scene_desc, card_size);
            match match_result {
                Ok(Some(cards)) => {
                     let card_list = cards.as_array().unwrap();
                     if !card_list.is_empty() {
                         let card_id = card_list[0]["id"].as_str().unwrap_or("").to_string();
                         println!("[YOLO Click] Card matched: {}", card_id);
                         let db_state = app.state::<DbState>();
                         if let Some(info) = get_item_info_internal(&db_state, card_id.clone()).await {
                             return Ok(Some(serde_json::json!({ "type": "item", "data": info })));
                         } else {
                             println!("[YOLO Click] Card info not found in DB for id: {}", card_id);
                             println!("[YOLO Click] Checking if DB is loaded...");
                             let items_db = db_state.items.read().unwrap();
                             println!("[YOLO Click] DB has {} items, id_map has {} entries", 
                                      items_db.list.len(), items_db.id_map.len());
                         }
                     } else {
                         println!("[YOLO Click] Card match returned empty list");
                     }
                },
                Ok(None) => println!("[YOLO Click] No card descriptors matched in {} category", card_size),
                Err(e) => println!("[YOLO Click] Card matching error: {}", e),
            }
        } else if det.class_id == 1 {
            // Event (1) -> Check for Monster Icon (3) overlap
            // Logic: Is there any Icon (3) inside this Event (1) with > 50% area overlap (relative to Icon)?
            let monster_icons: Vec<&YoloDetection> = detections.iter().filter(|d| d.class_id == 3).collect();
            let mut is_monster = false;
            
            for icon in monster_icons {
                // Calculate Intersection
                let ix1 = det.x1.max(icon.x1);
                let iy1 = det.y1.max(icon.y1);
                let ix2 = det.x2.min(icon.x2);
                let iy2 = det.y2.min(icon.y2);
                
                let i_area = (ix2 - ix1).max(0) * (iy2 - iy1).max(0);
                let icon_full_area = (icon.x2 - icon.x1) * (icon.y2 - icon.y1);
                
                if icon_full_area > 0 && (i_area as f32 / icon_full_area as f32) > 0.5 {
                    is_monster = true;
                    break;
                }
            }
            
            if is_monster {
                let monster_match = monster_recognition::match_monster_descriptors_from_mat(&scene_desc)?;
                if let Some(monster_name) = monster_match {
                    let db_state = app.state::<DbState>();
                    let monsters = db_state.monsters.read().unwrap();
                    if let Some(m) = monsters.get(&monster_name) {
                        return Ok(Some(serde_json::json!({ "type": "monster", "data": m })));
                    }
                }
            } else {
                // Pure event (no monster icon) -> Event Recognition
                let event_match = monster_recognition::match_event_descriptors_from_mat(&scene_desc)?;
                if let Some(event_id) = event_match {
                    // 读取 event_encounters.json 获取完整事件数据
                    let event_json_path = app.path().resolve("resources/event_encounters.json", tauri::path::BaseDirectory::Resource)
                        .map_err(|e| format!("Failed to resolve event_encounters.json: {}", e))?;
                    
                    if let Ok(json_data) = std::fs::read_to_string(&event_json_path) {
                        if let Ok(events) = serde_json::from_str::<Vec<serde_json::Value>>(&json_data) {
                            if let Some(event) = events.iter().find(|e| e.get("Id").and_then(|v| v.as_str()) == Some(&event_id)) {
                                return Ok(Some(serde_json::json!({ "type": "event", "data": event })));
                            }
                        }
                    }
                }
            }
        } else {
             // Fallback or other classes (e.g. 3 directly?)
             // Monster recognition for direct MonsterIcon (3) or others if needed
             if det.class_id == 3 {
                 let monster_match = monster_recognition::match_monster_descriptors_from_mat(&scene_desc)?;
                 if let Some(monster_name) = monster_match {
                     let db_state = app.state::<DbState>();
                     let monsters = db_state.monsters.read().unwrap();
                     if let Some(m) = monsters.get(&monster_name) {
                         return Ok(Some(serde_json::json!({ "type": "monster", "data": m })));
                     }
                 }
             }
        }
    }
    Ok(None)
}

async fn get_item_info_internal(state: &DbState, id: String) -> Option<ItemData> {
    let db = state.items.read().unwrap();
    if let Some(&idx) = db.id_map.get(&id) {
        return Some(db.list[idx].clone());
    }
    None
}

// --- Logger Helper ---
pub fn log_to_file(msg: &str) {
    if let Ok(mut exe_path) = std::env::current_exe() {
        exe_path.pop();
        exe_path.push("app_debug.txt");
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(exe_path) {
            let _ = writeln!(f, "[{}] {}", get_time_str(), msg);
            let _ = f.flush();
        }
    }
}

fn get_time_str() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string()
}

pub fn set_panic_hook() {
    panic::set_hook(Box::new(|panic_info| {
        let payload = panic_info.payload();
        let message = if let Some(s) = payload.downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = payload.downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic".to_string()
        };

        let location = panic_info.location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown location".to_string());
        
        log_to_file(&format!("FATAL PANIC at {}: {}", location, message));
        
        // Output to stderr as well
        eprintln!("FATAL PANIC at {}: {}", location, message);
    }));
}

pub fn log_system_info(app_handle: &tauri::AppHandle) {
    log_to_file("--- System Info ---");
    log_to_file(&format!("OS: {}", std::env::consts::OS));
    log_to_file(&format!("ARCH: {}", std::env::consts::ARCH));
    
    if let Ok(exe_path) = std::env::current_exe() {
        log_to_file(&format!("EXE Path: {:?}", exe_path));
    }
    
    if let Ok(cwd) = std::env::current_dir() {
        log_to_file(&format!("CWD: {:?}", cwd));
    }

    log_to_file(&format!("Resource Dir: {:?}", app_handle.path().resource_dir().ok()));
    log_to_file(&format!("App Config Dir: {:?}", app_handle.path().app_config_dir().ok()));
    log_to_file(&format!("App Local Data Dir: {:?}", app_handle.path().app_local_data_dir().ok()));
    
    // Log environment variables that might affect execution
    for var in ["PATH", "USERNAME", "APPDATA", "LOCALAPPDATA"] {
        if let Ok(val) = std::env::var(var) {
            log_to_file(&format!("Env {}: {}", var, val));
        }
    }

    let lp = get_log_path();
    log_to_file(&format!("Game Log Path: {:?}", lp));
    log_to_file(&format!("Game Log Exists: {}", lp.exists()));
    
    log_to_file("-------------------");
}

// --- Data Models ---
// 野怪识别区域数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonsterRegion {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

// 野怪识别校准数据
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MonsterCalibration {
    pub regions: Vec<MonsterRegion>, // 三个区域，按照x坐标从左到右排序
    pub game_window_width: u32,
    pub game_window_height: u32,
    pub screen_width: u32,
    pub screen_height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistentState {
    pub day: u32,
    pub inst_to_temp: HashMap<String, String>,
    pub current_hand: HashSet<String>,
    pub current_stash: HashSet<String>,
    #[serde(default)]
    pub detection_hotkey: Option<i32>,
    #[serde(default)]
    pub card_detection_hotkey: Option<i32>,
    #[serde(default)]
    pub toggle_collapse_hotkey: Option<i32>,
    #[serde(default)]
    pub yolo_hotkey: Option<i32>,
    #[serde(default)]
    pub detail_display_hotkey: Option<i32>,
    #[serde(default = "default_show_yolo_monitor")]
    pub show_yolo_monitor: bool,
    #[serde(default)]
    pub detail_popup_x: Option<i32>,
    #[serde(default)]
    pub detail_popup_y: Option<i32>,
    #[serde(default)]
    pub detail_popup_width: Option<u32>,
    #[serde(default)]
    pub detail_popup_height: Option<u32>,
    #[serde(default)]
    pub monster_calibration: Option<MonsterCalibration>,
    // Main Window Geometry Persistence
    #[serde(default)]
    pub main_window_x: Option<i32>,
    #[serde(default)]
    pub main_window_y: Option<i32>,
    #[serde(default)]
    pub main_window_width: Option<u32>,
    #[serde(default)]
    pub main_window_height: Option<u32>,
}

// 跨平台虚拟键常量
const VK_RBUTTON_CODE: i32 = 2;   // 鼠标右键 (Windows VK_RBUTTON = 0x02)
const VK_MENU_CODE: i32 = 18;     // Alt 键 (Windows VK_MENU = 0x12)

impl Default for PersistentState {
    fn default() -> Self {
        Self {
            day: 1,
            inst_to_temp: HashMap::new(),
            current_hand: HashSet::new(),
            current_stash: HashSet::new(),
            detection_hotkey: Some(VK_RBUTTON_CODE),
            card_detection_hotkey: Some(VK_MENU_CODE),
            toggle_collapse_hotkey: Some(192), // Default: ~ key (Backtick) (VK_OEM_3 is 192 usually, or 0xC0)
            yolo_hotkey: Some(81), // Default: Q key (VK_Q = 81)
            detail_display_hotkey: Some(VK_RBUTTON_CODE), // Default: Right mouse button
            show_yolo_monitor: true,
            detail_popup_x: None,
            detail_popup_y: None,
            detail_popup_width: None,
            detail_popup_height: None,
            monster_calibration: None,
            main_window_x: None,
            main_window_y: None,
            main_window_width: None,
            main_window_height: None,
        }
    }
}

fn default_show_yolo_monitor() -> bool { true }
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RawSkill {
    pub en: Option<String>,
    pub cn: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RawItem {
    pub id: String,
    pub name_en: Option<String>,
    pub name_cn: Option<String>,
    pub starting_tier: Option<String>,
    pub available_tiers: Option<String>,
    pub heroes: Option<String>,
    pub tags: Option<String>,
    pub hidden_tags: Option<String>,
    pub size: Option<String>,
    pub cooldown: Option<f32>,
    pub cooldown_tiers: Option<String>,
    pub damage: Option<i32>,
    pub damage_tiers: Option<String>,
    pub heal: Option<i32>,
    pub heal_tiers: Option<String>,
    pub shield: Option<i32>,
    pub shield_tiers: Option<String>,
    pub ammo: Option<i32>,
    pub ammo_tiers: Option<String>,
    pub crit: Option<i32>,
    pub crit_tiers: Option<String>,
    pub multicast: Option<i32>,
    pub multicast_tiers: Option<String>,
    pub burn: Option<i32>,
    pub burn_tiers: Option<String>,
    pub poison: Option<i32>,
    pub poison_tiers: Option<String>,
    pub regen: Option<i32>,
    pub regen_tiers: Option<String>,
    pub lifesteal: Option<i32>,
    pub lifesteal_tiers: Option<String>,
    pub skills: Option<Vec<RawSkill>>,
    pub skills_passive: Option<Vec<RawSkill>>,
    pub quests: Option<serde_json::Value>,
    pub descriptions: Option<Vec<RawSkill>>,
    pub enchantments: Option<serde_json::Value>,
    pub image: Option<String>,
    #[serde(default)]
    pub description_cn: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ItemData {
    #[serde(alias = "id")]
    pub uuid: String,
    pub name: String,
    pub name_cn: String,
    pub tier: String,
    pub available_tiers: String,
    pub tags: String,
    pub hidden_tags: String,
    pub size: Option<String>,
    pub processed_tags: Vec<String>,
    pub heroes: Option<String>,
    pub cooldown: Option<f32>,
    pub cooldown_tiers: String,
    pub damage_tiers: String,
    pub damage: Option<i32>,
    pub heal_tiers: String,
    pub heal: Option<i32>,
    pub shield_tiers: String,
    pub shield: Option<i32>,
    pub ammo_tiers: String,
    pub ammo: Option<i32>,
    pub crit_tiers: String,
    pub crit: Option<i32>,
    pub multicast_tiers: String,
    pub multicast: Option<i32>,
    pub burn_tiers: String,
    pub burn: Option<i32>,
    pub poison_tiers: String,
    pub poison: Option<i32>,
    pub regen_tiers: String,
    pub regen: Option<i32>,
    pub lifesteal_tiers: String,
    pub lifesteal: Option<i32>,
    pub skills: Vec<SkillText>,
    pub skills_passive: Option<Vec<SkillText>>,
    pub quests: Option<serde_json::Value>,
    pub enchantments: Vec<String>,
    pub description: String,
    pub instance_id: Option<String>,
    pub description_cn: Option<String>, // Added this
    pub image: Option<String>, // Added this
}

impl From<RawItem> for ItemData {
    fn from(raw: RawItem) -> Self {
        let name_en = raw.name_en.clone().unwrap_or_else(|| "Unknown".to_string());
        let name_cn = raw.name_cn.clone().unwrap_or_else(|| name_en.clone());

        let processed_tags = raw.tags.as_deref().unwrap_or_default()
            .split('|')
            .map(|s| {
                let part = s.trim();
                // Pick the last part after / if it exists
                part.split(" / ").last().unwrap_or(part).trim().to_string()
            })
            .filter(|s| !s.is_empty())
            .filter(|s| !s.contains("隐藏") && !s.contains("Hide") && !s.contains("Hidden"))
            .collect();

        // 提取隐藏标签
        let hidden_tags = raw.hidden_tags.unwrap_or_default();

        // Use descriptions if skills is empty (for skill-type items from skills_db)
        let skill_source = if raw.skills.is_some() { 
            raw.skills.unwrap_or_default() 
        } else { 
            raw.descriptions.unwrap_or_default() 
        };
        
        let skills = skill_source.into_iter()
            .map(|s| SkillText {
                en: s.en.unwrap_or_default(),
                cn: s.cn.unwrap_or_default(),
            })
            .filter(|s| !s.cn.is_empty() || !s.en.is_empty())
            .collect();
        
        // Handle skills_passive
        let skills_passive = raw.skills_passive.map(|passive_skills| {
            passive_skills.into_iter()
                .map(|s| SkillText {
                    en: s.en.unwrap_or_default(),
                    cn: s.cn.unwrap_or_default(),
                })
                .filter(|s| !s.cn.is_empty() || !s.en.is_empty())
                .collect()
        });
        
        // Handle quests
        let quests = raw.quests;
        
        // Handle enchantments
        let mut enchantments = Vec::new();
        if let Some(val) = raw.enchantments {
            if let Some(obj) = val.as_object() {
                for (_key, details) in obj {
                    let name_cn = details.get("name_cn").and_then(|v| v.as_str());
                    let effect_cn = details.get("effect_cn").and_then(|v| v.as_str());
                    let effect_en = details.get("effect_en").and_then(|v| v.as_str());
                    
                    let effect = effect_cn.or(effect_en);
                    if let Some(eff) = effect {
                        if let Some(n) = name_cn {
                            // 使用分隔符方便前端拆分名称和描述
                            enchantments.push(format!("{}|{}", n, eff));
                        } else {
                            enchantments.push(eff.to_string());
                        }
                    }
                }
            }
        }
        
        let damage = raw.damage;
        let heal = raw.heal;
        let shield = raw.shield;
        let ammo = raw.ammo;
        let crit = raw.crit;
        let multicast = raw.multicast;
        let burn = raw.burn;
        let poison = raw.poison;
        let regen = raw.regen;
        let lifesteal = raw.lifesteal;
        // Removed .sort() to keep JSON order

        ItemData {
            uuid: raw.id,
            name: name_en,
            name_cn,
            tier: raw.starting_tier.clone().unwrap_or_else(|| "Bronze".to_string()),
            available_tiers: raw.available_tiers.unwrap_or_default(),
            tags: raw.tags.unwrap_or_default(),
            hidden_tags,
            size: raw.size,
            processed_tags,
            heroes: raw.heroes,
            cooldown: raw.cooldown,
            cooldown_tiers: raw.cooldown_tiers.unwrap_or_default(),
            damage_tiers: raw.damage_tiers.unwrap_or_default(),
            damage,
            heal_tiers: raw.heal_tiers.unwrap_or_default(),
            heal,
            shield_tiers: raw.shield_tiers.unwrap_or_default(),
            shield,
            ammo_tiers: raw.ammo_tiers.unwrap_or_default(),
            ammo,
            crit_tiers: raw.crit_tiers.unwrap_or_default(),
            crit,
            multicast_tiers: raw.multicast_tiers.unwrap_or_default(),
            multicast,
            burn_tiers: raw.burn_tiers.unwrap_or_default(),
            burn,
            poison_tiers: raw.poison_tiers.unwrap_or_default(),
            poison,
            regen_tiers: raw.regen_tiers.unwrap_or_default(),
            regen,
            lifesteal_tiers: raw.lifesteal_tiers.unwrap_or_default(),
            lifesteal,
            skills,
            skills_passive,
            quests,
            enchantments,
            description: "".to_string(), // will be populated
            instance_id: None, // Used for tracked stash items
            description_cn: raw.description_cn,
            image: raw.image,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TierInfo {
    pub description: Vec<String>,
    pub extra_description: Vec<String>,
    pub cd: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SkillText {
    pub en: String,
    pub cn: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MonsterSubItem {
    pub id: Option<String>,
    pub name: String,
    pub name_en: Option<String>,
    pub tier: Option<String>,
    pub current_tier: Option<String>,
    pub starting_tier: Option<String>,
    pub tags: Option<Vec<String>>,
    pub tiers: Option<HashMap<String, Option<TierInfo>>>,
    pub size: Option<String>,
    pub damage_tiers: Option<String>,
    pub heal_tiers: Option<String>,
    pub shield_tiers: Option<String>,
    pub ammo_tiers: Option<String>,
    pub burn_tiers: Option<String>,
    pub poison_tiers: Option<String>,
    pub regen_tiers: Option<String>,
    pub lifesteal_tiers: Option<String>,
    pub multicast_tiers: Option<String>,
    pub cooldown: Option<i32>,
    pub cooldown_tiers: Option<String>,
    pub skills: Option<Vec<SkillText>>,
    pub damage: Option<i32>,
    pub heal: Option<i32>,
    pub shield: Option<i32>,
    pub burn: Option<i32>,
    pub poison: Option<i32>,
    pub regen: Option<i32>,
    pub lifesteal: Option<i32>,
    pub ammo: Option<i32>,
    pub multicast: Option<i32>,
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MonsterData {
    pub name: String,
    pub name_zh: String,
    pub available: Option<String>,
    pub health: Option<serde_json::Value>,
    pub level: Option<serde_json::Value>,
    pub skills: Option<Vec<MonsterSubItem>>,
    pub items: Option<Vec<MonsterSubItem>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SyncPayload {
    pub hand_items: Vec<ItemData>,
    pub stash_items: Vec<ItemData>,
    pub all_tags: Vec<String>,
}

pub struct ItemDb {
    pub list: Vec<ItemData>,
    pub id_map: HashMap<String, usize>,
    pub unique_tags: Vec<String>,
}

pub struct SkillDb {
    pub list: Vec<ItemData>, // Skills have similar structure
    pub id_map: HashMap<String, usize>,
}

pub struct DbState {
    pub items: Arc<RwLock<ItemDb>>,
    pub skills: Arc<RwLock<SkillDb>>,
    pub monsters: Arc<RwLock<serde_json::Map<String, serde_json::Value>>>,
}

#[allow(dead_code)]
fn construct_monster_sub_item(item_data: Option<ItemData>, fallback_name_cn: &str, fallback_name_en: &str, current_tier: &str, override_size: Option<&str>) -> serde_json::Value {
    let mut desc = Vec::new();
    let mut name_cn = fallback_name_cn.to_string();
    let mut name_en = fallback_name_en.to_string();
    let mut cooldown = None;
    let mut size = override_size.map(|s| s.to_string());
    let mut id = "".to_string();
    let mut tiers = serde_json::Map::new();
    let mut skills: Vec<SkillText> = Vec::new();
    let mut damage_tiers = None;
    let mut heal_tiers = None;
    let mut shield_tiers = None;
    let mut ammo_tiers = None;
    let mut burn_tiers = None;
    let mut poison_tiers = None;
    let mut regen_tiers = None;
    let mut lifesteal_tiers = None;
    let mut multicast_tiers = None;
    let mut cooldown_tiers = None;
    let mut starting_tier: Option<String> = None;
    
    // Single value fallbacks
    let mut damage_val = None;
    let mut heal_val = None;
    let mut shield_val = None;
    let mut burn_val = None;
    let mut poison_val = None;
    let mut regen_val = None;
    let mut lifesteal_val = None;
    let mut ammo_val = None;
    let mut multicast_val = None;

    if let Some(item) = item_data {
        name_cn = item.name_cn;
        name_en = item.name;
        id = item.uuid;
        starting_tier = Some(item.tier.clone());

        if size.is_none() {
            size = item.size;
        }
        if !item.description.is_empty() {
            desc.push(item.description.clone());
        }
        
        // 直接使用ItemData中的SkillText数组
        skills = item.skills.clone();
        
        // 为desc添加技能文本（用于tiers显示）
        for skill in &item.skills {
            let skill_text = if !skill.cn.is_empty() { &skill.cn } else { &skill.en };
            if !skill_text.is_empty() {
                desc.push(skill_text.clone());
            }
        }
        cooldown = item.cooldown;
        
        // Populate single values from ItemData
        damage_val = item.damage;
        heal_val = item.heal;
        shield_val = item.shield;
        burn_val = item.burn;
        poison_val = item.poison;
        regen_val = item.regen;
        lifesteal_val = item.lifesteal;
        ammo_val = item.ammo;
        multicast_val = item.multicast;
        
        // 提取各种tier字段（移除原来的skills提取代码）
        damage_tiers = if !item.damage_tiers.is_empty() { Some(item.damage_tiers.clone()) } else { None };
        heal_tiers = if !item.heal_tiers.is_empty() { Some(item.heal_tiers.clone()) } else { None };
        shield_tiers = if !item.shield_tiers.is_empty() { Some(item.shield_tiers.clone()) } else { None };
        ammo_tiers = if !item.ammo_tiers.is_empty() { Some(item.ammo_tiers.clone()) } else { None };
        burn_tiers = if !item.burn_tiers.is_empty() { Some(item.burn_tiers.clone()) } else { None };
        poison_tiers = if !item.poison_tiers.is_empty() { Some(item.poison_tiers.clone()) } else { None };
        regen_tiers = if !item.regen_tiers.is_empty() { Some(item.regen_tiers.clone()) } else { None };
        lifesteal_tiers = if !item.lifesteal_tiers.is_empty() { Some(item.lifesteal_tiers.clone()) } else { None };
        multicast_tiers = if !item.multicast_tiers.is_empty() { Some(item.multicast_tiers.clone()) } else { None };
        cooldown_tiers = if !item.cooldown_tiers.is_empty() { Some(item.cooldown_tiers.clone()) } else { None };

        // Parse multiples tiers if available
        if !item.available_tiers.is_empty() {
            let avail_list: Vec<&str> = item.available_tiers.split('/').collect();
            let cd_list: Vec<&str> = item.cooldown_tiers.split('/').collect();
            
            for (i, t_name) in avail_list.iter().enumerate() {
                let mut t_info = serde_json::Map::new();
                t_info.insert("description".to_string(), serde_json::Value::Array(desc.iter().map(|s| serde_json::Value::String(s.clone())).collect()));
                t_info.insert("extra_description".to_string(), serde_json::Value::Array(vec![]));
                
                let cd_val = if i < cd_list.len() {
                    let ms: f32 = cd_list[i].trim().parse().unwrap_or(0.0);
                    if ms > 0.0 { Some(format!("{:.1}s", ms / 1000.0)) } else { None }
                } else if !cd_list.is_empty() && !cd_list[0].is_empty() {
                    // Repeat last cd value if more tiers exist
                    let ms: f32 = cd_list.last().unwrap().trim().parse().unwrap_or(0.0);
                    if ms > 0.0 { Some(format!("{:.1}s", ms / 1000.0)) } else { None }
                } else if i == 0 {
                    cooldown.map(|c| format!("{:.1}s", c))
                } else {
                    None
                };
                
                t_info.insert("cd".to_string(), cd_val.map(serde_json::Value::String).unwrap_or(serde_json::Value::Null));
                tiers.insert(t_name.to_lowercase(), serde_json::Value::Object(t_info));
            }
        }
    }

    if tiers.is_empty() || !tiers.contains_key(&current_tier.to_lowercase()) {
        let mut t_info = serde_json::Map::new();
        t_info.insert("description".to_string(), serde_json::Value::Array(desc.into_iter().map(serde_json::Value::String).collect()));
        t_info.insert("extra_description".to_string(), serde_json::Value::Array(vec![]));
        t_info.insert("cd".to_string(), cooldown.map(|c| serde_json::Value::String(format!("{:.1}s", c))).unwrap_or(serde_json::Value::Null));
        
        tiers.insert(current_tier.to_lowercase(), serde_json::Value::Object(t_info));
    }
    
    let tier_label = format!("{}+", current_tier);
    
    let mut sub = serde_json::Map::new();
    sub.insert("name".to_string(), serde_json::Value::String(name_cn));
    sub.insert("name_en".to_string(), serde_json::Value::String(name_en));
    sub.insert("id".to_string(), serde_json::Value::String(id));
    sub.insert("tier".to_string(), serde_json::Value::String(tier_label));
    sub.insert("current_tier".to_string(), serde_json::Value::String(current_tier.to_string()));
    
    // Normalize size if it exists
    let final_size = size.map(|s| {
        let normalized = s.split(" / ").next().unwrap_or(&s).to_string();
        normalized
    });
    
    sub.insert("size".to_string(), final_size.map(serde_json::Value::String).unwrap_or(serde_json::Value::Null));
    sub.insert("tiers".to_string(), serde_json::Value::Object(tiers));
    
    // 添加所有新字段
    sub.insert("damage_tiers".to_string(), damage_tiers.map(serde_json::Value::String).unwrap_or(serde_json::Value::Null));
    sub.insert("heal_tiers".to_string(), heal_tiers.map(serde_json::Value::String).unwrap_or(serde_json::Value::Null));
    sub.insert("shield_tiers".to_string(), shield_tiers.map(serde_json::Value::String).unwrap_or(serde_json::Value::Null));
    sub.insert("ammo_tiers".to_string(), ammo_tiers.map(serde_json::Value::String).unwrap_or(serde_json::Value::Null));
    sub.insert("burn_tiers".to_string(), burn_tiers.map(serde_json::Value::String).unwrap_or(serde_json::Value::Null));
    sub.insert("poison_tiers".to_string(), poison_tiers.map(serde_json::Value::String).unwrap_or(serde_json::Value::Null));
    sub.insert("regen_tiers".to_string(), regen_tiers.map(serde_json::Value::String).unwrap_or(serde_json::Value::Null));
    sub.insert("lifesteal_tiers".to_string(), lifesteal_tiers.map(serde_json::Value::String).unwrap_or(serde_json::Value::Null));
    sub.insert("multicast_tiers".to_string(), multicast_tiers.map(serde_json::Value::String).unwrap_or(serde_json::Value::Null));
    sub.insert("cooldown".to_string(), cooldown.map(|c| serde_json::Value::Number((c as i32).into())).unwrap_or(serde_json::Value::Null));
    sub.insert("cooldown_tiers".to_string(), cooldown_tiers.map(serde_json::Value::String).unwrap_or(serde_json::Value::Null));
    sub.insert("skills".to_string(), serde_json::to_value(skills).unwrap_or(serde_json::Value::Null));
    sub.insert("starting_tier".to_string(), starting_tier.map(serde_json::Value::String).unwrap_or(serde_json::Value::Null));

    // Valid single values
    if let Some(v) = damage_val { sub.insert("damage".to_string(), serde_json::Value::Number(v.into())); }
    if let Some(v) = heal_val { sub.insert("heal".to_string(), serde_json::Value::Number(v.into())); }
    if let Some(v) = shield_val { sub.insert("shield".to_string(), serde_json::Value::Number(v.into())); }
    if let Some(v) = burn_val { sub.insert("burn".to_string(), serde_json::Value::Number(v.into())); }
    if let Some(v) = poison_val { sub.insert("poison".to_string(), serde_json::Value::Number(v.into())); }
    if let Some(v) = regen_val { sub.insert("regen".to_string(), serde_json::Value::Number(v.into())); }
    if let Some(v) = lifesteal_val { sub.insert("lifesteal".to_string(), serde_json::Value::Number(v.into())); }
    if let Some(v) = ammo_val { sub.insert("ammo".to_string(), serde_json::Value::Number(v.into())); }
    if let Some(v) = multicast_val { sub.insert("multicast".to_string(), serde_json::Value::Number(v.into())); }
    
    serde_json::Value::Object(sub)
}

fn get_log_path() -> PathBuf {
    if cfg!(target_os = "macos") {
        let home = std::env::var("HOME").unwrap_or_default();
        PathBuf::from(home)
            .join("Library")
            .join("Logs")
            .join("Tempo Storm")
            .join("The Bazaar")
            .join("Player.log")
    } else {
        let home = std::env::var("USERPROFILE").unwrap_or_default();
        PathBuf::from(home)
            .join("AppData")
            .join("LocalLow")
            .join("Tempo Storm")
            .join("The Bazaar")
            .join("Player.log")
    }
}

#[tauri::command]
async fn start_template_loading(app: tauri::AppHandle) -> Result<(), String> {
    println!("=============== [start_template_loading] CALLED ===============");
    let resources_path = app.path().resource_dir().map_err(|e| {
        let err = format!("Failed to get resource dir in template loading: {}", e);
        log_to_file(&err);
        err
    })?;
    let res_dir = resources_path.join("resources");
    let cache_dir = get_cache_path().parent().ok_or_else(|| {
        let err = "Failed to get cache parent dir".to_string();
        log_to_file(&err);
        err
    })?.to_path_buf();
    
    // 加载items_db.json到DbState
    let items_db_path = res_dir.join("items_db.json");
    println!("[ItemDB] Attempting to load from: {:?}", items_db_path);
    if items_db_path.exists() {
        match std::fs::read_to_string(&items_db_path) {
            Ok(json_str) => {
                println!("[ItemDB] File read successfully, length: {} bytes", json_str.len());
                match serde_json::from_str::<Vec<RawItem>>(&json_str) {
                    Ok(raw_items_list) => {
                        println!("[ItemDB] Parsed {} raw items from JSON", raw_items_list.len());
                        let items_list: Vec<ItemData> = raw_items_list.into_iter()
                            .map(ItemData::from)
                            .collect();
                        println!("[ItemDB] Converted to {} ItemData entries", items_list.len());
                        
                        let db_state = app.state::<DbState>();
                        let mut items_db = db_state.items.write().unwrap();
                        
                        // 按照size分类统计
                        let mut small_count = 0;
                        let mut medium_count = 0;
                        let mut large_count = 0;
                        
                        items_db.list = items_list.clone();
                        items_db.id_map.clear();
                        for (idx, item) in items_list.iter().enumerate() {
                            items_db.id_map.insert(item.uuid.clone(), idx);
                            
                            // 统计size分类
                            if let Some(size) = &item.size {
                                if size.contains("Small") || size.contains("小型") {
                                    small_count += 1;
                                } else if size.contains("Medium") || size.contains("中型") {
                                    medium_count += 1;
                                } else if size.contains("Large") || size.contains("大型") {
                                    large_count += 1;
                                }
                            }
                        }
                        
                        println!("[ItemDB] Loaded {} items: Small={}, Medium={}, Large={}", 
                                 items_db.list.len(), small_count, medium_count, large_count);
                        println!("[ItemDB] id_map has {} entries", items_db.id_map.len());
                    }
                    Err(e) => {
                        println!("[ItemDB] Failed to parse items_db.json: {}", e);
                        log_to_file(&format!("ItemDB parse error: {}", e));
                    }
                }
            }
            Err(e) => {
                println!("[ItemDB] Failed to read items_db.json: {}", e);
                log_to_file(&format!("ItemDB read error: {}", e));
            }
        }
    } else {
        println!("[ItemDB] items_db.json not found at: {:?}", items_db_path);
        log_to_file(&format!("ItemDB not found: {:?}", items_db_path));
    }
    
    // 异步加载特征模板（现在按size分类加载）
    let res_dir_async = res_dir.clone();
    let cache_dir_async = cache_dir.clone();
    let app_async = app.clone();
    tauri::async_runtime::spawn(async move {
        let res_dir_clone = res_dir_async.clone();
        let cache_dir_clone = cache_dir_async.clone();
        
        // Create clones for the new all-cards loader
        let res_dir_clone2 = res_dir_async.clone();
        let cache_dir_clone2 = cache_dir_async.clone();
        
        let _ = monster_recognition::preload_templates_async(res_dir_async, cache_dir_async).await;
        
        // 加载事件特征模板
        let _ = monster_recognition::load_event_templates(app_async).await;
        
        // 按size分类加载卡牙特征 (用于精准的高效识别)
        let _ = monster_recognition::preload_card_templates_by_size_async(res_dir_clone, cache_dir_clone).await;
        
        // 加载所有卡牌模板 (用于通用的鼠标指向识别 - 900+ 张图片)
        let _ = monster_recognition::preload_card_templates_async(res_dir_clone2, cache_dir_clone2).await;
    });
    
    // 验证items_db是否加载成功
    {
        let db_state = app.state::<DbState>();
        let items_db = db_state.items.read().unwrap();
        println!("[ItemDB] Verification: {} items loaded, {} in id_map", items_db.list.len(), items_db.id_map.len());
    }

    // Load skills_db.json
    let skills_db_path = res_dir.join("skills_db.json");
    if skills_db_path.exists() {
        match std::fs::read_to_string(&skills_db_path) {
            Ok(json_str) => {
                match serde_json::from_str::<Vec<RawItem>>(&json_str) {
                    Ok(raw_list) => {
                        let list: Vec<ItemData> = raw_list.into_iter().map(ItemData::from).collect();
                        let db_state = app.state::<DbState>();
                        let mut skills_db = db_state.skills.write().unwrap();
                        
                        skills_db.list = list.clone();
                        skills_db.id_map.clear();
                        for (idx, item) in list.iter().enumerate() {
                            skills_db.id_map.insert(item.uuid.clone(), idx);
                        }
                        println!("[SkillDB] Loaded {} skills", skills_db.list.len());
                    },
                    Err(e) => println!("[SkillDB] Parse error: {}", e),
                }
            },
           Err(e) => println!("[SkillDB] Read error: {}", e),
        }
    }

    // Load monsters_db.json
    let monsters_db_path = res_dir.join("monsters_db.json");
    if monsters_db_path.exists() {
        match std::fs::read_to_string(&monsters_db_path) {
            Ok(json_str) => {
                match serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&json_str) {
                     Ok(map) => {
                         let db_state = app.state::<DbState>();
                         let mut monsters_db = db_state.monsters.write().unwrap();
                         *monsters_db = map;
                         println!("[MonsterDB] Loaded {} monsters", monsters_db.len());
                     },
                     Err(e) => println!("[MonsterDB] Parse error: {}", e),
                }
            },
            Err(e) => println!("[MonsterDB] Read error: {}", e),
        }
    }
    
    Ok(())
}

// #[tauri::command]
// #[allow(dead_code)]
// async fn clear_monster_cache() -> Result<(), String> {
//     let cache_dir = get_cache_path().parent().unwrap().to_path_buf();
//     let cache_file = cache_dir.join("monster_features.bin");
//     if cache_file.exists() {
//         std::fs::remove_file(cache_file).map_err(|e| e.to_string())?;
//     }
//     Ok(())
// }

#[tauri::command]
async fn get_item_info(state: tauri::State<'_, DbState>, id: String) -> Result<Option<ItemData>, String> {
    let db = state.items.read().unwrap();
    if let Some(&idx) = db.id_map.get(&id) {
        return Ok(Some(db.list[idx].clone()));
    }
    // Also check skills if not found in items
    let sdb = state.skills.read().unwrap();
    if let Some(&idx) = sdb.id_map.get(&id) {
        return Ok(Some(sdb.list[idx].clone()));
    }
    Ok(None)
}

#[tauri::command]
async fn set_overlay_ignore_cursor(_app: tauri::AppHandle, _ignore: bool) -> Result<(), String> {
    // 新方案：yolo-monitor 和 detail-popup 窗口始终可交互
    // 通过缩小窗口来避免妨碍用户，而不是设置鼠标穿透
    // 此命令保留以兼容旧代码，但不执行任何操作
    Ok(())
}

#[tauri::command]
async fn show_yolo_monitor_window(app: tauri::AppHandle, show: bool) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("yolo-monitor") {
        if show {
            let _ = window.show();
            // 确保显示时应用无焦点样式
            apply_no_activate_style(&window);
        } else {
            let _ = window.hide();
        }
    }
    Ok(())
}

#[tauri::command]
async fn show_detail_popup_at(app: tauri::AppHandle, x: i32, y: i32, data_type: String, data: serde_json::Value) -> Result<(), String> {
    println!("[Show Detail Popup] Requested position: ({}, {}), Type: {}", x, y, data_type);
    
    if let Some(window) = app.get_webview_window("detail-popup") {
        println!("[Show Detail Popup] Window found");
        
        // 加载保存的状态
        let state = load_state();
        
        // 使用保存的位置和大小
        let saved_width = state.detail_popup_width.unwrap_or(480);
        let saved_height = state.detail_popup_height.unwrap_or(700);
        let final_width = if saved_width < 100 { 480 } else { saved_width };
        let final_height = if saved_height < 100 { 700 } else { saved_height };
        
        // 如果有保存的位置，使用保存的位置；否则在鼠标所在屏幕中心显示
        let (final_x, final_y) = if let (Some(saved_x), Some(saved_y)) = (state.detail_popup_x, state.detail_popup_y) {
            (saved_x, saved_y)
        } else {
            // 获取鼠标所在的屏幕
            let monitors = window.available_monitors().map_err(|e| e.to_string())?;
            
            // 找到包含鼠标位置的屏幕
            let target_monitor = monitors.iter().find(|m| {
                let pos = m.position();
                let size = m.size();
                x >= pos.x && x < (pos.x + size.width as i32) &&
                y >= pos.y && y < (pos.y + size.height as i32)
            });
            
            if let Some(monitor) = target_monitor {
                // 在该屏幕中心显示
                let pos = monitor.position();
                let size = monitor.size();
                let center_x = pos.x + (size.width as i32 / 2) - (final_width as i32 / 2);
                let center_y = pos.y + (size.height as i32 / 2) - (final_height as i32 / 2);
                println!("[Show Detail Popup] Mouse on monitor at ({}, {}), size {}x{}, centering at ({}, {})", 
                         pos.x, pos.y, size.width, size.height, center_x, center_y);
                (center_x, center_y)
            } else {
                // 降级：使用传入的位置
                (x - 200, y - 300)
            }
        };
        
        println!("[Show Detail Popup] Using position: ({}, {}), size: {}x{}", final_x, final_y, final_width, final_height);
        
        // 设置窗口大小
        let _ = window.set_size(tauri::PhysicalSize::new(final_width, final_height));
        
        // 设置窗口位置
        match window.set_position(tauri::PhysicalPosition::new(final_x, final_y)) {
            Ok(_) => println!("[Show Detail Popup] Position set successfully"),
            Err(e) => println!("[Show Detail Popup] Failed to set position: {}", e),
        }
        
        // 确保窗口在最前面
        match window.set_always_on_top(true) {
            Ok(_) => println!("[Show Detail Popup] Set always on top"),
            Err(e) => println!("[Show Detail Popup] Failed to set always on top: {}", e),
        }
        
        // 显示窗口
        match window.show() {
            Ok(_) => {
                println!("[Show Detail Popup] Window shown successfully");
                // 再次检查可见性
                if let Ok(visible) = window.is_visible() {
                    println!("[Show Detail Popup] Visibility after show: {}", visible);
                }
                // 应用无焦点样式，防止抢夺游戏焦点
                apply_no_activate_style(&window);
            },
            Err(e) => println!("[Show Detail Popup] Failed to show window: {}", e),
        }
        
        // 移除 set_focus，因为它会导致游戏失去焦点
        // let _ = window.set_focus();
        
        // 发送数据给前端 - 直接向 detail-popup 窗口发射事件
        let payload = serde_json::json!({
            "type": data_type,
            "data": data
        });
        println!("[Show Detail Popup] Emitting event with payload type: {}", data_type);
        match window.emit("show-detail-popup", payload) {
            Ok(_) => println!("[Show Detail Popup] Event emitted to detail-popup window successfully"),
            Err(e) => println!("[Show Detail Popup] Failed to emit event: {}", e),
        }
        
        // 最后检查窗口的 label
        println!("[Show Detail Popup] Window label: {}", window.label());
    } else {
        println!("[Show Detail Popup] ERROR: detail-popup window not found!");
        return Err("detail-popup window not found".to_string());
    }
    Ok(())
}

#[tauri::command]
async fn hide_detail_popup(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("detail-popup") {
        // 保存当前位置和大小（但不保存太小的尺寸）
        if let (Ok(position), Ok(size)) = (window.outer_position(), window.outer_size()) {
            // 只有当窗口尺寸合理时才保存（避免保存缩小后的 1x1）
            if size.width >= 100 && size.height >= 100 {
                let mut state = load_state();
                state.detail_popup_x = Some(position.x);
                state.detail_popup_y = Some(position.y);
                state.detail_popup_width = Some(size.width);
                state.detail_popup_height = Some(size.height);
                save_state(&state);
                println!("[Hide Detail Popup] Saved position: ({}, {}), size: {}x{}", position.x, position.y, size.width, size.height);
            } else {
                println!("[Hide Detail Popup] Skipping save - window too small: {}x{}", size.width, size.height);
            }
        }
        
        // 发送隐藏事件（触发缩小动画）- 直接向窗口发射
        let _ = window.emit("hide-detail-popup", ());
    }
    
    Ok(())
}

#[tauri::command]
async fn reset_detail_popup_position(app: tauri::AppHandle) -> Result<(), String> {
    println!("[Reset Detail Popup] Clearing saved position and size");
    
    // 清除保存的位置和大小
    let mut state = load_state();
    state.detail_popup_x = None;
    state.detail_popup_y = None;
    state.detail_popup_width = None;
    state.detail_popup_height = None;
    save_state(&state);
    
    // 如果窗口当前可见，隐藏它
    if let Some(window) = app.get_webview_window("detail-popup") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
            println!("[Reset Detail Popup] Window was visible, hiding it");
        }
    }
    
    println!("[Reset Detail Popup] Position reset complete");
    Ok(())
}

#[tauri::command]
fn save_window_geometry(window_label: String, x: Option<i32>, y: Option<i32>, width: Option<u32>, height: Option<u32>) {
    if window_label == "main" {
        println!("[Geometry] Saving main window: x={:?}, y={:?}, w={:?}, h={:?}", x, y, width, height);
        let mut state = load_state();
        let mut changed = false;
        
        if let Some(val) = x { state.main_window_x = Some(val); changed = true; }
        if let Some(val) = y { state.main_window_y = Some(val); changed = true; }
        if let Some(val) = width { state.main_window_width = Some(val); changed = true; }
        if let Some(val) = height { state.main_window_height = Some(val); changed = true; }
        
        if changed {
            save_state(&state);
            println!("[Geometry] State saved to disk.");
        }
    }
}

#[tauri::command]
fn get_window_geometry(window_label: String) -> serde_json::Value {
    if window_label == "main" {
        let state = load_state();
        println!("[Geometry] Loading saved position: x={:?}, y={:?}, w={:?}, h={:?}", 
                 state.main_window_x, state.main_window_y, state.main_window_width, state.main_window_height);
        return serde_json::json!({
            "x": state.main_window_x,
            "y": state.main_window_y,
            "width": state.main_window_width,
            "height": state.main_window_height
        });
    }
    serde_json::json!({})
}

#[tauri::command]
fn reset_window_geometry(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;
    
    println!("[Geometry] Reset window geometry triggered");
    
    // 1. 清除保存的几何信息
    let mut state = load_state();
    state.main_window_x = None;
    state.main_window_y = None;
    state.main_window_width = None;
    state.main_window_height = None;
    save_state(&state);
    println!("[Geometry] Cleared saved geometry");
    
    // 2. 获取主窗口
    let window = app.get_webview_window("main")
        .ok_or_else(|| "Main window not found".to_string())?;
    
    // 3. 获取主显示器信息
    use xcap::Monitor;
    let monitors = Monitor::all().map_err(|e| e.to_string())?;
    let primary_monitor = monitors.into_iter().next()
        .ok_or_else(|| "No monitor found".to_string())?;
    
    // 4. 计算默认位置（主显示器右上角）
    let default_width = 600;
    let default_height = 850;
    
    // 获取逻辑分辨率（考虑DPI缩放）
    let scale_factor = 1.0; // 默认值
    let logical_width = (primary_monitor.width() as f64 / scale_factor) as i32;
    let target_x = primary_monitor.x() + logical_width - default_width;
    let target_y = primary_monitor.y();
    
    println!("[Geometry] Resetting to: x={}, y={}, w={}, h={}", target_x, target_y, default_width, default_height);
    
    // 5. 设置窗口位置和大小
    use tauri::{PhysicalPosition, PhysicalSize};
    window.set_size(PhysicalSize::new(default_width as u32, default_height as u32))
        .map_err(|e| e.to_string())?;
    window.set_position(PhysicalPosition::new(target_x, target_y))
        .map_err(|e| e.to_string())?;
    
    // 6. 显示并获取焦点
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;
    
    // 7. 发送事件到前端让它展开窗口
    window.emit("reset-window-geometry", ()).map_err(|e| e.to_string())?;
    
    println!("[Geometry] Window geometry reset complete");
    Ok(())
}

// 存储最后一个前台窗口的信息
static LAST_FOREGROUND_WINDOW: std::sync::RwLock<Option<String>> = std::sync::RwLock::new(None);

// 检查游戏窗口是否活跃（是否是前台窗口）
fn is_game_window_active() -> bool {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextW};
        
        unsafe {
            let hwnd = GetForegroundWindow();
            if !hwnd.is_invalid() {
                let mut title: [u16; 512] = [0; 512];
                let len = GetWindowTextW(hwnd, &mut title);
                if len > 0 {
                    let window_title = String::from_utf16_lossy(&title[..len as usize]);
                    let is_game = window_title.to_lowercase().contains("the bazaar") || window_title.to_lowercase().contains("thebazaar");
                    return is_game;
                }
            }
        }
        false
    }
    #[cfg(not(target_os = "windows"))]
    {
        // macOS 等其他平台，默认返回 true
        true
    }
}

// 辅助函数：应用 WS_EX_NOACTIVATE 样式，防止窗口获取焦点
fn apply_no_activate_style(window: &tauri::WebviewWindow) {
    #[cfg(target_os = "windows")]
    if let Ok(hwnd) = window.hwnd() {
        unsafe {
            use windows::Win32::Foundation::HWND as HWND_TYPE;
            let hwnd_val = HWND_TYPE(hwnd.0 as _);
            let style = GetWindowLongW(hwnd_val, GWL_EXSTYLE);
            // WS_EX_NOACTIVATE: 防止窗口被激活（获取焦点）
            // WS_EX_TOOLWINDOW: 不在任务栏显示
            SetWindowLongW(hwnd_val, GWL_EXSTYLE, style | WS_EX_NOACTIVATE.0 as i32 | WS_EX_TOOLWINDOW.0 as i32);
        }
    }
}

// 更新最后一个前台窗口
fn update_last_foreground_window() {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextW};
        
        unsafe {
            let hwnd = GetForegroundWindow();
            if !hwnd.is_invalid() {
                let mut title: [u16; 512] = [0; 512];
                let len = GetWindowTextW(hwnd, &mut title);
                if len > 0 {
                    let window_title = String::from_utf16_lossy(&title[..len as usize]);
                    if let Ok(mut last) = LAST_FOREGROUND_WINDOW.write() {
                        *last = Some(window_title.clone());
                        println!("[Focus] Last foreground window: {}", window_title);
                    }
                }
            }
        }
    }
}

#[tauri::command]
fn was_last_foreground_game() -> bool {
    #[cfg(target_os = "windows")]
    {
        if let Ok(last) = LAST_FOREGROUND_WINDOW.read() {
            if let Some(title) = &*last {
                let lower = title.to_lowercase();
                return lower.contains("the bazaar") || lower.contains("thebazaar");
            }
        }
        false
    }
    #[cfg(not(target_os = "windows"))]
    {
        true
    }
}

#[tauri::command]
async fn restore_game_focus() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
            if !IsWindowVisible(hwnd).as_bool() {
                return BOOL(1);
            }

            let mut title: [u16; 512] = [0; 512];
            let len = GetWindowTextW(hwnd, &mut title);
            if len > 0 {
                let window_title = String::from_utf16_lossy(&title[..len as usize]).to_lowercase();
                if window_title.contains("the bazaar") || window_title.contains("thebazaar") {
                    let out_ptr = lparam.0 as *mut Option<HWND>;
                    if !out_ptr.is_null() {
                        *out_ptr = Some(hwnd);
                    }
                    return BOOL(0);
                }
            }

            BOOL(1)
        }

        let mut target_hwnd: Option<HWND> = None;
        unsafe {
            let ptr = &mut target_hwnd as *mut Option<HWND>;
            let _ = EnumWindows(Some(enum_windows_proc), LPARAM(ptr as isize));
        }

        if let Some(hwnd) = target_hwnd {
            unsafe {
                let _ = ShowWindow(hwnd, SW_RESTORE);
                let _ = SetForegroundWindow(hwnd);
            }
            println!("[Focus] Restored focus to The Bazaar window.");
            Ok(())
        } else {
            Err("The Bazaar window not found".to_string())
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok(())
    }
}

fn get_cache_path() -> PathBuf {
    if cfg!(target_os = "macos") {
        let home = std::env::var("HOME").unwrap_or_default();
        PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("com.duang.BazaarHelper")
            .join("state_cache.json")
    } else {
        let home = std::env::var("USERPROFILE").unwrap_or_default();
        PathBuf::from(home)
            .join("AppData")
            .join("Local")
            .join("BazaarHelper")
            .join("state_cache.json")
    }
}

#[tauri::command]
fn get_show_yolo_monitor() -> Result<bool, String> {
    let state = load_state();
    Ok(state.show_yolo_monitor)
}

#[allow(dead_code)]
fn get_prev_log_path() -> PathBuf {
    let mut p = get_log_path();
    p.set_file_name("Player-prev.log");
    p
}

fn save_state(state: &PersistentState) {
    let path = get_cache_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string(state) {
        let _ = std::fs::write(path, json);
    }
}

fn load_state() -> PersistentState {
    let path = get_cache_path();
    if let Ok(json) = std::fs::read_to_string(path) {
        if let Ok(state) = serde_json::from_str::<PersistentState>(&json) {
            return state;
        }
    }
    PersistentState::default()
}

#[allow(dead_code)]
fn lookup_item(tid: &str, items_db: &ItemDb, skills_db: &SkillDb) -> Option<ItemData> {
    if let Some(&index) = items_db.id_map.get(tid) {
        return items_db.list.get(index).cloned();
    }
    if let Some(&index) = skills_db.id_map.get(tid) {
        return skills_db.list.get(index).cloned();
    }
    None
}

#[allow(dead_code)]
fn lookup_item_by_name(name_cn: &str, items_db: &ItemDb, skills_db: &SkillDb) -> Option<ItemData> {
    // 先在物品库中查找完整名字
    for item in &items_db.list {
        if item.name_cn == name_cn {
            return Some(item.clone());
        }
    }
    // 再在技能库中查找完整名字
    for skill in &skills_db.list {
        if skill.name_cn == name_cn {
            return Some(skill.clone());
        }
    }
    
    // 如果找不到，尝试去除空格及空格之前的前缀（如"毒性蔓延 獠牙" -> "獠牙"）
    if let Some(space_pos) = name_cn.rfind(' ') {
        let base_name = &name_cn[space_pos + 1..];
        
        // 用基础名字再查找一次
        for item in &items_db.list {
            if item.name_cn == base_name {
                return Some(item.clone());
            }
        }
        for skill in &skills_db.list {
            if skill.name_cn == base_name {
                return Some(skill.clone());
            }
        }
    }
    
    None
}

// --- Commands ---
#[derive(Debug, serde::Deserialize)]
pub struct SearchQuery {
    pub keyword: Option<String>,
    pub item_type: Option<String>, // "all", "item", "skill"
    pub size: Option<String>,
    pub start_tier: Option<String>,
    pub hero: Option<String>,
    pub tags: Option<String>,
    pub hidden_tags: Option<String>,
}

#[tauri::command]
fn search_items(query: SearchQuery, state: State<'_, DbState>) -> Result<Vec<ItemData>, String> {
    let mut results = Vec::new();
    let keyword = query.keyword.as_deref().map(|s| s.to_lowercase());
    let size_filter = query.size.as_deref().map(|s| s.to_lowercase());
    let tier_filter = query.start_tier.as_deref().map(|s| s.to_lowercase());
    let hero_filter = query.hero.as_deref().map(|s| s.to_lowercase());
    let tags_filter = query.tags.as_deref().map(|s| s.to_lowercase());
    let htags_filter = query.hidden_tags.as_deref().map(|s| s.to_lowercase());

    let match_item = |item: &ItemData| -> bool {
        if let Some(ref k) = keyword {
            if !item.name_cn.to_lowercase().contains(k) && !item.name.to_lowercase().contains(k) {
                return false;
            }
        }
        if let Some(ref s) = size_filter {
            if !item.size.as_ref().map(|v| v.to_lowercase()).unwrap_or_default().contains(s) {
                return false;
            }
        }
        if let Some(ref t) = tier_filter {
            if !item.tier.to_lowercase().contains(t) {
                return false;
            }
        }
        if let Some(ref h) = hero_filter {
            if !item.heroes.iter().any(|hero| hero.to_lowercase().contains(h)) {
                return false;
            }
        }
        if let Some(ref t) = tags_filter {
             if !item.tags.to_lowercase().contains(t) {
                 return false;
             }
        }
        if let Some(ref h) = htags_filter {
             if !item.hidden_tags.to_lowercase().contains(h) {
                 return false;
             }
        }
        true
    };

    let search_type = query.item_type.as_deref().unwrap_or("all");

    if search_type == "all" || search_type == "item" {
        if let Ok(db) = state.items.read() {
            for item in &db.list {
                if match_item(item) {
                     results.push(item.clone());
                }
            }
        }
    }

    if search_type == "all" || search_type == "skill" {
        if let Ok(db) = state.skills.read() {
            for item in &db.list {
                if match_item(item) {
                     results.push(item.clone());
                }
            }
        }
    }

    // Sort by tier then name
    results.sort_by(|a, b| {
        // Simple tier sort logic (Bronze < Silver < Gold < Diamond < Legendary)
        let tier_rank = |t: &str| match t.split('/').next().unwrap_or("").trim() {
            "Bronze" | "Common" => 1,
            "Silver" => 2,
            "Gold" => 3,
            "Diamond" => 4,
            "Legendary" => 5,
            _ => 10,
        };
        let ta = tier_rank(&a.tier);
        let tb = tier_rank(&b.tier);
        if ta != tb {
            ta.cmp(&tb)
        } else {
            a.name_cn.cmp(&b.name_cn)
        }
    });

    Ok(results)
}

#[tauri::command]
fn get_all_monsters(state: State<'_, DbState>) -> Result<serde_json::Map<String, serde_json::Value>, String> {
    log_to_file("get_all_monsters called");
    let db = state.monsters.read().map_err(|_| "DB Busy")?;
    let count = db.len();
    log_to_file(&format!("Monsters DB contains {} entries", count));
    
    // 调试：输出前几个怪物名称
    if count > 0 {
        let sample_names: Vec<String> = db.keys().take(5).cloned().collect();
        log_to_file(&format!("Sample monster names: {:?}", sample_names));
    } else {
        log_to_file("Warning: Monsters DB is empty!");
    }
    
    Ok(db.clone())
}

#[tauri::command]
fn debug_monsters_db(state: State<'_, DbState>) -> Result<String, String> {
    let db = state.monsters.read().map_err(|_| "DB Busy")?;
    let count = db.len();
    let mut result = format!("Monsters DB Status:\n- Total entries: {}\n", count);
    
    if count > 0 {
        let sample: Vec<String> = db.keys().take(10).cloned().collect();
        result.push_str(&format!("- Sample entries: {:?}\n", sample));
        
        // 检查Day 1的怪物
        let day1_monsters: Vec<String> = db.iter()
            .filter(|(_, data)| {
                data.get("available").and_then(|v| v.as_str()) == Some("Day 1")
            })
            .map(|(name, _)| name.clone())
            .take(5)
            .collect();
        result.push_str(&format!("- Day 1 monsters: {:?}\n", day1_monsters));
    } else {
        result.push_str("- Database is empty!\n");
    }
    
    log_to_file(&result);
    Ok(result)
}

#[tauri::command]
fn clear_yolo_cache() -> Result<String, String> {
    // 清理YOLO扫描结果和图像缓存
    {
        let mut results = get_yolo_scan_results().write().unwrap();
        results.clear();
    }
    {
        let mut saved_img = get_yolo_scan_image().write().unwrap();
        *saved_img = None;
    }
    log_to_file("YOLO cache cleared to free memory");
    Ok("YOLO缓存已清理".to_string())
}

#[tauri::command]
fn debug_resource_paths(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    let resources_path = app.path().resource_dir().map_err(|e| e.to_string())?;
    let mut report = serde_json::Map::new();
    report.insert("resource_dir".to_string(), serde_json::Value::String(resources_path.to_string_lossy().to_string()));

    let files = [
        "monsters_db.json",
        "monsters_export.json",
        "images_monster_map.json",
        "items_db.json",
        "skills_db.json",
    ];

    let mut files_obj = serde_json::Map::new();
    for f in &files {
        let p1 = resources_path.join("resources").join(f);
        let p2 = resources_path.join(f);
        let mut info = serde_json::Map::new();
        info.insert("path1".to_string(), serde_json::Value::String(p1.to_string_lossy().to_string()));
        info.insert("exists1".to_string(), serde_json::Value::Bool(p1.exists()));
        if p1.exists() {
            if let Ok(md) = std::fs::metadata(&p1) {
                info.insert("size1".to_string(), serde_json::Value::Number(serde_json::Number::from(md.len())));
            }
        }
        info.insert("path2".to_string(), serde_json::Value::String(p2.to_string_lossy().to_string()));
        info.insert("exists2".to_string(), serde_json::Value::Bool(p2.exists()));
        if p2.exists() {
            if let Ok(md) = std::fs::metadata(&p2) {
                info.insert("size2".to_string(), serde_json::Value::Number(serde_json::Number::from(md.len())));
            }
        }
        files_obj.insert(f.to_string(), serde_json::Value::Object(info));
    }

    report.insert("files".to_string(), serde_json::Value::Object(files_obj));
    Ok(serde_json::Value::Object(report))
}

#[tauri::command]
fn recognize_monsters_from_screenshot(day: Option<u32>) -> Result<Vec<monster_recognition::MonsterRecognitionResult>, String> {
    let day_filter = day.map(|d| if d >= 10 { "Day 10+".to_string() } else { format!("Day {}", d) });
    monster_recognition::recognize_monsters(day_filter)
}

#[tauri::command]
fn get_template_loading_progress() -> monster_recognition::LoadingProgress {
    monster_recognition::get_loading_progress()
}

#[tauri::command]
fn get_current_day(hours_per_day: Option<u32>, retro: Option<bool>) -> Result<u32, String> {
    // Return cached value if available, log scan only as fallback
    let cached = load_state();
    if cached.day > 0 {
        return Ok(cached.day);
    }
    
    let hours = hours_per_day.unwrap_or(6);
    let retro = retro.unwrap_or(false);
    let log_path = get_log_path();
    
    // Fallback to scan only if cache is 0 (first run)
    if log_path.exists() {
        // Use a more memory-efficient way to read large logs
        let mut file = File::open(&log_path).map_err(|e| e.to_string())?;
        let metadata = file.metadata().map_err(|e| e.to_string())?;
        let file_size = metadata.len();
        
        // Read at most 5MB from the end
        let read_size = file_size.min(5_000_000) as usize;
        let mut buffer = vec![0u8; read_size];
        file.seek(SeekFrom::End(-(read_size as i64))).map_err(|e| e.to_string())?;
        file.read_exact(&mut buffer).map_err(|e| e.to_string())?;
        
        let content = String::from_utf8_lossy(&buffer);
        if let Some(day) = calculate_day_from_log(&content, hours, retro) {
            return Ok(day);
        }
    }

    Ok(1)
}

#[tauri::command]
fn update_day(day: u32) -> Result<(), String> {
    let mut state = load_state();
    state.day = day;
    save_state(&state);
    println!("[State] Manually updated Day to: {}", day);
    Ok(())
}

#[tauri::command]
fn get_detection_hotkey() -> Option<i32> {
    load_state().detection_hotkey
}

#[tauri::command]
fn get_card_detection_hotkey() -> Option<i32> {
    load_state().card_detection_hotkey
}

#[tauri::command]
fn get_toggle_collapse_hotkey() -> Option<i32> {
    load_state().toggle_collapse_hotkey
}

#[tauri::command]
fn set_detection_hotkey(hotkey: i32) {
    let mut state = load_state();
    state.detection_hotkey = Some(hotkey);
    save_state(&state);
    println!("[Config] Detection hotkey updated to: {}", hotkey);
    update_detection_hotkey_cache(Some(hotkey));
}

#[tauri::command]
fn set_card_detection_hotkey(hotkey: i32) {
    let mut state = load_state();
    state.card_detection_hotkey = Some(hotkey);
    save_state(&state);
    println!("[Config] Card detection hotkey updated to: {}", hotkey);
    update_card_detection_hotkey_cache(Some(hotkey));
}

#[tauri::command]
fn set_toggle_collapse_hotkey(hotkey: i32) {
    let mut state = load_state();
    state.toggle_collapse_hotkey = Some(hotkey);
    save_state(&state);
    update_toggle_collapse_hotkey_cache(Some(hotkey));
    println!("[Config] Toggle collapse hotkey updated to: {}", hotkey);
}

#[tauri::command]
fn get_yolo_hotkey() -> Option<i32> {
    load_state().yolo_hotkey
}

#[tauri::command]
fn set_yolo_hotkey(hotkey: i32) {
    let mut state = load_state();
    state.yolo_hotkey = Some(hotkey);
    save_state(&state);
    update_yolo_hotkey_cache(Some(hotkey));
    println!("[Config] YOLO hotkey updated to: {}", hotkey);
}

#[tauri::command]
fn reset_all_hotkeys() {
    let mut state = load_state();
    state.detection_hotkey = None;
    state.card_detection_hotkey = None;
    state.toggle_collapse_hotkey = None;
    state.yolo_hotkey = None;
    state.detail_display_hotkey = None;
    
    update_detection_hotkey_cache(None);
    update_card_detection_hotkey_cache(None);
    update_toggle_collapse_hotkey_cache(None);
    update_yolo_hotkey_cache(None);
    update_detail_hotkey_cache(None);
    save_state(&state);
    println!("[Config] All hotkeys reset to None (disabled)");
}

// 保存野怪识别校准数据
#[tauri::command]
fn save_monster_calibration(calibration: MonsterCalibration) {
    let mut state = load_state();
    
    // 按照x坐标从左到右排序
    let mut sorted_regions = calibration.regions;
    sorted_regions.sort_by(|a, b| a.x.cmp(&b.x));
    
    let sorted_calibration = MonsterCalibration {
        regions: sorted_regions,
        game_window_width: calibration.game_window_width,
        game_window_height: calibration.game_window_height,
        screen_width: calibration.screen_width,
        screen_height: calibration.screen_height,
    };
    
    state.monster_calibration = Some(sorted_calibration.clone());
    save_state(&state);
    println!("[Monster Calibration] Saved calibration data: {:?}", sorted_calibration);
}

// 加载野怪识别校准数据
#[tauri::command]
fn load_monster_calibration() -> Option<MonsterCalibration> {
    let state = load_state();
    state.monster_calibration
}

// 获取游戏窗口信息
#[tauri::command]
fn get_game_window_info() -> Result<serde_json::Value, String> {
    let windows = xcap::Window::all().map_err(|e| e.to_string())?;
    
    for window in windows {
        let title = window.title();
        if title.to_lowercase().contains("the bazaar") {
            let (x, y, width, height) = (window.x(), window.y(), window.width(), window.height());
            return Ok(serde_json::json!({
                "x": x,
                "y": y,
                "width": width,
                "height": height,
                "title": title
            }));
        }
    }
    
    Err("Game window not found".to_string())
}

// 打开校准窗口
#[tauri::command]
async fn open_calibration_window(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;
    
    // 先检查窗口是否已存在
    if let Some(window) = app.get_webview_window("monster-calibration") {
        let _ = window.set_focus();
        return Ok(());
    }
    
    // 获取游戏窗口信息（物理像素坐标）
    let windows = xcap::Window::all().map_err(|e| e.to_string())?;
    let mut game_window: Option<(i32, i32, u32, u32)> = None;
    
    for window in windows {
        let title = window.title();
        if title.to_lowercase().contains("the bazaar") {
            game_window = Some((window.x(), window.y(), window.width(), window.height()));
            println!("[Calibration] Game window found: x={}, y={}, width={}, height={}", 
                     window.x(), window.y(), window.width(), window.height());
            break;
        }
    }
    
    let (phys_x, phys_y, phys_width, phys_height) = game_window.ok_or("Game window not found".to_string())?;
    
    // 获取主窗口来确定DPI缩放
    let main_window = app.get_webview_window("main").ok_or("Main window not found")?;
    let scale_factor = main_window.scale_factor().map_err(|e| e.to_string())?;
    
    println!("[Calibration] Scale factor: {}", scale_factor);
    
    // 转换为逻辑坐标（Tauri使用逻辑像素）
    let logical_x = (phys_x as f64) / scale_factor;
    let logical_y = (phys_y as f64) / scale_factor;
    let logical_width = (phys_width as f64) / scale_factor;
    let logical_height = (phys_height as f64) / scale_factor;
    
    println!("[Calibration] Logical coordinates: x={}, y={}, width={}, height={}", 
             logical_x, logical_y, logical_width, logical_height);
    
    // 创建校准窗口
    use tauri::WebviewWindowBuilder;
    use tauri::WebviewUrl;
    
    let window = WebviewWindowBuilder::new(
        &app,
        "monster-calibration",
        WebviewUrl::App("index.html".into())
    )
    .title("野怪识别校准")
    .inner_size(logical_width, logical_height)
    .position(logical_x, logical_y)
    .resizable(false)
    .decorations(false)
    .transparent(true)
    .always_on_top(true)
    .skip_taskbar(true)
    .visible_on_all_workspaces(true)
    .initialization_script("window.__WINDOW_TYPE__ = 'monster-calibration';")
    .build()
    .map_err(|e| e.to_string())?;
    
    // 设置为不可点击穿透（校准时需要交互）
    window.set_ignore_cursor_events(false).map_err(|e| e.to_string())?;
    
    println!("[Calibration] Window created successfully");
    
    Ok(())
}

// 关闭校准窗口
#[tauri::command]
async fn close_calibration_window(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;
    
    if let Some(window) = app.get_webview_window("monster-calibration") {
        window.close().map_err(|e| e.to_string())?;
    }
    
    Ok(())
}

#[tauri::command]
fn get_detail_display_hotkey() -> Option<i32> {
    load_state().detail_display_hotkey
}

#[tauri::command]
fn set_detail_display_hotkey(hotkey: i32) {
    let mut state = load_state();
    state.detail_display_hotkey = Some(hotkey);
    save_state(&state);
    update_detail_hotkey_cache(Some(hotkey));
    println!("[Config] Detail display hotkey updated to: {}", hotkey);
}

fn calculate_day_from_log(content: &str, _hours: u32, retro: bool) -> Option<u32> {
    let start_pos = if retro { content.rfind("NetMessageRunInitialized").unwrap_or(0) } else { 0 };
    let slice = &content[start_pos..];
    let mut current_day: u32 = 1; // Default to 1
    let mut in_pvp = false;
    let mut hour_count: u32 = 0;

    for line in slice.lines() {
        let l = line.trim();
        if l.contains("NetMessageRunInitialized") {
            current_day = 1; in_pvp = false; hour_count = 0; continue;
        }
        
        if l.contains("to [PVPCombatState]") { in_pvp = true; continue; }

        if in_pvp && l.contains("State changed") && (l.contains("to [ChoiceState]") || l.contains("to [LevelUpState]")) {
            current_day = current_day.saturating_add(1);
            in_pvp = false; hour_count = 0; continue;
        }

        if l.starts_with("[") && l.contains("State changed from [ChoiceState] to [") {
             if !l.contains("to [ChoiceState]") && !l.contains("to [PVPCombatState]") {
                hour_count = hour_count.saturating_add(1);
                if hour_count >= 10 { // Fallback for modes without PVP or unexpected logs
                    current_day = current_day.saturating_add(1);
                    hour_count = 0;
                }
             }
        }
    }
    
    Some(current_day)
}

// --- App Run ---
#[tauri::command]
fn get_yolo_stats() -> serde_json::Value {
    let detections = get_yolo_scan_results().read().unwrap();
    let total = detections.len();
    let items = detections.iter().filter(|d| d.class_id == 2).count(); // item
    let events = detections.iter().filter(|d| d.class_id == 1).count(); // event
    let skills = detections.iter().filter(|d| d.class_id == 6).count(); // skill
    let monster_icons = detections.iter().filter(|d| d.class_id == 3).count(); // monstericon
    
    // 计算怪物数量（event和monstericon重叠的）
    let events_list: Vec<_> = detections.iter().filter(|d| d.class_id == 1).collect();
    let monsters_count = events_list.iter().map(|event| {
        detections.iter().filter(|d| d.class_id == 3).any(|icon| {
            // 检查交集
            let ix1 = event.x1.max(icon.x1);
            let iy1 = event.y1.max(icon.y1);
            let ix2 = event.x2.min(icon.x2);
            let iy2 = event.y2.min(icon.y2);
            let i_area = (ix2 - ix1).max(0) * (iy2 - iy1).max(0);
            let icon_area = (icon.x2 - icon.x1) * (icon.y2 - icon.y1);
            icon_area > 0 && (i_area as f32 / icon_area as f32) > 0.5
        })
    }).filter(|&has_monster| has_monster).count();

    serde_json::json!({
        "total": total,
        "items": items,
        "events": events,
        "monsters": monsters_count,
        "skills": skills,
        "monster_icons": monster_icons
    })
}

#[tauri::command]
async fn invoke_yolo_scan(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    monster_recognition::recognize_monsters_yolo(&app)
}

#[tauri::command]
async fn emit_to_main(app: tauri::AppHandle, event: String, payload: serde_json::Value) -> Result<(), String> {
    app.emit(&event, payload)
        .map_err(|e| format!("Failed to emit event: {}", e))?;
    Ok(())
}



#[tauri::command]
async fn save_detail_popup_geometry(_app: tauri::AppHandle, x: i32, y: i32, width: u32, height: u32) -> Result<(), String> {
    let mut state = load_state();
    
    // Only save if dimensions are reasonable
    if width > 100 && height > 100 {
        state.detail_popup_x = Some(x);
        state.detail_popup_y = Some(y);
        state.detail_popup_width = Some(width);
        state.detail_popup_height = Some(height);
        save_state(&state);
        println!("[Geometry] Saved Detail Popup: x={}, y={}, w={}, h={}", x, y, width, height);
    }
    Ok(())
}

#[tauri::command]
async fn get_sync_state(state: State<'_, DbState>) -> Result<SyncPayload, String> {
    let p_state = load_state();
    let items_db = state.items.read().map_err(|e| e.to_string())?;
    let skills_db = state.skills.read().map_err(|e| e.to_string())?;

    let map_items = |ids: &HashSet<String>| -> Vec<ItemData> {
        ids.iter()
           .filter_map(|iid| {
               let tid = p_state.inst_to_temp.get(iid)?;
               let mut item = lookup_item(tid, &items_db, &skills_db)?;
               item.instance_id = Some(iid.clone());
               Some(item)
           })
           .collect()
    };

    let hand_items = map_items(&p_state.current_hand);
    let stash_items = map_items(&p_state.current_stash);
    let all_tags = items_db.unique_tags.clone();

    Ok(SyncPayload {
        hand_items,
        stash_items,
        all_tags,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    set_panic_hook();
    log_to_file("=================== App Starting ===================");

    // Initialize Overlay Bounds State
    let bounds = Arc::new(std::sync::Mutex::new(Vec::new()));
    let bounds_clone = bounds.clone();

    // 在 macOS 上需要 mut（因为要添加额外的 plugin），在其他平台上不需要但也无害
    #[allow(unused_mut)]
    let mut builder = tauri::Builder::default()
        .manage(OverlayState(bounds))
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.maximize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build());

    // macOS: 添加 tauri-nspanel 插件（用于全屏覆盖）
    #[cfg(target_os = "macos")]
    {
        builder = builder.plugin(tauri_nspanel::init());
    }
    
    let builder = builder;

    builder
        .manage(DbState {
            items: Arc::new(RwLock::new(ItemDb {
                list: Vec::new(),
                id_map: HashMap::new(),
                unique_tags: Vec::new(),
            })),
            skills: Arc::new(RwLock::new(SkillDb {
                list: Vec::new(),
                id_map: HashMap::new(),
            })),
            monsters: Arc::new(RwLock::new(serde_json::Map::new())),
        })
        .setup(move |app| {
            // 初始化热键缓存
            let state = load_state();
            update_detail_hotkey_cache(state.detail_display_hotkey);
            update_detection_hotkey_cache(state.detection_hotkey);
            update_card_detection_hotkey_cache(state.card_detection_hotkey);
            update_toggle_collapse_hotkey_cache(state.toggle_collapse_hotkey);
            update_yolo_hotkey_cache(state.yolo_hotkey);
            
            let handle = app.handle().clone();
            log_system_info(&handle);

            // System Tray Setup
            {
                use tauri::Manager; // For get_webview_window
                use tauri::menu::{Menu, MenuItem};
                use tauri::tray::{TrayIconBuilder, TrayIconEvent, MouseButton, MouseButtonState};
                
                let quit_i = MenuItem::with_id(app, "quit", "Exit BazaarHelper", true, None::<&str>)?;
                let show_i = MenuItem::with_id(app, "show", "Show Main Window", true, None::<&str>)?;
                let reset_i = MenuItem::with_id(app, "reset", "复位", true, None::<&str>)?;
                let menu = Menu::with_items(app, &[&show_i, &reset_i, &quit_i])?;

                let icon = app.default_window_icon().cloned().expect("No default icon found");

                let _tray = TrayIconBuilder::new()
                    .icon(icon) 
                    .tooltip("BazaarHelper")
                    .menu(&menu)
                    .show_menu_on_left_click(false)
                    .on_menu_event(|app, event| {
                        match event.id.as_ref() {
                            "quit" => {
                                app.exit(0);
                            }
                            "show" => {
                                if let Some(window) = app.get_webview_window("main") {
                                    let _ = window.show();
                                    let _ = window.set_focus();
                                }
                            }
                            "reset" => {
                                // 调用复位命令
                                let app_clone = app.clone();
                                std::thread::spawn(move || {
                                    if let Err(e) = reset_window_geometry(app_clone) {
                                        eprintln!("[Tray] Failed to reset window geometry: {}", e);
                                    }
                                });
                            }
                            _ => {}
                        }
                    })
                    .on_tray_icon_event(|tray, event| {
                        if let TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        } = event {
                            let app = tray.app_handle();
                             if let Some(window) = app.get_webview_window("main") {
                                // Toggle visibility logic
                                if window.is_visible().unwrap_or(false) {
                                    let _ = window.hide();
                                } else {
                                    let _ = window.show();
                                    let _ = window.set_focus();
                                }
                             }
                        }
                    })
                    .build(app)?;
            }

            // macOS: 设置为 Accessory 模式（隐藏 dock 图标）
            // 这对于让窗口显示在全屏应用上方是必要的
            #[cfg(target_os = "macos")]
            {
                app.set_activation_policy(tauri::ActivationPolicy::Accessory);
                println!("[macOS] Set activation policy to Accessory (dock hidden)");
            }

            // --- Helper: Hide from Alt-Tab (ToolWindow Style) & Remove White Bar ---
            if let Some(window) = app.get_webview_window("main") {
                // Initial Geometry Restore (Backend Side)
                // let state = load_state();
                // println!("[Geometry] Restoring main window: x={:?}, y={:?}, w={:?}, h={:?}", state.main_window_x, state.main_window_y, state.main_window_width, state.main_window_height);

                // if let (Some(x), Some(y)) = (state.main_window_x, state.main_window_y) {
                //     let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
                // }
                // if let (Some(w), Some(h)) = (state.main_window_width, state.main_window_height) {
                //     // Only restore size if valid
                //      if w > 100 && h > 100 {
                //         let _ = window.set_size(tauri::PhysicalSize::new(w, h));
                //     }
                // }

                // 使用温和的样式处理，保留调整大小能力
                apply_main_window_style(&window);
                
                // Aggressively remove menu for this window
                let _ = window.remove_menu();

                // 延迟刷新窗口外观，确保样式完全应用（避免白边）
                #[cfg(target_os = "windows")]
                {
                    let window_clone = window.clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_millis(50));
                        if let Ok(hwnd) = window_clone.hwnd() {
                            unsafe {
                                use windows::Win32::Foundation::HWND as HWND_TYPE;

                                let hwnd_val = HWND_TYPE(hwnd.0 as _);
                                // let _ = ShowWindow(hwnd_val, SW_HIDE);
                                // let _ = ShowWindow(hwnd_val, SW_SHOW);
                                let _ = SetWindowPos(
                                    hwnd_val,
                                    None,
                                    0, 0, 0, 0,
                                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED
                                );
                            }
                        }
                    });
                }
            }

            // --- Game Focus Monitor Thread ---
            let handle_focus = handle.clone();
            std::thread::spawn(move || {
                let mut overlay_was_visible = false;

                loop {
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    
                    let game_active = is_game_window_active();
                    if game_active {
                        update_last_foreground_window();
                    }
                    let mut app_active = false;
                    
                    // Check if any of our windows allow focus and are active (like DetailPopup if interacting)
                    #[cfg(target_os = "windows")]
                    if !game_active {
                        unsafe {
                            use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
                            // use windows::Win32::Foundation::HWND as HWND_TYPE; 
                            let fg_hwnd = GetForegroundWindow().0;
                            
                            // Check 'main', 'yolo-monitor', 'detail-popup'
                            let check_windows = ["main", "yolo-monitor", "detail-popup", "monster-calibration"];
                            for label in check_windows {
                                if let Some(win) = handle_focus.get_webview_window(label) {
                                    if let Ok(hwnd) = win.hwnd() {
                                        if hwnd.0 as isize == fg_hwnd as isize {
                                            app_active = true;
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    let mut should_be_visible = game_active || app_active;
                    
                    // If card recognition is active, prevent hiding overlays even if focus is lost momentarily
                    if crate::IS_RECOGNIZING.load(Ordering::Relaxed) {
                        should_be_visible = true;
                    } else {
                        // Check grace period (1.0 second after recognition finishes)
                        if let Some(lock) = crate::LAST_RECOG_TIME.get() {
                            if let Ok(guard) = lock.read() {
                                if let Some(time) = *guard {
                                    if time.elapsed().as_secs_f32() < 1.0 {
                                        should_be_visible = true;
                                    }
                                }
                            }
                        }
                    }

                    if should_be_visible != overlay_was_visible {
                        println!("[Focus Monitor] Visibility state changing: {} -> {} (Game: {}, App: {})", 
                                 overlay_was_visible, should_be_visible, game_active, app_active);
                        
                        if !should_be_visible {
                             // Lost focus to OTHER app -> Hide overlays
                            if let Some(window) = handle_focus.get_webview_window("yolo-monitor") {
                                let _ = window.hide();
                            }
                            if let Some(window) = handle_focus.get_webview_window("detail-popup") {
                                if window.is_visible().unwrap_or(false) {
                                     let _ = window.hide();
                                }
                            }
                        } else {
                            // Gained focus (Game or App) -> Restore Yolo Monitor (if enabled)
                             let state = load_state();
                             if state.show_yolo_monitor {
                                if let Some(window) = handle_focus.get_webview_window("yolo-monitor") {
                                    let _ = window.show();
                                    // 确保显示时应用样式
                                    apply_no_activate_style(&window);
                                }
                            }
                            // Note: We don't auto-restore DetailPopup, it opens on demand
                        }
                        overlay_was_visible = should_be_visible;
                    }
                }
            });

            // --- Log Monitor Thread ---
            let db_state = app.state::<DbState>();
            let thread_items_db = db_state.items.clone();
            let thread_skills_db = db_state.skills.clone();
            let log_handle = handle.clone();
            
            std::thread::spawn(move || {
                let handle = log_handle;
                let log_path = get_log_path();
                let prev_path = get_prev_log_path(); // Add prev path handling
                
                let re_purchase = Regex::new(r"Card Purchased: InstanceId:\s*(?P<iid>[^ ]+)\s*-\s*TemplateId\s*(?P<tid>[^ ]+)(?:.*Target:(?P<tgt>[^ ]+))?(?:.*Section(?P<sec>[^ ]+))?").unwrap();
                let re_id = Regex::new(r"ID: \[(?P<id>[^\]]+)\]").unwrap();
                let re_tid = Regex::new(r"TemplateId: \[(?P<tid>[^\]]+)\]").unwrap();
                let re_owner = Regex::new(r"- Owner: \[(?P<val>[^\]]+)\]").unwrap();
                let re_section = Regex::new(r"- Section: \[(?P<val>[^\]]+)\]").unwrap();
                
                let re_item_id = Regex::new(r"itm_[A-Za-z0-9_-]+").unwrap();
                let re_sold = Regex::new(r"Sold Card\s+(?P<iid>itm_[^ ]+)").unwrap();
                let re_removed = Regex::new(r"Successfully removed item\s+(?P<iid>itm_[^ ]+)").unwrap();
                let re_moved_to = Regex::new(r"Successfully moved card\s+(?P<iid>itm_[^ ]+)\s+to\s+(?P<tgt>[^ ]+)").unwrap();

                // Initialize state
                let state_init = load_state();
                let mut inst_to_temp = state_init.inst_to_temp;
                let mut current_hand = state_init.current_hand;
                let mut current_stash = state_init.current_stash;
                let mut current_day = state_init.day;
                
                let mut in_pvp = false;
                let mut is_sync = false;
                let mut last_iid = String::new();
                let mut cur_owner = String::new();

                // --- Initial Sync: Replay Logs to catch up with current state ---
                println!("[LogMonitor] Initializing state from logs...");
                
                // Clear state for fresh scan (we'll recover inst_to_temp from logs too)
                // Note: Keep cached day if it's valid, effectively we trust logs more though.
                current_hand.clear();
                current_stash.clear();
                // inst_to_temp.clear(); // Keep existing mapping as backup

                let files_to_process = vec![prev_path, log_path.clone()];
                for path in files_to_process {
                    if !path.exists() { 
                        println!("[LogMonitor] Skipping non-existent file: {:?}", path);
                        continue; 
                    }
                    println!("[LogMonitor] Processing log file: {:?}", path);
                    if let Ok(file) = File::open(&path) {
                        let reader = BufReader::new(file);
                        for line in reader.lines() {
                            if let Ok(l) = line {
                                let trimmed = l.trim();
                                
                                // Reset everything if we see a new run start
                                if trimmed.contains("NetMessageRunInitialized") {
                                    current_day = 1; in_pvp = false;
                                    inst_to_temp.clear();
                                    current_hand.clear();
                                    current_stash.clear();
                                    is_sync = false;
                                }

                                if trimmed.contains("to [PVPCombatState]") { in_pvp = true; }
                                if in_pvp && trimmed.contains("State changed") && (trimmed.contains("to [ChoiceState]") || trimmed.contains("to [LevelUpState]")) {
                                    current_day = current_day.saturating_add(1); in_pvp = false;
                                }

                                if let Some(cap) = re_purchase.captures(trimmed) {
                                    let iid = cap["iid"].to_string();
                                    inst_to_temp.insert(iid.clone(), cap["tid"].to_string());
                                    let mut section = cap.name("sec").map(|s| s.as_str().to_string());
                                    if section.as_deref().unwrap_or("") == "" {
                                        if let Some(tgt) = cap.name("tgt").map(|t| t.as_str()) {
                                            if tgt.contains("PlayerStorageSocket") { section = Some("Stash".to_string()); }
                                            else if tgt.contains("PlayerSocket") { section = Some("Player".to_string()); }
                                        }
                                    }
                                    if let Some(s) = section {
                                        if s == "Player" || s == "Hand" { current_hand.insert(iid); }
                                        else if s == "Stash" || s == "Storage" || s == "PlayerStorage" { current_stash.insert(iid); }
                                    }
                                }
                                if let Some(cap) = re_moved_to.captures(trimmed) {
                                    let iid = cap["iid"].to_string();
                                    if cap["tgt"].contains("StorageSocket") {
                                        current_stash.insert(iid.clone()); current_hand.remove(&iid);
                                    } else if cap["tgt"].contains("Socket") {
                                        current_hand.insert(iid.clone()); current_stash.remove(&iid);
                                    }
                                }
                                if let Some(cap) = re_sold.captures(trimmed) {
                                    let iid = cap["iid"].to_string(); 
                                    current_hand.remove(&iid); current_stash.remove(&iid);
                                }
                                if let Some(cap) = re_removed.captures(trimmed) {
                                    let iid = cap["iid"].to_string(); 
                                    current_hand.remove(&iid); current_stash.remove(&iid);
                                }
                                if trimmed.contains("Cards Disposed:") {
                                    for mat in re_item_id.find_iter(trimmed) {
                                        let iid = mat.as_str().to_string(); 
                                        current_hand.remove(&iid); current_stash.remove(&iid);
                                    }
                                }
                                if trimmed.contains("Cards Spawned:") || trimmed.contains("Cards Dealt:") || trimmed.contains("NetMessageGameStateSync") { 
                                    is_sync = true; 
                                }
                                if is_sync {
                                    if let Some(cap) = re_id.captures(trimmed) { last_iid = cap["id"].to_string(); }
                                    else if let Some(cap) = re_tid.captures(trimmed) {
                                        if !last_iid.is_empty() {
                                            inst_to_temp.insert(last_iid.clone(), cap["tid"].to_string());
                                        }
                                    }
                                    else if let Some(cap) = re_owner.captures(trimmed) { cur_owner = cap["val"].to_string(); }
                                    else if let Some(cap) = re_section.captures(trimmed) {
                                        if !last_iid.is_empty() && &cur_owner == "Player" && last_iid.starts_with("itm_") {
                                            let sec_val = &cap["val"];
                                            if sec_val == "Hand" || sec_val == "Player" { 
                                                current_hand.insert(last_iid.clone()); 
                                                current_stash.remove(&last_iid);
                                            }
                                            else if sec_val == "Stash" || sec_val == "Storage" || sec_val == "PlayerStorage" { 
                                                current_stash.insert(last_iid.clone()); 
                                                current_hand.remove(&last_iid);
                                            }
                                            else {
                                                current_hand.remove(&last_iid); 
                                                current_stash.remove(&last_iid);
                                            }
                                        }
                                        last_iid.clear(); cur_owner.clear();
                                    }
                                    else if trimmed.contains("Finished processing") { is_sync = false; }
                                }
                            }
                        }
                    }
                }

                save_state(&PersistentState {
                    day: current_day,
                    inst_to_temp: inst_to_temp.clone(),
                    current_hand: current_hand.clone(),
                    current_stash: current_stash.clone(),
                    ..load_state()
                });

                // Initial UI Sync after loading/backfilling
                let init_handle = handle.clone();
                let init_items_db = thread_items_db.clone();
                let init_skills_db = thread_skills_db.clone();
                let init_hand = current_hand.clone();
                let init_stash = current_stash.clone();
                let init_map = inst_to_temp.clone();
                let init_day = current_day;
                
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
                    let _ = init_handle.emit("day-update", init_day);
                    let items_db = init_items_db.read().unwrap();
                    let skills_db = init_skills_db.read().unwrap();
                    let map_items = |ids: &HashSet<String>| -> Vec<ItemData> {
                        ids.iter()
                           .filter_map(|iid| {
                               let tid = init_map.get(iid)?;
                               let mut item = lookup_item(tid, &items_db, &skills_db)?;
                               item.instance_id = Some(iid.clone());
                               Some(item)
                           })
                           .collect()
                    };
                    let hand_items = map_items(&init_hand);
                    let stash_items = map_items(&init_stash);
                    let all_tags = items_db.unique_tags.clone();
                    let _ = init_handle.emit("sync-items", SyncPayload { hand_items, stash_items, all_tags });
                });

                let mut last_size = 0u64;
                if let Ok(meta) = std::fs::metadata(&log_path) {
                    last_size = meta.len();
                }

                println!("[Log Monitor] Initialization complete. Starting main monitoring loop from size: {}", last_size);

                loop {
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    
                    if let Ok(mut f) = File::open(&log_path) {
                        if let Ok(meta) = f.metadata() {
                            let len = meta.len();
                            if len < last_size {
                                last_size = 0;
                                in_pvp = false;
                                is_sync = false;
                            }
                            if len > last_size {
                                if let Ok(_) = f.seek(SeekFrom::Start(last_size)) {
                                    let mut buf = Vec::new();
                                    if let Ok(_) = f.take(1_000_000).read_to_end(&mut buf) {
                                        let new_content = String::from_utf8_lossy(&buf);
                                        let mut changed = false;
                                        let mut day_changed = false;

                                        for line in new_content.lines() {
                                            let trimmed = line.trim();
                                            
                                            if trimmed.contains("NetMessageRunInitialized") {
                                                current_day = 1; in_pvp = false; day_changed = true;
                                                inst_to_temp.clear(); current_hand.clear(); current_stash.clear();
                                                changed = true;
                                            }
                                            
                                            if trimmed.contains("to [PVPCombatState]") { in_pvp = true; }
                                            if in_pvp && trimmed.contains("State changed") && (trimmed.contains("to [ChoiceState]") || trimmed.contains("to [LevelUpState]")) {
                                                current_day = current_day.saturating_add(1);
                                                in_pvp = false; day_changed = true;
                                            }

                                            if let Some(cap) = re_purchase.captures(trimmed) {
                                                let iid = cap["iid"].to_string();
                                                inst_to_temp.insert(iid.clone(), cap["tid"].to_string());
                                                
                                                let mut section = cap.name("sec").map(|s| s.as_str().to_string());
                                                let target = cap.name("tgt").map(|t| t.as_str());
                                                
                                                if section.as_deref().unwrap_or("") == "" {
                                                    if let Some(tgt) = target {
                                                        if tgt.contains("PlayerStorageSocket") { section = Some("Stash".to_string()); }
                                                        else if tgt.contains("PlayerSocket") { section = Some("Player".to_string()); }
                                                    }
                                                }
                                                if let Some(s) = section {
                                                    if s == "Player" || s == "Hand" { current_hand.insert(iid); changed = true; }
                                                    else if s == "Stash" || s == "Storage" || s == "PlayerStorage" { current_stash.insert(iid); changed = true; }
                                                }
                                            }
                                            
                                            if let Some(cap) = re_moved_to.captures(trimmed) {
                                                let iid = cap["iid"].to_string();
                                                let tgt = &cap["tgt"];
                                                if tgt.contains("StorageSocket") {
                                                    current_stash.insert(iid.clone()); current_hand.remove(&iid); changed = true;
                                                } else if tgt.contains("Socket") {
                                                    current_hand.insert(iid.clone()); current_stash.remove(&iid); changed = true;
                                                }
                                            }

                                            if let Some(cap) = re_sold.captures(trimmed) {
                                                let iid = cap["iid"].to_string();
                                                if current_hand.remove(&iid) || current_stash.remove(&iid) { changed = true; }
                                            }
                                            if let Some(cap) = re_removed.captures(trimmed) {
                                                let iid = cap["iid"].to_string();
                                                if current_hand.remove(&iid) || current_stash.remove(&iid) { changed = true; }
                                            }
                                            if trimmed.contains("Cards Disposed:") {
                                                for mat in re_item_id.find_iter(trimmed) {
                                                    let iid = mat.as_str().to_string();
                                                    if current_hand.remove(&iid) || current_stash.remove(&iid) { changed = true; }
                                                }
                                            }

                                            if trimmed.contains("Cards Spawned:") || trimmed.contains("Cards Dealt:") || trimmed.contains("NetMessageGameStateSync") || trimmed.contains("Successfully moved card to:") {
                                                is_sync = true;
                                            }
                                            
                                            if is_sync {
                                                if let Some(cap) = re_id.captures(trimmed) { last_iid = cap["id"].to_string(); }
                                                else if let Some(cap) = re_tid.captures(trimmed) {
                                                    if !last_iid.is_empty() {
                                                        inst_to_temp.insert(last_iid.clone(), cap["tid"].to_string());
                                                    }
                                                }
                                                else if let Some(cap) = re_owner.captures(trimmed) { cur_owner = cap["val"].to_string(); }
                                                else if let Some(cap) = re_section.captures(trimmed) {
                                                    if !last_iid.is_empty() && &cur_owner == "Player" && last_iid.starts_with("itm_") {
                                                        let sec_val = &cap["val"];
                                                        if sec_val == "Hand" || sec_val == "Player" { 
                                                            current_hand.insert(last_iid.clone()); current_stash.remove(&last_iid);
                                                        } else if sec_val == "Stash" || sec_val == "Storage" || sec_val == "PlayerStorage" { 
                                                            current_stash.insert(last_iid.clone()); current_hand.remove(&last_iid);
                                                        } else {
                                                            current_hand.remove(&last_iid); current_stash.remove(&last_iid);
                                                        }
                                                        changed = true;
                                                    }
                                                    last_iid.clear(); cur_owner.clear();
                                                }
                                                else if trimmed.contains("Finished processing") { is_sync = false; changed = true; }
                                            }
                                        }

                                        if changed || day_changed {
                                            if day_changed {
                                                let _ = handle.emit("day-update", current_day);
                                            }
                                            
                                            if changed {
                                                let items_db = thread_items_db.read().unwrap();
                                                let skills_db = thread_skills_db.read().unwrap();
                                                
                                                let map_items = |ids: &HashSet<String>| -> Vec<ItemData> {
                                                    ids.iter()
                                                       .filter_map(|iid| {
                                                           let tid = inst_to_temp.get(iid)?;
                                                           let mut item = lookup_item(tid, &items_db, &skills_db)?;
                                                           item.instance_id = Some(iid.clone());
                                                           Some(item)
                                                       })
                                                       .collect()
                                                };

                                                let hand_items = map_items(&current_hand);
                                                let stash_items = map_items(&current_stash);
                                                let all_tags = items_db.unique_tags.clone();
                                                
                                                let _ = handle.emit("sync-items", SyncPayload { hand_items, stash_items, all_tags });
                                                
                                                save_state(&PersistentState {
                                                    day: current_day,
                                                    inst_to_temp: inst_to_temp.clone(),
                                                    current_hand: current_hand.clone(),
                                                    current_stash: current_stash.clone(),
                                                    ..load_state()
                                                });
                                            }
                                        }
                                        last_size = len;
                                    }
                                }
                            }
                        }
                    }
                }
            });

            // --- Helper: Start Mouse Monitor Thread (Global Click Detection Only) ---
            let handle_monitor = handle.clone();
            let _bounds_monitor = bounds_clone.clone();

            std::thread::spawn(move || {
                let device_state = DeviceState::new();
                let mut last_left_click = false;
                let mut _last_right_click = false;
                let mut last_trigger_active = false;
                
                let mut last_yolo_active = false;
                let mut last_detection_active = false;
                let mut last_card_active = false;
                let mut last_collapse_active = false;

                loop {
                    // Short sleep to avoid pegging CPU
                    std::thread::sleep(time::Duration::from_millis(50));

                    // Only process clicks if the game window is active OR app window is active
                    // But for "Click Outside to Hide", we mostly care if Detail Popup is visible
                    let detail_visible = if let Some(w) = handle_monitor.get_webview_window("detail-popup") {
                        w.is_visible().unwrap_or(false)
                    } else { false };

                    let game_active = is_game_window_active();
                    
                    // For mouse clicks and detail popup trigger, require game focus
                    if !game_active && !detail_visible {
                        // Reset mouse click state when game is not focused AND detail popup not open
                        last_left_click = false;
                        _last_right_click = false;
                        last_trigger_active = false;
                        // Note: We do NOT reset recognition hotkey states here
                        // to allow recognition even when not focused on game
                    }
                    
                    // Get mouse state for hotkey detection (always needed)
                    let mouse: MouseState = device_state.get_mouse();
                    let mx = mouse.coords.0;
                    let my = mouse.coords.1;
                    let left_click = mouse.button_pressed[1]; // Left Click
                    let right_click = mouse.button_pressed[3]; // Right Click
                    
                    // Process mouse clicks only when game is active or detail popup visible
                    if game_active || detail_visible {
                    // Mouse processing section - only when game is focused

                    // [Global Click Handler for Detail Popup]
                    // If popup is visible, any click (Left/Right) should hide it (unless clicking inside?)
                    // User request: "popup弹出来之后，点击鼠标右键左键... 需要能够把这个popup隐藏起来"
                    if detail_visible && (left_click || right_click) {
                        // Debounce slightly to allow the click that OPENED it to finish
                        // Check if click is outside detail-popup
                        if let Some(popup_window) = handle_monitor.get_webview_window("detail-popup") {
                             if let (Ok(pos), Ok(size)) = (popup_window.outer_position(), popup_window.outer_size()) {
                                 let px = pos.x;
                                 let py = pos.y;
                                 let pw = size.width as i32;
                                 let ph = size.height as i32;
                                 
                                 // Check collision
                                 let inside = mx >= px && mx <= px + pw && my >= py && my <= py + ph;
                                 
                                 // Only hide if clicked OUTSIDE (or user wants any click to hide?)
                                 // "点击鼠标右键左键... 需要能够把这个popup隐藏起来"
                                 // Usually implies "Dismissal". 
                                 // If I click INSIDE, I probably want to interact (copy text etc).
                                 // So I'll implement: Click OUTSIDE hides it.
                                 // Warning: If user clicks GAME, that is OUTSIDE.
                                 if !inside {
                                     // Double check previous state to trigger on "press" or "release"?
                                     // Trigger on PRESS (when `left_click` IS true)
                                     if (left_click && !last_left_click) || (right_click && !_last_right_click) {
                                        println!("[Mouse Monitor] Click detected outside detail-popup at ({}, {}), hiding.", mx, my);
                                        let _ = popup_window.emit("hide-detail-popup", ());
                                        // Also likely want to pass the click through to game if it's transparent, 
                                        // but device_query doesn't consume events, so it passes naturally.
                                     }
                                 }
                             }
                        }
                    }

                    // --- Click Outside to Hide Logic ---
                    // RE MOVED duplicate logic - merged into Global Click Handler above
                    // let left_click = mouse.button_pressed.get(0).copied().unwrap_or(false);
                    // if left_click ... (removed)

                    } // End of mouse processing section (game_active || detail_visible)

                    // --- Detail Popup Custom Hotkey (always check) ---
                    let hotkey_setup = get_cached_detail_hotkey();
                    let trigger_active = if let Some(code) = hotkey_setup {
                        is_key_pressed(code, &device_state, &mouse)
                    } else {
                        false
                    };

                    // Implement "Click or Hotkey hides popup"
                    // If popup is visible and hotkey is pressed -> Hide it
                    if detail_visible && trigger_active && !last_trigger_active {
                        if let Some(popup_window) = handle_monitor.get_webview_window("detail-popup") {
                             println!("[Hotkey Monitor] Custom hotkey pressed while popup open - hiding.");
                             let _ = popup_window.emit("hide-detail-popup", ());
                        }
                    }

                    if trigger_active && !last_trigger_active && (game_active || detail_visible) {
                        println!("[Global Hotkey] Detail Popup Hotkey pressed at ({}, {})", mx, my);
                        update_last_foreground_window();
                        
                        let handle_clone = handle_monitor.clone();
                        let click_x = mx;
                        let click_y = my;
                        
                        tauri::async_runtime::spawn(async move {
                            match handle_overlay_right_click(handle_clone.clone(), click_x, click_y).await {
                                Ok(Some(result)) => {
                                    println!("[Right Click] Found result: {:?}", result.get("type"));
                                    
                                    let result_type = result.get("type").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                                    let data = result.get("data").cloned().unwrap_or(serde_json::json!({}));
                                    
                                    if let Err(e) = show_detail_popup_at(handle_clone, click_x, click_y, result_type, data).await {
                                        println!("[Right Click] Error showing detail popup: {}", e);
                                    }
                                }
                                Ok(None) => { /* No object found, do nothing */ }
                                Err(e) => {
                                    println!("[Right Click] Error handling click: {}", e);
                                }
                            }
                        });
                    }

                    // --- YOLO Hotkey Logic ---
                    let yolo_active = if let Some(code) = get_cached_yolo_hotkey() {
                        is_key_pressed(code, &device_state, &mouse)
                    } else { false };
                    
                    if yolo_active && !last_yolo_active {
                        println!("[Global Hotkey] YOLO Trigger pressed!");
                        let h = handle_monitor.clone();
                        tauri::async_runtime::spawn(async move {
                            println!("[Global Hotkey] Emitting 'yolo_hotkey_pressed'");
                            let _ = h.emit("yolo_hotkey_pressed", ());
                        });
                    }

                    // --- Monster Recognition Hotkey Logic ---
                    // This section is kept for potential future use or cleanup.
                    // The actual mouse-based monster recognition is now handled by the 'Monster Mouse Trigger' block below.
                    // (Old logic removed)

                    // --- Monster Recognition Hotkey Logic (Mouse Based) ---
                    // Changed from screen-scan to mouse-hover recognition as per user request
                    let detection_active = if let Some(code) = get_cached_detection_hotkey() {
                        is_key_pressed(code, &device_state, &mouse)
                    } else { false };

                    if detection_active && !last_detection_active {
                        println!("[Global Hotkey] Monster Mouse Trigger pressed!");
                         // Prevent hiding
                        crate::IS_RECOGNIZING.store(true, Ordering::Relaxed);
                        crate::update_last_recog_time();

                        let h = handle_monitor.clone();
                         tauri::async_runtime::spawn(async move {
                            match crate::monster_recognition::recognize_monster_at_mouse().await {
                                Ok(Some(result)) => {
                                    // Result format: "Day X|MonsterName"
                                    if let Some((day_str, monster_name)) = result.split_once('|') {
                                        // Extract day number from "Day X" or "Day 10+"
                                        let day_num = if day_str.contains("10+") {
                                            10
                                        } else {
                                            day_str.replace("Day ", "").trim().parse::<u32>().unwrap_or(1)
                                        };
                                        
                                        println!("[Hotkey Monster] Matched: {} on Day {}", monster_name, day_num);
                                        let _ = h.emit("auto-jump-to-monster", serde_json::json!({
                                            "day": day_num,
                                            "monster_name": monster_name
                                        }));
                                    }
                                }
                                Ok(None) => println!("[Hotkey Monster] No object found at cursor"),
                                Err(e) => println!("[Hotkey Monster] Error: {}", e),
                            }
                        });
                    }

                    // --- Card Recognition Hotkey Logic ---
                    let card_active = if let Some(code) = get_cached_card_detection_hotkey() {
                        is_key_pressed(code, &device_state, &mouse)
                    } else { false };
                    
                    if card_active && !last_card_active {
                        println!("[Global Hotkey] Card Recognition Trigger pressed!");
                        
                        // Immediately flag that recognition is expected to prevent overlay hiding
                        crate::IS_RECOGNIZING.store(true, Ordering::Relaxed);
                        crate::update_last_recog_time(); // Also tick the timestamp just in case

                        let h = handle_monitor.clone();
                        tauri::async_runtime::spawn(async move {
                            println!("[Global Hotkey] Emitting 'hotkey-card'");
                             let _ = h.emit("hotkey-card", ());
                        });
                    }
                    
                    // --- Toggle Collapse Hotkey Logic ---
                    let collapse_active = if let Some(code) = get_cached_toggle_collapse_hotkey() {
                        is_key_pressed(code, &device_state, &mouse)
                    } else { false };
                    
                    if collapse_active && !last_collapse_active {
                        println!("[Global Hotkey] Collapse/Expand Trigger pressed!");
                        let h = handle_monitor.clone();
                        tauri::async_runtime::spawn(async move {
                             println!("[Global Hotkey] Emitting 'hotkey-collapse'");
                             let _ = h.emit("hotkey-collapse", ());
                        });
                    }

                    last_trigger_active = trigger_active;
                    last_left_click = left_click;
                    _last_right_click = right_click;
                    
                    last_yolo_active = yolo_active;
                    last_detection_active = detection_active;
                    last_card_active = card_active;
                    last_collapse_active = collapse_active;
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            update_overlay_bounds,
            abort_yolo_scan,
            set_show_yolo_monitor,
            trigger_yolo_scan,
            handle_overlay_right_click,
            get_item_info,
            set_overlay_ignore_cursor,
            show_yolo_monitor_window,
            show_detail_popup_at,
            hide_detail_popup,
            reset_detail_popup_position,
            restore_game_focus,
            was_last_foreground_game,
            get_show_yolo_monitor,
            start_template_loading,
            search_items,
            get_all_monsters,
            debug_monsters_db,
            clear_yolo_cache,
            debug_resource_paths,
            recognize_monsters_from_screenshot,
            get_template_loading_progress,
            get_current_day,
            update_day,
            get_detection_hotkey,
            get_card_detection_hotkey,
            get_toggle_collapse_hotkey,
            set_detection_hotkey,
            set_card_detection_hotkey,
            set_toggle_collapse_hotkey,
            get_yolo_hotkey,
            set_yolo_hotkey,
            reset_all_hotkeys,
            save_monster_calibration,
            load_monster_calibration,
            get_game_window_info,
            open_calibration_window,
            close_calibration_window,
            get_detail_display_hotkey,
            set_detail_display_hotkey,
            get_yolo_stats,
            get_sync_state,
            crate::monster_recognition::recognize_card_at_mouse,
            invoke_yolo_scan,
            emit_to_main,
            save_window_geometry,
            save_detail_popup_geometry,
            get_window_geometry,
            reset_window_geometry
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
