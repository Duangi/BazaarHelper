#[cfg(target_os = "windows")]
use windows::core::BOOL;
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{HWND, LPARAM};
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowTextW, IsWindowVisible, SetForegroundWindow, ShowWindow, SW_RESTORE,
};

#[allow(dead_code)]
static LAST_FOREGROUND_WINDOW: std::sync::RwLock<Option<String>> = std::sync::RwLock::new(None);

pub fn is_game_window_active() -> bool {
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
                    return window_title.to_lowercase().contains("the bazaar")
                        || window_title.to_lowercase().contains("thebazaar");
                }
            }
        }
        false
    }
    #[cfg(not(target_os = "windows"))]
    {
        true
    }
}

pub fn update_last_foreground_window() {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextW};
        unsafe {
            let hwnd = GetForegroundWindow();
            if !hwnd.is_invalid() {
                let mut title: [u16; 512] = [0; 512];
                let len = GetWindowTextW(hwnd, &mut title);
                if len > 0 {
                    let window_title = String::from_utf16_lossy(&title[..len as usize]).to_lowercase();
                    if window_title.contains("the bazaar") || window_title.contains("thebazaar") {
                        if let Ok(mut guard) = LAST_FOREGROUND_WINDOW.write() {
                            *guard = Some(window_title);
                        }
                    }
                }
            }
        }
    }
}

#[tauri::command]
pub fn was_last_foreground_game() -> bool {
    if is_game_window_active() {
        return true;
    }
    if let Ok(guard) = LAST_FOREGROUND_WINDOW.read() {
        if let Some(title) = &*guard {
            return title.contains("the bazaar") || title.contains("thebazaar");
        }
    }
    false
}

#[tauri::command]
pub async fn restore_game_focus() -> Result<(), String> {
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
            log::debug!("[Focus] Restored focus to The Bazaar window.");
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
