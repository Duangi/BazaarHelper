use tauri::Manager;

pub fn setup_app_shell(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    setup_system_tray(app)?;

    #[cfg(target_os = "macos")]
    {
        // Keep process alive reliably even when auxiliary overlays are hidden.
        app.set_activation_policy(tauri::ActivationPolicy::Regular);
        log::debug!("[macOS] Set activation policy to Regular");
    }

    if let Some(window) = app.get_webview_window("main") {
        crate::platforms::window_style::apply_main_window_style(&window);
        let _ = window.remove_menu();
        let _ = window.show();
        let _ = window.set_focus();
        crate::platforms::window_style::enforce_overlay_traits(&window, "main");

        #[cfg(target_os = "macos")]
        {
            // Startup race: macOS fullscreen/topmost flags may apply after first show.
            // Re-apply a few times so user doesn't need to drag window manually.
            let window_clone = window.clone();
            std::thread::spawn(move || {
                for (idx, delay_ms) in [200_u64, 700_u64, 1600_u64].into_iter().enumerate() {
                    std::thread::sleep(std::time::Duration::from_millis(delay_ms));
                    if idx == 0 {
                        crate::platforms::window_style::enforce_overlay_traits(&window_clone, "main");
                    } else {
                        crate::platforms::window_style::refresh_overlay_pin(&window_clone, "main");
                    }
                    let _ = window_clone.show();
                }
            });
        }

        #[cfg(target_os = "windows")]
        {
            use windows::Win32::Foundation::HWND as HWND_TYPE;
            use windows::Win32::UI::WindowsAndMessaging::{
                SetWindowPos, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
            };

            let window_clone = window.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(50));
                if let Ok(hwnd) = window_clone.hwnd() {
                    unsafe {
                        let hwnd_val = HWND_TYPE(hwnd.0 as _);
                        let _ = SetWindowPos(
                            hwnd_val,
                            None,
                            0,
                            0,
                            0,
                            0,
                            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
                        );
                    }
                }
            });
        }
    }

    if let Some(window) = app.get_webview_window("yolo-monitor") {
        let _ = window.hide();
    }

    // Ensure auxiliary windows can appear over fullscreen game space on macOS.
    for label in ["detail-popup"] {
        if let Some(window) = app.get_webview_window(label) {
            crate::platforms::window_style::enforce_overlay_traits(&window, label);
        }
    }

    Ok(())
}

fn setup_system_tray(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

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
        .on_menu_event(|app, event| match event.id.as_ref() {
            "quit" => {
                crate::core::lifecycle::allow_app_exit();
                app.exit(0);
            }
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    crate::platforms::window_style::enforce_overlay_traits(&window, "main");
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "reset" => {
                let app_clone = app.clone();
                std::thread::spawn(move || {
                    if let Err(e) = crate::platforms::commands::reset_window_geometry(app_clone) {
                        log::error!("[Tray] Failed to reset window geometry: {}", e);
                    }
                });
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    if window.is_visible().unwrap_or(false) {
                        let _ = window.hide();
                    } else {
                        crate::platforms::window_style::enforce_overlay_traits(&window, "main");
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
        })
        .build(app)?;

    Ok(())
}
