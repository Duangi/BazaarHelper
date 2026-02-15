#[tauri::command]
pub fn get_match_history() -> Result<serde_json::Value, String> {
    super::load_match_history()
}
