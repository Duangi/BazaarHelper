use std::sync::atomic::Ordering;
use std::time::Duration;

use device_query::{DeviceQuery, DeviceState, MouseState};
use tauri::{Emitter, Manager};

#[cfg(target_os = "macos")]
fn reapply_macos_overlay_traits(handle: &tauri::AppHandle) {
    for label in ["detail-popup", "monster-calibration"] {
        if let Some(window) = handle.get_webview_window(label) {
            crate::platforms::window_style::refresh_overlay_pin(&window, label);
        }
    }
}

pub fn spawn_focus_monitor(handle_focus: tauri::AppHandle) {
    std::thread::spawn(move || {
        let mut overlay_was_visible = false;
        let mut macos_guard_tick: u32 = 0;

        loop {
            std::thread::sleep(Duration::from_millis(500));

            #[cfg(target_os = "macos")]
            {
                macos_guard_tick = macos_guard_tick.wrapping_add(1);
                if macos_guard_tick % 4 == 0 {
                    reapply_macos_overlay_traits(&handle_focus);
                }
            }

            let game_active = crate::platforms::focus::is_game_window_active();
            if game_active {
                crate::platforms::focus::update_last_foreground_window();
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
        let device_state = DeviceState::new();
        let mut last_left_click = false;
        let mut last_right_click = false;
        let mut last_trigger_active = false;

        let mut last_yolo_active = false;
        let mut last_detection_active = false;
        let mut last_card_active = false;
        let mut last_collapse_active = false;

        loop {
            std::thread::sleep(Duration::from_millis(50));

            let detail_visible = if let Some(w) = handle_monitor.get_webview_window("detail-popup") {
                w.is_visible().unwrap_or(false)
            } else {
                false
            };

            let game_active = crate::platforms::focus::is_game_window_active();

            if !game_active && !detail_visible {
                last_left_click = false;
                last_right_click = false;
                last_trigger_active = false;
            }

            let mouse: MouseState = device_state.get_mouse();
            let mx = mouse.coords.0;
            let my = mouse.coords.1;
            let left_click = mouse.button_pressed[1];
            let right_click = mouse.button_pressed[3];

            if game_active || detail_visible {
                if detail_visible && (left_click || right_click) {
                    if let Some(popup_window) = handle_monitor.get_webview_window("detail-popup") {
                        if let (Ok(pos), Ok(size)) = (popup_window.outer_position(), popup_window.outer_size()) {
                            let inside = mx >= pos.x
                                && mx <= pos.x + size.width as i32
                                && my >= pos.y
                                && my <= pos.y + size.height as i32;

                            if !inside && ((left_click && !last_left_click) || (right_click && !last_right_click)) {
                                log::debug!(
                                    "[Mouse Monitor] Click detected outside detail-popup at ({}, {}), hiding.",
                                    mx,
                                    my
                                );
                                let _ = popup_window.emit("hide-detail-popup", ());
                            }
                        }
                    }
                }
            }

            let trigger_active = if let Some(code) = crate::core::hotkey_state::get_cached_detail_hotkey() {
                crate::core::hotkey_state::is_key_pressed(code, &device_state, &mouse)
            } else {
                false
            };

            if detail_visible && trigger_active && !last_trigger_active {
                if let Some(popup_window) = handle_monitor.get_webview_window("detail-popup") {
                    log::debug!("[Hotkey Monitor] Custom hotkey pressed while popup open - hiding.");
                    let _ = popup_window.emit("hide-detail-popup", ());
                }
            }

            if trigger_active && !last_trigger_active && (game_active || detail_visible) {
                log::debug!("[Global Hotkey] Detail Popup Hotkey pressed at ({}, {})", mx, my);
                crate::platforms::focus::update_last_foreground_window();

                let handle_clone = handle_monitor.clone();
                let click_x = mx;
                let click_y = my;

                tauri::async_runtime::spawn(async move {
                    match crate::services::overlay::handle_overlay_right_click(handle_clone.clone(), click_x, click_y).await {
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
                                        "message": format!("卡牌详情: {}", item_name),
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
                                        "message": format!("识别目标: {}", monster_name),
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
                                        "message": format!("识别事件: {}", event_name),
                                        "type": "info"
                                    })
                                }
                                _ => serde_json::json!({
                                    "message": "详情识别完成",
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
                                    "message": "卡牌详情识别失败",
                                    "type": "warning"
                                }),
                            );
                        }
                        Err(e) => {
                            log::debug!("[Right Click] Error handling click: {}", e);
                            let _ = handle_clone.emit(
                                "island-status",
                                serde_json::json!({
                                    "message": "卡牌详情识别失败",
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
            if detection_active && !last_detection_active {
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
            if card_active && !last_card_active {
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
            if collapse_active && !last_collapse_active {
                log::debug!("[Global Hotkey] Collapse/Expand Trigger pressed!");
                let h = handle_monitor.clone();
                tauri::async_runtime::spawn(async move {
                    log::debug!("[Global Hotkey] Emitting 'hotkey-collapse'");
                    let _ = h.emit("hotkey-collapse", ());
                });
            }

            last_trigger_active = trigger_active;
            last_left_click = left_click;
            last_right_click = right_click;
            last_yolo_active = yolo_active;
            last_detection_active = detection_active;
            last_card_active = card_active;
            last_collapse_active = collapse_active;
        }
    });
}
