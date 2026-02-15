use std::path::{Path, PathBuf};

pub const RESOURCE_DB_FILES: &[&str] = &[
    "items_db.json",
    "skills_db.json",
    "monsters_db.json",
    "event_encounters.json",
    "event_detail.json",
    "combat_encounters.json",
    "card_id_mapping.json",
];

pub const RESOURCE_DIR_ALIASES: &[(&str, &str)] = &[
    ("assets/items", "images"),
    ("assets/skills", "images/skill"),
    ("assets/monsters/characters", "images_monster_char"),
    ("assets/monsters/backgrounds", "images_monster_bg"),
    ("assets/gui", "images_GUI"),
    ("assets/events/characters", "EncEvent_CHAR"),
    ("assets/events/backgrounds", "EncEvent_BG"),
    ("assets/events/icons", "EncEvent_Icons"),
    ("assets/sponsor", "sponsor"),
];

fn map_prefix(value: &str, from_prefix: &str, to_prefix: &str) -> Option<String> {
    if value == from_prefix {
        return Some(to_prefix.to_string());
    }

    value
        .strip_prefix(&(from_prefix.to_string() + "/"))
        .map(|rest| format!("{to_prefix}/{rest}"))
}

pub fn build_resource_candidates(raw_path: &str) -> Vec<String> {
    let normalized = raw_path
        .trim_start_matches("resources/")
        .trim_start_matches('/');

    let mut output = vec![normalized.to_string()];

    for (canonical, legacy) in RESOURCE_DIR_ALIASES {
        if let Some(mapped) = map_prefix(normalized, canonical, legacy) {
            if !output.contains(&mapped) {
                output.push(mapped);
            }
        }

        if let Some(mapped) = map_prefix(normalized, legacy, canonical) {
            if !output.contains(&mapped) {
                output.push(mapped);
            }
        }
    }

    output
}

pub fn resolve_existing_resource(resources_dir: &Path, raw_path: &str) -> Option<PathBuf> {
    for candidate in build_resource_candidates(raw_path) {
        let full = resources_dir.join(&candidate);
        if full.exists() {
            return Some(full);
        }
    }
    None
}
