use crate::{ItemData, SkillText};

pub fn construct_monster_sub_item(
    item_data: Option<ItemData>,
    fallback_name_cn: &str,
    fallback_name_en: &str,
    current_tier: &str,
    override_size: Option<&str>,
) -> serde_json::Value {
    let mut desc = Vec::new();
    let mut name_cn = fallback_name_cn.to_string();
    let mut name_en = fallback_name_en.to_string();
    let mut cooldown = None;
    let mut size = override_size.map(|s| s.to_string());
    let mut id = "".to_string();
    let mut tiers = serde_json::Map::new();
    let mut skills: Vec<SkillText> = Vec::new();
    let mut damage_tiers = None;
    let mut heal_tiers = None;
    let mut shield_tiers = None;
    let mut ammo_tiers = None;
    let mut burn_tiers = None;
    let mut poison_tiers = None;
    let mut regen_tiers = None;
    let mut lifesteal_tiers = None;
    let mut multicast_tiers = None;
    let mut cooldown_tiers = None;
    let mut starting_tier: Option<String> = None;

    let mut damage_val = None;
    let mut heal_val = None;
    let mut shield_val = None;
    let mut burn_val = None;
    let mut poison_val = None;
    let mut regen_val = None;
    let mut lifesteal_val = None;
    let mut ammo_val = None;
    let mut multicast_val = None;

    if let Some(item) = item_data {
        name_cn = item.name_cn;
        name_en = item.name;
        id = item.uuid;
        starting_tier = Some(item.tier.clone());

        if size.is_none() {
            size = item.size;
        }
        if !item.description.is_empty() {
            desc.push(item.description.clone());
        }

        skills = item.skills.clone();

        for skill in &item.skills {
            let skill_text = if !skill.cn.is_empty() {
                &skill.cn
            } else {
                &skill.en
            };
            if !skill_text.is_empty() {
                desc.push(skill_text.clone());
            }
        }
        cooldown = item.cooldown;

        damage_val = item.damage;
        heal_val = item.heal;
        shield_val = item.shield;
        burn_val = item.burn;
        poison_val = item.poison;
        regen_val = item.regen;
        lifesteal_val = item.lifesteal;
        ammo_val = item.ammo;
        multicast_val = item.multicast;

        damage_tiers = if !item.damage_tiers.is_empty() {
            Some(item.damage_tiers.clone())
        } else {
            None
        };
        heal_tiers = if !item.heal_tiers.is_empty() {
            Some(item.heal_tiers.clone())
        } else {
            None
        };
        shield_tiers = if !item.shield_tiers.is_empty() {
            Some(item.shield_tiers.clone())
        } else {
            None
        };
        ammo_tiers = if !item.ammo_tiers.is_empty() {
            Some(item.ammo_tiers.clone())
        } else {
            None
        };
        burn_tiers = if !item.burn_tiers.is_empty() {
            Some(item.burn_tiers.clone())
        } else {
            None
        };
        poison_tiers = if !item.poison_tiers.is_empty() {
            Some(item.poison_tiers.clone())
        } else {
            None
        };
        regen_tiers = if !item.regen_tiers.is_empty() {
            Some(item.regen_tiers.clone())
        } else {
            None
        };
        lifesteal_tiers = if !item.lifesteal_tiers.is_empty() {
            Some(item.lifesteal_tiers.clone())
        } else {
            None
        };
        multicast_tiers = if !item.multicast_tiers.is_empty() {
            Some(item.multicast_tiers.clone())
        } else {
            None
        };
        cooldown_tiers = if !item.cooldown_tiers.is_empty() {
            Some(item.cooldown_tiers.clone())
        } else {
            None
        };

        if !item.available_tiers.is_empty() {
            let avail_list: Vec<&str> = item.available_tiers.split('/').collect();
            let cd_list: Vec<&str> = item.cooldown_tiers.split('/').collect();

            for (i, t_name) in avail_list.iter().enumerate() {
                let mut t_info = serde_json::Map::new();
                t_info.insert(
                    "description".to_string(),
                    serde_json::Value::Array(
                        desc.iter()
                            .map(|s| serde_json::Value::String(s.clone()))
                            .collect(),
                    ),
                );
                t_info.insert(
                    "extra_description".to_string(),
                    serde_json::Value::Array(vec![]),
                );

                let cd_val = if i < cd_list.len() {
                    let ms: f32 = cd_list[i].trim().parse().unwrap_or(0.0);
                    if ms > 0.0 {
                        Some(format!("{:.1}s", ms / 1000.0))
                    } else {
                        None
                    }
                } else if !cd_list.is_empty() && !cd_list[0].is_empty() {
                    let ms: f32 = cd_list.last().unwrap().trim().parse().unwrap_or(0.0);
                    if ms > 0.0 {
                        Some(format!("{:.1}s", ms / 1000.0))
                    } else {
                        None
                    }
                } else if i == 0 {
                    cooldown.map(|c| format!("{:.1}s", c))
                } else {
                    None
                };

                t_info.insert(
                    "cd".to_string(),
                    cd_val
                        .map(serde_json::Value::String)
                        .unwrap_or(serde_json::Value::Null),
                );
                tiers.insert(t_name.to_lowercase(), serde_json::Value::Object(t_info));
            }
        }
    }

    if tiers.is_empty() || !tiers.contains_key(&current_tier.to_lowercase()) {
        let mut t_info = serde_json::Map::new();
        t_info.insert(
            "description".to_string(),
            serde_json::Value::Array(desc.into_iter().map(serde_json::Value::String).collect()),
        );
        t_info.insert(
            "extra_description".to_string(),
            serde_json::Value::Array(vec![]),
        );
        t_info.insert(
            "cd".to_string(),
            cooldown
                .map(|c| serde_json::Value::String(format!("{:.1}s", c)))
                .unwrap_or(serde_json::Value::Null),
        );

        tiers.insert(current_tier.to_lowercase(), serde_json::Value::Object(t_info));
    }

    let tier_label = format!("{}+", current_tier);

    let mut sub = serde_json::Map::new();
    sub.insert("name".to_string(), serde_json::Value::String(name_cn));
    sub.insert("name_en".to_string(), serde_json::Value::String(name_en));
    sub.insert("id".to_string(), serde_json::Value::String(id));
    sub.insert("tier".to_string(), serde_json::Value::String(tier_label));
    sub.insert(
        "current_tier".to_string(),
        serde_json::Value::String(current_tier.to_string()),
    );

    let final_size = size.map(|s| s.split(" / ").next().unwrap_or(&s).to_string());

    sub.insert(
        "size".to_string(),
        final_size
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null),
    );
    sub.insert("tiers".to_string(), serde_json::Value::Object(tiers));
    sub.insert(
        "damage_tiers".to_string(),
        damage_tiers
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null),
    );
    sub.insert(
        "heal_tiers".to_string(),
        heal_tiers
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null),
    );
    sub.insert(
        "shield_tiers".to_string(),
        shield_tiers
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null),
    );
    sub.insert(
        "ammo_tiers".to_string(),
        ammo_tiers
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null),
    );
    sub.insert(
        "burn_tiers".to_string(),
        burn_tiers
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null),
    );
    sub.insert(
        "poison_tiers".to_string(),
        poison_tiers
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null),
    );
    sub.insert(
        "regen_tiers".to_string(),
        regen_tiers
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null),
    );
    sub.insert(
        "lifesteal_tiers".to_string(),
        lifesteal_tiers
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null),
    );
    sub.insert(
        "multicast_tiers".to_string(),
        multicast_tiers
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null),
    );
    sub.insert(
        "cooldown".to_string(),
        cooldown
            .map(|c| serde_json::Value::Number((c as i32).into()))
            .unwrap_or(serde_json::Value::Null),
    );
    sub.insert(
        "cooldown_tiers".to_string(),
        cooldown_tiers
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null),
    );
    sub.insert(
        "skills".to_string(),
        serde_json::to_value(skills).unwrap_or(serde_json::Value::Null),
    );
    sub.insert(
        "starting_tier".to_string(),
        starting_tier
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null),
    );

    if let Some(v) = damage_val {
        sub.insert("damage".to_string(), serde_json::Value::Number(v.into()));
    }
    if let Some(v) = heal_val {
        sub.insert("heal".to_string(), serde_json::Value::Number(v.into()));
    }
    if let Some(v) = shield_val {
        sub.insert("shield".to_string(), serde_json::Value::Number(v.into()));
    }
    if let Some(v) = burn_val {
        sub.insert("burn".to_string(), serde_json::Value::Number(v.into()));
    }
    if let Some(v) = poison_val {
        sub.insert("poison".to_string(), serde_json::Value::Number(v.into()));
    }
    if let Some(v) = regen_val {
        sub.insert("regen".to_string(), serde_json::Value::Number(v.into()));
    }
    if let Some(v) = lifesteal_val {
        sub.insert("lifesteal".to_string(), serde_json::Value::Number(v.into()));
    }
    if let Some(v) = ammo_val {
        sub.insert("ammo".to_string(), serde_json::Value::Number(v.into()));
    }
    if let Some(v) = multicast_val {
        sub.insert("multicast".to_string(), serde_json::Value::Number(v.into()));
    }

    serde_json::Value::Object(sub)
}
