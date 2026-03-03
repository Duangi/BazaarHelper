use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static LAST_INSIDE_INTERACTION_MS: AtomicU64 = AtomicU64::new(0);

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn mark_inside_interaction_now() {
    LAST_INSIDE_INTERACTION_MS.store(now_ms(), Ordering::Relaxed);
}

pub fn is_inside_interaction_recent(window_ms: u64) -> bool {
    let last = LAST_INSIDE_INTERACTION_MS.load(Ordering::Relaxed);
    if last == 0 {
        return false;
    }
    now_ms().saturating_sub(last) <= window_ms
}
