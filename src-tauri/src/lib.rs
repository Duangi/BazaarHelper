use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::{thread, time};
use tauri::{Emitter, Manager, State};

// --- 数据模型 ---
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Enchantment { pub id: String, pub name: String, pub description: String }

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ItemData {
    pub id: Option<String>,
    #[serde(default)] pub name_zh: String,
    pub enchantments: Option<Vec<Enchantment>>,
    pub image: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MonsterSubItem {
    pub name: String,
    pub description: String,
    pub image: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MonsterData {
    #[serde(default)] pub name: String,
    #[serde(default)] pub name_zh: String,
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
    let home = std::env::var("USERPROFILE").unwrap_or_default();
    PathBuf::from(home).join("AppData").join("LocalLow").join("Tempo Storm").join("The Bazaar").join("Player.log")
}

fn lookup_item(tid: &str, db: &HashMap<String, ItemData>) -> Option<ItemData> {
    db.get(tid).map(|item| {
        let mut cloned = item.clone();
        cloned.id = Some(tid.to_string());
        cloned
    })
}

// --- 指令接口 ---
#[tauri::command]
fn search_monsters(query: String, state: State<'_, DbState>) -> Result<Vec<MonsterData>, String> {
    let db = state.monsters.read().map_err(|_| "DB Busy")?;
    let q = query.to_lowercase();
    let results: Vec<MonsterData> = db.values()
        .filter(|m| m.name_zh.to_lowercase().contains(&q) || m.name.to_lowercase().contains(&q))
        .cloned()
        .collect();
    Ok(results)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(DbState {
            items: Arc::new(RwLock::new(HashMap::new())),
            monsters: Arc::new(RwLock::new(HashMap::new())),
        })
        .setup(|app| {
            let handle = app.handle().clone();
            let state = app.state::<DbState>();
            let item_db_instance = state.items.clone();
            let monster_db_instance = state.monsters.clone();

            // 1. 加载数据库
            let item_res = handle.path().resolve("resources/items_db.json", tauri::path::BaseDirectory::Resource).unwrap();
            let items: HashMap<String, ItemData> = serde_json::from_str(&std::fs::read_to_string(item_res).unwrap()).unwrap();
            *item_db_instance.write().unwrap() = items;

            let monster_res = handle.path().resolve("resources/combat_encounters.json", tauri::path::BaseDirectory::Resource).unwrap();
            let monsters: HashMap<String, MonsterData> = serde_json::from_str(&std::fs::read_to_string(monster_res).unwrap()).unwrap();
            *monster_db_instance.write().unwrap() = monsters;

            // 2. 日志监听线程
            tauri::async_runtime::spawn(async move {
                let log_path = get_log_path();
                while !log_path.exists() { thread::sleep(time::Duration::from_secs(2)); }
                
                let file_content = std::fs::read_to_string(&log_path).unwrap_or_default();
                
                // 正则表达式
                let re_purchase = Regex::new(r"Card Purchased: InstanceId:\s*(?P<iid>[^ ]+)\s*-\s*TemplateId(?P<tid>[^ ]+)\s*-\s*Target:(?P<target>[^\s]+)").unwrap();
                let re_sold = Regex::new(r"(Successfully removed item|Sold Card)\s+(?P<iid>[^ ]+)").unwrap();
                let re_id = Regex::new(r"ID: \[(?P<id>[^\]]+)\]").unwrap();
                let re_owner = Regex::new(r"- Owner: \[(?P<val>[^\]]+)\]").unwrap();
                let re_section = Regex::new(r"- Section: \[(?P<val>[^\]]+)\]").unwrap();
                let re_dealt = Regex::new(r"ID: \[(?P<id>[^\]]+)\]").unwrap();
                
                // 第一步：从头扫描所有购买记录，建立完整的 inst_to_temp 映射
                let mut inst_to_temp: HashMap<String, String> = HashMap::new();
                for cap in re_purchase.captures_iter(&file_content) {
                    inst_to_temp.insert(cap["iid"].to_string(), cap["tid"].to_string());
                }
                println!("建立了 {} 个实例映射", inst_to_temp.len());
                
                // 第二步：找到最后一个游戏开始标记
                let last_game_start = file_content.rfind("State changed from [null] to [StartRunAppState]")
                    .map(|pos| file_content[..pos].rfind('\n').map(|n| n + 1).unwrap_or(0))
                    .unwrap_or(0);
                println!("游戏开始位置: {}", last_game_start);
                
                let mut reader = BufReader::new(File::open(&log_path).unwrap());
                let _ = reader.seek(SeekFrom::Start(last_game_start as u64));

                let mut hand_iids: HashSet<String> = HashSet::new();
                let mut stash_iids: HashSet<String> = HashSet::new();
                let mut monster_ids: Vec<String> = Vec::new();
                let mut is_sync = false;
                let mut sync_hand_cleared = false; // 标记是否已清空手牌
                let (mut last_iid, mut cur_owner) = (String::new(), String::new());
                let mut initial_scan_done = false;

                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap() > 0 {
                        let trimmed = line.trim();
                        let mut changed = false;

                        if trimmed.contains("Cards Spawned:") {
                            // 进入同步模式，但不立即清空手牌
                            // 只清空怪物数据和同步标记
                            if !is_sync {
                                is_sync = true;
                                sync_hand_cleared = false;
                                monster_ids.clear();
                            }
                        }
                        if is_sync {
                            if let Some(cap) = re_id.captures(trimmed) { last_iid = cap["id"].to_string(); }
                            else if let Some(cap) = re_owner.captures(trimmed) { cur_owner = cap["val"].to_string(); }
                            else if let Some(cap) = re_section.captures(trimmed) {
                                if !last_iid.is_empty() {
                                    if &cur_owner == "Player" {
                                        // 第一次遇到玩家手牌时，清空旧的手牌数据
                                        if !sync_hand_cleared && last_iid.starts_with("itm_") && &cap["val"] == "Hand" {
                                            hand_iids.clear();
                                            sync_hand_cleared = true;
                                        }
                                        // 只处理物品（itm_ 开头），忽略效果(eft_)、技能(ste_)等
                                        if last_iid.starts_with("itm_") {
                                            if &cap["val"] == "Hand" { hand_iids.insert(last_iid.clone()); }
                                            else { stash_iids.insert(last_iid.clone()); }
                                            changed = true;
                                        }
                                    } else if &cur_owner == "Opponent" {
                                        let tid = inst_to_temp.get(&last_iid).cloned().unwrap_or(last_iid.clone());
                                        if !monster_ids.contains(&tid) { monster_ids.push(tid); }
                                        changed = true;
                                    }
                                }
                            }
                            if trimmed.contains("Finished processing") { 
                                is_sync = false; 
                                sync_hand_cleared = false;
                                changed = true; // 强制同步，确保状态更新
                            }
                        }
                        // 同步模式下仍需记录映射关系，但不立即更新手牌/仓库
                        if let Some(cap) = re_purchase.captures(trimmed) {
                            let iid = cap["iid"].to_string();
                            let tid = cap["tid"].to_string();
                            let target = cap["target"].to_string();
                            inst_to_temp.insert(iid.clone(), tid);
                            
                            // 只处理物品（itm_ 开头），忽略效果(eft_)、技能(ste_)等
                            if iid.starts_with("itm_") {
                                // 根据 Target 判断：包含 Storage 是仓库，否则是手牌
                                if target.contains("Storage") {
                                    // 仓库购买始终处理，不受同步模式影响
                                    stash_iids.insert(iid);
                                    changed = true;
                                } else if !is_sync {
                                    // 手牌购买只在非同步模式下处理
                                    hand_iids.insert(iid);
                                    changed = true;
                                }
                            }
                        }
                        // 同步模式下不处理出售事件，等待完整的同步数据
                        if !is_sync {
                            if let Some(cap) = re_sold.captures(trimmed) {
                                let iid = cap["iid"].to_string();
                                
                                // 只处理物品（itm_ 开头）
                                if iid.starts_with("itm_") {
                                    // 从手牌和仓库中移除这个实例
                                    hand_iids.remove(&iid);
                                    stash_iids.remove(&iid);
                                    changed = true;
                                }
                            }
                        }
                        if trimmed.contains("Cards Dealt") {
                            monster_ids.clear();
                            for cap in re_dealt.captures_iter(trimmed) { monster_ids.push(cap["id"].to_string()); }
                            changed = true;
                        }

                        if changed && !is_sync {
                            let items_db = item_db_instance.read().unwrap();
                            let payload = SyncPayload {
                                hand_items: hand_iids.iter().filter_map(|iid| lookup_item(inst_to_temp.get(iid).unwrap_or(iid), &items_db)).collect(),
                                stash_items: stash_iids.iter().filter_map(|iid| lookup_item(inst_to_temp.get(iid).unwrap_or(iid), &items_db)).collect(),
                            };
                            handle.emit("sync-items", payload).unwrap();
                        }
                    } else { 
                        // 读取完历史记录
                        if !initial_scan_done {
                            initial_scan_done = true;
                            println!("初始扫描完成 - 手牌: {}, 仓库: {}", hand_iids.len(), stash_iids.len());
                            for iid in &hand_iids {
                                let tid = inst_to_temp.get(iid).unwrap_or(iid);
                                println!("手牌 Instance: {} -> Template: {}", iid, tid);
                            }
                            
                            // 延迟2秒让前端监听器准备好
                            println!("等待2秒后发送初始状态...");
                            thread::sleep(time::Duration::from_millis(2000));
                        }
                        
                        // 发送当前状态
                        let items_db = item_db_instance.read().unwrap();
                        let hand_items: Vec<ItemData> = hand_iids.iter().filter_map(|iid| {
                            let tid = inst_to_temp.get(iid).unwrap_or(iid);
                            let item = lookup_item(tid, &items_db);
                            if item.is_none() {
                                println!("⚠️ 找不到物品: {}", tid);
                            }
                            item
                        }).collect();
                        
                        let stash_items: Vec<ItemData> = stash_iids.iter().filter_map(|iid| {
                            let tid = inst_to_temp.get(iid).unwrap_or(iid);
                            lookup_item(tid, &items_db)
                        }).collect();
                        
                        println!("发送状态 - 手牌: {}, 仓库: {}", hand_items.len(), stash_items.len());
                        let payload = SyncPayload {
                            hand_items,
                            stash_items,
                        };
                        handle.emit("sync-items", payload).unwrap();
                        
                        thread::sleep(time::Duration::from_millis(10000)); // 每10秒发送一次
                    }
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![search_monsters])
        .run(tauri::generate_context!())
        .expect("Error")
        ;
}