use std::sync::RwLock;
use std::sync::atomic::AtomicBool;


pub mod logs {
    include!("logs/mod.rs");
}
pub mod core;
pub mod data_management;
pub mod platforms;
pub mod services;
pub mod user_data;

pub mod monster_recognition;
use crate::monster_recognition::YoloDetection;

pub(crate) static ABORT_YOLO: AtomicBool = AtomicBool::new(false);

pub(crate) fn get_yolo_scan_results() -> &'static RwLock<Vec<YoloDetection>> {
    services::yolo_state::get_yolo_scan_results()
}

pub(crate) fn get_yolo_scan_image() -> &'static RwLock<Option<image::DynamicImage>> {
    services::yolo_state::get_yolo_scan_image()
}

pub(crate) fn get_yolo_window_offset() -> &'static RwLock<(i32, i32)> {
    services::yolo_state::get_yolo_window_offset()
}

// --- Logger Helper ---
pub fn log_to_file(msg: &str) {
    logs::log_compat(msg);
}

pub fn set_panic_hook() {
    logs::install_panic_hook();
}

pub fn log_system_info(app_handle: &tauri::AppHandle) {
    let lp = data_management::log_paths::get_log_path();
    logs::log_system_info(app_handle, &lp);
}

// --- Data Models ---
pub use core::models::{
    DbState, ItemData, ItemDb, MonsterCalibration, MonsterData, MonsterRegion, MonsterSubItem,
    PersistentState, RawItem, RawSkill, SkillDb, SkillText, SyncPayload, TierInfo,
};

#[allow(dead_code)]
fn construct_monster_sub_item(
    item_data: Option<ItemData>,
    fallback_name_cn: &str,
    fallback_name_en: &str,
    current_tier: &str,
    override_size: Option<&str>,
) -> serde_json::Value {
    data_management::monster_sub_item::construct_monster_sub_item(
        item_data,
        fallback_name_cn,
        fallback_name_en,
        current_tier,
        override_size,
    )
}

// #[tauri::command]
// #[allow(dead_code)]
// async fn clear_monster_cache() -> Result<(), String> {
//     let cache_dir = get_cache_path().parent().unwrap().to_path_buf();
//     let cache_file = cache_dir.join("monster_features.bin");
//     if cache_file.exists() {
//         std::fs::remove_file(cache_file).map_err(|e| e.to_string())?;
//     }
//     Ok(())
// }
pub(crate) fn save_state(state: &PersistentState) {
    user_data::state_store::save_state(state);
}

pub(crate) fn load_state() -> PersistentState {
    user_data::state_store::load_state()
}


// --- App Run ---

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    core::bootstrap::run();
}
