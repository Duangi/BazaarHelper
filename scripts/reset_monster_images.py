import json
import os

def reset_monster_images():
    path = r"d:\Projects\BazaarHelper\src-tauri\resources\monsters_db.json"
    if not os.path.exists(path):
        print("File not found")
        return
        
    with open(path, 'r', encoding='utf-8-sig') as f:
        monsters = json.load(f)
        
    changes = 0
    for m_name, m_data in monsters.items():
        name_zh = m_data.get("name_zh") or m_name
        # Use images_monster/中文名.jpg for the monster itself
        new_val = f"images_monster/{name_zh}.jpg"
        if m_data.get("image") != new_val:
            m_data["image"] = new_val
            changes += 1
            
    if changes > 0:
        with open(path, 'w', encoding='utf-8') as f:
            json.dump(monsters, f, ensure_ascii=False, indent=2)
        print(f"Updated {changes} monster image paths to use Chinese names in images_monster/")
    else:
        print("No changes needed")

if __name__ == "__main__":
    reset_monster_images()
