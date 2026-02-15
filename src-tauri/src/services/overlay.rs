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
    let img_opt = crate::get_yolo_scan_image().read().unwrap().clone();

    let (window_x, window_y, window_logical_width, window_logical_height) = {
        let game_window = xcap::Window::all().ok().and_then(|windows| {
            windows.into_iter().find(|w| {
                let title = w.title().to_lowercase();
                let app_name = w.app_name().to_lowercase();
                title.contains("the bazaar")
                    || app_name.contains("the bazaar")
                    || title.contains("thebazaar")
                    || app_name.contains("thebazaar")
            })
        });

        if let Some(window) = game_window {
            (window.x(), window.y(), window.width(), window.height())
        } else {
            let (x, y) = *crate::get_yolo_window_offset().read().unwrap();
            (x, y, 0, 0)
        }
    };

    if img_opt.is_none() {
        return Ok(None);
    }
    let img = img_opt.unwrap();
    let (img_w, img_h) = img.dimensions();

    let rel_x_logical = x - window_x;
    let rel_y_logical = y - window_y;

    let scale_factor = if window_logical_width > 0 && window_logical_height > 0 {
        let scale_x = img_w as f32 / window_logical_width as f32;
        let scale_y = img_h as f32 / window_logical_height as f32;
        (scale_x + scale_y) / 2.0
    } else {
        #[cfg(target_os = "macos")]
        {
            if img_w > 1920 { 2.0 } else { 1.0 }
        }
        #[cfg(not(target_os = "macos"))]
        {
            if img_w > 3000 { 1.5 } else { 1.0 }
        }
    };

    let rel_x = (rel_x_logical as f32 * scale_factor) as i32;
    let rel_y = (rel_y_logical as f32 * scale_factor) as i32;

    log::debug!("[YOLO Click] Screen coords: ({}, {}), Window offset: ({}, {}), Window size: {}x{}, Logical relative: ({}, {}), Scale: {:.2}, Physical relative: ({}, {})",
        x, y, window_x, window_y, window_logical_width, window_logical_height, rel_x_logical, rel_y_logical, scale_factor, rel_x, rel_y);
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
            let aspect_ratio = card_width / card_height;

            let card_size = if (0.85..=1.15).contains(&aspect_ratio) {
                "Medium"
            } else if aspect_ratio > 1.15 {
                "Large"
            } else {
                "Small"
            };

            log::debug!(
                "[YOLO Click] Card dimensions: {}x{}, aspect_ratio: {:.2}, detected size: {}",
                card_width,
                card_height,
                aspect_ratio,
                card_size
            );

            let match_result = monster_recognition::match_card_by_size(&scene_desc, card_size);
            match match_result {
                Ok(Some(cards)) => {
                    let card_list = cards.as_array().unwrap();
                    if !card_list.is_empty() {
                        let card_id = card_list[0]["id"].as_str().unwrap_or("").to_string();
                        log::debug!("[YOLO Click] Card matched: {}", card_id);
                        let db_state = app.state::<DbState>();
                        if let Some(info) = get_item_info_internal(&db_state, card_id.clone()).await {
                            return Ok(Some(serde_json::json!({ "type": "item", "data": info })));
                        }

                        log::debug!("[YOLO Click] Card info not found in DB for id: {}", card_id);
                        log::debug!("[YOLO Click] Checking if DB is loaded...");
                        let items_db = db_state.items.read().unwrap();
                        log::debug!(
                            "[YOLO Click] DB has {} items, id_map has {} entries",
                            items_db.list.len(),
                            items_db.id_map.len()
                        );
                    } else {
                        log::debug!("[YOLO Click] Card match returned empty list");
                    }
                }
                Ok(None) => log::debug!(
                    "[YOLO Click] No card descriptors matched in {} category",
                    card_size
                ),
                Err(e) => log::debug!("[YOLO Click] Card matching error: {}", e),
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

async fn get_item_info_internal(state: &DbState, id: String) -> Option<ItemData> {
    let db = state.items.read().unwrap();
    if let Some(&idx) = db.id_map.get(&id) {
        return Some(db.list[idx].clone());
    }
    None
}
