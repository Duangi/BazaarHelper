pub fn calculate_day_from_log(content: &str, retro: bool) -> Option<u32> {
    let start_pos = if retro {
        content.rfind("NetMessageRunInitialized").unwrap_or(0)
    } else {
        0
    };
    let slice = &content[start_pos..];
    let mut current_day: u32 = 1;
    let mut in_pvp = false;
    let mut hour_count: u32 = 0;

    for line in slice.lines() {
        let l = line.trim();
        if l.contains("NetMessageRunInitialized") {
            current_day = 1;
            in_pvp = false;
            hour_count = 0;
            continue;
        }

        if l.contains("to [PVPCombatState]") {
            in_pvp = true;
            continue;
        }

        if in_pvp && l.contains("State changed") && (l.contains("to [ChoiceState]") || l.contains("to [LevelUpState]")) {
            current_day = current_day.saturating_add(1);
            in_pvp = false;
            hour_count = 0;
            continue;
        }

        if l.starts_with('[') && l.contains("State changed from [ChoiceState] to [") {
            if !l.contains("to [ChoiceState]") && !l.contains("to [PVPCombatState]") {
                hour_count = hour_count.saturating_add(1);
                if hour_count >= 10 {
                    current_day = current_day.saturating_add(1);
                    hour_count = 0;
                }
            }
        }
    }

    Some(current_day)
}
