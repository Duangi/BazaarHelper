use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;

use tauri::{Manager, State};

use crate::{DbState, ItemData, SyncPayload};

#[derive(Debug, serde::Deserialize)]
pub struct SearchQuery {
    pub keyword: Option<String>,
    pub item_type: Option<String>,
    pub size: Option<String>,
    pub start_tier: Option<String>,
    pub hero: Option<String>,
    pub tags: Option<String>,
    pub hidden_tags: Option<String>,
}

#[tauri::command]
pub fn get_show_yolo_monitor() -> Result<bool, String> {
    Ok(false)
}

#[tauri::command]
pub fn search_items(query: SearchQuery, state: State<'_, DbState>) -> Result<Vec<ItemData>, String> {
    let mut results = Vec::new();
    let keyword = query.keyword.as_deref().map(|s| s.to_lowercase());
    let size_filter = query.size.as_deref().map(|s| s.to_lowercase());
    let tier_filter = query.start_tier.as_deref().map(|s| s.to_lowercase());
    let hero_filter = query.hero.as_deref().map(|s| s.to_lowercase());
    let tags_filter = query.tags.as_deref().map(|s| s.to_lowercase());
    let htags_filter = query.hidden_tags.as_deref().map(|s| s.to_lowercase());

    let match_item = |item: &ItemData| -> bool {
        if let Some(ref k) = keyword {
            if !item.name_cn.to_lowercase().contains(k) && !item.name.to_lowercase().contains(k) {
                return false;
            }
        }
        if let Some(ref s) = size_filter {
            if !item
                .size
                .as_ref()
                .map(|v| v.to_lowercase())
                .unwrap_or_default()
                .contains(s)
            {
                return false;
            }
        }
        if let Some(ref t) = tier_filter {
            if !item.tier.to_lowercase().contains(t) {
                return false;
            }
        }
        if let Some(ref h) = hero_filter {
            if !item.heroes.iter().any(|hero| hero.to_lowercase().contains(h)) {
                return false;
            }
        }
        if let Some(ref t) = tags_filter {
            if !item.tags.to_lowercase().contains(t) {
                return false;
            }
        }
        if let Some(ref h) = htags_filter {
            if !item.hidden_tags.to_lowercase().contains(h) {
                return false;
            }
        }
        true
    };

    let search_type = query.item_type.as_deref().unwrap_or("all");

    if search_type == "all" || search_type == "item" {
        if let Ok(db) = state.items.read() {
            for item in &db.list {
                if match_item(item) {
                    results.push(item.clone());
                }
            }
        }
    }

    if search_type == "all" || search_type == "skill" {
        if let Ok(db) = state.skills.read() {
            for item in &db.list {
                if match_item(item) {
                    results.push(item.clone());
                }
            }
        }
    }

    results.sort_by(|a, b| {
        let tier_rank = |t: &str| match t.split('/').next().unwrap_or("").trim() {
            "Bronze" | "Common" => 1,
            "Silver" => 2,
            "Gold" => 3,
            "Diamond" => 4,
            "Legendary" => 5,
            _ => 10,
        };
        let ta = tier_rank(&a.tier);
        let tb = tier_rank(&b.tier);
        if ta != tb {
            ta.cmp(&tb)
        } else {
            a.name_cn.cmp(&b.name_cn)
        }
    });

    Ok(results)
}

#[tauri::command]
pub fn get_all_monsters(state: State<'_, DbState>) -> Result<serde_json::Map<String, serde_json::Value>, String> {
    crate::log_to_file("get_all_monsters called");
    let db = state.monsters.read().map_err(|_| "DB Busy")?;
    let count = db.len();
    crate::log_to_file(&format!("Monsters DB contains {} entries", count));

    if count > 0 {
        let sample_names: Vec<String> = db.keys().take(5).cloned().collect();
        crate::log_to_file(&format!("Sample monster names: {:?}", sample_names));
    } else {
        crate::log_to_file("Warning: Monsters DB is empty!");
    }

    Ok(db.clone())
}

#[tauri::command]
pub fn debug_monsters_db(state: State<'_, DbState>) -> Result<String, String> {
    let db = state.monsters.read().map_err(|_| "DB Busy")?;
    let count = db.len();
    let mut result = format!("Monsters DB Status:\n- Total entries: {}\n", count);

    if count > 0 {
        let sample: Vec<String> = db.keys().take(10).cloned().collect();
        result.push_str(&format!("- Sample entries: {:?}\n", sample));

        let day1_monsters: Vec<String> = db
            .iter()
            .filter(|(_, data)| data.get("available").and_then(|v| v.as_str()) == Some("Day 1"))
            .map(|(name, _)| name.clone())
            .take(5)
            .collect();
        result.push_str(&format!("- Day 1 monsters: {:?}\n", day1_monsters));
    } else {
        result.push_str("- Database is empty!\n");
    }

    crate::log_to_file(&result);
    Ok(result)
}

#[tauri::command]
pub fn debug_resource_paths(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    let resources_path = app.path().resource_dir().map_err(|e| e.to_string())?;
    let mut report = serde_json::Map::new();
    report.insert(
        "resource_dir".to_string(),
        serde_json::Value::String(resources_path.to_string_lossy().to_string()),
    );

    let files = [
        "monsters_db.json",
        "monsters_export.json",
        "images_monster_map.json",
        "items_db.json",
        "skills_db.json",
    ];

    let mut files_obj = serde_json::Map::new();
    for f in &files {
        let p1 = resources_path.join("resources").join(f);
        let p2 = resources_path.join(f);
        let mut info = serde_json::Map::new();
        info.insert("path1".to_string(), serde_json::Value::String(p1.to_string_lossy().to_string()));
        info.insert("exists1".to_string(), serde_json::Value::Bool(p1.exists()));
        if p1.exists() {
            if let Ok(md) = std::fs::metadata(&p1) {
                info.insert("size1".to_string(), serde_json::Value::Number(serde_json::Number::from(md.len())));
            }
        }
        info.insert("path2".to_string(), serde_json::Value::String(p2.to_string_lossy().to_string()));
        info.insert("exists2".to_string(), serde_json::Value::Bool(p2.exists()));
        if p2.exists() {
            if let Ok(md) = std::fs::metadata(&p2) {
                info.insert("size2".to_string(), serde_json::Value::Number(serde_json::Number::from(md.len())));
            }
        }
        files_obj.insert(f.to_string(), serde_json::Value::Object(info));
    }

    report.insert("files".to_string(), serde_json::Value::Object(files_obj));
    Ok(serde_json::Value::Object(report))
}

#[tauri::command]
pub fn get_current_day(hours_per_day: Option<u32>, retro: Option<bool>) -> Result<u32, String> {
    let cached = crate::load_state();
    if cached.day > 0 {
        return Ok(cached.day);
    }

    let _hours = hours_per_day.unwrap_or(6);
    let retro = retro.unwrap_or(false);
    let log_path = crate::data_management::log_paths::get_log_path();

    if log_path.exists() {
        let mut file = File::open(&log_path).map_err(|e| e.to_string())?;
        let metadata = file.metadata().map_err(|e| e.to_string())?;
        let file_size = metadata.len();

        let read_size = file_size.min(5_000_000) as usize;
        let mut buffer = vec![0u8; read_size];
        file.seek(SeekFrom::End(-(read_size as i64))).map_err(|e| e.to_string())?;
        file.read_exact(&mut buffer).map_err(|e| e.to_string())?;

        let content = String::from_utf8_lossy(&buffer);
        if let Some(day) = crate::data_management::day_calc::calculate_day_from_log(&content, retro) {
            return Ok(day);
        }
    }

    Ok(1)
}

#[tauri::command]
pub fn update_day(day: u32) -> Result<(), String> {
    let mut state = crate::load_state();
    state.day = day;
    crate::save_state(&state);
    log::debug!("[State] Manually updated Day to: {}", day);
    Ok(())
}

#[tauri::command]
pub async fn get_item_info(state: State<'_, DbState>, id: String) -> Result<Option<ItemData>, String> {
    let db = state.items.read().unwrap();
    if let Some(&idx) = db.id_map.get(&id) {
        return Ok(Some(db.list[idx].clone()));
    }

    let sdb = state.skills.read().unwrap();
    if let Some(&idx) = sdb.id_map.get(&id) {
        return Ok(Some(sdb.list[idx].clone()));
    }

    Ok(None)
}

#[tauri::command]
pub async fn get_sync_state(state: State<'_, DbState>) -> Result<SyncPayload, String> {
    let p_state = crate::load_state();
    let items_db = state.items.read().map_err(|e| e.to_string())?;
    let skills_db = state.skills.read().map_err(|e| e.to_string())?;

    let map_items = |ids: &std::collections::HashSet<String>| -> Vec<ItemData> {
        let mut ordered_ids: Vec<&String> = ids.iter().collect();
        ordered_ids.sort();
        ordered_ids
            .into_iter()
            .filter_map(|iid| {
                let tid = p_state.inst_to_temp.get(iid)?;
                let mut item = crate::data_management::item_lookup::lookup_item(tid, &items_db, &skills_db)?;
                item.instance_id = Some(iid.clone());
                Some(item)
            })
            .collect()
    };

    let hand_items = map_items(&p_state.current_hand);
    let stash_items = map_items(&p_state.current_stash);
    let all_tags = items_db.unique_tags.clone();

    Ok(SyncPayload {
        hand_items,
        stash_items,
        all_tags,
    })
}

#[derive(Debug, serde::Serialize)]
pub struct RuntimeLogSnapshot {
    pub debug_mode: bool,
    pub log_dir: String,
    pub files: Vec<String>,
    pub lines: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct FileCheckItem {
    pub key: String,
    pub path: String,
    pub exists: bool,
    pub size_bytes: Option<u64>,
    pub required: bool,
}

#[derive(Debug, serde::Serialize)]
pub struct FileCheckReport {
    pub all_ok: bool,
    pub missing_count: usize,
    pub checked_files: usize,
    pub items: Vec<FileCheckItem>,
}

#[tauri::command]
pub fn get_runtime_logs(line_limit: Option<usize>) -> Result<RuntimeLogSnapshot, String> {
    let limit = line_limit.unwrap_or(300).clamp(50, 2000);
    let lines = crate::logs::read_recent_log_lines(limit)?;
    let files = crate::logs::list_log_files()?
        .into_iter()
        .map(|p: std::path::PathBuf| p.to_string_lossy().to_string())
        .collect::<Vec<_>>();
    let state = crate::load_state();

    Ok(RuntimeLogSnapshot {
        debug_mode: state.debug_mode,
        log_dir: crate::logs::log_dir_path().to_string_lossy().to_string(),
        files,
        lines,
    })
}

#[tauri::command]
pub fn set_debug_mode(enabled: bool) -> Result<bool, String> {
    let mut state = crate::load_state();
    state.debug_mode = enabled;
    crate::save_state(&state);
    crate::logs::set_debug_mode(enabled)?;
    log::info!("[DebugMode] {}", if enabled { "enabled" } else { "disabled" });
    Ok(enabled)
}

#[tauri::command]
pub fn get_debug_mode() -> Result<bool, String> {
    Ok(crate::load_state().debug_mode)
}

fn first_existing(paths: &[PathBuf]) -> PathBuf {
    for p in paths {
        if p.exists() {
            return p.clone();
        }
    }
    paths.first().cloned().unwrap_or_default()
}

#[tauri::command]
pub fn check_required_files(app: tauri::AppHandle) -> Result<FileCheckReport, String> {
    let resources_dir = app.path().resource_dir().map_err(|e| e.to_string())?;
    let app_cache_dir = app.path().app_cache_dir().ok();

    let mut items: Vec<FileCheckItem> = Vec::new();
    let mut push_item = |key: &str, required: bool, candidates: Vec<PathBuf>| {
        let chosen = first_existing(&candidates);
        let exists = chosen.exists();
        let size_bytes = if exists {
            std::fs::metadata(&chosen).ok().map(|m| m.len())
        } else {
            None
        };
        items.push(FileCheckItem {
            key: key.to_string(),
            path: chosen.to_string_lossy().to_string(),
            exists,
            size_bytes,
            required,
        });
    };

    for file in crate::data_management::resource_paths::RESOURCE_DB_FILES {
        push_item(
            &format!("json:{file}"),
            true,
            vec![resources_dir.join(file), resources_dir.join("resources").join(file)],
        );
    }

    push_item(
        "model:best.onnx",
        true,
        vec![
            resources_dir.join("models").join("best.onnx"),
            resources_dir.join("resources").join("models").join("best.onnx"),
        ],
    );

    let mut monster_cache_candidates = Vec::new();
    if let Some(cache_dir) = &app_cache_dir {
        monster_cache_candidates.push(cache_dir.join("monster_features_opencv_v2.bin"));
    }
    monster_cache_candidates.push(resources_dir.join("monster_features_opencv_v2.bin"));
    monster_cache_candidates.push(
        resources_dir
            .join("resources")
            .join("monster_features_opencv_v2.bin"),
    );
    push_item("cache:monster_features_opencv_v2.bin", false, monster_cache_candidates);
    for cache_name in [
        "card_features_small.bin",
        "card_features_medium.bin",
        "card_features_large.bin",
    ] {
        let mut cache_candidates = Vec::new();
        if let Some(cache_dir) = &app_cache_dir {
            cache_candidates.push(cache_dir.join(cache_name));
        }
        cache_candidates.push(resources_dir.join(cache_name));
        cache_candidates.push(resources_dir.join("resources").join(cache_name));
        push_item(
            &format!("cache:{cache_name}"),
            false,
            cache_candidates,
        );
    }

    let missing_required = items.iter().filter(|i| i.required && !i.exists).count();
    Ok(FileCheckReport {
        all_ok: missing_required == 0,
        missing_count: missing_required,
        checked_files: items.len(),
        items,
    })
}
