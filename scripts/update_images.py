#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Scan monsters DB, move monster images into images_monster, update JSON,
and report missing item/skill images. Provides CLI hook to accept
mappings for downloading/compositing later.
"""
import os
import json
import shutil
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MONSTERS_DB = ROOT / 'src-tauri' / 'resources' / 'monsters_db.json'
IMAGES_DIR = ROOT / 'src-tauri' / 'resources' / 'images'
IMAGES_MONSTER_DIR = ROOT / 'src-tauri' / 'resources' / 'images_monster'
REPORT = ROOT / 'missing_images_report.json'
BACKUP = ROOT / 'src-tauri' / 'resources' / 'monsters_db.json.bak'


def load_json(path):
    with open(path, 'r', encoding='utf-8') as f:
        return json.load(f)


def save_json(path, data):
    with open(path, 'w', encoding='utf-8') as f:
        json.dump(data, f, ensure_ascii=False, indent=2)


def ensure_dirs():
    IMAGES_DIR.mkdir(parents=True, exist_ok=True)
    IMAGES_MONSTER_DIR.mkdir(parents=True, exist_ok=True)


def file_exists_in_images(rel_path):
    # rel_path can be like 'images/name.jpg' or 'images/name.webp' or 'name.jpg'
    p = Path(rel_path)
    if p.is_absolute():
        return p.exists()
    # try relative to images dir
    candidate = IMAGES_DIR / p.name
    return candidate.exists()


def move_monster_image(name_zh):
    src = IMAGES_DIR / f"{name_zh}.jpg"
    if src.exists():
        dst = IMAGES_MONSTER_DIR / src.name
        # if destination exists, skip move
        if dst.exists():
            return dst
        shutil.move(str(src), str(dst))
        return dst
    return None


def scan_and_update():
    print(f"Loading {MONSTERS_DB}")
    monsters = load_json(MONSTERS_DB)

    missing_report = {
        'missing_monster_images': {},
        'missing_item_images': {},
        'missing_skill_images': {}
    }

    ensure_dirs()

    updated = False

    for key, data in list(monsters.items()):
        if not isinstance(data, dict):
            continue
        name_zh = data.get('name_zh') or key

        # If monster already has image path and file exists, keep it
        cur_image = data.get('image', '')
        image_found = False

        if cur_image:
            # if already points to images_monster or images and file exists
            candidate = ROOT / 'src-tauri' / 'resources' / cur_image
            if candidate.exists():
                image_found = True
            else:
                # maybe it's just a filename under images
                candidate2 = IMAGES_DIR / Path(cur_image).name
                if candidate2.exists():
                    # move to images_monster
                    dst = IMAGES_MONSTER_DIR / candidate2.name
                    shutil.move(str(candidate2), str(dst))
                    data['image'] = f"images_monster/{dst.name}"
                    updated = True
                    image_found = True

        if not image_found:
            # try find by name_zh.jpg in images
            moved = move_monster_image(name_zh)
            if moved:
                data['image'] = f"images_monster/{moved.name}"
                updated = True
            else:
                # record missing
                missing_report['missing_monster_images'][key] = {
                    'name_zh': name_zh,
                    'expected': f"images/{name_zh}.jpg",
                    'current_image_field': cur_image
                }

        # Check items
        for item in data.get('items', []):
            img = item.get('image', '')
            if not img:
                missing_report['missing_item_images'].setdefault(key, []).append({
                    'item_name': item.get('name'),
                    'image': img
                })
                continue
            # if image path points to images_monster, leave (items should stay in images)
            # check if exists; try in images folder
            candidate = ROOT / 'src-tauri' / 'resources' / img
            if not candidate.exists():
                candidate2 = IMAGES_DIR / Path(img).name
                if not candidate2.exists():
                    missing_report['missing_item_images'].setdefault(key, []).append({
                        'item_name': item.get('name'),
                        'image': img
                    })

        # Check skills
        for skill in data.get('skills', []):
            img = skill.get('image', '')
            if not img:
                missing_report['missing_skill_images'].setdefault(key, []).append({
                    'skill_name': skill.get('name'),
                    'image': img
                })
                continue
            candidate = ROOT / 'src-tauri' / 'resources' / img
            if not candidate.exists():
                candidate2 = IMAGES_DIR / Path(img).name
                if not candidate2.exists():
                    missing_report['missing_skill_images'].setdefault(key, []).append({
                        'skill_name': skill.get('name'),
                        'image': img
                    })

    # backup and save if updated
    if updated:
        print(f"Backing up original DB to {BACKUP}")
        shutil.copy2(MONSTERS_DB, BACKUP)
        print(f"Writing updated {MONSTERS_DB}")
        save_json(MONSTERS_DB, monsters)

    print(f"Writing missing report to {REPORT}")
    save_json(REPORT, missing_report)

    print("Done. Summary:")
    print(f"  Monsters scanned: {len(monsters)}")
    print(f"  Missing monster images: {len(missing_report['missing_monster_images'])}")
    print(f"  Missing item images entries: {sum(len(v) for v in missing_report['missing_item_images'].values())}")
    print(f"  Missing skill images entries: {sum(len(v) for v in missing_report['missing_skill_images'].values())}")

    return missing_report


if __name__ == '__main__':
    report = scan_and_update()

    # Direct interactive mode: prompt user once per missing item (monster/item/skill)
    if any(report.values()):
        monsters = load_json(MONSTERS_DB)

        def download_image_to(path, url):
            try:
                import requests
                from PIL import Image
                import io
                resp = requests.get(url, headers={'User-Agent': 'Mozilla/5.0'}, timeout=15)
                resp.raise_for_status()
                img = Image.open(io.BytesIO(resp.content))
                if img.mode == 'RGBA':
                    bg = Image.new('RGB', img.size, (255, 255, 255))
                    bg.paste(img, mask=img.split()[-1])
                    img = bg
                elif img.mode != 'RGB':
                    img = img.convert('RGB')
                img.save(path, 'JPEG', quality=95)
                return True
            except Exception as e:
                print(f'  ✗ 下载或保存失败: {e}')
                return False

        # For each missing monster image, ask once for a single image URL and save to images_monster
        for key, info in report.get('missing_monster_images', {}).items():
            name = info.get('name_zh') or key
            out = IMAGES_MONSTER_DIR / f"{name}.jpg"
            url = input(f"请提供怪物 '{key}' ({name}) 的图片URL (回车跳过): ").strip()
            if url:
                ok = download_image_to(str(out), url)
                if ok:
                    m = monsters.get(key, {})
                    m['image'] = f"images_monster/{out.name}"
                    monsters[key] = m
                    print(f"  ✓ 保存并更新: images_monster/{out.name}")
                else:
                    print("  ✗ 保存失败，已跳过")
            else:
                print("  跳过（未提供URL）")

        # For each missing item image, ask once for webp URL and save to images
        for key, items in report.get('missing_item_images', {}).items():
            for it in items:
                iname = it.get('item_name')
                out = IMAGES_DIR / f"{iname}.jpg"
                url = input(f"请提供怪物 '{key}' 的物品 '{iname}' 的图片URL (webp) (回车跳过): ").strip()
                if url:
                    ok = download_image_to(str(out), url)
                    if ok:
                        m = monsters.get(key, {})
                        for obj in m.get('items', []):
                            if obj.get('name') == iname:
                                obj['image'] = f"images/{out.name}"
                        monsters[key] = m
                        print(f"  ✓ 下载并保存: images/{out.name}")
                    else:
                        print("  ✗ 下载失败，已跳过")

        # For each missing skill image, ask once for webp URL and save to images
        for key, skills in report.get('missing_skill_images', {}).items():
            for sk in skills:
                sname = sk.get('skill_name')
                out = IMAGES_DIR / f"{sname}.jpg"
                url = input(f"请提供怪物 '{key}' 的技能 '{sname}' 的图片URL (webp) (回车跳过): ").strip()
                if url:
                    ok = download_image_to(str(out), url)
                    if ok:
                        m = monsters.get(key, {})
                        for obj in m.get('skills', []):
                            if obj.get('name') == sname:
                                obj['image'] = f"images/{out.name}"
                        monsters[key] = m
                        print(f"  ✓ 下载并保存: images/{out.name}")
                    else:
                        print("  ✗ 下载失败，已跳过")

        # Save updated monsters DB and re-run scan to refresh report
        print('\n保存更新后的 monsters_db.json')
        save_json(MONSTERS_DB, monsters)
        print('重新生成缺失图片报告')
        new_report = scan_and_update()
        print('交互完成.')
    else:
        print('未发现缺失图片，退出。')
