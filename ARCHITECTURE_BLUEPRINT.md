# BazaarHelper Architecture Blueprint

## Goals
- Split giant files (`src-tauri/src/lib.rs`, `src/App.tsx`) into clear domain modules.
- Separate platform code, services, data persistence, UI, and logs.
- Keep YOLO/ORB as replaceable services.
- Provide stable user data persistence and readable logs with debug/release policy.

## Directory Layout (Target)

### Rust (`src-tauri/src`)
- `analysis/`: log parsing and game-state analysis (day detection, battle timeline)
- `core/`: domain models and app state glue
- `data_management/`: DB/resource loading and cache lifecycle
- `gui/`: window commands and UI-facing tauri handlers
- `logs/`: logging bootstrap, policy, rotation config
- `platforms/`: OS-specific window/hotkey/focus behavior
- `services/`: replaceable engines (`yolo`, `orb`, future `deep_embedding`)
- `tests/`: integration and command-level tests
- `tools/`: debug helpers / diagnostics
- `user_data/`: persistent local user data (cache, history, preferences)
- `utils/`: shared low-level helpers

### Frontend (`src`)
- `components/`: reusable view components
- `views/`: page-level views (`HistoryView`, `ItemsView`, `CardRecognitionView`, `MonsterView`)
- `services/`: API adapters and tauri invoke wrappers
- `user_data/`: frontend-side persistence adapters (local settings abstraction)
- `utils/`: rendering and helper utilities

## What Is Refactored In This Commit
- Added backend modules:
  - `src-tauri/src/logs/mod.rs`
  - `src-tauri/src/user_data/mod.rs`
  - `src-tauri/src/platforms/{mod.rs,hotkey.rs,window_style.rs}`
  - `src-tauri/src/services/{mod.rs,yolo_state.rs}`
- `lib.rs` now delegates key concerns to these modules instead of inlining all logic.
- Added tauri command: `get_match_history` (reads `user_data/match_history.json`).
- Logging upgraded to colored + rotating file logs (`flexi_logger`), with debug/release log-level policy.

## Logging Policy
- Log folder: `<app_data_root>/logs`
- Rotation: size-based (`flexi_logger`)
- Default max single log file: `20MB`
- Override max size: `BAZAAR_HELPER_LOG_MAX_MB`
- Log level:
  - Debug build or `BAZAAR_HELPER_DEBUG=1`: `debug`
  - Release default: `info`
  - Override level string: `BAZAAR_HELPER_LOG`

## User Data Policy
- Root: `<app_data_root>/user_data`
- History file: `match_history.json`
- State cache: `state_cache.json`
- User data files are created automatically on startup.

## Frontend Navigation Refactor
- Replaced top nav tabs with left sidebar navigation (Playbook-style interaction):
  - `history`, `monster`, `card`, `items`, `search`
- Added `HistoryView` and connected to backend `get_match_history`.
- Added `SidebarNav` with icon rail + hover expand labels.

## Next Extraction Steps
1. Move `PersistentState`, `ItemData`, `MonsterData` and related models from `lib.rs` into `core/`.
2. Move `start_template_loading/search_items/get_all_monsters` into `data_management/` and `analysis/`.
3. Move window commands (`show_detail_popup_at`, geometry handlers) into `gui/commands` modules.
4. Move long-running monitor threads into `services/runtime` modules.
5. Split `App.tsx` settings/search/filter sections into dedicated feature components/hooks.
