#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DWMWA_BORDER_COLOR, DWMWA_CAPTION_COLOR, DWMWA_TEXT_COLOR,
    DWMWA_USE_IMMERSIVE_DARK_MODE,
};
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongW, SetWindowLongW, SetWindowPos, GWL_EXSTYLE, GWL_STYLE, SWP_FRAMECHANGED,
    SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, WS_BORDER, WS_CAPTION,
    WS_CLIPCHILDREN, WS_CLIPSIBLINGS, WS_DLGFRAME, WS_EX_APPWINDOW, WS_EX_CLIENTEDGE,
    WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_STATICEDGE, WS_EX_TOOLWINDOW, WS_EX_WINDOWEDGE,
    WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_POPUP, WS_SYSMENU, WS_THICKFRAME, WS_VISIBLE,
};

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{COLORREF, HWND};

#[cfg(target_os = "macos")]
#[allow(deprecated)]
use cocoa::appkit::{NSWindow, NSWindowCollectionBehavior};
#[cfg(target_os = "macos")]
#[allow(deprecated)]
use cocoa::base::id;
#[cfg(target_os = "macos")]
use objc::{msg_send, sel, sel_impl};
#[cfg(target_os = "macos")]
use tauri_nspanel::WebviewWindowExt as NSPanelExt;

#[cfg(target_os = "windows")]
pub fn apply_dark_theme(window: &tauri::WebviewWindow) {
    if let Ok(hwnd) = window.hwnd() {
        unsafe {
            let handle = HWND(hwnd.0 as _);

            let use_dark_mode = 1_i32;
            let _ = DwmSetWindowAttribute(
                handle,
                DWMWA_USE_IMMERSIVE_DARK_MODE,
                &use_dark_mode as *const _ as *const _,
                std::mem::size_of::<i32>() as u32,
            );

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
pub fn apply_pure_overlay_style(window: &tauri::WebviewWindow) {
    apply_dark_theme(window);

    if let Ok(hwnd) = window.hwnd() {
        unsafe {
            let handle = HWND(hwnd.0 as _);

            let current_style = GetWindowLongW(handle, GWL_STYLE) as u32;
            let mut new_style = current_style
                & !(WS_CAPTION.0
                    | WS_THICKFRAME.0
                    | WS_MINIMIZEBOX.0
                    | WS_MAXIMIZEBOX.0
                    | WS_SYSMENU.0
                    | WS_BORDER.0
                    | WS_DLGFRAME.0);
            new_style |= WS_POPUP.0 | WS_VISIBLE.0 | WS_CLIPSIBLINGS.0 | WS_CLIPCHILDREN.0;
            SetWindowLongW(handle, GWL_STYLE, new_style as i32);

            let current_ex_style = GetWindowLongW(handle, GWL_EXSTYLE) as u32;
            let mut new_ex_style =
                current_ex_style & !(WS_EX_APPWINDOW.0 | WS_EX_WINDOWEDGE.0 | WS_EX_CLIENTEDGE.0 | WS_EX_STATICEDGE.0);
            new_ex_style |= WS_EX_TOOLWINDOW.0 | WS_EX_LAYERED.0;
            SetWindowLongW(handle, GWL_EXSTYLE, new_ex_style as i32);

            let _ = SetWindowPos(
                handle,
                None,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            );
        }
    }
}

#[cfg(target_os = "windows")]
pub fn apply_main_window_style(window: &tauri::WebviewWindow) {
    apply_dark_theme(window);

    if let Ok(hwnd) = window.hwnd() {
        unsafe {
            let handle = HWND(hwnd.0 as _);

            let current_style = GetWindowLongW(handle, GWL_STYLE) as u32;
            let mut new_style =
                current_style & !(WS_CAPTION.0 | WS_SYSMENU.0 | WS_MINIMIZEBOX.0 | WS_MAXIMIZEBOX.0);
            new_style |= WS_POPUP.0 | WS_THICKFRAME.0;
            SetWindowLongW(handle, GWL_STYLE, new_style as i32);

            let current_ex_style = GetWindowLongW(handle, GWL_EXSTYLE) as u32;
            let mut new_ex_style = current_ex_style & !(WS_EX_TOOLWINDOW.0);
            new_ex_style |= WS_EX_APPWINDOW.0 | WS_EX_LAYERED.0;
            SetWindowLongW(handle, GWL_EXSTYLE, new_ex_style as i32);

            let _ = SetWindowPos(
                handle,
                None,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            );
        }
    }
}

#[cfg(target_os = "windows")]
pub fn apply_no_activate_style(window: &tauri::WebviewWindow) {
    if let Ok(hwnd) = window.hwnd() {
        unsafe {
            let hwnd_val = HWND(hwnd.0 as _);
            let style = GetWindowLongW(hwnd_val, GWL_EXSTYLE);
            SetWindowLongW(
                hwnd_val,
                GWL_EXSTYLE,
                style | WS_EX_NOACTIVATE.0 as i32 | WS_EX_TOOLWINDOW.0 as i32,
            );
        }
    }
}

#[cfg(not(target_os = "windows"))]
#[allow(dead_code)]
pub fn apply_dark_theme(_window: &tauri::WebviewWindow) {}

#[cfg(not(target_os = "windows"))]
#[allow(dead_code)]
pub fn apply_pure_overlay_style(_window: &tauri::WebviewWindow) {}

#[cfg(not(target_os = "windows"))]
#[allow(dead_code)]
pub fn apply_main_window_style(_window: &tauri::WebviewWindow) {}

#[cfg(not(target_os = "windows"))]
pub fn apply_no_activate_style(_window: &tauri::WebviewWindow) {}

pub fn enforce_overlay_traits(window: &tauri::WebviewWindow, label: &str) {
    refresh_overlay_pin(window, label);

    #[cfg(target_os = "macos")]
    {
        let label_owned = label.to_string();
        run_macos_on_main_thread(window, "enforce_overlay_traits", move |window_main| {
            // Use NSPanel path for all overlay windows (including main) to keep fullscreen/top-layer behavior consistent.
            setup_macos_fullscreen_overlay(&window_main);
            log::debug!("[macOS] Enforced fullscreen overlay traits for '{}' (NSPanel)", label_owned);
        });
    }
}

pub fn refresh_overlay_pin(window: &tauri::WebviewWindow, label: &str) {
    let _ = window.set_always_on_top(true);

    #[cfg(target_os = "macos")]
    {
        let _ = window.set_visible_on_all_workspaces(true);
        let label_owned = label.to_string();
        run_macos_on_main_thread(window, "refresh_overlay_pin", move |window_main| {
            fallback_setup_macos_overlay(&window_main);
            log::trace!("[macOS] Refreshed overlay pin for '{}'", label_owned);
        });
    }
}

#[cfg(target_os = "macos")]
fn run_macos_on_main_thread<F>(window: &tauri::WebviewWindow, task_name: &str, task: F)
where
    F: FnOnce(tauri::WebviewWindow) + Send + 'static,
{
    let window_clone = window.clone();
    if let Err(e) = window.run_on_main_thread(move || task(window_clone)) {
        log::warn!("[macOS] Failed to dispatch '{}' on main thread: {}", task_name, e);
    }
}

#[cfg(target_os = "macos")]
#[allow(unexpected_cfgs)]
fn macos_max_window_level() -> i64 {
    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGWindowLevelForKey(key: i32) -> i32;
    }
    const K_CG_MAXIMUM_WINDOW_LEVEL_KEY: i32 = 14;
    unsafe { CGWindowLevelForKey(K_CG_MAXIMUM_WINDOW_LEVEL_KEY) as i64 }
}

#[cfg(target_os = "macos")]
#[allow(deprecated)]
fn force_order_front_regardless(window: &tauri::WebviewWindow) {
    if let Ok(ns_window) = window.ns_window() {
        unsafe {
            let ns_win: id = ns_window as id;
            #[allow(clippy::let_unit_value)]
            let _: () = msg_send![ns_win, orderFrontRegardless];
        }
    }
}

#[cfg(target_os = "macos")]
#[allow(deprecated)]
pub fn setup_macos_fullscreen_overlay(window: &tauri::WebviewWindow) {
    match window.to_panel() {
        Ok(panel) => {
            // Preserve the existing style mask (including resizable bits) and only add
            // NonActivatingPanel, otherwise macOS resize can be unintentionally disabled.
            const NS_WINDOW_STYLE_MASK_NON_ACTIVATING_PANEL: i64 = 1 << 7;
            if let Ok(ns_window) = window.ns_window() {
                unsafe {
                    let ns_win: id = ns_window as id;
                    let current_mask: i64 = msg_send![ns_win, styleMask];
                    let new_mask = current_mask | NS_WINDOW_STYLE_MASK_NON_ACTIVATING_PANEL;
                    #[allow(clippy::let_unit_value)]
                    let _: () = msg_send![ns_win, setStyleMask: new_mask];
                }
            }

            let max_level = macos_max_window_level();
            panel.set_level(max_level as i32);

            panel.set_collection_behaviour(
                tauri_nspanel::cocoa::appkit::NSWindowCollectionBehavior::NSWindowCollectionBehaviorCanJoinAllSpaces
                    | tauri_nspanel::cocoa::appkit::NSWindowCollectionBehavior::NSWindowCollectionBehaviorFullScreenAuxiliary
                    | tauri_nspanel::cocoa::appkit::NSWindowCollectionBehavior::NSWindowCollectionBehaviorStationary,
            );

            panel.set_floating_panel(true);
            panel.set_hides_on_deactivate(false);
            force_order_front_regardless(window);

            log::info!(
                "[macOS] Overlay converted to NSPanel with NonActivatingPanel style, level={}",
                max_level
            );
        }
        Err(e) => {
            log::warn!("[macOS] Failed to convert to NSPanel: {:?}, falling back to NSWindow", e);
            fallback_setup_macos_overlay(window);
        }
    }
}

#[cfg(target_os = "macos")]
#[allow(deprecated)]
#[allow(unexpected_cfgs)]
pub fn fallback_setup_macos_overlay(window: &tauri::WebviewWindow) {
    use cocoa::base::BOOL;

    if let Ok(ns_window) = window.ns_window() {
        unsafe {
            let ns_win: id = ns_window as id;

            let behavior = NSWindowCollectionBehavior::NSWindowCollectionBehaviorCanJoinAllSpaces
                | NSWindowCollectionBehavior::NSWindowCollectionBehaviorStationary
                | NSWindowCollectionBehavior::NSWindowCollectionBehaviorIgnoresCycle
                | NSWindowCollectionBehavior::NSWindowCollectionBehaviorFullScreenAuxiliary;
            ns_win.setCollectionBehavior_(behavior);

            let max_level = macos_max_window_level();
            ns_win.setLevel_(max_level);

            #[allow(clippy::let_unit_value)]
            let _: () = msg_send![ns_win, setHidesOnDeactivate: false as BOOL];
            #[allow(clippy::let_unit_value)]
            let _: () = msg_send![ns_win, orderFrontRegardless];

            log::trace!("[macOS] Overlay window (fallback) configured with maximum level: {}", max_level);
        }
    }
}

#[cfg(not(target_os = "macos"))]
#[allow(dead_code)]
pub fn setup_macos_fullscreen_overlay(_window: &tauri::WebviewWindow) {}
