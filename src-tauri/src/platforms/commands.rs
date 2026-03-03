use tauri::Emitter;

#[tauri::command]
pub fn save_window_geometry(window_label: String, x: Option<i32>, y: Option<i32>, width: Option<u32>, height: Option<u32>) {
    if window_label == "main" {
        log::debug!("[Geometry] Saving main window: x={:?}, y={:?}, w={:?}, h={:?}", x, y, width, height);
        let mut state = crate::load_state();
        let mut changed = false;

        if let Some(val) = x {
            state.main_window_x = Some(val);
            changed = true;
        }
        if let Some(val) = y {
            state.main_window_y = Some(val);
            changed = true;
        }
        if let Some(val) = width {
            state.main_window_width = Some(val);
            changed = true;
        }
        if let Some(val) = height {
            state.main_window_height = Some(val);
            changed = true;
        }

        if changed {
            crate::save_state(&state);
            log::debug!("[Geometry] State saved to disk.");
        }
    }
}

#[tauri::command]
pub fn get_window_geometry(window_label: String) -> serde_json::Value {
    if window_label == "main" {
        let state = crate::load_state();
        log::debug!(
            "[Geometry] Loading saved position: x={:?}, y={:?}, w={:?}, h={:?}",
            state.main_window_x,
            state.main_window_y,
            state.main_window_width,
            state.main_window_height
        );
        return serde_json::json!({
            "x": state.main_window_x,
            "y": state.main_window_y,
            "width": state.main_window_width,
            "height": state.main_window_height
        });
    }
    if window_label == "detail-popup" {
        let state = crate::load_state();
        return serde_json::json!({
            "x": state.detail_popup_x,
            "y": state.detail_popup_y,
            "width": state.detail_popup_width,
            "height": state.detail_popup_height
        });
    }
    serde_json::json!({})
}

#[tauri::command]
pub fn reset_window_geometry(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;
    use xcap::Monitor;

    log::debug!("[Geometry] Reset window geometry triggered");

    let mut state = crate::load_state();
    state.main_window_x = None;
    state.main_window_y = None;
    state.main_window_width = None;
    state.main_window_height = None;
    crate::save_state(&state);
    log::debug!("[Geometry] Cleared saved geometry");

    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "Main window not found".to_string())?;

    let monitors = Monitor::all().map_err(|e| e.to_string())?;
    let primary_monitor = monitors
        .into_iter()
        .next()
        .ok_or_else(|| "No monitor found".to_string())?;

    let default_width = 600;
    let default_height = 850;

    let scale_factor = 1.0;
    let logical_width = (primary_monitor.width() as f64 / scale_factor) as i32;
    let target_x = primary_monitor.x() + logical_width - default_width;
    let target_y = primary_monitor.y();

    log::debug!(
        "[Geometry] Resetting to: x={}, y={}, w={}, h={}",
        target_x,
        target_y,
        default_width,
        default_height
    );

    use tauri::{PhysicalPosition, PhysicalSize};
    window
        .set_size(PhysicalSize::new(default_width as u32, default_height as u32))
        .map_err(|e| e.to_string())?;
    window
        .set_position(PhysicalPosition::new(target_x, target_y))
        .map_err(|e| e.to_string())?;

    crate::platforms::window_style::enforce_overlay_traits(&window, "main");
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;
    window
        .emit("reset-window-geometry", ())
        .map_err(|e| e.to_string())?;

    log::debug!("[Geometry] Window geometry reset complete");
    Ok(())
}

#[tauri::command]
pub fn prepare_app_exit() {
    crate::core::lifecycle::allow_app_exit();
}

#[tauri::command]
pub fn request_app_exit(app: tauri::AppHandle) -> Result<(), String> {
    crate::core::lifecycle::allow_app_exit();
    app.exit(0);
    Ok(())
}

#[tauri::command]
pub fn get_detection_hotkey() -> Option<i32> {
    crate::load_state().detection_hotkey
}

#[tauri::command]
pub fn get_card_detection_hotkey() -> Option<i32> {
    crate::load_state().card_detection_hotkey
}

#[tauri::command]
pub fn get_toggle_collapse_hotkey() -> Option<i32> {
    crate::load_state().toggle_collapse_hotkey
}

#[tauri::command]
pub fn set_detection_hotkey(hotkey: i32) {
    let mut state = crate::load_state();
    state.detection_hotkey = Some(hotkey);
    crate::save_state(&state);
    log::debug!("[Config] Detection hotkey updated to: {}", hotkey);
    crate::core::hotkey_state::update_detection_hotkey_cache(Some(hotkey));
}

#[tauri::command]
pub fn set_card_detection_hotkey(hotkey: i32) {
    let mut state = crate::load_state();
    state.card_detection_hotkey = Some(hotkey);
    crate::save_state(&state);
    log::debug!("[Config] Card detection hotkey updated to: {}", hotkey);
    crate::core::hotkey_state::update_card_detection_hotkey_cache(Some(hotkey));
}

#[tauri::command]
pub fn set_toggle_collapse_hotkey(hotkey: i32) {
    let mut state = crate::load_state();
    state.toggle_collapse_hotkey = Some(hotkey);
    crate::save_state(&state);
    crate::core::hotkey_state::update_toggle_collapse_hotkey_cache(Some(hotkey));
    log::debug!("[Config] Toggle collapse hotkey updated to: {}", hotkey);
}

#[tauri::command]
pub fn get_yolo_hotkey() -> Option<i32> {
    crate::load_state().yolo_hotkey
}

#[tauri::command]
pub fn set_yolo_hotkey(hotkey: i32) {
    let mut state = crate::load_state();
    state.yolo_hotkey = Some(hotkey);
    crate::save_state(&state);
    crate::core::hotkey_state::update_yolo_hotkey_cache(Some(hotkey));
    log::debug!("[Config] YOLO hotkey updated to: {}", hotkey);
}

#[tauri::command]
pub fn reset_all_hotkeys() {
    let mut state = crate::load_state();
    state.detection_hotkey = None;
    state.card_detection_hotkey = None;
    state.toggle_collapse_hotkey = None;
    state.yolo_hotkey = None;
    state.detail_display_hotkey = None;

    crate::core::hotkey_state::update_detection_hotkey_cache(None);
    crate::core::hotkey_state::update_card_detection_hotkey_cache(None);
    crate::core::hotkey_state::update_toggle_collapse_hotkey_cache(None);
    crate::core::hotkey_state::update_yolo_hotkey_cache(None);
    crate::core::hotkey_state::update_detail_hotkey_cache(None);
    crate::save_state(&state);
    log::debug!("[Config] All hotkeys reset to None (disabled)");
}

#[tauri::command]
pub fn save_monster_calibration(calibration: crate::MonsterCalibration) {
    let mut state = crate::load_state();

    let mut sorted_regions = calibration.regions;
    sorted_regions.sort_by(|a, b| a.x.cmp(&b.x));

    let sorted_calibration = crate::MonsterCalibration {
        regions: sorted_regions,
        game_window_width: calibration.game_window_width,
        game_window_height: calibration.game_window_height,
        screen_width: calibration.screen_width,
        screen_height: calibration.screen_height,
    };

    state.monster_calibration = Some(sorted_calibration.clone());
    crate::save_state(&state);
    log::debug!("[Monster Calibration] Saved calibration data: {:?}", sorted_calibration);
}

#[tauri::command]
pub fn load_monster_calibration() -> Option<crate::MonsterCalibration> {
    let state = crate::load_state();
    state.monster_calibration
}

#[tauri::command]
pub fn get_game_window_info() -> Result<serde_json::Value, String> {
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

#[tauri::command]
pub async fn open_calibration_window(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;

    if let Some(window) = app.get_webview_window("monster-calibration") {
        let _ = window.set_focus();
        return Ok(());
    }

    let windows = xcap::Window::all().map_err(|e| e.to_string())?;
    let mut game_window: Option<(i32, i32, u32, u32)> = None;

    for window in windows {
        let title = window.title();
        if title.to_lowercase().contains("the bazaar") {
            game_window = Some((window.x(), window.y(), window.width(), window.height()));
            log::debug!(
                "[Calibration] Game window found: x={}, y={}, width={}, height={}",
                window.x(),
                window.y(),
                window.width(),
                window.height()
            );
            break;
        }
    }

    let (phys_x, phys_y, phys_width, phys_height) = game_window.ok_or("Game window not found".to_string())?;

    let main_window = app.get_webview_window("main").ok_or("Main window not found")?;
    let scale_factor = main_window.scale_factor().map_err(|e| e.to_string())?;

    log::debug!("[Calibration] Scale factor: {}", scale_factor);

    let logical_x = (phys_x as f64) / scale_factor;
    let logical_y = (phys_y as f64) / scale_factor;
    let logical_width = (phys_width as f64) / scale_factor;
    let logical_height = (phys_height as f64) / scale_factor;

    log::debug!(
        "[Calibration] Logical coordinates: x={}, y={}, width={}, height={}",
        logical_x,
        logical_y,
        logical_width,
        logical_height
    );

    use tauri::WebviewUrl;
    use tauri::WebviewWindowBuilder;
    let window = WebviewWindowBuilder::new(
        &app,
        "monster-calibration",
        WebviewUrl::App("index.html".into()),
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

    crate::platforms::window_style::enforce_overlay_traits(&window, "monster-calibration");
    window.set_ignore_cursor_events(false).map_err(|e| e.to_string())?;
    log::debug!("[Calibration] Window created successfully");

    Ok(())
}

#[tauri::command]
pub async fn close_calibration_window(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;
    if let Some(window) = app.get_webview_window("monster-calibration") {
        window.close().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn get_detail_display_hotkey() -> Option<i32> {
    crate::load_state().detail_display_hotkey
}

#[tauri::command]
pub fn set_detail_display_hotkey(hotkey: i32) {
    let mut state = crate::load_state();
    state.detail_display_hotkey = Some(hotkey);
    crate::save_state(&state);
    crate::core::hotkey_state::update_detail_hotkey_cache(Some(hotkey));
    log::debug!("[Config] Detail display hotkey updated to: {}", hotkey);
}

#[tauri::command]
pub async fn set_overlay_ignore_cursor(_app: tauri::AppHandle, _ignore: bool) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub async fn show_yolo_monitor_window(app: tauri::AppHandle, show: bool) -> Result<(), String> {
    use tauri::Manager;
    if let Some(window) = app.get_webview_window("yolo-monitor") {
        let _ = show;
        let _ = window.hide();
    }
    Ok(())
}

fn ensure_detail_popup_window(app: &tauri::AppHandle) -> Result<tauri::WebviewWindow, String> {
    use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

    if let Some(window) = app.get_webview_window("detail-popup") {
        return Ok(window);
    }

    let window = WebviewWindowBuilder::new(app, "detail-popup", WebviewUrl::App("index.html".into()))
        .title("Detail Popup")
        .inner_size(480.0, 700.0)
        .resizable(true)
        .always_on_top(true)
        .decorations(false)
        .transparent(true)
        .skip_taskbar(true)
        .visible(false)
        .visible_on_all_workspaces(true)
        .build()
        .map_err(|e| e.to_string())?;

    crate::platforms::window_style::reset_overlay_init_flag("detail-popup");
    crate::platforms::window_style::enforce_overlay_traits(&window, "detail-popup");
    Ok(window)
}

#[tauri::command]
pub async fn warmup_detail_popup(app: tauri::AppHandle) -> Result<(), String> {
    let window = ensure_detail_popup_window(&app)?;
    crate::platforms::window_style::enforce_overlay_traits(&window, "detail-popup");
    let _ = window.hide();
    log::debug!("[Detail Popup] warmup complete");
    Ok(())
}

#[tauri::command]
pub async fn show_detail_popup_at(
    app: tauri::AppHandle,
    x: i32,
    y: i32,
    data_type: String,
    data: serde_json::Value,
) -> Result<(), String> {
    use tauri::Emitter;

    log::debug!(
        "[Show Detail Popup] Requested position: ({}, {}), Type: {}",
        x,
        y,
        data_type
    );

    let window = ensure_detail_popup_window(&app)?;
    log::debug!("[Show Detail Popup] Window ready");
    let state = crate::load_state();

    let saved_width = state.detail_popup_width.unwrap_or(480);
    let saved_height = state.detail_popup_height.unwrap_or(700);
    let final_width = if saved_width < 100 { 480 } else { saved_width };
    let final_height = if saved_height < 100 { 700 } else { saved_height };

    let (final_x, final_y) =
        if let (Some(saved_x), Some(saved_y)) = (state.detail_popup_x, state.detail_popup_y) {
            (saved_x, saved_y)
        } else {
            let monitors = window.available_monitors().map_err(|e| e.to_string())?;

            let target_monitor = monitors.iter().find(|m| {
                let pos = m.position();
                let size = m.size();
                x >= pos.x && x < (pos.x + size.width as i32) && y >= pos.y && y < (pos.y + size.height as i32)
            });

            if let Some(monitor) = target_monitor {
                let pos = monitor.position();
                let size = monitor.size();
                let center_x = pos.x + (size.width as i32 / 2) - (final_width as i32 / 2);
                let center_y = pos.y + (size.height as i32 / 2) - (final_height as i32 / 2);
                log::debug!(
                    "[Show Detail Popup] Mouse on monitor at ({}, {}), size {}x{}, centering at ({}, {})",
                    pos.x,
                    pos.y,
                    size.width,
                    size.height,
                    center_x,
                    center_y
                );
                (center_x, center_y)
            } else {
                (x - 200, y - 300)
            }
        };

    log::debug!(
        "[Show Detail Popup] Using position: ({}, {}), size: {}x{}",
        final_x,
        final_y,
        final_width,
        final_height
    );

    #[cfg(target_os = "macos")]
    {
        let window_main = window.clone();
        if let Err(e) = window.run_on_main_thread(move || {
            let _ = window_main.set_size(tauri::PhysicalSize::new(final_width, final_height));
            let _ = window_main.set_position(tauri::PhysicalPosition::new(final_x, final_y));
            let _ = window_main.set_always_on_top(true);
            crate::platforms::window_style::enforce_overlay_traits(&window_main, "detail-popup");
            let _ = window_main.show();
            crate::platforms::window_style::apply_no_activate_style(&window_main);
        }) {
            log::warn!("[Show Detail Popup] Failed to dispatch on macOS main thread: {}", e);
        } else {
            log::debug!("[Show Detail Popup] Window shown successfully (macOS main thread)");
            if let Ok(visible) = window.is_visible() {
                log::debug!("[Show Detail Popup] Visibility after show: {}", visible);
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = window.set_size(tauri::PhysicalSize::new(final_width, final_height));

        match window.set_position(tauri::PhysicalPosition::new(final_x, final_y)) {
            Ok(_) => log::debug!("[Show Detail Popup] Position set successfully"),
            Err(e) => log::debug!("[Show Detail Popup] Failed to set position: {}", e),
        }

        match window.set_always_on_top(true) {
            Ok(_) => log::debug!("[Show Detail Popup] Set always on top"),
            Err(e) => log::debug!("[Show Detail Popup] Failed to set always on top: {}", e),
        }
        crate::platforms::window_style::enforce_overlay_traits(&window, "detail-popup");

        match window.show() {
            Ok(_) => {
                log::debug!("[Show Detail Popup] Window shown successfully");
                if let Ok(visible) = window.is_visible() {
                    log::debug!("[Show Detail Popup] Visibility after show: {}", visible);
                }
                crate::platforms::window_style::apply_no_activate_style(&window);
            }
            Err(e) => log::debug!("[Show Detail Popup] Failed to show window: {}", e),
        }
    }

    let payload = serde_json::json!({
        "type": data_type,
        "data": data
    });
    log::debug!("[Show Detail Popup] Emitting event with payload type: {}", data_type);
    match window.emit("show-detail-popup", payload) {
        Ok(_) => log::debug!("[Show Detail Popup] Event emitted to detail-popup window successfully"),
        Err(e) => log::debug!("[Show Detail Popup] Failed to emit event: {}", e),
    }

    log::debug!("[Show Detail Popup] Window label: {}", window.label());

    Ok(())
}

#[tauri::command]
pub async fn hide_detail_popup(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::{Emitter, Manager};
    if let Some(window) = app.get_webview_window("detail-popup") {
        if let (Ok(position), Ok(size)) = (window.outer_position(), window.outer_size()) {
            if size.width >= 100 && size.height >= 100 {
                let mut state = crate::load_state();
                state.detail_popup_x = Some(position.x);
                state.detail_popup_y = Some(position.y);
                state.detail_popup_width = Some(size.width);
                state.detail_popup_height = Some(size.height);
                crate::save_state(&state);
                log::debug!(
                    "[Hide Detail Popup] Saved position: ({}, {}), size: {}x{}",
                    position.x,
                    position.y,
                    size.width,
                    size.height
                );
            } else {
                log::debug!(
                    "[Hide Detail Popup] Skipping save - window too small: {}x{}",
                    size.width,
                    size.height
                );
            }
        }
        let _ = window.emit("hide-detail-popup", ());
    }
    Ok(())
}

#[tauri::command]
pub async fn reset_detail_popup_position(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;

    log::debug!("[Reset Detail Popup] Clearing saved position and size");
    let mut state = crate::load_state();
    state.detail_popup_x = None;
    state.detail_popup_y = None;
    state.detail_popup_width = None;
    state.detail_popup_height = None;
    crate::save_state(&state);

    if let Some(window) = app.get_webview_window("detail-popup") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
            log::debug!("[Reset Detail Popup] Window was visible, hiding it");
        }
    }

    log::debug!("[Reset Detail Popup] Position reset complete");
    Ok(())
}

#[tauri::command]
pub async fn save_detail_popup_geometry(
    _app: tauri::AppHandle,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<(), String> {
    let mut state = crate::load_state();

    if width > 100 && height > 100 {
        state.detail_popup_x = Some(x);
        state.detail_popup_y = Some(y);
        state.detail_popup_width = Some(width);
        state.detail_popup_height = Some(height);
        crate::save_state(&state);
        log::debug!(
            "[Geometry] Saved Detail Popup: x={}, y={}, w={}, h={}",
            x,
            y,
            width,
            height
        );
    }
    Ok(())
}
