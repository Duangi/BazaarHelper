import json

with open('src-tauri/resources/monsters_db.json', encoding='utf-8') as f:
    data = json.load(f)

samples = list(data.items())[:10]
for k, v in samples:
    print(f"{k}: image_path={v.get('image_path', '无')}")
