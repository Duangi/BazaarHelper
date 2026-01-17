#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
从 monsters_export.json 创建完整的英文名到中文名映射
"""

import json
import os
from pathlib import Path
import shutil

# 定义路径
SCRIPT_DIR = Path(__file__).parent
PROJECT_ROOT = SCRIPT_DIR.parent
RESOURCES_DIR = PROJECT_ROOT / "src-tauri" / "resources"
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
    # 读取导出数据
    print(f"读取 monsters_export.json...")
    with open(MONSTERS_EXPORT, 'r', encoding='utf-8') as f:
        monsters_export = json.load(f)
    
    # 创建英文名 -> 中文名的映射
    en_to_cn = {}
    for monster in monsters_export:
        name_en = monster.get("name_en", "")
        name_cn = monster.get("name_cn", "")
        if name_en and name_cn:
            # 添加多种变体
            en_to_cn[name_en] = name_cn
            en_to_cn[name_en.replace(" ", "_")] = name_cn
            en_to_cn[name_en.replace("_", " ")] = name_cn
    
    print(f"创建了 {len(en_to_cn)} 个英文名映射")
    
    # 获取所有现有的英文名文件
    bg_files = list(IMAGES_BG_DIR.glob("*.webp"))
    char_files = list(IMAGES_CHAR_DIR.glob("*.webp"))
    
    print(f"找到 {len(bg_files)} 个背景图, {len(char_files)} 个角色图")
    
    renamed_bg = 0
    renamed_char = 0
    not_found = []
    
    # 处理背景图
    for bg_file in bg_files:
        filename_without_ext = bg_file.stem
        
        # 跳过已经是中文的文件
        if any(ord(c) > 127 for c in filename_without_ext):
            continue
        
        # 在映射中查找
        chinese_name = None
        if filename_without_ext in en_to_cn:
            chinese_name = en_to_cn[filename_without_ext]
        
        if chinese_name:
            safe_chinese_name = sanitize_filename(chinese_name)
            new_filename = f"{safe_chinese_name}.webp"
            new_path = IMAGES_BG_DIR / new_filename
            
            if not new_path.exists():
                print(f"背景图: {bg_file.name} -> {new_filename}")
                shutil.move(str(bg_file), str(new_path))
                renamed_bg += 1
            else:
                print(f"跳过（目标已存在）: {new_filename}")
        else:
            not_found.append(filename_without_ext)
    
    # 处理角色图
    for char_file in char_files:
        filename_without_ext = char_file.stem
        
        # 跳过已经是中文的文件
        if any(ord(c) > 127 for c in filename_without_ext):
            continue
        
        # 在映射中查找
        chinese_name = None
        if filename_without_ext in en_to_cn:
            chinese_name = en_to_cn[filename_without_ext]
        
        if chinese_name:
            safe_chinese_name = sanitize_filename(chinese_name)
            new_filename = f"{safe_chinese_name}.webp"
            new_path = IMAGES_CHAR_DIR / new_filename
            
            if not new_path.exists():
                print(f"角色图: {char_file.name} -> {new_filename}")
                shutil.move(str(char_file), str(new_path))
                renamed_char += 1
            else:
                print(f"跳过（目标已存在）: {new_filename}")
    
    print(f"\n=== 英文名重命名完成 ===")
    print(f"背景图重命名: {renamed_bg} 个")
    print(f"角色图重命名: {renamed_char} 个")
    
    if not_found:
        print(f"\n未找到映射的英文名 ({len(set(not_found))} 个):")
        for name in sorted(set(not_found))[:20]:
            print(f"  {name}")

if __name__ == "__main__":
    main()
