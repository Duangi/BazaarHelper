#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
将怪物图片重命名为英文名（去掉_bg和_char后缀）
"""
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BG_DIR = ROOT / 'src-tauri' / 'resources' / 'images_monster_bg'
CHAR_DIR = ROOT / 'src-tauri' / 'resources' / 'images_monster_char'

def rename_to_english_name():
    print("=" * 60)
    print("重命名怪物图片为英文名")
    print("=" * 60)
    
    # 处理背景图
    print("\n处理背景图...")
    bg_count = 0
    for img_file in BG_DIR.glob('*.webp'):
        if img_file.stem.endswith('_bg'):
            new_name = img_file.stem[:-3] + '.webp'  # 去掉 _bg
            new_path = img_file.parent / new_name
            if not new_path.exists():
                img_file.rename(new_path)
                bg_count += 1
                print(f"  ✓ {img_file.name} → {new_name}")
    
    # 处理角色图
    print(f"\n处理角色图...")
    char_count = 0
    for img_file in CHAR_DIR.glob('*.webp'):
        if img_file.stem.endswith('_char'):
            new_name = img_file.stem[:-5] + '.webp'  # 去掉 _char
            new_path = img_file.parent / new_name
            if not new_path.exists():
                img_file.rename(new_path)
                char_count += 1
                print(f"  ✓ {img_file.name} → {new_name}")
    
    print("\n" + "=" * 60)
    print("完成！")
    print("=" * 60)
    print(f"背景图重命名: {bg_count} 张")
    print(f"角色图重命名: {char_count} 张")
    print("=" * 60)

if __name__ == '__main__':
    rename_to_english_name()
