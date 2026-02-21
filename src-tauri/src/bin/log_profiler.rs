use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use clap::Parser;
use regex::Regex;
use sysinfo::{Pid, ProcessesToUpdate, System};

#[derive(Parser, Debug)]
#[command(name = "log_profiler")]
#[command(about = "Standalone log parser memory profiler", long_about = None)]
struct Args {
    #[arg(long)]
    log: PathBuf,
    #[arg(long)]
    prev: Option<PathBuf>,
    #[arg(long, default_value_t = 2)]
    interval_sec: u64,
    #[arg(long, default_value_t = 256000)]
    tail_chunk_bytes: u64,
}

#[derive(Default)]
struct ParserState {
    inst_to_temp: HashMap<String, String>,
    current_hand: HashSet<String>,
    current_stash: HashSet<String>,
    is_sync: bool,
    last_iid: String,
    cur_owner: String,
    total_lines: u64,
    lines_in_window: u64,
}

fn estimate_state_bytes(state: &ParserState) -> u64 {
    let map_bytes = state
        .inst_to_temp
        .iter()
        .map(|(k, v)| (k.capacity() + v.capacity()) as u64 + 64)
        .sum::<u64>();
    let hand_bytes = state
        .current_hand
        .iter()
        .map(|k| k.capacity() as u64 + 48)
        .sum::<u64>();
    let stash_bytes = state
        .current_stash
        .iter()
        .map(|k| k.capacity() as u64 + 48)
        .sum::<u64>();
    map_bytes + hand_bytes + stash_bytes
}

fn process_line(
    line: &str,
    state: &mut ParserState,
    re_purchase: &Regex,
    re_id: &Regex,
    re_tid: &Regex,
    re_owner: &Regex,
    re_section: &Regex,
    re_item_id: &Regex,
    re_sold: &Regex,
    re_removed: &Regex,
    re_moved_to: &Regex,
) {
    let trimmed = line.trim();
    state.total_lines = state.total_lines.saturating_add(1);
    state.lines_in_window = state.lines_in_window.saturating_add(1);

    if trimmed.contains("[GameInstance] Starting new run...") {
        state.inst_to_temp.clear();
        state.current_hand.clear();
        state.current_stash.clear();
        state.is_sync = false;
        state.last_iid.clear();
        state.cur_owner.clear();
    }

    if let Some(cap) = re_purchase.captures(trimmed) {
        let iid = cap["iid"].to_string();
        state.inst_to_temp.insert(iid.clone(), cap["tid"].to_string());

        let mut section = cap.name("sec").map(|s| s.as_str().to_string());
        if section.as_deref().unwrap_or("").is_empty() {
            if let Some(tgt) = cap.name("tgt").map(|t| t.as_str()) {
                if tgt.contains("PlayerStorageSocket") {
                    section = Some("Stash".to_string());
                } else if tgt.contains("PlayerSocket") {
                    section = Some("Player".to_string());
                }
            }
        }
        if let Some(s) = section {
            if s == "Player" || s == "Hand" {
                state.current_hand.insert(iid.clone());
                state.current_stash.remove(&iid);
            } else if s == "Stash" || s == "Storage" || s == "PlayerStorage" {
                state.current_stash.insert(iid.clone());
                state.current_hand.remove(&iid);
            }
        }
    }

    if let Some(cap) = re_moved_to.captures(trimmed) {
        let iid = cap["iid"].to_string();
        if cap["tgt"].contains("StorageSocket") {
            state.current_stash.insert(iid.clone());
            state.current_hand.remove(&iid);
        } else if cap["tgt"].contains("Socket") {
            state.current_hand.insert(iid.clone());
            state.current_stash.remove(&iid);
        }
    }

    if let Some(cap) = re_sold.captures(trimmed) {
        let iid = cap["iid"].to_string();
        state.current_hand.remove(&iid);
        state.current_stash.remove(&iid);
    }
    if let Some(cap) = re_removed.captures(trimmed) {
        let iid = cap["iid"].to_string();
        state.current_hand.remove(&iid);
        state.current_stash.remove(&iid);
    }
    if trimmed.contains("Cards Disposed:") {
        for mat in re_item_id.find_iter(trimmed) {
            let iid = mat.as_str().to_string();
            state.current_hand.remove(&iid);
            state.current_stash.remove(&iid);
        }
    }

    if trimmed.contains("Cards Spawned:")
        || trimmed.contains("Cards Dealt:")
        || trimmed.contains("NetMessageGameStateSync")
    {
        state.is_sync = true;
    }
    if state.is_sync {
        if let Some(cap) = re_id.captures(trimmed) {
            state.last_iid = cap["id"].to_string();
        } else if let Some(cap) = re_tid.captures(trimmed) {
            if !state.last_iid.is_empty() {
                state
                    .inst_to_temp
                    .insert(state.last_iid.clone(), cap["tid"].to_string());
            }
        } else if let Some(cap) = re_owner.captures(trimmed) {
            state.cur_owner = cap["val"].to_string();
        } else if let Some(cap) = re_section.captures(trimmed) {
            if !state.last_iid.is_empty() && state.cur_owner == "Player" && state.last_iid.starts_with("itm_") {
                let sec = &cap["val"];
                if sec == "Hand" || sec == "Player" {
                    state.current_hand.insert(state.last_iid.clone());
                    state.current_stash.remove(&state.last_iid);
                } else if sec == "Stash" || sec == "Storage" || sec == "PlayerStorage" {
                    state.current_stash.insert(state.last_iid.clone());
                    state.current_hand.remove(&state.last_iid);
                } else {
                    state.current_hand.remove(&state.last_iid);
                    state.current_stash.remove(&state.last_iid);
                }
            }
            state.last_iid.clear();
            state.cur_owner.clear();
        } else if trimmed.contains("Finished processing") {
            state.is_sync = false;
        }
    }

    if state.inst_to_temp.len() > 2048 {
        state
            .inst_to_temp
            .retain(|iid, _| state.current_hand.contains(iid) || state.current_stash.contains(iid));
    }
}

fn read_rss_bytes(sys: &mut System, pid: Pid) -> u64 {
    sys.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);
    sys.process(pid).map(|p| p.memory()).unwrap_or(0)
}

fn replay_file(
    path: &PathBuf,
    state: &mut ParserState,
    tail_chunk_bytes: u64,
    re_purchase: &Regex,
    re_id: &Regex,
    re_tid: &Regex,
    re_owner: &Regex,
    re_section: &Regex,
    re_item_id: &Regex,
    re_sold: &Regex,
    re_removed: &Regex,
    re_moved_to: &Regex,
) -> std::io::Result<u64> {
    let mut file = File::open(path)?;
    let start_offset = file
        .metadata()
        .ok()
        .map(|m| m.len().saturating_sub(tail_chunk_bytes))
        .unwrap_or(0);
    file.seek(SeekFrom::Start(start_offset))?;
    let mut reader = BufReader::new(file);
    if start_offset > 0 {
        let mut throwaway = String::new();
        let _ = reader.read_line(&mut throwaway);
    }
    for line in reader.lines() {
        process_line(
            &line?,
            state,
            re_purchase,
            re_id,
            re_tid,
            re_owner,
            re_section,
            re_item_id,
            re_sold,
            re_removed,
            re_moved_to,
        );
    }
    Ok(std::fs::metadata(path)?.len())
}

fn main() -> Result<(), String> {
    let args = Args::parse();
    if !args.log.exists() {
        return Err(format!("log file not found: {:?}", args.log));
    }

    let re_purchase = Regex::new(r"Card Purchased: InstanceId:\s*(?P<iid>[^ ]+)\s*-\s*TemplateId\s*(?P<tid>[^ ]+)(?:.*Target:(?P<tgt>[^ ]+))?(?:.*Section(?P<sec>[^ ]+))?")
        .map_err(|e| e.to_string())?;
    let re_id = Regex::new(r"ID: \[(?P<id>[^\]]+)\]").map_err(|e| e.to_string())?;
    let re_tid = Regex::new(r"TemplateId: \[(?P<tid>[^\]]+)\]").map_err(|e| e.to_string())?;
    let re_owner = Regex::new(r"- Owner: \[(?P<val>[^\]]+)\]").map_err(|e| e.to_string())?;
    let re_section = Regex::new(r"- Section: \[(?P<val>[^\]]+)\]").map_err(|e| e.to_string())?;
    let re_item_id = Regex::new(r"itm_[A-Za-z0-9_-]+").map_err(|e| e.to_string())?;
    let re_sold = Regex::new(r"Sold Card\s+(?P<iid>itm_[^ ]+)").map_err(|e| e.to_string())?;
    let re_removed =
        Regex::new(r"Successfully removed item\s+(?P<iid>itm_[^ ]+)").map_err(|e| e.to_string())?;
    let re_moved_to = Regex::new(r"Successfully moved card\s+(?P<iid>itm_[^ ]+)\s+to\s+(?P<tgt>[^ ]+)")
        .map_err(|e| e.to_string())?;

    let mut state = ParserState::default();
    let pid = sysinfo::get_current_pid().map_err(|e| e.to_string())?;
    let mut system = System::new_all();
    let start = Instant::now();
    let mut last_print = Instant::now();

    if let Some(prev) = &args.prev {
        if prev.exists() {
            let prev_size = replay_file(
                prev,
                &mut state,
                args.tail_chunk_bytes,
                &re_purchase,
                &re_id,
                &re_tid,
                &re_owner,
                &re_section,
                &re_item_id,
                &re_sold,
                &re_removed,
                &re_moved_to,
            )
            .unwrap_or(0);
            println!("[bootstrap] replayed prev: {:?}, size={}", prev, prev_size);
        }
    }

    let mut last_size = replay_file(
        &args.log,
        &mut state,
        args.tail_chunk_bytes,
        &re_purchase,
        &re_id,
        &re_tid,
        &re_owner,
        &re_section,
        &re_item_id,
        &re_sold,
        &re_removed,
        &re_moved_to,
    )
    .map_err(|e| e.to_string())?;
    println!("[bootstrap] replayed main: {:?}, size={}", args.log, last_size);

    loop {
        std::thread::sleep(Duration::from_millis(300));
        let mut did_work = false;

        if let Ok(mut f) = File::open(&args.log) {
            if let Ok(meta) = f.metadata() {
                let len = meta.len();
                if len < last_size {
                    last_size = 0;
                }
                if len > last_size {
                    if f.seek(SeekFrom::Start(last_size)).is_ok() {
                        let mut reader = BufReader::new(f);
                        let mut bytes_read = 0_u64;
                        let mut line = String::new();
                        while bytes_read < args.tail_chunk_bytes {
                            line.clear();
                            let n = reader.read_line(&mut line).unwrap_or(0);
                            if n == 0 {
                                break;
                            }
                            bytes_read = bytes_read.saturating_add(n as u64);
                            process_line(
                                &line,
                                &mut state,
                                &re_purchase,
                                &re_id,
                                &re_tid,
                                &re_owner,
                                &re_section,
                                &re_item_id,
                                &re_sold,
                                &re_removed,
                                &re_moved_to,
                            );
                        }
                        last_size = last_size.saturating_add(bytes_read).min(len);
                        did_work = true;
                    }
                }
            }
        }

        if did_work || last_print.elapsed() >= Duration::from_secs(args.interval_sec) {
            let rss = read_rss_bytes(&mut system, pid);
            let est = estimate_state_bytes(&state);
            let elapsed = start.elapsed().as_secs_f32();
            let lps = if args.interval_sec == 0 {
                0.0
            } else {
                state.lines_in_window as f32 / args.interval_sec as f32
            };
            println!(
                "[t={:.1}s] rss={:.2}MB est_state={:.2}MB lines={} (+{} | {:.1} l/s) map={} hand={} stash={} last_size={}",
                elapsed,
                rss as f64 / 1024.0 / 1024.0,
                est as f64 / 1024.0 / 1024.0,
                state.total_lines,
                state.lines_in_window,
                lps,
                state.inst_to_temp.len(),
                state.current_hand.len(),
                state.current_stash.len(),
                last_size
            );
            state.lines_in_window = 0;
            last_print = Instant::now();
        }
    }
}
