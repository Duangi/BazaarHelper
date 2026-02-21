use std::sync::atomic::{AtomicBool, Ordering};

static GAME_LOG_MONITOR_ENABLED: AtomicBool = AtomicBool::new(true);

pub fn set_enabled(enabled: bool) {
    GAME_LOG_MONITOR_ENABLED.store(enabled, Ordering::Relaxed);
}

pub fn is_enabled() -> bool {
    GAME_LOG_MONITOR_ENABLED.load(Ordering::Relaxed)
}
