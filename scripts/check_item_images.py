import json
import os

def check_item_images():
    base_dir = os.path.abspath(os.path.dirname(__file__))
    project_root = os.path.dirname(base_dir)
    items_db_path = os.path.join(project_root, "src-tauri", "resources", "items_db.json")
    images_dir = os.path.join(project_root, "src-tauri", "resources", "images")

    if not os.path.exists(items_db_path):
        print(f"Error: items_db.json not found at {items_db_path}")
        return

    if not os.path.exists(images_dir):
        print(f"Error: images directory not found at {images_dir}")
        return

    with open(items_db_path, 'r', encoding='utf-8') as f:
        items = json.load(f)

    image_files = {f.lower(): f for f in os.listdir(images_dir)}
    
    missing_items = []
    found_by_id = 0
    found_by_name = 0

    extensions = ['.webp', '.png', '.jpg', '.jpeg']

    for item in items:
        item_id = item.get('id')
        name_en = item.get('name_en')
        name_cn = item.get('name_cn')
        
        found = False
        # Check by ID with multiple extensions
        for ext in extensions:
            id_filename = f"{item_id}{ext}".lower()
            if id_filename in image_files:
                found_by_id += 1
                found = True
                break
        
        if found:
            continue
            
        # Check by English Name
        if name_en:
            for ext in extensions:
                en_filename = f"{name_en}{ext}".lower()
                if en_filename in image_files:
                    found_by_name += 1
                    found = True
                    break
        
        if found:
            continue
            
        # Check by Chinese Name
        if name_cn:
            for ext in extensions:
                cn_filename = f"{name_cn}{ext}".lower()
                if cn_filename in image_files:
                    found_by_name += 1
                    found = True
                    break
            
        if not found:
            missing_items.append({
                "id": item_id,
                "name_en": name_en,
                "name_cn": name_cn
            })

    print(f"Total items in DB: {len(items)}")
    print(f"Images found by ID: {found_by_id}")
    print(f"Images found by Name: {found_by_name}")
    print(f"Missing images: {len(missing_items)}")

    if missing_items:
        print("\nTop 20 missing items:")
        for i, item in enumerate(missing_items[:20]):
            print(f"{i+1}. {item['name_cn']} ({item['name_en']}) - ID: {item['id']}")
            
        # Save full list to a file
        output_file = os.path.join(base_dir, "missing_item_images.json")
        with open(output_file, 'w', encoding='utf-8') as f:
            json.dump(missing_items, f, ensure_ascii=False, indent=2)
        print(f"\nFull list of missing items saved to: {output_file}")

if __name__ == "__main__":
    check_item_images()
