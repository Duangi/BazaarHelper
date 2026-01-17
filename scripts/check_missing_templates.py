import json
import os

db_path = r"d:\Projects\BazaarHelper\src-tauri\resources\monsters_db.json"
img_char_dir = r"d:\Projects\BazaarHelper\src-tauri\resources\images_monster_char"
img_bg_dir = r"d:\Projects\BazaarHelper\src-tauri\resources\images_monster_bg"
resources_dir = r"d:\Projects\BazaarHelper\src-tauri\resources"

with open(db_path, 'r', encoding='utf-8') as f:
    monsters = json.load(f)

missing = []
for key, entry in monsters.items():
    if "available" in entry and entry["available"]:
        found = False
        # 1. Check path from DB
        if "image" in entry and entry["image"]:
            p = os.path.join(resources_dir, entry["image"])
            if os.path.exists(p):
                found = True
            else:
                p_char = entry["image"].replace("images_monster/", "images_monster_char/")
                if os.path.exists(os.path.join(resources_dir, p_char)):
                    found = True
        
        # 2. Check fallback (Chinese name)
        if not found:
            char_path = os.path.join(img_char_dir, f"{key}.webp")
            if os.path.exists(char_path):
                found = True
            
        if not found:
            bg_path = os.path.join(img_bg_dir, f"{key}.webp")
            if os.path.exists(bg_path):
                found = True
        
        if not found:
            missing.append(f"{key} ({entry.get('available')})")

print(f"Total missing images for monsters with 'available': {len(missing)}")
for m in missing:
    print(f" - {m}")
