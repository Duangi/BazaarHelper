use std::collections::{HashMap, HashSet};
use std::collections::VecDeque;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::sync::{Arc, RwLock};

use chrono::{DateTime, Local};
use regex::Regex;
use tauri::Emitter;

use crate::{ItemData, ItemDb, PersistentState, SkillDb, SyncPayload};
use crate::{load_state, save_state};
use crate::data_management::item_lookup::lookup_item;
use crate::data_management::log_paths::{get_log_path, get_prev_log_path};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct HistoryBattleRecord {
    day: u32,
    start_time: String,
    victory: bool,
    duration: Option<f64>,
    screenshot: Option<String>,
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

fn load_history_root() -> HistoryRoot {
    match crate::user_data::load_match_history() {
        Ok(value) => serde_json::from_value::<HistoryRoot>(value).unwrap_or_default(),
        Err(_) => HistoryRoot::default(),
    }
}

fn save_history_root(history: &HistoryRoot) -> Result<(), String> {
    let value = serde_json::to_value(history).map_err(|e| e.to_string())?;
    crate::user_data::save_match_history(&value)
}

fn extract_timestamp(line: &str) -> Option<String> {
    if !line.starts_with('[') {
        return None;
    }
    let end = line.find(']')?;
    if end <= 1 {
        return None;
    }
    Some(line[1..end].to_string())
}

fn parse_time_to_millis(raw: &str) -> Option<i64> {
    let mut parts = raw.split(':');
    let h = parts.next()?.parse::<i64>().ok()?;
    let m = parts.next()?.parse::<i64>().ok()?;
    let sec_part = parts.next()?;
    let mut sec_parts = sec_part.split('.');
    let s = sec_parts.next()?.parse::<i64>().ok()?;
    let ms_raw = sec_parts.next().unwrap_or("0");
    let ms = format!("{ms_raw:0<3}")
        .chars()
        .take(3)
        .collect::<String>()
        .parse::<i64>()
        .ok()?;
    Some(((h * 60 + m) * 60 + s) * 1000 + ms)
}

fn is_run_init_line(line: &str) -> bool {
    line.contains("[GameInstance] Starting new run...")
}

fn build_match_id(start_time: &str, line_no_hint: usize) -> String {
    let mut hasher = DefaultHasher::new();
    format!("{start_time}-{line_no_hint}").hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn start_or_resume_match(
    history: &mut HistoryRoot,
    active_match_id: &mut Option<String>,
    start_time: String,
    game_date: Option<String>,
    line_no_hint: usize,
) -> bool {
    // Playbook behavior: if there is an unfinished active session, treat new start signal
    // as potential restart/continue instead of creating a second run.
    if let Some(match_id) = active_match_id.clone() {
        if history.matches.iter().any(|m| m.match_id == match_id && !m.is_finished) {
            return false;
        }
    }

    if let Some(existing) = history.matches.iter().find(|m| {
        m.start_time == start_time
            && m.game_date == game_date
    }) {
        *active_match_id = Some(existing.match_id.clone());
        return false;
    }

    let match_id = build_match_id(&start_time, line_no_hint);
    if history.matches.iter().any(|m| m.match_id == match_id) {
        *active_match_id = Some(match_id);
        return false;
    }

    history.matches.insert(
        0,
        HistoryMatchRecord {
            match_id: match_id.clone(),
            hero: None,
            start_time,
            end_time: None,
            game_date,
            days: 1,
            victory: false,
            is_finished: false,
            pvp_battles: Vec::new(),
        },
    );
    *active_match_id = Some(match_id);
    true
}

fn merge_match_record(dst: &mut HistoryMatchRecord, src: &mut HistoryMatchRecord) {
    if dst.hero.is_none() && src.hero.is_some() {
        dst.hero = src.hero.clone();
    }
    if dst.end_time.is_none() && src.end_time.is_some() {
        dst.end_time = src.end_time.clone();
    }
    dst.days = dst.days.max(src.days);
    dst.victory = dst.victory || src.victory;
    dst.is_finished = dst.is_finished || src.is_finished;
    dst.pvp_battles.append(&mut src.pvp_battles);
}

fn normalize_history(history: &mut HistoryRoot) -> bool {
    let mut changed = false;
    let mut grouped: HashMap<(Option<String>, String), HistoryMatchRecord> = HashMap::new();

    for mut entry in std::mem::take(&mut history.matches) {
        if entry.hero.as_deref() == Some("") {
            entry.hero = None;
            changed = true;
        }
        if entry.days == 0 {
            entry.days = 1;
            changed = true;
        }

        let key = (entry.game_date.clone(), entry.start_time.clone());
        if let Some(existing) = grouped.get_mut(&key) {
            changed = true;
            merge_match_record(existing, &mut entry);
        } else {
            grouped.insert(key, entry);
        }
    }

    let mut timeline: Vec<HistoryMatchRecord> = grouped.into_values().collect();
    timeline.sort_by(|a, b| {
        let ka = format!("{} {}", a.game_date.clone().unwrap_or_default(), a.start_time);
        let kb = format!("{} {}", b.game_date.clone().unwrap_or_default(), b.start_time);
        ka.cmp(&kb)
    });

    // Playbook behavior: if the previous session is unfinished, the next start usually
    // means restart/continue; merge into the previous session.
    let mut merged: Vec<HistoryMatchRecord> = Vec::new();
    for mut entry in timeline {
        if let Some(last) = merged.last_mut() {
            if !last.is_finished {
                changed = true;
                merge_match_record(last, &mut entry);
                continue;
            }
        }
        merged.push(entry);
    }

    for entry in &mut merged {
        if entry.is_finished {
            if let (Some(start), Some(end)) = (Some(entry.start_time.clone()), entry.end_time.clone()) {
                if !start.is_empty() && !end.is_empty() && start > end {
                    // Corrupted record from old writer: keep as unfinished.
                    entry.is_finished = false;
                    entry.victory = false;
                    entry.end_time = None;
                    changed = true;
                }
            }
        }

        let mut battles = std::mem::take(&mut entry.pvp_battles);
        battles.sort_by(|a, b| {
            match (parse_time_to_millis(&a.start_time), parse_time_to_millis(&b.start_time)) {
                (Some(ta), Some(tb)) => ta.cmp(&tb),
                _ => a.day.cmp(&b.day),
            }
        });

        let mut seen = HashSet::new();
        let before = battles.len();
        battles.retain(|b| {
            let key = if b.start_time.is_empty() {
                format!("day-{}", b.day)
            } else {
                b.start_time.clone()
            };
            seen.insert(key)
        });
        if battles.len() != before {
            changed = true;
        }

        entry.pvp_battles = battles;
        if entry.days == 0 {
            entry.days = 1;
            changed = true;
        }
        if let Some(max_day) = entry.pvp_battles.iter().map(|b| b.day).max() {
            if entry.days < max_day {
                entry.days = max_day;
                changed = true;
            }
        }
    }

    merged.sort_by(|a, b| {
        let ka = format!("{} {}", a.game_date.clone().unwrap_or_default(), a.start_time);
        let kb = format!("{} {}", b.game_date.clone().unwrap_or_default(), b.start_time);
        kb.cmp(&ka)
    });

    history.matches = merged;
    changed
}

fn update_active_match<F>(history: &mut HistoryRoot, active_match_id: &Option<String>, mut updater: F) -> bool
where
    F: FnMut(&mut HistoryMatchRecord) -> bool,
{
    if let Some(match_id) = active_match_id {
        if let Some(target) = history.matches.iter_mut().find(|m| &m.match_id == match_id) {
            return updater(target);
        }
    }
    false
}

pub fn spawn_log_monitor(
    log_handle: tauri::AppHandle,
    thread_items_db: Arc<RwLock<ItemDb>>,
    thread_skills_db: Arc<RwLock<SkillDb>>,
) {
            std::thread::spawn(move || {
                let handle = log_handle;
                let log_path = get_log_path();
                let prev_path = get_prev_log_path(); // Add prev path handling
                
                let re_purchase = Regex::new(r"Card Purchased: InstanceId:\s*(?P<iid>[^ ]+)\s*-\s*TemplateId\s*(?P<tid>[^ ]+)(?:.*Target:(?P<tgt>[^ ]+))?(?:.*Section(?P<sec>[^ ]+))?").unwrap();
                let re_id = Regex::new(r"ID: \[(?P<id>[^\]]+)\]").unwrap();
                let re_tid = Regex::new(r"TemplateId: \[(?P<tid>[^\]]+)\]").unwrap();
                let re_owner = Regex::new(r"- Owner: \[(?P<val>[^\]]+)\]").unwrap();
                let re_section = Regex::new(r"- Section: \[(?P<val>[^\]]+)\]").unwrap();
                
                let re_item_id = Regex::new(r"itm_[A-Za-z0-9_-]+").unwrap();
                let re_sold = Regex::new(r"Sold Card\s+(?P<iid>itm_[^ ]+)").unwrap();
                let re_removed = Regex::new(r"Successfully removed item\s+(?P<iid>itm_[^ ]+)").unwrap();
                let re_moved_to = Regex::new(r"Successfully moved card\s+(?P<iid>itm_[^ ]+)\s+to\s+(?P<tgt>[^ ]+)").unwrap();
                let re_hero = Regex::new(r"Hero: \[(?P<hero>[^\]]+)\]").unwrap();
                let re_state_change = Regex::new(r"State changed from \[.*?\] to \[(?P<state>[^\]]+)\]").unwrap();
                let re_combat_duration = Regex::new(r"Combat simulation completed in (?P<dur>[\d\.]+)s").unwrap();

                // Initialize state
                let state_init = load_state();
                let mut inst_to_temp = state_init.inst_to_temp;
                let mut current_hand = state_init.current_hand;
                let mut current_stash = state_init.current_stash;
                let mut current_day = state_init.day;
                
                let mut in_pvp = false;
                let mut is_sync = false;
                let mut last_iid = String::new();
                let mut cur_owner = String::new();
                let mut history_changed = false;
                let mut history = HistoryRoot::default();
                let mut active_match_id: Option<String> = None;
                let mut replayed_log_count: usize = 0;
                let mut history_in_pvp = false;
                let mut history_last_pvp_start = String::new();
                let mut history_last_pvp_duration: Option<f64> = None;
                let mut recent_lines: VecDeque<String> = VecDeque::with_capacity(6);

                // --- Initial Sync: Replay Logs to catch up with current state ---
                log::debug!("[LogMonitor] Initializing state from logs...");
                
                // Clear state for fresh scan (we'll recover inst_to_temp from logs too)
                // Note: Keep cached day if it's valid, effectively we trust logs more though.
                current_hand.clear();
                current_stash.clear();
                // inst_to_temp.clear(); // Keep existing mapping as backup

                let files_to_process = vec![prev_path, log_path.clone()];
                for path in files_to_process {
                    if !path.exists() { 
                        log::debug!("[LogMonitor] Skipping non-existent file: {:?}", path);
                        continue; 
                    }
                    replayed_log_count += 1;
                    log::debug!("[LogMonitor] Processing log file: {:?}", path);
                    let file_date = std::fs::metadata(&path)
                        .ok()
                        .and_then(|m| m.modified().ok())
                        .map(|ts| {
                            let dt: DateTime<Local> = DateTime::from(ts);
                            dt.format("%Y-%m-%d").to_string()
                        });
                    if let Ok(file) = File::open(&path) {
                        let reader = BufReader::new(file);
                        for (line_idx, line) in reader.lines().enumerate() {
                            if let Ok(l) = line {
                                let trimmed = l.trim();

                                if recent_lines.len() >= 6 {
                                    recent_lines.pop_front();
                                }
                                recent_lines.push_back(trimmed.to_string());

                                // Reset everything if we see a new run start.
                                if is_run_init_line(trimmed) {
                                    current_day = 1;
                                    in_pvp = false;
                                    history_in_pvp = false;
                                    history_last_pvp_duration = None;
                                    inst_to_temp.clear();
                                    current_hand.clear();
                                    current_stash.clear();
                                    is_sync = false;

                                    let start_time = extract_timestamp(trimmed)
                                        .unwrap_or_else(|| Local::now().format("%H:%M:%S%.3f").to_string());
                                    if start_or_resume_match(
                                        &mut history,
                                        &mut active_match_id,
                                        start_time,
                                        file_date.clone(),
                                        line_idx,
                                    ) {
                                        history_changed = true;
                                    }
                                }

                                if let Some(cap) = re_hero.captures(trimmed) {
                                    let hero_name = cap["hero"].to_string();
                                    if update_active_match(&mut history, &active_match_id, |m| {
                                        if m.hero.as_deref() == Some(hero_name.as_str()) {
                                            false
                                        } else {
                                            m.hero = Some(hero_name.clone());
                                            true
                                        }
                                    }) {
                                        history_changed = true;
                                    }
                                }

                                if let Some(cap) = re_combat_duration.captures(trimmed) {
                                    if let Ok(duration) = cap["dur"].parse::<f64>() {
                                        history_last_pvp_duration = Some(duration);
                                    }
                                }

                                if let Some(cap) = re_state_change.captures(trimmed) {
                                    let next_state = cap["state"].to_string();
                                    if next_state == "PVPCombatState" {
                                        history_in_pvp = true;
                                        history_last_pvp_start = extract_timestamp(trimmed).unwrap_or_default();
                                        history_last_pvp_duration = None;
                                    } else if next_state == "ReplayState" && history_in_pvp {
                                        let battle_victory = recent_lines
                                            .iter()
                                            .rev()
                                            .nth(3)
                                            .map(|line| line.contains("All exit tasks completed"))
                                            .unwrap_or(false);

                                        if update_active_match(&mut history, &active_match_id, |m| {
                                            let start_time = if history_last_pvp_start.is_empty() {
                                                extract_timestamp(trimmed).unwrap_or_default()
                                            } else {
                                                history_last_pvp_start.clone()
                                            };
                                            let battle_day = m.days.max(current_day.max(1));
                                            let duplicated = m.pvp_battles.iter().any(|b| b.start_time == start_time);
                                            if duplicated {
                                                return false;
                                            }
                                            m.pvp_battles.push(HistoryBattleRecord {
                                                day: battle_day,
                                                start_time,
                                                victory: battle_victory,
                                                duration: history_last_pvp_duration,
                                                screenshot: None,
                                            });
                                            m.days = battle_day.saturating_add(1);
                                            true
                                        }) {
                                            history_changed = true;
                                        }
                                        history_in_pvp = false;
                                    } else if next_state == "EndRunVictoryState" || next_state == "EndRunDefeatState" {
                                        let is_victory = next_state == "EndRunVictoryState";
                                        if update_active_match(&mut history, &active_match_id, |m| {
                                            m.end_time = extract_timestamp(trimmed);
                                            m.victory = is_victory;
                                            m.is_finished = true;
                                            true
                                        }) {
                                            history_changed = true;
                                        }
                                        active_match_id = None;
                                        history_in_pvp = false;
                                        history_last_pvp_duration = None;
                                    }
                                }

                                if trimmed.contains("to [PVPCombatState]") { in_pvp = true; }
                                if in_pvp && trimmed.contains("State changed") && (trimmed.contains("to [ChoiceState]") || trimmed.contains("to [LevelUpState]")) {
                                    current_day = current_day.saturating_add(1);
                                    in_pvp = false;
                                    if update_active_match(&mut history, &active_match_id, |m| {
                                        if m.days == current_day {
                                            false
                                        } else {
                                            m.days = current_day;
                                            true
                                        }
                                    }) {
                                        history_changed = true;
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
                                    else if let Some(cap) = re_tid.captures(trimmed) {
                                        if !last_iid.is_empty() {
                                            inst_to_temp.insert(last_iid.clone(), cap["tid"].to_string());
                                        }
                                    }
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

                if replayed_log_count == 0 {
                    history = load_history_root();
                    if normalize_history(&mut history) {
                        history_changed = true;
                    }
                } else if !history.matches.is_empty() {
                    let _ = normalize_history(&mut history);
                    // Rebuild-from-log should replace stale on-disk history.
                    history_changed = true;
                }

                if active_match_id.is_none() {
                    active_match_id = history
                        .matches
                        .iter()
                        .find(|m| !m.is_finished)
                        .map(|m| m.match_id.clone());
                }

                save_state(&PersistentState {
                    day: current_day,
                    inst_to_temp: inst_to_temp.clone(),
                    current_hand: current_hand.clone(),
                    current_stash: current_stash.clone(),
                    ..load_state()
                });

                if history_changed {
                    let _ = normalize_history(&mut history);
                    match save_history_root(&history) {
                        Ok(_) => {
                            let _ = handle.emit("match-history-updated", ());
                        }
                        Err(e) => {
                            log::warn!("[LogMonitor] Failed to save match history after bootstrap scan: {}", e);
                        }
                    }
                    history_changed = false;
                }

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
                    let map_items = |ids: &HashSet<String>| -> Vec<ItemData> {
                        let mut ordered_ids: Vec<&String> = ids.iter().collect();
                        ordered_ids.sort();
                        ordered_ids
                           .into_iter()
                           .filter_map(|iid| {
                               let tid = init_map.get(iid)?;
                               let mut item = lookup_item(tid, &items_db, &skills_db)?;
                               item.instance_id = Some(iid.clone());
                               Some(item)
                           })
                           .collect()
                    };
                    let hand_items = map_items(&init_hand);
                    let stash_items = map_items(&init_stash);
                    let all_tags = items_db.unique_tags.clone();
                    let _ = init_handle.emit("sync-items", SyncPayload { hand_items, stash_items, all_tags });
                });

                let mut last_size = 0u64;
                if let Ok(meta) = std::fs::metadata(&log_path) {
                    last_size = meta.len();
                }

                log::debug!("[Log Monitor] Initialization complete. Starting main monitoring loop from size: {}", last_size);

                loop {
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    
                    if let Ok(mut f) = File::open(&log_path) {
                        if let Ok(meta) = f.metadata() {
                            let len = meta.len();
                            if len < last_size {
                                last_size = 0;
                                in_pvp = false;
                                history_in_pvp = false;
                                history_last_pvp_duration = None;
                                is_sync = false;
                                recent_lines.clear();
                            }
                            if len > last_size {
                                if let Ok(_) = f.seek(SeekFrom::Start(last_size)) {
                                    let mut buf = Vec::new();
                                    if let Ok(_) = f.take(1_000_000).read_to_end(&mut buf) {
                                        let new_content = String::from_utf8_lossy(&buf);
                                        let mut changed = false;
                                        let mut day_changed = false;
                                        let current_file_date = meta
                                            .modified()
                                            .ok()
                                            .map(|ts| {
                                                let dt: DateTime<Local> = DateTime::from(ts);
                                                dt.format("%Y-%m-%d").to_string()
                                            });

                                        for (line_idx, line) in new_content.lines().enumerate() {
                                            let trimmed = line.trim();

                                            if recent_lines.len() >= 6 {
                                                recent_lines.pop_front();
                                            }
                                            recent_lines.push_back(trimmed.to_string());

                                            if is_run_init_line(trimmed) {
                                                current_day = 1;
                                                in_pvp = false;
                                                history_in_pvp = false;
                                                history_last_pvp_duration = None;
                                                day_changed = true;
                                                inst_to_temp.clear();
                                                current_hand.clear();
                                                current_stash.clear();
                                                changed = true;

                                                let start_time = extract_timestamp(trimmed)
                                                    .unwrap_or_else(|| Local::now().format("%H:%M:%S%.3f").to_string());
                                                if start_or_resume_match(
                                                    &mut history,
                                                    &mut active_match_id,
                                                    start_time,
                                                    current_file_date.clone(),
                                                    line_idx,
                                                ) {
                                                    history_changed = true;
                                                }
                                            }

                                            if let Some(cap) = re_hero.captures(trimmed) {
                                                let hero_name = cap["hero"].to_string();
                                                if update_active_match(&mut history, &active_match_id, |m| {
                                                    if m.hero.as_deref() == Some(hero_name.as_str()) {
                                                        false
                                                    } else {
                                                        m.hero = Some(hero_name.clone());
                                                        true
                                                    }
                                                }) {
                                                    history_changed = true;
                                                }
                                            }

                                            if let Some(cap) = re_combat_duration.captures(trimmed) {
                                                if let Ok(duration) = cap["dur"].parse::<f64>() {
                                                    history_last_pvp_duration = Some(duration);
                                                }
                                            }

                                            if let Some(cap) = re_state_change.captures(trimmed) {
                                                let next_state = cap["state"].to_string();
                                                if next_state == "PVPCombatState" {
                                                    history_in_pvp = true;
                                                    history_last_pvp_start = extract_timestamp(trimmed).unwrap_or_default();
                                                    history_last_pvp_duration = None;
                                                } else if next_state == "ReplayState" && history_in_pvp {
                                                    let battle_victory = recent_lines
                                                        .iter()
                                                        .rev()
                                                        .nth(3)
                                                        .map(|line| line.contains("All exit tasks completed"))
                                                        .unwrap_or(false);

                                                    if update_active_match(&mut history, &active_match_id, |m| {
                                                        let start_time = if history_last_pvp_start.is_empty() {
                                                            extract_timestamp(trimmed).unwrap_or_default()
                                                        } else {
                                                            history_last_pvp_start.clone()
                                                        };
                                                        let battle_day = m.days.max(current_day.max(1));
                                                        let duplicated = m.pvp_battles.iter().any(|b| b.start_time == start_time);
                                                        if duplicated {
                                                            return false;
                                                        }
                                                        m.pvp_battles.push(HistoryBattleRecord {
                                                            day: battle_day,
                                                            start_time,
                                                            victory: battle_victory,
                                                            duration: history_last_pvp_duration,
                                                            screenshot: None,
                                                        });
                                                        m.days = battle_day.saturating_add(1);
                                                        true
                                                    }) {
                                                        history_changed = true;
                                                    }
                                                    history_in_pvp = false;
                                                } else if next_state == "EndRunVictoryState" || next_state == "EndRunDefeatState" {
                                                    let is_victory = next_state == "EndRunVictoryState";
                                                    if update_active_match(&mut history, &active_match_id, |m| {
                                                        m.end_time = extract_timestamp(trimmed);
                                                        m.victory = is_victory;
                                                        m.is_finished = true;
                                                        true
                                                    }) {
                                                        history_changed = true;
                                                    }
                                                    active_match_id = None;
                                                    history_in_pvp = false;
                                                    history_last_pvp_duration = None;
                                                }
                                            }
                                            
                                            if trimmed.contains("to [PVPCombatState]") { in_pvp = true; }
                                            if in_pvp && trimmed.contains("State changed") && (trimmed.contains("to [ChoiceState]") || trimmed.contains("to [LevelUpState]")) {
                                                current_day = current_day.saturating_add(1);
                                                in_pvp = false;
                                                day_changed = true;
                                                if update_active_match(&mut history, &active_match_id, |m| {
                                                    if m.days == current_day {
                                                        false
                                                    } else {
                                                        m.days = current_day;
                                                        true
                                                    }
                                                }) {
                                                    history_changed = true;
                                                }
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
                                                
                                                if section.as_deref().unwrap_or("") == "" {
                                                    if let Some(tgt) = target {
                                                        if tgt.contains("PlayerStorageSocket") { section = Some("Stash".to_string()); }
                                                        else if tgt.contains("PlayerSocket") { section = Some("Player".to_string()); }
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
                                                    }
                                                    else if s == "Stash" || s == "Storage" || s == "PlayerStorage" {
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
                                                if current_hand.remove(&iid) || current_stash.remove(&iid) { changed = true; }
                                            }
                                            if let Some(cap) = re_removed.captures(trimmed) {
                                                let iid = cap["iid"].to_string();
                                                if current_hand.remove(&iid) || current_stash.remove(&iid) { changed = true; }
                                            }
                                            if trimmed.contains("Cards Disposed:") {
                                                for mat in re_item_id.find_iter(trimmed) {
                                                    let iid = mat.as_str().to_string();
                                                    if current_hand.remove(&iid) || current_stash.remove(&iid) { changed = true; }
                                                }
                                            }

                                            if trimmed.contains("Cards Spawned:") || trimmed.contains("Cards Dealt:") || trimmed.contains("NetMessageGameStateSync") || trimmed.contains("Successfully moved card to:") {
                                                is_sync = true;
                                            }
                                            
                                            if is_sync {
                                                if let Some(cap) = re_id.captures(trimmed) { last_iid = cap["id"].to_string(); }
                                                else if let Some(cap) = re_tid.captures(trimmed) {
                                                    if !last_iid.is_empty() {
                                                        let tid = cap["tid"].to_string();
                                                        let old_tid = inst_to_temp.insert(last_iid.clone(), tid.clone());
                                                        if old_tid.as_deref() != Some(tid.as_str()) {
                                                            changed = true;
                                                        }
                                                    }
                                                }
                                                else if let Some(cap) = re_owner.captures(trimmed) { cur_owner = cap["val"].to_string(); }
                                                else if let Some(cap) = re_section.captures(trimmed) {
                                                    if !last_iid.is_empty() && &cur_owner == "Player" && last_iid.starts_with("itm_") {
                                                        let sec_val = &cap["val"];
                                                        if sec_val == "Hand" || sec_val == "Player" { 
                                                            if current_hand.insert(last_iid.clone()) {
                                                                changed = true;
                                                            }
                                                            if current_stash.remove(&last_iid) {
                                                                changed = true;
                                                            }
                                                        } else if sec_val == "Stash" || sec_val == "Storage" || sec_val == "PlayerStorage" { 
                                                            if current_stash.insert(last_iid.clone()) {
                                                                changed = true;
                                                            }
                                                            if current_hand.remove(&last_iid) {
                                                                changed = true;
                                                            }
                                                        } else {
                                                            if current_hand.remove(&last_iid) {
                                                                changed = true;
                                                            }
                                                            if current_stash.remove(&last_iid) {
                                                                changed = true;
                                                            }
                                                        }
                                                    }
                                                    last_iid.clear(); cur_owner.clear();
                                                }
                                                else if trimmed.contains("Finished processing") { is_sync = false; }
                                            }
                                        }

                                        if changed || day_changed {
                                            if day_changed {
                                                let _ = handle.emit("day-update", current_day);
                                            }
                                            
                                            if changed {
                                                let items_db = thread_items_db.read().unwrap();
                                                let skills_db = thread_skills_db.read().unwrap();
                                                
                                                let map_items = |ids: &HashSet<String>| -> Vec<ItemData> {
                                                    let mut ordered_ids: Vec<&String> = ids.iter().collect();
                                                    ordered_ids.sort();
                                                    ordered_ids
                                                       .into_iter()
                                                       .filter_map(|iid| {
                                                           let tid = inst_to_temp.get(iid)?;
                                                           let mut item = lookup_item(tid, &items_db, &skills_db)?;
                                                           item.instance_id = Some(iid.clone());
                                                           Some(item)
                                                       })
                                                       .collect()
                                                };

                                                let hand_items = map_items(&current_hand);
                                                let stash_items = map_items(&current_stash);
                                                let all_tags = items_db.unique_tags.clone();
                                                
                                                let _ = handle.emit("sync-items", SyncPayload { hand_items, stash_items, all_tags });
                                                
                                                save_state(&PersistentState {
                                                    day: current_day,
                                                    inst_to_temp: inst_to_temp.clone(),
                                                    current_hand: current_hand.clone(),
                                                    current_stash: current_stash.clone(),
                                                    ..load_state()
                                                });
                                            }
                                        }

                                        if history_changed {
                                            let _ = normalize_history(&mut history);
                                            match save_history_root(&history) {
                                                Ok(_) => {
                                                    let _ = handle.emit("match-history-updated", ());
                                                }
                                                Err(e) => {
                                                    log::warn!("[LogMonitor] Failed to save match history during tailing: {}", e);
                                                }
                                            }
                                            history_changed = false;
                                        }
                                        last_size = len;
                                    }
                                }
                            }
                        }
                    }
                }
            });
}
