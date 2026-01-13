import json
from pathlib import Path
import requests
from PIL import Image
from io import BytesIO

ROOT = Path(__file__).resolve().parent.parent
MONSTERS_JSON = ROOT / 'src-tauri' / 'resources' / 'monsters_db.json'
IMAGES_DIR = ROOT / 'src-tauri' / 'resources' / 'images'

def safe_filename(name: str) -> str:
    # simple sanitization
    return ''.join(c for c in name if c not in '/\\:?"<>|').strip()

def download_and_convert(url: str, dest: Path) -> bool:
    try:
        resp = requests.get(url, timeout=15)
        resp.raise_for_status()
        img = Image.open(BytesIO(resp.content))
        # convert to RGB if needed (remove alpha)
        if img.mode in ('RGBA', 'LA'):
            bg = Image.new('RGB', img.size, (0,0,0))
            bg.paste(img, mask=img.split()[-1])
            img = bg
        else:
            img = img.convert('RGB')
        dest.parent.mkdir(parents=True, exist_ok=True)
        img.save(dest, format='JPEG', quality=85)
        return True
    except Exception as e:
        print(f"Download/convert failed: {e}")
        return False

def main():
    if not MONSTERS_JSON.exists():
        print(f"monsters_db.json not found: {MONSTERS_JSON}")
        return
    with open(MONSTERS_JSON, 'r', encoding='utf-8') as f:
        data = json.load(f)

    # collect all missing entries (monster_name, type, entry_index)
    tasks = []
    for mname, monster in data.items():
        avail = monster.get('available', '')
        skills = monster.get('skills', [])
        for idx, s in enumerate(skills):
            if not s.get('image'):
                tasks.append(('skill', mname, avail, idx, s.get('name','')))
        items = monster.get('items', [])
        for idx, it in enumerate(items):
            if not it.get('image'):
                tasks.append(('item', mname, avail, idx, it.get('name','')))

    if not tasks:
        print("No missing skill/item images found.")
        return

    print(f"Found {len(tasks)} missing images. You'll be prompted for each. Press Enter to skip.")

    for typ, mname, avail, idx, entry_name in tasks:
        print("---------------------------")
        print(f"Monster: {mname}")
        print(f"Available: {avail}")
        print(f"Type: {typ}  Name: {entry_name}")
        url = input("Enter image URL (webp/png/jpg) or blank to skip: ").strip()
        if not url:
            print("Skipped.")
            continue
        safe_name = safe_filename(entry_name or mname)
        filename = f"{safe_name}.jpg"
        dest = IMAGES_DIR / filename
        ok = download_and_convert(url, dest)
        if ok:
            # update JSON
            if typ == 'skill':
                data[mname]['skills'][idx]['image'] = f"images/{filename}"
            else:
                data[mname]['items'][idx]['image'] = f"images/{filename}"
            print(f"Saved and updated: images/{filename}")
        else:
            print("Failed to save image; entry not updated.")

    # write back
    bak = MONSTERS_JSON.with_suffix('.json.fill.bak')
    with open(bak, 'w', encoding='utf-8') as f:
        json.dump(data, f, ensure_ascii=False, indent=2)
    with open(MONSTERS_JSON, 'w', encoding='utf-8') as f:
        json.dump(data, f, ensure_ascii=False, indent=2)
    print(f"Done. Backup written to {bak}")

if __name__ == '__main__':
    main()
