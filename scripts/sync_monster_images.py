import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MONSTERS_JSON = ROOT / 'src-tauri' / 'resources' / 'monsters_db.json'
IMAGES_DIR = ROOT / 'src-tauri' / 'resources' / 'images_monster'

def main():
    if not MONSTERS_JSON.exists():
        print(f"monsters_db.json not found at {MONSTERS_JSON}")
        return
    if not IMAGES_DIR.exists():
        print(f"images_monster folder not found at {IMAGES_DIR}")
        return

    with open(MONSTERS_JSON, 'r', encoding='utf-8') as f:
        data = json.load(f)

    # Backup
    bak = MONSTERS_JSON.with_suffix('.json.bak')
    with open(bak, 'w', encoding='utf-8') as f:
        json.dump(data, f, ensure_ascii=False, indent=2)
    print(f"Backup written to {bak}")

    existing_files = {p.name for p in IMAGES_DIR.glob('*.jpg')}
    missing = []

    changed = False
    for name, obj in data.items():
        expected = f"{name}.jpg"
        img_path = f"images_monster/{expected}"
        if expected in existing_files:
            if obj.get('image') != img_path:
                obj['image'] = img_path
                changed = True
        else:
            missing.append(name)

    if missing:
        print("Missing image files for the following monsters:")
        for m in missing:
            print(f" - {m} -> images_monster/{m}.jpg")
    else:
        print("All monster images exist in images_monster/")

    if changed:
        with open(MONSTERS_JSON, 'w', encoding='utf-8') as f:
            json.dump(data, f, ensure_ascii=False, indent=2)
        print(f"Updated {MONSTERS_JSON} image paths.")
    else:
        print("No changes needed to monsters_db.json")

if __name__ == '__main__':
    main()
