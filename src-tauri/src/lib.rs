use std::sync::{Arc, RwLock};
use tauri::{State, Manager, Emitter};
use serde::{Serialize, Deserialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use regex::Regex;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::fs::File;
use std::{thread, time};

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
    pub tiers: HashMap<String, Option<TierInfo>>,
    pub image: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MonsterData {
    pub name: String,
    pub name_zh: String,
    pub available: String,
    pub health: Option<u32>,
    pub level: Option<u32>,
    pub skills: Vec<MonsterSubItem>,
    pub items: Vec<MonsterSubItem>,
    pub image: String,
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
        if let Ok(content) = std::fs::read_to_string(&log_path) {
             if let Some(day) = calculate_day_from_log(&content, hours, retro) {
                 return Ok(day);
             }
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

fn calculate_day_from_log(content: &str, hours: u32, retro: bool) -> Option<u32> {
    let start_pos = if retro { content.rfind("NetMessageRunInitialized").unwrap_or(0) } else { 0 };
    let slice = &content[start_pos..];
    let mut current_day: u32 = 0;
    let mut in_pvp = false;
    let mut hour_count: u32 = 0;

    for line in slice.lines() {
        let l = line.trim();
        if l.contains("NetMessageRunInitialized") {
            current_day = 1; in_pvp = false; hour_count = 0; continue;
        }
        if l.contains("to [PVPCombatState]") { in_pvp = true; continue; }
        if l.contains("to [EncounterState]") || l.contains("to [ShopState]") {
            hour_count = hour_count.saturating_add(1);
        }
        if in_pvp && l.contains("State changed") && (l.contains("to [ChoiceState]") || l.contains("to [LevelUpState]") || l.contains("to [ReplayState]")) {
            if current_day == 0 { current_day = 1; }
            current_day = current_day.saturating_add(1);
            in_pvp = false; hour_count = 0; continue;
        }
        if hour_count >= hours && l.contains("to [ChoiceState]") {
            if current_day == 0 { current_day = 1; }
            current_day = current_day.saturating_add(1);
            hour_count = 0; continue;
        }
    }
    
    if current_day == 0 { None } else { Some(current_day) }
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
            let handle = app.handle().clone();
            let resources_path = app.path().resource_dir().unwrap();
            let item_db_instance = app.state::<DbState>().items.clone();
            
            // Initial DB Load
            let db_state = app.state::<DbState>();
            let items_json = std::fs::read_to_string(resources_path.join("resources").join("items_db.json")).unwrap_or_default();
            if let Ok(items) = serde_json::from_str::<HashMap<String, ItemData>>(&items_json) {
                *db_state.items.write().unwrap() = items;
            }
            let monsters_json = std::fs::read_to_string(resources_path.join("resources").join("monsters_db.json")).unwrap_or_default();
            if let Ok(monsters) = serde_json::from_str::<HashMap<String, MonsterData>>(&monsters_json) {
                *db_state.monsters.write().unwrap() = monsters;
            }

            // Async Preload Templates
            let res_dir_clone = resources_path.join("resources");
            tauri::async_runtime::spawn(async move {
                let _ = monster_recognition::preload_templates_async(res_dir_clone).await;
            });

            // Log Monitor Thread
            thread::spawn(move || {
                let log_path = get_log_path();
                let re_purchase = Regex::new(r"Card Purchased: InstanceId:\s*(?P<iid>[^ ]+)\s*-\s*TemplateId(?P<tid>[^ ]+)(?:.*Target:(?P<tgt>[^ ]+))?(?:.*Section(?P<sec>[^ ]+))?").unwrap();
                let re_id = Regex::new(r"ID: \[(?P<id>[^\]]+)\]").unwrap();
                let re_owner = Regex::new(r"- Owner: \[(?P<val>[^\]]+)\]").unwrap();
                let re_section = Regex::new(r"- Section: \[(?P<val>[^\]]+)\]").unwrap();
                let re_state_change = Regex::new(r"State changed from \[EncounterState\] to \[ChoiceState\]").unwrap();
                let re_enc_id = Regex::new(r"enc_[A-Za-z0-9]+").unwrap();
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
                    thread::sleep(time::Duration::from_millis(1000));
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
                let mut encounter_state_detected = false;
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
                            if trimmed.contains("to [PVPCombatState]") { in_pvp = true; }
                            if trimmed.contains("to [EncounterState]") || trimmed.contains("to [ShopState]") {
                                hour_count = hour_count.saturating_add(1);
                            }
                            if in_pvp && trimmed.contains("State changed") && (trimmed.contains("to [ChoiceState]") || trimmed.contains("to [LevelUpState]") || trimmed.contains("to [ReplayState]")) {
                                current_day = current_day.saturating_add(1);
                                in_pvp = false; hour_count = 0; day_changed = true;
                            }
                            if hour_count >= 6 && trimmed.contains("to [ChoiceState]") {
                                current_day = current_day.saturating_add(1);
                                hour_count = 0; day_changed = true;
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

                            if re_state_change.is_match(trimmed) {
                                encounter_state_detected = true;
                            }

                            if re_enc_id.is_match(trimmed) {
                                if encounter_state_detected {
                                    let h = handle.clone();
                                    let d = current_day;
                                    tauri::async_runtime::spawn(async move {
                                        thread::sleep(time::Duration::from_secs(1));
                                        let _ = h.emit("trigger-monster-recognition", d);
                                    });
                                    encounter_state_detected = false;
                                }
                            }

                            if trimmed.contains("Cards Spawned:") || trimmed.contains("Cards Dealt:") {
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
            update_day
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
