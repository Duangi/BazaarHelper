use std::sync::{Arc, RwLock};
use tauri::{State, Manager, Emitter};
use serde::{Serialize, Deserialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use regex::Regex;
use std::io::{Read, BufRead, BufReader, Seek, SeekFrom};
use std::fs::File;
use std::{thread, time};
use tokio;

pub mod monster_recognition;

// --- Data Models ---
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PersistentState {
    pub day: u32,
    pub inst_to_temp: HashMap<String, String>,
    pub current_hand: HashSet<String>,
    pub current_stash: HashSet<String>,
}

impl Default for PersistentState {
    fn default() -> Self {
        Self {
            day: 1,
            inst_to_temp: HashMap::new(),
            current_hand: HashSet::new(),
            current_stash: HashSet::new(),
        }
    }
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Enchantment {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ItemData {
    pub id: Option<String>,
    pub name: String,
    pub name_zh: String,
    pub image: String,
    pub enchantments: Option<Vec<Enchantment>>,
    pub display_img: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TierInfo {
    pub description: Vec<String>,
    pub extra_description: Vec<String>,
    pub cd: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MonsterSubItem {
    pub name: String,
    pub name_en: Option<String>,
    pub tier: Option<String>,
    pub current_tier: Option<String>,
    pub tags: Option<Vec<String>>,
    pub tiers: Option<HashMap<String, Option<TierInfo>>>,
    pub image: Option<String>,
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
    pub image: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SyncPayload {
    pub hand_items: Vec<ItemData>,
    pub stash_items: Vec<ItemData>,
}

pub struct DbState {
    pub items: Arc<RwLock<HashMap<String, ItemData>>>,
    pub monsters: Arc<RwLock<HashMap<String, MonsterData>>>,
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
    let resources_path = app.path().resource_dir().unwrap();
    let res_dir = resources_path.join("resources");
    let cache_dir = get_cache_path().parent().unwrap().to_path_buf();
    
    // 异步加载
    tauri::async_runtime::spawn(async move {
        let _ = monster_recognition::preload_templates_async(res_dir, cache_dir).await;
    });
    
    Ok(())
}

#[tauri::command]
#[allow(dead_code)]
async fn clear_monster_cache() -> Result<(), String> {
    let cache_dir = get_cache_path().parent().unwrap().to_path_buf();
    let cache_file = cache_dir.join("monster_features.bin");
    if cache_file.exists() {
        std::fs::remove_file(cache_file).map_err(|e| e.to_string())?;
    }
    Ok(())
}

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

fn lookup_item(tid: &str, db: &HashMap<String, ItemData>) -> Option<ItemData> {
    db.get(tid).map(|item| {
        let mut cloned = item.clone();
        cloned.id = Some(tid.to_string());
        cloned
    })
}

// --- Commands ---
#[tauri::command]
fn get_all_monsters(state: State<'_, DbState>) -> Result<HashMap<String, MonsterData>, String> {
    let db = state.monsters.read().map_err(|_| "DB Busy")?;
    Ok(db.clone())
}

#[tauri::command]
fn recognize_monsters_from_screenshot(day: Option<u32>) -> Result<Vec<monster_recognition::MonsterRecognitionResult>, String> {
    let day_filter = day.map(|d| if d > 10 { "Day 10+".to_string() } else { format!("Day {}", d) });
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
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .manage(DbState {
            items: Arc::new(RwLock::new(HashMap::new())),
            monsters: Arc::new(RwLock::new(HashMap::new())),
        })
        .setup(|app| {
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
            let resources_path = app.path().resource_dir().unwrap();
            let item_db_instance = app.state::<DbState>().items.clone();
            
            // Initial DB Load
            let db_state = app.state::<DbState>();
            
            // 尝试多个可能的路径
            let monsters_possible_paths = [
                resources_path.join("resources").join("monsters_db.json"),
                resources_path.join("monsters_db.json"),
            ];
            
            for path in &monsters_possible_paths {
                if path.exists() {
                    match std::fs::read_to_string(path) {
                        Ok(json) => {
                            match serde_json::from_str::<HashMap<String, MonsterData>>(&json) {
                                Ok(monsters) => {
                                    println!("[Init] Successfully loaded {} monsters from {:?}", monsters.len(), path);
                                    *db_state.monsters.write().unwrap() = monsters;
                                    break;
                                }
                                Err(e) => println!("[Error] Failed to parse monsters_db.json at {:?}: {}", path, e),
                            }
                        }
                        Err(e) => println!("[Error] Failed to read monsters_db.json at {:?}: {}", path, e),
                    }
                }
            }

            let items_possible_paths = [
                resources_path.join("resources").join("items_db.json"),
                resources_path.join("items_db.json"),
            ];
            for path in &items_possible_paths {
                if path.exists() {
                    match std::fs::read_to_string(path) {
                        Ok(json) => {
                            match serde_json::from_str::<HashMap<String, ItemData>>(&json) {
                                Ok(items) => {
                                    println!("[Init] Successfully loaded {} items from {:?}", items.len(), path);
                                    *db_state.items.write().unwrap() = items;
                                    break;
                                }
                                Err(e) => println!("[Error] Failed to parse items_db.json at {:?}: {}", path, e),
                            }
                        }
                        Err(e) => println!("[Error] Failed to read items_db.json at {:?}: {}", path, e),
                    }
                }
            }

            // Log Monitor Thread
            thread::spawn(move || {
                let log_path = get_log_path();
                let re_purchase = Regex::new(r"Card Purchased: InstanceId:\s*(?P<iid>[^ ]+)\s*-\s*TemplateId\s*(?P<tid>[^ ]+)(?:.*Target:(?P<tgt>[^ ]+))?(?:.*Section(?P<sec>[^ ]+))?").unwrap();
                let re_id = Regex::new(r"ID: \[(?P<id>[^\]]+)\]").unwrap();
                let re_owner = Regex::new(r"- Owner: \[(?P<val>[^\]]+)\]").unwrap();
                let re_section = Regex::new(r"- Section: \[(?P<val>[^\]]+)\]").unwrap();

                let re_item_id = Regex::new(r"itm_[A-Za-z0-9_-]+").unwrap();
                let re_sold = Regex::new(r"Sold Card\s+(?P<iid>itm_[^ ]+)").unwrap();
                let re_removed = Regex::new(r"Successfully removed item\s+(?P<iid>itm_[^ ]+)").unwrap();
                let re_moved_to = Regex::new(r"Successfully moved card\s+(?P<iid>itm_[^ ]+)\s+to\s+(?P<tgt>[^ ]+)").unwrap();
                
                // Load from cache or defaults
                let state_init = load_state();
                let mut inst_to_temp = state_init.inst_to_temp;
                let mut current_hand = state_init.current_hand;
                let mut current_stash = state_init.current_stash;
                let mut current_day = state_init.day;

                // Emit initial state
                let init_handle = handle.clone();
                let init_db = item_db_instance.clone();
                let init_hand = current_hand.clone();
                let init_stash = current_stash.clone();
                let init_map = inst_to_temp.clone();
                let init_day = current_day;
                
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                    let _ = init_handle.emit("day-update", init_day);
                    let db = init_db.read().unwrap();
                    let hand_items = init_hand.iter()
                        .filter_map(|iid| init_map.get(iid))
                        .filter_map(|tid| lookup_item(tid, &db))
                        .collect();
                    let stash_items = init_stash.iter()
                        .filter_map(|iid| init_map.get(iid))
                        .filter_map(|tid| lookup_item(tid, &db))
                        .collect();
                    let _ = init_handle.emit("sync-items", SyncPayload { hand_items, stash_items });
                });

                let mut last_file_size = 0;
                let mut is_sync = false;
                let mut last_iid = String::new();
                let mut cur_owner = String::new();
                let mut in_pvp = false;
                let mut hour_count: u32 = 0;

                loop {
                    if !log_path.exists() { thread::sleep(time::Duration::from_secs(2)); continue; }
                    let meta = std::fs::metadata(&log_path).unwrap();
                    let current_file_size = meta.len();
                    
                    if current_file_size < last_file_size {
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
                            current_stash: current_stash.clone() 
                        });
                    }
                    
                    if current_file_size > last_file_size {
                        let mut f = File::open(&log_path).unwrap();
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
                                if section.is_none() || section.as_ref().unwrap() == "" {
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
                            let db = item_db_instance.read().unwrap();
                            let hand_items = current_hand.iter()
                                .filter_map(|iid| inst_to_temp.get(iid))
                                .filter_map(|tid| lookup_item(tid, &db))
                                .collect();
                            let stash_items = current_stash.iter()
                                .filter_map(|iid| inst_to_temp.get(iid))
                                .filter_map(|tid| lookup_item(tid, &db))
                                .collect();
                            
                            let _ = handle.emit("sync-items", SyncPayload { hand_items, stash_items });
                            
                            save_state(&PersistentState {
                                day: current_day,
                                inst_to_temp: inst_to_temp.clone(),
                                current_hand: current_hand.clone(),
                                current_stash: current_stash.clone(),
                            });
                        }
                        last_file_size = current_file_size;
                    }
                    thread::sleep(time::Duration::from_millis(500));
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_all_monsters,
            recognize_monsters_from_screenshot,
            get_template_loading_progress,
            get_current_day,
            update_day,
            start_template_loading,
            clear_monster_cache,
            restore_game_focus
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
