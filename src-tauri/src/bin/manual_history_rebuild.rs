use tauri_app_lib::data_management::commands::rebuild_match_history;

fn main() {
    println!("[ManualRebuild] Start force rebuild with screenshot visual backfill...");

    let rebuilt = match rebuild_match_history(Some(true)) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("[ManualRebuild] rebuild failed: {}", e);
            std::process::exit(1);
        }
    };

    let matches = rebuilt
        .get("matches")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    println!("[ManualRebuild] total matches: {}", matches.len());

    let mut total_battles = 0usize;
    let mut screenshot_battles = 0usize;
    let mut lineup_filled_battles = 0usize;
    let mut screenshot_samples: Vec<String> = Vec::new();

    for m in &matches {
        let battles = m
            .get("pvp_battles")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        for b in battles {
            total_battles += 1;
            let shot_path = b
                .get("screenshot")
                .and_then(|v| v.as_str())
                .map(|s| s.trim())
                .unwrap_or("");
            if !shot_path.is_empty() {
                screenshot_battles += 1;
                if screenshot_samples.len() < 12 {
                    let day = b.get("day").and_then(|v| v.as_u64()).unwrap_or(0);
                    screenshot_samples.push(format!("day{} -> {}", day, shot_path));
                }
            }
            let self_count = b
                .get("lineup_cards")
                .and_then(|v| v.as_array())
                .map(|arr| arr.len())
                .unwrap_or(0);
            let enemy_count = b
                .get("enemy_lineup_cards")
                .and_then(|v| v.as_array())
                .map(|arr| arr.len())
                .unwrap_or(0);
            if self_count > 0 || enemy_count > 0 {
                lineup_filled_battles += 1;
            }
        }
    }

    println!(
        "[ManualRebuild] battles total={}, with_screenshot={}, with_lineup={}",
        total_battles, screenshot_battles, lineup_filled_battles
    );
    if !screenshot_samples.is_empty() {
        println!("[ManualRebuild] screenshot samples:");
        for s in &screenshot_samples {
            println!("  - {}", s);
        }
    }

    for (idx, m) in matches.iter().take(10).enumerate() {
        let match_id = m.get("match_id").and_then(|v| v.as_str()).unwrap_or("");
        let game_date = m.get("game_date").and_then(|v| v.as_str()).unwrap_or("");
        let start_time = m.get("start_time").and_then(|v| v.as_str()).unwrap_or("");
        let battles = m
            .get("pvp_battles")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        println!(
            "[Match#{:02}] {} {} {} battles={} ",
            idx + 1,
            game_date,
            start_time,
            match_id,
            battles.len()
        );

        for b in battles.iter().rev().take(6) {
            let day = b.get("day").and_then(|v| v.as_u64()).unwrap_or(0);
            let self_count = b
                .get("lineup_cards")
                .and_then(|v| v.as_array())
                .map(|arr| arr.len())
                .unwrap_or(0);
            let enemy_count = b
                .get("enemy_lineup_cards")
                .and_then(|v| v.as_array())
                .map(|arr| arr.len())
                .unwrap_or(0);
            let shot = b
                .get("screenshot")
                .and_then(|v| v.as_str())
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false);

            let self_cards = b
                .get("lineup_cards")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .take(4)
                        .filter_map(|c| c.get("name_cn").and_then(|v| v.as_str()))
                        .collect::<Vec<_>>()
                        .join("/")
                })
                .unwrap_or_default();

            println!(
                "  - Day {:>2}: self={} enemy={} screenshot={} sample=[{}]",
                day, self_count, enemy_count, shot, self_cards
            );
        }
    }

    println!("[ManualRebuild] Done.");
}
