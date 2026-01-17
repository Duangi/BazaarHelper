#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
交互式重命名怪物图片 - 通过用户输入中文名来匹配ID
重命名为纯ID（不带后缀）
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

def build_chinese_to_id_map(monsters_db):
    """构建中文名到ID的映射"""
    chinese_to_id = {}
    for monster_id, data in monsters_db.items():
        if isinstance(data, dict):
            # 获取中文名
            chinese_name = data.get('name_zh', '')
            # 获取ID（如果有的话）
            actual_id = data.get('id', '')
            if chinese_name and actual_id:
                chinese_to_id[chinese_name] = actual_id
    return chinese_to_id

def get_unmapped_files(directory, suffix):
    """获取所有未映射的文件（仍是英文名的文件）"""
    unmapped = []
    if not directory.exists():
        return unmapped
    
    for img_file in directory.glob('*.webp'):
        file_stem = img_file.stem
        # 如果不是UUID格式，说明还是英文名
        # UUID格式: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
        if not re.match(r'^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}(_bg|_char)?$', file_stem):
            if suffix in file_stem:
                eng_name = file_stem.replace(suffix, '')
            else:
                eng_name = file_stem
            # 检查是否包含字母（确认是英文名）
            if any(c.isalpha() for c in eng_name):
                unmapped.append((img_file, eng_name))
    
    return unmapped

def main():
    print("=" * 60)
    print("交互式重命名怪物图片")
    print("=" * 60)
    
    # 加载数据库
    monsters_db = load_monsters_db()
    chinese_to_id = build_chinese_to_id_map(monsters_db)
    print(f"已加载 {len(chinese_to_id)} 个怪物中文名映射\n")
    
    # 获取所有未映射的背景图文件
    bg_unmapped = get_unmapped_files(BG_DIR, '_bg')
    char_unmapped = get_unmapped_files(CHAR_DIR, '_char')
    
    # 合并去重（基于英文名）
    eng_names_set = set()
    for _, eng_name in bg_unmapped:
        eng_names_set.add(eng_name)
    for _, eng_name in char_unmapped:
        eng_names_set.add(eng_name)
    
    eng_names = sorted(eng_names_set)
    
    if not eng_names:
        print("所有文件都已重命名完成！")
        return
    
    print(f"发现 {len(eng_names)} 个未映射的怪物\n")
    print("请输入中文名（输入 'skip' 跳过，输入 'quit' 退出）\n")
    
    renamed_count = 0
    skipped_count = 0
    updated_db = False
    
    for i, eng_name in enumerate(eng_names, 1):
        print(f"[{i}/{len(eng_names)}] 英文名: {eng_name}")
        chinese_name = input("中文名: ").strip()
        
        if chinese_name.lower() == 'quit':
            print("\n用户退出")
            break
        
        if chinese_name.lower() == 'skip' or not chinese_name:
            print("  → 跳过\n")
            skipped_count += 1
            continue
        
        # 查找ID
        monster_id = chinese_to_id.get(chinese_name)
        
        # 如果没找到ID，询问用户
        if not monster_id:
            print(f"  未找到中文名 '{chinese_name}' 对应的ID")
            user_id = input("  请输入ID（或按回车跳过）: ").strip()
            
            if not user_id:
                print("  → 跳过\n")
                skipped_count += 1
                continue
            
            monster_id = user_id
            
            # 更新 monsters_db.json
            if chinese_name in monsters_db:
                monsters_db[chinese_name]['id'] = monster_id
                monsters_db[chinese_name]['name'] = eng_name
            else:
                monsters_db[chinese_name] = {
                    'id': monster_id,
                    'name': eng_name,
                    'name_zh': chinese_name
                }
            
            chinese_to_id[chinese_name] = monster_id
            updated_db = True
            print(f"  ✓ 已添加到数据库")
        
        # 重命名背景图 - 改为纯ID.webp
        bg_file = BG_DIR / f"{eng_name}_bg.webp"
        if not bg_file.exists():
            bg_file = BG_DIR / f"{eng_name}.webp"
        
        if bg_file.exists():
            new_bg = BG_DIR / f"{monster_id}.webp"
            if not new_bg.exists():
                bg_file.rename(new_bg)
                print(f"  ✓ 背景图: {bg_file.name} → {new_bg.name}")
        
        # 重命名角色图 - 改为纯ID.webp
        char_file = CHAR_DIR / f"{eng_name}_char.webp"
        if not char_file.exists():
            char_file = CHAR_DIR / f"{eng_name}.webp"
        
        if char_file.exists():
            new_char = CHAR_DIR / f"{monster_id}.webp"
            if not new_char.exists():
                char_file.rename(new_char)
                print(f"  ✓ 角色图: {char_file.name} → {new_char.name}")
        
        renamed_count += 1
        print()
    
    # 保存更新后的数据库
    if updated_db:
        print("\n保存更新后的 monsters_db.json...")
        with open(MONSTERS_DB, 'w', encoding='utf-8') as f:
            json.dump(monsters_db, f, ensure_ascii=False, indent=2)
        print("✓ 数据库已更新")
    
    # 汇总结果
    print("\n" + "=" * 60)
    print("重命名完成")
    print("=" * 60)
    print(f"成功重命名: {renamed_count} 个怪物")
    print(f"跳过: {skipped_count} 个")
    if updated_db:
        print(f"数据库已更新")
    print("=" * 60)

if __name__ == '__main__':
    main()
