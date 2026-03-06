use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::fs::File;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use chrono::Local;
use image::GenericImageView;
use regex::Regex;
use sysinfo::{Pid, ProcessesToUpdate, System};
use tauri::{Emitter, Manager, State};

use crate::{DbState, ItemData, SyncPayload};

#[derive(Debug, Clone, serde::Serialize)]
pub struct GameIdentityInfo {
    pub username: String,
    pub account_id: String,
    pub steam_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct GeneratedLoginKey {
    pub key: String,
    pub username: String,
    pub account_id: String,
}

fn bazaar_log_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    #[cfg(target_os = "windows")]
    {
        let home = std::env::var("USERPROFILE").unwrap_or_default();
        if !home.is_empty() {
            let root = PathBuf::from(home)
                .join("AppData")
                .join("LocalLow")
                .join("Tempo Storm")
                .join("The Bazaar");
            paths.push(root.join("Player.log"));
            paths.push(root.join("Player-prev.log"));
        }
    }
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME").unwrap_or_default();
        if !home.is_empty() {
            let root = PathBuf::from(home)
                .join("Library")
                .join("Logs")
                .join("Tempo Storm")
                .join("The Bazaar");
            paths.push(root.join("Player.log"));
            paths.push(root.join("Player-prev.log"));
        }
    }
    paths
}

fn latest_game_identity_from_logs() -> Option<GameIdentityInfo> {
    let re_profile = Regex::new(r"Username:\s*(?P<username>.+?)\s*-\s*AccountId:\s*(?P<account>[0-9a-fA-F-]{8,})").ok()?;
    let re_socket = Regex::new(r"AccountId:\s*\[(?P<account>[0-9a-fA-F-]{8,})\]").ok()?;
    let re_steam = Regex::new(r"ID:\s*(?P<steam>\d{8,})").ok()?;

    let mut username: Option<String> = None;
    let mut account_id: Option<String> = None;
    let mut steam_id: Option<String> = None;

    for path in bazaar_log_paths() {
        if !path.exists() {
            continue;
        }
        let Ok(file) = File::open(&path) else {
            continue;
        };
        let reader = BufReader::new(file);
        for line in reader.lines().map_while(Result::ok) {
            if let Some(cap) = re_profile.captures(&line) {
                if let Some(v) = cap.name("username") {
                    let s = v.as_str().trim();
                    if !s.is_empty() {
                        username = Some(s.to_string());
                    }
                }
                if let Some(v) = cap.name("account") {
                    let s = v.as_str().trim();
                    if !s.is_empty() {
                        account_id = Some(s.to_string());
                    }
                }
            }
            if let Some(cap) = re_socket.captures(&line) {
                if let Some(v) = cap.name("account") {
                    let s = v.as_str().trim();
                    if !s.is_empty() {
                        account_id = Some(s.to_string());
                    }
                }
            }
            if let Some(cap) = re_steam.captures(&line) {
                if let Some(v) = cap.name("steam") {
                    let s = v.as_str().trim();
                    if !s.is_empty() {
                        steam_id = Some(s.to_string());
                    }
                }
            }
        }
    }

    let account = account_id?;
    let user = username.unwrap_or_else(|| "Unknown".to_string());
    Some(GameIdentityInfo {
        username: user,
        account_id: account,
        steam_id,
    })
}

fn to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{:02x}", b));
    }
    out
}

fn obfuscate_login_payload(raw: &str) -> String {
    let secret = b"BazaarHelper@LoginKey:v1";
    let mut buf = Vec::with_capacity(raw.len());
    for (idx, byte) in raw.as_bytes().iter().enumerate() {
        let mask = secret[idx % secret.len()] ^ ((idx as u8).wrapping_mul(31));
        buf.push(byte ^ mask);
    }
    to_hex(&buf)
}

fn checksum16(input: &str) -> String {
    let mut hasher = DefaultHasher::new();
    ("bh.login.v1", input).hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

#[tauri::command]
pub fn get_game_identity() -> Result<GameIdentityInfo, String> {
    latest_game_identity_from_logs().ok_or_else(|| "无法在日志中读取到账号信息，请先进入游戏主界面再试".to_string())
}

#[tauri::command]
pub fn generate_game_login_key() -> Result<GeneratedLoginKey, String> {
    let identity = latest_game_identity_from_logs()
        .ok_or_else(|| "无法在日志中读取到账号信息，请先进入游戏主界面再试".to_string())?;

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let payload = format!(
        "v1|{}|{}|{}",
        identity.username.trim(),
        identity.account_id.trim(),
        ts
    );
    let encrypted = obfuscate_login_payload(&payload);
    let sign = checksum16(&encrypted);
    let key = format!("bh1.{}.{}", encrypted, sign);

    Ok(GeneratedLoginKey {
        key,
        username: identity.username,
        account_id: identity.account_id,
    })
}

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

#[derive(Debug, Clone, serde::Serialize)]
pub struct SearchItemLite {
    pub uuid: String,
    pub name: String,
    pub name_cn: String,
    pub tier: String,
    pub available_tiers: String,
    pub size: Option<String>,
    pub tags: String,
    pub hidden_tags: String,
    pub processed_tags: Vec<String>,
    pub heroes: Vec<String>,
}

fn parse_hero_list(raw: Option<&str>) -> Vec<String> {
    raw.unwrap_or_default()
        .split('|')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn to_search_item_lite(item: &ItemData) -> SearchItemLite {
    SearchItemLite {
        uuid: item.uuid.clone(),
        name: item.name.clone(),
        name_cn: item.name_cn.clone(),
        tier: item.tier.clone(),
        available_tiers: item.available_tiers.clone(),
        size: item.size.clone(),
        tags: item.tags.clone(),
        hidden_tags: item.hidden_tags.clone(),
        processed_tags: item.processed_tags.clone(),
        heroes: parse_hero_list(item.heroes.as_deref()),
    }
}

#[tauri::command]
pub fn get_show_yolo_monitor() -> Result<bool, String> {
    Ok(false)
}

#[tauri::command]
pub fn search_items_light(query: SearchQuery, state: State<'_, DbState>) -> Result<Vec<SearchItemLite>, String> {
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
            let hero_text = item.heroes.as_deref().unwrap_or_default().to_lowercase();
            if !hero_text.contains(h) {
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
                    results.push(to_search_item_lite(item));
                }
            }
        }
    }

    if search_type == "all" || search_type == "skill" {
        if let Ok(db) = state.skills.read() {
            for item in &db.list {
                if match_item(item) {
                    results.push(to_search_item_lite(item));
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
    let history_day = crate::user_data::infer_current_day_from_history()
        .ok()
        .flatten()
        .unwrap_or(0);

    let _hours = hours_per_day.unwrap_or(6);
    let retro = retro.unwrap_or(false);
    let log_path = crate::data_management::log_paths::get_log_path();
    let mut log_day = 0u32;

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
            log_day = day.max(0);
        }
    }

    let resolved = log_day.max(history_day).max(cached.day).max(1);
    if resolved != cached.day {
        let mut state = cached.clone();
        state.day = resolved;
        crate::save_state(&state);
    }

    Ok(resolved)
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
    let lookup_id = id.trim();
    if lookup_id.is_empty() {
        return Ok(None);
    }

    {
        let db = state.items.read().unwrap();
        if let Some(&idx) = db.id_map.get(lookup_id) {
            return Ok(Some(db.list[idx].clone()));
        }
        if let Some(item) = db.list.iter().find(|item| {
            item.uuid.eq_ignore_ascii_case(lookup_id)
                || item.name.eq_ignore_ascii_case(lookup_id)
                || item.name_cn.eq_ignore_ascii_case(lookup_id)
        }) {
            return Ok(Some(item.clone()));
        }
    }

    {
        let sdb = state.skills.read().unwrap();
        if let Some(&idx) = sdb.id_map.get(lookup_id) {
            return Ok(Some(sdb.list[idx].clone()));
        }
        if let Some(item) = sdb.list.iter().find(|item| {
            item.uuid.eq_ignore_ascii_case(lookup_id)
                || item.name.eq_ignore_ascii_case(lookup_id)
                || item.name_cn.eq_ignore_ascii_case(lookup_id)
        }) {
            return Ok(Some(item.clone()));
        }
    }

    Ok(None)
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ResourceCatalogMaps {
    pub skills_art_map: HashMap<String, String>,
    pub item_sizes: HashMap<String, String>,
}

static RESOURCE_CATALOG_CACHE: OnceLock<Mutex<Option<ResourceCatalogMaps>>> = OnceLock::new();

fn load_resource_catalog_maps(app: &tauri::AppHandle) -> Result<ResourceCatalogMaps, String> {
    let resources_dir = app.path().resource_dir().map_err(|e| e.to_string())?;

    let skills_path = first_existing(&[
        resources_dir.join("skills_db.json"),
        resources_dir.join("resources").join("skills_db.json"),
    ]);
    let items_path = first_existing(&[
        resources_dir.join("items_db.json"),
        resources_dir.join("resources").join("items_db.json"),
    ]);

    let mut skills_art_map: HashMap<String, String> = HashMap::new();
    if skills_path.exists() {
        let text = std::fs::read_to_string(&skills_path).map_err(|e| e.to_string())?;
        let value: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        if let Some(list) = value.as_array() {
            for entry in list {
                let id = entry.get("id").and_then(|v| v.as_str()).unwrap_or_default();
                if id.is_empty() {
                    continue;
                }
                if let Some(art_key) = entry.get("art_key").and_then(|v| v.as_str()) {
                    let basename = art_key
                        .rsplit('/')
                        .next()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| art_key.to_string());
                    skills_art_map.insert(id.to_string(), basename);
                }
            }
        }
    }

    let mut item_sizes: HashMap<String, String> = HashMap::new();
    if items_path.exists() {
        let text = std::fs::read_to_string(&items_path).map_err(|e| e.to_string())?;
        let value: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        if let Some(list) = value.as_array() {
            for entry in list {
                let id = entry.get("id").and_then(|v| v.as_str()).unwrap_or_default();
                if id.is_empty() {
                    continue;
                }
                if let Some(size) = entry.get("size").and_then(|v| v.as_str()) {
                    item_sizes.insert(id.to_string(), size.to_string());
                }
            }
        }
    }

    Ok(ResourceCatalogMaps {
        skills_art_map,
        item_sizes,
    })
}

#[tauri::command]
pub fn get_resource_catalog_maps(
    app: tauri::AppHandle,
    force_reload: Option<bool>,
) -> Result<ResourceCatalogMaps, String> {
    let force_reload = force_reload.unwrap_or(false);
    let cache = RESOURCE_CATALOG_CACHE.get_or_init(|| Mutex::new(None));

    if !force_reload {
        if let Ok(guard) = cache.lock() {
            if let Some(cached) = guard.as_ref() {
                return Ok(cached.clone());
            }
        }
    }

    let loaded = load_resource_catalog_maps(&app)?;
    if let Ok(mut guard) = cache.lock() {
        *guard = Some(loaded.clone());
    }
    Ok(loaded)
}

#[tauri::command]
pub fn get_search_thumbnail_path(
    app: tauri::AppHandle,
    resource_path: String,
) -> Result<String, String> {
    resolve_search_thumbnail_path(&app, &resource_path)
}

fn resolve_search_thumbnail_path(app: &tauri::AppHandle, resource_path: &str) -> Result<String, String> {
    let normalized = resource_path
        .trim()
        .replace('\\', "/")
        .trim_start_matches('/')
        .trim_start_matches("resources/")
        .to_string();
    if normalized.is_empty() || normalized.contains("..") {
        return Err("invalid resource path".to_string());
    }

    let resources_dir = app.path().resource_dir().map_err(|e| e.to_string())?;
    let src = first_existing(&[
        resources_dir.join(&normalized),
        resources_dir.join("resources").join(&normalized),
    ]);
    if !src.exists() {
        return Err(format!("resource not found: {}", normalized));
    }

    let cache_root = app
        .path()
        .app_cache_dir()
        .or_else(|_| app.path().app_local_data_dir())
        .map_err(|e| e.to_string())?;
    let thumbs_dir = cache_root.join("search_thumbs");
    std::fs::create_dir_all(&thumbs_dir).map_err(|e| e.to_string())?;

    let mut hasher = DefaultHasher::new();
    normalized.hash(&mut hasher);
    let key = format!("{:016x}.png", hasher.finish());
    let thumb_path = thumbs_dir.join(key);

    let thumb_fresh = if thumb_path.exists() {
        let src_m = std::fs::metadata(&src).ok().and_then(|m| m.modified().ok());
        let dst_m = std::fs::metadata(&thumb_path).ok().and_then(|m| m.modified().ok());
        match (src_m, dst_m) {
            (Some(s), Some(d)) => d >= s,
            _ => true,
        }
    } else {
        false
    };

    if !thumb_fresh {
        let img = image::open(&src).map_err(|e| e.to_string())?;
        let thumb = img.thumbnail(128, 128);
        thumb.save(&thumb_path).map_err(|e| e.to_string())?;
    }

    Ok(thumb_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn get_search_thumbnail_paths(
    app: tauri::AppHandle,
    resource_paths: Vec<String>,
) -> Result<HashMap<String, String>, String> {
    let mut result = HashMap::new();

    for path in resource_paths {
        if path.trim().is_empty() {
            continue;
        }
        if let Ok(resolved) = resolve_search_thumbnail_path(&app, &path) {
            result.insert(path, resolved);
        }
    }

    Ok(result)
}

#[derive(Debug, serde::Serialize)]
pub struct ImageCacheCleanupReport {
    pub removed_dirs: usize,
    pub removed_files: usize,
    pub removed_bytes: u64,
    pub scanned_targets: Vec<String>,
}

fn collect_dir_file_stats(path: &std::path::Path) -> (usize, u64) {
    let mut files = 0_usize;
    let mut bytes = 0_u64;
    if !path.exists() {
        return (0, 0);
    }
    let mut stack = vec![path.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if let Ok(meta) = entry.metadata() {
                    if meta.is_dir() {
                        stack.push(p);
                    } else if meta.is_file() {
                        files += 1;
                        bytes = bytes.saturating_add(meta.len());
                    }
                }
            }
        }
    }
    (files, bytes)
}

#[tauri::command]
pub fn clear_generated_image_caches(app: tauri::AppHandle) -> Result<ImageCacheCleanupReport, String> {
    let mut targets: Vec<PathBuf> = Vec::new();

    if let Ok(dir) = app.path().app_cache_dir() {
        targets.push(dir.join("search_thumbs"));
    }
    if let Ok(dir) = app.path().app_local_data_dir() {
        targets.push(dir.join("search_thumbs"));
    }

    targets.push(crate::user_data::app_data_root().join("debug_images"));
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            targets.push(PathBuf::from(appdata).join("BazaarHelper").join("debug_images"));
        }
    }
    #[cfg(debug_assertions)]
    {
        targets.push(PathBuf::from("target").join("debug").join("monster_debug"));
    }

    let mut unique: Vec<PathBuf> = Vec::new();
    for t in targets {
        if !unique.iter().any(|x| x == &t) {
            unique.push(t);
        }
    }

    let mut removed_dirs = 0_usize;
    let mut removed_files = 0_usize;
    let mut removed_bytes = 0_u64;
    let mut scanned_targets = Vec::new();

    for dir in unique {
        scanned_targets.push(dir.to_string_lossy().to_string());
        if !dir.exists() {
            continue;
        }
        let (files, bytes) = collect_dir_file_stats(&dir);
        if std::fs::remove_dir_all(&dir).is_ok() {
            removed_dirs += 1;
            removed_files += files;
            removed_bytes = removed_bytes.saturating_add(bytes);
        }
    }

    Ok(ImageCacheCleanupReport {
        removed_dirs,
        removed_files,
        removed_bytes,
        scanned_targets,
    })
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

#[derive(Debug, serde::Serialize)]
pub struct MemoryModuleStat {
    pub key: String,
    pub label: String,
    pub estimated_bytes: u64,
    pub items: usize,
    pub note: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct MemoryBreakdownSnapshot {
    pub process_rss_bytes: Option<u64>,
    pub process_rss_mb: Option<f64>,
    pub estimated_total_bytes: u64,
    pub estimated_total_mb: f64,
    pub real_process: RealProcessMemorySnapshot,
    pub modules: Vec<MemoryModuleStat>,
}

#[derive(Debug, serde::Serialize, Clone)]
pub struct RealProcessMemoryRow {
    pub pid: u32,
    pub ppid: Option<u32>,
    pub name: String,
    pub role: String,
    pub rss_bytes: u64,
}

#[derive(Debug, serde::Serialize, Clone)]
pub struct RealProcessMemorySnapshot {
    pub self_pid: u32,
    pub app_tree_rss_bytes: u64,
    pub app_tree_processes: usize,
    pub engine_tree_rss_bytes: u64,
    pub engine_global_rss_bytes: u64,
    pub engine_extra_rss_bytes: u64,
    pub rows: Vec<RealProcessMemoryRow>,
    pub note: Option<String>,
}

fn estimate_skill_text_bytes(skill: &crate::core::models::SkillText) -> u64 {
    std::mem::size_of::<crate::core::models::SkillText>() as u64
        + skill.en.capacity() as u64
        + skill.cn.capacity() as u64
}

fn estimate_json_value_bytes(value: &serde_json::Value) -> u64 {
    match value {
        serde_json::Value::Null => 0,
        serde_json::Value::Bool(_) => std::mem::size_of::<bool>() as u64,
        serde_json::Value::Number(_) => 16,
        serde_json::Value::String(s) => std::mem::size_of::<String>() as u64 + s.capacity() as u64,
        serde_json::Value::Array(arr) => {
            std::mem::size_of::<Vec<serde_json::Value>>() as u64
                + (arr.capacity() as u64 * std::mem::size_of::<serde_json::Value>() as u64)
                + arr.iter().map(estimate_json_value_bytes).sum::<u64>()
        }
        serde_json::Value::Object(obj) => {
            std::mem::size_of::<serde_json::Map<String, serde_json::Value>>() as u64
                + obj
                    .iter()
                    .map(|(k, v)| {
                        std::mem::size_of::<String>() as u64
                            + k.capacity() as u64
                            + estimate_json_value_bytes(v)
                    })
                    .sum::<u64>()
        }
    }
}

fn estimate_item_data_bytes(item: &ItemData) -> u64 {
    let mut bytes = std::mem::size_of::<ItemData>() as u64;
    bytes += item.uuid.capacity() as u64;
    bytes += item.name.capacity() as u64;
    bytes += item.name_cn.capacity() as u64;
    bytes += item.tier.capacity() as u64;
    bytes += item.available_tiers.capacity() as u64;
    bytes += item.tags.capacity() as u64;
    bytes += item.hidden_tags.capacity() as u64;
    bytes += item.cooldown_tiers.capacity() as u64;
    bytes += item.damage_tiers.capacity() as u64;
    bytes += item.heal_tiers.capacity() as u64;
    bytes += item.shield_tiers.capacity() as u64;
    bytes += item.ammo_tiers.capacity() as u64;
    bytes += item.crit_tiers.capacity() as u64;
    bytes += item.multicast_tiers.capacity() as u64;
    bytes += item.burn_tiers.capacity() as u64;
    bytes += item.poison_tiers.capacity() as u64;
    bytes += item.regen_tiers.capacity() as u64;
    bytes += item.lifesteal_tiers.capacity() as u64;
    bytes += item.description.capacity() as u64;
    if let Some(v) = &item.size {
        bytes += v.capacity() as u64;
    }
    if let Some(v) = &item.heroes {
        bytes += v.capacity() as u64;
    }
    if let Some(v) = &item.instance_id {
        bytes += v.capacity() as u64;
    }
    if let Some(v) = &item.description_cn {
        bytes += v.capacity() as u64;
    }
    if let Some(v) = &item.image {
        bytes += v.capacity() as u64;
    }
    bytes += std::mem::size_of::<Vec<String>>() as u64
        + item
            .processed_tags
            .iter()
            .map(|s| std::mem::size_of::<String>() as u64 + s.capacity() as u64)
            .sum::<u64>();
    bytes += std::mem::size_of::<Vec<String>>() as u64
        + item
            .enchantments
            .iter()
            .map(|s| std::mem::size_of::<String>() as u64 + s.capacity() as u64)
            .sum::<u64>();
    bytes += std::mem::size_of::<Vec<crate::core::models::SkillText>>() as u64
        + item.skills.iter().map(estimate_skill_text_bytes).sum::<u64>();
    if let Some(skills) = &item.skills_passive {
        bytes += std::mem::size_of::<Vec<crate::core::models::SkillText>>() as u64
            + skills.iter().map(estimate_skill_text_bytes).sum::<u64>();
    }
    if let Some(q) = &item.quests {
        bytes += estimate_json_value_bytes(q);
    }
    bytes
}

fn estimate_string_set_bytes(set: &std::collections::HashSet<String>) -> u64 {
    std::mem::size_of::<std::collections::HashSet<String>>() as u64
        + set
            .iter()
            .map(|s| std::mem::size_of::<String>() as u64 + s.capacity() as u64)
            .sum::<u64>()
}

fn estimate_string_map_bytes(map: &std::collections::HashMap<String, String>) -> u64 {
    std::mem::size_of::<std::collections::HashMap<String, String>>() as u64
        + map
            .iter()
            .map(|(k, v)| {
                std::mem::size_of::<String>() as u64
                    + k.capacity() as u64
                    + std::mem::size_of::<String>() as u64
                    + v.capacity() as u64
            })
            .sum::<u64>()
}

#[cfg(target_family = "unix")]
fn read_process_rss_bytes() -> Option<u64> {
    let pid = std::process::id().to_string();
    let output = std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &pid])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    let kb = text.trim().parse::<u64>().ok()?;
    Some(kb * 1024)
}

#[cfg(not(target_family = "unix"))]
fn read_process_rss_bytes() -> Option<u64> {
    None
}

fn memory_module_label(key: &str) -> &'static str {
    match key {
        "cache:monster_templates" => "怪物模板缓存",
        "cache:card_templates" => "卡牌模板缓存(总)",
        "cache:card_templates_small" => "卡牌模板缓存(小)",
        "cache:card_templates_medium" => "卡牌模板缓存(中)",
        "cache:card_templates_large" => "卡牌模板缓存(大)",
        "cache:event_templates" => "事件模板缓存",
        "cache:yolo_session" => "YOLO 会话",
        _ => "模块",
    }
}

fn is_engine_process(name: &str, cmd: &str) -> bool {
    let n = name.to_ascii_lowercase();
    let c = cmd.to_ascii_lowercase();
    n.contains("webkit.webcontent")
        || c.contains("webkit.webcontent")
        || n.contains("msedgewebview2")
        || c.contains("msedgewebview2")
        || n.contains("webview2")
        || c.contains("webview2")
}

fn collect_descendant_pids(system: &System, root: Pid) -> HashSet<Pid> {
    let mut all = HashSet::new();
    all.insert(root);

    loop {
        let mut changed = false;
        for (pid, process) in system.processes() {
            if let Some(parent) = process.parent() {
                if all.contains(&parent) && !all.contains(pid) {
                    all.insert(*pid);
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    all
}

fn collect_real_process_memory(process_rss_bytes: Option<u64>) -> RealProcessMemorySnapshot {
    let self_pid_u32 = std::process::id();
    let self_pid = Pid::from_u32(self_pid_u32);
    let mut system = System::new_all();
    system.refresh_processes(ProcessesToUpdate::All, true);

    let mut rows: Vec<RealProcessMemoryRow> = Vec::new();
    let mut app_tree_rss = 0_u64;
    let mut engine_tree_rss = 0_u64;
    let mut engine_global_rss = 0_u64;

    let mut multiplier = 1_u64;
    if let (Some(main_proc), Some(ps_main_rss)) = (system.process(self_pid), process_rss_bytes) {
        let raw = main_proc.memory();
        if raw > 0 && ps_main_rss / raw > 8 {
            multiplier = 1024;
        }
    }

    let app_pids = collect_descendant_pids(&system, self_pid);
    let mut extra_engine_rows: Vec<RealProcessMemoryRow> = Vec::new();

    for (pid, process) in system.processes() {
        let pid_u32 = pid.as_u32();
        let ppid = process.parent().map(|p| p.as_u32());
        let name = process.name().to_string_lossy().to_string();
        let cmd = process
            .cmd()
            .iter()
            .map(|v| v.to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join(" ");
        let rss = process.memory().saturating_mul(multiplier);
        let in_tree = app_pids.contains(pid);
        let engine = is_engine_process(&name, &cmd);

        if in_tree {
            app_tree_rss = app_tree_rss.saturating_add(rss);
            if engine {
                engine_tree_rss = engine_tree_rss.saturating_add(rss);
            }
            rows.push(RealProcessMemoryRow {
                pid: pid_u32,
                ppid,
                name,
                role: if *pid == self_pid {
                    "main".to_string()
                } else if engine {
                    "engine(tree)".to_string()
                } else {
                    "child".to_string()
                },
                rss_bytes: rss,
            });
        } else if engine {
            engine_global_rss = engine_global_rss.saturating_add(rss);
            extra_engine_rows.push(RealProcessMemoryRow {
                pid: pid_u32,
                ppid,
                name,
                role: "engine(global)".to_string(),
                rss_bytes: rss,
            });
        }
    }

    extra_engine_rows.sort_by(|a, b| b.rss_bytes.cmp(&a.rss_bytes));
    extra_engine_rows.truncate(10);
    rows.extend(extra_engine_rows);
    rows.sort_by(|a, b| b.rss_bytes.cmp(&a.rss_bytes));

    let engine_extra_rss = engine_global_rss.saturating_sub(engine_tree_rss);
    let note = Some(
        "app_tree 是当前进程及其子进程；engine(global) 是系统级 WebView 引擎进程，可能包含其他应用。".to_string(),
    );

    RealProcessMemorySnapshot {
        self_pid: self_pid_u32,
        app_tree_rss_bytes: app_tree_rss,
        app_tree_processes: app_pids.len(),
        engine_tree_rss_bytes: engine_tree_rss,
        engine_global_rss_bytes: engine_global_rss,
        engine_extra_rss_bytes: engine_extra_rss,
        rows,
        note,
    }
}

#[tauri::command]
pub fn get_memory_breakdown(state: State<'_, DbState>) -> Result<MemoryBreakdownSnapshot, String> {
    let mut modules: Vec<MemoryModuleStat> = Vec::new();

    if let Ok(items_db) = state.items.read() {
        let list_bytes = std::mem::size_of::<Vec<ItemData>>() as u64
            + (items_db.list.capacity() as u64 * std::mem::size_of::<ItemData>() as u64)
            + items_db.list.iter().map(estimate_item_data_bytes).sum::<u64>();
        let id_map_bytes = std::mem::size_of::<std::collections::HashMap<String, usize>>() as u64
            + items_db
                .id_map
                .iter()
                .map(|(k, _)| std::mem::size_of::<String>() as u64 + k.capacity() as u64 + 16)
                .sum::<u64>();
        let tags_bytes = std::mem::size_of::<Vec<String>>() as u64
            + items_db
                .unique_tags
                .iter()
                .map(|t| std::mem::size_of::<String>() as u64 + t.capacity() as u64)
                .sum::<u64>();
        modules.push(MemoryModuleStat {
            key: "db:items".to_string(),
            label: "物品数据库".to_string(),
            estimated_bytes: list_bytes + id_map_bytes + tags_bytes,
            items: items_db.list.len(),
            note: None,
        });
    }

    if let Ok(skills_db) = state.skills.read() {
        let list_bytes = std::mem::size_of::<Vec<ItemData>>() as u64
            + (skills_db.list.capacity() as u64 * std::mem::size_of::<ItemData>() as u64)
            + skills_db.list.iter().map(estimate_item_data_bytes).sum::<u64>();
        let id_map_bytes = std::mem::size_of::<std::collections::HashMap<String, usize>>() as u64
            + skills_db
                .id_map
                .iter()
                .map(|(k, _)| std::mem::size_of::<String>() as u64 + k.capacity() as u64 + 16)
                .sum::<u64>();
        modules.push(MemoryModuleStat {
            key: "db:skills".to_string(),
            label: "技能数据库".to_string(),
            estimated_bytes: list_bytes + id_map_bytes,
            items: skills_db.list.len(),
            note: None,
        });
    }

    if let Ok(monsters_db) = state.monsters.read() {
        let monsters_bytes = std::mem::size_of::<serde_json::Map<String, serde_json::Value>>() as u64
            + monsters_db
                .iter()
                .map(|(k, v)| {
                    std::mem::size_of::<String>() as u64 + k.capacity() as u64 + estimate_json_value_bytes(v)
                })
                .sum::<u64>();
        modules.push(MemoryModuleStat {
            key: "db:monsters".to_string(),
            label: "野怪数据库".to_string(),
            estimated_bytes: monsters_bytes,
            items: monsters_db.len(),
            note: None,
        });
    }

    let p_state = crate::load_state();
    modules.push(MemoryModuleStat {
        key: "state:inst_to_temp".to_string(),
        label: "日志实例映射".to_string(),
        estimated_bytes: estimate_string_map_bytes(&p_state.inst_to_temp),
        items: p_state.inst_to_temp.len(),
        note: None,
    });
    let hand_bytes = estimate_string_set_bytes(&p_state.current_hand);
    let stash_bytes = estimate_string_set_bytes(&p_state.current_stash);
    modules.push(MemoryModuleStat {
        key: "state:inventory_sets".to_string(),
        label: "手牌/仓库集合".to_string(),
        estimated_bytes: hand_bytes + stash_bytes,
        items: p_state.current_hand.len() + p_state.current_stash.len(),
        note: None,
    });

    if let Ok(detections) = crate::get_yolo_scan_results().read() {
        let bytes = std::mem::size_of::<Vec<crate::monster_recognition::YoloDetection>>() as u64
            + (detections.capacity() as u64
                * std::mem::size_of::<crate::monster_recognition::YoloDetection>() as u64);
        modules.push(MemoryModuleStat {
            key: "yolo:detections".to_string(),
            label: "YOLO检测结果缓存".to_string(),
            estimated_bytes: bytes,
            items: detections.len(),
            note: None,
        });
    }

    if let Ok(image_opt) = crate::get_yolo_scan_image().read() {
        let (bytes, note) = if let Some(encoded) = image_opt.as_ref() {
            let dims = image::load_from_memory(encoded).ok().map(|img| img.dimensions());
            let note = if let Some((w, h)) = dims {
                Some(format!("jpeg:{} bytes, {}x{}", encoded.len(), w, h))
            } else {
                Some(format!("jpeg:{} bytes, decode_failed", encoded.len()))
            };
            (
                encoded.len() as u64,
                note,
            )
        } else {
            (0, Some("empty".to_string()))
        };
        modules.push(MemoryModuleStat {
            key: "yolo:image".to_string(),
            label: "YOLO截图缓存".to_string(),
            estimated_bytes: bytes,
            items: if image_opt.is_some() { 1 } else { 0 },
            note,
        });
    }

    for cache_stat in crate::monster_recognition::collect_recognition_cache_memory_stats() {
        modules.push(MemoryModuleStat {
            key: cache_stat.key.clone(),
            label: memory_module_label(&cache_stat.key).to_string(),
            estimated_bytes: cache_stat.estimated_bytes,
            items: cache_stat.count,
            note: cache_stat.note.clone(),
        });
    }

    modules.sort_by(|a, b| b.estimated_bytes.cmp(&a.estimated_bytes));
    let estimated_total_bytes = modules.iter().map(|m| m.estimated_bytes).sum::<u64>();
    let process_rss_bytes = read_process_rss_bytes();
    let real_process = collect_real_process_memory(process_rss_bytes);

    Ok(MemoryBreakdownSnapshot {
        process_rss_bytes,
        process_rss_mb: process_rss_bytes.map(|b| b as f64 / 1024.0 / 1024.0),
        estimated_total_bytes,
        estimated_total_mb: estimated_total_bytes as f64 / 1024.0 / 1024.0,
        real_process,
        modules,
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

#[tauri::command]
pub fn set_game_log_monitor_enabled(enabled: bool) -> Result<bool, String> {
    crate::core::log_monitor_state::set_enabled(enabled);
    let mut state = crate::load_state();
    state.enable_game_log_monitor = enabled;
    crate::save_state(&state);
    log::info!(
        "[LogMonitor] runtime switch set to {}",
        if enabled { "enabled" } else { "disabled" }
    );
    Ok(enabled)
}

#[tauri::command]
pub fn get_game_log_monitor_enabled() -> Result<bool, String> {
    Ok(crate::core::log_monitor_state::is_enabled())
}

#[tauri::command]
pub fn set_game_log_monitor_runtime(enabled: bool) -> Result<bool, String> {
    crate::core::log_monitor_state::set_enabled(enabled);
    log::info!(
        "[LogMonitor] runtime transient switch set to {}",
        if enabled { "enabled" } else { "disabled" }
    );
    Ok(enabled)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct HistoryBattleLineupCard {
    instance_id: String,
    template_id: String,
    name_cn: String,
    name_en: String,
    image: Option<String>,
    #[serde(default)]
    size: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct HistoryBattleRecord {
    day: u32,
    start_time: String,
    victory: bool,
    duration: Option<f64>,
    screenshot: Option<String>,
    #[serde(default)]
    lineup_cards: Vec<HistoryBattleLineupCard>,
    #[serde(default)]
    enemy_lineup_cards: Vec<HistoryBattleLineupCard>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct HistoryMatchRecord {
    match_id: String,
    hero: Option<String>,
    start_time: String,
    end_time: Option<String>,
    game_date: Option<String>,
    days: u32,
    victory: bool,
    is_finished: bool,
    pvp_battles: Vec<HistoryBattleRecord>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
struct HistoryRoot {
    #[serde(default)]
    matches: Vec<HistoryMatchRecord>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ManualBattleLineupAnalyzeRequest {
    #[serde(default)]
    match_id: Option<String>,
    match_start_time: String,
    battle_day: u32,
    battle_start_time: String,
    victory: bool,
    duration: Option<f64>,
    screenshot_path: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ManualBattleLineupAnalyzeResult {
    self_count: usize,
    enemy_count: usize,
    screenshot_path: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ManualBattleScreenshotRequest {
    match_start_time: String,
    battle_day: u32,
    battle_start_time: String,
    victory: bool,
    duration: Option<f64>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ManualBattleScreenshotResult {
    screenshot_path: String,
}

const HISTORY_SCAN_MAX_BYTES: u64 = 6_000_000;
const MAX_HISTORY_MATCHES: usize = 120;
const MAX_PVP_BATTLES_PER_MATCH: usize = 40;

#[derive(Debug, Clone, PartialEq, Eq)]
struct LogFileStamp {
    path: String,
    exists: bool,
    size: u64,
    modified_ms: u128,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LogSignature {
    entries: Vec<LogFileStamp>,
}

static LAST_HISTORY_LOG_SIGNATURE: OnceLock<Mutex<Option<LogSignature>>> = OnceLock::new();

fn parse_socket_index_from_target(target: &str) -> Option<u32> {
    if !(target.contains("PlayerSocket") || target.contains("Hand")) {
        return None;
    }
    let digits_rev: String = target
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    if digits_rev.is_empty() {
        return None;
    }
    digits_rev.chars().rev().collect::<String>().parse::<u32>().ok()
}

fn remove_hand_slot_mapping(
    iid: &str,
    hand_slot_to_iid: &mut BTreeMap<u32, String>,
    iid_to_hand_slot: &mut HashMap<String, u32>,
) {
    if let Some(slot) = iid_to_hand_slot.remove(iid) {
        if hand_slot_to_iid.get(&slot).map(|v| v == iid).unwrap_or(false) {
            hand_slot_to_iid.remove(&slot);
        }
    }
}

fn set_hand_slot_mapping(
    iid: &str,
    slot: u32,
    hand_slot_to_iid: &mut BTreeMap<u32, String>,
    iid_to_hand_slot: &mut HashMap<String, u32>,
) {
    if let Some(prev_slot) = iid_to_hand_slot.insert(iid.to_string(), slot) {
        if prev_slot != slot && hand_slot_to_iid.get(&prev_slot).map(|v| v == iid).unwrap_or(false) {
            hand_slot_to_iid.remove(&prev_slot);
        }
    }
    hand_slot_to_iid.insert(slot, iid.to_string());
}

fn build_history_log_signature(paths: &[PathBuf]) -> LogSignature {
    let mut entries: Vec<LogFileStamp> = paths
        .iter()
        .map(|path| {
            let path_str = path.to_string_lossy().to_string();
            if let Ok(md) = std::fs::metadata(path) {
                let modified_ms = md
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_millis())
                    .unwrap_or(0);
                LogFileStamp {
                    path: path_str,
                    exists: true,
                    size: md.len(),
                    modified_ms,
                }
            } else {
                LogFileStamp {
                    path: path_str,
                    exists: false,
                    size: 0,
                    modified_ms: 0,
                }
            }
        })
        .collect();

    entries.sort_by(|a, b| a.path.cmp(&b.path));
    LogSignature { entries }
}

fn extract_log_timestamp(line: &str) -> Option<String> {
    if !line.starts_with('[') {
        return None;
    }
    let end = line.find(']')?;
    if end <= 1 {
        return None;
    }
    Some(line[1..end].to_string())
}

fn build_history_match_id(start_time: &str) -> String {
    let mut hasher = DefaultHasher::new();
    start_time.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn history_today_date() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

fn history_screenshot_root_dir() -> PathBuf {
    crate::user_data::battle_screenshots_dir()
}

fn history_time_token(raw: &str) -> String {
    let digits: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        "000000000".to_string()
    } else {
        digits
    }
}

#[allow(dead_code)]
fn canonical_screenshot_path_for_battle(record: &HistoryMatchRecord, battle_day: u32) -> PathBuf {
    let root = history_screenshot_root_dir();
    let match_id = if record.match_id.trim().is_empty() {
        build_history_match_id(record.start_time.trim())
    } else {
        record.match_id.trim().to_string()
    };
    let start_token = history_time_token(&record.start_time);
    root.join(format!("start{}_{}", start_token, match_id))
        .join(format!("pvp_day{:02}.png", battle_day.max(1)))
}

fn parse_hms_token_to_millis(token: &str) -> Option<i64> {
    let digits: String = token.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() < 6 {
        return None;
    }
    let h = digits.get(0..2)?.parse::<i64>().ok()?;
    let m = digits.get(2..4)?.parse::<i64>().ok()?;
    let s = digits.get(4..6)?.parse::<i64>().ok()?;
    if !(0..24).contains(&h) || !(0..60).contains(&m) || !(0..60).contains(&s) {
        return None;
    }
    let ms = if digits.len() >= 9 {
        digits.get(6..9).and_then(|v| v.parse::<i64>().ok()).unwrap_or(0)
    } else {
        0
    };
    Some(((h * 3600 + m * 60 + s) * 1000) + ms)
}

fn parse_start_folder_token_millis(folder_name: &str) -> Option<i64> {
    let rest = folder_name.strip_prefix("start")?;
    let token = rest.split('_').next().unwrap_or_default();
    parse_hms_token_to_millis(token)
}

fn migrate_legacy_split_screenshots(
    root: &std::path::Path,
    canonical_folder_name: &str,
    canonical_folder: &std::path::Path,
    canonical_start_token: &str,
) {
    let Some(canonical_ms) = parse_hms_token_to_millis(canonical_start_token) else {
        return;
    };

    let mut candidates: Vec<(i64, PathBuf, String)> = Vec::new();
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name == canonical_folder_name {
            continue;
        }
        let Some(folder_ms) = parse_start_folder_token_millis(&name) else {
            continue;
        };
        let diff = (folder_ms - canonical_ms).abs();
        // Same run split by timestamp drift is usually within a few seconds.
        if diff <= 30_000 {
            candidates.push((diff, path, name));
        }
    }

    candidates.sort_by_key(|(diff, _, _)| *diff);
    let Some((_, legacy_dir, legacy_name)) = candidates.first().cloned() else {
        return;
    };

    let mut moved = 0usize;
    let mut ensured_canonical_dir = false;
    let Ok(files) = std::fs::read_dir(&legacy_dir) else {
        return;
    };
    for file_entry in files.flatten() {
        let src = file_entry.path();
        if !src.is_file() {
            continue;
        }
        let Some(file_name) = src.file_name().map(|v| v.to_string_lossy().to_string()) else {
            continue;
        };
        if !file_name.starts_with("pvp_day") {
            continue;
        }
        let ext_ok = src
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("png"))
            .unwrap_or(false);
        if !ext_ok {
            continue;
        }
        let dest = canonical_folder.join(&file_name);
        if dest.exists() {
            continue;
        }
        if !ensured_canonical_dir {
            let _ = std::fs::create_dir_all(canonical_folder);
            ensured_canonical_dir = true;
        }
        let moved_ok = if std::fs::rename(&src, &dest).is_ok() {
            true
        } else if std::fs::copy(&src, &dest).is_ok() {
            let _ = std::fs::remove_file(&src);
            true
        } else {
            false
        };
        if moved_ok {
            moved += 1;
        }
    }

    if moved > 0 {
        log::info!(
            "[History] Migrated {} screenshot(s) from split folder {} -> {}",
            moved,
            legacy_name,
            canonical_folder_name
        );
    }

    let is_empty = std::fs::read_dir(&legacy_dir)
        .ok()
        .map(|mut it| it.next().is_none())
        .unwrap_or(false);
    if is_empty {
        let _ = std::fs::remove_dir(&legacy_dir);
    }
}

fn remap_battle_screenshots_strict(root: &mut HistoryRoot) {
    for record in &mut root.matches {
        let match_id = if record.match_id.trim().is_empty() {
            build_history_match_id(record.start_time.trim())
        } else {
            record.match_id.trim().to_string()
        };
        let start_token = history_time_token(&record.start_time);
        let screenshot_root = history_screenshot_root_dir();
        let canonical_folder_name = format!("start{}_{}", start_token, match_id);
        let match_folder = screenshot_root.join(&canonical_folder_name);

        let has_missing = record
            .pvp_battles
            .iter()
            .any(|battle| !match_folder.join(format!("pvp_day{:02}.png", battle.day.max(1))).exists());
        if has_missing {
            migrate_legacy_split_screenshots(
                &screenshot_root,
                &canonical_folder_name,
                &match_folder,
                &start_token,
            );
        }

        for battle in &mut record.pvp_battles {
            let canonical = match_folder.join(format!("pvp_day{:02}.png", battle.day.max(1)));
            if canonical.exists() {
                battle.screenshot = Some(canonical.to_string_lossy().to_string());
            } else {
                battle.screenshot = None;
            }
        }
    }
}

#[allow(dead_code)]
fn parse_log_hms_seconds(raw: &str) -> Option<i32> {
    let time_part = raw
        .split_whitespace()
        .last()
        .unwrap_or(raw)
        .split('.')
        .next()
        .unwrap_or(raw);
    let mut parts = time_part.split(':');
    let h = parts.next()?.parse::<i32>().ok()?;
    let m = parts.next()?.parse::<i32>().ok()?;
    let s = parts.next()?.parse::<i32>().ok()?;
    Some(h * 3600 + m * 60 + s)
}

#[allow(dead_code)]
fn parse_capture_hms_seconds(path: &std::path::Path) -> Option<i32> {
    let stem = path.file_stem()?.to_string_lossy();
    let stem_ref = stem.as_ref();

    for token in stem_ref.split('_') {
        let candidate = token.strip_prefix("cap").unwrap_or(token);
        if candidate.len() < 6 || !candidate.chars().take(6).all(|c| c.is_ascii_digit()) {
            continue;
        }
        let h = candidate[0..2].parse::<i32>().ok()?;
        let m = candidate[2..4].parse::<i32>().ok()?;
        let s = candidate[4..6].parse::<i32>().ok()?;
        if (0..24).contains(&h) && (0..60).contains(&m) && (0..60).contains(&s) {
            return Some(h * 3600 + m * 60 + s);
        }
    }

    let token = stem_ref.strip_prefix("battle_")?.split('_').next()?;
    if token.len() >= 6 && token.chars().take(6).all(|c| c.is_ascii_digit()) {
        let h = token[0..2].parse::<i32>().ok()?;
        let m = token[2..4].parse::<i32>().ok()?;
        let s = token[4..6].parse::<i32>().ok()?;
        if (0..24).contains(&h) && (0..60).contains(&m) && (0..60).contains(&s) {
            return Some(h * 3600 + m * 60 + s);
        }
    }

    None
}

#[allow(dead_code)]
fn parse_capture_day(path: &std::path::Path) -> Option<u32> {
    for component in path.components() {
        let token = component.as_os_str().to_string_lossy();
        if let Some(day_token) = token.strip_prefix("day") {
            let day_digits: String = day_token.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(day) = day_digits.parse::<u32>() {
                if day > 0 {
                    return Some(day);
                }
            }
        }
    }

    let stem = path.file_stem()?.to_string_lossy();
    for token in stem.split('_') {
        if let Some(day_token) = token.strip_prefix("day") {
            let day_digits: String = day_token.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(day) = day_digits.parse::<u32>() {
                if day > 0 {
                    return Some(day);
                }
            }
        }
    }
    None
}

#[allow(dead_code)]
fn collect_screenshot_candidates_by_date() -> HashMap<String, Vec<PathBuf>> {
    let mut map = HashMap::new();
    let root = history_screenshot_root_dir();
    if !root.exists() {
        return map;
    }

    fn collect_recursive(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_recursive(&path, out);
                continue;
            }
            let is_image = path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| {
                    ext.eq_ignore_ascii_case("png")
                        || ext.eq_ignore_ascii_case("jpg")
                        || ext.eq_ignore_ascii_case("jpeg")
                })
                .unwrap_or(false);
            if is_image {
                out.push(path);
            }
        }
    }

    let mut all_images: Vec<PathBuf> = Vec::new();
    collect_recursive(&root, &mut all_images);
    all_images.sort();

    let Ok(date_dirs) = std::fs::read_dir(root) else {
        if !all_images.is_empty() {
            map.insert("*".to_string(), all_images);
        }
        return map;
    };

    for date_entry in date_dirs.flatten() {
        let date_path = date_entry.path();
        if !date_path.is_dir() {
            continue;
        }
        let Some(date_name) = date_path.file_name().map(|n| n.to_string_lossy().to_string()) else {
            continue;
        };

        let Ok(files) = std::fs::read_dir(&date_path) else {
            continue;
        };
        let mut shots: Vec<PathBuf> = files
            .flatten()
            .map(|entry| entry.path())
            .filter(|p| {
                p.extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext.eq_ignore_ascii_case("png") || ext.eq_ignore_ascii_case("jpg") || ext.eq_ignore_ascii_case("jpeg"))
                    .unwrap_or(false)
            })
            .collect();

        shots.sort();
        if !shots.is_empty() {
            map.insert(date_name, shots);
        }
    }

    if !all_images.is_empty() {
        map.entry("*".to_string())
            .or_insert_with(|| all_images);
    }

    map
}

#[allow(dead_code)]
fn backfill_missing_battle_screenshots(root: &mut HistoryRoot) {
    let candidates_by_date = collect_screenshot_candidates_by_date();
    let mut used_by_date: HashMap<String, HashSet<String>> = HashMap::new();
    let mut used_global: HashSet<String> = HashSet::new();

    let mut all_candidates: Vec<PathBuf> = candidates_by_date
        .values()
        .flat_map(|paths| paths.iter().cloned())
        .collect();
    all_candidates.sort();
    all_candidates.dedup();

    // first pass: reserve already-assigned screenshots
    for record in &root.matches {
        let Some(date) = record.game_date.clone() else {
            continue;
        };
        let used = used_by_date.entry(date).or_default();
        for battle in &record.pvp_battles {
            if let Some(path) = battle.screenshot.as_ref() {
                if !path.trim().is_empty() {
                    used.insert(path.clone());
                    used_global.insert(path.clone());
                }
            }
        }
    }

    for record in &mut root.matches {
        let date = record.game_date.clone().unwrap_or_default();
        let local_candidates: Vec<PathBuf> = date
            .is_empty()
            .then(Vec::new)
            .unwrap_or_else(|| candidates_by_date.get(&date).cloned().unwrap_or_default());

        let used = used_by_date.entry(date.clone()).or_default();

        for battle in &mut record.pvp_battles {
            if battle.screenshot.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false) {
                continue;
            }

            let battle_ts = parse_log_hms_seconds(&battle.start_time);
            let battle_day = battle.day.max(1);
            let mut best_idx: Option<usize> = None;
            let mut best_day_penalty = i32::MAX;
            let mut best_score = i32::MAX;

            for (idx, path) in local_candidates.iter().enumerate() {
                let path_str = path.to_string_lossy().to_string();
                if used.contains(&path_str) {
                    continue;
                }

                let day_penalty = if parse_capture_day(path) == Some(battle_day) { 0 } else { 1 };

                let score = match (battle_ts, parse_capture_hms_seconds(path)) {
                    (Some(bt), Some(st)) => (st - bt).abs(),
                    _ => 999_999,
                };

                if day_penalty < best_day_penalty || (day_penalty == best_day_penalty && score < best_score) {
                    best_day_penalty = day_penalty;
                    best_score = score;
                    best_idx = Some(idx);
                    if day_penalty == 0 && score <= 1 {
                        break;
                    }
                }
            }

            if let Some(idx) = best_idx {
                let path_str = local_candidates[idx].to_string_lossy().to_string();
                battle.screenshot = Some(path_str.clone());
                used.insert(path_str);
                used_global.insert(local_candidates[idx].to_string_lossy().to_string());
                continue;
            }

            // fallback: if date-based matching has no candidates (or all are used),
            // try all historical screenshots to handle date drift in log parsing
            let mut fallback_best: Option<String> = None;
            let mut fallback_day_penalty = i32::MAX;
            let mut fallback_score = i32::MAX;

            for path in &all_candidates {
                let path_str = path.to_string_lossy().to_string();
                if used_global.contains(&path_str) {
                    continue;
                }

                let day_penalty = if parse_capture_day(path) == Some(battle_day) { 0 } else { 1 };
                let score = match (battle_ts, parse_capture_hms_seconds(path)) {
                    (Some(bt), Some(st)) => (st - bt).abs(),
                    _ => 999_999,
                };

                if day_penalty < fallback_day_penalty || (day_penalty == fallback_day_penalty && score < fallback_score) {
                    fallback_day_penalty = day_penalty;
                    fallback_score = score;
                    fallback_best = Some(path_str.clone());
                    if day_penalty == 0 && score <= 1 {
                        break;
                    }
                }
            }

            if let Some(path_str) = fallback_best {
                battle.screenshot = Some(path_str.clone());
                used.insert(path_str.clone());
                used_global.insert(path_str);
            }
        }
    }
}

#[derive(Debug, Clone)]
struct HistoryCardMeta {
    name_cn: String,
    name_en: String,
    size: Option<String>,
}

fn load_history_card_meta_map() -> HashMap<String, HistoryCardMeta> {
    let mut map = HashMap::new();
    let resources_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources");
    for file_name in ["items_db.json", "skills_db.json"] {
        let path = resources_root.join(file_name);
        if !path.exists() {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
            continue;
        };
        let Some(list) = value.as_array() else {
            continue;
        };
        for item in list {
            let id = item.get("id").and_then(|v| v.as_str()).unwrap_or_default().trim();
            if id.is_empty() {
                continue;
            }
            let name_cn = item
                .get("name_cn")
                .and_then(|v| v.as_str())
                .unwrap_or(id)
                .to_string();
            let name_en = item
                .get("name_en")
                .and_then(|v| v.as_str())
                .unwrap_or(id)
                .to_string();
            let size = item.get("size").and_then(|v| v.as_str()).map(|s| s.to_string());
            map.entry(id.to_string()).or_insert(HistoryCardMeta { name_cn, name_en, size });
        }
    }
    map
}

fn history_build_lineup_from_visual(
    cards: &[crate::monster_recognition::VisualLineupCard],
    meta_map: &HashMap<String, HistoryCardMeta>,
) -> Vec<HistoryBattleLineupCard> {
    cards
        .iter()
        .map(|card| {
            let meta = meta_map.get(&card.template_id);
            HistoryBattleLineupCard {
                instance_id: String::new(),
                template_id: card.template_id.clone(),
                name_cn: meta
                    .map(|m| m.name_cn.clone())
                    .unwrap_or_else(|| card.template_id.clone()),
                name_en: meta
                    .map(|m| m.name_en.clone())
                    .unwrap_or_else(|| card.template_id.clone()),
                image: None,
                size: meta
                    .and_then(|m| m.size.clone())
                    .or_else(|| Some(card.size.clone())),
            }
        })
        .collect()
}

#[allow(dead_code)]
fn backfill_visual_lineups_from_screenshots(root: &mut HistoryRoot) {
    let model_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("resources")
        .join("models")
        .join("best.onnx");
    if !model_path.exists() {
        return;
    }

    let card_meta_map = load_history_card_meta_map();

    for record in &mut root.matches {
        for battle in &mut record.pvp_battles {
            let Some(screenshot_path) = battle.screenshot.as_ref().filter(|s| !s.trim().is_empty()) else {
                continue;
            };

            let visual = crate::monster_recognition::extract_lineups_from_battle_file(screenshot_path, &model_path, true)
                .or_else(|_| crate::monster_recognition::extract_lineups_from_battle_file(screenshot_path, &model_path, false));

            if let Ok((self_cards, enemy_cards)) = visual {
                if !self_cards.is_empty() {
                    battle.lineup_cards = history_build_lineup_from_visual(&self_cards, &card_meta_map);
                }
                if !enemy_cards.is_empty() {
                    battle.enemy_lineup_cards = history_build_lineup_from_visual(&enemy_cards, &card_meta_map);
                }
            }
        }
    }
}

fn history_match_sort_key(record: &HistoryMatchRecord) -> String {
    format!(
        "{} {}",
        record.game_date.clone().unwrap_or_default(),
        record.start_time
    )
}

fn history_record_key(record: &HistoryMatchRecord) -> String {
    if !record.match_id.is_empty() {
        return record.match_id.clone();
    }
    format!(
        "{}|{}",
        record.game_date.clone().unwrap_or_default(),
        record.start_time
    )
}

fn normalize_optional_text(value: &Option<String>) -> Option<String> {
    value.as_ref().and_then(|v| {
        let trimmed = v.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn normalize_conflicting_battle_days(battles: &mut Vec<HistoryBattleRecord>) {
    if battles.len() < 2 {
        return;
    }

    let mut seen_days: HashSet<u32> = HashSet::new();
    let mut last_day: u32 = 0;
    for battle in battles.iter_mut() {
        let mut day = battle.day.max(1);
        if day <= last_day || seen_days.contains(&day) {
            day = last_day.saturating_add(1).max(1);
            while seen_days.contains(&day) {
                day = day.saturating_add(1);
            }
        }
        battle.day = day;
        seen_days.insert(day);
        last_day = day;
    }
}

fn history_battle_identity_key(battle: &HistoryBattleRecord) -> String {
    let start = battle.start_time.trim();
    if !start.is_empty() {
        return format!("start:{}", start);
    }
    format!("legacy:{}:{}", battle.day.max(1), battle.victory)
}

fn merge_battle_record(base: &mut HistoryBattleRecord, incoming: &HistoryBattleRecord) {
    let incoming_day = incoming.day.max(1);
    let base_day = base.day.max(1);
    base.day = base_day.min(incoming_day).max(1);
    base.victory = incoming.victory;

    if base.duration.is_none() || incoming.duration.is_some() {
        base.duration = incoming.duration;
    }
    if incoming
        .screenshot
        .as_ref()
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false)
    {
        base.screenshot = incoming.screenshot.clone();
    }
    if !incoming.lineup_cards.is_empty() {
        base.lineup_cards = incoming.lineup_cards.clone();
    }
    if !incoming.enemy_lineup_cards.is_empty() {
        base.enemy_lineup_cards = incoming.enemy_lineup_cards.clone();
    }
}

fn dedupe_battles_by_start_time(battles: &mut Vec<HistoryBattleRecord>) {
    if battles.len() < 2 {
        return;
    }

    let mut merged: Vec<HistoryBattleRecord> = Vec::with_capacity(battles.len());
    let mut index_by_key: HashMap<String, usize> = HashMap::new();

    for battle in battles.drain(..) {
        let key = history_battle_identity_key(&battle);
        if let Some(&idx) = index_by_key.get(&key) {
            let existing = &mut merged[idx];
            merge_battle_record(existing, &battle);
        } else {
            index_by_key.insert(key, merged.len());
            merged.push(battle);
        }
    }

    *battles = merged;
}

fn merge_match_records(base: &mut HistoryMatchRecord, incoming: &HistoryMatchRecord) {
    if normalize_optional_text(&base.hero).is_none() && normalize_optional_text(&incoming.hero).is_some() {
        base.hero = normalize_optional_text(&incoming.hero);
    } else {
        base.hero = normalize_optional_text(&base.hero).or_else(|| normalize_optional_text(&incoming.hero));
    }

    if base.end_time.is_none() && incoming.end_time.is_some() {
        base.end_time = incoming.end_time.clone();
    }
    if incoming.game_date.is_some() && base.game_date.is_none() {
        base.game_date = incoming.game_date.clone();
    }

    base.days = base.days.max(incoming.days);
    if incoming.is_finished {
        base.victory = incoming.victory;
    } else {
        base.victory = base.victory || incoming.victory;
    }
    base.is_finished = base.is_finished || incoming.is_finished;

    dedupe_battles_by_start_time(&mut base.pvp_battles);

    let mut index_by_key: HashMap<String, usize> = HashMap::new();
    for (idx, b) in base.pvp_battles.iter().enumerate() {
        index_by_key.insert(history_battle_identity_key(b), idx);
    }

    for b in &incoming.pvp_battles {
        let key = history_battle_identity_key(b);
        if let Some(&existing_idx) = index_by_key.get(&key) {
            let existing = &mut base.pvp_battles[existing_idx];
            merge_battle_record(existing, b);
        } else {
            index_by_key.insert(key, base.pvp_battles.len());
            base.pvp_battles.push(b.clone());
        }
    }

    dedupe_battles_by_start_time(&mut base.pvp_battles);
    normalize_conflicting_battle_days(&mut base.pvp_battles);
    if base.pvp_battles.len() > MAX_PVP_BATTLES_PER_MATCH {
        let keep_from = base.pvp_battles.len() - MAX_PVP_BATTLES_PER_MATCH;
        base.pvp_battles.drain(0..keep_from);
    }
}

fn normalize_history(mut root: HistoryRoot) -> HistoryRoot {
    let mut merged_map: HashMap<String, HistoryMatchRecord> = HashMap::new();

    for mut record in root.matches.drain(..) {
        record.hero = normalize_optional_text(&record.hero);
        dedupe_battles_by_start_time(&mut record.pvp_battles);
        if record.days == 0 {
            record.days = 1;
        }
        if let Some(max_day) = record.pvp_battles.iter().map(|b| b.day).max() {
            record.days = record.days.max(max_day.saturating_add(1));
        }
        if record.pvp_battles.is_empty() {
            record.days = record.days.max(1);
        }

        normalize_conflicting_battle_days(&mut record.pvp_battles);
        if record.pvp_battles.len() > MAX_PVP_BATTLES_PER_MATCH {
            let keep_from = record.pvp_battles.len() - MAX_PVP_BATTLES_PER_MATCH;
            record.pvp_battles.drain(0..keep_from);
        }

        let key = history_record_key(&record);
        if let Some(existing) = merged_map.get_mut(&key) {
            merge_match_records(existing, &record);
        } else {
            merged_map.insert(key, record);
        }
    }

    let mut matches: Vec<HistoryMatchRecord> = merged_map.into_values().collect();
    matches.sort_by(|a, b| {
        match (a.is_finished, b.is_finished) {
            (false, true) => std::cmp::Ordering::Less,
            (true, false) => std::cmp::Ordering::Greater,
            _ => history_match_sort_key(b).cmp(&history_match_sort_key(a)),
        }
    });
    if matches.len() > MAX_HISTORY_MATCHES {
        matches.truncate(MAX_HISTORY_MATCHES);
    }

    HistoryRoot { matches }
}

fn history_detect_pvp_victory(recent_lines: &VecDeque<String>) -> bool {
    recent_lines
        .iter()
        .rev()
        .skip(1)
        .take(5)
        .any(|line| line.contains("All exit tasks completed"))
}

fn history_merge_restart_sessions(records: &mut Vec<HistoryMatchRecord>) {
    if records.len() < 2 {
        return;
    }

    let mut idx = 0;
    while idx + 1 < records.len() {
        if !records[idx].is_finished {
            let mut next = records.remove(idx + 1);
            let base_max_day = records[idx]
                .pvp_battles
                .iter()
                .map(|b| b.day.max(1))
                .max()
                .unwrap_or(0);
            let next_min_day = next
                .pvp_battles
                .iter()
                .map(|b| b.day.max(1))
                .min()
                .unwrap_or(1);
            if base_max_day > 0 && !next.pvp_battles.is_empty() {
                let offset = base_max_day.saturating_add(1).saturating_sub(next_min_day);
                if offset > 0 {
                    for battle in &mut next.pvp_battles {
                        battle.day = battle.day.max(1).saturating_add(offset);
                    }
                    next.days = next.days.saturating_add(offset);
                }
            }
            merge_match_records(&mut records[idx], &next);
        } else {
            idx += 1;
        }
    }
}

fn parse_history_from_logs(paths: &[PathBuf]) -> HistoryRoot {
    let re_hero = Regex::new(r"Hero: \[(?P<hero>[^\]]+)\]").unwrap();
    let re_state_change = Regex::new(r"State changed from \[.*?\] to \[(?P<state>[^\]]+)\]").unwrap();
    let re_combat_duration = Regex::new(r"Combat simulation completed in (?P<dur>[\d\.]+)s").unwrap();
    let re_purchase = Regex::new(r"Card Purchased: InstanceId:\s*(?P<iid>[^ ]+)\s*-\s*TemplateId\s*(?P<tid>[^ ]+)(?:.*Target:(?P<tgt>[^ ]+))?(?:.*Section(?P<sec>[^ ]+))?").unwrap();
    let re_id = Regex::new(r"ID: \[(?P<id>[^\]]+)\]").unwrap();
    let re_tid = Regex::new(r"TemplateId: \[(?P<tid>[^\]]+)\]").unwrap();
    let re_owner = Regex::new(r"- Owner: \[(?P<val>[^\]]+)\]").unwrap();
    let re_section = Regex::new(r"- Section: \[(?P<val>[^\]]+)\]").unwrap();
    let re_item_id = Regex::new(r"itm_[A-Za-z0-9_-]+").unwrap();
    let re_sold = Regex::new(r"Sold Card\s+(?P<iid>itm_[^ ]+)").unwrap();
    let re_removed = Regex::new(r"Successfully removed item\s+(?P<iid>itm_[^ ]+)").unwrap();
    let re_moved_to = Regex::new(r"Successfully moved card\s+(?P<iid>itm_[^ ]+)\s+to\s+(?P<tgt>[^ ]+)").unwrap();

    let mut records: Vec<HistoryMatchRecord> = Vec::new();
    let mut active_idx: Option<usize> = None;
    let mut in_pvp = false;
    let mut is_sync = false;
    let mut last_iid = String::new();
    let mut cur_owner = String::new();
    let mut inst_to_temp: HashMap<String, String> = HashMap::new();
    let mut current_hand: HashSet<String> = HashSet::new();
    let mut current_stash: HashSet<String> = HashSet::new();
    let mut hand_slot_to_iid: BTreeMap<u32, String> = BTreeMap::new();
    let mut iid_to_hand_slot: HashMap<String, u32> = HashMap::new();
    let mut last_pvp_start: Option<String> = None;
    let mut last_pvp_duration: Option<f64> = None;
    let mut recent_lines: VecDeque<String> = VecDeque::with_capacity(6);

    for path in paths {
        if !path.exists() {
            continue;
        }

        if let Ok(mut file) = File::open(path) {
            let start_offset = file
                .metadata()
                .ok()
                .map(|m| m.len().saturating_sub(HISTORY_SCAN_MAX_BYTES))
                .unwrap_or(0);
            let _ = file.seek(SeekFrom::Start(start_offset));
            let mut reader = BufReader::new(file);
            if start_offset > 0 {
                let mut discard = String::new();
                let _ = reader.read_line(&mut discard);
            }

            for (line_idx, line) in reader.lines().enumerate() {
                let Ok(raw) = line else {
                    continue;
                };
                let trimmed = raw.trim();
                let timestamp = extract_log_timestamp(trimmed);

                if recent_lines.len() >= 6 {
                    recent_lines.pop_front();
                }
                recent_lines.push_back(trimmed.to_string());

                if trimmed.contains("NetMessageRunInitialized")
                    || trimmed.contains("[GameInstance] Starting new run...")
                {
                    in_pvp = false;
                    is_sync = false;
                    last_iid.clear();
                    cur_owner.clear();
                    inst_to_temp.clear();
                    current_hand.clear();
                    current_stash.clear();
                    hand_slot_to_iid.clear();
                    iid_to_hand_slot.clear();
                    last_pvp_start = None;
                    last_pvp_duration = None;

                    let start_time = timestamp.clone().unwrap_or_else(|| {
                        format!(
                            "00:00:{:02}.{:03}",
                            line_idx % 60,
                            (line_idx * 17) % 1000
                        )
                    });
                    let match_id = build_history_match_id(&start_time);
                    records.push(HistoryMatchRecord {
                        match_id,
                        hero: None,
                        start_time,
                        end_time: None,
                        game_date: None,
                        days: 1,
                        victory: false,
                        is_finished: false,
                        pvp_battles: Vec::new(),
                    });
                    active_idx = records.len().checked_sub(1);
                    continue;
                }

                if let Some(cap) = re_purchase.captures(trimmed) {
                    let iid = cap["iid"].to_string();
                    let tid = cap["tid"].to_string();
                    inst_to_temp.insert(iid.clone(), tid);

                    let mut section = cap.name("sec").map(|s| s.as_str().to_string());
                    let target = cap.name("tgt").map(|t| t.as_str());
                    if section.as_deref().unwrap_or("").is_empty() {
                        if let Some(tgt) = target {
                            if tgt.contains("PlayerStorageSocket") {
                                section = Some("Stash".to_string());
                            } else if tgt.contains("PlayerSocket") {
                                section = Some("Player".to_string());
                            }
                        }
                    }
                    if let Some(sec) = section {
                        if sec == "Player" || sec == "Hand" {
                            current_hand.insert(iid.clone());
                            current_stash.remove(&iid);
                            if let Some(tgt) = target {
                                if let Some(slot) = parse_socket_index_from_target(tgt) {
                                    set_hand_slot_mapping(&iid, slot, &mut hand_slot_to_iid, &mut iid_to_hand_slot);
                                }
                            }
                        } else if sec == "Stash" || sec == "Storage" || sec == "PlayerStorage" {
                            current_stash.insert(iid.clone());
                            current_hand.remove(&iid);
                            remove_hand_slot_mapping(&iid, &mut hand_slot_to_iid, &mut iid_to_hand_slot);
                        }
                    }
                }

                if let Some(cap) = re_moved_to.captures(trimmed) {
                    let iid = cap["iid"].to_string();
                    let tgt = &cap["tgt"];
                    if tgt.contains("StorageSocket") {
                        current_stash.insert(iid.clone());
                        current_hand.remove(&iid);
                        remove_hand_slot_mapping(&iid, &mut hand_slot_to_iid, &mut iid_to_hand_slot);
                    } else if tgt.contains("Socket") {
                        current_hand.insert(iid.clone());
                        current_stash.remove(&iid);
                        if let Some(slot) = parse_socket_index_from_target(tgt) {
                            set_hand_slot_mapping(&iid, slot, &mut hand_slot_to_iid, &mut iid_to_hand_slot);
                        }
                    }
                }

                if let Some(cap) = re_sold.captures(trimmed) {
                    let iid = cap["iid"].to_string();
                    current_hand.remove(&iid);
                    current_stash.remove(&iid);
                    remove_hand_slot_mapping(&iid, &mut hand_slot_to_iid, &mut iid_to_hand_slot);
                }
                if let Some(cap) = re_removed.captures(trimmed) {
                    let iid = cap["iid"].to_string();
                    current_hand.remove(&iid);
                    current_stash.remove(&iid);
                    remove_hand_slot_mapping(&iid, &mut hand_slot_to_iid, &mut iid_to_hand_slot);
                }
                if trimmed.contains("Cards Disposed:") {
                    for mat in re_item_id.find_iter(trimmed) {
                        let iid = mat.as_str().to_string();
                        current_hand.remove(&iid);
                        current_stash.remove(&iid);
                        remove_hand_slot_mapping(&iid, &mut hand_slot_to_iid, &mut iid_to_hand_slot);
                    }
                }

                if trimmed.contains("Cards Spawned:")
                    || trimmed.contains("Cards Dealt:")
                    || trimmed.contains("NetMessageGameStateSync")
                    || trimmed.contains("Successfully moved card to:")
                {
                    is_sync = true;
                }

                if is_sync {
                    if let Some(cap) = re_id.captures(trimmed) {
                        last_iid = cap["id"].to_string();
                    } else if let Some(cap) = re_tid.captures(trimmed) {
                        if !last_iid.is_empty() {
                            inst_to_temp.insert(last_iid.clone(), cap["tid"].to_string());
                        }
                    } else if let Some(cap) = re_owner.captures(trimmed) {
                        cur_owner = cap["val"].to_string();
                    } else if let Some(cap) = re_section.captures(trimmed) {
                        if !last_iid.is_empty() && cur_owner.as_str() == "Player" && last_iid.starts_with("itm_") {
                            let sec_val = &cap["val"];
                            if sec_val == "Hand" || sec_val == "Player" {
                                current_hand.insert(last_iid.clone());
                                current_stash.remove(&last_iid);
                            } else if sec_val == "Stash" || sec_val == "Storage" || sec_val == "PlayerStorage" {
                                current_stash.insert(last_iid.clone());
                                current_hand.remove(&last_iid);
                                remove_hand_slot_mapping(&last_iid, &mut hand_slot_to_iid, &mut iid_to_hand_slot);
                            } else {
                                current_hand.remove(&last_iid);
                                current_stash.remove(&last_iid);
                                remove_hand_slot_mapping(&last_iid, &mut hand_slot_to_iid, &mut iid_to_hand_slot);
                            }
                        }
                        last_iid.clear();
                        cur_owner.clear();
                    } else if trimmed.contains("Finished processing") {
                        is_sync = false;
                    }
                }

                let Some(idx) = active_idx else {
                    continue;
                };
                if idx >= records.len() {
                    active_idx = None;
                    continue;
                }

                if let Some(cap) = re_hero.captures(trimmed) {
                    let hero_name = cap["hero"].to_string();
                    if records[idx].hero.as_ref().map(|h| h.trim().is_empty()).unwrap_or(true) {
                        records[idx].hero = Some(hero_name);
                    }
                }

                if let Some(cap) = re_combat_duration.captures(trimmed) {
                    if let Ok(duration) = cap["dur"].parse::<f64>() {
                        last_pvp_duration = Some(duration);
                    }
                }

                if let Some(cap) = re_state_change.captures(trimmed) {
                    let next_state = cap["state"].trim();
                    match next_state {
                        "PVPCombatState" => {
                            in_pvp = true;
                            last_pvp_start = timestamp.clone();
                            last_pvp_duration = None;
                        }
                        "ReplayState" if in_pvp => {
                            let battle_victory = history_detect_pvp_victory(&recent_lines);
                            let start_time = last_pvp_start
                                .clone()
                                .or(timestamp.clone())
                                .unwrap_or_default();
                            let battle_day = records[idx].days.max(1);

                            if !start_time.is_empty() {
                                let duplicated = records[idx]
                                    .pvp_battles
                                    .iter()
                                    .any(|b| b.day == battle_day && b.start_time == start_time);
                                if !duplicated {
                                    records[idx].pvp_battles.push(HistoryBattleRecord {
                                        day: battle_day,
                                        start_time,
                                        victory: battle_victory,
                                        duration: last_pvp_duration,
                                        screenshot: None,
                                        lineup_cards: Vec::new(),
                                        enemy_lineup_cards: Vec::new(),
                                    });
                                    records[idx].days = records[idx].days.saturating_add(1);
                                }
                            }

                            in_pvp = false;
                            last_pvp_start = None;
                            last_pvp_duration = None;
                        }
                        "EndRunVictoryState" | "EndRunDefeatState" => {
                            records[idx].end_time = timestamp.clone();
                            records[idx].victory = next_state == "EndRunVictoryState";
                            records[idx].is_finished = true;
                            active_idx = None;
                            in_pvp = false;
                            last_pvp_start = None;
                            last_pvp_duration = None;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    history_merge_restart_sessions(&mut records);
    normalize_history(HistoryRoot { matches: records })
}

#[tauri::command]
pub fn rebuild_match_history(force: Option<bool>) -> Result<serde_json::Value, String> {
    let force = force.unwrap_or(false);
    let paths = vec![
        crate::data_management::log_paths::get_prev_log_path(),
        crate::data_management::log_paths::get_log_path(),
    ];
    let signature = build_history_log_signature(&paths);

    if !force {
        let guard = LAST_HISTORY_LOG_SIGNATURE.get_or_init(|| Mutex::new(None));
        if let Ok(last) = guard.lock() {
            if let Some(prev_sig) = last.as_ref() {
                if prev_sig == &signature {
                    return crate::user_data::load_match_history();
                }
            }
        }
    }

    let parsed = parse_history_from_logs(&paths);
    let today = history_today_date();
    let existing = crate::user_data::load_match_history()
        .unwrap_or_else(|_| serde_json::json!({ "matches": [] }));
    let mut existing_root = serde_json::from_value::<HistoryRoot>(existing).unwrap_or_default();
    let legacy_path = crate::user_data::match_history_path();
    if legacy_path.exists() {
        if let Ok(text) = std::fs::read_to_string(&legacy_path) {
            if let Ok(legacy_root) = serde_json::from_str::<HistoryRoot>(&text) {
                if !legacy_root.matches.is_empty() {
                    existing_root.matches.extend(legacy_root.matches);
                }
            }
        }
    }

    let mut merged: HashMap<String, HistoryMatchRecord> = HashMap::new();
    let mut start_time_index: HashMap<String, String> = HashMap::new();
    let mut first_seen_date_by_start: HashMap<String, String> = HashMap::new();
    let mut first_seen_date_by_match_id: HashMap<String, String> = HashMap::new();
    let mut existing_battle_screenshots: HashMap<(String, u32), String> = HashMap::new();
    let mut existing_battle_lineups: HashMap<(String, u32), Vec<HistoryBattleLineupCard>> = HashMap::new();
    let mut existing_enemy_battle_lineups: HashMap<(String, u32), Vec<HistoryBattleLineupCard>> = HashMap::new();

    for mut record in existing_root.matches {
        for battle in &record.pvp_battles {
            if let Some(path) = battle.screenshot.as_ref().filter(|s| !s.trim().is_empty()) {
                existing_battle_screenshots.insert(
                    (
                        record.match_id.clone(),
                        battle.day,
                    ),
                    path.clone(),
                );
            }
            if !battle.lineup_cards.is_empty() {
                existing_battle_lineups.insert(
                    (
                        record.match_id.clone(),
                        battle.day,
                    ),
                    battle.lineup_cards.clone(),
                );
            }
            if !battle.enemy_lineup_cards.is_empty() {
                existing_enemy_battle_lineups.insert(
                    (
                        record.match_id.clone(),
                        battle.day,
                    ),
                    battle.enemy_lineup_cards.clone(),
                );
            }
        }

        if force && !record.is_finished {
            continue;
        }

        if let Some(date) = normalize_optional_text(&record.game_date) {
            if !record.start_time.trim().is_empty() {
                first_seen_date_by_start
                    .entry(record.start_time.clone())
                    .or_insert_with(|| date.clone());
            }
            if !record.match_id.trim().is_empty() {
                first_seen_date_by_match_id
                    .entry(record.match_id.clone())
                    .or_insert(date);
            }
        }

        if record.game_date.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true) {
            record.game_date = Some(today.clone());
        }
        let key = history_record_key(&record);
        start_time_index
            .entry(record.start_time.clone())
            .or_insert_with(|| key.clone());
        merged.insert(key, record);
    }

    for mut record in parsed.matches {
        for battle in &mut record.pvp_battles {
            if battle
                .screenshot
                .as_ref()
                .map(|s| s.trim().is_empty())
                .unwrap_or(true)
            {
                if let Some(path) = existing_battle_screenshots.get(&(
                    record.match_id.clone(),
                    battle.day,
                )) {
                    battle.screenshot = Some(path.clone());
                }
            }
            if battle.lineup_cards.is_empty() {
                if let Some(lineup) = existing_battle_lineups.get(&(
                    record.match_id.clone(),
                    battle.day,
                )) {
                    battle.lineup_cards = lineup.clone();
                }
            }
            if battle.enemy_lineup_cards.is_empty() {
                if let Some(lineup) = existing_enemy_battle_lineups.get(&(
                    record.match_id.clone(),
                    battle.day,
                )) {
                    battle.enemy_lineup_cards = lineup.clone();
                }
            }
        }

        if record.game_date.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true) {
            if let Some(existing_date) = first_seen_date_by_match_id
                .get(&record.match_id)
                .or_else(|| first_seen_date_by_start.get(&record.start_time))
            {
                record.game_date = Some(existing_date.clone());
            }
        }
        if record.game_date.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true) {
            record.game_date = Some(today.clone());
        }

        let key = history_record_key(&record);
        if let Some(existing_rec) = merged.get_mut(&key) {
            if force {
                *existing_rec = record;
            } else {
                merge_match_records(existing_rec, &record);
            }
            continue;
        }

        if let Some(existing_key) = start_time_index.get(&record.start_time).cloned() {
            if let Some(existing_rec) = merged.get_mut(&existing_key) {
                if force {
                    *existing_rec = record;
                } else {
                    merge_match_records(existing_rec, &record);
                }
                continue;
            }
        }

        start_time_index
            .entry(record.start_time.clone())
            .or_insert_with(|| key.clone());
        merged.insert(key, record);
    }

    let mut normalized = normalize_history(HistoryRoot {
        matches: merged.into_values().collect(),
    });

    remap_battle_screenshots_strict(&mut normalized);

    let value = serde_json::to_value(&normalized).map_err(|e| e.to_string())?;
    crate::user_data::save_match_history(&value)?;

    let guard = LAST_HISTORY_LOG_SIGNATURE.get_or_init(|| Mutex::new(None));
    if let Ok(mut last) = guard.lock() {
        *last = Some(signature);
    }

    Ok(value)
}

#[tauri::command]
pub async fn analyze_battle_lineup_from_screenshot(
    app: tauri::AppHandle,
    req: ManualBattleLineupAnalyzeRequest,
) -> Result<ManualBattleLineupAnalyzeResult, String> {
    let model_path = app
        .path()
        .resource_dir()
        .ok()
        .map(|p| p.join("resources").join("models").join("best.onnx"))
        .filter(|p| p.exists())
        .or_else(|| {
            let local = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("resources")
                .join("models")
                .join("best.onnx");
            if local.exists() { Some(local) } else { None }
        })
        .ok_or_else(|| "missing model file: resources/models/best.onnx".to_string())?;

    let screenshot_path = req.screenshot_path.trim().to_string();
    if screenshot_path.is_empty() {
        return Err("empty screenshot_path".to_string());
    }
    if !PathBuf::from(&screenshot_path).exists() {
        return Err(format!("screenshot file not found: {}", screenshot_path));
    }

    let match_start_time = req.match_start_time.trim().to_string();
    let match_id = req.match_id.clone().unwrap_or_default();
    let battle_start_time = req.battle_start_time.trim().to_string();
    if match_start_time.is_empty() || battle_start_time.is_empty() {
        return Err("empty match_start_time or battle_start_time".to_string());
    }

    let battle_day = req.battle_day;
    let victory = req.victory;
    let duration = req.duration;
    let event_start_time = battle_start_time.clone();
    let event_match_id = match_id.clone();

    let _ = app.emit(
        "manual-lineup-progress",
        serde_json::json!({
            "match_id": event_match_id,
            "battle_day": battle_day,
            "battle_start_time": event_start_time,
            "phase": "yolo",
            "done": 0,
            "total": 1
        }),
    );

    let app_for_task = app.clone();

    let result = tauri::async_runtime::spawn_blocking(move || {
        let card_meta_map = load_history_card_meta_map();
        let _ = app_for_task.emit(
            "manual-lineup-progress",
            serde_json::json!({
                "match_id": match_id,
                "battle_day": battle_day,
                "battle_start_time": battle_start_time,
                "phase": "yolo",
                "done": 1,
                "total": 1
            }),
        );

        let visual = crate::monster_recognition::extract_lineups_from_battle_file_with_progress(
            &screenshot_path,
            &model_path,
            true,
            |done, total| {
                let _ = app_for_task.emit(
                    "manual-lineup-progress",
                    serde_json::json!({
                        "match_id": match_id,
                        "battle_day": battle_day,
                        "battle_start_time": battle_start_time,
                        "phase": "matching",
                        "done": done,
                        "total": total
                    }),
                );
            },
        )
        .or_else(|_| {
            crate::monster_recognition::extract_lineups_from_battle_file_with_progress(
                &screenshot_path,
                &model_path,
                false,
                |done, total| {
                    let _ = app_for_task.emit(
                        "manual-lineup-progress",
                        serde_json::json!({
                            "match_id": match_id,
                            "battle_day": battle_day,
                            "battle_start_time": battle_start_time,
                            "phase": "matching",
                            "done": done,
                            "total": total
                        }),
                    );
                },
            )
        })?;

        let (self_cards, enemy_cards) = visual;
        let self_lineup = history_build_lineup_from_visual(&self_cards, &card_meta_map);
        let enemy_lineup = history_build_lineup_from_visual(&enemy_cards, &card_meta_map);

        let self_json = if self_lineup.is_empty() {
            None
        } else {
            serde_json::to_string(&self_lineup).ok()
        };
        let enemy_json = if enemy_lineup.is_empty() {
            None
        } else {
            serde_json::to_string(&enemy_lineup).ok()
        };

        crate::user_data::upsert_match_battle_snapshot(
            &match_start_time,
            battle_day,
            &battle_start_time,
            victory,
            duration,
            Some(&screenshot_path),
            self_json.as_deref(),
            enemy_json.as_deref(),
        )?;

        Ok::<ManualBattleLineupAnalyzeResult, String>(ManualBattleLineupAnalyzeResult {
            self_count: self_lineup.len(),
            enemy_count: enemy_lineup.len(),
            screenshot_path,
        })
    })
    .await
    .map_err(|e| e.to_string())??;

    let _ = app.emit(
        "match-history-updated",
        serde_json::json!({
            "day": battle_day,
            "start_time": event_start_time,
            "has_screenshot": true,
            "manual_analyze": true
        }),
    );

    let _ = app.emit(
        "manual-lineup-progress",
        serde_json::json!({
            "match_id": event_match_id,
            "battle_day": battle_day,
            "battle_start_time": event_start_time,
            "phase": "done",
            "done": 1,
            "total": 1
        }),
    );

    Ok(result)
}

#[tauri::command]
pub async fn capture_battle_screenshot_manual(
    app: tauri::AppHandle,
    req: ManualBattleScreenshotRequest,
) -> Result<ManualBattleScreenshotResult, String> {
    let match_start_time = req.match_start_time.trim().to_string();
    let battle_start_time = req.battle_start_time.trim().to_string();
    if match_start_time.is_empty() || battle_start_time.is_empty() {
        return Err("empty match_start_time or battle_start_time".to_string());
    }

    let battle_day = req.battle_day;
    let victory = req.victory;
    let duration = req.duration;
    let match_start_for_capture = match_start_time.clone();

    let captured = tauri::async_runtime::spawn_blocking(move || {
        crate::data_management::log_monitor::capture_bazaar_round_screenshot(
            &match_start_for_capture,
            battle_day,
            &battle_start_time,
            false,
        )
    })
    .await
    .map_err(|e| e.to_string())??;

    let Some(path) = captured else {
        return Err("未检测到游戏窗口，无法执行仅游戏窗口截图。请先切回《The Bazaar》后重试。".to_string());
    };

    let screenshot_path = path.to_string_lossy().to_string();

    crate::user_data::upsert_match_battle_snapshot(
        &match_start_time,
        battle_day,
        &req.battle_start_time,
        victory,
        duration,
        Some(&screenshot_path),
        None,
        None,
    )?;

    let _ = app.emit(
        "match-history-updated",
        serde_json::json!({
            "day": battle_day,
            "start_time": req.battle_start_time,
            "has_screenshot": true,
            "manual_screenshot": true
        }),
    );

    Ok(ManualBattleScreenshotResult { screenshot_path })
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
