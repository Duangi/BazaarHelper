use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};

// 野怪识别区域数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonsterRegion {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

// 野怪识别校准数据
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MonsterCalibration {
    pub regions: Vec<MonsterRegion>, // 三个区域，按照x坐标从左到右排序
    pub game_window_width: u32,
    pub game_window_height: u32,
    pub screen_width: u32,
    pub screen_height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistentState {
    pub day: u32,
    pub inst_to_temp: HashMap<String, String>,
    pub current_hand: HashSet<String>,
    pub current_stash: HashSet<String>,
    #[serde(default)]
    pub detection_hotkey: Option<i32>,
    #[serde(default)]
    pub card_detection_hotkey: Option<i32>,
    #[serde(default)]
    pub toggle_collapse_hotkey: Option<i32>,
    #[serde(default)]
    pub yolo_hotkey: Option<i32>,
    #[serde(default)]
    pub detail_display_hotkey: Option<i32>,
    #[serde(default = "default_show_yolo_monitor")]
    pub show_yolo_monitor: bool,
    #[serde(default)]
    pub detail_popup_x: Option<i32>,
    #[serde(default)]
    pub detail_popup_y: Option<i32>,
    #[serde(default)]
    pub detail_popup_width: Option<u32>,
    #[serde(default)]
    pub detail_popup_height: Option<u32>,
    #[serde(default)]
    pub monster_calibration: Option<MonsterCalibration>,
    // Main Window Geometry Persistence
    #[serde(default)]
    pub main_window_x: Option<i32>,
    #[serde(default)]
    pub main_window_y: Option<i32>,
    #[serde(default)]
    pub main_window_width: Option<u32>,
    #[serde(default)]
    pub main_window_height: Option<u32>,
    #[serde(default)]
    pub debug_mode: bool,
    #[serde(default = "default_enable_game_log_monitor")]
    pub enable_game_log_monitor: bool,
    #[serde(default)]
    pub macos_prompted_accessibility: bool,
    #[serde(default)]
    pub macos_prompted_screen_recording: bool,
    #[serde(default)]
    pub screenshot_capture_delay_ms: u64,
    #[serde(default)]
    pub suppress_upload_notice: bool,
    #[serde(default = "default_auto_collapse_to_island")]
    pub auto_collapse_to_island: bool,
}

// 跨平台虚拟键常量
const VK_RBUTTON_CODE: i32 = 2;   // 鼠标右键 (Windows VK_RBUTTON = 0x02)
const VK_MENU_CODE: i32 = 18;     // Alt 键 (Windows VK_MENU = 0x12)

impl Default for PersistentState {
    fn default() -> Self {
        Self {
            day: 1,
            inst_to_temp: HashMap::new(),
            current_hand: HashSet::new(),
            current_stash: HashSet::new(),
            detection_hotkey: Some(VK_RBUTTON_CODE),
            card_detection_hotkey: Some(VK_MENU_CODE),
            toggle_collapse_hotkey: Some(192), // Default: ~ key (Backtick) (VK_OEM_3 is 192 usually, or 0xC0)
            yolo_hotkey: Some(81), // Default: Q key (VK_Q = 81)
            detail_display_hotkey: Some(VK_RBUTTON_CODE), // Default: Right mouse button
            show_yolo_monitor: false,
            detail_popup_x: None,
            detail_popup_y: None,
            detail_popup_width: None,
            detail_popup_height: None,
            monster_calibration: None,
            main_window_x: None,
            main_window_y: None,
            main_window_width: None,
            main_window_height: None,
            debug_mode: false,
            enable_game_log_monitor: true,
            macos_prompted_accessibility: false,
            macos_prompted_screen_recording: false,
            screenshot_capture_delay_ms: 0,
            suppress_upload_notice: false,
            auto_collapse_to_island: false,
        }
    }
}

fn default_show_yolo_monitor() -> bool { false }
fn default_enable_game_log_monitor() -> bool { true }
fn default_auto_collapse_to_island() -> bool { false }
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RawSkill {
    pub en: Option<String>,
    pub cn: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RawItem {
    pub id: String,
    pub name_en: Option<String>,
    pub name_cn: Option<String>,
    pub starting_tier: Option<String>,
    pub available_tiers: Option<String>,
    pub heroes: Option<String>,
    pub tags: Option<String>,
    pub hidden_tags: Option<String>,
    pub size: Option<String>,
    pub cooldown: Option<f32>,
    pub cooldown_tiers: Option<String>,
    pub damage: Option<i32>,
    pub damage_tiers: Option<String>,
    pub heal: Option<i32>,
    pub heal_tiers: Option<String>,
    pub shield: Option<i32>,
    pub shield_tiers: Option<String>,
    pub ammo: Option<i32>,
    pub ammo_tiers: Option<String>,
    pub crit: Option<i32>,
    pub crit_tiers: Option<String>,
    pub multicast: Option<i32>,
    pub multicast_tiers: Option<String>,
    pub burn: Option<i32>,
    pub burn_tiers: Option<String>,
    pub poison: Option<i32>,
    pub poison_tiers: Option<String>,
    pub regen: Option<i32>,
    pub regen_tiers: Option<String>,
    pub lifesteal: Option<i32>,
    pub lifesteal_tiers: Option<String>,
    pub skills: Option<Vec<RawSkill>>,
    pub skills_passive: Option<Vec<RawSkill>>,
    pub quests: Option<serde_json::Value>,
    pub descriptions: Option<Vec<RawSkill>>,
    pub enchantments: Option<serde_json::Value>,
    pub image: Option<String>,
    #[serde(default)]
    pub description_cn: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ItemData {
    #[serde(alias = "id")]
    pub uuid: String,
    pub name: String,
    pub name_cn: String,
    pub tier: String,
    pub available_tiers: String,
    pub tags: String,
    pub hidden_tags: String,
    pub size: Option<String>,
    pub processed_tags: Vec<String>,
    pub heroes: Option<String>,
    pub cooldown: Option<f32>,
    pub cooldown_tiers: String,
    pub damage_tiers: String,
    pub damage: Option<i32>,
    pub heal_tiers: String,
    pub heal: Option<i32>,
    pub shield_tiers: String,
    pub shield: Option<i32>,
    pub ammo_tiers: String,
    pub ammo: Option<i32>,
    pub crit_tiers: String,
    pub crit: Option<i32>,
    pub multicast_tiers: String,
    pub multicast: Option<i32>,
    pub burn_tiers: String,
    pub burn: Option<i32>,
    pub poison_tiers: String,
    pub poison: Option<i32>,
    pub regen_tiers: String,
    pub regen: Option<i32>,
    pub lifesteal_tiers: String,
    pub lifesteal: Option<i32>,
    pub skills: Vec<SkillText>,
    pub skills_passive: Option<Vec<SkillText>>,
    pub quests: Option<serde_json::Value>,
    pub enchantments: Vec<String>,
    pub description: String,
    pub instance_id: Option<String>,
    pub description_cn: Option<String>, // Added this
    pub image: Option<String>, // Added this
}

impl From<RawItem> for ItemData {
    fn from(raw: RawItem) -> Self {
        let name_en = raw.name_en.clone().unwrap_or_else(|| "Unknown".to_string());
        let name_cn = raw.name_cn.clone().unwrap_or_else(|| name_en.clone());

        let processed_tags = raw.tags.as_deref().unwrap_or_default()
            .split('|')
            .map(|s| {
                let part = s.trim();
                // Pick the last part after / if it exists
                part.split(" / ").last().unwrap_or(part).trim().to_string()
            })
            .filter(|s| !s.is_empty())
            .filter(|s| !s.contains("隐藏") && !s.contains("Hide") && !s.contains("Hidden"))
            .collect();

        // 提取隐藏标签
        let hidden_tags = raw.hidden_tags.unwrap_or_default();

        // Use descriptions if skills is empty (for skill-type items from skills_db)
        let skill_source = if raw.skills.is_some() { 
            raw.skills.unwrap_or_default() 
        } else { 
            raw.descriptions.unwrap_or_default() 
        };
        
        let skills = skill_source.into_iter()
            .map(|s| SkillText {
                en: s.en.unwrap_or_default(),
                cn: s.cn.unwrap_or_default(),
            })
            .filter(|s| !s.cn.is_empty() || !s.en.is_empty())
            .collect();
        
        // Handle skills_passive
        let skills_passive = raw.skills_passive.map(|passive_skills| {
            passive_skills.into_iter()
                .map(|s| SkillText {
                    en: s.en.unwrap_or_default(),
                    cn: s.cn.unwrap_or_default(),
                })
                .filter(|s| !s.cn.is_empty() || !s.en.is_empty())
                .collect()
        });
        
        // Handle quests
        let quests = raw.quests;
        
        // Handle enchantments
        let mut enchantments = Vec::new();
        if let Some(val) = raw.enchantments {
            if let Some(obj) = val.as_object() {
                for (_key, details) in obj {
                    let name_cn = details.get("name_cn").and_then(|v| v.as_str());
                    let effect_cn = details.get("effect_cn").and_then(|v| v.as_str());
                    let effect_en = details.get("effect_en").and_then(|v| v.as_str());
                    
                    let effect = effect_cn.or(effect_en);
                    if let Some(eff) = effect {
                        if let Some(n) = name_cn {
                            // 使用分隔符方便前端拆分名称和描述
                            enchantments.push(format!("{}|{}", n, eff));
                        } else {
                            enchantments.push(eff.to_string());
                        }
                    }
                }
            }
        }
        
        let damage = raw.damage;
        let heal = raw.heal;
        let shield = raw.shield;
        let ammo = raw.ammo;
        let crit = raw.crit;
        let multicast = raw.multicast;
        let burn = raw.burn;
        let poison = raw.poison;
        let regen = raw.regen;
        let lifesteal = raw.lifesteal;
        // Removed .sort() to keep JSON order

        ItemData {
            uuid: raw.id,
            name: name_en,
            name_cn,
            tier: raw.starting_tier.clone().unwrap_or_else(|| "Bronze".to_string()),
            available_tiers: raw.available_tiers.unwrap_or_default(),
            tags: raw.tags.unwrap_or_default(),
            hidden_tags,
            size: raw.size,
            processed_tags,
            heroes: raw.heroes,
            cooldown: raw.cooldown,
            cooldown_tiers: raw.cooldown_tiers.unwrap_or_default(),
            damage_tiers: raw.damage_tiers.unwrap_or_default(),
            damage,
            heal_tiers: raw.heal_tiers.unwrap_or_default(),
            heal,
            shield_tiers: raw.shield_tiers.unwrap_or_default(),
            shield,
            ammo_tiers: raw.ammo_tiers.unwrap_or_default(),
            ammo,
            crit_tiers: raw.crit_tiers.unwrap_or_default(),
            crit,
            multicast_tiers: raw.multicast_tiers.unwrap_or_default(),
            multicast,
            burn_tiers: raw.burn_tiers.unwrap_or_default(),
            burn,
            poison_tiers: raw.poison_tiers.unwrap_or_default(),
            poison,
            regen_tiers: raw.regen_tiers.unwrap_or_default(),
            regen,
            lifesteal_tiers: raw.lifesteal_tiers.unwrap_or_default(),
            lifesteal,
            skills,
            skills_passive,
            quests,
            enchantments,
            description: "".to_string(), // will be populated
            instance_id: None, // Used for tracked stash items
            description_cn: raw.description_cn,
            image: raw.image,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TierInfo {
    pub description: Vec<String>,
    pub extra_description: Vec<String>,
    pub cd: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SkillText {
    pub en: String,
    pub cn: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MonsterSubItem {
    pub id: Option<String>,
    pub name: String,
    pub name_en: Option<String>,
    pub tier: Option<String>,
    pub current_tier: Option<String>,
    pub starting_tier: Option<String>,
    pub tags: Option<Vec<String>>,
    pub tiers: Option<HashMap<String, Option<TierInfo>>>,
    pub size: Option<String>,
    pub damage_tiers: Option<String>,
    pub heal_tiers: Option<String>,
    pub shield_tiers: Option<String>,
    pub ammo_tiers: Option<String>,
    pub burn_tiers: Option<String>,
    pub poison_tiers: Option<String>,
    pub regen_tiers: Option<String>,
    pub lifesteal_tiers: Option<String>,
    pub multicast_tiers: Option<String>,
    pub cooldown: Option<i32>,
    pub cooldown_tiers: Option<String>,
    pub skills: Option<Vec<SkillText>>,
    pub damage: Option<i32>,
    pub heal: Option<i32>,
    pub shield: Option<i32>,
    pub burn: Option<i32>,
    pub poison: Option<i32>,
    pub regen: Option<i32>,
    pub lifesteal: Option<i32>,
    pub ammo: Option<i32>,
    pub multicast: Option<i32>,
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MonsterData {
    pub name: String,
    pub name_zh: String,
    pub available: Option<String>,
    pub health: Option<serde_json::Value>,
    pub level: Option<serde_json::Value>,
    pub skills: Option<Vec<MonsterSubItem>>,
    pub items: Option<Vec<MonsterSubItem>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SyncPayload {
    pub hand_items: Vec<ItemData>,
    pub stash_items: Vec<ItemData>,
    pub all_tags: Vec<String>,
}

pub struct ItemDb {
    pub list: Vec<ItemData>,
    pub id_map: HashMap<String, usize>,
    pub unique_tags: Vec<String>,
}

pub struct SkillDb {
    pub list: Vec<ItemData>, // Skills have similar structure
    pub id_map: HashMap<String, usize>,
}

pub struct DbState {
    pub items: Arc<RwLock<ItemDb>>,
    pub skills: Arc<RwLock<SkillDb>>,
    pub monsters: Arc<RwLock<serde_json::Map<String, serde_json::Value>>>,
}
