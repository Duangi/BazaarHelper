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

def get_available_image_names():
    """获取所有可用的图片文件名（不含扩展名）"""
    image_names = set()
    for img_file in CHAR_DIR.glob('*.webp'):
        image_names.add(img_file.stem)
    return image_names

def main():
    print("=" * 60)
    print("更新 monsters_db.json 中的英文名")
    print("=" * 60)
    
    # 加载数据库
    with open(MONSTERS_DB, 'r', encoding='utf-8') as f:
        monsters_db = json.load(f)
    
    # 获取所有图片文件名
    available_images = get_available_image_names()
    print(f"找到 {len(available_images)} 个图片文件\n")
    
    # 从"凯沃斯指挥官"开始
    start_processing = False
    updated_count = 0
    need_input_count = 0
    updated_db = False
    
    print("从'凯沃斯指挥官'开始处理...")
    print("如果图片存在，自动更新；如果不存在，询问你正确的英文名\n")
    
    for cn_name, data in monsters_db.items():
        if not isinstance(data, dict):
            continue
        
        # 从"凯沃斯指挥官"开始处理
        if cn_name == "凯沃斯指挥官":
            start_processing = True
        
        if not start_processing:
            continue
        
        current_en_name = data.get('name', '')
        
        # 检查当前英文名对应的图片是否存在
        if current_en_name and current_en_name in available_images:
            # 图片存在，不需要更新
            print(f"✓ {cn_name}: {current_en_name} (图片存在)")
            continue
        
        # 图片不存在，需要询问
        print(f"\n[{cn_name}]")
        print(f"  当前英文名: {current_en_name if current_en_name else '(无)'}")
        print(f"  图片不存在，请输入正确的英文名")
        
        user_input = input("  英文名 (或输入 'skip' 跳过, 'quit' 退出): ").strip()
        
        if user_input.lower() == 'quit':
            print("\n用户退出")
            break
        
        if user_input.lower() == 'skip' or not user_input:
            print("  → 跳过")
            continue
        
        # 检查输入的英文名是否有对应图片
        if user_input in available_images:
            data['name'] = user_input
            updated_count += 1
            updated_db = True
            print(f"  ✓ 已更新为: {user_input}")
        else:
            print(f"  ✗ 警告: 图片 {user_input}.webp 不存在，但已保存")
            data['name'] = user_input
            updated_count += 1
            updated_db = True
    
    # 保存更新后的数据库
    if updated_db:
        print("\n保存更新后的 monsters_db.json...")
        with open(MONSTERS_DB, 'w', encoding='utf-8') as f:
            json.dump(monsters_db, f, ensure_ascii=False, indent=2)
        print("✓ 数据库已更新")
    
    print("\n" + "=" * 60)
    print("完成！")
    print("=" * 60)
    print(f"已更新: {updated_count} 个怪物")
    print("=" * 60)

if __name__ == '__main__':
    main()
