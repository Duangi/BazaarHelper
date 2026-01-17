import json
import os
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DB_PATH = ROOT / 'src-tauri' / 'resources' / 'monsters_db.json'
ITEMS_DB = ROOT / 'src-tauri' / 'resources' / 'items_db.json'
SKILLS_DB = ROOT / 'src-tauri' / 'resources' / 'skills_db.json'

def load_json(p):
    with open(p, 'r', encoding='utf-8') as f:
        return json.load(f)

def cleanup_monsters():
    print(f"Loading files...")
    monsters = load_json(DB_PATH)
    items = load_json(ITEMS_DB)
    skills = load_json(SKILLS_DB)
    
    # Create name to ID maps
    item_map = {it.get('name_cn') or it.get('name'): it.get('uuid') for it in items if it.get('uuid')}
    skill_map = {sk.get('name_cn') or sk.get('name'): sk.get('uuid') for sk in skills if sk.get('uuid')}
    
    print(f"Cleaning up {len(monsters)} monster entries...")
    
    for key, monster in monsters.items():
        # 1. Remove 'image' key from monster
        if 'image' in monster:
            del monster['image']
            
        # 2. Process items
        if 'items' in monster and isinstance(monster['items'], list):
            for item in monster['items']:
                name = item.get('name')
                it_id = item.get('id')
                
                # Try to get ID if missing
                if not it_id or it_id == "":
                    it_id = item_map.get(name, "")
                    if it_id:
                        item['id'] = it_id
                
                # Remove image key
                if 'image' in item:
                    del item['image']
        
        # 3. Process skills
        if 'skills' in monster and isinstance(monster['skills'], list):
            for skill in monster['skills']:
                name = skill.get('name')
                sk_id = skill.get('id')
                
                if not sk_id or sk_id == "":
                    sk_id = skill_map.get(name, "")
                    if sk_id:
                        skill['id'] = sk_id
                
                # Remove image key
                if 'image' in skill:
                    del skill['image']
        if 'skills' in monster and isinstance(monster['skills'], list):
            for skill in monster['skills']:
                name = skill.get('name')
                sk_id = skill.get('id')
                
                # Try to get ID if missing
                if not sk_id or sk_id == "":
                    sk_id = skill_map.get(name, "")
                    if sk_id:
                        skill['id'] = sk_id
                
                # Update image path: images/[id].webp
                if sk_id:
                    skill['image'] = f"images/{sk_id}.webp"
                elif 'image' in skill:
                    skill['image'] = skill['image'].replace('.jpg', '.webp')

    with open(DB_PATH, 'w', encoding='utf-8') as f:
        json.dump(monsters, f, ensure_ascii=False, indent=2)
    
    print("Cleanup complete! 'image' keys removed and item/skill paths updated to WebP with IDs.")

if __name__ == "__main__":
    cleanup_monsters()
