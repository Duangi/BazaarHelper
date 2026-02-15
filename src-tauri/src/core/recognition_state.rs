use std::sync::atomic::AtomicBool;
use std::sync::{OnceLock, RwLock};

pub static IS_RECOGNIZING: AtomicBool = AtomicBool::new(false);
pub static LAST_RECOG_TIME: OnceLock<RwLock<Option<std::time::Instant>>> = OnceLock::new();

pub fn update_last_recog_time() {
    let cache = LAST_RECOG_TIME.get_or_init(|| RwLock::new(None));
    if let Ok(mut writer) = cache.write() {
        *writer = Some(std::time::Instant::now());
    }
}
