#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
使用 monsters_db.json 中的数据重命名所有剩余的英文文件
通过智能匹配和手动映射
"""

import json
import os
from pathlib import Path
import shutil

# 定义路径
SCRIPT_DIR = Path(__file__).parent
PROJECT_ROOT = SCRIPT_DIR.parent
RESOURCES_DIR = PROJECT_ROOT / "src-tauri" / "resources"
MONSTERS_DB = RESOURCES_DIR / "monsters_db.json"
MONSTERS_EXPORT = RESOURCES_DIR / "monsters_export.json"
IMAGES_BG_DIR = RESOURCES_DIR / "images_monster_bg"
IMAGES_CHAR_DIR = RESOURCES_DIR / "images_monster_char"

def sanitize_filename(name):
    """清理文件名中的非法字符"""
    invalid_chars = '<>:"/\\|?*'
    for char in invalid_chars:
        name = name.replace(char, '_')
    return name

def main():
    # 读取怪物数据库
    with open(MONSTERS_DB, 'r', encoding='utf-8') as f:
        monsters_db = json.load(f)
    
    # 读取导出数据
    with open(MONSTERS_EXPORT, 'r', encoding='utf-8') as f:
        monsters_export = json.load(f)
    
    # 建立完整的映射：英文名 -> 中文名
    en_to_cn = {}
    
    # 从 monsters_db 添加
    for cn_name, data in monsters_db.items():
        en_name = data.get('name', '')
        if en_name:
            en_to_cn[en_name] = cn_name
            en_to_cn[en_name.replace(' ', '_')] = cn_name
            en_to_cn[en_name.replace('_', ' ')] = cn_name
    
    # 从 monsters_export 添加（会覆盖不一致的）
    for monster in monsters_export:
        en_name = monster.get('name_en', '')
        cn_name = monster.get('name_cn', '')
        if en_name and cn_name:
            en_to_cn[en_name] = cn_name
            en_to_cn[en_name.replace(' ', '_')] = cn_name
            en_to_cn[en_name.replace('_', ' ')] = cn_name
    
    print(f"建立了包含 {len(en_to_cn)} 个条目的映射")
    
    # 获取所有英文文件
    all_files = {}
    for file in IMAGES_CHAR_DIR.glob("*.webp"):
        if not any(ord(c) > 127 for c in file.stem):  # 只处理英文文件
            all_files[file.stem] = file
    
    print(f"\n找到 {len(all_files)} 个英文文件名:")
    for name in sorted(all_files.keys()):
        print(f"  {name}")
    
    # 处理重命名
    print(f"\n开始重命名...")
    renamed = 0
    not_found = []
    
    for en_name, file_path in all_files.items():
        # 尝试各种变体
        possible_keys = [
            en_name,
            en_name.replace('_', ' '),
            en_name.replace('__', ': '),  # Dr__Vortex -> Dr: Vortex
        ]
        
        cn_name = None
        for key in possible_keys:
            if key in en_to_cn:
                cn_name = en_to_cn[key]
                break
        
        if cn_name:
            safe_cn = sanitize_filename(cn_name)
            
            # 重命名背景图
            bg_old = IMAGES_BG_DIR / f"{en_name}.webp"
            bg_new = IMAGES_BG_DIR / f"{safe_cn}.webp"
            if bg_old.exists() and not bg_new.exists():
                print(f"背景: {en_name} -> {safe_cn}")
                shutil.move(str(bg_old), str(bg_new))
                renamed += 1
            
            # 重命名角色图
            char_old = IMAGES_CHAR_DIR / f"{en_name}.webp"
            char_new = IMAGES_CHAR_DIR / f"{safe_cn}.webp"
            if char_old.exists() and not char_new.exists():
                print(f"角色: {en_name} -> {safe_cn}")
                shutil.move(str(char_old), str(char_new))
                renamed += 1
        else:
            not_found.append(en_name)
    
    print(f"\n=== 完成 ===")
    print(f"重命名: {renamed} 个文件")
    
    if not_found:
        print(f"\n未找到映射 ({len(not_found)} 个):")
        for name in sorted(not_found):
            print(f"  {name}")
        
        print(f"\n请为这些文件手动查找中文名...")

if __name__ == "__main__":
    main()
