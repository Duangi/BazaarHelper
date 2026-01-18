use std::sync::{Arc, RwLock};
use tauri::{State, Manager, Emitter};
use serde::{Serialize, Deserialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use regex::Regex;
use std::io::{Read, BufRead, BufReader, Seek, SeekFrom, Write};
use std::fs::File;
use std::{thread, time, panic};
use tokio;
use chrono::Local;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_RBUTTON};

use crate::monster_recognition::scan_and_identify_monster_at_mouse;

pub mod monster_recognition;

// --- Logger Helper ---
pub fn log_to_file(msg: &str) {
    if let Ok(mut exe_path) = std::env::current_exe() {
        exe_path.pop();
        exe_path.push("app_debug.txt");
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(exe_path) {
            let _ = writeln!(f, "[{}] {}", get_time_str(), msg);
            let _ = f.flush();
        }
    }
}

fn get_time_str() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string()
}

pub fn set_panic_hook() {
    panic::set_hook(Box::new(|panic_info| {
        let payload = panic_info.payload();
        let message = if let Some(s) = payload.downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = payload.downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic".to_string()
        };

        let location = panic_info.location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown location".to_string());
        
        log_to_file(&format!("FATAL PANIC at {}: {}", location, message));
        
        // Output to stderr as well
        eprintln!("FATAL PANIC at {}: {}", location, message);
    }));
}

pub fn log_system_info(app_handle: &tauri::AppHandle) {
    log_to_file("--- System Info ---");
    log_to_file(&format!("OS: {}", std::env::consts::OS));
    log_to_file(&format!("ARCH: {}", std::env::consts::ARCH));
    
    if let Ok(exe_path) = std::env::current_exe() {
        log_to_file(&format!("EXE Path: {:?}", exe_path));
    }
    
    if let Ok(cwd) = std::env::current_dir() {
        log_to_file(&format!("CWD: {:?}", cwd));
    }

    log_to_file(&format!("Resource Dir: {:?}", app_handle.path().resource_dir().ok()));
    log_to_file(&format!("App Config Dir: {:?}", app_handle.path().app_config_dir().ok()));
    log_to_file(&format!("App Local Data Dir: {:?}", app_handle.path().app_local_data_dir().ok()));
    
    // Log environment variables that might affect execution
    for var in ["PATH", "USERNAME", "APPDATA", "LOCALAPPDATA"] {
        if let Ok(val) = std::env::var(var) {
            log_to_file(&format!("Env {}: {}", var, val));
        }
    }

    let lp = get_log_path();
    log_to_file(&format!("Game Log Path: {:?}", lp));
    log_to_file(&format!("Game Log Exists: {}", lp.exists()));
    
    log_to_file("-------------------");
}

// --- Data Models ---
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PersistentState {
    pub day: u32,
    pub inst_to_temp: HashMap<String, String>,
    pub current_hand: HashSet<String>,
    pub current_stash: HashSet<String>,
    #[serde(default)]
    pub detection_hotkey: Option<i32>,
}

impl Default for PersistentState {
    fn default() -> Self {
        Self {
            day: 1,
            inst_to_temp: HashMap::new(),
            current_hand: HashSet::new(),
            current_stash: HashSet::new(),
            detection_hotkey: Some(VK_RBUTTON.0 as i32),
        }
    }
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RawSkill {
    pub en: Option<String>,
    pub cn: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RawItem {
    pub id: String,
    pub name_en: Option<String>,
    pub name_cn: Option<String>,
    pub starting_tier: Option<String>,
    pub available_tiers: Option<String>,
    pub heroes: Option<String>,
    pub tags: Option<String>,
    pub size: Option<String>,
    pub cooldown: Option<f32>,
    pub cooldown_tiers: Option<String>,
    pub damage_tiers: Option<String>,
    pub heal_tiers: Option<String>,
    pub shield_tiers: Option<String>,
    pub ammo_tiers: Option<String>,
    pub crit_tiers: Option<String>,
    pub multicast_tiers: Option<String>,
    pub burn_tiers: Option<String>,
    pub poison_tiers: Option<String>,
    pub regen_tiers: Option<String>,
    pub lifesteal_tiers: Option<String>,
    pub skills: Option<Vec<RawSkill>>,
    pub enchantments: Option<serde_json::Value>,
    pub image: Option<String>,
    #[serde(default)]
    pub description_cn: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ItemData {
    pub uuid: String,
    pub name: String,
    pub name_cn: String,
    pub tier: String,
    pub available_tiers: String,
    pub tags: String,
    pub size: Option<String>,
    pub processed_tags: Vec<String>,
    pub heroes: Vec<String>,
    pub cooldown: Option<f32>,
    pub cooldown_tiers: String,
    pub damage_tiers: String,
    pub heal_tiers: String,
    pub shield_tiers: String,
    pub ammo_tiers: String,
    pub crit_tiers: String,
    pub multicast_tiers: String,
    pub burn_tiers: String,
    pub poison_tiers: String,
    pub regen_tiers: String,
    pub lifesteal_tiers: String,
    pub skills: Vec<String>,
    pub enchantments: Vec<String>,
    pub description: String,
}

impl From<RawItem> for ItemData {
    fn from(raw: RawItem) -> Self {
        let name_en = raw.name_en.clone().unwrap_or_else(|| "Unknown".to_string());
        let name_cn = raw.name_cn.clone().unwrap_or_else(|| name_en.clone());

        let h_str = raw.heroes.clone().unwrap_or_default();
        let heroes = if h_str.is_empty() {
            vec!["Common".to_string()]
        } else {
            h_str.split('|').map(|s| s.trim().to_string()).collect()
        };

        let processed_tags = raw.tags.as_deref().unwrap_or_default()
            .split('|')
            .map(|s| {
                let part = s.trim();
                // Pick the last part after / if it exists
                part.split(" / ").last().unwrap_or(part).trim().to_string()
            })
            .filter(|s| !s.is_empty())
            .filter(|s| !s.contains("隐藏") && !s.contains("Hide") && !s.contains("Hidden"))
            .collect();

        let skills = raw.skills.unwrap_or_default().into_iter()
            .filter_map(|s| s.cn.or(s.en))
            .filter(|s| !s.is_empty())
            .collect();
        
        // Handle enchantments
        let mut enchantments = Vec::new();
        if let Some(val) = raw.enchantments {
            if let Some(obj) = val.as_object() {
                for (_key, details) in obj {
                    let name_cn = details.get("name_cn").and_then(|v| v.as_str());
                    let effect_cn = details.get("effect_cn").and_then(|v| v.as_str());
                    let effect_en = details.get("effect_en").and_then(|v| v.as_str());
                    
                    let effect = effect_cn.or(effect_en);
                    if let Some(eff) = effect {
                        if let Some(n) = name_cn {
                            // 使用分隔符方便前端拆分名称和描述
                            enchantments.push(format!("{}|{}", n, eff));
                        } else {
                            enchantments.push(eff.to_string());
                        }
                    }
                }
            }
        }
        // Removed .sort() to keep JSON order

        ItemData {
            uuid: raw.id,
            name: name_en,
            name_cn,
            tier: raw.starting_tier.clone().unwrap_or_else(|| "Bronze".to_string()),
            available_tiers: raw.available_tiers.unwrap_or_default(),
            tags: raw.tags.unwrap_or_default(),
            size: raw.size,
            processed_tags,
            heroes,
            cooldown: raw.cooldown.map(|c| c / 1000.0), // ms to s
            cooldown_tiers: raw.cooldown_tiers.unwrap_or_default(),
            damage_tiers: raw.damage_tiers.unwrap_or_default(),
            heal_tiers: raw.heal_tiers.unwrap_or_default(),
            shield_tiers: raw.shield_tiers.unwrap_or_default(),
            ammo_tiers: raw.ammo_tiers.unwrap_or_default(),
            crit_tiers: raw.crit_tiers.unwrap_or_default(),
            multicast_tiers: raw.multicast_tiers.unwrap_or_default(),
            burn_tiers: raw.burn_tiers.unwrap_or_default(),
            poison_tiers: raw.poison_tiers.unwrap_or_default(),
            regen_tiers: raw.regen_tiers.unwrap_or_default(),
            lifesteal_tiers: raw.lifesteal_tiers.unwrap_or_default(),
            skills,
            enchantments,
            description: raw.description_cn.unwrap_or_default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TierInfo {
    pub description: Vec<String>,
    pub extra_description: Vec<String>,
    pub cd: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MonsterSubItem {
    pub id: Option<String>,
    pub name: String,
    pub name_en: Option<String>,
    pub tier: Option<String>,
    pub current_tier: Option<String>,
    pub tags: Option<Vec<String>>,
    pub tiers: Option<HashMap<String, Option<TierInfo>>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MonsterData {
    pub name: String,
    pub name_zh: String,
    pub available: Option<String>,
    pub health: Option<serde_json::Value>,
    pub level: Option<serde_json::Value>,
    pub skills: Option<Vec<MonsterSubItem>>,
    pub items: Option<Vec<MonsterSubItem>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SyncPayload {
    pub hand_items: Vec<ItemData>,
    pub stash_items: Vec<ItemData>,
    pub all_tags: Vec<String>,
}

pub struct ItemDb {
    pub list: Vec<ItemData>,
    pub id_map: HashMap<String, usize>,
    pub unique_tags: Vec<String>,
}

pub struct SkillDb {
    pub list: Vec<ItemData>, // Skills have similar structure
    pub id_map: HashMap<String, usize>,
}

pub struct DbState {
    pub items: Arc<RwLock<ItemDb>>,
    pub skills: Arc<RwLock<SkillDb>>,
    pub monsters: Arc<RwLock<serde_json::Map<String, serde_json::Value>>>,
}

fn construct_monster_sub_item(item_data: Option<ItemData>, fallback_name_cn: &str, fallback_name_en: &str, current_tier: &str, override_size: Option<&str>) -> serde_json::Value {
    let mut desc = Vec::new();
    let mut name_cn = fallback_name_cn.to_string();
    let mut name_en = fallback_name_en.to_string();
    let mut cooldown = None;
    let mut size = override_size.map(|s| s.to_string());
    let mut id = "".to_string();
    let mut tiers = serde_json::Map::new();

    if let Some(item) = item_data {
        name_cn = item.name_cn;
        name_en = item.name;
        id = item.uuid;
        if size.is_none() {
            size = item.size;
        }
        if !item.description.is_empty() {
            desc.push(item.description.clone());
        }
        for s in &item.skills {
            desc.push(s.clone());
        }
        cooldown = item.cooldown;

        // Parse multiples tiers if available
        if !item.available_tiers.is_empty() {
            let avail_list: Vec<&str> = item.available_tiers.split('/').collect();
            let cd_list: Vec<&str> = item.cooldown_tiers.split('/').collect();
            
            for (i, t_name) in avail_list.iter().enumerate() {
                let mut t_info = serde_json::Map::new();
                t_info.insert("description".to_string(), serde_json::Value::Array(desc.iter().map(|s| serde_json::Value::String(s.clone())).collect()));
                t_info.insert("extra_description".to_string(), serde_json::Value::Array(vec![]));
                
                let cd_val = if i < cd_list.len() {
                    let ms: f32 = cd_list[i].trim().parse().unwrap_or(0.0);
                    if ms > 0.0 { Some(format!("{:.1}s", ms / 1000.0)) } else { None }
                } else if !cd_list.is_empty() && !cd_list[0].is_empty() {
                    // Repeat last cd value if more tiers exist
                    let ms: f32 = cd_list.last().unwrap().trim().parse().unwrap_or(0.0);
                    if ms > 0.0 { Some(format!("{:.1}s", ms / 1000.0)) } else { None }
                } else if i == 0 {
                    cooldown.map(|c| format!("{:.1}s", c))
                } else {
                    None
                };
                
                t_info.insert("cd".to_string(), cd_val.map(serde_json::Value::String).unwrap_or(serde_json::Value::Null));
                tiers.insert(t_name.to_lowercase(), serde_json::Value::Object(t_info));
            }
        }
    }

    if tiers.is_empty() || !tiers.contains_key(&current_tier.to_lowercase()) {
        let mut t_info = serde_json::Map::new();
        t_info.insert("description".to_string(), serde_json::Value::Array(desc.into_iter().map(serde_json::Value::String).collect()));
        t_info.insert("extra_description".to_string(), serde_json::Value::Array(vec![]));
        t_info.insert("cd".to_string(), cooldown.map(|c| serde_json::Value::String(format!("{:.1}s", c))).unwrap_or(serde_json::Value::Null));
        
        tiers.insert(current_tier.to_lowercase(), serde_json::Value::Object(t_info));
    }
    
    let tier_label = format!("{}+", current_tier);
    
    let mut sub = serde_json::Map::new();
    sub.insert("name".to_string(), serde_json::Value::String(name_cn));
    sub.insert("name_en".to_string(), serde_json::Value::String(name_en));
    sub.insert("id".to_string(), serde_json::Value::String(id));
    sub.insert("tier".to_string(), serde_json::Value::String(tier_label));
    sub.insert("current_tier".to_string(), serde_json::Value::String(current_tier.to_string()));
    
    // Normalize size if it exists
    let final_size = size.map(|s| {
        let normalized = s.split(" / ").next().unwrap_or(&s).to_string();
        normalized
    });
    
    sub.insert("size".to_string(), final_size.map(serde_json::Value::String).unwrap_or(serde_json::Value::Null));
    sub.insert("tiers".to_string(), serde_json::Value::Object(tiers));
    
    serde_json::Value::Object(sub)
}

fn get_log_path() -> PathBuf {
    if cfg!(target_os = "macos") {
        let home = std::env::var("HOME").unwrap_or_default();
        PathBuf::from(home)
            .join("Library")
            .join("Logs")
            .join("Tempo Storm")
            .join("The Bazaar")
            .join("Player.log")
    } else {
        let home = std::env::var("USERPROFILE").unwrap_or_default();
        PathBuf::from(home)
            .join("AppData")
            .join("LocalLow")
            .join("Tempo Storm")
            .join("The Bazaar")
            .join("Player.log")
    }
}

#[tauri::command]
#[allow(dead_code)]
async fn start_template_loading(app: tauri::AppHandle) -> Result<(), String> {
    let resources_path = app.path().resource_dir().map_err(|e| {
        let err = format!("Failed to get resource dir in template loading: {}", e);
        log_to_file(&err);
        err
    })?;
    let res_dir = resources_path.join("resources");
    let cache_dir = get_cache_path().parent().ok_or_else(|| {
        let err = "Failed to get cache parent dir".to_string();
        log_to_file(&err);
        err
    })?.to_path_buf();
    
    // 异步加载
    tauri::async_runtime::spawn(async move {
        let _ = monster_recognition::preload_templates_async(res_dir, cache_dir).await;
    });
    
    Ok(())
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

#[tauri::command]
async fn restore_game_focus() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, SetForegroundWindow, ShowWindow, SW_SHOW};
        use windows::core::PCWSTR;

        let window_name: Vec<u16> = "The Bazaar\0".encode_utf16().collect();
        unsafe {
            if let Ok(hwnd) = FindWindowW(PCWSTR::null(), PCWSTR(window_name.as_ptr())) {
                if !hwnd.is_invalid() {
                    // 先 ShowWindow 确保不是最小化
                    let _ = ShowWindow(hwnd, SW_SHOW);
                    let _ = SetForegroundWindow(hwnd);
                }
            }
        }
    }
    Ok(())
}

fn get_cache_path() -> PathBuf {
    if cfg!(target_os = "macos") {
        let home = std::env::var("HOME").unwrap_or_default();
        PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("com.duang.BazaarHelper")
            .join("state_cache.json")
    } else {
        let home = std::env::var("USERPROFILE").unwrap_or_default();
        PathBuf::from(home)
            .join("AppData")
            .join("Local")
            .join("BazaarHelper")
            .join("state_cache.json")
    }
}

fn get_prev_log_path() -> PathBuf {
    let mut p = get_log_path();
    p.set_file_name("Player-prev.log");
    p
}

fn save_state(state: &PersistentState) {
    let path = get_cache_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string(state) {
        let _ = std::fs::write(path, json);
    }
}

fn load_state() -> PersistentState {
    let path = get_cache_path();
    if let Ok(json) = std::fs::read_to_string(path) {
        if let Ok(state) = serde_json::from_str::<PersistentState>(&json) {
            return state;
        }
    }
    PersistentState::default()
}

fn lookup_item(tid: &str, items_db: &ItemDb, skills_db: &SkillDb) -> Option<ItemData> {
    if let Some(&index) = items_db.id_map.get(tid) {
        return items_db.list.get(index).cloned();
    }
    if let Some(&index) = skills_db.id_map.get(tid) {
        return skills_db.list.get(index).cloned();
    }
    None
}

fn lookup_item_by_name(name_cn: &str, items_db: &ItemDb, skills_db: &SkillDb) -> Option<ItemData> {
    // 先在物品库中查找完整名字
    for item in &items_db.list {
        if item.name_cn == name_cn {
            return Some(item.clone());
        }
    }
    // 再在技能库中查找完整名字
    for skill in &skills_db.list {
        if skill.name_cn == name_cn {
            return Some(skill.clone());
        }
    }
    
    // 如果找不到，尝试去除空格及空格之前的前缀（如"毒性蔓延 獠牙" -> "獠牙"）
    if let Some(space_pos) = name_cn.rfind(' ') {
        let base_name = &name_cn[space_pos + 1..];
        
        // 用基础名字再查找一次
        for item in &items_db.list {
            if item.name_cn == base_name {
                return Some(item.clone());
            }
        }
        for skill in &skills_db.list {
            if skill.name_cn == base_name {
                return Some(skill.clone());
            }
        }
    }
    
    None
}

// --- Commands ---
#[tauri::command]
fn get_all_monsters(state: State<'_, DbState>) -> Result<serde_json::Map<String, serde_json::Value>, String> {
    let db = state.monsters.read().map_err(|_| "DB Busy")?;
    Ok(db.clone())
}

#[tauri::command]
fn recognize_monsters_from_screenshot(day: Option<u32>) -> Result<Vec<monster_recognition::MonsterRecognitionResult>, String> {
    let day_filter = day.map(|d| if d >= 10 { "Day 10+".to_string() } else { format!("Day {}", d) });
    monster_recognition::recognize_monsters(day_filter)
}

#[tauri::command]
fn get_template_loading_progress() -> monster_recognition::LoadingProgress {
    monster_recognition::get_loading_progress()
}

#[tauri::command]
fn get_current_day(hours_per_day: Option<u32>, retro: Option<bool>) -> Result<u32, String> {
    // Return cached value if available, log scan only as fallback
    let cached = load_state();
    if cached.day > 0 {
        return Ok(cached.day);
    }
    
    let hours = hours_per_day.unwrap_or(6);
    let retro = retro.unwrap_or(false);
    let log_path = get_log_path();
    
    // Fallback to scan only if cache is 0 (first run)
    if log_path.exists() {
        // Use a more memory-efficient way to read large logs
        let mut file = File::open(&log_path).map_err(|e| e.to_string())?;
        let metadata = file.metadata().map_err(|e| e.to_string())?;
        let file_size = metadata.len();
        
        // Read at most 5MB from the end
        let read_size = file_size.min(5_000_000) as usize;
        let mut buffer = vec![0u8; read_size];
        file.seek(SeekFrom::End(-(read_size as i64))).map_err(|e| e.to_string())?;
        file.read_exact(&mut buffer).map_err(|e| e.to_string())?;
        
        let content = String::from_utf8_lossy(&buffer);
        if let Some(day) = calculate_day_from_log(&content, hours, retro) {
            return Ok(day);
        }
    }

    Ok(1)
}

#[tauri::command]
fn update_day(day: u32) -> Result<(), String> {
    let mut state = load_state();
    state.day = day;
    save_state(&state);
    println!("[State] Manually updated Day to: {}", day);
    Ok(())
}

#[tauri::command]
fn get_detection_hotkey() -> Option<i32> {
    load_state().detection_hotkey
}

#[tauri::command]
fn set_detection_hotkey(hotkey: i32) {
    let mut state = load_state();
    state.detection_hotkey = Some(hotkey);
    save_state(&state);
    println!("[Config] Detection hotkey updated to: {}", hotkey);
}

fn calculate_day_from_log(content: &str, _hours: u32, retro: bool) -> Option<u32> {
    let start_pos = if retro { content.rfind("NetMessageRunInitialized").unwrap_or(0) } else { 0 };
    let slice = &content[start_pos..];
    let mut current_day: u32 = 1; // Default to 1
    let mut in_pvp = false;
    let mut hour_count: u32 = 0;

    for line in slice.lines() {
        let l = line.trim();
        if l.contains("NetMessageRunInitialized") {
            current_day = 1; in_pvp = false; hour_count = 0; continue;
        }
        
        if l.contains("to [PVPCombatState]") { in_pvp = true; continue; }

        if in_pvp && l.contains("State changed") && (l.contains("to [ChoiceState]") || l.contains("to [LevelUpState]")) {
            current_day = current_day.saturating_add(1);
            in_pvp = false; hour_count = 0; continue;
        }

        if l.starts_with("[") && l.contains("State changed from [ChoiceState] to [") {
             if !l.contains("to [ChoiceState]") && !l.contains("to [PVPCombatState]") {
                hour_count = hour_count.saturating_add(1);
                if hour_count >= 10 { // Fallback for modes without PVP or unexpected logs
                    current_day = current_day.saturating_add(1);
                    hour_count = 0;
                }
             }
        }
    }
    
    Some(current_day)
}

// --- App Run ---
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    set_panic_hook();
    log_to_file("=================== App Starting ===================");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(DbState {
            items: Arc::new(RwLock::new(ItemDb {
                list: Vec::new(),
                id_map: HashMap::new(),
                unique_tags: Vec::new(),
            })),
            skills: Arc::new(RwLock::new(SkillDb {
                list: Vec::new(),
                id_map: HashMap::new(),
            })),
            monsters: Arc::new(RwLock::new(serde_json::Map::new())),
        })
        .setup(|app| {
            log_system_info(app.handle());
            
            // 设置窗口不占据焦点，穿透焦点解决遮挡游戏悬浮的问题 (仅 Windows)
            #[cfg(target_os = "windows")]
            {
                use tauri::Manager;
                if let Some(window) = app.get_webview_window("main") {
                    if let Ok(hwnd) = window.hwnd() {
                        use windows::Win32::Foundation::HWND;
                        use windows::Win32::UI::WindowsAndMessaging::{GetWindowLongW, SetWindowLongW, GWL_EXSTYLE, WS_EX_NOACTIVATE};
                        unsafe {
                            let handle = HWND(hwnd.0 as _);
                            let ex_style = GetWindowLongW(handle, GWL_EXSTYLE);
                            SetWindowLongW(handle, GWL_EXSTYLE, ex_style | WS_EX_NOACTIVATE.0 as i32);
                        }
                    }
                }
            }

            let handle = app.handle().clone();
            let resources_path = match app.path().resource_dir() {
                Ok(p) => p,
                Err(e) => {
                    log_to_file(&format!("CRITICAL ERROR: Failed to get resource_dir: {}", e));
                    PathBuf::new()
                }
            };
            log_to_file(&format!("Resolved Resources Path: {:?}", resources_path));

            let db_state = app.state::<DbState>();
            
            // 1. Load Items DB
            let items_possible_paths = [
                resources_path.join("resources").join("items_db.json"),
                resources_path.join("items_db.json"),
            ];
            log_to_file("Attempting to load Items DB...");
            for path in &items_possible_paths {
                log_to_file(&format!("Checking path: {:?}", path));
                if path.exists() {
                     match std::fs::read_to_string(path) {
                        Ok(json) => {
                            match serde_json::from_str::<Vec<RawItem>>(&json) {
                                Ok(raw_list) => {
                                    let items_list: Vec<ItemData> = raw_list.into_iter().map(ItemData::from).collect();
                                    let mut id_map = HashMap::new();
                                    let mut tag_set = std::collections::HashSet::new();
                                    for (index, item) in items_list.iter().enumerate() {
                                        id_map.insert(item.uuid.clone(), index);
                                        for tag in &item.processed_tags { tag_set.insert(tag.clone()); }
                                    }
                                    let mut unique_tags: Vec<String> = tag_set.into_iter().collect();
                                    unique_tags.sort();
                                    let count = items_list.len();
                                    let mut db = db_state.items.write().unwrap();
                                    db.list = items_list;
                                    db.id_map = id_map;
                                    db.unique_tags = unique_tags;
                                    log_to_file(&format!("[Init] Successfully loaded {} items from {:?}", count, path));
                                    break;
                                },
                                Err(e) => log_to_file(&format!("Error parsing items_db.json: {}", e)),
                            }
                        },
                        Err(e) => log_to_file(&format!("Error reading items_db.json: {}", e)),
                    }
                } else {
                    log_to_file("Path does not exist.");
                }
            }

            // 2. Load Skills DB
            let skills_possible_paths = [
                resources_path.join("resources").join("skills_db.json"),
                resources_path.join("skills_db.json"),
            ];
            log_to_file("Attempting to load Skills DB...");
            for path in &skills_possible_paths {
                log_to_file(&format!("Checking path: {:?}", path));
                if path.exists() {
                    match std::fs::read_to_string(path) {
                        Ok(json) => {
                            match serde_json::from_str::<Vec<RawItem>>(&json) {
                                Ok(raw_list) => {
                                    let skills_list: Vec<ItemData> = raw_list.into_iter().map(ItemData::from).collect();
                                    let mut id_map = HashMap::new();
                                    for (index, item) in skills_list.iter().enumerate() { id_map.insert(item.uuid.clone(), index); }
                                    let count = skills_list.len();
                                    let mut db = db_state.skills.write().unwrap();
                                    db.list = skills_list;
                                    db.id_map = id_map;
                                    log_to_file(&format!("[Init] Successfully loaded {} skills from {:?}", count, path));
                                    break;
                                },
                                Err(e) => log_to_file(&format!("Error parsing skills_db.json: {}", e)),
                            }
                        },
                        Err(e) => log_to_file(&format!("Error reading skills_db.json: {}", e)),
                    }
                } else {
                    log_to_file("Path does not exist.");
                }
            }

            // 3. Load Monster Image Map
            let monster_img_map_path = resources_path.join("resources").join("images_monster_map.json");
            log_to_file(&format!("Attempting to load Monster Image Map from {:?}", monster_img_map_path));
            let mut monster_img_lookup = HashMap::new();
            if let Ok(json) = std::fs::read_to_string(&monster_img_map_path) {
                if let Ok(serde_json::Value::Object(map)) = serde_json::from_str::<serde_json::Value>(&json) {
                    for (name, info) in map {
                        if let Some(out) = info.get("out").and_then(|v| v.as_str()) {
                            monster_img_lookup.insert(name, out.replace("\\", "/"));
                        }
                    }
                }
            }

            // 4. Load & Merge Monsters (Export First, then DB)
            let monsters_export_path = resources_path.join("resources").join("monsters_export.json");
            let monsters_db_path = resources_path.join("resources").join("monsters_db.json");
            
            let mut final_monsters = serde_json::Map::new();
            let mut export_by_day: HashMap<String, Vec<serde_json::Value>> = HashMap::new();
            
            // Move locks outside to be used by both Export and Fallback
            let items_db = db_state.items.read().unwrap();
            let skills_db = db_state.skills.read().unwrap();

            if monsters_export_path.exists() {
                if let Ok(json) = std::fs::read_to_string(&monsters_export_path) {
                    if let Ok(serde_json::Value::Array(exports)) = serde_json::from_str::<serde_json::Value>(&json) {
                        for m_val in exports {
                            if let Some(m_obj) = m_val.as_object() {
                                let level = m_obj.get("level").and_then(|v| v.as_u64()).unwrap_or(0);
                                let day_label = if level >= 10 { "Day 10+".to_string() } else { format!("Day {}", level) };
                                let name_zh = m_obj.get("name_cn").and_then(|v| v.as_str()).unwrap_or("未知");
                                let name_en = m_obj.get("name_en").and_then(|v| v.as_str()).unwrap_or("Unknown");
                                
                                let mut m_entry = serde_json::Map::new();
                                m_entry.insert("name".to_string(), serde_json::Value::String(name_en.to_string()));
                                m_entry.insert("name_zh".to_string(), serde_json::Value::String(name_zh.to_string()));
                                m_entry.insert("available".to_string(), serde_json::Value::String(day_label.clone()));
                                m_entry.insert("health".to_string(), m_obj.get("max_health").cloned().unwrap_or(0.into()));
                                
                                // 使用角色图路径（中文名.webp）
                                let img = format!("images_monster_char/{}.webp", name_zh);
                                m_entry.insert("image".to_string(), serde_json::Value::String(img));

                                // Loadout Items
                                let mut items_list = Vec::new();
                                if let Some(loadout) = m_obj.get("loadout_items").and_then(|v| v.as_array()) {
                                    for it_val in loadout {
                                        if let Some(it_obj) = it_val.as_object() {
                                            let id = it_obj.get("id").and_then(|v| v.as_str()).unwrap_or("");
                                            let tier_raw = it_obj.get("tier").and_then(|v| v.as_str()).unwrap_or("Bronze");
                                            let tier = tier_raw.split(" / ").next().unwrap_or(tier_raw);
                                            let it_name_cn = it_obj.get("name_cn").and_then(|v| v.as_str()).unwrap_or("未知");
                                            let it_name_en = it_obj.get("name_en").and_then(|v| v.as_str()).unwrap_or("Unknown");
                                            let it_size = it_obj.get("size").and_then(|v| v.as_str());
                                            
                                            let item_data = lookup_item(id, &items_db, &skills_db);
                                            items_list.push(construct_monster_sub_item(item_data, it_name_cn, it_name_en, tier, it_size));
                                        }
                                    }
                                }
                                m_entry.insert("items".to_string(), serde_json::Value::Array(items_list));

                                // Loadout Skills
                                let mut skills_list = Vec::new();
                                if let Some(loadout) = m_obj.get("loadout_skills").and_then(|v| v.as_array()) {
                                    for sk_val in loadout {
                                        if let Some(sk_obj) = sk_val.as_object() {
                                            let id = sk_obj.get("id").and_then(|v| v.as_str()).unwrap_or("");
                                            let tier_raw = sk_obj.get("tier").and_then(|v| v.as_str()).unwrap_or("Bronze");
                                            let tier = tier_raw.split(" / ").next().unwrap_or(tier_raw);
                                            let sk_name_cn = sk_obj.get("name_cn").and_then(|v| v.as_str()).unwrap_or("未知");
                                            let sk_name_en = sk_obj.get("name_en").and_then(|v| v.as_str()).unwrap_or("Unknown");
                                            let sk_size = sk_obj.get("size").and_then(|v| v.as_str());
                                            
                                            let skill_data = lookup_item(id, &items_db, &skills_db);
                                            skills_list.push(construct_monster_sub_item(skill_data, sk_name_cn, sk_name_en, tier, sk_size));
                                        }
                                    }
                                }
                                m_entry.insert("skills".to_string(), serde_json::Value::Array(skills_list));

                                export_by_day.entry(day_label).or_default().push(serde_json::Value::Object(m_entry));
                            }
                        }
                    }
                }
            }

            let mut db_by_day: HashMap<String, Vec<(String, serde_json::Value)>> = HashMap::new();
            if monsters_db_path.exists() {
                if let Ok(json) = std::fs::read_to_string(&monsters_db_path) {
                    if let Ok(serde_json::Value::Object(monsters)) = serde_json::from_str::<serde_json::Value>(&json) {
                        for (name, data) in monsters {
                            let day = data.get("available").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            if !day.is_empty() { db_by_day.entry(day).or_default().push((name, data)); }
                        }
                    }
                }
            }

            // Consolidate: Prioritize monsters_db, then supplement with monsters_export
            for i in 0..21 {
                let day_label = if i >= 10 { "Day 10+".to_string() } else { format!("Day {}", i) };
                
                // First check if Day exists in monsters_db
                if let Some(db_monsters) = db_by_day.get(&day_label) {
                    for (name, m) in db_monsters {
                        let mut enriched_m = m.clone();
                        if let Some(m_obj) = enriched_m.as_object_mut() {
                            // 强制设置图片路径（使用角色图），增加陷阱类前缀回退逻辑
                            let mut img_name = name.clone();
                            let img_path = resources_path.join("resources").join(format!("images_monster_char/{}.webp", img_name));
                            if !img_path.exists() {
                                // 1. 尝试去除 _Day 序列后缀 (如 快乐杰克南瓜_Day8 -> 快乐杰克南瓜)
                                if let Some(idx) = img_name.find("_Day") {
                                    let base = &img_name[0..idx];
                                    if resources_path.join("resources").join(format!("images_monster_char/{}.webp", base)).exists() {
                                        img_name = base.to_string();
                                    }
                                }
                                
                                // 2. 尝试剥离陷阱类前缀 (如 毒素 吹箭枪陷阱 -> 吹箭枪陷阱)
                                if !resources_path.join("resources").join(format!("images_monster_char/{}.webp", img_name)).exists() {
                                    if let Some(space_pos) = img_name.rfind(' ') {
                                        let base_name = &img_name[space_pos + 1..];
                                        let base_path = resources_path.join("resources").join(format!("images_monster_char/{}.webp", base_name));
                                        if base_path.exists() {
                                            img_name = base_name.to_string();
                                        }
                                    }
                                }
                            }
                            let img_rel = format!("images_monster_char/{}.webp", img_name);
                            m_obj.insert("image".to_string(), serde_json::Value::String(img_rel));
                            
                            // Enrich items
                            if let Some(items) = m_obj.get_mut("items").and_then(|v| v.as_array_mut()) {
                                for item_val in items {
                                    if let Some(item_obj) = item_val.as_object_mut() {
                                        let id = item_obj.get("id").and_then(|v| v.as_str()).unwrap_or("");
                                        let name_cn = item_obj.get("name").and_then(|v| v.as_str()).unwrap_or("");
                                        
                                        // 如果 id 为空，尝试通过中文名查找
                                        let found = if id.is_empty() && !name_cn.is_empty() {
                                            lookup_item_by_name(name_cn, &items_db, &skills_db)
                                        } else {
                                            lookup_item(id, &items_db, &skills_db)
                                        };
                                        
                                        if let Some(found_item) = found {
                                            // 更新 id
                                            if id.is_empty() {
                                                item_obj.insert("id".to_string(), serde_json::Value::String(found_item.uuid.clone()));
                                            }
                                            // 注入升级数据
                                            item_obj.insert("cooldown_tiers".to_string(), serde_json::Value::String(found_item.cooldown_tiers.clone()));
                                            item_obj.insert("available_tiers".to_string(), serde_json::Value::String(found_item.available_tiers.clone()));
                                            item_obj.insert("damage_tiers".to_string(), serde_json::Value::String(found_item.damage_tiers.clone()));
                                            item_obj.insert("heal_tiers".to_string(), serde_json::Value::String(found_item.heal_tiers.clone()));
                                            item_obj.insert("shield_tiers".to_string(), serde_json::Value::String(found_item.shield_tiers.clone()));

                                            // 强制使用 id.webp 格式作为图片路径
                                            let webp_img = format!("images/{}.webp", found_item.uuid);
                                            item_obj.insert("image".to_string(), serde_json::Value::String(webp_img));
                                            
                                            // 更新 size
                                            if let Some(s) = found_item.size {
                                                let norm = s.split(" / ").next().unwrap_or(&s).to_string();
                                                item_obj.insert("size".to_string(), serde_json::Value::String(norm));
                                            }
                                        }
                                    }
                                }
                            }
                            // Enrich skills
                            if let Some(skills) = m_obj.get_mut("skills").and_then(|v| v.as_array_mut()) {
                                for skill_val in skills {
                                    if let Some(skill_obj) = skill_val.as_object_mut() {
                                        let id = skill_obj.get("id").and_then(|v| v.as_str()).unwrap_or("");
                                        let name_cn = skill_obj.get("name").and_then(|v| v.as_str()).unwrap_or("");
                                        
                                        // 如果 id 为空，尝试通过中文名查找
                                        let found = if id.is_empty() && !name_cn.is_empty() {
                                            lookup_item_by_name(name_cn, &items_db, &skills_db)
                                        } else {
                                            lookup_item(id, &items_db, &skills_db)
                                        };
                                        
                                        if let Some(found_skill) = found {
                                            // 更新 id
                                            if id.is_empty() {
                                                skill_obj.insert("id".to_string(), serde_json::Value::String(found_skill.uuid.clone()));
                                            }
                                            // 注入升级数据
                                            skill_obj.insert("cooldown_tiers".to_string(), serde_json::Value::String(found_skill.cooldown_tiers.clone()));
                                            skill_obj.insert("available_tiers".to_string(), serde_json::Value::String(found_skill.available_tiers.clone()));
                                            skill_obj.insert("damage_tiers".to_string(), serde_json::Value::String(found_skill.damage_tiers.clone()));
                                            skill_obj.insert("heal_tiers".to_string(), serde_json::Value::String(found_skill.heal_tiers.clone()));
                                            skill_obj.insert("shield_tiers".to_string(), serde_json::Value::String(found_skill.shield_tiers.clone()));

                                            // 强制使用 id.webp 格式作为图片路径
                                            let webp_img = format!("images/{}.webp", found_skill.uuid);
                                            skill_obj.insert("image".to_string(), serde_json::Value::String(webp_img));
                                            
                                            // 更新 size
                                            if let Some(s) = found_skill.size {
                                                let norm = s.split(" / ").next().unwrap_or(&s).to_string();
                                                skill_obj.insert("size".to_string(), serde_json::Value::String(norm));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        final_monsters.insert(name.clone(), enriched_m);
                    }
                } 
                // Then supplement with Export if Day doesn't exist in DB (or if you want to merge, but user said "switch back")
                else if let Some(exports) = export_by_day.get(&day_label) {
                    for m in exports {
                        if let Some(name_zh) = m.get("name_zh").and_then(|v| v.as_str()) {
                            final_monsters.insert(name_zh.to_string(), m.clone());
                        }
                    }
                }
            }
            let monster_count = final_monsters.len();
            *db_state.monsters.write().unwrap() = final_monsters;
            println!("[Init] Successfully consolidated {} monsters (Export prioritized by day)", monster_count);

            // Log Monitor Thread
            let thread_items_db = db_state.items.clone();
            let thread_skills_db = db_state.skills.clone();
            let log_handle = handle.clone();
            
            thread::spawn(move || {
                let handle = log_handle;
                let log_path = get_log_path();
                let prev_path = get_prev_log_path();
                
                let re_purchase = Regex::new(r"Card Purchased: InstanceId:\s*(?P<iid>[^ ]+)\s*-\s*TemplateId\s*(?P<tid>[^ ]+)(?:.*Target:(?P<tgt>[^ ]+))?(?:.*Section(?P<sec>[^ ]+))?").unwrap();
                let re_id = Regex::new(r"ID: \[(?P<id>[^\]]+)\]").unwrap();
                let re_owner = Regex::new(r"- Owner: \[(?P<val>[^\]]+)\]").unwrap();
                let re_section = Regex::new(r"- Section: \[(?P<val>[^\]]+)\]").unwrap();

                let re_item_id = Regex::new(r"itm_[A-Za-z0-9_-]+").unwrap();
                let re_sold = Regex::new(r"Sold Card\s+(?P<iid>itm_[^ ]+)").unwrap();
                let re_removed = Regex::new(r"Successfully removed item\s+(?P<iid>itm_[^ ]+)").unwrap();
                let re_moved_to = Regex::new(r"Successfully moved card\s+(?P<iid>itm_[^ ]+)\s+to\s+(?P<tgt>[^ ]+)").unwrap();
                
                // Initialize state from cache
                let _cache_path = get_cache_path();
                let _has_cache = _cache_path.exists();
                let state_init = load_state();
                
                let mut inst_to_temp = state_init.inst_to_temp;
                let mut current_hand = state_init.current_hand;
                let mut current_stash = state_init.current_stash;
                let mut current_day = state_init.day;
                
                let mut last_file_size = if log_path.exists() {
                    std::fs::metadata(&log_path).map(|m| m.len()).unwrap_or(0)
                } else {
                    0
                };
                
                let mut last_iid = String::new();
                let mut cur_owner = String::new();
                let mut in_pvp = false;
                let mut hour_count: u32 = 0;
                let mut is_sync = false;

                // --- Initial Sync: Replay Logs to catch up with current state ---
                println!("[LogMonitor] Initializing state from logs...");
                
                // Clear state for fresh scan (we'll recover inst_to_temp from logs too)
                current_hand.clear();
                current_stash.clear();
                // inst_to_temp.clear(); // We keep cache as fallback, but logs will overwrite

                let files_to_process = vec![prev_path, log_path.clone()];
                for path in files_to_process {
                    if !path.exists() { continue; }
                    if let Ok(file) = File::open(&path) {
                        let reader = BufReader::new(file);
                        for line in reader.lines() {
                            if let Ok(l) = line {
                                let trimmed = l.trim();
                                
                                // Reset everything if we see a new run start
                                if trimmed.contains("NetMessageRunInitialized") {
                                    current_day = 1; in_pvp = false; hour_count = 0;
                                    inst_to_temp.clear();
                                    current_hand.clear();
                                    current_stash.clear();
                                    is_sync = false;
                                }

                                if trimmed.contains("to [PVPCombatState]") { in_pvp = true; }
                                if in_pvp && trimmed.contains("State changed") && (trimmed.contains("to [ChoiceState]") || trimmed.contains("to [LevelUpState]")) {
                                    current_day = current_day.saturating_add(1); in_pvp = false; hour_count = 0;
                                }
                                if trimmed.contains("State changed from [ChoiceState] to [") {
                                    if !trimmed.contains("to [ChoiceState]") && !trimmed.contains("to [PVPCombatState]") {
                                        hour_count = hour_count.saturating_add(1);
                                        if hour_count >= 10 { current_day = current_day.saturating_add(1); hour_count = 0; }
                                    }
                                }

                                if let Some(cap) = re_purchase.captures(trimmed) {
                                    let iid = cap["iid"].to_string();
                                    inst_to_temp.insert(iid.clone(), cap["tid"].to_string());
                                    let mut section = cap.name("sec").map(|s| s.as_str().to_string());
                                    if section.as_deref().unwrap_or("") == "" {
                                        if let Some(tgt) = cap.name("tgt").map(|t| t.as_str()) {
                                            if tgt.contains("PlayerStorageSocket") { section = Some("Stash".to_string()); }
                                            else if tgt.contains("PlayerSocket") { section = Some("Player".to_string()); }
                                        }
                                    }
                                    if let Some(s) = section {
                                        if s == "Player" || s == "Hand" { current_hand.insert(iid); }
                                        else if s == "Stash" || s == "Storage" || s == "PlayerStorage" { current_stash.insert(iid); }
                                    }
                                }
                                if let Some(cap) = re_moved_to.captures(trimmed) {
                                    let iid = cap["iid"].to_string();
                                    if cap["tgt"].contains("StorageSocket") {
                                        current_stash.insert(iid.clone()); current_hand.remove(&iid);
                                    } else if cap["tgt"].contains("Socket") {
                                        current_hand.insert(iid.clone()); current_stash.remove(&iid);
                                    }
                                }
                                if let Some(cap) = re_sold.captures(trimmed) {
                                    let iid = cap["iid"].to_string(); 
                                    current_hand.remove(&iid); current_stash.remove(&iid);
                                }
                                if let Some(cap) = re_removed.captures(trimmed) {
                                    let iid = cap["iid"].to_string(); 
                                    current_hand.remove(&iid); current_stash.remove(&iid);
                                }
                                if trimmed.contains("Cards Disposed:") {
                                    for mat in re_item_id.find_iter(trimmed) {
                                        let iid = mat.as_str().to_string(); 
                                        current_hand.remove(&iid); current_stash.remove(&iid);
                                    }
                                }
                                if trimmed.contains("Cards Spawned:") || trimmed.contains("Cards Dealt:") || trimmed.contains("NetMessageGameStateSync") { 
                                    is_sync = true; 
                                }
                                if is_sync {
                                    if let Some(cap) = re_id.captures(trimmed) { last_iid = cap["id"].to_string(); }
                                    else if let Some(cap) = re_owner.captures(trimmed) { cur_owner = cap["val"].to_string(); }
                                    else if let Some(cap) = re_section.captures(trimmed) {
                                        if !last_iid.is_empty() && &cur_owner == "Player" && last_iid.starts_with("itm_") {
                                            let sec_val = &cap["val"];
                                            if sec_val == "Hand" || sec_val == "Player" { 
                                                current_hand.insert(last_iid.clone()); 
                                                current_stash.remove(&last_iid);
                                            }
                                            else if sec_val == "Stash" || sec_val == "Storage" || sec_val == "PlayerStorage" { 
                                                current_stash.insert(last_iid.clone()); 
                                                current_hand.remove(&last_iid);
                                            }
                                            else {
                                                current_hand.remove(&last_iid); 
                                                current_stash.remove(&last_iid);
                                            }
                                        }
                                        last_iid.clear(); cur_owner.clear();
                                    }
                                    else if trimmed.contains("Finished processing") { is_sync = false; }
                                }
                            }
                        }
                    }
                }

                save_state(&PersistentState {
                    day: current_day,
                    inst_to_temp: inst_to_temp.clone(),
                    current_hand: current_hand.clone(),
                    current_stash: current_stash.clone(),
                    detection_hotkey: load_state().detection_hotkey,
                });

                // Initial UI Sync after loading/backfilling
                let init_handle = handle.clone();
                let init_items_db = thread_items_db.clone();
                let init_skills_db = thread_skills_db.clone();
                let init_hand = current_hand.clone();
                let init_stash = current_stash.clone();
                let init_map = inst_to_temp.clone();
                let init_day = current_day;
                
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
                    let _ = init_handle.emit("day-update", init_day);
                    let items_db = init_items_db.read().unwrap();
                    let skills_db = init_skills_db.read().unwrap();
                    let hand_items = init_hand.iter()
                        .filter_map(|iid| init_map.get(iid))
                        .filter_map(|tid| lookup_item(tid, &items_db, &skills_db))
                        .collect();
                    let stash_items = init_stash.iter()
                        .filter_map(|iid| init_map.get(iid))
                        .filter_map(|tid| lookup_item(tid, &items_db, &skills_db))
                        .collect();
                    let all_tags = items_db.unique_tags.clone();
                    let _ = init_handle.emit("sync-items", SyncPayload { hand_items, stash_items, all_tags });
                });

                loop {
                    if !log_path.exists() { thread::sleep(time::Duration::from_secs(2)); continue; }
                    let current_file_size = match std::fs::metadata(&log_path) {
                        Ok(meta) => meta.len(),
                        Err(e) => {
                            log_to_file(&format!("Error reading log metadata: {}. Retrying...", e));
                            thread::sleep(time::Duration::from_secs(1));
                            continue;
                        }
                    };
                    
                    if current_file_size < last_file_size {
                        println!("[LogMonitor] Log truncated, resetting state...");
                        inst_to_temp.clear();
                        current_hand.clear();
                        current_stash.clear();
                        current_day = 1;
                        is_sync = false;
                        last_file_size = 0;
                        save_state(&PersistentState { 
                            day: current_day, 
                            inst_to_temp: inst_to_temp.clone(), 
                            current_hand: current_hand.clone(), 
                            current_stash: current_stash.clone(),
                            // Preserve hotkey setting from disk logic if necessary, but here we are in a reset state.
                            // However, we shouldn't lose the hotkey setting just because the log was truncated.
                            // We should load the *existing* state to get the hotkey, or just set it to default if we can't.
                            // Better: read the current state from disk before overwriting.
                            detection_hotkey: load_state().detection_hotkey, 
                        });
                    }
                    
                    if current_file_size > last_file_size {
                        let mut f = match File::open(&log_path) {
                            Ok(file) => file,
                            Err(e) => {
                                log_to_file(&format!("Failed to open log file for reading: {}", e));
                                thread::sleep(time::Duration::from_secs(1));
                                continue;
                            }
                        };
                        let _ = f.seek(SeekFrom::Start(last_file_size));
                        let reader = BufReader::new(f);
                        
                        let mut changed = false;
                        let mut day_changed = false;
                        for line in reader.lines() {
                            let l = if let Ok(l) = line { l } else { continue };
                            let trimmed = l.trim();

                            // Day Detection Logic
                            if trimmed.contains("NetMessageRunInitialized") {
                                current_day = 1; in_pvp = false; hour_count = 0; day_changed = true;
                                inst_to_temp.clear();
                                current_hand.clear();
                                current_stash.clear();
                                changed = true;
                            }
                            
                            // Tracks PVP state
                            if trimmed.contains("to [PVPCombatState]") { 
                                in_pvp = true; 
                            }
                            
                            // Day increment: The most reliable trigger is the transition back to Map (ChoiceState) after a PVP fight.
                            if in_pvp && trimmed.contains("State changed") && (trimmed.contains("to [ChoiceState]") || trimmed.contains("to [LevelUpState]")) {
                                current_day = current_day.saturating_add(1);
                                in_pvp = false; 
                                hour_count = 0; 
                                day_changed = true;
                                println!("[DayMonitor] Day increased to {} after PVP completion", current_day);
                            }

                            // Optional: PVE-only day detection (less common, but as a fallback)
                            if trimmed.contains("State changed from [ChoiceState] to [") {
                                if !trimmed.contains("to [ChoiceState]") && !trimmed.contains("to [PVPCombatState]") {
                                    hour_count = hour_count.saturating_add(1);
                                    if hour_count >= 10 { 
                                        current_day = current_day.saturating_add(1);
                                        hour_count = 0;
                                        day_changed = true;
                                        println!("[DayMonitor] Day increased to {} after 10 encounters", current_day);
                                    }
                                }
                            }

                            if let Some(cap) = re_purchase.captures(trimmed) {
                                let iid = cap["iid"].to_string();
                                inst_to_temp.insert(iid.clone(), cap["tid"].to_string());
                                
                                let mut section = cap.name("sec").map(|s| s.as_str().to_string());
                                let target = cap.name("tgt").map(|t| t.as_str());

                                // Fallback: Derive section from Target if Section is missing or ambiguous
                                if section.as_deref().unwrap_or("") == "" {
                                    if let Some(tgt) = target {
                                        if tgt.contains("PlayerStorageSocket") { section = Some("Stash".to_string()); }
                                        else if tgt.contains("PlayerSocket") { section = Some("Player".to_string()); }
                                    }
                                }

                                if let Some(s) = section {
                                    if s == "Player" || s == "Hand" { 
                                        current_hand.insert(iid); changed = true; 
                                    }
                                    else if s == "Stash" || s == "Storage" || s == "PlayerStorage" { 
                                        current_stash.insert(iid); changed = true; 
                                    }
                                }
                            }

                            if let Some(cap) = re_moved_to.captures(trimmed) {
                                let iid = cap["iid"].to_string();
                                let tgt = &cap["tgt"];
                                if tgt.contains("StorageSocket") {
                                    current_stash.insert(iid.clone());
                                    current_hand.remove(&iid);
                                    changed = true;
                                } else if tgt.contains("Socket") { // General Socket_X
                                    current_hand.insert(iid.clone());
                                    current_stash.remove(&iid);
                                    changed = true;
                                }
                            }

                            if let Some(cap) = re_sold.captures(trimmed) {
                                let iid = cap["iid"].to_string();
                                if current_hand.remove(&iid) || current_stash.remove(&iid) {
                                    changed = true;
                                }
                            }

                            if let Some(cap) = re_removed.captures(trimmed) {
                                let iid = cap["iid"].to_string();
                                if current_hand.remove(&iid) || current_stash.remove(&iid) {
                                    changed = true;
                                }
                            }

                            if trimmed.contains("Cards Disposed:") {
                                for mat in re_item_id.find_iter(trimmed) {
                                    let iid = mat.as_str().to_string();
                                    if current_hand.remove(&iid) || current_stash.remove(&iid) {
                                        changed = true;
                                    }
                                }
                            }

                            if trimmed.contains("Cards Spawned:") || trimmed.contains("Cards Dealt:") || trimmed.contains("NetMessageGameStateSync") {
                                is_sync = true;
                            } else if trimmed.contains("Successfully moved card to:") {
                                is_sync = true;
                            }

                            if is_sync {
                                if let Some(cap) = re_id.captures(trimmed) { last_iid = cap["id"].to_string(); }
                                else if let Some(cap) = re_owner.captures(trimmed) { cur_owner = cap["val"].to_string(); }
                                else if let Some(cap) = re_section.captures(trimmed) {
                                    if !last_iid.is_empty() && &cur_owner == "Player" {
                                        if last_iid.starts_with("itm_") {
                                            let sec_val = &cap["val"];
                                            if sec_val == "Hand" || sec_val == "Player" { 
                                                current_hand.insert(last_iid.clone());
                                                current_stash.remove(&last_iid);
                                            }
                                            else if sec_val == "Stash" || sec_val == "Storage" || sec_val == "PlayerStorage" { 
                                                current_stash.insert(last_iid.clone());
                                                current_hand.remove(&last_iid);
                                            }
                                            else {
                                                current_hand.remove(&last_iid);
                                                current_stash.remove(&last_iid);
                                            }
                                            changed = true;
                                        }
                                    }
                                    // Reset for next block
                                    last_iid.clear();
                                    cur_owner.clear();
                                }
                                else if trimmed.contains("Finished processing") {
                                    is_sync = false;
                                    changed = true;
                                }
                            }
                        }

                        if changed || day_changed {
                            if day_changed {
                                let _ = handle.emit("day-update", current_day);
                            }
                            let items_db = thread_items_db.read().unwrap();
                            let skills_db = thread_skills_db.read().unwrap();
                            let hand_items = current_hand.iter()
                                .filter_map(|iid| inst_to_temp.get(iid))
                                .filter_map(|tid| lookup_item(tid, &items_db, &skills_db))
                                .collect();
                            let stash_items = current_stash.iter()
                                .filter_map(|iid| inst_to_temp.get(iid))
                                .filter_map(|tid| lookup_item(tid, &items_db, &skills_db))
                                .collect();
                            
                            let all_tags = items_db.unique_tags.clone();
                            let _ = handle.emit("sync-items", SyncPayload { hand_items, stash_items, all_tags });
                            
                            save_state(&PersistentState {
                                day: current_day,
                                inst_to_temp: inst_to_temp.clone(),
                                current_hand: current_hand.clone(),
                                current_stash: current_stash.clone(),
                                detection_hotkey: load_state().detection_hotkey,
                            });
                        }
                        last_file_size = current_file_size;
                    }
                    thread::sleep(time::Duration::from_millis(500));
                }
            });

            // 启动鼠标监听线程 (识别怪物)
            let handle_mouse = handle.clone();
            std::thread::spawn(move || {
                let mut last_trigger = time::Instant::now();
                loop {
                    // 读取配置的按键
                    let hotkey = {
                        let state = load_state();
                        state.detection_hotkey.unwrap_or(VK_RBUTTON.0 as i32)
                    };
                    
                    unsafe {
                        // 检测按键按下
                        if (GetAsyncKeyState(hotkey) as i16) < 0 {
                            if last_trigger.elapsed() > time::Duration::from_millis(500) {
                                last_trigger = time::Instant::now();
                                
                                log_to_file("Hotkey pressed, starting scan...");

                                // 尝试识别
                                match scan_and_identify_monster_at_mouse() {
                                    Ok(Some(monster_name)) => {
                                        log_to_file(&format!("Success! Valid monster found: {}", monster_name));
                                        
                                        // 关键修复：处理陷阱类并列名称
                                        let lookup_name = if monster_name.contains('|') {
                                            monster_name.split('|').next().unwrap_or(&monster_name).to_string()
                                        } else {
                                            monster_name.clone()
                                        };

                                        if let Some(db_state) = handle_mouse.try_state::<DbState>() {
                                            if let Ok(monsters) = db_state.monsters.read() {
                                                // 首先尝试通过 Key 获取 Entry，如果不行，尝试遍历匹配 name_zh
                                                let entry_opt = monsters.get(&lookup_name)
                                                    .or_else(|| {
                                                        monsters.values().find(|v| {
                                                            v.get("name_zh").and_then(|nz| nz.as_str()) == Some(&lookup_name)
                                                        })
                                                    });

                                                if let Some(entry) = entry_opt {
                                                    let target_name_zh = entry.get("name_zh").and_then(|v| v.as_str()).unwrap_or(&monster_name);
                                                    let mut candidate_days: Vec<u32> = Vec::new();
                                                    
                                                    // 寻找所有具有相同中文名的怪物条目（解决同名不同天数问题）
                                                    for (_, v) in monsters.iter() {
                                                        if let Some(n_zh) = v.get("name_zh").and_then(|val| val.as_str()) {
                                                            if n_zh == target_name_zh {
                                                                if let Some(d_str) = v.get("available").and_then(|val| val.as_str()) {
                                                                    if d_str.starts_with("Day ") {
                                                                        let num_part = d_str[4..].trim_end_matches('+');
                                                                        if let Ok(d_num) = num_part.parse::<u32>() {
                                                                            candidate_days.push(d_num);
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                    
                                                    if !candidate_days.is_empty() {
                                                        candidate_days.sort();
                                                        candidate_days.dedup();

                                                        let current_day = load_state().day;
                                                        let target_day = if candidate_days.contains(&current_day) {
                                                            current_day
                                                        } else {
                                                            *candidate_days.iter().min_by_key(|&&d| (d as i32 - current_day as i32).abs()).unwrap()
                                                        };

                                                        match handle_mouse.emit("auto-jump-to-monster", serde_json::json!({
                                                            "day": target_day,
                                                            "monster_name": monster_name // 使用包含 | 的原始名称
                                                        })) {
                                                            Ok(_) => {},
                                                            Err(e) => println!("Failed to emit auto-jump-to-monster: {}", e),
                                                        }
                                                        
                                                        let mut state = load_state();
                                                        state.day = target_day;
                                                        save_state(&state);
                                                        
                                                        println!("自动跳转到 Day {} (识别: {}, 候选天数: {:?})", target_day, lookup_name, candidate_days);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    Ok(None) => {
                                        // Scan successful but no monster found
                                        log_to_file("Scan complete, no monster matched.");
                                    }
                                    Err(e) => {
                                        let err_msg = format!("Monster Scan Failed: {}", e);
                                        println!("[Error] {}", err_msg);
                                        log_to_file(&format!("Error: {}", err_msg));
                                        // Emit error to frontend for toast
                                        let _ = handle_mouse.emit("scan-error", e);
                                    }
                                }
                            }
                        }
                    }
                    thread::sleep(time::Duration::from_millis(100));
                }
            });

            log_to_file("Setup complete. Initializing main loop...");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_all_monsters,
            recognize_monsters_from_screenshot,
            get_template_loading_progress,
            get_current_day,
            update_day,
            get_detection_hotkey,
            set_detection_hotkey,
            start_template_loading,
            crate::monster_recognition::check_opencv_load, 
            // clear_monster_cache,
            restore_game_focus
        ])
        .run(tauri::generate_context!())
        .map_err(|e| {
            log_to_file(&format!("FATAL: Error while running tauri application: {}", e));
            e
        })
        .expect("error while running tauri application");
}
