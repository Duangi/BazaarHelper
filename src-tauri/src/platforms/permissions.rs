use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct MacPermissionStatus {
    pub accessibility: bool,
    pub screen_recording: bool,
}

#[cfg(target_os = "macos")]
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGPreflightScreenCaptureAccess() -> bool;
    fn CGRequestScreenCaptureAccess() -> bool;
}

#[cfg(target_os = "macos")]
fn check_macos_permissions() -> MacPermissionStatus {
    let accessibility = macos_accessibility_client::accessibility::application_is_trusted();
    let screen_recording = unsafe { CGPreflightScreenCaptureAccess() };
    MacPermissionStatus {
        accessibility,
        screen_recording,
    }
}

#[cfg(not(target_os = "macos"))]
fn check_macos_permissions() -> MacPermissionStatus {
    MacPermissionStatus {
        accessibility: true,
        screen_recording: true,
    }
}

#[cfg(target_os = "macos")]
fn open_privacy_pane(anchor: &str) {
    let url = format!(
        "x-apple.systempreferences:com.apple.preference.security?{}",
        anchor
    );
    let _ = std::process::Command::new("open").arg(url).spawn();
}

#[cfg(not(target_os = "macos"))]
#[allow(dead_code)]
fn open_privacy_pane(_anchor: &str) {}

pub fn ensure_macos_permissions_on_startup() -> MacPermissionStatus {
    ensure_macos_permissions(false)
}

fn ensure_macos_permissions(
    #[cfg_attr(not(target_os = "macos"), allow(unused_variables))] force_prompt: bool,
) -> MacPermissionStatus {
    #[cfg(not(target_os = "macos"))]
    {
        return check_macos_permissions();
    }

    #[cfg(target_os = "macos")]
    let mut status = check_macos_permissions();

    #[cfg(target_os = "macos")]
    {
        let mut persisted = crate::load_state();

        let should_prompt_accessibility =
            !status.accessibility && (force_prompt || !persisted.macos_prompted_accessibility);
        if should_prompt_accessibility {
            let _ = macos_accessibility_client::accessibility::application_is_trusted_with_prompt();
            open_privacy_pane("Privacy_Accessibility");
            persisted.macos_prompted_accessibility = true;
        }

        let should_prompt_screen =
            !status.screen_recording && (force_prompt || !persisted.macos_prompted_screen_recording);
        if should_prompt_screen {
            let _ = unsafe { CGRequestScreenCaptureAccess() };
            open_privacy_pane("Privacy_ScreenCapture");
            persisted.macos_prompted_screen_recording = true;
        }

        if status.accessibility {
            persisted.macos_prompted_accessibility = false;
        }
        if status.screen_recording {
            persisted.macos_prompted_screen_recording = false;
        }

        crate::save_state(&persisted);
        status = check_macos_permissions();
    }

    #[cfg(target_os = "macos")]
    status
}

pub fn can_start_global_hotkey_monitor() -> bool {
    #[cfg(target_os = "macos")]
    {
        check_macos_permissions().accessibility
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

#[tauri::command]
pub fn get_macos_permission_status() -> MacPermissionStatus {
    check_macos_permissions()
}

#[tauri::command]
pub fn request_macos_permissions() -> MacPermissionStatus {
    ensure_macos_permissions(true)
}
