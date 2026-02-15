use std::sync::{OnceLock, RwLock};

use crate::monster_recognition::YoloDetection;

static YOLO_SCAN_RESULTS: OnceLock<RwLock<Vec<YoloDetection>>> = OnceLock::new();
static YOLO_SCAN_IMAGE: OnceLock<RwLock<Option<image::DynamicImage>>> = OnceLock::new();
static YOLO_WINDOW_OFFSET: OnceLock<RwLock<(i32, i32)>> = OnceLock::new();

pub fn get_yolo_scan_results() -> &'static RwLock<Vec<YoloDetection>> {
    YOLO_SCAN_RESULTS.get_or_init(|| RwLock::new(Vec::new()))
}

pub fn get_yolo_scan_image() -> &'static RwLock<Option<image::DynamicImage>> {
    YOLO_SCAN_IMAGE.get_or_init(|| RwLock::new(None))
}

pub fn get_yolo_window_offset() -> &'static RwLock<(i32, i32)> {
    YOLO_WINDOW_OFFSET.get_or_init(|| RwLock::new((0, 0)))
}

pub fn clear_cache() {
    {
        let mut results = get_yolo_scan_results().write().unwrap();
        results.clear();
    }
    {
        let mut saved_img = get_yolo_scan_image().write().unwrap();
        *saved_img = None;
    }
}

pub fn collect_stats() -> serde_json::Value {
    let detections = get_yolo_scan_results().read().unwrap();
    let total = detections.len();
    let items = detections.iter().filter(|d| d.class_id == 2).count();
    let events = detections.iter().filter(|d| d.class_id == 1).count();
    let skills = detections.iter().filter(|d| d.class_id == 6).count();
    let monster_icons = detections.iter().filter(|d| d.class_id == 3).count();

    let events_list: Vec<_> = detections.iter().filter(|d| d.class_id == 1).collect();
    let monsters_count = events_list
        .iter()
        .map(|event| {
            detections.iter().filter(|d| d.class_id == 3).any(|icon| {
                let ix1 = event.x1.max(icon.x1);
                let iy1 = event.y1.max(icon.y1);
                let ix2 = event.x2.min(icon.x2);
                let iy2 = event.y2.min(icon.y2);
                let i_area = (ix2 - ix1).max(0) * (iy2 - iy1).max(0);
                let icon_area = (icon.x2 - icon.x1) * (icon.y2 - icon.y1);
                icon_area > 0 && (i_area as f32 / icon_area as f32) > 0.5
            })
        })
        .filter(|&has_monster| has_monster)
        .count();

    serde_json::json!({
        "total": total,
        "items": items,
        "events": events,
        "monsters": monsters_count,
        "skills": skills,
        "monster_icons": monster_icons
    })
}
