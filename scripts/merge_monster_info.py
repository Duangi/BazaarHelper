import json
import os

def merge_data():
    db_path = r"d:\Projects\BazaarHelper\src-tauri\resources\monsters_db.json"
    enc_path = r"d:\Projects\BazaarHelper\src-tauri\resources\combat_encounters.json"
    
    with open(db_path, "r", encoding="utf-8") as f:
        db = json.load(f)
        
    with open(enc_path, "r", encoding="utf-8") as f:
        enc = json.load(f)
        
    updated_count = 0
    
    # Create a mapping for encounters by name_zh to handle cases where names might slightly differ or have variants
    # Actually combat_encounters.json seems to use name_zh as key too
    
    for key, monster in db.items():
        available = monster.get("available", "")
        # Check if it's Day 3 or later
        # Day 1, Day 2 are skipped. Day 10+ is included.
        if not available or available in ["Day 1", "Day 2"]:
            continue
            
        name_zh = monster.get("name_zh")
        if not name_zh:
            continue
            
        if name_zh in enc:
            source = enc[name_zh]
            
            # Supplement skills if empty
            if not monster.get("skills"):
                # We need to transform the skill structure if it differs
                # Let's check if we need to map fields
                # monsters_db expect: tiers (Record<string, TierInfo>), etc.
                # combat_encounters has: description (string)
                # This might be tricky because monsters_db uses a more complex structure (tiers)
                # than combat_encounters' simple description.
                pass

            # Supplement items if empty
            if not monster.get("items"):
                pass
                
    # Wait, looking at the attachments:
    # monsters_db skills have "tiers" field with "bronze", "silver", "gold", "diamond".
    # combat_encounters skills have "description" field with " » " for tiers.
    
    # I need to parse the description from combat_encounters and put it into the correct tier in monsters_db.

def parse_description(desc):
    # Split "X » Y » Z" into [X, Y, Z]
    if not desc:
        return []
    parts = [p.strip() for p in desc.split("»")]
    return parts

def main():
    db_path = r"d:\Projects\BazaarHelper\src-tauri\resources\monsters_db.json"
    enc_path = r"d:\Projects\BazaarHelper\src-tauri\resources\combat_encounters.json"
    
    with open(db_path, "r", encoding="utf-8") as f:
        db = json.load(f)
        
    with open(enc_path, "r", encoding="utf-8") as f:
        enc = json.load(f)
        
    updated_count = 0
    
    for key, monster in db.items():
        available = monster.get("available", "")
        if not available or available in ["Day 1", "Day 2"]:
            continue
            
        name_zh = monster.get("name_zh")
        if not name_zh or name_zh not in enc:
            continue
            
        source = enc[name_zh]
        updated_this_one = False
        
        if "skills" not in monster or monster["skills"] is None:
            monster["skills"] = []
        if "items" not in monster or monster["items"] is None:
            monster["items"] = []

        existing_skill_names = {s.get("name") for s in monster["skills"] if s.get("name")}
        existing_item_names = {i.get("name") for i in monster["items"] if i.get("name")}

        # Supplement skills
        if source.get("skills"):
            for s in source["skills"]:
                s_name = s.get("name")
                if s_name and s_name not in existing_skill_names:
                    desc_parts = parse_description(s.get("description", ""))
                    tiers = {
                        "bronze": {"description": [desc_parts[0]], "extra_description": [], "cd": None} if len(desc_parts) > 0 else None,
                        "silver": {"description": [desc_parts[1]], "extra_description": [], "cd": None} if len(desc_parts) > 1 else None,
                        "gold": {"description": [desc_parts[2]], "extra_description": [], "cd": None} if len(desc_parts) > 2 else None,
                        "diamond": {"description": [desc_parts[3]], "extra_description": [], "cd": None} if len(desc_parts) > 3 else None,
                    }
                    if tiers["bronze"] and not tiers["silver"]: tiers["silver"] = tiers["bronze"]
                    if tiers["silver"] and not tiers["gold"]: tiers["gold"] = tiers["silver"]
                    if tiers["gold"] and not tiers["diamond"]: tiers["diamond"] = tiers["gold"]
                    
                    monster["skills"].append({
                        "id": s.get("id", ""),
                        "name": s_name,
                        "name_en": s.get("name_en", ""),
                        "tier": s.get("tier", "").upper(),
                        "current_tier": s.get("tier", "Bronze"),
                        "tags": [],
                        "tiers": tiers,
                        "image": s.get("image", "")
                    })
                    updated_this_one = True

        # Supplement items
        if source.get("items"):
            for item in source["items"]:
                i_name = item.get("name")
                if i_name and i_name not in existing_item_names:
                    desc_parts = parse_description(item.get("description", ""))
                    tiers = {
                        "bronze": {"description": [desc_parts[0]], "extra_description": [], "cd": None} if len(desc_parts) > 0 else None,
                        "silver": {"description": [desc_parts[1]], "extra_description": [], "cd": None} if len(desc_parts) > 1 else None,
                        "gold": {"description": [desc_parts[2]], "extra_description": [], "cd": None} if len(desc_parts) > 2 else None,
                        "diamond": {"description": [desc_parts[3]], "extra_description": [], "cd": None} if len(desc_parts) > 3 else None,
                    }
                    if tiers["bronze"] and not tiers["silver"]: tiers["silver"] = tiers["bronze"]
                    if tiers["silver"] and not tiers["gold"]: tiers["gold"] = tiers["silver"]
                    if tiers["gold"] and not tiers["diamond"]: tiers["diamond"] = tiers["gold"]

                    monster["items"].append({
                        "id": item.get("id", ""),
                        "name": i_name,
                        "tier": item.get("tier", "").upper(),
                        "current_tier": item.get("tier", "Bronze"),
                        "tags": [],
                        "tiers": tiers,
                        "image": item.get("image", "")
                    })
                    updated_this_one = True
        
        if updated_this_one:
            updated_count += 1
            
    with open(db_path, "w", encoding="utf-8") as f:
        json.dump(db, f, ensure_ascii=False, indent=2)
    
    print(f"Updated {updated_count} entries.")

if __name__ == "__main__":
    main()
