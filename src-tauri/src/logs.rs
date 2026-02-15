use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use flexi_logger::{
    colored_default_format, Cleanup, Criterion, Duplicate, FileSpec, Logger, LoggerHandle, Naming,
};
use tauri::Manager;

use crate::user_data;

static LOGGER_STARTED: OnceLock<()> = OnceLock::new();
static LOGGER_HANDLE: OnceLock<LoggerHandle> = OnceLock::new();
static MEMORY_MONITOR_STARTED: OnceLock<()> = OnceLock::new();
const DEFAULT_MAX_LOG_MB: u64 = 20;

pub fn init_logging() -> Result<(), String> {
    if LOGGER_STARTED.get().is_some() {
        return Ok(());
    }

    let log_dir = log_dir_path();
    std::fs::create_dir_all(&log_dir).map_err(|e| e.to_string())?;

    let log_spec = compute_log_spec();
    let max_bytes = log_max_bytes();

    let logger_handle = Logger::try_with_str(&log_spec)
        .map_err(|e| e.to_string())?
        .log_to_file(FileSpec::default().directory(&log_dir).basename("bazaarhelper"))
        .duplicate_to_stdout(Duplicate::All)
        .format_for_stdout(colored_default_format)
        .rotate(Criterion::Size(max_bytes), Naming::Numbers, Cleanup::KeepLogFiles(8))
        .start()
        .map_err(|e| e.to_string())?;

    let _ = LOGGER_HANDLE.set(logger_handle);
    let _ = LOGGER_STARTED.set(());
    log::info!(
        "logger initialized: dir={}, spec={}, max_file={}MB",
        log_dir.to_string_lossy(),
        log_spec,
        max_bytes / 1024 / 1024
    );

    Ok(())
}

pub fn log_dir_path() -> PathBuf {
    user_data::app_data_root().join("logs")
}

pub fn log_compat(msg: &str) {
    log::info!("{}", msg);
}

pub fn install_panic_hook() {
    std::panic::set_hook(Box::new(|panic_info| {
        let payload = panic_info.payload();
        let message = if let Some(s) = payload.downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = payload.downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic".to_string()
        };

        let location = panic_info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown location".to_string());

        log::error!("FATAL PANIC at {}: {}", location, message);
        eprintln!("FATAL PANIC at {}: {}", location, message);
    }));
}

pub fn log_system_info(app_handle: &tauri::AppHandle, game_log_path: &Path) {
    log::info!("--- System Info ---");
    log::info!("OS: {}", std::env::consts::OS);
    log::info!("ARCH: {}", std::env::consts::ARCH);

    if let Ok(exe_path) = std::env::current_exe() {
        log::info!("EXE Path: {:?}", exe_path);
    }

    if let Ok(cwd) = std::env::current_dir() {
        log::info!("CWD: {:?}", cwd);
    }

    log::info!("Resource Dir: {:?}", app_handle.path().resource_dir().ok());
    log::info!("App Config Dir: {:?}", app_handle.path().app_config_dir().ok());
    log::info!(
        "App Local Data Dir: {:?}",
        app_handle.path().app_local_data_dir().ok()
    );

    for var in ["PATH", "USERNAME", "APPDATA", "LOCALAPPDATA"] {
        if let Ok(val) = std::env::var(var) {
            log::debug!("Env {}: {}", var, val);
        }
    }

    log::info!("Game Log Path: {:?}", game_log_path);
    log::info!("Game Log Exists: {}", game_log_path.exists());
    log::info!("-------------------");
}

fn compute_log_spec() -> String {
    if let Ok(spec) = std::env::var("BAZAAR_HELPER_LOG") {
        let v = spec.trim();
        if !v.is_empty() {
            return v.to_string();
        }
    }

    if force_debug_mode() || state_debug_mode() || cfg!(debug_assertions) {
        "debug".to_string()
    } else {
        "info".to_string()
    }
}

fn force_debug_mode() -> bool {
    matches!(
        std::env::var("BAZAAR_HELPER_DEBUG")
            .unwrap_or_default()
            .to_lowercase()
            .as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn state_debug_mode() -> bool {
    crate::load_state().debug_mode
}

pub fn set_debug_mode(enabled: bool) -> Result<(), String> {
    let spec = if enabled { "debug" } else { "info" };
    let handle = LOGGER_HANDLE
        .get()
        .ok_or_else(|| "logger handle not initialized".to_string())?;
    handle.parse_new_spec(spec).map_err(|e| e.to_string())
}

pub fn list_log_files() -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    let dir = log_dir_path();
    if !dir.exists() {
        return Ok(files);
    }

    let iter = std::fs::read_dir(&dir).map_err(|e| e.to_string())?;
    for entry in iter {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let is_log = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("log"))
            .unwrap_or(false);
        if is_log {
            files.push(path);
        }
    }

    files.sort_by(|a, b| {
        let ma = std::fs::metadata(a)
            .and_then(|m| m.modified())
            .ok();
        let mb = std::fs::metadata(b)
            .and_then(|m| m.modified())
            .ok();
        mb.cmp(&ma)
    });
    Ok(files)
}

pub fn read_recent_log_lines(limit: usize) -> Result<Vec<String>, String> {
    let files = list_log_files()?;
    let mut lines: Vec<String> = Vec::new();

    for path in files {
        let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let mut file_lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        file_lines.reverse();
        for line in file_lines {
            lines.push(line);
            if lines.len() >= limit {
                lines.reverse();
                return Ok(lines);
            }
        }
    }

    lines.reverse();
    Ok(lines)
}

fn log_max_bytes() -> u64 {
    let configured_mb = std::env::var("BAZAAR_HELPER_LOG_MAX_MB")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|v| *v >= 1)
        .unwrap_or(DEFAULT_MAX_LOG_MB);

    configured_mb * 1024 * 1024
}

pub fn start_memory_monitor() {
    if MEMORY_MONITOR_STARTED.get().is_some() {
        return;
    }
    if !(force_debug_mode() || state_debug_mode() || cfg!(debug_assertions)) {
        return;
    }

    let _ = MEMORY_MONITOR_STARTED.set(());

    let interval_secs = std::env::var("BAZAAR_HELPER_MEM_LOG_SEC")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .map(|v| v.clamp(5, 600))
        .unwrap_or(20);
    let pid = std::process::id();

    std::thread::spawn(move || {
        let mut baseline_kb: Option<u64> = None;
        let mut peak_kb: u64 = 0;
        log::info!("[Memory] monitor started (pid={}, interval={}s)", pid, interval_secs);

        loop {
            if let Some(rss_kb) = read_process_rss_kb(pid) {
                if baseline_kb.is_none() {
                    baseline_kb = Some(rss_kb);
                }

                if rss_kb > peak_kb {
                    peak_kb = rss_kb;
                }

                let base = baseline_kb.unwrap_or(rss_kb);
                let delta_kb = rss_kb.saturating_sub(base);
                let rss_mb = rss_kb as f64 / 1024.0;
                let delta_mb = delta_kb as f64 / 1024.0;
                let peak_mb = peak_kb as f64 / 1024.0;

                if rss_mb >= 1024.0 {
                    log::warn!(
                        "[Memory] high RSS: {:.1} MB (delta +{:.1} MB, peak {:.1} MB)",
                        rss_mb,
                        delta_mb,
                        peak_mb
                    );
                } else {
                    log::debug!(
                        "[Memory] RSS {:.1} MB (delta +{:.1} MB, peak {:.1} MB)",
                        rss_mb,
                        delta_mb,
                        peak_mb
                    );
                }
            }

            std::thread::sleep(std::time::Duration::from_secs(interval_secs));
        }
    });
}

#[cfg(target_family = "unix")]
fn read_process_rss_kb(pid: u32) -> Option<u64> {
    let output = std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &pid.to_string()])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    text.trim().parse::<u64>().ok()
}

#[cfg(not(target_family = "unix"))]
fn read_process_rss_kb(_pid: u32) -> Option<u64> {
    None
}
