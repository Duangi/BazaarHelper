import json
import os
from pathlib import Path
import shutil

WORK = Path.cwd()
SKILLS = WORK / 'src-tauri' / 'resources' / 'skills_db.json'
ASSETS_ROOT = Path('D:/TheBazaarData/Assets/Texture2D')
DEST = WORK / 'src-tauri' / 'resources' / 'images' / 'skill'

EXT_TRY = ['.png', '.PNG', '.tga', '.TGA', '.dds', '.DDS', '.jpg', '.jpeg', '.JPG', '.JPEG']

if not SKILLS.exists():
    print('skills_db.json not found:', SKILLS)
    raise SystemExit(1)

with SKILLS.open('r', encoding='utf-8') as f:
    skills = json.load(f)

# collect art_key values
art_keys = []
for entry in skills:
    ak = entry.get('art_key')
    if ak and isinstance(ak, str) and ak.strip():
        art_keys.append(ak.strip())

art_keys = list(dict.fromkeys(art_keys))
print('Unique art_keys count:', len(art_keys))

# build filename -> path map by walking ASSETS_ROOT
filename_map = {}
if ASSETS_ROOT.exists():
    for root, dirs, files in os.walk(ASSETS_ROOT):
        for fn in files:
            filename_map.setdefault(fn.lower(), []).append(Path(root) / fn)
else:
    print('Assets root not found:', ASSETS_ROOT)

# ensure dest
DEST.mkdir(parents=True, exist_ok=True)

found = []
not_found = []

for ak in art_keys:
    base = os.path.basename(ak)
    base_lower = base.lower()
    matched_paths = filename_map.get(base_lower)
    if matched_paths:
        # choose the first
        src = matched_paths[0]
        dst = DEST / src.name
        try:
            shutil.copy2(src, dst)
            found.append((ak, str(src)))
        except Exception as e:
            print('Failed copy', src, e)
            not_found.append((ak, 'copy_failed'))
        continue

    # if base has no extension, try adding common extensions
    name_no_ext, ext = os.path.splitext(base)
    if ext == '':
        matched = None
        for ext_try in EXT_TRY:
            cand = (name_no_ext + ext_try).lower()
            if cand in filename_map:
                matched = filename_map[cand][0]
                break
        if matched:
            try:
                shutil.copy2(matched, DEST / matched.name)
                found.append((ak, str(matched)))
                continue
            except Exception as e:
                print('Failed copy', matched, e)
                not_found.append((ak, 'copy_failed'))
                continue

    # fallback: fuzzy contains search in filename_map keys
    found_candidate = None
    for fn_lower, paths in filename_map.items():
        if name_no_ext.lower() in fn_lower:
            found_candidate = paths[0]
            break
    if found_candidate:
        try:
            shutil.copy2(found_candidate, DEST / found_candidate.name)
            found.append((ak, str(found_candidate)))
            continue
        except Exception as e:
            print('Failed copy', found_candidate, e)
            not_found.append((ak, 'copy_failed'))
            continue

    # last resort: search whole tree for file name substring (costly)
    searched = False
    for root, dirs, files in os.walk(ASSETS_ROOT):
        for fn in files:
            if name_no_ext.lower() in fn.lower():
                src = Path(root) / fn
                try:
                    shutil.copy2(src, DEST / src.name)
                    found.append((ak, str(src)))
                except Exception as e:
                    print('Failed copy', src, e)
                    not_found.append((ak, 'copy_failed'))
                searched = True
                break
        if searched:
            break
    if not searched:
        not_found.append((ak, 'not_found'))

print('\nSummary:')
print('Total art_keys:', len(art_keys))
print('Found:', len(found))
print('Not found:', len(not_found))
if not_found:
    print('\nMissing list:')
    for ak, reason in not_found:
        print(ak, reason)

# Optionally write a report
report = {
    'total': len(art_keys),
    'found_count': len(found),
    'not_found_count': len(not_found),
    'found': found,
    'not_found': not_found
}
with (WORK / 'scripts' / 'copy_skill_images_report.json').open('w', encoding='utf-8') as rf:
    json.dump(report, rf, ensure_ascii=False, indent=2)

print('\nReport written to scripts/copy_skill_images_report.json')
