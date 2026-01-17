#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
使用 monsters_export.json 的映射来重命名 UUID 格式的图片文件
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
    print(f"读取 monsters_db.json...")
    with open(MONSTERS_DB, 'r', encoding='utf-8') as f:
        monsters_db = json.load(f)
    
    # 读取导出数据
    print(f"读取 monsters_export.json...")
    with open(MONSTERS_EXPORT, 'r', encoding='utf-8') as f:
        monsters_export = json.load(f)
    
    # 创建 UUID -> 中文名的映射
    uuid_to_chinese = {}
    for monster in monsters_export:
        uuid = monster.get("id", "")
        name_cn = monster.get("name_cn", "")
        if uuid and name_cn:
            uuid_to_chinese[uuid] = name_cn
    
    print(f"找到 {len(uuid_to_chinese)} 个 UUID 映射")
    
    # 获取所有现有的图片文件
    bg_files = list(IMAGES_BG_DIR.glob("*.webp"))
    char_files = list(IMAGES_CHAR_DIR.glob("*.webp"))
    
    print(f"找到 {len(bg_files)} 个背景图, {len(char_files)} 个角色图")
    
    renamed_bg = 0
    renamed_char = 0
    
    # 处理背景图
    for bg_file in bg_files:
        filename = bg_file.stem  # 不含扩展名的文件名
        
        # 检查是否是 UUID 格式（36个字符，包含连字符）
        if len(filename) == 36 and filename.count('-') == 4:
            # 在映射中查找
            if filename in uuid_to_chinese:
                chinese_name = uuid_to_chinese[filename]
                safe_chinese_name = sanitize_filename(chinese_name)
                new_filename = f"{safe_chinese_name}.webp"
                new_path = IMAGES_BG_DIR / new_filename
                
                if not new_path.exists():
                    print(f"背景图: {filename}.webp -> {new_filename}")
                    shutil.move(str(bg_file), str(new_path))
                    renamed_bg += 1
                else:
                    print(f"跳过（目标已存在）: {new_filename}")
    
    # 处理角色图
    for char_file in char_files:
        filename = char_file.stem
        
        if len(filename) == 36 and filename.count('-') == 4:
            if filename in uuid_to_chinese:
                chinese_name = uuid_to_chinese[filename]
                safe_chinese_name = sanitize_filename(chinese_name)
                new_filename = f"{safe_chinese_name}.webp"
                new_path = IMAGES_CHAR_DIR / new_filename
                
                if not new_path.exists():
                    print(f"角色图: {filename}.webp -> {new_filename}")
                    shutil.move(str(char_file), str(new_path))
                    renamed_char += 1
                else:
                    print(f"跳过（目标已存在）: {new_filename}")
    
    print(f"\n=== UUID 重命名完成 ===")
    print(f"背景图重命名: {renamed_bg} 个")
    print(f"角色图重命名: {renamed_char} 个")
    
    # 检查还剩多少 UUID 文件
    remaining_bg_uuids = [f for f in IMAGES_BG_DIR.glob("*.webp") 
                          if len(f.stem) == 36 and f.stem.count('-') == 4]
    remaining_char_uuids = [f for f in IMAGES_CHAR_DIR.glob("*.webp") 
                            if len(f.stem) == 36 and f.stem.count('-') == 4]
    
    print(f"\n剩余 UUID 格式文件:")
    print(f"  背景图: {len(remaining_bg_uuids)} 个")
    print(f"  角色图: {len(remaining_char_uuids)} 个")
    
    if remaining_bg_uuids:
        print(f"\n未映射的 UUID (背景图前10个):")
        for f in remaining_bg_uuids[:10]:
            print(f"  {f.stem}")

if __name__ == "__main__":
    main()
