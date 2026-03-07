#[tauri::command]
pub fn get_match_history() -> Result<serde_json::Value, String> {
    super::load_match_history()
}

#[tauri::command]
pub fn get_screenshot_capture_delay_ms() -> Result<u64, String> {
    let state = crate::load_state();
    Ok(state.screenshot_capture_delay_ms.min(3000))
}

#[tauri::command]
pub fn set_screenshot_capture_delay_ms(delay_ms: u64) -> Result<u64, String> {
    let mut state = crate::load_state();
    let clamped = delay_ms.min(3000);
    state.screenshot_capture_delay_ms = clamped;
    crate::save_state(&state);
    Ok(clamped)
}

#[tauri::command]
pub fn get_upload_notice_suppressed() -> Result<bool, String> {
    let state = crate::load_state();
    Ok(state.suppress_upload_notice)
}

#[tauri::command]
pub fn set_upload_notice_suppressed(suppressed: bool) -> Result<bool, String> {
    let mut state = crate::load_state();
    state.suppress_upload_notice = suppressed;
    crate::save_state(&state);
    Ok(state.suppress_upload_notice)
}

#[tauri::command]
pub fn get_auto_collapse_to_island_enabled() -> Result<bool, String> {
    let state = crate::load_state();
    Ok(state.auto_collapse_to_island)
}

#[tauri::command]
pub fn set_auto_collapse_to_island_enabled(enabled: bool) -> Result<bool, String> {
    let mut state = crate::load_state();
    state.auto_collapse_to_island = enabled;
    crate::save_state(&state);
    Ok(state.auto_collapse_to_island)
}
