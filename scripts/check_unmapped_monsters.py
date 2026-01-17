#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
列出未改名的怪物及其信息
"""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EXPORT_FILE = ROOT / 'src-tauri' / 'resources' / 'monsters_export.json'
MONSTERS_DB = ROOT / 'src-tauri' / 'resources' / 'monsters_db.json'
BG_DIR = ROOT / 'src-tauri' / 'resources' / 'images_monster_bg'

# 加载数据
with open(EXPORT_FILE, 'r', encoding='utf-8') as f:
    export_data = json.load(f)

with open(MONSTERS_DB, 'r', encoding='utf-8') as f:
    monsters_db = json.load(f)

# 从export构建映射
export_names = set()
for monster in export_data:
    en_name = monster.get('name_en', '').strip()
    if en_name:
        # 添加各种可能的文件名格式
        export_names.add(en_name)
        export_names.add(en_name.replace(' ', '_'))
        export_names.add(en_name.replace('.', '_'))

# 找出未改名的文件
unmapped = []
for img_file in BG_DIR.glob('*.webp'):
    # 检查是否是UUID格式
    if not (len(img_file.stem) > 36 and img_file.stem[36:] == '_bg'):
        file_name = img_file.stem[:-3]  # 去掉_bg
        
        # 查找对应的怪物信息
        info = None
        for key, value in monsters_db.items():
            if isinstance(value, dict):
                db_name = value.get('name', '')
                if db_name:
                    if db_name == file_name or db_name.replace(' ', '_') == file_name:
                        info = value
                        break
        
        unmapped.append({
            'file': file_name,
            'in_export': file_name in export_names or file_name.replace('_', ' ') in export_names,
            'db_info': info
        })

print(f"发现 {len(unmapped)} 个未改名的文件\n")
print("=" * 80)

for item in unmapped:
    file_name = item['file']
    in_export = item['in_export']
    db_info = item['db_info']
    
    print(f"文件名: {file_name}")
    print(f"  在 export.json: {'是' if in_export else '否'}")
    
    if db_info:
        cn_name = db_info.get('name_zh', '未知')
        has_id = 'id' in db_info
        print(f"  中文名: {cn_name}")
        print(f"  有ID: {'是' if has_id else '否'}")
        if has_id:
            print(f"  ID: {db_info['id']}")
    else:
        print(f"  monsters_db: 未找到")
    print()
