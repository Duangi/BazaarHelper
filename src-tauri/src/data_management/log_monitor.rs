use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::fs::File;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use chrono::Local;
use device_query::DeviceQuery;
use regex::Regex;
use tauri::Emitter;

use crate::data_management::item_lookup::lookup_item;
use crate::data_management::log_paths::{get_log_path, get_prev_log_path};
use crate::{load_state, save_state};
use crate::{ItemData, ItemDb, PersistentState, SkillDb, SyncPayload};

const BOOTSTRAP_TAIL_BYTES: u64 = 2_000_000;
const TAIL_READ_MAX_BYTES: u64 = 256_000;
const MAX_SYNC_ITEMS_PER_SECTION: usize = 120;
const INST_MAP_PRUNE_THRESHOLD: usize = 2_048;

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

fn detect_pvp_victory(recent_lines: &VecDeque<String>) -> bool {
    recent_lines
        .iter()
        .rev()
        .nth(3)
        .map(|line| line.contains("All exit tasks completed"))
        .unwrap_or(false)
}

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
    let digits: String = digits_rev.chars().rev().collect();
    digits.parse::<u32>().ok()
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

fn build_lineup_cards_json_from_visual(
    cards: &[crate::monster_recognition::VisualLineupCard],
    items_db: &ItemDb,
    skills_db: &SkillDb,
) -> serde_json::Value {
    let lineup: Vec<serde_json::Value> = cards
        .iter()
        .map(|card| {
            let tid = card.template_id.clone();
            let item = lookup_item(&tid, items_db, skills_db);
            match item {
                Some(matched) => serde_json::json!({
                    "instance_id": serde_json::Value::Null,
                    "template_id": tid.clone(),
                    "name_cn": matched.name_cn,
                    "name_en": matched.name,
                    "image": matched.image,
                    "size": matched.size.unwrap_or_else(|| "medium".to_string()),
                }),
                None => serde_json::json!({
                    "instance_id": serde_json::Value::Null,
                    "template_id": tid.clone(),
                    "name_cn": tid.clone(),
                    "name_en": tid,
                    "image": serde_json::Value::Null,
                    "size": card.size.clone(),
                }),
            }
        })
        .collect();

    serde_json::Value::Array(lineup)
}

fn sanitize_log_time_token(raw: &str) -> String {
    let digits: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        Local::now().format("%H%M%S").to_string()
    } else {
        digits
    }
}

fn infer_day_from_log_tail(path: &PathBuf, retro: bool) -> Option<u32> {
    if !path.exists() {
        return None;
    }
    let mut file = File::open(path).ok()?;
    let metadata = file.metadata().ok()?;
    let file_size = metadata.len();
    let read_size = file_size.min(5_000_000) as usize;
    let mut buffer = vec![0u8; read_size];
    file.seek(SeekFrom::End(-(read_size as i64))).ok()?;
    file.read_exact(&mut buffer).ok()?;
    let content = String::from_utf8_lossy(&buffer);
    crate::data_management::day_calc::calculate_day_from_log(&content, retro)
}

fn build_match_id_from_start_time(start_time: &str) -> String {
    let mut hasher = DefaultHasher::new();
    start_time.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn build_match_folder_name(match_start_time: &str) -> String {
    let start_token = sanitize_log_time_token(match_start_time.trim());
    let match_id = build_match_id_from_start_time(match_start_time.trim());
    format!("start{}_{}", start_token, match_id)
}

pub fn capture_bazaar_round_screenshot(
    match_start_time: &str,
    battle_day: u32,
    battle_start_time: &str,
    allow_monitor_fallback: bool,
) -> Result<Option<PathBuf>, String> {
    use xcap::{Monitor, Window};

    let windows = Window::all().map_err(|e| e.to_string())?;
    let target = windows.iter().find(|w| {
        let title = w.title().to_lowercase();
        let app = w.app_name().to_lowercase();
        (title.contains("the bazaar") || title.contains("bazaar") || app.contains("the bazaar") || app.contains("bazaar"))
            && !title.contains("bazaarhelper")
            && !app.contains("bazaarhelper")
            && w.width() >= 640
            && w.height() >= 360
    });

    let image = if let Some(window) = target {
        log::debug!(
            "[RoundCapture] Capturing game window: title='{}' app='{}' pos=({}, {}) size={}x{}",
            window.title(),
            window.app_name(),
            window.x(),
            window.y(),
            window.width(),
            window.height()
        );
        window.capture_image().map_err(|e| e.to_string())?
    } else if !allow_monitor_fallback {
        log::warn!("[RoundCapture] Game window not found; window-only mode skip capture");
        return Ok(None);
    } else {
        let monitors = Monitor::all().map_err(|e| e.to_string())?;
        if monitors.is_empty() {
            return Ok(None);
        }
        let device_state = device_query::DeviceState::new();
        let mouse = device_state.get_mouse();
        let (mx, my) = mouse.coords;
        let picked = monitors
            .iter()
            .find(|m| {
                mx >= m.x()
                    && mx < (m.x() + m.width() as i32)
                    && my >= m.y()
                    && my < (m.y() + m.height() as i32)
            })
            .or_else(|| monitors.first())
            .ok_or("No monitor found")?;
        log::warn!(
            "[RoundCapture] Game window not found, fallback monitor capture: mouse=({}, {}), monitor=({}, {}) {}x{}",
            mx,
            my,
            picked.x(),
            picked.y(),
            picked.width(),
            picked.height()
        );
        picked.capture_image().map_err(|e| e.to_string())?
    };
    let screenshot_root = {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let workspace_root = manifest_dir
            .parent()
            .map(|p| p.to_path_buf())
            .filter(|p| p.exists());
        workspace_root
            .unwrap_or_else(crate::user_data::app_data_root)
            .join("battle_screenshots")
    };

    let match_folder = build_match_folder_name(match_start_time);
    let screenshot_dir = screenshot_root.join(match_folder);
    std::fs::create_dir_all(&screenshot_dir).map_err(|e| e.to_string())?;

    let _start_token = sanitize_log_time_token(battle_start_time);
    let file_name = format!("pvp_day{:02}.png", battle_day.max(1));
    let full_path = screenshot_dir.join(file_name);
    image::DynamicImage::ImageRgba8(image)
        .save(&full_path)
        .map_err(|e| e.to_string())?;

    log::info!("[RoundCapture] Screenshot saved: {}", full_path.to_string_lossy());

    Ok(Some(full_path))
}

fn is_run_init_line(line: &str) -> bool {
    line.contains("NetMessageRunInitialized") || line.contains("[GameInstance] Starting new run...")
}

fn fingerprint_inventory_sets(hand: &HashSet<String>, stash: &HashSet<String>) -> u64 {
    let mut hasher = DefaultHasher::new();

    "hand".hash(&mut hasher);
    let mut hand_ids: Vec<&String> = hand.iter().collect();
    hand_ids.sort();
    for id in hand_ids {
        id.hash(&mut hasher);
    }

    "stash".hash(&mut hasher);
    let mut stash_ids: Vec<&String> = stash.iter().collect();
    stash_ids.sort();
    for id in stash_ids {
        id.hash(&mut hasher);
    }

    hasher.finish()
}

fn prune_inst_to_temp(
    inst_to_temp: &mut HashMap<String, String>,
    current_hand: &HashSet<String>,
    current_stash: &HashSet<String>,
) {
    if inst_to_temp.len() <= INST_MAP_PRUNE_THRESHOLD {
        return;
    }

    inst_to_temp.retain(|iid, _| current_hand.contains(iid) || current_stash.contains(iid));
}

fn map_items(
    ids: &HashSet<String>,
    inst_to_temp: &HashMap<String, String>,
    items_db: &ItemDb,
    skills_db: &SkillDb,
) -> Vec<ItemData> {
    let mut ordered_ids: Vec<&String> = ids.iter().collect();
    ordered_ids.sort();
    if ordered_ids.len() > MAX_SYNC_ITEMS_PER_SECTION {
        ordered_ids.truncate(MAX_SYNC_ITEMS_PER_SECTION);
    }

    ordered_ids
        .into_iter()
        .filter_map(|iid| {
            let tid = inst_to_temp.get(iid)?;
            let mut item = lookup_item(tid, items_db, skills_db)?;
            item.instance_id = Some(iid.clone());
            Some(item)
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn process_log_line(
    trimmed: &str,
    re_purchase: &Regex,
    re_id: &Regex,
    re_tid: &Regex,
    re_owner: &Regex,
    re_section: &Regex,
    re_item_id: &Regex,
    re_sold: &Regex,
    re_removed: &Regex,
    re_moved_to: &Regex,
    inst_to_temp: &mut HashMap<String, String>,
    current_hand: &mut HashSet<String>,
    current_stash: &mut HashSet<String>,
    hand_slot_to_iid: &mut BTreeMap<u32, String>,
    iid_to_hand_slot: &mut HashMap<String, u32>,
    current_day: &mut u32,
    in_pvp: &mut bool,
    is_sync: &mut bool,
    last_iid: &mut String,
    cur_owner: &mut String,
) -> (bool, bool) {
    let mut changed = false;
    let mut day_changed = false;

    if is_run_init_line(trimmed) {
        *current_day = 1;
        *in_pvp = false;
        *is_sync = false;
        inst_to_temp.clear();
        current_hand.clear();
        current_stash.clear();
        hand_slot_to_iid.clear();
        iid_to_hand_slot.clear();
        changed = true;
        day_changed = true;
    }

    if trimmed.contains("to [PVPCombatState]") {
        *in_pvp = true;
    }
    if *in_pvp
        && trimmed.contains("State changed")
        && (trimmed.contains("to [ChoiceState]") || trimmed.contains("to [LevelUpState]"))
    {
        *current_day = current_day.saturating_add(1);
        *in_pvp = false;
        day_changed = true;
    }

    if let Some(cap) = re_purchase.captures(trimmed) {
        let iid = cap["iid"].to_string();
        let tid = cap["tid"].to_string();
        let old_tid = inst_to_temp.insert(iid.clone(), tid.clone());
        if old_tid.as_deref() != Some(tid.as_str()) {
            changed = true;
        }

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

        if let Some(s) = section {
            if s == "Player" || s == "Hand" {
                if current_hand.insert(iid.clone()) {
                    changed = true;
                }
                if current_stash.remove(&iid) {
                    changed = true;
                }
                if let Some(tgt) = target {
                    if let Some(slot) = parse_socket_index_from_target(tgt) {
                        set_hand_slot_mapping(&iid, slot, hand_slot_to_iid, iid_to_hand_slot);
                    }
                }
            } else if s == "Stash" || s == "Storage" || s == "PlayerStorage" {
                if current_stash.insert(iid.clone()) {
                    changed = true;
                }
                if current_hand.remove(&iid) {
                    changed = true;
                }
                remove_hand_slot_mapping(&iid, hand_slot_to_iid, iid_to_hand_slot);
            }
        }
    }

    if let Some(cap) = re_moved_to.captures(trimmed) {
        let iid = cap["iid"].to_string();
        let tgt = &cap["tgt"];
        if tgt.contains("StorageSocket") {
            if current_stash.insert(iid.clone()) {
                changed = true;
            }
            if current_hand.remove(&iid) {
                changed = true;
            }
            remove_hand_slot_mapping(&iid, hand_slot_to_iid, iid_to_hand_slot);
        } else if tgt.contains("Socket") {
            if current_hand.insert(iid.clone()) {
                changed = true;
            }
            if current_stash.remove(&iid) {
                changed = true;
            }
            if let Some(slot) = parse_socket_index_from_target(tgt) {
                set_hand_slot_mapping(&iid, slot, hand_slot_to_iid, iid_to_hand_slot);
            }
        }
    }

    if let Some(cap) = re_sold.captures(trimmed) {
        let iid = cap["iid"].to_string();
        if current_hand.remove(&iid) || current_stash.remove(&iid) {
            changed = true;
        }
        remove_hand_slot_mapping(&iid, hand_slot_to_iid, iid_to_hand_slot);
    }
    if let Some(cap) = re_removed.captures(trimmed) {
        let iid = cap["iid"].to_string();
        if current_hand.remove(&iid) || current_stash.remove(&iid) {
            changed = true;
        }
        remove_hand_slot_mapping(&iid, hand_slot_to_iid, iid_to_hand_slot);
    }
    if trimmed.contains("Cards Disposed:") {
        for mat in re_item_id.find_iter(trimmed) {
            let iid = mat.as_str().to_string();
            if current_hand.remove(&iid) || current_stash.remove(&iid) {
                changed = true;
            }
            remove_hand_slot_mapping(&iid, hand_slot_to_iid, iid_to_hand_slot);
        }
    }

    if trimmed.contains("Cards Spawned:")
        || trimmed.contains("Cards Dealt:")
        || trimmed.contains("NetMessageGameStateSync")
        || trimmed.contains("Successfully moved card to:")
    {
        *is_sync = true;
    }

    if *is_sync {
        if let Some(cap) = re_id.captures(trimmed) {
            *last_iid = cap["id"].to_string();
        } else if let Some(cap) = re_tid.captures(trimmed) {
            if !last_iid.is_empty() {
                let tid = cap["tid"].to_string();
                let old_tid = inst_to_temp.insert(last_iid.clone(), tid.clone());
                if old_tid.as_deref() != Some(tid.as_str()) {
                    changed = true;
                }
            }
        } else if let Some(cap) = re_owner.captures(trimmed) {
            *cur_owner = cap["val"].to_string();
        } else if let Some(cap) = re_section.captures(trimmed) {
            if !last_iid.is_empty() && cur_owner.as_str() == "Player" && last_iid.starts_with("itm_") {
                let sec_val = &cap["val"];
                if sec_val == "Hand" || sec_val == "Player" {
                    if current_hand.insert(last_iid.clone()) {
                        changed = true;
                    }
                    if current_stash.remove(last_iid) {
                        changed = true;
                    }
                } else if sec_val == "Stash" || sec_val == "Storage" || sec_val == "PlayerStorage" {
                    if current_stash.insert(last_iid.clone()) {
                        changed = true;
                    }
                    if current_hand.remove(last_iid) {
                        changed = true;
                    }
                    remove_hand_slot_mapping(last_iid, hand_slot_to_iid, iid_to_hand_slot);
                } else {
                    if current_hand.remove(last_iid) {
                        changed = true;
                    }
                    if current_stash.remove(last_iid) {
                        changed = true;
                    }
                    remove_hand_slot_mapping(last_iid, hand_slot_to_iid, iid_to_hand_slot);
                }
            }
            last_iid.clear();
            cur_owner.clear();
        } else if trimmed.contains("Finished processing") {
            *is_sync = false;
        }
    }

    (changed, day_changed)
}

pub fn spawn_log_monitor(
    log_handle: tauri::AppHandle,
    thread_items_db: Arc<RwLock<ItemDb>>,
    thread_skills_db: Arc<RwLock<SkillDb>>,
) {
    std::thread::spawn(move || {
        let handle = log_handle;
        let log_path = get_log_path();
        let prev_path = get_prev_log_path();

        let re_purchase = Regex::new(r"Card Purchased: InstanceId:\s*(?P<iid>[^ ]+)\s*-\s*TemplateId\s*(?P<tid>[^ ]+)(?:.*Target:(?P<tgt>[^ ]+))?(?:.*Section(?P<sec>[^ ]+))?").unwrap();
        let re_id = Regex::new(r"ID: \[(?P<id>[^\]]+)\]").unwrap();
        let re_tid = Regex::new(r"TemplateId: \[(?P<tid>[^\]]+)\]").unwrap();
        let re_owner = Regex::new(r"- Owner: \[(?P<val>[^\]]+)\]").unwrap();
        let re_section = Regex::new(r"- Section: \[(?P<val>[^\]]+)\]").unwrap();
        let re_item_id = Regex::new(r"itm_[A-Za-z0-9_-]+").unwrap();
        let re_sold = Regex::new(r"Sold Card\s+(?P<iid>itm_[^ ]+)").unwrap();
        let re_removed = Regex::new(r"Successfully removed item\s+(?P<iid>itm_[^ ]+)").unwrap();
        let re_moved_to =
            Regex::new(r"Successfully moved card\s+(?P<iid>itm_[^ ]+)\s+to\s+(?P<tgt>[^ ]+)").unwrap();
        let re_state_change = Regex::new(r"State changed from \[.*?\] to \[(?P<state>[^\]]+)\]").unwrap();
        let re_combat_duration = Regex::new(r"Combat simulation completed in (?P<dur>[\d\.]+)s").unwrap();

        let state_init = load_state();
        let mut inst_to_temp = state_init.inst_to_temp;
        let mut current_hand = state_init.current_hand;
        let mut current_stash = state_init.current_stash;
        let mut hand_slot_to_iid: BTreeMap<u32, String> = BTreeMap::new();
        let mut iid_to_hand_slot: HashMap<String, u32> = HashMap::new();
        let inferred_day = infer_day_from_log_tail(&log_path, true)
            .or_else(|| infer_day_from_log_tail(&prev_path, true))
            .unwrap_or(0);
        let history_day = crate::user_data::infer_current_day_from_history()
            .ok()
            .flatten()
            .unwrap_or(0);
        let mut current_day = inferred_day
            .max(history_day)
            .max(state_init.day)
            .max(1);

        let mut in_pvp = false;
        let mut is_sync = false;
        let mut last_iid = String::new();
        let mut cur_owner = String::new();
        let mut current_match_start_time: Option<String> = None;
        let mut current_battle_start_time: Option<String> = None;
        let mut last_pvp_duration: Option<f64> = None;
        let mut recent_lines: VecDeque<String> = VecDeque::with_capacity(8);
        let mut last_captured_round_key: Option<String> = None;

        if crate::core::log_monitor_state::is_enabled() {
            log::debug!("[LogMonitor] Initializing simplified state from logs...");
            current_hand.clear();
            current_stash.clear();

            let files_to_process = vec![prev_path.clone(), log_path.clone()];
            for path in files_to_process {
                if !path.exists() {
                    continue;
                }
                if let Ok(mut file) = File::open(&path) {
                    let start_offset = file
                        .metadata()
                        .ok()
                        .map(|m| m.len().saturating_sub(BOOTSTRAP_TAIL_BYTES))
                        .unwrap_or(0);
                    let _ = file.seek(SeekFrom::Start(start_offset));
                    let mut reader = BufReader::new(file);

                    if start_offset > 0 {
                        let mut discard = String::new();
                        let _ = reader.read_line(&mut discard);
                    }

                    for line in reader.lines() {
                        if let Ok(l) = line {
                            let _ = process_log_line(
                                l.trim(),
                                &re_purchase,
                                &re_id,
                                &re_tid,
                                &re_owner,
                                &re_section,
                                &re_item_id,
                                &re_sold,
                                &re_removed,
                                &re_moved_to,
                                &mut inst_to_temp,
                                &mut current_hand,
                                &mut current_stash,
                                &mut hand_slot_to_iid,
                                &mut iid_to_hand_slot,
                                &mut current_day,
                                &mut in_pvp,
                                &mut is_sync,
                                &mut last_iid,
                                &mut cur_owner,
                            );

                            if recent_lines.len() >= 8 {
                                recent_lines.pop_front();
                            }
                            recent_lines.push_back(l.trim().to_string());

                            let timestamp = extract_log_timestamp(l.trim());
                            if is_run_init_line(l.trim()) {
                                current_match_start_time = timestamp.clone();
                                current_battle_start_time = None;
                                last_pvp_duration = None;
                                last_captured_round_key = None;
                            }
                            if let Some(cap) = re_combat_duration.captures(l.trim()) {
                                if let Ok(dur) = cap["dur"].parse::<f64>() {
                                    last_pvp_duration = Some(dur);
                                }
                            }
                            if let Some(cap) = re_state_change.captures(l.trim()) {
                                let next_state = cap["state"].trim();
                                if next_state == "PVPCombatState" {
                                    if current_match_start_time.is_none() {
                                        current_match_start_time = timestamp.clone();
                                    }
                                    current_battle_start_time = timestamp.clone();
                                    last_pvp_duration = None;
                                } else if next_state == "ReplayState" {
                                    current_battle_start_time = None;
                                    last_pvp_duration = None;
                                }
                            }
                        }
                    }
                }
            }

            let inferred_day_post = infer_day_from_log_tail(&log_path, true)
                .or_else(|| infer_day_from_log_tail(&prev_path, true))
                .unwrap_or(0);
            let history_day_post = crate::user_data::infer_current_day_from_history()
                .ok()
                .flatten()
                .unwrap_or(0);
            current_day = current_day
                .max(inferred_day_post)
                .max(history_day_post)
                .max(state_init.day)
                .max(1);

            prune_inst_to_temp(&mut inst_to_temp, &current_hand, &current_stash);
            save_state(&PersistentState {
                day: current_day,
                inst_to_temp: inst_to_temp.clone(),
                current_hand: current_hand.clone(),
                current_stash: current_stash.clone(),
                ..load_state()
            });

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
                let hand_items = map_items(&init_hand, &init_map, &items_db, &skills_db);
                let stash_items = map_items(&init_stash, &init_map, &items_db, &skills_db);
                let all_tags = items_db.unique_tags.clone();
                let _ = init_handle.emit(
                    "sync-items",
                    SyncPayload {
                        hand_items,
                        stash_items,
                        all_tags,
                    },
                );
            });
        } else {
            log::warn!("[LogMonitor] bootstrap replay skipped because monitor is disabled");
        }

        let mut last_size = 0u64;
        if let Ok(meta) = std::fs::metadata(&log_path) {
            last_size = meta.len();
        }
        let mut last_sync_fingerprint = fingerprint_inventory_sets(&current_hand, &current_stash);
        let mut monitor_was_enabled = crate::core::log_monitor_state::is_enabled();

        log::debug!(
            "[LogMonitor] Simplified monitor started. initial_size={}",
            last_size
        );

        loop {
            std::thread::sleep(std::time::Duration::from_millis(500));

            let monitor_enabled = crate::core::log_monitor_state::is_enabled();
            if !monitor_enabled {
                if monitor_was_enabled {
                    log::info!(
                        "[LogMonitor] paused by runtime switch (cursor kept at {}, backlog will be replayed on resume)",
                        last_size
                    );
                }
                monitor_was_enabled = false;
                continue;
            }
            if !monitor_was_enabled {
                log::info!("[LogMonitor] resumed by runtime switch, replaying backlog from {}", last_size);
                monitor_was_enabled = true;
                continue;
            }

            if let Ok(mut f) = File::open(&log_path) {
                if let Ok(meta) = f.metadata() {
                    let len = meta.len();
                    if len < last_size {
                        last_size = 0;
                        in_pvp = false;
                        is_sync = false;
                        last_iid.clear();
                        cur_owner.clear();
                    }
                    if len > last_size {
                        if f.seek(SeekFrom::Start(last_size)).is_ok() {
                            let mut reader = BufReader::new(f);
                            let mut bytes_read: u64 = 0;
                            let mut line_buf = String::new();
                            let mut changed = false;
                            let mut day_changed = false;

                            while bytes_read < TAIL_READ_MAX_BYTES {
                                line_buf.clear();
                                let n = reader.read_line(&mut line_buf).unwrap_or(0);
                                if n == 0 {
                                    break;
                                }
                                bytes_read = bytes_read.saturating_add(n as u64);
                                let was_in_pvp = in_pvp;
                                let (line_changed, line_day_changed) = process_log_line(
                                    line_buf.trim(),
                                    &re_purchase,
                                    &re_id,
                                    &re_tid,
                                    &re_owner,
                                    &re_section,
                                    &re_item_id,
                                    &re_sold,
                                    &re_removed,
                                    &re_moved_to,
                                    &mut inst_to_temp,
                                    &mut current_hand,
                                    &mut current_stash,
                                    &mut hand_slot_to_iid,
                                    &mut iid_to_hand_slot,
                                    &mut current_day,
                                    &mut in_pvp,
                                    &mut is_sync,
                                    &mut last_iid,
                                    &mut cur_owner,
                                );
                                changed |= line_changed;
                                day_changed |= line_day_changed;

                                if recent_lines.len() >= 8 {
                                    recent_lines.pop_front();
                                }
                                recent_lines.push_back(line_buf.trim().to_string());

                                let timestamp = extract_log_timestamp(line_buf.trim());
                                if is_run_init_line(line_buf.trim()) {
                                    current_match_start_time = timestamp.clone();
                                    current_battle_start_time = None;
                                    last_pvp_duration = None;
                                    last_captured_round_key = None;
                                }

                                if let Some(cap) = re_combat_duration.captures(line_buf.trim()) {
                                    if let Ok(dur) = cap["dur"].parse::<f64>() {
                                        last_pvp_duration = Some(dur);
                                    }
                                }

                                if let Some(cap) = re_state_change.captures(line_buf.trim()) {
                                    let next_state = cap["state"].trim();

                                    log::debug!(
                                        "[RoundCapture] State transition observed: next_state={}, was_in_pvp={}, in_pvp={}, day={}, ts={:?}",
                                        next_state,
                                        was_in_pvp,
                                        in_pvp,
                                        current_day,
                                        timestamp
                                    );

                                    if next_state == "PVPCombatState" {
                                        if current_match_start_time.is_none() {
                                            current_match_start_time = timestamp.clone();
                                        }
                                        current_battle_start_time = timestamp.clone();
                                        last_pvp_duration = None;
                                        log::info!(
                                            "[RoundCapture] PVP started: match_start={:?}, battle_start={:?}, day={}",
                                            current_match_start_time,
                                            current_battle_start_time,
                                            current_day
                                        );
                                    }

                                    if next_state == "ReplayState" && was_in_pvp {
                                        let battle_start = current_battle_start_time
                                            .clone()
                                            .or(timestamp.clone())
                                            .unwrap_or_else(|| Local::now().format("%H:%M:%S").to_string());
                                        let match_start = current_match_start_time
                                            .clone()
                                            .unwrap_or_else(|| battle_start.clone());
                                        let battle_day = current_day.max(1);
                                        let battle_victory = detect_pvp_victory(&recent_lines);
                                        let duration = last_pvp_duration;
                                        let round_key = format!("{}|{}|{}", match_start, battle_day, battle_start);

                                        if last_captured_round_key.as_ref() == Some(&round_key) {
                                            log::debug!(
                                                "[RoundCapture] Skip duplicated round trigger: {}",
                                                round_key
                                            );
                                            continue;
                                        }
                                        last_captured_round_key = Some(round_key);

                                        log::info!(
                                            "[RoundCapture] Replay trigger accepted: match_start={}, battle_day={}, battle_start={}, victory={}, duration={:?}",
                                            match_start,
                                            battle_day,
                                            battle_start,
                                            battle_victory,
                                            duration
                                        );

                                        let status_text = if let Some(dur) = duration {
                                            format!("本回合战斗时长约 {:.1}s，正在截图保存...", dur)
                                        } else {
                                            "本回合战斗结束，正在截图保存...".to_string()
                                        };
                                        log::info!("[RoundCapture] Island status emit: {}", status_text);
                                        let _ = handle.emit(
                                            "island-status",
                                            serde_json::json!({
                                                "message": status_text,
                                                "type": "info"
                                            }),
                                        );

                                        let handle_for_task = handle.clone();
                                        let match_start_for_task = match_start.clone();
                                        let battle_start_for_task = battle_start.clone();
                                        let capture_delay_ms = crate::load_state().screenshot_capture_delay_ms.min(3000);
                                        std::thread::spawn(move || {
                                            let canonical_match_start = crate::user_data::resolve_active_match_start_time(&match_start_for_task)
                                                .unwrap_or_else(|_| match_start_for_task.clone());

                                            if capture_delay_ms > 0 {
                                                std::thread::sleep(std::time::Duration::from_millis(capture_delay_ms));
                                            }

                                            let screenshot_path = match capture_bazaar_round_screenshot(
                                                &canonical_match_start,
                                                battle_day,
                                                &battle_start_for_task,
                                                true,
                                            ) {
                                                Ok(Some(path)) => Some(path.to_string_lossy().to_string()),
                                                Ok(None) => None,
                                                Err(e) => {
                                                    log::warn!("[LogMonitor] Round screenshot capture failed: {}", e);
                                                    None
                                                }
                                            };

                                            log::info!(
                                                "[RoundCapture] Capture result: has_screenshot={}, path={:?}",
                                                screenshot_path.is_some(),
                                                screenshot_path
                                            );

                                            // Auto visual lineup analysis is intentionally disabled to avoid
                                            // repeated heavy matching loops and UI stalls.
                                            let lineup_cards_raw: Option<String> = None;
                                            let enemy_lineup_cards_raw: Option<String> = None;

                                            if let Err(e) = crate::user_data::upsert_match_battle_snapshot(
                                                &canonical_match_start,
                                                battle_day,
                                                &battle_start_for_task,
                                                battle_victory,
                                                duration,
                                                screenshot_path.as_deref(),
                                                lineup_cards_raw.as_deref(),
                                                enemy_lineup_cards_raw.as_deref(),
                                            ) {
                                                log::warn!("[LogMonitor] Failed to persist round snapshot: {}", e);
                                            } else {
                                                log::info!(
                                                    "[RoundCapture] DB upsert ok: match_start={}, battle_day={}, battle_start={}",
                                                    canonical_match_start,
                                                    battle_day,
                                                    battle_start_for_task
                                                );
                                                let _ = handle_for_task.emit("match-history-updated", serde_json::json!({
                                                    "day": battle_day,
                                                    "start_time": battle_start_for_task,
                                                    "has_screenshot": screenshot_path.is_some()
                                                }));
                                                log::info!("[RoundCapture] match-history-updated emitted");
                                            }
                                        });

                                        current_battle_start_time = None;
                                        last_pvp_duration = None;
                                    }
                                }
                            }

                            if day_changed {
                                let _ = handle.emit("day-update", current_day);
                            }

                            if changed {
                                let current_fingerprint =
                                    fingerprint_inventory_sets(&current_hand, &current_stash);
                                if current_fingerprint != last_sync_fingerprint {
                                    let items_db = thread_items_db.read().unwrap();
                                    let skills_db = thread_skills_db.read().unwrap();

                                    let hand_items =
                                        map_items(&current_hand, &inst_to_temp, &items_db, &skills_db);
                                    let stash_items =
                                        map_items(&current_stash, &inst_to_temp, &items_db, &skills_db);
                                    let all_tags = items_db.unique_tags.clone();
                                    let _ = handle.emit(
                                        "sync-items",
                                        SyncPayload {
                                            hand_items,
                                            stash_items,
                                            all_tags,
                                        },
                                    );
                                    last_sync_fingerprint = current_fingerprint;
                                }

                                prune_inst_to_temp(&mut inst_to_temp, &current_hand, &current_stash);
                                save_state(&PersistentState {
                                    day: current_day,
                                    inst_to_temp: inst_to_temp.clone(),
                                    current_hand: current_hand.clone(),
                                    current_stash: current_stash.clone(),
                                    ..load_state()
                                });
                            }

                            last_size = last_size.saturating_add(bytes_read).min(len);
                        }
                    }
                }
            }
        }
    });
}
