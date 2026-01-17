#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
使用 monsters_db.json 中已有的ID重命名剩余图片
"""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MONSTERS_DB = ROOT / 'src-tauri' / 'resources' / 'monsters_db.json'
BG_DIR = ROOT / 'src-tauri' / 'resources' / 'images_monster_bg'
CHAR_DIR = ROOT / 'src-tauri' / 'resources' / 'images_monster_char'

def main():
    print("=" * 60)
    print("使用 monsters_db.json 中的 ID 重命名剩余图片")
    print("=" * 60)
    
    # 加载 monsters_db.json
    with open(MONSTERS_DB, 'r', encoding='utf-8') as f:
        monsters_db = json.load(f)
    
    # 构建 英文名 -> ID 的映射
    en_to_id = {}
    for key, value in monsters_db.items():
        if isinstance(value, dict) and 'id' in value:
            monster_id = value.get('id', '').strip()
            en_name = value.get('name', '').strip()
            
            if monster_id and en_name:
                # 添加多种格式
                en_to_id[en_name] = monster_id
                en_to_id[en_name.replace(' ', '_')] = monster_id
                en_to_id[en_name.replace('.', '_')] = monster_id
                en_to_id[en_name.replace(' ', '_').replace('.', '_')] = monster_id
    
    print(f"找到 {len(set(en_to_id.values()))} 个有ID的怪物\n")
    
    # 重命名背景图
    print("重命名背景图...")
    bg_renamed = 0
    for img_file in BG_DIR.glob('*.webp'):
        # 跳过已经是UUID格式的
        if len(img_file.stem) > 36 and img_file.stem[36:] == '_bg':
            continue
        
        file_stem = img_file.stem
        if file_stem.endswith('_bg'):
            en_name = file_stem[:-3]
            monster_id = en_to_id.get(en_name)
            
            if monster_id:
                new_path = BG_DIR / f"{monster_id}_bg.webp"
                if not new_path.exists():
                    img_file.rename(new_path)
                    bg_renamed += 1
                    print(f"  ✓ {img_file.name} -> {new_path.name}")
            else:
                print(f"  ✗ 未找到ID: {en_name}")
    
    # 重命名角色图
    print(f"\n重命名角色图...")
    char_renamed = 0
    for img_file in CHAR_DIR.glob('*.webp'):
        # 跳过已经是UUID格式的
        if len(img_file.stem) > 36 and img_file.stem[36:] == '_char':
            continue
        
        file_stem = img_file.stem
        if file_stem.endswith('_char'):
            en_name = file_stem[:-5]
            monster_id = en_to_id.get(en_name)
            
            if monster_id:
                new_path = CHAR_DIR / f"{monster_id}_char.webp"
                if not new_path.exists():
                    img_file.rename(new_path)
                    char_renamed += 1
                    print(f"  ✓ {img_file.name} -> {new_path.name}")
            else:
                print(f"  ✗ 未找到ID: {en_name}")
    
    print("\n" + "=" * 60)
    print("完成！")
    print("=" * 60)
    print(f"背景图重命名: {bg_renamed} 张")
    print(f"角色图重命名: {char_renamed} 张")
    print("=" * 60)

if __name__ == '__main__':
    main()
