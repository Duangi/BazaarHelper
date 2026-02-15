use std::sync::atomic::{AtomicBool, Ordering};

static ALLOW_APP_EXIT: AtomicBool = AtomicBool::new(false);

pub fn allow_app_exit() {
    ALLOW_APP_EXIT.store(true, Ordering::SeqCst);
}

pub fn reset_app_exit_flag() {
    ALLOW_APP_EXIT.store(false, Ordering::SeqCst);
}

pub fn is_app_exit_allowed() -> bool {
    ALLOW_APP_EXIT.load(Ordering::SeqCst)
}
