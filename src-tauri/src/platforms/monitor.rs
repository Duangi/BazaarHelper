use std::sync::atomic::Ordering;
use std::time::Duration;

use device_query::{DeviceQuery, DeviceState, MouseState};
use tauri::{Emitter, Manager};

#[cfg(target_os = "macos")]
use objc::{class, msg_send, sel, sel_impl};
#[cfg(target_os = "macos")]
use objc::rc::autoreleasepool;

#[cfg(target_os = "macos")]
fn macos_pressed_mouse_buttons_mask() -> Option<u64> {
    autoreleasepool(|| unsafe {
        let ns_event_class = class!(NSEvent);
        let mask: u64 = msg_send![ns_event_class, pressedMouseButtons];
        Some(mask)
    })
}

#[cfg(target_os = "macos")]
fn macos_mouse_button_pressed(button: i32) -> bool {
    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn CGEventSourceButtonState(state_id: i32, button: i32) -> bool;
    }
    // kCGEventSourceStateCombinedSessionState = 0
    let quartz = unsafe { CGEventSourceButtonState(0, button) };
    let appkit = macos_pressed_mouse_buttons_mask()
        .map(|mask| ((mask >> button) & 1) == 1)
        .unwrap_or(false);
    quartz || appkit
}

#[cfg(target_os = "macos")]
fn macos_recent_mouse_down(button: i32, window_ms: f64) -> bool {
    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn CGEventSourceSecondsSinceLastEventType(state_id: i32, event_type: u32) -> f64;
    }

    let event_type = match button {
        0 => 1_u32,  // kCGEventLeftMouseDown
        1 => 3_u32,  // kCGEventRightMouseDown
        2 => 25_u32, // kCGEventOtherMouseDown
        _ => return false,
    };
    // kCGEventSourceStateCombinedSessionState = 0
    let secs = unsafe { CGEventSourceSecondsSinceLastEventType(0, event_type) };
    (secs * 1000.0) <= window_ms
}

#[cfg(target_os = "macos")]
fn reapply_macos_overlay_traits(handle: &tauri::AppHandle) {
    // Safety: never re-pin main window in background loop (can steal focus/input on macOS).
    // Only refresh auxiliary overlays when they are visible.
    for label in ["detail-popup", "monster-calibration"] {
        if let Some(window) = handle.get_webview_window(label) {
            if window.is_visible().unwrap_or(false) {
                crate::platforms::window_style::refresh_overlay_pin(&window, label);
            }
        }
    }
}

pub fn spawn_focus_monitor(handle_focus: tauri::AppHandle) {
    std::thread::spawn(move || {
        let mut overlay_was_visible = false;
        let mut macos_guard_tick: u32 = 0;

        loop {
            std::thread::sleep(Duration::from_millis(1000));

            let game_active = crate::platforms::focus::is_game_window_active();
            if game_active {
                crate::platforms::focus::update_last_foreground_window();
            }

            #[cfg(target_os = "macos")]
            {
                macos_guard_tick = macos_guard_tick.wrapping_add(1);
                if game_active && macos_guard_tick % 6 == 0 {
                    reapply_macos_overlay_traits(&handle_focus);
                }
            }
            #[allow(unused_mut)]
            let mut app_active = false;

            #[cfg(target_os = "windows")]
            if !game_active {
                unsafe {
                    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
                    let fg_hwnd = GetForegroundWindow().0;

                    let check_windows = ["main", "detail-popup", "monster-calibration"];
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

            if crate::core::recognition_state::IS_RECOGNIZING.load(Ordering::Relaxed) {
                should_be_visible = true;
            } else if let Some(lock) = crate::core::recognition_state::LAST_RECOG_TIME.get() {
                if let Ok(guard) = lock.read() {
                    if let Some(time) = *guard {
                        if time.elapsed().as_secs_f32() < 1.0 {
                            should_be_visible = true;
                        }
                    }
                }
            }

            if should_be_visible != overlay_was_visible {
                log::debug!(
                    "[Focus Monitor] Visibility state changing: {} -> {} (Game: {}, App: {})",
                    overlay_was_visible,
                    should_be_visible,
                    game_active,
                    app_active
                );

                if !should_be_visible {
                    if let Some(window) = handle_focus.get_webview_window("detail-popup") {
                        if window.is_visible().unwrap_or(false) {
                            let _ = window.hide();
                        }
                    }
                }
                overlay_was_visible = should_be_visible;
            }
        }
    });
}

pub fn spawn_mouse_hotkey_monitor(handle_monitor: tauri::AppHandle) {
    std::thread::spawn(move || {
        log::info!(
            "[Hotkey Monitor] started. detail_hotkey={:?}, yolo_hotkey={:?}, monster_hotkey={:?}, card_hotkey={:?}",
            crate::core::hotkey_state::get_cached_detail_hotkey(),
            crate::core::hotkey_state::get_cached_yolo_hotkey(),
            crate::core::hotkey_state::get_cached_detection_hotkey(),
            crate::core::hotkey_state::get_cached_card_detection_hotkey()
        );
        #[cfg(target_os = "macos")]
        log::info!(
            "[Hotkey Monitor] macOS fallback enabled: if right-click is swallowed by fullscreen game mode, press Q or E to trigger detail."
        );
        let device_state = DeviceState::new();
        let mut last_trigger_active = false;
        let mut heartbeat_tick: u32 = 0;
        let mut game_active_cached = false;
        let mut game_refresh_tick: u32 = 0;

        let mut last_yolo_active = false;
        let mut last_detection_active = false;
        let mut last_card_active = false;
        let mut last_collapse_active = false;

        loop {
            let loop_sleep_ms = if game_active_cached { 50 } else { 250 };
            std::thread::sleep(Duration::from_millis(loop_sleep_ms));
            heartbeat_tick = heartbeat_tick.wrapping_add(1);

            let detail_visible = if let Some(w) = handle_monitor.get_webview_window("detail-popup") {
                w.is_visible().unwrap_or(false)
            } else {
                false
            };
            game_refresh_tick = game_refresh_tick.wrapping_add(1);
            if game_refresh_tick % 4 == 0 || detail_visible {
                game_active_cached = crate::platforms::focus::is_game_window_active();
                if game_active_cached {
                    crate::platforms::focus::update_last_foreground_window();
                }
            }
            let game_active = game_active_cached;
            let allow_hotkey_actions = game_active || detail_visible;

            if !game_active && !detail_visible {
                last_trigger_active = false;
                last_yolo_active = false;
                last_detection_active = false;
                last_card_active = false;
                last_collapse_active = false;
                continue;
            }

            let mouse: MouseState = device_state.get_mouse();
            let mx = mouse.coords.0;
            let my = mouse.coords.1;
            let left_click_idx = mouse.button_pressed.get(1).copied().unwrap_or(false)
                || mouse.button_pressed.first().copied().unwrap_or(false);
            let right_click_idx = mouse.button_pressed.get(3).copied().unwrap_or(false)
                || mouse.button_pressed.get(2).copied().unwrap_or(false);
            let left_click = {
                #[cfg(target_os = "macos")]
                {
                    left_click_idx || macos_mouse_button_pressed(0) || macos_recent_mouse_down(0, 120.0)
                }
                #[cfg(not(target_os = "macos"))]
                {
                    left_click_idx
                }
            };
            let right_click = {
                #[cfg(target_os = "macos")]
                {
                    right_click_idx || macos_mouse_button_pressed(1) || macos_recent_mouse_down(1, 120.0)
                }
                #[cfg(not(target_os = "macos"))]
                {
                    right_click_idx
                }
            };
            let middle_click = {
                #[cfg(target_os = "macos")]
                {
                    mouse.button_pressed.get(2).copied().unwrap_or(false)
                        || mouse.button_pressed.get(1).copied().unwrap_or(false)
                        || macos_mouse_button_pressed(2)
                        || macos_recent_mouse_down(2, 120.0)
                }
                #[cfg(not(target_os = "macos"))]
                {
                    mouse.button_pressed.get(2).copied().unwrap_or(false)
                        || mouse.button_pressed.get(1).copied().unwrap_or(false)
                }
            };

            // Keep detail-popup interactive: do not auto-hide on global mouse clicks.
            // Closing is handled by explicit user actions (hotkey toggle / UI controls / focus rules).

            let detail_code = crate::core::hotkey_state::get_cached_detail_hotkey().unwrap_or(2);
            let mut trigger_active = {
                #[cfg(target_os = "macos")]
                {
                    match detail_code {
                        1 => left_click,
                        2 => right_click,
                        4 => middle_click,
                        _ => crate::core::hotkey_state::is_key_pressed(detail_code, &device_state, &mouse),
                    }
                }
                #[cfg(not(target_os = "macos"))]
                {
                    crate::core::hotkey_state::is_key_pressed(detail_code, &device_state, &mouse)
                }
            };

            #[cfg(target_os = "macos")]
            {
                // Compatibility: in fullscreen game mode, always allow right-click to trigger detail
                // even when persisted hotkey was accidentally recorded as a keyboard key.
                if !trigger_active && game_active && right_click {
                    trigger_active = true;
                    log::info!(
                        "[Hotkey Monitor] macOS right-click compatibility trigger fired (configured detail_code={})",
                        detail_code
                    );
                }

                // Game mode may swallow mouse button state in exclusive fullscreen.
                // Keep keyboard fallback when detail hotkey is right click.
                if !trigger_active && detail_code == 2 {
                    trigger_active = crate::core::hotkey_state::is_key_pressed(81, &device_state, &mouse) // Q
                        || crate::core::hotkey_state::is_key_pressed(69, &device_state, &mouse); // E
                    if trigger_active {
                        log::info!("[Hotkey Monitor] fallback key(Q/E) triggered detail popup");
                    }
                }
            }

            if heartbeat_tick % 80 == 0 {
                log::debug!(
                    "[Hotkey Monitor] heartbeat detail_code={} trigger={} left={} right={} middle={} game_active={}",
                    detail_code, trigger_active, left_click, right_click, middle_click, game_active
                );
            }
            if trigger_active != last_trigger_active {
                log::debug!(
                    "[Hotkey Monitor] detail hotkey state: {} -> {} (mx={}, my={}, buttons={:?})",
                    last_trigger_active,
                    trigger_active,
                    mx,
                    my,
                    mouse.button_pressed
                );
                if trigger_active && !last_trigger_active {
                    log::info!(
                        "[Hotkey Monitor] detail trigger detected (code={}, game_active={}, detail_visible={})",
                        detail_code,
                        game_active,
                        detail_visible
                    );
                }
            }

            if detail_visible && trigger_active && !last_trigger_active {
                if let Some(popup_window) = handle_monitor.get_webview_window("detail-popup") {
                    log::debug!("[Hotkey Monitor] Custom hotkey pressed while popup open - hiding.");
                    let _ = popup_window.emit("hide-detail-popup", ());
                }
            }

            if trigger_active && !last_trigger_active && allow_hotkey_actions {
                log::debug!("[Global Hotkey] Detail Popup Hotkey pressed at ({}, {})", mx, my);
                crate::platforms::focus::update_last_foreground_window();

                let handle_clone = handle_monitor.clone();
                let click_x = mx;
                let click_y = my;

                tauri::async_runtime::spawn(async move {
                    let started = std::time::Instant::now();
                    let _ = handle_clone.emit(
                        "island-status",
                        serde_json::json!({
                            "message": "正在识别详情...",
                            "type": "info"
                        }),
                    );
                    match crate::services::overlay::handle_detail_hotkey_click(handle_clone.clone(), click_x, click_y).await {
                        Ok(Some(result)) => {
                            log::debug!("[Right Click] Found result: {:?}", result.get("type"));
                            let result_type = result
                                .get("type")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown")
                                .to_string();
                            let data = result.get("data").cloned().unwrap_or(serde_json::json!({}));

                            let island_payload = match result_type.as_str() {
                                "item" => {
                                    let item_name = data
                                        .get("name_cn")
                                        .and_then(|v| v.as_str())
                                        .or_else(|| data.get("name").and_then(|v| v.as_str()))
                                        .unwrap_or("未知卡牌");
                                    serde_json::json!({
                                        "message": format!("卡牌详情: {} ({}ms)", item_name, started.elapsed().as_millis()),
                                        "type": "success"
                                    })
                                }
                                "monster" => {
                                    let monster_name = data
                                        .get("name_zh")
                                        .and_then(|v| v.as_str())
                                        .or_else(|| data.get("name").and_then(|v| v.as_str()))
                                        .unwrap_or("未知野怪");
                                    serde_json::json!({
                                        "message": format!("识别目标: {} ({}ms)", monster_name, started.elapsed().as_millis()),
                                        "type": "info"
                                    })
                                }
                                "event" => {
                                    let event_name = data
                                        .get("Name")
                                        .and_then(|v| v.as_str())
                                        .or_else(|| data.get("Id").and_then(|v| v.as_str()))
                                        .unwrap_or("未知事件");
                                    serde_json::json!({
                                        "message": format!("识别事件: {} ({}ms)", event_name, started.elapsed().as_millis()),
                                        "type": "info"
                                    })
                                }
                                _ => serde_json::json!({
                                    "message": format!("详情识别完成 ({}ms)", started.elapsed().as_millis()),
                                    "type": "info"
                                }),
                            };
                            let _ = handle_clone.emit("island-status", island_payload);

                            if let Err(e) = crate::platforms::commands::show_detail_popup_at(
                                handle_clone.clone(),
                                click_x,
                                click_y,
                                result_type,
                                data,
                            )
                            .await
                            {
                                log::debug!("[Right Click] Error showing detail popup: {}", e);
                                let _ = handle_clone.emit(
                                    "island-status",
                                    serde_json::json!({
                                        "message": "详情弹窗显示失败",
                                        "type": "error"
                                    }),
                                );
                            }
                        }
                        Ok(None) => {
                            let _ = handle_clone.emit(
                                "island-status",
                                serde_json::json!({
                                    "message": format!("未识别到详情目标 ({}ms)", started.elapsed().as_millis()),
                                    "type": "warning"
                                }),
                            );
                        }
                        Err(e) => {
                            log::debug!("[Right Click] Error handling click: {}", e);
                            let _ = handle_clone.emit(
                                "island-status",
                                serde_json::json!({
                                    "message": format!("详情识别失败: {} ({}ms)", e, started.elapsed().as_millis()),
                                    "type": "error"
                                }),
                            );
                        }
                    }
                });
            }

            let yolo_active = if let Some(code) = crate::core::hotkey_state::get_cached_yolo_hotkey() {
                crate::core::hotkey_state::is_key_pressed(code, &device_state, &mouse)
            } else {
                false
            };
            if yolo_active && !last_yolo_active {
                log::debug!("[Global Hotkey] YOLO Trigger pressed!");
                let h = handle_monitor.clone();
                tauri::async_runtime::spawn(async move {
                    log::debug!("[Global Hotkey] Emitting 'yolo_hotkey_pressed'");
                    let _ = h.emit("yolo_hotkey_pressed", ());
                });
            }

            let detection_active = if let Some(code) = crate::core::hotkey_state::get_cached_detection_hotkey() {
                crate::core::hotkey_state::is_key_pressed(code, &device_state, &mouse)
            } else {
                false
            };
            if detection_active && !last_detection_active && allow_hotkey_actions {
                log::debug!("[Global Hotkey] Monster Mouse Trigger pressed!");
                crate::core::recognition_state::IS_RECOGNIZING.store(true, Ordering::Relaxed);
                crate::core::recognition_state::update_last_recog_time();

                let h = handle_monitor.clone();
                tauri::async_runtime::spawn(async move {
                    match crate::monster_recognition::recognize_monster_at_mouse().await {
                        Ok(Some(result)) => {
                            if let Some((day_str, monster_name)) = result.split_once('|') {
                                let day_num = if day_str.contains("10+") {
                                    10
                                } else {
                                    day_str.replace("Day ", "").trim().parse::<u32>().unwrap_or(1)
                                };

                                log::debug!("[Hotkey Monster] Matched: {} on Day {}", monster_name, day_num);
                                let _ = h.emit(
                                    "auto-jump-to-monster",
                                    serde_json::json!({
                                        "day": day_num,
                                        "monster_name": monster_name
                                    }),
                                );
                            }
                        }
                        Ok(None) => log::debug!("[Hotkey Monster] No object found at cursor"),
                        Err(e) => log::debug!("[Hotkey Monster] Error: {}", e),
                    }
                });
            }

            let card_active = if let Some(code) = crate::core::hotkey_state::get_cached_card_detection_hotkey() {
                crate::core::hotkey_state::is_key_pressed(code, &device_state, &mouse)
            } else {
                false
            };
            if card_active && !last_card_active && allow_hotkey_actions {
                log::debug!("[Global Hotkey] Card Recognition Trigger pressed!");
                crate::core::recognition_state::IS_RECOGNIZING.store(true, Ordering::Relaxed);
                crate::core::recognition_state::update_last_recog_time();

                let h = handle_monitor.clone();
                tauri::async_runtime::spawn(async move {
                    log::debug!("[Global Hotkey] Emitting 'hotkey-card'");
                    let _ = h.emit("hotkey-card", ());
                });
            }

            let collapse_active = if let Some(code) = crate::core::hotkey_state::get_cached_toggle_collapse_hotkey() {
                crate::core::hotkey_state::is_key_pressed(code, &device_state, &mouse)
            } else {
                false
            };
            if collapse_active && !last_collapse_active && allow_hotkey_actions {
                log::debug!("[Global Hotkey] Collapse/Expand Trigger pressed!");
                let h = handle_monitor.clone();
                tauri::async_runtime::spawn(async move {
                    log::debug!("[Global Hotkey] Emitting 'hotkey-collapse'");
                    let _ = h.emit("hotkey-collapse", ());
                });
            }

            last_trigger_active = trigger_active;
            last_yolo_active = yolo_active;
            last_detection_active = detection_active;
            last_card_active = card_active;
            last_collapse_active = collapse_active;
        }
    });
}
