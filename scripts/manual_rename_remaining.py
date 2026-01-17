#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
手动输入英文名对应的中文名来重命名剩余文件
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

# 需要手动映射的英文文件名
REMAINING_FILES = [
    "Ahexa",
    "Aleppa_Tantius",
    "Annex_Trooper",
    "Banannibal",
    "Blowguns_Trap",
    "Boarrior",
    "Boulder_Trap",
    "Bouncertron",
    "Burninator_Bot",
    "Caltrops_Trap",
    "Car_Conductor",
    "Cloudtop_Admiral",
    "Dooltron",
    "Dr__Vortex",
    "Eight_Arm_Davvy",
    "Foundation_Revenant",
    "Foundation_Weeper",
    "Gibbus",
    "Grandfather_Klok",
    "Greenheart_Guardian",
    "Harkuvian_Rocket_Trooper",
    "Haunted_Kimono",
    "Hellbilly",
    "Hoverbike_Hooligan",
    "Infernal_Dreadnought",
    "Kyver_Commander",
    "Lich",
    "Mimic",
    "Morguloth",
    "Mythkeeper",
    "Property_Baron",
    "Pyro",
    "Qomatz",
    "Scovyle",
    "Stew",
    "Street_Gamer",
    "Tempest_Bravo",
    "Terrorform",
    "Tortuga",
    "Trashtown_Mayor",
    "Void_Colossus",
    "Weapons_Platform",
    "Yerdan",
    "Zookeeper"
]

def sanitize_filename(name):
    """清理文件名中的非法字符"""
    invalid_chars = '<>:"/\\|?*'
    for char in invalid_chars:
        name = name.replace(char, '_')
    return name

def main():
    # 读取数据库
    with open(MONSTERS_DB, 'r', encoding='utf-8') as f:
        monsters_db = json.load(f)
    
    print(f"共有 {len(REMAINING_FILES)} 个文件需要重命名\n")
    
    renamed_count = 0
    mapping_records = []
    
    for i, en_name in enumerate(REMAINING_FILES, 1):
        print(f"\n[{i}/{len(REMAINING_FILES)}] 英文名: {en_name}")
        
        # 检查文件是否还存在
        char_file = IMAGES_CHAR_DIR / f"{en_name}.webp"
        if not char_file.exists():
            print(f"  -> 文件不存在，跳过")
            continue
        
        # 询问中文名
        cn_name = input("  请输入对应的中文名（按回车跳过）: ").strip()
        
        if not cn_name:
            print("  -> 已跳过")
            continue
        
        # 清理中文名
        safe_cn = sanitize_filename(cn_name)
        
        # 重命名背景图
        bg_old = IMAGES_BG_DIR / f"{en_name}.webp"
        bg_new = IMAGES_BG_DIR / f"{safe_cn}.webp"
        if bg_old.exists():
            if not bg_new.exists():
                shutil.move(str(bg_old), str(bg_new))
                print(f"  ✓ 背景图已重命名: {safe_cn}.webp")
                renamed_count += 1
            else:
                print(f"  ! 背景图目标已存在: {safe_cn}.webp")
        
        # 重命名角色图
        char_old = IMAGES_CHAR_DIR / f"{en_name}.webp"
        char_new = IMAGES_CHAR_DIR / f"{safe_cn}.webp"
        if char_old.exists():
            if not char_new.exists():
                shutil.move(str(char_old), str(char_new))
                print(f"  ✓ 角色图已重命名: {safe_cn}.webp")
                renamed_count += 1
            else:
                print(f"  ! 角色图目标已存在: {safe_cn}.webp")
        
        # 记录映射
        mapping_records.append({
            "english": en_name,
            "chinese": cn_name
        })
        
        # 更新数据库（如果中文名在数据库中）
        if cn_name in monsters_db:
            monsters_db[cn_name]["image_path"] = f"images_monster/{safe_cn}.webp"
            print(f"  ✓ 已更新数据库")
    
    # 保存更新后的数据库
    print(f"\n保存数据库...")
    with open(MONSTERS_DB, 'w', encoding='utf-8') as f:
        json.dump(monsters_db, f, ensure_ascii=False, indent=2)
    
    # 保存映射记录
    if mapping_records:
        mapping_file = SCRIPT_DIR / "manual_mapping.json"
        with open(mapping_file, 'w', encoding='utf-8') as f:
            json.dump(mapping_records, f, ensure_ascii=False, indent=2)
        print(f"映射记录已保存到: {mapping_file}")
    
    print(f"\n=== 完成 ===")
    print(f"共重命名了 {renamed_count} 个文件")
    print(f"记录了 {len(mapping_records)} 个映射")

if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print("\n\n已取消操作")
