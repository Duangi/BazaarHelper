import json
import re
import os

ITEMS_DB_PATH = r"d:\Projects\BazaarHelper\src-tauri\resources\items_db.json"

def fix_seconds_v2():
    print(f"Loading {ITEMS_DB_PATH}...")
    with open(ITEMS_DB_PATH, 'r', encoding='utf-8') as f:
        data = json.load(f)
    
    count = 0
    # Pattern explanation:
    # (\d+)000 : Match digits ending in 000, capture the prefix.
    # (?= ... ) : Lookahead assertion (don't consume).
    # (?:/[\d\.]+)* : Non-capturing group for zero or more slash-number sequences.
    # 秒 : Must end with this.
    # This finds 1000 in "1000/2秒" or "1000/2000秒" (before 2000 is processed).
    # Since regex replacement in Python processes strictly left-to-right, non-overlapping?
    # Actually re.sub might not handle overlapping lookaheads well if they are complex.
    # But since we are shrinking the string, repeated application might be needed or a smart regex.
    
    # Better approach: find the whole string "X/Y/Z秒" and process matches within it.
    
    # Regex to find time strings:  ((\d+000)(?:/\d+)*秒) is too simple.
    # (\d+(?:/\d+)+秒) finds slash separated groups ending in 秒.
    
    search_pattern = re.compile(r'(\d+(?:/[\d\.]+)*秒)')
    
    for item in data:
        if "skills" in item and isinstance(item["skills"], list):
            for skill in item["skills"]:
                if "cn" in skill and isinstance(skill["cn"], str):
                    original = skill["cn"]
                    
                    def repl(match):
                        full_str = match.group(1) # e.g. "1000/2000秒" or "1000/2秒"
                        # Remove '秒' temporarily
                        val_str = full_str.replace('秒', '')
                        parts = val_str.split('/')
                        new_parts = []
                        changed = False
                        for p in parts:
                            if p.endswith('000') and p.isdigit(): # Strictly digits ending in 000
                                new_parts.append(p[:-3]) # Remove last 3 chars
                                changed = True
                            else:
                                new_parts.append(p)
                        
                        if changed:
                            return '/'.join(new_parts) + '秒'
                        return full_str

                    new_text = search_pattern.sub(repl, original)
                    
                    if new_text != original:
                        print(f"Refining: '{original}' -> '{new_text}'")
                        skill["cn"] = new_text
                        count += 1
                        
    print(f"Total refined replacements: {count}")

    if count > 0:
        print("Saving changes...")
        with open(ITEMS_DB_PATH, 'w', encoding='utf-8') as f:
            json.dump(data, f, ensure_ascii=False, indent=2)

if __name__ == "__main__":
    fix_seconds_v2()
