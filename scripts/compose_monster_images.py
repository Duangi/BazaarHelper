#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Compose monster background + character images into a single JPG per monster.
Reads `monsters_image.json` in repo root (array of objects). For each entry,
downloads `背景图`/`background` and `图片链接`/`image` and alpha-composites
character over background. Saves result as `src-tauri/resources/images_monster/<中文名>.jpg`.
Also writes a JSON report at `src-tauri/resources/images_monster_map.json` with successes/errors.

If only one image URL exists, the script will download and convert it to JPG
and save using the monster name.
"""
import json
import time
from pathlib import Path
import requests
from PIL import Image
import io

ROOT = Path(__file__).resolve().parents[1]
INPUT = ROOT / 'monsters_image.json'
OUT_DIR = ROOT / 'src-tauri' / 'resources' / 'images_monster'
OUT_DIR.mkdir(parents=True, exist_ok=True)
MAP_FILE = ROOT / 'src-tauri' / 'resources' / 'images_monster_map.json'


def load_json(p):
    with open(p, 'r', encoding='utf-8') as f:
        return json.load(f)


def save_json(p, data):
    with open(p, 'w', encoding='utf-8') as f:
        json.dump(data, f, ensure_ascii=False, indent=2)


def download_image(url, timeout=20):
    headers = {'User-Agent': 'Mozilla/5.0'}
    resp = requests.get(url, headers=headers, timeout=timeout)
    resp.raise_for_status()
    return Image.open(io.BytesIO(resp.content))


def ensure_rgba(img: Image.Image):
    if img.mode != 'RGBA':
        try:
            return img.convert('RGBA')
        except Exception:
            # fallback: create RGBA and paste
            rgba = Image.new('RGBA', img.size)
            rgba.paste(img)
            return rgba
    return img


def compose_and_save(bg_img, ch_img, out_path: Path):
    # bg_img and ch_img are PIL Image objects
    bg = ensure_rgba(bg_img)
    ch = ensure_rgba(ch_img)
    if ch.size != bg.size:
        ch = ch.resize(bg.size, Image.Resampling.LANCZOS)
    try:
        comp = Image.alpha_composite(bg, ch)
    except Exception:
        # fallback: paste with alpha
        comp = bg.copy()
        comp.paste(ch, (0, 0), ch)
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
    data = load_json(INPUT)
    report = {}
    total = len(data)
    print(f'Processing {total} entries from {INPUT}')

    for i, entry in enumerate(data, 1):
        # determine name
        name = entry.get('中文名') or entry.get('name') or entry.get('name_zh') or f'monster_{i}'
        safe_name = ''.join(c for c in name if c not in '/\\:')
        out_file = OUT_DIR / f"{safe_name}.jpg"
        bg_url = entry.get('背景图') or entry.get('background') or ''
        ch_url = entry.get('图片链接') or entry.get('image') or ''

        rec = {'background_url': bg_url, 'image_url': ch_url, 'out': str(out_file.relative_to(ROOT / 'src-tauri' / 'resources'))}

        try:
            if bg_url and ch_url:
                print(f'[{i}/{total}] {name}: downloading background and image')
                bg_img = download_image(bg_url)
                ch_img = download_image(ch_url)
                compose_and_save(bg_img, ch_img, out_file)
                rec['status'] = 'ok_composed'
            elif ch_url:
                print(f'[{i}/{total}] {name}: downloading single image')
                img = download_image(ch_url)
                single_image_save(img, out_file)
                rec['status'] = 'ok_single'
            elif bg_url:
                print(f'[{i}/{total}] {name}: downloading single background image')
                img = download_image(bg_url)
                single_image_save(img, out_file)
                rec['status'] = 'ok_single_background'
            else:
                print(f'[{i}/{total}] {name}: no urls, skipping')
                rec['status'] = 'no_urls'
        except Exception as e:
            print(f'[{i}/{total}] {name}: ERROR - {e}')
            rec['status'] = 'error'
            rec['error'] = str(e)

        report[name] = rec
        # polite delay
        time.sleep(0.8)

    save_json(MAP_FILE, report)
    print(f'Done. Report written to {MAP_FILE}')


if __name__ == '__main__':
    main()
