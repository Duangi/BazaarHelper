use std::path::{Path, PathBuf};

use rusqlite::{params, Connection};

pub mod commands;
pub mod state_store;

pub fn app_data_root() -> PathBuf {
    if cfg!(target_os = "macos") {
        let home = std::env::var("HOME").unwrap_or_default();
        PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("com.duang.BazaarHelper")
    } else {
        let home = std::env::var("USERPROFILE").unwrap_or_default();
        PathBuf::from(home)
            .join("AppData")
            .join("Local")
            .join("BazaarHelper")
    }
}

pub fn state_cache_path() -> PathBuf {
    app_data_root().join("state_cache.json")
}

pub fn user_data_dir() -> PathBuf {
    app_data_root().join("user_data")
}

pub fn match_history_path() -> PathBuf {
    user_data_dir().join("match_history.json")
}

pub fn match_history_db_path() -> PathBuf {
    user_data_dir().join("match_history.db")
}

pub fn ensure_parent_dir(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn ensure_user_data_files() -> Result<(), String> {
    let db_path = match_history_db_path();
    ensure_parent_dir(&db_path)?;
    let mut conn = open_history_db()?;
    migrate_legacy_json_if_needed(&mut conn)?;
    Ok(())
}

pub fn load_match_history() -> Result<serde_json::Value, String> {
    ensure_user_data_files()?;
    let conn = open_history_db()?;
    load_match_history_from_db(&conn)
}

pub fn save_match_history(history: &serde_json::Value) -> Result<(), String> {
    ensure_user_data_files()?;
    let mut conn = open_history_db()?;
    save_match_history_to_db(&mut conn, history)
}

fn open_history_db() -> Result<Connection, String> {
    let db_path = match_history_db_path();
    ensure_parent_dir(&db_path)?;
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    init_history_schema(&conn)?;
    Ok(conn)
}

fn init_history_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "
        PRAGMA journal_mode = WAL;
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS matches (
            match_id TEXT PRIMARY KEY,
            hero TEXT,
            start_time TEXT NOT NULL,
            end_time TEXT,
            game_date TEXT,
            days INTEGER NOT NULL DEFAULT 1,
            victory INTEGER NOT NULL DEFAULT 0,
            is_finished INTEGER NOT NULL DEFAULT 0,
            updated_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS battles (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            match_id TEXT NOT NULL,
            battle_order INTEGER NOT NULL DEFAULT 0,
            day INTEGER NOT NULL,
            start_time TEXT NOT NULL,
            victory INTEGER NOT NULL DEFAULT 0,
            duration REAL,
            screenshot TEXT,
            lineup_json TEXT,
            enemy_lineup_json TEXT,
            UNIQUE(match_id, day, start_time, victory),
            FOREIGN KEY(match_id) REFERENCES matches(match_id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_battles_match_order ON battles(match_id, battle_order);
        CREATE INDEX IF NOT EXISTS idx_matches_date_time ON matches(game_date DESC, start_time DESC);
        ",
    )
    .map_err(|e| e.to_string())?;

    ensure_battles_column(conn, "lineup_json", "TEXT")?;
    ensure_battles_column(conn, "enemy_lineup_json", "TEXT")?;
    Ok(())
}

fn has_table_column(conn: &Connection, table: &str, column: &str) -> Result<bool, String> {
    let pragma_sql = format!("PRAGMA table_info({})", table);
    let mut stmt = conn.prepare(&pragma_sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| e.to_string())?;
    for row in rows {
        if row.map_err(|e| e.to_string())? == column {
            return Ok(true);
        }
    }
    Ok(false)
}

fn ensure_battles_column(conn: &Connection, column: &str, column_type: &str) -> Result<(), String> {
    if has_table_column(conn, "battles", column)? {
        return Ok(());
    }
    let alter_sql = format!("ALTER TABLE battles ADD COLUMN {} {}", column, column_type);
    conn.execute(&alter_sql, []).map_err(|e| e.to_string())?;
    Ok(())
}

fn db_has_history_data(conn: &Connection) -> Result<bool, String> {
    conn.query_row("SELECT EXISTS(SELECT 1 FROM matches LIMIT 1)", [], |row| {
        row.get::<_, i64>(0)
    })
    .map(|v| v != 0)
    .map_err(|e| e.to_string())
}

fn migrate_legacy_json_if_needed(conn: &mut Connection) -> Result<(), String> {
    if db_has_history_data(conn)? {
        return Ok(());
    }

    let legacy_path = match_history_path();
    if !legacy_path.exists() {
        return Ok(());
    }

    let text = std::fs::read_to_string(&legacy_path).map_err(|e| e.to_string())?;
    let legacy_value: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    save_match_history_to_db(conn, &legacy_value)
}

fn load_match_history_from_db(conn: &Connection) -> Result<serde_json::Value, String> {
    let mut stmt = conn
        .prepare(
            "
            SELECT match_id, hero, start_time, end_time, game_date, days, victory, is_finished
            FROM matches
            ORDER BY is_finished ASC, game_date DESC, start_time DESC, updated_at DESC
            ",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, i64>(7)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    let mut matches_json = Vec::new();

    for row in rows {
        let (match_id, hero, start_time, end_time, game_date, days, victory, is_finished) =
            row.map_err(|e| e.to_string())?;

        let mut battle_stmt = conn
            .prepare(
                "
                SELECT day, start_time, victory, duration, screenshot, lineup_json, enemy_lineup_json
                FROM battles
                WHERE match_id = ?1
                ORDER BY battle_order ASC, day ASC, start_time ASC
                ",
            )
            .map_err(|e| e.to_string())?;

        let battle_rows = battle_stmt
            .query_map(params![match_id.clone()], |brow| {
                let lineup_json = brow.get::<_, Option<String>>(5)?;
                let lineup_cards = lineup_json
                    .as_deref()
                    .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok());
                let enemy_lineup_json = brow.get::<_, Option<String>>(6)?;
                let enemy_lineup_cards = enemy_lineup_json
                    .as_deref()
                    .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok());
                Ok(serde_json::json!({
                    "day": brow.get::<_, i64>(0)? as u32,
                    "start_time": brow.get::<_, String>(1)?,
                    "victory": brow.get::<_, i64>(2)? != 0,
                    "duration": brow.get::<_, Option<f64>>(3)?,
                    "screenshot": brow.get::<_, Option<String>>(4)?,
                    "lineup_cards": lineup_cards,
                    "enemy_lineup_cards": enemy_lineup_cards
                }))
            })
            .map_err(|e| e.to_string())?;

        let mut battles = Vec::new();
        for battle in battle_rows {
            battles.push(battle.map_err(|e| e.to_string())?);
        }

        matches_json.push(serde_json::json!({
            "match_id": match_id,
            "hero": hero,
            "start_time": start_time,
            "end_time": end_time,
            "game_date": game_date,
            "days": (days.max(1)) as u32,
            "victory": victory != 0,
            "is_finished": is_finished != 0,
            "pvp_battles": battles,
        }));
    }

    Ok(serde_json::json!({ "matches": matches_json }))
}

fn save_match_history_to_db(conn: &mut Connection, history: &serde_json::Value) -> Result<(), String> {
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    tx.execute("DELETE FROM battles", []).map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM matches", []).map_err(|e| e.to_string())?;

    let matches = history
        .get("matches")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    for (idx, item) in matches.iter().enumerate() {
        let match_id_raw = item
            .get("match_id")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .trim()
            .to_string();

        let start_time = item
            .get("start_time")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .trim()
            .to_string();

        let match_id = if match_id_raw.is_empty() {
            format!("legacy-{}-{}", idx, start_time)
        } else {
            match_id_raw
        };

        let hero = item.get("hero").and_then(|v| v.as_str()).map(|s| s.to_string());
        let end_time = item
            .get("end_time")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let game_date = item
            .get("game_date")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let days = item.get("days").and_then(|v| v.as_u64()).unwrap_or(1).max(1) as i64;
        let victory = item.get("victory").and_then(|v| v.as_bool()).unwrap_or(false);
        let is_finished = item
            .get("is_finished")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        tx.execute(
            "
            INSERT INTO matches (match_id, hero, start_time, end_time, game_date, days, victory, is_finished, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            ",
            params![
                &match_id,
                hero,
                start_time,
                end_time,
                game_date,
                days,
                if victory { 1_i64 } else { 0_i64 },
                if is_finished { 1_i64 } else { 0_i64 },
                now_ms + idx as i64,
            ],
        )
        .map_err(|e| e.to_string())?;

        if let Some(battles) = item.get("pvp_battles").and_then(|v| v.as_array()) {
            for (battle_order, battle) in battles.iter().enumerate() {
                let day = battle.get("day").and_then(|v| v.as_u64()).unwrap_or(1).max(1) as i64;
                let battle_start_time = battle
                    .get("start_time")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let battle_victory = battle
                    .get("victory")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let duration = battle.get("duration").and_then(|v| v.as_f64());
                let screenshot = battle
                    .get("screenshot")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let lineup_json = battle
                    .get("lineup_cards")
                    .and_then(|v| if v.is_null() { None } else { Some(v) })
                    .and_then(|v| serde_json::to_string(v).ok());
                let enemy_lineup_json = battle
                    .get("enemy_lineup_cards")
                    .and_then(|v| if v.is_null() { None } else { Some(v) })
                    .and_then(|v| serde_json::to_string(v).ok());

                tx.execute(
                    "
                    INSERT OR IGNORE INTO battles (match_id, battle_order, day, start_time, victory, duration, screenshot, lineup_json, enemy_lineup_json)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                    ",
                    params![
                        &match_id,
                        battle_order as i64,
                        day,
                        battle_start_time,
                        if battle_victory { 1_i64 } else { 0_i64 },
                        duration,
                        screenshot,
                        lineup_json,
                        enemy_lineup_json,
                    ],
                )
                .map_err(|e| e.to_string())?;
            }
        }
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

fn build_match_id_from_start_time(start_time: &str) -> String {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    start_time.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

pub fn upsert_match_battle_snapshot(
    match_start_time: &str,
    battle_day: u32,
    battle_start_time: &str,
    victory: bool,
    duration: Option<f64>,
    screenshot: Option<&str>,
    lineup_cards_json: Option<&str>,
    enemy_lineup_cards_json: Option<&str>,
) -> Result<(), String> {
    ensure_user_data_files()?;
    let mut conn = open_history_db()?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let match_start = match_start_time.trim();
    if match_start.is_empty() {
        return Err("empty match_start_time".to_string());
    }

    let match_id = build_match_id_from_start_time(match_start);
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    tx.execute(
        "
        INSERT INTO matches (match_id, hero, start_time, end_time, game_date, days, victory, is_finished, updated_at)
        VALUES (?1, NULL, ?2, NULL, ?3, ?4, 0, 0, ?5)
        ON CONFLICT(match_id) DO UPDATE SET
            days = MAX(matches.days, excluded.days),
            updated_at = excluded.updated_at,
            game_date = COALESCE(matches.game_date, excluded.game_date)
        ",
        params![
            &match_id,
            match_start,
            today,
            (battle_day.saturating_add(1)).max(1) as i64,
            now_ms,
        ],
    )
    .map_err(|e| e.to_string())?;

    let updated = tx
        .execute(
            "
            UPDATE battles
            SET
                start_time = COALESCE(NULLIF(?1, ''), start_time),
                victory = ?2,
                duration = COALESCE(?3, duration),
                screenshot = COALESCE(?4, screenshot),
                lineup_json = COALESCE(?5, lineup_json),
                enemy_lineup_json = COALESCE(?6, enemy_lineup_json)
            WHERE match_id = ?7 AND day = ?8 AND start_time = ?9 AND victory = ?10
            ",
            params![
                battle_start_time,
                if victory { 1_i64 } else { 0_i64 },
                duration,
                screenshot,
                lineup_cards_json,
                enemy_lineup_cards_json,
                &match_id,
                battle_day as i64,
                battle_start_time,
                if victory { 1_i64 } else { 0_i64 },
            ],
        )
        .map_err(|e| e.to_string())?;

    let updated = if updated == 0 {
        let day_rows = tx
            .query_row(
                "SELECT COUNT(1) FROM battles WHERE match_id = ?1 AND day = ?2",
                params![&match_id, battle_day as i64],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(0);

        if day_rows == 1 {
            tx.execute(
                "
                UPDATE battles
                SET
                    start_time = COALESCE(NULLIF(?1, ''), start_time),
                    victory = ?2,
                    duration = COALESCE(?3, duration),
                    screenshot = COALESCE(?4, screenshot),
                    lineup_json = COALESCE(?5, lineup_json),
                    enemy_lineup_json = COALESCE(?6, enemy_lineup_json)
                WHERE match_id = ?7 AND day = ?8
                ",
                params![
                    battle_start_time,
                    if victory { 1_i64 } else { 0_i64 },
                    duration,
                    screenshot,
                    lineup_cards_json,
                    enemy_lineup_cards_json,
                    &match_id,
                    battle_day as i64,
                ],
            )
            .map_err(|e| e.to_string())?
        } else {
            0
        }
    } else {
        updated
    };

    if updated == 0 {
        let battle_order = tx
            .query_row(
                "SELECT COALESCE(MAX(battle_order), -1) + 1 FROM battles WHERE match_id = ?1",
                params![&match_id],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(0);

        tx.execute(
            "
            INSERT INTO battles (match_id, battle_order, day, start_time, victory, duration, screenshot, lineup_json, enemy_lineup_json)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            ON CONFLICT(match_id, day, start_time, victory) DO UPDATE SET
                duration = COALESCE(excluded.duration, battles.duration),
                screenshot = COALESCE(excluded.screenshot, battles.screenshot),
                lineup_json = COALESCE(excluded.lineup_json, battles.lineup_json),
                enemy_lineup_json = COALESCE(excluded.enemy_lineup_json, battles.enemy_lineup_json)
            ",
            params![
                &match_id,
                battle_order,
                battle_day as i64,
                battle_start_time,
                if victory { 1_i64 } else { 0_i64 },
                duration,
                screenshot,
                lineup_cards_json,
                enemy_lineup_cards_json,
            ],
        )
        .map_err(|e| e.to_string())?;
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}
