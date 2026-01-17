import json
import os
import uuid

def update_monster_map_to_id():
    base_dir = r"d:\Projects\BazaarHelper\src-tauri\resources"
    map_path = os.path.join(base_dir, "images_monster_map.json")
    
    NAMESPACE_DNS = uuid.UUID('6ba7b810-9dad-11d1-80b4-00c04fd430c8')

    if not os.path.exists(map_path):
        print("images_monster_map.json not found")
        return

    with open(map_path, 'r', encoding='utf-8') as f:
        data = json.load(f)

    changes = 0
    for name, info in data.items():
        m_id = str(uuid.uuid5(NAMESPACE_DNS, name))
        ext = ".jpg"
        if "out" in info:
            _, existing_ext = os.path.splitext(info["out"])
            if existing_ext:
                ext = existing_ext
        
        new_out = f"images_monster/{m_id}{ext}"
        if info.get("out") != new_out:
            info["out"] = new_out
            changes += 1

    if changes > 0:
        with open(map_path, 'w', encoding='utf-8') as f:
            json.dump(data, f, ensure_ascii=False, indent=2)
        print(f"Updated {changes} entries in images_monster_map.json")

if __name__ == "__main__":
    update_monster_map_to_id()
