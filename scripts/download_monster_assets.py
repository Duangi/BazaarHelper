#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Download background and character images for monsters listed in
src-tauri/resources/monsters_db.json whose keys do NOT contain 'Day'.
Saves images in separate directories:
- Background layer: src-tauri/resources/images_monster_bg/
- Character layer: src-tauri/resources/images_monster_char/
Updates monsters_db.json with paths to character images for recognition.
"""
import os
import time
import json
from pathlib import Path
import requests
from PIL import Image
import io

ROOT = Path(__file__).resolve().parents[1]
MONSTERS_DB = ROOT / 'src-tauri' / 'resources' / 'monsters_db.json'
BG_DIR = ROOT / 'src-tauri' / 'resources' / 'images_monster_bg'
CHAR_DIR = ROOT / 'src-tauri' / 'resources' / 'images_monster_char'
MAP_FILE = ROOT / 'src-tauri' / 'resources' / 'images_monster_map.json'

BG_DIR.mkdir(parents=True, exist_ok=True)
CHAR_DIR.mkdir(parents=True, exist_ok=True)


def load_json(p):
    with open(p, 'r', encoding='utf-8') as f:
        return json.load(f)


def save_json(p, data):
    with open(p, 'w', encoding='utf-8') as f:
        json.dump(data, f, ensure_ascii=False, indent=2)


def download_and_save(url, out_path, keep_transparency=False):
    """
    Download image from URL and save to out_path.
    If keep_transparency is True, saves as PNG preserving alpha channel.
    Otherwise converts to JPEG with white background.
    """
    try:
        headers = {'User-Agent': 'Mozilla/5.0'}
        r = requests.get(url, headers=headers, timeout=20)
        r.raise_for_status()
        img = Image.open(io.BytesIO(r.content))
        
        if keep_transparency:
            # Save as PNG to preserve transparency
            if img.mode != 'RGBA':
                img = img.convert('RGBA')
            img.save(out_path, 'PNG')
        else:
            # Convert to JPEG with white background
            if img.mode == 'RGBA':
                bg = Image.new('RGB', img.size, (255,255,255))
                bg.paste(img, mask=img.split()[-1])
                img = bg
            elif img.mode != 'RGB':
                img = img.convert('RGB')
            img.save(out_path, 'JPEG', quality=95)
        return True, None
    except Exception as e:
        return False, str(e)


def main():
    monsters = load_json(MONSTERS_DB)
    results = {}
    count = 0
    updated_monsters = {}

    # Filter: keys that are objects and the key string does NOT contain 'Day'
    targets = [k for k,v in monsters.items() if isinstance(v, dict) and 'Day' not in k]
    print(f"Found {len(targets)} monster entries to process")

    for key in targets:
        data = monsters[key]
        name_zh = data.get('name_zh') or key
        bg_url = data.get('background_url') or data.get('background') or ''
        ch_url = data.get('character_url') or data.get('character') or ''

        if not bg_url and not ch_url:
            results[key] = {'status': 'no_urls', 'background_url': bg_url, 'character_url': ch_url}
            print(f"[{key}] 无 background/character URL，跳过")
            updated_monsters[key] = data
            continue

        entry = {'background': None, 'character': None, 'background_url': bg_url, 'character_url': ch_url}

        # Background (saved to BG_DIR as JPEG)
        if bg_url:
            out_name = f"{name_zh}_bg.jpg"
            out_path = BG_DIR / out_name
            if out_path.exists():
                entry['background'] = str(out_path.relative_to(ROOT / 'src-tauri' / 'resources'))
                print(f"[{key}] 背景已存在，跳过: {out_name}")
            else:
                ok, err = download_and_save(bg_url, out_path, keep_transparency=False)
                if ok:
                    entry['background'] = str(out_path.relative_to(ROOT / 'src-tauri' / 'resources'))
                    print(f"[{key}] 下载背景 -> {out_name}")
                else:
                    entry['background_error'] = err
                    print(f"[{key}] 下载背景失败: {err}")
        else:
            print(f"[{key}] 无背景URL")

        # Character (saved to CHAR_DIR as PNG to preserve transparency)
        char_path_rel = None
        if ch_url:
            out_name = f"{name_zh}_char.png"
            out_path = CHAR_DIR / out_name
            if out_path.exists():
                entry['character'] = str(out_path.relative_to(ROOT / 'src-tauri' / 'resources'))
                char_path_rel = entry['character']
                print(f"[{key}] 角色图已存在，跳过: {out_name}")
            else:
                ok, err = download_and_save(ch_url, out_path, keep_transparency=True)
                if ok:
                    entry['character'] = str(out_path.relative_to(ROOT / 'src-tauri' / 'resources'))
                    char_path_rel = entry['character']
                    print(f"[{key}] 下载角色 -> {out_name}")
                else:
                    entry['character_error'] = err
                    print(f"[{key}] 下载角色失败: {err}")
        else:
            print(f"[{key}] 无角色图URL")

        # Update monsters data with character image path for recognition
        updated_data = data.copy()
        if char_path_rel:
            updated_data['image'] = char_path_rel
        updated_monsters[key] = updated_data

        results[key] = entry
        count += 1

        # polite delay
        time.sleep(1.0)

    # Save mapping file
    save_json(MAP_FILE, results)
    print(f"\n映射文件保存在: {MAP_FILE}")
    
    # Update monsters_db.json with character image paths
    save_json(MONSTERS_DB, updated_monsters)
    print(f"更新 monsters_db.json，使用角色层图片路径")
    
    print(f"\n完成: 处理 {count} 条目标")
    print(f"背景图保存在: {BG_DIR}")
    print(f"角色图保存在: {CHAR_DIR}")
    print(f"识图将使用角色层 PNG 图片（保留透明通道）")

if __name__ == '__main__':
    main()
