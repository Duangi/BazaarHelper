import json
import os
import urllib.request
from pathlib import Path

def download_missing_images():
    root = Path(r"d:\Projects\BazaarHelper")
    items_db_path = root / "src-tauri" / "resources" / "items_db.json"
    images_dir = root / "src-tauri" / "resources" / "images"
    missing_json_path = root / "scripts" / "missing_item_images.json"

    # Use the existing missing items list if available, otherwise find them again
    if missing_json_path.exists():
        with open(missing_json_path, "r", encoding="utf-8") as f:
            missing_items = json.load(f)
    else:
        # Re-scan logic if missing_item_images.json is gone
        if not items_db_path.exists():
            print(f"Error: {items_db_path} not found.")
            return
        with open(items_db_path, "r", encoding="utf-8") as f:
            items = json.load(f)
        existing_ids = {f.stem.lower() for f in images_dir.glob("*")}
        missing_items = [
            {"id": item["id"], "name_cn": item.get("name_cn", "未知"), "name_en": item.get("name_en", "Unknown")}
            for item in items if item.get("id", "").lower() not in existing_ids
        ]

    if not missing_items:
        print("No missing images found! Everything is up to date.")
        return

    print(f"Total missing images: {len(missing_items)}")
    print("For each item, paste the image URL. Press Enter to skip, or 'q' to quit.\n")

    for i, item in enumerate(missing_items):
        name_cn = item['name_cn']
        name_en = item['name_en']
        item_id = item['id']
        
        print(f"[{i+1}/{len(missing_items)}] Item: {name_cn} / {name_en}")
        print(f"ID: {item_id}")
        url = input("Paste URL: ").strip()

        if url.lower() == 'q':
            print("Quitting...")
            break
        
        if not url:
            print("Skipped.\n")
            continue

        try:
            target_path = images_dir / f"{item_id}.webp"
            
            # Basic headers to avoid some simple blocks
            headers = {
                'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36'
            }
            req = urllib.request.Request(url, headers=headers)
            
            with urllib.request.urlopen(req) as response:
                with open(target_path, 'wb') as out_file:
                    out_file.write(response.read())
            
            print(f"Successfully downloaded and saved to {target_path.name}\n")
        except Exception as e:
            print(f"Error downloading from {url}: {e}\n")

    print("Finished.")

if __name__ == "__main__":
    download_missing_images()
