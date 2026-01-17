#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
将所有怪物图片重命名为中文名
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
IMAGES_BG_DIR = RESOURCES_DIR / "images_monster_bg"
IMAGES_CHAR_DIR = RESOURCES_DIR / "images_monster_char"

def sanitize_filename(name):
    """清理文件名中的非法字符"""
    # Windows 文件名非法字符
    invalid_chars = '<>:"/\\|?*'
    for char in invalid_chars:
        name = name.replace(char, '_')
    return name

def main():
    # 读取怪物数据库
    print(f"读取数据库: {MONSTERS_DB}")
    with open(MONSTERS_DB, 'r', encoding='utf-8') as f:
        monsters_db = json.load(f)
    
    print(f"共有 {len(monsters_db)} 个怪物条目")
    
    # 获取所有现有的图片文件
    bg_files = {f.name: f for f in IMAGES_BG_DIR.glob("*.webp")}
    char_files = {f.name: f for f in IMAGES_CHAR_DIR.glob("*.webp")}
    
    print(f"找到 {len(bg_files)} 个背景图, {len(char_files)} 个角色图")
    
    # 统计信息
    renamed_bg = 0
    renamed_char = 0
    not_found = []
    
    # 为每个怪物创建英文名到中文名的映射
    name_mapping = {}
    for chinese_name, monster_data in monsters_db.items():
        english_name = monster_data.get("name", "")
        if english_name:
            name_mapping[english_name] = chinese_name
            # 处理可能的下划线和空格变体
            name_mapping[english_name.replace(" ", "_")] = chinese_name
            name_mapping[english_name.replace("_", " ")] = chinese_name
    
    # 处理每个怪物
    for chinese_name, monster_data in monsters_db.items():
        # 清理中文名作为文件名
        safe_chinese_name = sanitize_filename(chinese_name)
        target_filename = f"{safe_chinese_name}.webp"
        
        # 尝试通过英文名查找现有文件
        english_name = monster_data.get("name", "")
        possible_names = [
            f"{english_name}.webp",
            f"{english_name.replace(' ', '_')}.webp",
            f"{english_name.replace('_', ' ')}.webp",
        ]
        
        # 查找匹配的现有文件
        found_bg = None
        found_char = None
        
        for possible_name in possible_names:
            if possible_name in bg_files:
                found_bg = possible_name
                break
        
        for possible_name in possible_names:
            if possible_name in char_files:
                found_char = possible_name
                break
        
        # 如果没找到，尝试通过 UUID 查找（monsters_export.json 中的映射）
        if not found_bg:
            # 检查是否有 UUID 格式的文件
            for filename in bg_files.keys():
                if filename.endswith('.webp') and len(filename) == 41:  # UUID.webp
                    # 这里需要额外的逻辑来匹配，暂时跳过
                    pass
        
        # 重命名背景图
        if found_bg and found_bg != target_filename:
            bg_old_path = IMAGES_BG_DIR / found_bg
            bg_new_path = IMAGES_BG_DIR / target_filename
            if not bg_new_path.exists():
                print(f"背景图: {found_bg} -> {target_filename}")
                shutil.move(str(bg_old_path), str(bg_new_path))
                renamed_bg += 1
        
        # 重命名角色图
        if found_char and found_char != target_filename:
            char_old_path = IMAGES_CHAR_DIR / found_char
            char_new_path = IMAGES_CHAR_DIR / target_filename
            if not char_new_path.exists():
                print(f"角色图: {found_char} -> {target_filename}")
                shutil.move(str(char_old_path), str(char_new_path))
                renamed_char += 1
        
        if not found_bg and not found_char:
            not_found.append((chinese_name, english_name))
        
        # 更新数据库中的路径（使用中文文件名）
        monster_data["image_path"] = f"images_monster/{target_filename}"
    
    # 保存更新后的数据库
    print(f"\n保存更新后的数据库...")
    with open(MONSTERS_DB, 'w', encoding='utf-8') as f:
        json.dump(monsters_db, f, ensure_ascii=False, indent=2)
    
    # 打印统计信息
    print(f"\n=== 重命名完成 ===")
    print(f"背景图重命名: {renamed_bg} 个")
    print(f"角色图重命名: {renamed_char} 个")
    
    if not_found:
        print(f"\n未找到图片的怪物 ({len(not_found)} 个):")
        for cn_name, en_name in not_found[:20]:  # 只显示前20个
            print(f"  {cn_name} ({en_name})")
        if len(not_found) > 20:
            print(f"  ... 还有 {len(not_found) - 20} 个")

if __name__ == "__main__":
    main()
