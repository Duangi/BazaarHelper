use std::sync::{Arc, RwLock, OnceLock};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{State, Manager, Emitter};

use serde::{Serialize, Deserialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use regex::Regex;
use std::io::{Read, BufRead, BufReader, Seek, SeekFrom, Write};
use std::fs::File;
use std::{thread, time, panic};
use tokio;
use chrono::Local;

// Windows 特定导入
#[cfg(target_os = "windows")]
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_RBUTTON, VK_MENU};
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongW, SetWindowLongW, SetWindowPos,
    GWL_EXSTYLE, GWL_STYLE,
    WS_EX_TOOLWINDOW, WS_EX_APPWINDOW, WS_EX_WINDOWEDGE, WS_EX_CLIENTEDGE, WS_EX_STATICEDGE,
    WS_EX_LAYERED, WS_EX_NOACTIVATE,
    WS_CAPTION, WS_THICKFRAME, WS_POPUP, WS_SYSMENU, WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_BORDER, WS_DLGFRAME,
    WS_VISIBLE, WS_CLIPSIBLINGS, WS_CLIPCHILDREN,
    SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SWP_NOACTIVATE, SWP_FRAMECHANGED
};
#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute,
    DWMWA_USE_IMMERSIVE_DARK_MODE, DWMWA_BORDER_COLOR, DWMWA_CAPTION_COLOR, DWMWA_TEXT_COLOR
};
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{HWND, COLORREF};

use opencv::core::MatTraitConst;
use device_query::{DeviceQuery, DeviceState, MouseState};

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

// 获取默认热键值（跨平台）
fn default_monster_hotkey() -> i32 {
    #[cfg(target_os = "windows")]
    { VK_RBUTTON.0 as i32 }
    #[cfg(not(target_os = "windows"))]
    { VK_RBUTTON }
}

fn default_card_hotkey() -> i32 {
    #[cfg(target_os = "windows")]
    { VK_MENU.0 as i32 }
    #[cfg(not(target_os = "windows"))]
    { VK_MENU }
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
            new_style |= WS_POPUP.0 | WS_VISIBLE.0 | WS_THICKFRAME.0;
            SetWindowLongW(handle, GWL_STYLE, new_style as i32);

            let current_ex_style = GetWindowLongW(handle, GWL_EXSTYLE) as u32;
            let mut new_ex_style = current_ex_style & !(WS_EX_APPWINDOW.0);
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

            let _: () = msg_send![ns_win, setHidesOnDeactivate: false as BOOL];

            println!("[macOS] Overlay window (fallback) configured with maximum level: {}", max_level);
        }
    }
}

use crate::monster_recognition::{scan_and_identify_monster_at_mouse, YoloDetection};

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
    // Broadcast the show/hide event to all windows; overlay will handle it
    let _ = app.emit("set-show-yolo-monitor", show);
    // Persist preference
    let mut state = load_state();
    state.show_yolo_monitor = show;
    save_state(&state);
    Ok(())
}

#[tauri::command]
fn update_overlay_detail_position(app: tauri::AppHandle, x: i32, y: i32, scale: i32, width: Option<i32>, height: Option<i32>) -> Result<(), String> {
    // Broadcast the position update to overlay window
    let _ = app.emit("update-overlay-detail-position", serde_json::json!({
        "x": x,
        "y": y,
        "scale": scale,
        "width": width.unwrap_or(420),
        "height": height.unwrap_or(600)
    }));
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
    let (window_x, window_y) = *get_yolo_window_offset().read().unwrap();
    
    // 将屏幕坐标转换为相对窗口坐标
    let rel_x = x - window_x;
    let rel_y = y - window_y;
    
    println!("[YOLO Click] Screen coords: ({}, {}), Window offset: ({}, {}), Relative: ({}, {})", 
             x, y, window_x, window_y, rel_x, rel_y);
    
    if img_opt.is_none() {
        return Ok(None);
    }
    let img = img_opt.unwrap();

    // Check for any detection hit (使用相对坐标)
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
            let match_result = monster_recognition::match_card_descriptors(&scene_desc)?;
            if let Some(cards) = match_result {
                let card_list = cards.as_array().unwrap();
                if !card_list.is_empty() {
                    let card_id = card_list[0]["id"].as_str().unwrap_or("").to_string();
                    let db_state = app.state::<DbState>();
                    if let Some(info) = get_item_info_internal(&db_state, card_id).await {
                        return Ok(Some(serde_json::json!({ "type": "item", "data": info })));
                    }
                }
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
#[derive(Debug, Serialize, Deserialize, Clone)]
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
    pub descriptions: Option<Vec<RawSkill>>,
    pub enchantments: Option<serde_json::Value>,
    pub image: Option<String>,
    #[serde(default)]
    pub description_cn: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ItemData {
    pub uuid: String,
    pub name: String,
    pub name_cn: String,
    pub tier: String,
    pub available_tiers: String,
    pub tags: String,
    pub hidden_tags: String,
    pub size: Option<String>,
    pub processed_tags: Vec<String>,
    pub heroes: Vec<String>,
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

        let h_str = raw.heroes.clone().unwrap_or_default();
        let heroes = if h_str.is_empty() {
            vec!["Common".to_string()]
        } else {
            h_str.split('|').map(|s| s.trim().to_string()).collect()
        };

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
            heroes,
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
#[allow(dead_code)]
async fn start_template_loading(app: tauri::AppHandle) -> Result<(), String> {
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
    
    // 异步加载
    tauri::async_runtime::spawn(async move {
        let res_dir_clone = res_dir.clone();
        let cache_dir_clone = cache_dir.clone();
        let _ = monster_recognition::preload_templates_async(res_dir, cache_dir).await;
        let _ = monster_recognition::preload_card_templates_async(res_dir_clone, cache_dir_clone).await;
    });
    
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
async fn set_overlay_ignore_cursor(app: tauri::AppHandle, ignore: bool) -> Result<(), String> {
    if let Some(overlay) = app.get_webview_window("overlay") {
        overlay.set_ignore_cursor_events(ignore).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
async fn restore_game_focus() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, SetForegroundWindow, ShowWindow, SW_SHOW};
        use windows::core::PCWSTR;

        let window_name: Vec<u16> = "The Bazaar\0".encode_utf16().collect();
        unsafe {
            if let Ok(hwnd) = FindWindowW(PCWSTR::null(), PCWSTR(window_name.as_ptr())) {
                if !hwnd.is_invalid() {
                    // 先 ShowWindow 确保不是最小化
                    let _ = ShowWindow(hwnd, SW_SHOW);
                    let _ = SetForegroundWindow(hwnd);
                }
            }
        }
    }
    Ok(())
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

fn lookup_item(tid: &str, items_db: &ItemDb, skills_db: &SkillDb) -> Option<ItemData> {
    if let Some(&index) = items_db.id_map.get(tid) {
        return items_db.list.get(index).cloned();
    }
    if let Some(&index) = skills_db.id_map.get(tid) {
        return skills_db.list.get(index).cloned();
    }
    None
}

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
}

#[tauri::command]
fn set_card_detection_hotkey(hotkey: i32) {
    let mut state = load_state();
    state.card_detection_hotkey = Some(hotkey);
    save_state(&state);
    println!("[Config] Card detection hotkey updated to: {}", hotkey);
}

#[tauri::command]
fn set_toggle_collapse_hotkey(hotkey: i32) {
    let mut state = load_state();
    state.toggle_collapse_hotkey = Some(hotkey);
    save_state(&state);
    println!("[Config] Toggle collapse hotkey updated to: {}", hotkey);
}

#[tauri::command]
fn set_yolo_hotkey(hotkey: i32) {
    let mut state = load_state();
    state.yolo_hotkey = Some(hotkey);
    save_state(&state);
    println!("[Config] YOLO hotkey updated to: {}", hotkey);
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    set_panic_hook();
    log_to_file("=================== App Starting ===================");

    // Initialize Overlay Bounds State
    let bounds = Arc::new(std::sync::Mutex::new(Vec::new()));
    let bounds_clone = bounds.clone();

    let builder = tauri::Builder::default()
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
            let handle = app.handle().clone();
            log_system_info(&handle);

            // macOS: 设置为 Accessory 模式（隐藏 dock 图标）
            // 这对于让窗口显示在全屏应用上方是必要的
            #[cfg(target_os = "macos")]
            {
                app.set_activation_policy(tauri::ActivationPolicy::Accessory);
                println!("[macOS] Set activation policy to Accessory (dock hidden)");
            }

            // --- Helper: Hide from Alt-Tab (ToolWindow Style) & Remove White Bar ---
            if let Some(window) = app.get_webview_window("main") {
                // 使用温和的样式处理，保留调整大小能力
                apply_main_window_style(&window);
                
                // Aggressively remove menu for this window
                let _ = window.remove_menu();

                // Windows: 设置工具窗口样式
                #[cfg(target_os = "windows")]
                if let Ok(hwnd) = window.hwnd() {
                    unsafe {
                        use windows::Win32::Foundation::HWND as HWND_TYPE;
                        let hwnd_val = HWND_TYPE(hwnd.0 as _);
                        let style = GetWindowLongW(hwnd_val, GWL_EXSTYLE);
                        SetWindowLongW(hwnd_val, GWL_EXSTYLE, (style | WS_EX_TOOLWINDOW.0 as i32) & !WS_EX_APPWINDOW.0 as i32);
                    }
                }
            }

            // --- Helper: Start Mouse Monitor Thread (Global Click Detection Only) ---
            let handle_monitor = handle.clone();
            let _bounds_monitor = bounds_clone.clone();

            std::thread::spawn(move || {
                let device_state = DeviceState::new();
                let mut last_right_click = false;

                loop {
                    let mouse: MouseState = device_state.get_mouse();
                    let mx = mouse.coords.0;
                    let my = mouse.coords.1;

                    // 跨平台检测右键点击（使用 device_query）
                    let right_click = mouse.button_pressed[2]; // 右键是索引 2
                    if right_click && !last_right_click {
                        let _ = handle_monitor.emit("global-right-click", serde_json::json!({ "x": mx, "y": my }));
                    }
                    last_right_click = right_click;

                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
            });
            
            // ============== Windows 特定窗口初始化 ==============
            #[cfg(target_os = "windows")]
            {
                use tauri::Manager;
                if let Some(window) = app.get_webview_window("main") {
                    if let Ok(hwnd) = window.hwnd() {
                        use windows::Win32::Foundation::HWND;
                        unsafe {
                            let handle = HWND(hwnd.0 as _);
                            let ex_style = GetWindowLongW(handle, GWL_EXSTYLE);
                            let new_style = (ex_style | WS_EX_NOACTIVATE.0 as i32 | WS_EX_TOOLWINDOW.0 as i32) & !WS_EX_APPWINDOW.0 as i32;
                            SetWindowLongW(handle, GWL_EXSTYLE, new_style);
                        }
                    }
                }
            }

            // ============== macOS 托盘图标（点击激活窗口到当前空间） ==============
            #[cfg(target_os = "macos")]
            {
                let tray_handle = app.handle().clone();
                let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
                let menu = Menu::with_items(app, &[&quit_item])?;

                let _tray = TrayIconBuilder::new()
                    .icon(app.default_window_icon().unwrap().clone())
                    .menu(&menu)
                    .on_menu_event(move |app, event| {
                        if event.id.as_ref() == "quit" {
                            app.exit(0);
                        }
                    })
                    .on_tray_icon_event(move |_tray, event| {
                        if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                            // 点击托盘图标时，显示窗口到当前空间
                            if let Some(main_win) = tray_handle.get_webview_window("main") {
                                let _ = main_win.show();
                                let _ = main_win.set_focus();
                            }
                            if let Some(overlay_win) = tray_handle.get_webview_window("overlay") {
                                let _ = overlay_win.show();
                            }
                        }
                    })
                    .build(app)?;
            }

            // ============== 跨平台 Overlay 初始化 ==============
            if let Some(overlay) = app.get_webview_window("overlay") {
                let _ = overlay.set_ignore_cursor_events(true);

                // macOS: 设置窗口可覆盖全屏应用
                #[cfg(target_os = "macos")]
                setup_macos_fullscreen_overlay(&overlay);

                if let Ok(Some(monitor)) = overlay.primary_monitor() {
                    let size = monitor.size();
                    let position = monitor.position();
                    println!("[Overlay Init] Setting overlay: x={}, y={}, w={}, h={}",
                            position.x, position.y, size.width, size.height);
                    let _ = overlay.set_size(tauri::PhysicalSize::new(size.width, size.height));
                    let _ = overlay.set_position(tauri::PhysicalPosition::new(position.x, position.y));
                } else {
                    println!("[Overlay Init] Using fallback 4K resolution");
                    let _ = overlay.set_size(tauri::PhysicalSize::new(3840, 2160));
                    let _ = overlay.set_position(tauri::PhysicalPosition::new(0, 0));
                }
                let _ = overlay.show();
            }

            // macOS: 主窗口也设置全屏覆盖
            #[cfg(target_os = "macos")]
            if let Some(main_win) = app.get_webview_window("main") {
                setup_macos_fullscreen_overlay(&main_win);
            }

            let handle = app.handle().clone();
            let resources_path = match app.path().resource_dir() {
                Ok(p) => p,
                Err(e) => {
                    log_to_file(&format!("CRITICAL ERROR: Failed to get resource_dir: {}", e));
                    PathBuf::new()
                }
            };
            log_to_file(&format!("Resolved Resources Path: {:?}", resources_path));

            let db_state = app.state::<DbState>();

            // ============== 窗口同步线程（跨平台） ==============
            let sync_handle = app.handle().clone();
            std::thread::spawn(move || {
                let mut was_game_running = true;

                loop {
                    std::thread::sleep(std::time::Duration::from_millis(500));

                    // 使用 xcap 跨平台查找游戏窗口
                    let game_window = xcap::Window::all()
                        .ok()
                        .and_then(|windows| {
                            windows.into_iter().find(|w| w.title().contains("The Bazaar"))
                        });

                    let main_win = sync_handle.get_webview_window("main");
                    let overlay_win = sync_handle.get_webview_window("overlay");

                    if let Some(_game_win) = game_window {
                        // 游戏正在运行
                        if !was_game_running {
                            // 游戏刚启动，显示窗口
                            if let Some(ref w) = main_win {
                                let _ = w.show();
                                let _ = w.set_always_on_top(true);
                            }
                            if let Some(ref w) = overlay_win {
                                let _ = w.show();
                                let _ = w.set_always_on_top(true);
                            }
                        }
                        was_game_running = true;
                    } else {
                        // 游戏没运行
                        if was_game_running {
                            if let Some(ref w) = overlay_win {
                                let _ = w.hide();
                            }
                            was_game_running = false;
                        }
                    }
                }
            });

            // 1. Load Items DB
            let items_possible_paths = [
                resources_path.join("resources").join("items_db.json"),
                resources_path.join("items_db.json"),
            ];
            log_to_file("Attempting to load Items DB...");
            for path in &items_possible_paths {
                log_to_file(&format!("Checking path: {:?}", path));
                if path.exists() {
                     match std::fs::read_to_string(path) {
                        Ok(json) => {
                            match serde_json::from_str::<Vec<RawItem>>(&json) {
                                Ok(raw_list) => {
                                    let items_list: Vec<ItemData> = raw_list.into_iter().map(ItemData::from).collect();
                                    let mut id_map = HashMap::new();
                                    let mut tag_set = std::collections::HashSet::new();
                                    for (index, item) in items_list.iter().enumerate() {
                                        id_map.insert(item.uuid.clone(), index);
                                        for tag in &item.processed_tags { tag_set.insert(tag.clone()); }
                                    }
                                    let mut unique_tags: Vec<String> = tag_set.into_iter().collect();
                                    unique_tags.sort();
                                    let count = items_list.len();
                                    let mut db = db_state.items.write().unwrap();
                                    db.list = items_list;
                                    db.id_map = id_map;
                                    db.unique_tags = unique_tags;
                                    log_to_file(&format!("[Init] Successfully loaded {} items from {:?}", count, path));
                                    break;
                                },
                                Err(e) => log_to_file(&format!("Error parsing items_db.json: {}", e)),
                            }
                        },
                        Err(e) => log_to_file(&format!("Error reading items_db.json: {}", e)),
                    }
                } else {
                    log_to_file("Path does not exist.");
                }
            }

            // 2. Load Skills DB
            let skills_possible_paths = [
                resources_path.join("resources").join("skills_db.json"),
                resources_path.join("skills_db.json"),
            ];
            log_to_file("Attempting to load Skills DB...");
            for path in &skills_possible_paths {
                log_to_file(&format!("Checking path: {:?}", path));
                if path.exists() {
                    match std::fs::read_to_string(path) {
                        Ok(json) => {
                            match serde_json::from_str::<Vec<RawItem>>(&json) {
                                Ok(raw_list) => {
                                    let skills_list: Vec<ItemData> = raw_list.into_iter().map(ItemData::from).collect();
                                    let mut id_map = HashMap::new();
                                    for (index, item) in skills_list.iter().enumerate() { id_map.insert(item.uuid.clone(), index); }
                                    let count = skills_list.len();
                                    let mut db = db_state.skills.write().unwrap();
                                    db.list = skills_list;
                                    db.id_map = id_map;
                                    log_to_file(&format!("[Init] Successfully loaded {} skills from {:?}", count, path));
                                    break;
                                },
                                Err(e) => log_to_file(&format!("Error parsing skills_db.json: {}", e)),
                            }
                        },
                        Err(e) => log_to_file(&format!("Error reading skills_db.json: {}", e)),
                    }
                } else {
                    log_to_file("Path does not exist.");
                }
            }

            // 3. Load Monster Image Map
            let mut monster_img_map_path = resources_path.join("resources").join("images_monster_map.json");
            if !monster_img_map_path.exists() {
                monster_img_map_path = resources_path.join("images_monster_map.json");
            }
            log_to_file(&format!("Attempting to load Monster Image Map from {:?}", monster_img_map_path));
            let mut monster_img_lookup = HashMap::new();
            if let Ok(json) = std::fs::read_to_string(&monster_img_map_path) {
                if let Ok(serde_json::Value::Object(map)) = serde_json::from_str::<serde_json::Value>(&json) {
                    for (name, info) in map {
                        if let Some(out) = info.get("out").and_then(|v| v.as_str()) {
                            monster_img_lookup.insert(name, out.replace("\\", "/"));
                        }
                    }
                }
            }

            // 4. Load & Merge Monsters (Export First, then DB)
            // 尝试多种路径方式以兼容dev和release模式
            let mut monsters_export_path = resources_path.join("resources").join("monsters_export.json");
            let mut monsters_db_path = resources_path.join("resources").join("monsters_db.json");
            
            // 如果第一种路径不存在，尝试直接从resources_path查找
            if !monsters_export_path.exists() {
                monsters_export_path = resources_path.join("monsters_export.json");
            }
            if !monsters_db_path.exists() {
                monsters_db_path = resources_path.join("monsters_db.json");
            }
            
            // 调试日志：检查路径
            log_to_file(&format!("Resources base path: {:?}", resources_path));
            log_to_file(&format!("Monsters export path: {:?}", monsters_export_path));
            log_to_file(&format!("Monsters db path: {:?}", monsters_db_path));
            log_to_file(&format!("Monsters export exists: {}", monsters_export_path.exists()));
            log_to_file(&format!("Monsters db exists: {}", monsters_db_path.exists()));
            
            let mut final_monsters = serde_json::Map::new();
            let mut export_by_day: HashMap<String, Vec<serde_json::Value>> = HashMap::new();
            
            // Move locks outside to be used by both Export and Fallback
            let items_db = db_state.items.read().unwrap();
            let skills_db = db_state.skills.read().unwrap();

            if monsters_export_path.exists() {
                if let Ok(json) = std::fs::read_to_string(&monsters_export_path) {
                    if let Ok(serde_json::Value::Array(exports)) = serde_json::from_str::<serde_json::Value>(&json) {
                        for m_val in exports {
                            if let Some(m_obj) = m_val.as_object() {
                                let level = m_obj.get("level").and_then(|v| v.as_u64()).unwrap_or(0);
                                let day_label = if level >= 10 { "Day 10+".to_string() } else { format!("Day {}", level) };
                                let name_zh = m_obj.get("name_cn").and_then(|v| v.as_str()).unwrap_or("未知");
                                let name_en = m_obj.get("name_en").and_then(|v| v.as_str()).unwrap_or("Unknown");
                                
                                let mut m_entry = serde_json::Map::new();
                                m_entry.insert("name".to_string(), serde_json::Value::String(name_en.to_string()));
                                m_entry.insert("name_zh".to_string(), serde_json::Value::String(name_zh.to_string()));
                                m_entry.insert("available".to_string(), serde_json::Value::String(day_label.clone()));
                                m_entry.insert("health".to_string(), m_obj.get("max_health").cloned().unwrap_or(0.into()));
                                
                                // 使用角色图路径（中文名.webp）
                                let img = format!("images_monster_char/{}.webp", name_zh);
                                m_entry.insert("image".to_string(), serde_json::Value::String(img));

                                // Loadout Items
                                let mut items_list = Vec::new();
                                if let Some(loadout) = m_obj.get("loadout_items").and_then(|v| v.as_array()) {
                                    for it_val in loadout {
                                        if let Some(it_obj) = it_val.as_object() {
                                            let id = it_obj.get("id").and_then(|v| v.as_str()).unwrap_or("");
                                            let tier_raw = it_obj.get("tier").and_then(|v| v.as_str()).unwrap_or("Bronze");
                                            let tier = tier_raw.split(" / ").next().unwrap_or(tier_raw);
                                            let it_name_cn = it_obj.get("name_cn").and_then(|v| v.as_str()).unwrap_or("未知");
                                            let it_name_en = it_obj.get("name_en").and_then(|v| v.as_str()).unwrap_or("Unknown");
                                            let it_size = it_obj.get("size").and_then(|v| v.as_str());
                                            
                                            let item_data = lookup_item(id, &items_db, &skills_db);
                                            items_list.push(construct_monster_sub_item(item_data, it_name_cn, it_name_en, tier, it_size));
                                        }
                                    }
                                }
                                m_entry.insert("items".to_string(), serde_json::Value::Array(items_list));

                                // Loadout Skills
                                let mut skills_list = Vec::new();
                                if let Some(loadout) = m_obj.get("loadout_skills").and_then(|v| v.as_array()) {
                                    for sk_val in loadout {
                                        if let Some(sk_obj) = sk_val.as_object() {
                                            let id = sk_obj.get("id").and_then(|v| v.as_str()).unwrap_or("");
                                            let tier_raw = sk_obj.get("tier").and_then(|v| v.as_str()).unwrap_or("Bronze");
                                            let tier = tier_raw.split(" / ").next().unwrap_or(tier_raw);
                                            let sk_name_cn = sk_obj.get("name_cn").and_then(|v| v.as_str()).unwrap_or("未知");
                                            let sk_name_en = sk_obj.get("name_en").and_then(|v| v.as_str()).unwrap_or("Unknown");
                                            let sk_size = sk_obj.get("size").and_then(|v| v.as_str());
                                            
                                            let skill_data = lookup_item(id, &items_db, &skills_db);
                                            skills_list.push(construct_monster_sub_item(skill_data, sk_name_cn, sk_name_en, tier, sk_size));
                                        }
                                    }
                                }
                                m_entry.insert("skills".to_string(), serde_json::Value::Array(skills_list));

                                export_by_day.entry(day_label).or_default().push(serde_json::Value::Object(m_entry));
                            }
                        }
                    }
                }
            }

            let mut db_by_day: HashMap<String, Vec<(String, serde_json::Value)>> = HashMap::new();
            if monsters_db_path.exists() {
                if let Ok(json) = std::fs::read_to_string(&monsters_db_path) {
                    if let Ok(serde_json::Value::Object(monsters)) = serde_json::from_str::<serde_json::Value>(&json) {
                        for (name, data) in monsters {
                            let day = data.get("available").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            if !day.is_empty() { db_by_day.entry(day).or_default().push((name, data)); }
                        }
                    }
                }
            }

            // Consolidate: Prioritize monsters_db, then supplement with monsters_export
            for i in 0..21 {
                let day_label = if i >= 10 { "Day 10+".to_string() } else { format!("Day {}", i) };
                
                // First check if Day exists in monsters_db
                if let Some(db_monsters) = db_by_day.get(&day_label) {
                    for (name, m) in db_monsters {
                        let mut enriched_m = m.clone();
                        if let Some(m_obj) = enriched_m.as_object_mut() {
                            // 强制设置图片路径（使用角色图），增加陷阱类前缀回退逻辑
                            let mut img_name = name.clone();
                            let img_path = resources_path.join("resources").join(format!("images_monster_char/{}.webp", img_name));
                            if !img_path.exists() {
                                // 1. 尝试去除 _Day 序列后缀 (如 快乐杰克南瓜_Day8 -> 快乐杰克南瓜)
                                if let Some(idx) = img_name.find("_Day") {
                                    let base = &img_name[0..idx];
                                    if resources_path.join("resources").join(format!("images_monster_char/{}.webp", base)).exists() {
                                        img_name = base.to_string();
                                    }
                                }
                                
                                // 2. 尝试剥离陷阱类前缀 (如 毒素 吹箭枪陷阱 -> 吹箭枪陷阱)
                                if !resources_path.join("resources").join(format!("images_monster_char/{}.webp", img_name)).exists() {
                                    if let Some(space_pos) = img_name.rfind(' ') {
                                        let base_name = &img_name[space_pos + 1..];
                                        let base_path = resources_path.join("resources").join(format!("images_monster_char/{}.webp", base_name));
                                        if base_path.exists() {
                                            img_name = base_name.to_string();
                                        }
                                    }
                                }
                            }
                            let img_rel = format!("images_monster_char/{}.webp", img_name);
                            m_obj.insert("image".to_string(), serde_json::Value::String(img_rel));
                            
                            // Enrich items
                            if let Some(items) = m_obj.get_mut("items").and_then(|v| v.as_array_mut()) {
                                for item_val in items {
                                    if let Some(item_obj) = item_val.as_object_mut() {
                                        let id = item_obj.get("id").and_then(|v| v.as_str()).unwrap_or("");
                                        let name_cn = item_obj.get("name").and_then(|v| v.as_str()).unwrap_or("");
                                        
                                        // 如果 id 为空，尝试通过中文名查找
                                        let found = if id.is_empty() && !name_cn.is_empty() {
                                            lookup_item_by_name(name_cn, &items_db, &skills_db)
                                        } else {
                                            lookup_item(id, &items_db, &skills_db)
                                        };
                                        
                                        if let Some(found_item) = found {
                                            // 更新 id
                                            if id.is_empty() {
                                                item_obj.insert("id".to_string(), serde_json::Value::String(found_item.uuid.clone()));
                                            }
                                            // 注入升级数据
                                            item_obj.insert("cooldown_tiers".to_string(), serde_json::Value::String(found_item.cooldown_tiers.clone()));
                                            item_obj.insert("available_tiers".to_string(), serde_json::Value::String(found_item.available_tiers.clone()));
                                            item_obj.insert("damage_tiers".to_string(), serde_json::Value::String(found_item.damage_tiers.clone()));
                                            item_obj.insert("heal_tiers".to_string(), serde_json::Value::String(found_item.heal_tiers.clone()));
                                            item_obj.insert("shield_tiers".to_string(), serde_json::Value::String(found_item.shield_tiers.clone()));

                                            // 强制使用 id.webp 格式作为图片路径
                                            let webp_img = format!("images/{}.webp", found_item.uuid);
                                            item_obj.insert("image".to_string(), serde_json::Value::String(webp_img));
                                            
                                            // 更新 size
                                            if let Some(s) = found_item.size {
                                                let norm = s.split(" / ").next().unwrap_or(&s).to_string();
                                                item_obj.insert("size".to_string(), serde_json::Value::String(norm));
                                            }
                                        }
                                    }
                                }
                            }
                            // Enrich skills
                            if let Some(skills) = m_obj.get_mut("skills").and_then(|v| v.as_array_mut()) {
                                for skill_val in skills {
                                    if let Some(skill_obj) = skill_val.as_object_mut() {
                                        let id = skill_obj.get("id").and_then(|v| v.as_str()).unwrap_or("");
                                        let name_cn = skill_obj.get("name").and_then(|v| v.as_str()).unwrap_or("");
                                        
                                        // 如果 id 为空，尝试通过中文名查找
                                        let found = if id.is_empty() && !name_cn.is_empty() {
                                            lookup_item_by_name(name_cn, &items_db, &skills_db)
                                        } else {
                                            lookup_item(id, &items_db, &skills_db)
                                        };
                                        
                                        if let Some(found_skill) = found {
                                            // 更新 id
                                            if id.is_empty() {
                                                skill_obj.insert("id".to_string(), serde_json::Value::String(found_skill.uuid.clone()));
                                            }
                                            // 注入升级数据
                                            skill_obj.insert("cooldown_tiers".to_string(), serde_json::Value::String(found_skill.cooldown_tiers.clone()));
                                            skill_obj.insert("available_tiers".to_string(), serde_json::Value::String(found_skill.available_tiers.clone()));
                                            skill_obj.insert("damage_tiers".to_string(), serde_json::Value::String(found_skill.damage_tiers.clone()));
                                            skill_obj.insert("heal_tiers".to_string(), serde_json::Value::String(found_skill.heal_tiers.clone()));
                                            skill_obj.insert("shield_tiers".to_string(), serde_json::Value::String(found_skill.shield_tiers.clone()));

                                            // 强制使用 id.webp 格式作为图片路径
                                            let webp_img = format!("images/{}.webp", found_skill.uuid);
                                            skill_obj.insert("image".to_string(), serde_json::Value::String(webp_img));
                                            
                                            // 更新 size
                                            if let Some(s) = found_skill.size {
                                                let norm = s.split(" / ").next().unwrap_or(&s).to_string();
                                                skill_obj.insert("size".to_string(), serde_json::Value::String(norm));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        final_monsters.insert(name.clone(), enriched_m);
                    }
                } 
                // Then supplement with Export if Day doesn't exist in DB (or if you want to merge, but user said "switch back")
                else if let Some(exports) = export_by_day.get(&day_label) {
                    for m in exports {
                        if let Some(name_zh) = m.get("name_zh").and_then(|v| v.as_str()) {
                            final_monsters.insert(name_zh.to_string(), m.clone());
                        }
                    }
                }
            }
            let monster_count = final_monsters.len();
            *db_state.monsters.write().unwrap() = final_monsters.clone();
            log_to_file(&format!("Monsters DB populated with {} entries", monster_count));
            
            // 调试：输出前几个怪物名称，并通知前端数据库已准备好
            if monster_count > 0 {
                let sample_names: Vec<String> = final_monsters.keys().take(5).cloned().collect();
                log_to_file(&format!("Sample loaded monsters: {:?}", sample_names));
                // Emit an event so the frontend knows the monsters DB is ready
                let _ = handle.emit("monsters-db-ready", serde_json::json!({
                    "total": monster_count,
                    "sample": sample_names,
                }));
            } else {
                log_to_file("Warning: No monsters were loaded!");
            }

            println!("[Init] Successfully consolidated {} monsters (Export prioritized by day)", monster_count);

            // Log Monitor Thread
            let thread_items_db = db_state.items.clone();
            let thread_skills_db = db_state.skills.clone();
            let log_handle = handle.clone();
            
            thread::spawn(move || {
                let handle = log_handle;
                let log_path = get_log_path();
                let prev_path = get_prev_log_path();
                
                let re_purchase = Regex::new(r"Card Purchased: InstanceId:\s*(?P<iid>[^ ]+)\s*-\s*TemplateId\s*(?P<tid>[^ ]+)(?:.*Target:(?P<tgt>[^ ]+))?(?:.*Section(?P<sec>[^ ]+))?").unwrap();
                let re_id = Regex::new(r"ID: \[(?P<id>[^\]]+)\]").unwrap();
                let re_owner = Regex::new(r"- Owner: \[(?P<val>[^\]]+)\]").unwrap();
                let re_section = Regex::new(r"- Section: \[(?P<val>[^\]]+)\]").unwrap();

                let re_item_id = Regex::new(r"itm_[A-Za-z0-9_-]+").unwrap();
                let re_sold = Regex::new(r"Sold Card\s+(?P<iid>itm_[^ ]+)").unwrap();
                let re_removed = Regex::new(r"Successfully removed item\s+(?P<iid>itm_[^ ]+)").unwrap();
                let re_moved_to = Regex::new(r"Successfully moved card\s+(?P<iid>itm_[^ ]+)\s+to\s+(?P<tgt>[^ ]+)").unwrap();
                
                // Initialize state from cache
                let _cache_path = get_cache_path();
                let _has_cache = _cache_path.exists();
                let state_init = load_state();
                
                let mut inst_to_temp = state_init.inst_to_temp;
                let mut current_hand = state_init.current_hand;
                let mut current_stash = state_init.current_stash;
                let mut current_day = state_init.day;
                
                let mut last_file_size = if log_path.exists() {
                    std::fs::metadata(&log_path).map(|m| m.len()).unwrap_or(0)
                } else {
                    0
                };
                
                let mut last_iid = String::new();
                let mut cur_owner = String::new();
                let mut in_pvp = false;
                let mut is_sync = false;

                // --- Initial Sync: Replay Logs to catch up with current state ---
                println!("[LogMonitor] Initializing state from logs...");
                
                // Clear state for fresh scan (we'll recover inst_to_temp from logs too)
                current_hand.clear();
                current_stash.clear();
                // inst_to_temp.clear(); // We keep cache as fallback, but logs will overwrite

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
                    let hand_items = init_hand.iter()
                        .filter_map(|iid| init_map.get(iid))
                        .filter_map(|tid| lookup_item(tid, &items_db, &skills_db))
                        .collect();
                    let stash_items = init_stash.iter()
                        .filter_map(|iid| init_map.get(iid))
                        .filter_map(|tid| lookup_item(tid, &items_db, &skills_db))
                        .collect();
                    let all_tags = items_db.unique_tags.clone();
                    let _ = init_handle.emit("sync-items", SyncPayload { hand_items, stash_items, all_tags });
                });

                println!("[LogMonitor] Initialization complete. Starting main monitoring loop...");
                // Debug: Log the path being monitored at startup
                log_to_file(&format!("[LogMonitor] Monitoring log file: {:?}", log_path));
                println!("[LogMonitor] Monitoring log file: {:?}", log_path);
                log_to_file(&format!("[LogMonitor] Starting monitor loop, initial size: {}", last_file_size));
                
                loop {
                    if !log_path.exists() { 
                        log_to_file(&format!("[LogMonitor] Log file not found: {:?}", log_path));
                        thread::sleep(time::Duration::from_secs(2)); 
                        continue; 
                    }
                    let current_file_size = match std::fs::metadata(&log_path) {
                        Ok(meta) => meta.len(),
                        Err(e) => {
                            log_to_file(&format!("Error reading log metadata: {}. Retrying...", e));
                            thread::sleep(time::Duration::from_secs(1));
                            continue;
                        }
                    };
                    
                    // Debug: Log size changes
                    if current_file_size != last_file_size {
                        log_to_file(&format!("[LogMonitor] File size changed: {} -> {}", last_file_size, current_file_size));
                    }
                    
                    if current_file_size < last_file_size {
                        println!("[LogMonitor] Log truncated, resetting state...");
                        inst_to_temp.clear();
                        current_hand.clear();
                        current_stash.clear();
                        current_day = 1;
                        is_sync = false;
                        last_file_size = 0;
                        save_state(&PersistentState { 
                            day: current_day, 
                            inst_to_temp: inst_to_temp.clone(), 
                            current_hand: current_hand.clone(), 
                            current_stash: current_stash.clone(),
                            ..load_state()
                        });
                    }
                    
                    if current_file_size > last_file_size {
                        // Prevent spamming triggers if we are catching up on a large log chunk (>5000 bytes)
                        let is_bulk_read = (current_file_size - last_file_size) > 5000;
                        if is_bulk_read {
                            log_to_file(&format!("[LogMonitor] Bulk read detected: {} bytes, will skip YOLO triggers for this batch", current_file_size - last_file_size));
                        }
                        
                        let mut f = match File::open(&log_path) {
                            Ok(file) => file,
                            Err(e) => {
                                log_to_file(&format!("Failed to open log file for reading: {}", e));
                                thread::sleep(time::Duration::from_secs(1));
                                continue;
                            }
                        };
                        let _ = f.seek(SeekFrom::Start(last_file_size));
                        let reader = BufReader::new(f);
                        
                        let mut changed = false;
                        let mut day_changed = false;
                        for line in reader.lines() {
                            let l = if let Ok(l) = line { l } else { continue };
                            let trimmed = l.trim();

                            // Day Detection Logic
                            if trimmed.contains("NetMessageRunInitialized") {
                                current_day = 1; in_pvp = false; day_changed = true;
                                inst_to_temp.clear();
                                current_hand.clear();
                                current_stash.clear();
                                changed = true;
                            }
                            
                            // Tracks PVP state
                            if trimmed.contains("to [PVPCombatState]") { 
                                in_pvp = true; 
                            }
                            
                            // Day increment: The most reliable trigger is the transition back to Map (ChoiceState) after a PVP fight.
                            if in_pvp && trimmed.contains("State changed") && (trimmed.contains("to [ChoiceState]") || trimmed.contains("to [LevelUpState]")) {
                                current_day = current_day.saturating_add(1);
                                in_pvp = false;
                                day_changed = true;
                                println!("[DayMonitor] Day increased to {} after PVP completion", current_day);
                            }

                            /* 
                            // YOLO Trigger on ANY State changed (controlled by enable-yolo-auto setting)
                            // Debug: Log every state change line and check conditions
                            if trimmed.contains("State changed") {
                                println!("[Debug] Found 'State changed' line: {}", trimmed);
                                println!("[Debug] is_bulk_read: {}", is_bulk_read);
                                println!("[Debug] contains 'State changed from [': {}", trimmed.contains("State changed from ["));
                                println!("[Debug] contains '] to [': {}", trimmed.contains("] to ["));
                            }
                            
                            if !is_bulk_read && trimmed.contains("State changed from [") && trimmed.contains("] to [") {
                                println!("[State Change Detected] Emitting YOLO trigger for: {}", trimmed);
                                log_to_file(&format!("[State Change Detected] {}", trimmed));
                                log_to_file("[Backend] Emitting trigger_yolo_scan event to frontend");
                                // Emit event to frontend, which will check enable-yolo-auto setting
                                match handle.emit("trigger_yolo_scan", ()) {
                                    Ok(_) => {
                                        println!("[Backend] trigger_yolo_scan event emitted successfully");
                                        log_to_file("[Backend] trigger_yolo_scan event emitted successfully");
                                    },
                                    Err(e) => {
                                        println!("[Backend] Failed to emit trigger_yolo_scan: {}", e);
                                        log_to_file(&format!("[Backend] Failed to emit trigger_yolo_scan: {}", e));
                                    },
                                }
                            }
                            */

                            if let Some(cap) = re_purchase.captures(trimmed) {
                                let iid = cap["iid"].to_string();
                                inst_to_temp.insert(iid.clone(), cap["tid"].to_string());
                                
                                let mut section = cap.name("sec").map(|s| s.as_str().to_string());
                                let target = cap.name("tgt").map(|t| t.as_str());

                                // Fallback: Derive section from Target if Section is missing or ambiguous
                                if section.as_deref().unwrap_or("") == "" {
                                    if let Some(tgt) = target {
                                        if tgt.contains("PlayerStorageSocket") { section = Some("Stash".to_string()); }
                                        else if tgt.contains("PlayerSocket") { section = Some("Player".to_string()); }
                                    }
                                }

                                if let Some(s) = section {
                                    if s == "Player" || s == "Hand" { 
                                        current_hand.insert(iid); changed = true; 
                                    }
                                    else if s == "Stash" || s == "Storage" || s == "PlayerStorage" { 
                                        current_stash.insert(iid); changed = true; 
                                    }
                                }
                            }

                            if let Some(cap) = re_moved_to.captures(trimmed) {
                                let iid = cap["iid"].to_string();
                                let tgt = &cap["tgt"];
                                if tgt.contains("StorageSocket") {
                                    current_stash.insert(iid.clone());
                                    current_hand.remove(&iid);
                                    changed = true;
                                } else if tgt.contains("Socket") { // General Socket_X
                                    current_hand.insert(iid.clone());
                                    current_stash.remove(&iid);
                                    changed = true;
                                }
                            }

                            if let Some(cap) = re_sold.captures(trimmed) {
                                let iid = cap["iid"].to_string();
                                if current_hand.remove(&iid) || current_stash.remove(&iid) {
                                    changed = true;
                                }
                            }

                            if let Some(cap) = re_removed.captures(trimmed) {
                                let iid = cap["iid"].to_string();
                                if current_hand.remove(&iid) || current_stash.remove(&iid) {
                                    changed = true;
                                }
                            }

                            if trimmed.contains("Cards Disposed:") {
                                for mat in re_item_id.find_iter(trimmed) {
                                    let iid = mat.as_str().to_string();
                                    if current_hand.remove(&iid) || current_stash.remove(&iid) {
                                        changed = true;
                                    }
                                }
                            }

                            if trimmed.contains("Cards Spawned:") || trimmed.contains("Cards Dealt:") || trimmed.contains("NetMessageGameStateSync") {
                                is_sync = true;
                            } else if trimmed.contains("Successfully moved card to:") {
                                is_sync = true;
                            }

                            if is_sync {
                                if let Some(cap) = re_id.captures(trimmed) { last_iid = cap["id"].to_string(); }
                                else if let Some(cap) = re_owner.captures(trimmed) { cur_owner = cap["val"].to_string(); }
                                else if let Some(cap) = re_section.captures(trimmed) {
                                    if !last_iid.is_empty() && &cur_owner == "Player" {
                                        if last_iid.starts_with("itm_") {
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
                                            changed = true;
                                        }
                                    }
                                    // Reset for next block
                                    last_iid.clear();
                                    cur_owner.clear();
                                }
                                else if trimmed.contains("Finished processing") {
                                    is_sync = false;
                                    changed = true;
                                }
                            }
                        }

                        if changed || day_changed {
                            if day_changed {
                                let _ = handle.emit("day-update", current_day);
                            }
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
                        last_file_size = current_file_size;
                    }
                    thread::sleep(time::Duration::from_millis(500));
                }
            });

            // 启动鼠标监听线程 (识别怪物与卡牌) - 跨平台实现
            let handle_mouse = handle.clone();
            std::thread::spawn(move || {
                let device_state = DeviceState::new();
                let mut last_trigger = time::Instant::now();
                let mut last_card_trigger = time::Instant::now();
                let mut last_toggle_trigger = time::Instant::now();
                let mut last_yolo_trigger = time::Instant::now();
                loop {
                    let mouse_state = device_state.get_mouse();

                    // 读取配置的按键
                    let (monster_hotkey, card_hotkey, toggle_hotkey, yolo_hotkey) = {
                        let state = load_state();
                        (
                            state.detection_hotkey.unwrap_or(default_monster_hotkey()),
                            state.card_detection_hotkey.unwrap_or(default_card_hotkey()),
                            state.toggle_collapse_hotkey.unwrap_or(192),
                            state.yolo_hotkey.unwrap_or(81)
                        )
                    };

                    // 1. 检测怪物识别按键
                    if is_key_pressed(monster_hotkey, &device_state, &mouse_state) {
                            if last_trigger.elapsed() > time::Duration::from_millis(500) {
                                last_trigger = time::Instant::now();
                                log_to_file("Monster Hotkey pressed, starting scan...");
                                
                                // 尝试识别怪物
                                match scan_and_identify_monster_at_mouse() {
                                    Ok(Some(monster_name)) => {
                                        log_to_file(&format!("Success! Valid monster found: {}", monster_name));
                                        
                                        // 关键修复：处理陷阱类并列名称
                                        let lookup_name = if monster_name.contains('|') {
                                            monster_name.split('|').next().unwrap_or(&monster_name).to_string()
                                        } else {
                                            monster_name.clone()
                                        };

                                        if let Some(db_state) = handle_mouse.try_state::<DbState>() {
                                            if let Ok(monsters) = db_state.monsters.read() {
                                                // 首先尝试通过 Key 获取 Entry，如果不行，尝试遍历匹配 name_zh
                                                let entry_opt = monsters.get(&lookup_name)
                                                    .or_else(|| {
                                                        monsters.values().find(|v| {
                                                            v.get("name_zh").and_then(|nz| nz.as_str()) == Some(&lookup_name)
                                                        })
                                                    });

                                                if let Some(entry) = entry_opt {
                                                    let target_name_zh = entry.get("name_zh").and_then(|v| v.as_str()).unwrap_or(&monster_name);
                                                    let mut candidate_days: Vec<u32> = Vec::new();
                                                    
                                                    // 寻找所有具有相同中文名的怪物条目（解决同名不同天数问题）
                                                    for (_, v) in monsters.iter() {
                                                        if let Some(n_zh) = v.get("name_zh").and_then(|val| val.as_str()) {
                                                            if n_zh == target_name_zh {
                                                                if let Some(d_str) = v.get("available").and_then(|val| val.as_str()) {
                                                                    if d_str.starts_with("Day ") {
                                                                        let num_part = d_str[4..].trim_end_matches('+');
                                                                        if let Ok(d_num) = num_part.parse::<u32>() {
                                                                            candidate_days.push(d_num);
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                    
                                                    if !candidate_days.is_empty() {
                                                        candidate_days.sort();
                                                        candidate_days.dedup();

                                                        let current_day = load_state().day;
                                                        let target_day = if candidate_days.contains(&current_day) {
                                                            current_day
                                                        } else {
                                                            *candidate_days.iter().min_by_key(|&&d| (d as i32 - current_day as i32).abs()).unwrap()
                                                        };

                                                        match handle_mouse.emit("auto-jump-to-monster", serde_json::json!({
                                                            "day": target_day,
                                                            "monster_name": monster_name // 使用包含 | 的原始名称
                                                        })) {
                                                            Ok(_) => {},
                                                            Err(e) => println!("Failed to emit auto-jump-to-monster: {}", e),
                                                        }
                                                        
                                                        let mut state = load_state();
                                                        state.day = target_day;
                                                        save_state(&state);
                                                        
                                                        println!("自动跳转到 Day {} (识别: {}, 候选天数: {:?})", target_day, lookup_name, candidate_days);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    Ok(None) => {
                                        // Scan successful but no monster found
                                        log_to_file("Scan complete, no monster matched.");
                                    }
                                    Err(e) => {
                                        let err_msg = format!("Monster Scan Failed: {}", e);
                                        println!("[Error] {}", err_msg);
                                        log_to_file(&format!("Error: {}", err_msg));
                                        // Emit error to frontend for toast
                                        let _ = handle_mouse.emit("scan-error", e);
                                    }
                                }
                            }
                        }

                    // 2. 检测卡牌识别按键
                    if is_key_pressed(card_hotkey, &device_state, &mouse_state) {
                        if last_card_trigger.elapsed() > time::Duration::from_millis(500) {
                            last_card_trigger = time::Instant::now();
                            log_to_file("Card Hotkey pressed, triggering recognition...");
                            let _ = handle_mouse.emit("hotkey-detect-card", ());
                        }
                    }

                    // 3. 检测折叠/展开按键
                    if is_key_pressed(toggle_hotkey, &device_state, &mouse_state) {
                        if last_toggle_trigger.elapsed() > time::Duration::from_millis(500) {
                            last_toggle_trigger = time::Instant::now();
                            log_to_file("Toggle Hotkey pressed");
                            let _ = handle_mouse.emit("toggle-collapse", ());
                        }
                    }

                    // 4. 检测YOLO手动触发按键（排除左右键）
                    if yolo_hotkey != 1 && yolo_hotkey != 2 && is_key_pressed(yolo_hotkey, &device_state, &mouse_state) {
                        if last_yolo_trigger.elapsed() > time::Duration::from_millis(500) {
                            last_yolo_trigger = time::Instant::now();
                            log_to_file("YOLO Hotkey pressed");
                            let _ = handle_mouse.emit("yolo_hotkey_pressed", ());
                        }
                    }

                    thread::sleep(time::Duration::from_millis(100));
                }
            });

            log_to_file("Setup complete. Initializing main loop...");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_all_monsters,
            debug_monsters_db,
            debug_resource_paths,
            clear_yolo_cache,
            recognize_monsters_from_screenshot,
            get_template_loading_progress,
            get_current_day,
            update_day,
            get_detection_hotkey,
            set_detection_hotkey,
            get_card_detection_hotkey,
            set_card_detection_hotkey,
            get_toggle_collapse_hotkey,
            set_toggle_collapse_hotkey,
            set_yolo_hotkey,
            get_detail_display_hotkey,
            set_detail_display_hotkey,
            start_template_loading,
            get_item_info,
            search_items,
            crate::monster_recognition::check_opencv_load, 
            crate::monster_recognition::recognize_card_at_mouse,
            crate::monster_recognition::load_event_templates,
            crate::monster_recognition::recognize_event_at_mouse,
            trigger_yolo_scan,
            abort_yolo_scan,
            invoke_yolo_scan,
            handle_overlay_right_click,
            update_overlay_bounds,
            emit_to_main,
            get_yolo_stats,
            get_show_yolo_monitor,
            // clear_monster_cache,
            set_overlay_ignore_cursor,
            set_show_yolo_monitor,
            update_overlay_detail_position,
            restore_game_focus
        ])
        .run(tauri::generate_context!())
        .map_err(|e| {
            log_to_file(&format!("FATAL: Error while running tauri application: {}", e));
            e
        })
        .expect("error while running tauri application");
}
