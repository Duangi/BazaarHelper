import json
import os
import shutil

def rename_images_to_id():
    base_dir = r"d:\Projects\BazaarHelper\src-tauri\resources"
    items_path = os.path.join(base_dir, "items_db.json")
    skills_path = os.path.join(base_dir, "skills_db.json")
    monsters_path = os.path.join(base_dir, "monsters_db.json")
    v2_mapping_path = os.path.join(base_dir, "v2_cards_id_to_chinese.json")
    images_dir = os.path.join(base_dir, "images")

    # 1. Load data
    name_to_id = {}
    all_ids = set()
    id_to_name = {}

    def add_to_mapping(name, id):
        if not name or not id:
            return
        name = name.strip()
        name_to_id[name] = id
        all_ids.add(id)
        if id not in id_to_name:
            id_to_name[id] = name

    # Load from mapping files
    if os.path.exists(v2_mapping_path):
        with open(v2_mapping_path, 'r', encoding='utf-8-sig') as f:
            v2_mapping = json.load(f)
            for cid, cname in v2_mapping.items():
                add_to_mapping(cname, cid)

    # Load from monsters_db
    if os.path.exists(monsters_path):
        with open(monsters_path, 'r', encoding='utf-8-sig') as f:
            monsters = json.load(f)
            for m_name, m_data in monsters.items():
                # Add monster itself
                add_to_mapping(m_name, m_data.get("id"))
                add_to_mapping(m_data.get("name_zh"), m_data.get("id"))
                
                # Add its items and skills
                for list_key in ["items", "skills"]:
                    if list_key in m_data:
                        for it in m_data[list_key]:
                            add_to_mapping(it.get("name"), it.get("id"))

    # Load from items/skills db
    if os.path.exists(items_path):
        with open(items_path, 'r', encoding='utf-8-sig') as f:
            items = json.load(f)
            for item in items:
                add_to_mapping(item.get("name_cn"), item.get("id"))

    if os.path.exists(skills_path):
        with open(skills_path, 'r', encoding='utf-8-sig') as f:
            skills = json.load(f)
            for skill in skills:
                name = skill.get("name_cn") or skill.get("name")
                add_to_mapping(name, skill.get("id"))

    # 2. Rename images
    renamed_count = 0
    unmatched_images = []
    
    if os.path.exists(images_dir):
        files = os.listdir(images_dir)
        for filename in files:
            if not filename.lower().endswith(('.jpg', '.png', '.webp')):
                continue
                
            name_part, ext = os.path.splitext(filename)
            name_clean = name_part.strip()
            
            target_id = None
            if name_clean in name_to_id:
                target_id = name_to_id[name_clean]
            elif name_clean.split('_')[0] in name_to_id:
                target_id = name_to_id[name_clean.split('_')[0]]

            if target_id:
                old_path = os.path.join(images_dir, filename)
                new_path = os.path.join(images_dir, f"{target_id}{ext}")
                
                if old_path == new_path:
                    continue
                
                if os.path.exists(new_path):
                    try:
                        os.remove(old_path)
                    except:
                        pass
                else:
                    try:
                        os.rename(old_path, new_path)
                        renamed_count += 1
                    except Exception as e:
                        print(f"Error renaming {filename}: {e}")
            elif name_clean in all_ids:
                pass
            else:
                unmatched_images.append(filename)

    # 3. Final Check for missing images
    current_images = set()
    if os.path.exists(images_dir):
        for f in os.listdir(images_dir):
            name_part, _ = os.path.splitext(f)
            current_images.add(name_part)

    missing_ids = []
    for id in sorted(all_ids):
        if id not in current_images:
            name = id_to_name.get(id, "Unknown")
            missing_ids.append(f"{id}: {name}")

    with open(os.path.join(r"d:\Projects\BazaarHelper", "missing_images_report.txt"), 'w', encoding='utf-8') as f:
        f.write("\n".join(missing_ids))

    # 4. Update monsters_db.json
    if os.path.exists(monsters_path):
        with open(monsters_path, 'r', encoding='utf-8-sig') as f:
            monsters = json.load(f)
        
        monster_updated = 0
        for m_name, m_data in monsters.items():
            # Update monster image path if it exists
            # We don't have an "image" field at top level in monsters_db.json usually
            # but let's check if there is one
            if "image" in m_data:
                m_id = m_data.get("id")
                if m_id:
                    ext = ".jpg"
                    if "images/" in m_data["image"]:
                         _, ext = os.path.splitext(m_data["image"])
                    m_data["image"] = f"images/{m_id}{ext}"
                    monster_updated += 1

            for list_key in ["items", "skills"]:
                if list_key in m_data:
                    for it in m_data[list_key]:
                        it_id = it.get("id")
                        if it_id:
                            ext = ".jpg"
                            old_img = it.get("image", "")
                            if old_img:
                                _, discovered_ext = os.path.splitext(old_img)
                                if discovered_ext:
                                    ext = discovered_ext
                            
                            new_val = f"images/{it_id}{ext}"
                            if it.get("image") != new_val:
                                it["image"] = new_val
                                monster_updated += 1
                                
        with open(monsters_path, 'w', encoding='utf-8') as f:
            json.dump(monsters, f, ensure_ascii=False, indent=2)

    print(f"Total renamed: {renamed_count}")
    print(f"Total unmatched Chinese files: {len(unmatched_images)}")
    print(f"Total missing assets (reported): {len(missing_ids)}")

if __name__ == "__main__":
    rename_images_to_id()
