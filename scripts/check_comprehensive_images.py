import json
import os
from pathlib import Path

def check_all_images():
    root = Path(__file__).resolve().parents[1]
    resources = root / 'src-tauri' / 'resources'
    images_dir = resources / 'images'
    
    items_db_path = resources / 'items_db.json'
    monsters_db_path = resources / 'monsters_db.json'
    skills_db_path = resources / 'skills_db.json'

    if not images_dir.exists():
        print(f"Error: {images_dir} not found")
        return

    # Image files (lowercase for case-insensitive matching)
    image_files = {f.name.lower(): f.name for f in images_dir.glob('*')}
    extensions = ['.webp', '.png', '.jpg', '.jpeg']

    all_entities = []

    # 1. Load units from items_db.json
    if items_db_path.exists():
        with open(items_db_path, 'r', encoding='utf-8') as f:
            items = json.load(f)
            for item in items:
                all_entities.append({
                    'id': item.get('id'),
                    'name_en': item.get('name_en'),
                    'name_cn': item.get('name_cn'),
                    'source': 'items_db.json',
                    'type': 'item'
                })

    # 2. Load units from monsters_db.json
    if monsters_db_path.exists():
        with open(monsters_db_path, 'r', encoding='utf-8') as f:
            monsters = json.load(f)
            for m_name, m_data in monsters.items():
                # Add items from monsters
                for it in m_data.get('items', []):
                    all_entities.append({
                        'id': it.get('id'),
                        'name_en': it.get('name_en') or it.get('name'),
                        'name_cn': it.get('name_zh') or it.get('name'),
                        'source': f'monsters_db.json ({m_name})',
                        'type': 'monster_item'
                    })
                # Add skills from monsters
                for sk in m_data.get('skills', []):
                    all_entities.append({
                        'id': sk.get('id'),
                        'name_en': sk.get('name_en') or sk.get('name'),
                        'name_cn': sk.get('name_zh') or sk.get('name'),
                        'source': f'monsters_db.json ({m_name})',
                        'type': 'monster_skill'
                    })

    # 3. Load units from skills_db.json
    if skills_db_path.exists():
        with open(skills_db_path, 'r', encoding='utf-8') as f:
            skills = json.load(f)
            for sk in skills:
                all_entities.append({
                    'id': sk.get('id'),
                    'name_en': sk.get('name_en'),
                    'name_cn': sk.get('name_cn'),
                    'source': 'skills_db.json',
                    'type': 'skill'
                })

    # Deduplicate by ID
    unique_entities = {}
    for ent in all_entities:
        eid = ent['id']
        if not eid:
            # Fallback to name if ID is missing
            eid = ent['name_en'] or ent['name_cn']
        
        if eid not in unique_entities:
            unique_entities[eid] = ent

    missing = []
    found_count = 0

    for eid, ent in unique_entities.items():
        id_str = str(ent['id']).lower()
        name_en = str(ent['name_en']).lower() if ent['name_en'] else ""
        name_cn = str(ent['name_cn']).lower() if ent['name_cn'] else ""
        
        found = False
        # Try ID
        for ext in extensions:
            if f"{id_str}{ext}" in image_files:
                found = True
                break
        
        if not found and name_en:
            for ext in extensions:
                if f"{name_en}{ext}" in image_files:
                    found = True
                    break
        
        if not found and name_cn:
            for ext in extensions:
                if f"{name_cn}{ext}" in image_files:
                    found = True
                    break

        if found:
            found_count += 1
        else:
            missing.append(ent)

    print(f"Total unique entities scanned: {len(unique_entities)}")
    print(f"Entities with images: {found_count}")
    print(f"Entities missing images: {len(missing)}")

    if missing:
        print("\nFirst 30 missing entities:")
        for i, ent in enumerate(missing[:30]):
            name = ent['name_cn'] or ent['name_en'] or "Unknown"
            print(f"{i+1:2}. [{ent['type']}] {name} (ID: {ent['id']}) Source: {ent['source']}")
        
        # Save to file
        output = root / 'scripts' / 'missing_images_report.json'
        with open(output, 'w', encoding='utf-8') as f:
            json.dump(missing, f, ensure_ascii=False, indent=2)
        print(f"\nFull report saved to {output}")

if __name__ == "__main__":
    check_all_images()
