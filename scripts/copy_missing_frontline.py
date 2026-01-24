import shutil
from pathlib import Path

ASSETS = Path(r"D:/TheBazaarData/Assets/Texture2D")
DEST = Path.cwd() / 'src-tauri' / 'resources' / 'images' / 'skill'
DEST.mkdir(parents=True, exist_ok=True)

candidates = [
    'Icon_Skill_VAN_FrontlineLogistics.png',
    'Icon_Skill_FrontlineLogistics.png',
]
found = []
for root in [ASSETS]:
    if not root.exists():
        continue
    for cand in candidates:
        for p in root.rglob(cand):
            dst = DEST / p.name
            shutil.copy2(p, dst)
            found.append(str(p))

if found:
    print('Copied:', found)
else:
    print('Not found any of candidates in', ASSETS)
