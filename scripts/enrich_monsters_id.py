import json
import os

def enrich_monsters():
    base_dir = r"d:\Projects\BazaarHelper\src-tauri\resources"
    items_path = os.path.join(base_dir, "items_db.json")
    skills_path = os.path.join(base_dir, "skills_db.json")
    monsters_path = os.path.join(base_dir, "monsters_db.json")

    # Load items and skills to create name -> id mapping
    name_to_id = {}
    
    if os.path.exists(items_path):
        print(f"Loading items from {items_path}")
        with open(items_path, 'r', encoding='utf-8') as f:
            items = json.load(f)
            print(f"Loaded {len(items)} items")
            for item in items:
                name = item.get("name_cn")
                if name:
                    name_to_id[name] = item.get("id")

    if os.path.exists(skills_path):
        print(f"Loading skills from {skills_path}")
        with open(skills_path, 'r', encoding='utf-8') as f:
            skills = json.load(f)
            print(f"Loaded {len(skills)} skills")
            for skill in skills:
                # Some skills might have name_cn or just name depending on format
                name = skill.get("name_cn") or skill.get("name")
                if name:
                    name_to_id[name] = skill.get("id")

    print(f"Mapping size: {len(name_to_id)}")

    # Load monsters
    if not os.path.exists(monsters_path):
        print(f"monsters_db.json not found at {monsters_path}")
        return

    print(f"Loading monsters from {monsters_path}")
    with open(monsters_path, 'r', encoding='utf-8') as f:
        monsters = json.load(f)
    print(f"Loaded {len(monsters)} monsters")

    # Enrich
    updated_count = 0
    missing_count = 0
    for m_name, m_data in monsters.items():
        # Items
        if "items" in m_data:
            for it in m_data["items"]:
                name = it.get("name")
                if name in name_to_id:
                    new_id = name_to_id[name]
                    if it.get("id") != new_id:
                        it["id"] = new_id
                        updated_count += 1
                else:
                    missing_count += 1
        
        # Skills
        if "skills" in m_data:
            for sk in m_data["skills"]:
                name = sk.get("name")
                if name in name_to_id:
                    new_id = name_to_id[name]
                    if sk.get("id") != new_id:
                        sk["id"] = new_id
                        updated_count += 1
                else:
                    missing_count += 1

    # Save back
    with open(monsters_path, 'w', encoding='utf-8') as f:
        json.dump(monsters, f, ensure_ascii=False, indent=2)

    print(f"Successfully updated/verified {updated_count} IDs in monsters_db.json")
    print(f"Items/Skills with no matching ID found: {missing_count}")

if __name__ == "__main__":
    enrich_monsters()
