#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
测试分层图片显示效果
将背景层和角色层合成显示，验证分层存储是否正确
"""
import json
from pathlib import Path
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
RESOURCES = ROOT / 'src-tauri' / 'resources'
BG_DIR = RESOURCES / 'images_monster_bg'
CHAR_DIR = RESOURCES / 'images_monster_char'
MAP_FILE = RESOURCES / 'images_monster_map.json'
OUTPUT_DIR = RESOURCES / 'images_monster_composed'

OUTPUT_DIR.mkdir(parents=True, exist_ok=True)


def compose_layers(bg_path, char_path, output_path):
    """合成背景层和角色层"""
    try:
        # 读取背景（JPEG）
        bg = Image.open(bg_path).convert('RGBA')
        
        # 读取角色（PNG with alpha）
        char = Image.open(char_path).convert('RGBA')
        
        # 调整角色层大小以匹配背景
        if char.size != bg.size:
            char = char.resize(bg.size, Image.Resampling.LANCZOS)
        
        # 合成
        composed = Image.alpha_composite(bg, char)
        
        # 保存为 JPEG
        rgb_composed = composed.convert('RGB')
        rgb_composed.save(output_path, 'JPEG', quality=95)
        return True
    except Exception as e:
        print(f"合成失败: {e}")
        return False


def main():
    with open(MAP_FILE, 'r', encoding='utf-8') as f:
        mapping = json.load(f)
    
    count = 0
    for key, data in mapping.items():
        bg_rel = data.get('background')
        char_rel = data.get('character')
        
        if not bg_rel or not char_rel:
            continue
        
        bg_path = RESOURCES / bg_rel
        char_path = RESOURCES / char_rel
        
        if not bg_path.exists() or not char_path.exists():
            print(f"[{key}] 文件不存在，跳过")
            continue
        
        output_name = f"{key}_composed.jpg"
        output_path = OUTPUT_DIR / output_name
        
        if compose_layers(bg_path, char_path, output_path):
            count += 1
            print(f"[{count}] {key} -> {output_name}")
    
    print(f"\n完成！合成了 {count} 张图片")
    print(f"输出目录: {OUTPUT_DIR}")
    print(f"\n你可以对比:")
    print(f"  - 角色层 (用于识图): {CHAR_DIR}")
    print(f"  - 背景层: {BG_DIR}")
    print(f"  - 合成图 (用于展示): {OUTPUT_DIR}")


if __name__ == '__main__':
    main()
