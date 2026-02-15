use crate::PersistentState;

pub fn state_cache_path() -> std::path::PathBuf {
    super::state_cache_path()
}

pub fn save_state(state: &PersistentState) {
    let path = state_cache_path();
    let _ = super::ensure_parent_dir(&path);
    if let Ok(json) = serde_json::to_string(state) {
        let _ = std::fs::write(path, json);
    }
}

pub fn load_state() -> PersistentState {
    let path = state_cache_path();
    if let Ok(json) = std::fs::read_to_string(path) {
        if let Ok(state) = serde_json::from_str::<PersistentState>(&json) {
            return state;
        }
    }
    PersistentState::default()
}
