use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use tauri::{Emitter, Manager, RunEvent, WindowEvent};

use crate::{
    core, data_management, load_state, log_system_info, log_to_file, logs, platforms, services,
    user_data, DbState, ItemDb, SkillDb,
};

pub fn run() {
    crate::core::lifecycle::reset_app_exit_flag();

    if let Err(e) = logs::init_logging() {
        eprintln!("[Logger] init failed: {}", e);
    }
    logs::start_memory_monitor();
    if let Err(e) = user_data::ensure_user_data_files() {
        log::warn!("[UserData] init failed: {}", e);
    }
    crate::set_panic_hook();
    log_to_file("=================== App Starting ===================");

    #[allow(unused_mut)]
    let mut builder = tauri::Builder::default()
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }
            match event {
                WindowEvent::CloseRequested { api, .. } => {
                    log::warn!("[WindowEvent] main close requested; preventing close and hiding window");
                    api.prevent_close();
                    let _ = window.hide();
                }
                WindowEvent::Destroyed => {
                    log::error!("[WindowEvent] main window destroyed");
                }
                _ => {}
            }
        })
        .manage(services::overlay::OverlayState::new())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.maximize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build());

    #[cfg(target_os = "macos")]
    {
        builder = builder.plugin(tauri_nspanel::init());
    }

    let builder = builder;

    let app = builder
        .manage(DbState {
            items: Arc::new(RwLock::new(ItemDb {
                list: Vec::new(),
                id_map: HashMap::new(),
                unique_tags: Vec::new(),
            })),
            skills: Arc::new(RwLock::new(SkillDb {
                list: Vec::new(),
                id_map: HashMap::new(),
            })),
            monsters: Arc::new(RwLock::new(serde_json::Map::new())),
        })
        .setup(move |app| {
            let state = load_state();
            core::hotkey_state::update_detail_hotkey_cache(state.detail_display_hotkey);
            core::hotkey_state::update_detection_hotkey_cache(state.detection_hotkey);
            core::hotkey_state::update_card_detection_hotkey_cache(state.card_detection_hotkey);
            core::hotkey_state::update_toggle_collapse_hotkey_cache(state.toggle_collapse_hotkey);
            core::hotkey_state::update_yolo_hotkey_cache(state.yolo_hotkey);

            let handle = app.handle().clone();
            log_system_info(&handle);
            let mac_perm = platforms::permissions::ensure_macos_permissions_on_startup();
            let _ = handle.emit("macos-permission-status", &mac_perm);
            if !mac_perm.accessibility {
                log::warn!("[macOS Permissions] Accessibility permission missing. Global hotkeys disabled.");
            }
            if !mac_perm.screen_recording {
                log::warn!("[macOS Permissions] Screen recording permission missing. Capture may fail.");
            }

            platforms::setup::setup_app_shell(app)?;

            platforms::monitor::spawn_focus_monitor(handle.clone());

            let db_state = app.state::<DbState>();
            let thread_items_db = db_state.items.clone();
            let thread_skills_db = db_state.skills.clone();
            let log_handle = handle.clone();
            data_management::log_monitor::spawn_log_monitor(log_handle, thread_items_db, thread_skills_db);

            let startup_handle = handle.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = services::template_loading::start_template_loading(startup_handle).await {
                    log::warn!("[TemplateLoading] startup preload failed: {}", e);
                }
            });

            if platforms::permissions::can_start_global_hotkey_monitor() {
                platforms::monitor::spawn_mouse_hotkey_monitor(handle.clone());
            } else {
                log::warn!("[Hotkey Monitor] Not started because accessibility permission is missing.");
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            services::overlay::update_overlay_bounds,
            services::commands::abort_yolo_scan,
            services::commands::set_show_yolo_monitor,
            services::commands::trigger_yolo_scan,
            services::overlay::handle_overlay_right_click,
            data_management::commands::get_item_info,
            platforms::commands::set_overlay_ignore_cursor,
            platforms::commands::show_yolo_monitor_window,
            platforms::commands::show_detail_popup_at,
            platforms::commands::hide_detail_popup,
            platforms::commands::reset_detail_popup_position,
            platforms::focus::restore_game_focus,
            platforms::focus::was_last_foreground_game,
            data_management::commands::get_show_yolo_monitor,
            user_data::commands::get_match_history,
            services::template_loading::start_template_loading,
            data_management::commands::search_items,
            data_management::commands::get_all_monsters,
            data_management::commands::debug_monsters_db,
            services::commands::clear_yolo_cache,
            data_management::commands::debug_resource_paths,
            services::commands::recognize_monsters_from_screenshot,
            services::commands::get_template_loading_progress,
            data_management::commands::get_current_day,
            data_management::commands::update_day,
            platforms::commands::get_detection_hotkey,
            platforms::commands::get_card_detection_hotkey,
            platforms::commands::get_toggle_collapse_hotkey,
            platforms::commands::set_detection_hotkey,
            platforms::commands::set_card_detection_hotkey,
            platforms::commands::set_toggle_collapse_hotkey,
            platforms::commands::get_yolo_hotkey,
            platforms::commands::set_yolo_hotkey,
            platforms::commands::reset_all_hotkeys,
            platforms::commands::save_monster_calibration,
            platforms::commands::load_monster_calibration,
            platforms::commands::get_game_window_info,
            platforms::commands::open_calibration_window,
            platforms::commands::close_calibration_window,
            platforms::commands::get_detail_display_hotkey,
            platforms::commands::set_detail_display_hotkey,
            services::commands::get_yolo_stats,
            data_management::commands::get_sync_state,
            data_management::commands::get_runtime_logs,
            data_management::commands::set_debug_mode,
            data_management::commands::get_debug_mode,
            data_management::commands::check_required_files,
            crate::monster_recognition::recognize_card_at_mouse,
            services::commands::invoke_yolo_scan,
            services::commands::emit_to_main,
            platforms::commands::save_window_geometry,
            platforms::commands::save_detail_popup_geometry,
            platforms::commands::get_window_geometry,
            platforms::commands::reset_window_geometry,
            platforms::commands::prepare_app_exit,
            platforms::commands::request_app_exit,
            platforms::permissions::get_macos_permission_status,
            platforms::permissions::request_macos_permissions
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|_app_handle, event| {
        match event {
            RunEvent::Exit => {
                log::error!("[RunEvent] Exit");
            }
            RunEvent::ExitRequested { api, .. } => {
                if !crate::core::lifecycle::is_app_exit_allowed() {
                    log::warn!("[RunEvent] Exit requested but blocked (not explicitly allowed)");
                    api.prevent_exit();
                } else {
                    log::info!("[RunEvent] Exit requested and allowed");
                }
            }
            #[cfg(target_os = "macos")]
            RunEvent::Reopen { .. } => {
                if let Some(window) = _app_handle.get_webview_window("main") {
                    let _ = window.unminimize();
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            _ => {}
        }
    });
}
