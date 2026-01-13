import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MONSTERS_JSON = ROOT / 'src-tauri' / 'resources' / 'monsters_db.json'
IMAGES_DIR = ROOT / 'src-tauri' / 'resources' / 'images'

def main():
    if not MONSTERS_JSON.exists():
        print(f"monsters_db.json not found at {MONSTERS_JSON}")
        return
    if not IMAGES_DIR.exists():
        print(f"images folder not found at {IMAGES_DIR}")
        return

    with open(MONSTERS_JSON, 'r', encoding='utf-8') as f:
        data = json.load(f)

    bak = MONSTERS_JSON.with_suffix('.json.images.bak')
    with open(bak, 'w', encoding='utf-8') as f:
        json.dump(data, f, ensure_ascii=False, indent=2)
    print(f"Backup written to {bak}")

    existing = {p.stem: p.name for p in IMAGES_DIR.glob('*.jpg')}

    updated_skills = 0
    updated_items = 0
    missing_skills = 0
    missing_items = 0

    for mname, monster in data.items():
        # skills
        skills = monster.get('skills', [])
        for s in skills:
            sname = s.get('name') or s.get('id') or ''
            key = sname.strip()
            if key in existing:
                s['image'] = f"images/{existing[key]}"
                updated_skills += 1
            else:
                if 'image' not in s or s.get('image'):
                    s['image'] = ""
                missing_skills += 1

        # items
        items = monster.get('items', [])
        for it in items:
            iname = it.get('name') or it.get('id') or ''
            key = iname.strip()
            if key in existing:
                it['image'] = f"images/{existing[key]}"
                updated_items += 1
            else:
                if 'image' not in it or it.get('image'):
                    it['image'] = ""
                missing_items += 1

    with open(MONSTERS_JSON, 'w', encoding='utf-8') as f:
        json.dump(data, f, ensure_ascii=False, indent=2)

    print(f"Skills: updated={updated_skills}, missing={missing_skills}")
    print(f"Items:  updated={updated_items}, missing={missing_items}")

if __name__ == '__main__':
    main()
