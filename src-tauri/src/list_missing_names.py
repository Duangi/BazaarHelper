import json
from pathlib import Path

root = Path(__file__).resolve().parents[1]
enc_path = root / 'resources' / 'combat_encounters.json'
out_path = Path(__file__).resolve().parent / 'missing_namezh.json'

with open(enc_path, 'r', encoding='utf-8') as f:
    data = json.load(f)

missing = []
for key, obj in data.items():
    img = obj.get('image') if isinstance(obj, dict) else None
    if not img:
        namezh = obj.get('name_zh') if isinstance(obj, dict) else ''
        missing.append({'key': key, 'name_zh': namezh})

with open(out_path, 'w', encoding='utf-8') as f:
    json.dump(missing, f, ensure_ascii=False, indent=2)

print(f'WROTE {out_path}')
for e in missing:
    print(e.get('name_zh') or '(empty name_zh)')
