use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use tauri_app_lib::data_management::commands::rebuild_match_history;
use tauri_app_lib::user_data::save_match_history;

fn test_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .expect("test lock poisoned")
}

struct EnvRestore {
    old_userprofile: Option<String>,
}

impl Drop for EnvRestore {
    fn drop(&mut self) {
        if let Some(v) = self.old_userprofile.take() {
            std::env::set_var("USERPROFILE", v);
        } else {
            std::env::remove_var("USERPROFILE");
        }
    }
}

fn prepare_test_profile() -> Result<PathBuf, Box<dyn Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests");
    let run_id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let profile_dir = root.join("runtime_profile").join(format!("run_{run_id}"));
    let log_dir = profile_dir
        .join("AppData")
        .join("LocalLow")
        .join("Tempo Storm")
        .join("The Bazaar");
    fs::create_dir_all(&log_dir)?;

    fs::copy(
        root.join("records").join("Player.log"),
        log_dir.join("Player.log"),
    )?;
    fs::copy(
        root.join("records").join("Player-prev.log"),
        log_dir.join("Player-prev.log"),
    )?;

    Ok(profile_dir)
}

fn string_field<'a>(obj: &'a serde_json::Value, key: &str) -> &'a str {
    obj.get(key).and_then(|v| v.as_str()).unwrap_or("")
}

#[test]
fn analyze_records_with_history_rebuild() -> Result<(), Box<dyn Error>> {
    let _guard = test_lock();
    let _restore = EnvRestore {
        old_userprofile: std::env::var("USERPROFILE").ok(),
    };

    let profile_dir = prepare_test_profile()?;
    std::env::set_var("USERPROFILE", &profile_dir);

    let rebuilt = rebuild_match_history(Some(true))
        .map_err(|e| format!("rebuild_match_history failed: {e}"))?;
    let matches = rebuilt
        .get("matches")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    println!("total matches: {}", matches.len());

    for (idx, m) in matches.iter().enumerate() {
        let hero = string_field(m, "hero");
        let start = string_field(m, "start_time");
        let date = string_field(m, "game_date");
        let battles = m
            .get("pvp_battles")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let wins = battles
            .iter()
            .filter(|b| b.get("victory").and_then(|v| v.as_bool()).unwrap_or(false))
            .count();
        let losses = battles.len().saturating_sub(wins);
        println!(
            "match[{idx}] {date} {start} hero={hero} wins={wins} losses={losses} finished={}",
            m.get("is_finished")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        );
    }

    let latest = matches.iter().max_by_key(|m| {
        (
            string_field(m, "game_date").to_string(),
            string_field(m, "start_time").to_string(),
        )
    });

    let latest = latest.ok_or("no parsed matches from records logs")?;
    let latest_hero = string_field(latest, "hero");
    let latest_battles = latest
        .get("pvp_battles")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let latest_wins = latest_battles
        .iter()
        .filter(|b| b.get("victory").and_then(|v| v.as_bool()).unwrap_or(false))
        .count();
    let latest_losses = latest_battles.len().saturating_sub(latest_wins);

    println!(
        "LATEST => date={} start={} hero={} wins={} losses={} battles={}",
        string_field(latest, "game_date"),
        string_field(latest, "start_time"),
        latest_hero,
        latest_wins,
        latest_losses,
        latest_battles.len()
    );

    assert!(
        !latest_battles.is_empty(),
        "latest match should include at least one pvp battle"
    );

    Ok(())
}

#[test]
fn force_rebuild_should_not_accumulate_duplicate_battles() -> Result<(), Box<dyn Error>> {
    let _guard = test_lock();
    let _restore = EnvRestore {
        old_userprofile: std::env::var("USERPROFILE").ok(),
    };

    let profile_dir = prepare_test_profile()?;
    std::env::set_var("USERPROFILE", &profile_dir);

    let baseline = rebuild_match_history(Some(true))
        .map_err(|e| format!("baseline rebuild failed: {e}"))?;

    let mut polluted = baseline.clone();
    if let Some(first_match) = polluted
        .get_mut("matches")
        .and_then(|v| v.as_array_mut())
        .and_then(|arr| arr.get_mut(0))
    {
        first_match["days"] = serde_json::json!(29);
        if let Some(battles) = first_match.get_mut("pvp_battles").and_then(|v| v.as_array_mut()) {
            if let Some(seed) = battles.first().cloned() {
                let mut extra_a = seed.clone();
                extra_a["day"] = serde_json::json!(28);
                extra_a["victory"] = serde_json::json!(false);
                let mut extra_b = seed;
                extra_b["day"] = serde_json::json!(29);
                extra_b["victory"] = serde_json::json!(true);
                battles.push(extra_a);
                battles.push(extra_b);
            }
        }
    }

    save_match_history(&polluted).map_err(|e| format!("save polluted history failed: {e}"))?;

    let rebuilt_once = rebuild_match_history(Some(true))
        .map_err(|e| format!("rebuild once failed: {e}"))?;
    let rebuilt_twice = rebuild_match_history(Some(true))
        .map_err(|e| format!("rebuild twice failed: {e}"))?;

    let read_stats = |root: &serde_json::Value| -> (usize, usize, usize) {
        let first_match = root
            .get("matches")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .cloned()
            .unwrap_or_default();
        let battles = first_match
            .get("pvp_battles")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let wins = battles
            .iter()
            .filter(|b| b.get("victory").and_then(|v| v.as_bool()).unwrap_or(false))
            .count();
        let losses = battles.len().saturating_sub(wins);
        (battles.len(), wins, losses)
    };

    let s1 = read_stats(&rebuilt_once);
    let s2 = read_stats(&rebuilt_twice);

    println!("rebuild#1 battles/w/l = {:?}", s1);
    println!("rebuild#2 battles/w/l = {:?}", s2);

    assert_eq!(s1, (11, 10, 1), "first force rebuild should recover clean log result");
    assert_eq!(s2, (11, 10, 1), "second force rebuild should stay stable");

    Ok(())
}
