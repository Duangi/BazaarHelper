use tauri::{Emitter, Manager};
use std::sync::atomic::Ordering;
use std::time::Duration;
use tokio::time::sleep;

fn should_fallback_gpu_to_cpu(err: &str) -> bool {
    let lower = err.to_ascii_lowercase();
    lower.contains("quickgelu")
        || lower.contains("directml")
        || lower.contains("execution provider")
        || lower.contains("tensor type")
        || lower.contains("non-zero status code")
}

#[tauri::command]
pub fn clear_yolo_cache() -> Result<String, String> {
    if crate::services::yolo_state::clear_cache() {
        crate::log_to_file("YOLO cache cleared to free memory");
    }
    Ok("YOLO缓存已清理".to_string())
}

#[tauri::command]
pub fn abort_yolo_scan() {
    log::debug!("[YOLO] Abort requested.");
    crate::ABORT_YOLO.store(true, Ordering::SeqCst);
}

#[tauri::command]
pub fn set_show_yolo_monitor(app: tauri::AppHandle, show: bool) -> Result<(), String> {
    use tauri::Manager;

    log::debug!("[YOLO Monitor] Deprecated, forcing hidden. Requested visibility: {}", show);
    if let Some(window) = app.get_webview_window("yolo-monitor") {
        let _ = window.hide();
    } else {
        log::debug!("[YOLO Monitor] WARNING: Window not found!");
    }

    let mut state = crate::load_state();
    state.show_yolo_monitor = false;
    crate::save_state(&state);
    Ok(())
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn trigger_yolo_scan(
    app: tauri::AppHandle,
    useGpu: bool,
    storeImage: Option<bool>,
) -> Result<usize, String> {
    crate::ABORT_YOLO.store(false, Ordering::SeqCst);
    let use_gpu_flag = useGpu;
    let store_image = storeImage.unwrap_or(true);
    use device_query::{DeviceQuery, DeviceState};
    use xcap::{Monitor, Window};

    let _ = app.emit("yolo-scan-start", ());
    for label in ["main"] {
        if let Some(window) = app.get_webview_window(label) {
            crate::platforms::window_style::refresh_overlay_pin(&window, label);
        }
    }

    let result = (|| -> Result<usize, String> {
        let resources_path = app.path().resource_dir().map_err(|e| e.to_string())?;
        let model_path = resources_path.join("resources").join("models").join("best.onnx");

        if crate::ABORT_YOLO.load(Ordering::SeqCst) {
            return Err("Aborted".into());
        }

        let windows = Window::all().map_err(|e| e.to_string())?;
        if crate::ABORT_YOLO.load(Ordering::SeqCst) {
            return Err("Aborted".into());
        }

        let device_state = DeviceState::new();
        let mouse = device_state.get_mouse();
        let (mouse_x, mouse_y) = mouse.coords;

        let candidates: Vec<&Window> = windows
            .iter()
            .filter(|w| {
                let title = w.title().to_lowercase();
                let app_name = w.app_name().to_lowercase();
                let is_excluded = title.contains("bazaarhelper") || app_name.contains("bazaarhelper");
                let is_bazaar = title.contains("the bazaar")
                    || app_name.contains("the bazaar")
                    || title.contains("thebazaar")
                    || app_name.contains("thebazaar")
                    || title.contains("bazaar")
                    || app_name.contains("bazaar");
                let valid_size = w.width() >= 640 && w.height() >= 360;
                is_bazaar && !is_excluded && valid_size
            })
            .collect();

        for w in candidates.iter().take(5) {
            log::debug!(
                "[YOLO] Candidate window: title='{}', app='{}', pos=({},{}), size={}x{}",
                w.title(),
                w.app_name(),
                w.x(),
                w.y(),
                w.width(),
                w.height()
            );
        }

        let target_window = candidates.into_iter().max_by_key(|w| {
            let inside_cursor = mouse_x >= w.x()
                && mouse_x < (w.x() + w.width() as i32)
                && mouse_y >= w.y()
                && mouse_y < (w.y() + w.height() as i32);
            let area = w.width() as i64 * w.height() as i64;
            (if inside_cursor { 1_i64 } else { 0_i64 }, area)
        });

        let capture_monitor_by_cursor =
            || -> Result<(image::RgbaImage, i32, i32, u32, u32), String> {
                let monitors = Monitor::all().map_err(|e| e.to_string())?;
                let device_state = DeviceState::new();
                let mouse = device_state.get_mouse();
                let (mx, my) = mouse.coords;

                let picked = monitors
                    .iter()
                    .find(|m| {
                        mx >= m.x()
                            && mx < (m.x() + m.width() as i32)
                            && my >= m.y()
                            && my < (m.y() + m.height() as i32)
                    })
                    .or_else(|| monitors.first())
                    .ok_or("No monitor found")?;

                log::debug!(
                    "[YOLO] Fallback capture monitor by cursor: mouse=({}, {}), monitor=({}, {}) {}x{}",
                    mx,
                    my,
                    picked.x(),
                    picked.y(),
                    picked.width(),
                    picked.height()
                );

                Ok((
                    picked.capture_image().map_err(|e| e.to_string())?,
                    picked.x(),
                    picked.y(),
                    picked.width(),
                    picked.height(),
                ))
            };

        let (screenshot, window_x, window_y, logical_width, logical_height) = if let Some(w) = target_window {
            let under_cursor = mouse_x >= w.x()
                && mouse_x < (w.x() + w.width() as i32)
                && mouse_y >= w.y()
                && mouse_y < (w.y() + w.height() as i32);
            log::debug!(
                "[YOLO] Found Game Window: title='{}', app='{}', pos=({},{}), size={}x{}, under_cursor={}",
                w.title(),
                w.app_name(),
                w.x(),
                w.y(),
                w.width(),
                w.height(),
                under_cursor
            );
            match w.capture_image() {
                Ok(img) => (img, w.x(), w.y(), w.width(), w.height()),
                Err(e) => {
                    log::warn!(
                        "[YOLO] Failed to capture matched game window ({}). Falling back to cursor monitor.",
                        e
                    );
                    capture_monitor_by_cursor()?
                }
            }
        } else {
            log::debug!("[YOLO] The Bazaar window not found, falling back to cursor monitor scan.");
            let monitors = Monitor::all().map_err(|e| e.to_string())?;
            if monitors.is_empty() {
                return Err("No monitor found".to_string());
            }
            capture_monitor_by_cursor()?
        };

        if crate::ABORT_YOLO.load(Ordering::SeqCst) {
            return Err("Aborted".into());
        }

        let img = image::DynamicImage::ImageRgba8(screenshot);
        log::debug!("[YOLO] Starting manual scan with GPU acceleration: {}...", use_gpu_flag);
        let detections = match crate::monster_recognition::run_yolo_inference(&img, &model_path, use_gpu_flag) {
            Ok(v) => v,
            Err(e) if use_gpu_flag && should_fallback_gpu_to_cpu(&e) => {
                log::warn!("[YOLO] GPU inference failed, retrying with CPU. cause={}", e);
                crate::log_to_file(&format!("[YOLO] GPU fallback to CPU: {}", e));
                crate::monster_recognition::run_yolo_inference(&img, &model_path, false)
                    .map_err(|cpu_err| format!("GPU推理失败且CPU回退失败: {}; {}", e, cpu_err))?
            }
            Err(e) => return Err(e),
        };
        if crate::ABORT_YOLO.load(Ordering::SeqCst) {
            return Err("Aborted".into());
        }

        log::debug!("[YOLO] Scan complete. Found {} objects.", detections.len());

        {
            let mut results = crate::get_yolo_scan_results().write().unwrap();
            *results = detections.clone();
        }
        if store_image {
            match crate::services::yolo_state::store_scan_image(&img) {
                Ok(seq) => {
                    tauri::async_runtime::spawn(async move {
                        sleep(Duration::from_secs(20)).await;
                        crate::services::yolo_state::clear_scan_image_if_seq(seq);
                    });
                }
                Err(e) => {
                    log::warn!("[YOLO] Failed to store scan image cache: {}", e);
                }
            }
        } else {
            let mut saved_img = crate::get_yolo_scan_image().write().unwrap();
            *saved_img = None;
        }
        {
            let mut offset = crate::get_yolo_window_offset().write().unwrap();
            *offset = (window_x, window_y);
            log::debug!("[YOLO] Saved window offset: ({}, {})", window_x, window_y);
        }
        crate::services::yolo_state::set_yolo_capture_meta(crate::services::yolo_state::YoloCaptureMeta {
            origin_x: window_x,
            origin_y: window_y,
            logical_width,
            logical_height,
            image_width: img.width(),
            image_height: img.height(),
        });
        log::debug!(
            "[YOLO] Saved capture meta: origin=({}, {}), logical={}x{}, image={}x{}",
            window_x,
            window_y,
            logical_width,
            logical_height,
            img.width(),
            img.height()
        );

        Ok(detections.len())
    })();

    match &result {
        Ok(count) => {
            log::debug!("[YOLO] Scan succeeded with {} detections", count);
            let _ = app.emit("yolo-scan-end", ());
        }
        Err(e) if e == "Aborted" => {
            log::debug!("[YOLO] Scan aborted by user.");
            let _ = app.emit("yolo-scan-end", ());
        }
        Err(e) => {
            crate::log_to_file(&format!("[YOLO Error] {}", e));
            let _ = app.emit("scan-error", e.clone());
        }
    }

    result
}

#[tauri::command]
pub fn recognize_monsters_from_screenshot(
    day: Option<u32>,
) -> Result<Vec<crate::monster_recognition::MonsterRecognitionResult>, String> {
    let day_filter = day.map(|d| {
        if d >= 10 {
            "Day 10+".to_string()
        } else {
            format!("Day {}", d)
        }
    });
    crate::monster_recognition::recognize_monsters(day_filter)
}

#[tauri::command]
pub fn get_template_loading_progress() -> crate::monster_recognition::LoadingProgress {
    crate::monster_recognition::get_loading_progress()
}

#[tauri::command]
pub fn get_yolo_stats() -> serde_json::Value {
    crate::services::yolo_state::collect_stats()
}

#[tauri::command]
pub async fn invoke_yolo_scan(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    crate::monster_recognition::recognize_monsters_yolo(&app)
}

#[tauri::command]
pub async fn emit_to_main(
    app: tauri::AppHandle,
    event: String,
    payload: serde_json::Value,
) -> Result<(), String> {
    app.emit(&event, payload)
        .map_err(|e| format!("Failed to emit event: {}", e))?;
    Ok(())
}
