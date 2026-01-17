#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
根据 monsters_db.json 中的 ID 重命名怪物图片
"""
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MONSTERS_DB = ROOT / 'src-tauri' / 'resources' / 'monsters_db.json'
BG_DIR = ROOT / 'src-tauri' / 'resources' / 'images_monster_bg'
CHAR_DIR = ROOT / 'src-tauri' / 'resources' / 'images_monster_char'

def load_monsters_db():
    """加载怪物数据库"""
    with open(MONSTERS_DB, 'r', encoding='utf-8') as f:
        return json.load(f)

def build_name_to_id_map(monsters_db):
    """构建英文名到ID的映射"""
    name_to_id = {}
    for monster_id, data in monsters_db.items():
        if isinstance(data, dict):
            # 尝试从多个字段获取英文名
            eng_name = data.get('name', '') or data.get('name_zh', '')
            if eng_name:
                name_to_id[eng_name] = monster_id
    return name_to_id

def rename_images_in_dir(directory, name_to_id, suffix):
    """重命名指定目录中的图片文件
    
    Args:
        directory: 目录路径
        name_to_id: 英文名到ID的映射字典
        suffix: 文件后缀（如 '_bg' 或 '_char'）
    """
    if not directory.exists():
        print(f"目录不存在: {directory}")
        return
    
    renamed_count = 0
    not_found_count = 0
    
    for img_file in directory.glob('*.webp'):
        # 提取英文名（去掉后缀）
        # 例如: "Banannibal_bg.webp" -> "Banannibal"
        file_stem = img_file.stem
        if file_stem.endswith(suffix):
            eng_name = file_stem[:-len(suffix)]
        else:
            eng_name = file_stem
        
        # 查找对应的ID
        monster_id = name_to_id.get(eng_name)
        
        if monster_id:
            # 新文件名: id + suffix + .webp
            new_name = f"{monster_id}{suffix}.webp"
            new_path = directory / new_name
            
            # 重命名
            img_file.rename(new_path)
            renamed_count += 1
            print(f"  ✓ {img_file.name} -> {new_name}")
        else:
            not_found_count += 1
            print(f"  ✗ 未找到ID: {eng_name} ({img_file.name})")
    
    return renamed_count, not_found_count

def main():
    print("=" * 60)
    print("根据 ID 重命名怪物图片")
    print("=" * 60)
    
    # 加载数据库
    print(f"\n加载数据库: {MONSTERS_DB}")
    monsters_db = load_monsters_db()
    name_to_id = build_name_to_id_map(monsters_db)
    print(f"已加载 {len(name_to_id)} 个怪物名称映射")
    
    # 重命名背景图
    print(f"\n处理背景图: {BG_DIR}")
    bg_renamed, bg_not_found = rename_images_in_dir(BG_DIR, name_to_id, '_bg')
    
    # 重命名角色图
    print(f"\n处理角色图: {CHAR_DIR}")
    char_renamed, char_not_found = rename_images_in_dir(CHAR_DIR, name_to_id, '_char')
    
    # 汇总结果
    print("\n" + "=" * 60)
    print("重命名完成")
    print("=" * 60)
    print(f"背景图: {bg_renamed} 张成功, {bg_not_found} 张未找到ID")
    print(f"角色图: {char_renamed} 张成功, {char_not_found} 张未找到ID")
    print(f"总计: {bg_renamed + char_renamed} 张成功")
    print("=" * 60)

if __name__ == '__main__':
    main()
