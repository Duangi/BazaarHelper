import json
import re
import os

ITEMS_DB_PATH = r"d:\Projects\BazaarHelper\src-tauri\resources\items_db.json"

def fix_seconds():
    print(f"Loading {ITEMS_DB_PATH}...")
    with open(ITEMS_DB_PATH, 'r', encoding='utf-8') as f:
        data = json.load(f)
    
    count = 0
    pattern = re.compile(r'(\d+)000秒')

    for item in data:
        if "skills" in item and isinstance(item["skills"], list):
            for skill in item["skills"]:
                if "cn" in skill and isinstance(skill["cn"], str):
                    original = skill["cn"]
                    # Replace 1000秒 -> 1秒, 20000秒 -> 20秒
                    new_text = pattern.sub(r'\1秒', original)
                    
                    if new_text != original:
                        print(f"Fixed: '{original}' -> '{new_text}'")
                        skill["cn"] = new_text
                        count += 1
                        
    print(f"Total replacements: {count}")

    if count > 0:
        print("Saving changes...")
        with open(ITEMS_DB_PATH, 'w', encoding='utf-8') as f:
            json.dump(data, f, ensure_ascii=False, indent=2)

if __name__ == "__main__":
    fix_seconds()
