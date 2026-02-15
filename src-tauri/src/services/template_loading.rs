use tauri::{Emitter, Manager, State};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::{monster_recognition, DbState, ItemData, RawItem};

static TEMPLATE_LOADING_STARTED: AtomicBool = AtomicBool::new(false);

#[tauri::command]
pub async fn start_template_loading(app: tauri::AppHandle) -> Result<(), String> {
    if TEMPLATE_LOADING_STARTED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        log::debug!("[TemplateLoading] start_template_loading called again, skipping.");
        return Ok(());
    }

    log::debug!("=============== [start_template_loading] CALLED ===============");
    let resources_path = app.path().resource_dir().map_err(|e| {
        let err = format!("Failed to get resource dir in template loading: {}", e);
        crate::log_to_file(&err);
        err
    })?;
    let res_dir = resources_path.join("resources");
    let cache_dir = crate::user_data::state_cache_path()
        .parent()
        .ok_or_else(|| {
            let err = "Failed to get cache parent dir".to_string();
            crate::log_to_file(&err);
            err
        })?
        .to_path_buf();

    let items_db_path = res_dir.join("items_db.json");
    log::debug!("[ItemDB] Attempting to load from: {:?}", items_db_path);
    if items_db_path.exists() {
        match std::fs::read_to_string(&items_db_path) {
            Ok(json_str) => {
                log::debug!("[ItemDB] File read successfully, length: {} bytes", json_str.len());
                match serde_json::from_str::<Vec<RawItem>>(&json_str) {
                    Ok(raw_items_list) => {
                        log::debug!("[ItemDB] Parsed {} raw items from JSON", raw_items_list.len());
                        let items_list: Vec<ItemData> = raw_items_list.into_iter().map(ItemData::from).collect();
                        log::debug!("[ItemDB] Converted to {} ItemData entries", items_list.len());

                        let db_state: State<'_, DbState> = app.state();
                        let mut items_db = db_state.items.write().unwrap();

                        let mut small_count = 0;
                        let mut medium_count = 0;
                        let mut large_count = 0;

                        items_db.list = items_list.clone();
                        items_db.id_map.clear();
                        for (idx, item) in items_list.iter().enumerate() {
                            items_db.id_map.insert(item.uuid.clone(), idx);

                            if let Some(size) = &item.size {
                                if size.contains("Small") || size.contains("小型") {
                                    small_count += 1;
                                } else if size.contains("Medium") || size.contains("中型") {
                                    medium_count += 1;
                                } else if size.contains("Large") || size.contains("大型") {
                                    large_count += 1;
                                }
                            }
                        }

                        log::debug!(
                            "[ItemDB] Loaded {} items: Small={}, Medium={}, Large={}",
                            items_db.list.len(),
                            small_count,
                            medium_count,
                            large_count
                        );
                        log::debug!("[ItemDB] id_map has {} entries", items_db.id_map.len());
                    }
                    Err(e) => {
                        log::debug!("[ItemDB] Failed to parse items_db.json: {}", e);
                        crate::log_to_file(&format!("ItemDB parse error: {}", e));
                    }
                }
            }
            Err(e) => {
                log::debug!("[ItemDB] Failed to read items_db.json: {}", e);
                crate::log_to_file(&format!("ItemDB read error: {}", e));
            }
        }
    } else {
        log::debug!("[ItemDB] items_db.json not found at: {:?}", items_db_path);
        crate::log_to_file(&format!("ItemDB not found: {:?}", items_db_path));
    }

    let res_dir_async = res_dir.clone();
    let cache_dir_async = cache_dir.clone();
    let app_async = app.clone();
    tauri::async_runtime::spawn(async move {
        let res_dir_clone = res_dir_async.clone();
        let cache_dir_clone = cache_dir_async.clone();

        let res_dir_clone2 = res_dir_async.clone();
        let cache_dir_clone2 = cache_dir_async.clone();

        let _ = monster_recognition::preload_templates_async(res_dir_async, cache_dir_async).await;
        let _ = monster_recognition::load_event_templates(app_async).await;
        let _ =
            monster_recognition::preload_card_templates_by_size_async(res_dir_clone, cache_dir_clone)
                .await;
        let _ = monster_recognition::preload_card_templates_async(res_dir_clone2, cache_dir_clone2)
            .await;
    });

    {
        let db_state: State<'_, DbState> = app.state();
        let items_db = db_state.items.read().unwrap();
        log::debug!(
            "[ItemDB] Verification: {} items loaded, {} in id_map",
            items_db.list.len(),
            items_db.id_map.len()
        );
    }

    let skills_db_path = res_dir.join("skills_db.json");
    if skills_db_path.exists() {
        match std::fs::read_to_string(&skills_db_path) {
            Ok(json_str) => match serde_json::from_str::<Vec<RawItem>>(&json_str) {
                Ok(raw_list) => {
                    let list: Vec<ItemData> = raw_list.into_iter().map(ItemData::from).collect();
                    let db_state: State<'_, DbState> = app.state();
                    let mut skills_db = db_state.skills.write().unwrap();

                    skills_db.list = list.clone();
                    skills_db.id_map.clear();
                    for (idx, item) in list.iter().enumerate() {
                        skills_db.id_map.insert(item.uuid.clone(), idx);
                    }
                    log::debug!("[SkillDB] Loaded {} skills", skills_db.list.len());
                }
                Err(e) => log::debug!("[SkillDB] Parse error: {}", e),
            },
            Err(e) => log::debug!("[SkillDB] Read error: {}", e),
        }
    }

    let monsters_db_path = res_dir.join("monsters_db.json");
    if monsters_db_path.exists() {
        match std::fs::read_to_string(&monsters_db_path) {
            Ok(json_str) => {
                match serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&json_str) {
                    Ok(map) => {
                        let db_state: State<'_, DbState> = app.state();
                        let mut monsters_db = db_state.monsters.write().unwrap();
                        *monsters_db = map;
                        let count = monsters_db.len();
                        log::debug!("[MonsterDB] Loaded {} monsters", count);
                        let _ = app.emit("monsters-db-ready", serde_json::json!({ "count": count }));
                    }
                    Err(e) => log::debug!("[MonsterDB] Parse error: {}", e),
                }
            }
            Err(e) => log::debug!("[MonsterDB] Read error: {}", e),
        }
    }

    Ok(())
}
