import json
import os
from pathlib import Path

def find_missing_images():
    root = Path(r"d:\Projects\BazaarHelper")
    items_db_path = root / "src-tauri" / "resources" / "items_db.json"
    images_dir = root / "src-tauri" / "resources" / "images"

    if not items_db_path.exists():
        print(f"Error: {items_db_path} not found.")
        return

    with open(items_db_path, "r", encoding="utf-8") as f:
        items = json.load(f)

    # Get all filenames in images dir (without extension)
    # Most are .webp, but let's be safe
    existing_image_ids = {f.stem.lower() for f in images_dir.glob("*")}

    missing = []
    for item in items:
        item_id = item.get("id", "").lower()
        if not item_id:
            continue
            
        if item_id not in existing_image_ids:
            missing.append({
                "id": item.get("id"),
                "name_cn": item.get("name_cn", "未知"),
                "name_en": item.get("name_en", "Unknown")
            })

    print(f"Found {len(missing)} missing item images out of {len(items)} total items.\n")
    print("-" * 80)
    print(f"{'ID':<40} | {'中文名':<20} | {'英文名'}")
    print("-" * 80)
    for m in missing:
        print(f"{m['id']:<40} | {m['name_cn']:<20} | {m['name_en']}")
    print("-" * 80)

if __name__ == "__main__":
    find_missing_images()
