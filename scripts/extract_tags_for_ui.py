import json
import os

def extract_tags():
    tags_set = set()
    hidden_tags_set = set()

    # Use absolute paths or relative to cwd which is D:\Projects\BazaarHelper
    files = [
        r'd:\Projects\BazaarHelper\src-tauri\resources\items_db.json', 
        r'd:\Projects\BazaarHelper\src-tauri\resources\skills_db.json'
    ]
    
    for fpath in files:
        if not os.path.exists(fpath):
            print(f"File not found: {fpath}")
            continue
            
        with open(fpath, 'r', encoding='utf-8') as f:
            data = json.load(f)
            
        # Check if data is a list (based on file read) or a dict with 'list' key
        if isinstance(data, list):
            items = data
        else:
            items = data.get('list', [])

        for item in items:
            # Stats tags
            t_str = item.get('tags', '')
            if t_str:
                parts = t_str.split('|')
                for p in parts:
                    clean = p.split(" / ")[-1].strip()
                    if clean and "Hide" not in clean and "Hidden" not in clean and "隐藏" not in clean:
                        tags_set.add(clean)

            # Hidden tags
            ht_str = item.get('hidden_tags', '')
            if ht_str:
                # Based on usage, hidden tags might be comma-separated or space-separated?
                # Let's split by space just in case, and also handle commas if present.
                parts = ht_str.replace(',', ' ').split()
                for p in parts:
                    if p.strip():
                        hidden_tags_set.add(p.strip())

    sorted_tags = sorted(list(tags_set))
    sorted_hidden = sorted(list(hidden_tags_set))
    
    print("ALL TAGS JSON:")
    print(json.dumps(sorted_tags, ensure_ascii=False))
    print("\nALL HIDDEN TAGS JSON:")
    print(json.dumps(sorted_hidden, ensure_ascii=False))

if __name__ == "__main__":
    extract_tags()
