#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
从 monsters_export.json 同步 ID 到 monsters_db.json，并重命名图片文件
"""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EXPORT_FILE = ROOT / 'src-tauri' / 'resources' / 'monsters_export.json'
MONSTERS_DB = ROOT / 'src-tauri' / 'resources' / 'monsters_db.json'
BG_DIR = ROOT / 'src-tauri' / 'resources' / 'images_monster_bg'
CHAR_DIR = ROOT / 'src-tauri' / 'resources' / 'images_monster_char'

def main():
    print("=" * 60)
    print("同步怪物 ID 并重命名图片")
    print("=" * 60)
    
    # 1. 加载 monsters_export.json
    print(f"\n加载 {EXPORT_FILE.name}")
    with open(EXPORT_FILE, 'r', encoding='utf-8') as f:
        export_data = json.load(f)
    
    # 2. 构建映射: 中文名 -> {id, name_en}
    cn_to_info = {}
    for monster in export_data:
        cn_name = monster.get('name_cn', '').strip()
        monster_id = monster.get('id', '').strip()
        en_name = monster.get('name_en', '').strip()
        
        if cn_name and monster_id:
            cn_to_info[cn_name] = {
                'id': monster_id,
                'name_en': en_name
            }
    
    print(f"从 export 文件中提取了 {len(cn_to_info)} 个怪物映射")
    
    # 3. 加载 monsters_db.json
    print(f"\n加载 {MONSTERS_DB.name}")
    with open(MONSTERS_DB, 'r', encoding='utf-8') as f:
        monsters_db = json.load(f)
    
    # 4. 更新 monsters_db，添加 id 和英文名
    print("\n更新 monsters_db...")
    updated_count = 0
    for key, value in monsters_db.items():
        if isinstance(value, dict):
            cn_name = value.get('name_zh', '')
            if cn_name in cn_to_info:
                info = cn_to_info[cn_name]
                value['id'] = info['id']
                if 'name' not in value or not value['name']:
                    value['name'] = info['name_en']
                updated_count += 1
    
    print(f"  更新了 {updated_count} 个怪物条目")
    
    # 5. 保存更新后的 monsters_db.json
    print(f"\n保存 {MONSTERS_DB.name}")
    with open(MONSTERS_DB, 'w', encoding='utf-8') as f:
        json.dump(monsters_db, f, ensure_ascii=False, indent=2)
    
    # 6. 构建英文名到ID的映射（用于重命名图片）
    en_to_id = {}
    for cn_name, info in cn_to_info.items():
        en_name = info['name_en']
        monster_id = info['id']
        if en_name:
            # 处理文件名中的特殊字符（替换为下划线）
            safe_en_name = en_name.replace(' ', '_').replace('.', '_').replace('/', '_')
            en_to_id[safe_en_name] = monster_id
            # 同时添加原始名字（不带下划线），以防万一
            en_to_id[en_name] = monster_id
    
    # 7. 重命名背景图
    print(f"\n重命名背景图...")
    bg_renamed = 0
    for img_file in BG_DIR.glob('*.webp'):
        file_stem = img_file.stem
        if file_stem.endswith('_bg'):
            en_name = file_stem[:-3]  # 去掉 '_bg'
            monster_id = en_to_id.get(en_name)
            
            if monster_id:
                new_path = BG_DIR / f"{monster_id}_bg.webp"
                img_file.rename(new_path)
                bg_renamed += 1
                print(f"  ✓ {img_file.name} -> {new_path.name}")
    
    # 8. 重命名角色图
    print(f"\n重命名角色图...")
    char_renamed = 0
    for img_file in CHAR_DIR.glob('*.webp'):
        file_stem = img_file.stem
        if file_stem.endswith('_char'):
            en_name = file_stem[:-5]  # 去掉 '_char'
            monster_id = en_to_id.get(en_name)
            
            if monster_id:
                new_path = CHAR_DIR / f"{monster_id}_char.webp"
                img_file.rename(new_path)
                char_renamed += 1
                print(f"  ✓ {img_file.name} -> {new_path.name}")
    
    # 汇总结果
    print("\n" + "=" * 60)
    print("完成！")
    print("=" * 60)
    print(f"monsters_db 更新: {updated_count} 个条目")
    print(f"背景图重命名: {bg_renamed} 张")
    print(f"角色图重命名: {char_renamed} 张")
    print("=" * 60)

if __name__ == '__main__':
    main()
