use std::sync::{Arc, Mutex};

use image::GenericImageView;
use opencv::core::MatTraitConst;
use tauri::{Manager, State};

use crate::monster_recognition::{self, YoloDetection};
use crate::{DbState, ItemData};

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct BoundsRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

pub struct OverlayState(Arc<Mutex<Vec<BoundsRect>>>);

impl OverlayState {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(Vec::new())))
    }
}

fn infer_card_size_by_ratio(aspect_ratio: f32) -> &'static str {
    // Use nearest canonical ratio to reduce misclassification around thresholds.
    // Small ~= 0.56 (1x2), Medium ~= 1.0 (2x2), Large ~= 1.5 (3x2)
    let candidates = [("Small", 0.56_f32), ("Medium", 1.0_f32), ("Large", 1.5_f32)];
    candidates
        .iter()
        .min_by(|a, b| {
            let da = (aspect_ratio - a.1).abs();
            let db = (aspect_ratio - b.1).abs();
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(name, _)| *name)
        .unwrap_or("Medium")
}

fn prioritized_card_sizes(primary: &str) -> [&'static str; 3] {
    match primary {
        "Small" => ["Small", "Medium", "Large"],
        "Large" => ["Large", "Medium", "Small"],
        _ => ["Medium", "Small", "Large"],
    }
}

#[tauri::command]
pub fn update_overlay_bounds(bounds: Vec<BoundsRect>, state: State<'_, OverlayState>) {
    let mut bounds_state = state.0.lock().unwrap();
    *bounds_state = bounds;
}

#[tauri::command]
pub async fn handle_overlay_right_click(
    app: tauri::AppHandle,
    x: i32,
    y: i32,
) -> Result<Option<serde_json::Value>, String> {
    let detections = crate::get_yolo_scan_results().read().unwrap().clone();
    let img_opt = {
        let image_bytes = crate::get_yolo_scan_image().read().unwrap();
        image_bytes
            .as_ref()
            .and_then(|bytes| image::load_from_memory(bytes).ok())
    };

    let meta = *crate::services::yolo_state::get_yolo_capture_meta()
        .read()
        .unwrap();
    let (fallback_x, fallback_y) = *crate::get_yolo_window_offset().read().unwrap();
    let window_x = if meta.logical_width > 0 { meta.origin_x } else { fallback_x };
    let window_y = if meta.logical_height > 0 { meta.origin_y } else { fallback_y };

    if img_opt.is_none() {
        return Err("暂无YOLO截图，请先触发一次YOLO扫描".to_string());
    }
    if detections.is_empty() {
        return Err("YOLO结果为空，请先触发YOLO并确认识别到目标".to_string());
    }
    let img = img_opt.unwrap();
    let (img_w, img_h) = img.dimensions();

    let logical_width = if meta.logical_width > 0 { meta.logical_width } else { img_w };
    let logical_height = if meta.logical_height > 0 { meta.logical_height } else { img_h };
    let rel_x_logical = x - window_x;
    let rel_y_logical = y - window_y;
    let scale_x = img_w as f32 / logical_width.max(1) as f32;
    let scale_y = img_h as f32 / logical_height.max(1) as f32;
    let rel_x = (rel_x_logical as f32 * scale_x).round() as i32;
    let rel_y = (rel_y_logical as f32 * scale_y).round() as i32;

    log::debug!("[YOLO Click] Screen=({}, {}), origin=({}, {}), logical={}x{}, image={}x{}, rel_logical=({}, {}), scale=({:.3},{:.3}), rel_image=({}, {})",
        x, y, window_x, window_y, logical_width, logical_height, img_w, img_h, rel_x_logical, rel_y_logical, scale_x, scale_y, rel_x, rel_y);
    log::debug!("[DEBUG] Image dimensions: {}x{}, Total detections: {}", img_w, img_h, detections.len());

    for (i, d) in detections.iter().enumerate() {
        log::debug!("[DEBUG] Detection {}: class={}, bounds=[{},{},{},{}], size={}x{}",
            i, d.class_id, d.x1, d.y1, d.x2, d.y2, d.x2 - d.x1, d.y2 - d.y1);
    }

    let target_detection = detections
        .iter()
        .find(|d| rel_x >= d.x1 && rel_x <= d.x2 && rel_y >= d.y1 && rel_y <= d.y2);

    if let Some(det) = target_detection {
        log::debug!(
            "[YOLO Click] Clicked on Class {} at [{}, {}, {}, {}]",
            det.class_id,
            det.x1,
            det.y1,
            det.x2,
            det.y2
        );

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

        if det.class_id == 2 || det.class_id == 6 {
            let card_width = (det.x2 - det.x1) as f32;
            let card_height = (det.y2 - det.y1) as f32;
            let aspect_ratio = if card_height > 0.0 {
                card_width / card_height
            } else {
                1.0
            };
            let inferred_size = infer_card_size_by_ratio(aspect_ratio);

            log::debug!(
                "[YOLO Click] Card dimensions: {}x{}, aspect_ratio: {:.2}, inferred size: {}",
                card_width,
                card_height,
                aspect_ratio,
                inferred_size
            );

            for card_size in prioritized_card_sizes(inferred_size) {
                let match_result = monster_recognition::match_card_by_size(&scene_desc, card_size);
                match match_result {
                    Ok(Some(cards)) => {
                        if let Some(card_list) = cards.as_array() {
                            if !card_list.is_empty() {
                                let card_id = card_list[0]["id"].as_str().unwrap_or("").to_string();
                                log::debug!("[YOLO Click] Card matched by {}: {}", card_size, card_id);
                                let db_state = app.state::<DbState>();
                                if let Some(info) = get_item_info_internal(&db_state, card_id.clone()).await {
                                    return Ok(Some(serde_json::json!({ "type": "item", "data": info })));
                                }
                                log::debug!(
                                    "[YOLO Click] Card {} not found in DB after {} match",
                                    card_id,
                                    card_size
                                );
                            }
                        }
                    }
                    Ok(None) => {
                        log::debug!(
                            "[YOLO Click] No card descriptors matched in {} category",
                            card_size
                        );
                    }
                    Err(e) => {
                        log::debug!("[YOLO Click] Card matching error ({}): {}", card_size, e);
                    }
                }
            }

            // Fallback ORB only when cursor is confirmed on a YOLO card box.
            match crate::monster_recognition::recognize_card_at_mouse().await {
                Ok(Some(raw)) => {
                    let first_id = raw
                        .as_array()
                        .and_then(|arr| arr.first())
                        .and_then(|v| v.get("id"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    if let Some(card_id) = first_id {
                        let db_state = app.state::<DbState>();
                        if let Some(info) = get_item_info_internal(&db_state, card_id.clone()).await {
                            log::debug!(
                                "[YOLO Click] ORB fallback matched card under YOLO-hit: {}",
                                card_id
                            );
                            return Ok(Some(serde_json::json!({ "type": "item", "data": info })));
                        }
                    }
                }
                Ok(None) => {
                    log::debug!("[YOLO Click] ORB fallback returned no result under YOLO-hit card.");
                }
                Err(e) => {
                    log::debug!("[YOLO Click] ORB fallback error under YOLO-hit card: {}", e);
                }
            }
        } else if det.class_id == 1 {
            let monster_icons: Vec<&YoloDetection> =
                detections.iter().filter(|d| d.class_id == 3).collect();
            let mut is_monster = false;

            for icon in monster_icons {
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
                let event_match = monster_recognition::match_event_descriptors_from_mat(&scene_desc)?;
                if let Some(event_id) = event_match {
                    let event_json_path = app
                        .path()
                        .resolve(
                            "resources/event_encounters.json",
                            tauri::path::BaseDirectory::Resource,
                        )
                        .map_err(|e| format!("Failed to resolve event_encounters.json: {}", e))?;

                    if let Ok(json_data) = std::fs::read_to_string(&event_json_path) {
                        if let Ok(events) = serde_json::from_str::<Vec<serde_json::Value>>(&json_data) {
                            if let Some(event) = events
                                .iter()
                                .find(|e| e.get("Id").and_then(|v| v.as_str()) == Some(&event_id))
                            {
                                return Ok(Some(serde_json::json!({ "type": "event", "data": event })));
                            }
                        }
                    }
                }
            }
        } else if det.class_id == 3 {
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

    Ok(None)
}

pub async fn handle_detail_hotkey_click(
    app: tauri::AppHandle,
    x: i32,
    y: i32,
) -> Result<Option<serde_json::Value>, String> {
    let primary = handle_overlay_right_click(app.clone(), x, y).await;
    if let Ok(Some(value)) = primary {
        return Ok(Some(value));
    }

    // Strict mode: detail popup is only triggered by YOLO-hit target under cursor.
    // Do not fallback to ORB-only mouse recognition anymore.
    let mut primary_err = primary.err();
    let should_retry_with_rescan = matches!(
        &primary_err,
        Some(e)
            if e.contains("暂无YOLO截图")
                || e.contains("YOLO结果为空")
                || e.contains("Invalid crop size")
    );

    if should_retry_with_rescan {
        log::debug!("[Detail Hotkey] YOLO cache miss, triggering fresh scan and retrying click.");
        match crate::services::commands::trigger_yolo_scan(app.clone(), true, Some(true)).await {
            Ok(count) => log::debug!("[Detail Hotkey] Fresh YOLO scan done, detections={}", count),
            Err(e) => log::debug!("[Detail Hotkey] Fresh YOLO scan failed: {}", e),
        }

        let retry = handle_overlay_right_click(app.clone(), x, y).await;
        if let Ok(Some(value)) = retry {
            return Ok(Some(value));
        }
        if let Ok(None) = retry {
            return Ok(None);
        }
        if primary_err.is_none() {
            primary_err = retry.err();
        }
    }

    if let Some(err) = primary_err {
        Err(err)
    } else {
        Ok(None)
    }
}

async fn get_item_info_internal(state: &DbState, id: String) -> Option<ItemData> {
    let db = state.items.read().unwrap();
    if let Some(&idx) = db.id_map.get(&id) {
        return Some(db.list[idx].clone());
    }
    None
}
