#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
更新 monsters_db.json 中的英文名，使其与图片文件名匹配
"""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MONSTERS_DB = ROOT / 'src-tauri' / 'resources' / 'monsters_db.json'
CHAR_DIR = ROOT / 'src-tauri' / 'resources' / 'images_monster_char'

def main():
    print("=" * 60)
    print("更新 monsters_db.json 中的英文名")
    print("=" * 60)
    
    # 1. 获取所有图片文件名（不含扩展名）
    image_names = set()
    for img_file in CHAR_DIR.glob('*.webp'):
        image_names.add(img_file.stem)
    
    print(f"\n找到 {len(image_names)} 个图片文件")
    
    # 2. 加载 monsters_db.json
    with open(MONSTERS_DB, 'r', encoding='utf-8') as f:
        monsters_db = json.load(f)
    
    # 3. 检查每个怪物的英文名是否有对应的图片
    need_input = []
    matched = 0
    updated_db = False
    
    for cn_name, data in monsters_db.items():
        if not isinstance(data, dict):
            continue
        
        en_name = data.get('name', '')
        monster_id = data.get('id', '')
        
        # 检查是否有匹配的图片（英文名或ID）
        has_match = False
        matched_name = None
        
        # 尝试多种格式
        candidates = []
        if en_name:
            candidates.append(en_name)
            candidates.append(en_name.replace(' ', '_'))
            candidates.append(en_name.replace('.', '_'))
        if monster_id:
            candidates.append(monster_id)
        
        for candidate in candidates:
            if candidate in image_names:
                has_match = True
                matched_name = candidate
                break
        
        if has_match:
            # 更新英文名为匹配的文件名
            if data.get('name') != matched_name:
                data['name'] = matched_name
                updated_db = True
                print(f"  ✓ {cn_name}: {en_name} → {matched_name}")
            matched += 1
        else:
            # 没有找到匹配的图片
            need_input.append((cn_name, en_name, monster_id))
    
    print(f"\n已匹配: {matched} 个怪物")
    print(f"需要输入: {len(need_input)} 个怪物\n")
    
    # 4. 对于没有匹配的，询问用户
    if need_input:
        print("=" * 60)
        print("以下怪物找不到对应的图片，请提供英文名")
        print("（输入 'skip' 跳过，'quit' 退出）")
        print("=" * 60)
        
        for i, (cn_name, old_en_name, monster_id) in enumerate(need_input, 1):
            print(f"\n[{i}/{len(need_input)}] 中文名: {cn_name}")
            if old_en_name:
                print(f"  当前英文名: {old_en_name}")
            if monster_id:
                print(f"  ID: {monster_id}")
            
            new_en_name = input("  新英文名: ").strip()
            
            if new_en_name.lower() == 'quit':
                print("\n用户退出")
                break
            
            if new_en_name.lower() == 'skip' or not new_en_name:
                print("  → 跳过")
                continue
            
            # 检查新英文名是否有对应的图片
            if new_en_name in image_names or new_en_name.replace(' ', '_') in image_names:
                actual_name = new_en_name if new_en_name in image_names else new_en_name.replace(' ', '_')
                monsters_db[cn_name]['name'] = actual_name
                updated_db = True
                print(f"  ✓ 已更新为: {actual_name}")
            else:
                print(f"  ⚠ 警告: 找不到图片 {new_en_name}.webp，但仍然更新")
                monsters_db[cn_name]['name'] = new_en_name
                updated_db = True
    
    # 5. 保存更新后的数据库
    if updated_db:
        print("\n保存更新后的 monsters_db.json...")
        with open(MONSTERS_DB, 'w', encoding='utf-8') as f:
            json.dump(monsters_db, f, ensure_ascii=False, indent=2)
        print("✓ 数据库已更新")
    else:
        print("\n无需更新")
    
    print("\n" + "=" * 60)
    print("完成！")
    print("=" * 60)

if __name__ == '__main__':
    main()
