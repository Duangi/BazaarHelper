from PIL import Image
from pathlib import Path
import sys

SRC = Path.cwd() / 'src-tauri' / 'resources' / 'images' / 'skill'
if not SRC.exists():
    print('Source dir not found:', SRC)
    sys.exit(1)

count = 0
converted = 0
errors = []
for p in SRC.glob('*.png'):
    count += 1
    out = p.with_suffix('.webp')
    try:
        im = Image.open(p).convert('RGBA')
        im.save(out, 'WEBP', quality=80, method=6)
        converted += 1
    except Exception as e:
        errors.append((str(p), str(e)))

print('PNG count:', count)
print('Converted:', converted)
print('Errors:', len(errors))
if errors:
    for e in errors:
        print(e)
