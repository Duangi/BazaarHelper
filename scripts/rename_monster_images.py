import json
import os
import shutil

def rename_monster_images_to_id():
    base_dir = r"d:\Projects\BazaarHelper\src-tauri\resources"
    monsters_path = os.path.join(base_dir, "monsters_db.json")
    monster_images_dir = os.path.join(base_dir, "images_monster")

    if not os.path.exists(monsters_path):
        print("monsters_db.json not found")
        return

    with open(monsters_path, 'r', encoding='utf-8-sig') as f:
        monsters = json.load(f)

    # 1. Build mapping and Update monsters_db.json
    name_to_id = {}
    changes = 0
    for m_name, m_data in monsters.items():
        m_id = m_data.get("id")
        name_zh = m_data.get("name_zh") or m_name
        
        if m_id:
            name_to_id[name_zh] = m_id
            
            # Update the image path in JSON
            # We want it to be images_monster/{id}.jpg
            ext = ".jpg"
            if "image" in m_data:
                _, existing_ext = os.path.splitext(m_data["image"])
                if existing_ext:
                    ext = existing_ext
            
            new_img_path = f"images_monster/{m_id}{ext}"
            if m_data.get("image") != new_img_path:
                m_data["image"] = new_img_path
                changes += 1

    if changes > 0:
        with open(monsters_path, 'w', encoding='utf-8') as f:
            json.dump(monsters, f, ensure_ascii=False, indent=2)
        print(f"Updated {changes} monster image references in monsters_db.json")

    # 2. Rename physical files
    if not os.path.exists(monster_images_dir):
        print("images_monster directory not found")
        return

    renamed_count = 0
    files = os.listdir(monster_images_dir)
    for filename in files:
        if not filename.lower().endswith(('.jpg', '.png', '.webp')):
            continue
            
        name_part, ext = os.path.splitext(filename)
        name_clean = name_part.strip()
        
        if name_clean in name_to_id:
            target_id = name_to_id[name_clean]
            old_path = os.path.join(monster_images_dir, filename)
            new_path = os.path.join(monster_images_dir, f"{target_id}{ext}")
            
            if old_path == new_path:
                continue
                
            if os.path.exists(new_path):
                # If target ID file already exists, just remove the Chinese named one
                try:
                    os.remove(old_path)
                    print(f"Removed redundant: {filename}")
                except Exception as e:
                    print(f"Error removing {filename}: {e}")
            else:
                try:
                    os.rename(old_path, new_path)
                    renamed_count += 1
                    print(f"Renamed: {filename} -> {target_id}{ext}")
                except Exception as e:
                    print(f"Error renaming {filename}: {e}")

    print(f"Successfully renamed {renamed_count} monster images to ID format.")

if __name__ == "__main__":
    rename_monster_images_to_id()
