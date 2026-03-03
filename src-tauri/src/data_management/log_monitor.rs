use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::sync::{Arc, RwLock};

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
            } else if s == "Stash" || s == "Storage" || s == "PlayerStorage" {
                if current_stash.insert(iid.clone()) {
                    changed = true;
                }
                if current_hand.remove(&iid) {
                    changed = true;
                }
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
        } else if tgt.contains("Socket") {
            if current_hand.insert(iid.clone()) {
                changed = true;
            }
            if current_stash.remove(&iid) {
                changed = true;
            }
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
                } else {
                    if current_hand.remove(last_iid) {
                        changed = true;
                    }
                    if current_stash.remove(last_iid) {
                        changed = true;
                    }
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

        let state_init = load_state();
        let mut inst_to_temp = state_init.inst_to_temp;
        let mut current_hand = state_init.current_hand;
        let mut current_stash = state_init.current_stash;
        let mut current_day = state_init.day;

        let mut in_pvp = false;
        let mut is_sync = false;
        let mut last_iid = String::new();
        let mut cur_owner = String::new();

        if crate::core::log_monitor_state::is_enabled() {
            log::debug!("[LogMonitor] Initializing simplified state from logs...");
            current_hand.clear();
            current_stash.clear();

            let files_to_process = vec![prev_path, log_path.clone()];
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
                                &mut current_day,
                                &mut in_pvp,
                                &mut is_sync,
                                &mut last_iid,
                                &mut cur_owner,
                            );
                        }
                    }
                }
            }

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
                                    &mut current_day,
                                    &mut in_pvp,
                                    &mut is_sync,
                                    &mut last_iid,
                                    &mut cur_owner,
                                );
                                changed |= line_changed;
                                day_changed |= line_day_changed;
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
