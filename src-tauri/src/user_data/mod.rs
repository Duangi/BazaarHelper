use std::path::{Path, PathBuf};

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

pub fn ensure_parent_dir(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn ensure_user_data_files() -> Result<(), String> {
    let history_path = match_history_path();
    ensure_parent_dir(&history_path)?;

    if !history_path.exists() {
        let seed = serde_json::json!({ "matches": [] });
        std::fs::write(&history_path, seed.to_string()).map_err(|e| e.to_string())?;
    }

    Ok(())
}

pub fn load_match_history() -> Result<serde_json::Value, String> {
    ensure_user_data_files()?;
    let path = match_history_path();
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

pub fn save_match_history(history: &serde_json::Value) -> Result<(), String> {
    let path = match_history_path();
    ensure_parent_dir(&path)?;
    let text = serde_json::to_string_pretty(history).map_err(|e| e.to_string())?;
    std::fs::write(path, text).map_err(|e| e.to_string())
}
