#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Retry compose/download for a single monster from monsters_image.json
Usage: python scripts/retry_compose.py "焰风勇士"
"""
import sys
import json
from pathlib import Path
import time
import requests
from PIL import Image
import io

ROOT = Path(__file__).resolve().parents[1]
INPUT = ROOT / 'monsters_image.json'
OUT_DIR = ROOT / 'src-tauri' / 'resources' / 'images_monster'
MAP_FILE = ROOT / 'src-tauri' / 'resources' / 'images_monster_map.json'
OUT_DIR.mkdir(parents=True, exist_ok=True)


def load_json(p):
    with open(p, 'r', encoding='utf-8') as f:
        return json.load(f)


def save_json(p, data):
    with open(p, 'w', encoding='utf-8') as f:
        json.dump(data, f, ensure_ascii=False, indent=2)


def download_image(url, timeout=20):
    headers = {'User-Agent': 'Mozilla/5.0'}
    r = requests.get(url, headers=headers, timeout=timeout)
    r.raise_for_status()
    return Image.open(io.BytesIO(r.content))


def ensure_rgba(img: Image.Image):
    if img.mode != 'RGBA':
        return img.convert('RGBA')
    return img


def compose_and_save(bg_img, ch_img, out_path: Path):
    bg = ensure_rgba(bg_img)
    ch = ensure_rgba(ch_img)
    if ch.size != bg.size:
        ch = ch.resize(bg.size, Image.Resampling.LANCZOS)
    comp = Image.alpha_composite(bg, ch)
    if comp.mode == 'RGBA':
        rgb = Image.new('RGB', comp.size, (255, 255, 255))
        rgb.paste(comp, mask=comp.split()[-1])
        comp = rgb
    comp.save(out_path, 'JPEG', quality=95)


def single_image_save(img, out_path: Path):
    if img.mode == 'RGBA':
        bg = Image.new('RGB', img.size, (255, 255, 255))
        bg.paste(img, mask=img.split()[-1])
        img = bg
    elif img.mode != 'RGB':
        img = img.convert('RGB')
    img.save(out_path, 'JPEG', quality=95)


def main():
    if len(sys.argv) < 2:
        print('Usage: python scripts/retry_compose.py "怪物中文名"')
        return
    target = sys.argv[1]
    data = load_json(INPUT)
    found = None
    for entry in data:
        if entry.get('中文名') == target or entry.get('name') == target or entry.get('name_zh') == target:
            found = entry
            break
    if not found:
        print(f'未在 {INPUT} 找到: {target}')
        return

    name = found.get('中文名') or found.get('name') or target
    safe_name = ''.join(c for c in name if c not in '/\\:')
    out_file = OUT_DIR / f"{safe_name}.jpg"
    bg_url = found.get('背景图') or found.get('background') or ''
    ch_url = found.get('图片链接') or found.get('image') or ''

    report = load_json(MAP_FILE) if MAP_FILE.exists() else {}

    try:
        if bg_url and ch_url:
            print(f'Processing {name}: downloading background and image')
            bg = download_image(bg_url)
            ch = download_image(ch_url)
            compose_and_save(bg, ch, out_file)
            status = 'ok_composed'
        elif ch_url:
            print(f'Processing {name}: downloading single image')
            img = download_image(ch_url)
            single_image_save(img, out_file)
            status = 'ok_single'
        elif bg_url:
            print(f'Processing {name}: downloading single background')
            img = download_image(bg_url)
            single_image_save(img, out_file)
            status = 'ok_single_background'
        else:
            print('No URLs found')
            status = 'no_urls'
    except Exception as e:
        print(f'Error: {e}')
        status = 'error'
        report[name] = {'status': status, 'error': str(e)}
        save_json(MAP_FILE, report)
        return

    report[name] = {'status': status, 'background_url': bg_url, 'image_url': ch_url, 'out': str(out_file.relative_to(ROOT / 'src-tauri' / 'resources'))}
    save_json(MAP_FILE, report)
    print(f'Done: {out_file}')


if __name__ == '__main__':
    main()
