#!/usr/bin/env python3
"""
batch_compare_encounters.py
Crop the input with given params, then compare the crop against each encounter image
listed in src-tauri/resources/combat_encounters.json (uses the `image` field under each top-level key).
Outputs a JSON array of results.
"""
import os
import sys
import json
import argparse
from pathlib import Path

repo_root = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import crop_focus
import compare_retired


def load_encounter_images(enc_path):
    with open(enc_path, 'r', encoding='utf-8') as f:
        data = json.load(f)
    items = []
    for key, val in data.items():
        if isinstance(val, dict) and 'image' in val:
            items.append((key, val['image']))
    return items


def build_img_path(img_rel):
    # img_rel like "images/xxx.jpg"
    p = repo_root / 'src-tauri' / 'resources' / img_rel
    return str(p)


def compare_crop_to_db(crop_path, enc_path, out_path=None, tpl_thresh=0.75):
    items = load_encounter_images(enc_path)
    results = []
    for key, img_rel in items:
        img_path = build_img_path(img_rel)
        if not os.path.exists(img_path):
            results.append({'key': key, 'image': img_rel, 'error': 'missing'})
            continue
        ph = compare_retired.phash_distance(crop_path, img_path)
        # above compute_mse call will be replaced by calling compute_mse with proper grays below
        # compute grayscale images
        a = compare_retired.load_image(crop_path)
        b = compare_retired.load_image(img_path)
        if a is None or b is None:
            results.append({'key': key, 'image': img_rel, 'error': 'load_failed'})
            continue
        a_gray = compare_retired.cv2.cvtColor(a, compare_retired.cv2.COLOR_BGR2GRAY)
        b_gray = compare_retired.cv2.cvtColor(b, compare_retired.cv2.COLOR_BGR2GRAY)
        mse = compare_retired.compute_mse(a_gray, b_gray)
        ssim = compare_retired.compute_ssim(a_gray, b_gray)
        tpl_scores = compare_retired.template_match_score(crop_path, img_path)
        orb = compare_retired.orb_match_stats(crop_path, img_path)

        results.append({
            'key': key,
            'image': img_rel,
            'phash_distance': ph,
            'mse': mse,
            'ssim': ssim,
            'template_scores': tpl_scores,
            'orb': orb,
        })

    if out_path:
        with open(out_path, 'w', encoding='utf-8') as f:
            json.dump(results, f, ensure_ascii=False, indent=2)
    return results


def main():
    p = argparse.ArgumentParser()
    p.add_argument('--input', '-i', required=True, help='Input image path')
    p.add_argument('--top-fraction', type=float, default=0.5)
    p.add_argument('--center-fraction', type=float, default=0.6)
    p.add_argument('--keep', choices=['top', 'bottom'], default='top')
    p.add_argument('--h-offset', type=float, default=0.0)
    p.add_argument('--pad-px', type=int, default=0)
    p.add_argument('--combat-json', default=str(repo_root / 'src-tauri' / 'resources' / 'combat_encounters.json'))
    p.add_argument('--out', default=None, help='Output JSON report path')
    args = p.parse_args()

    # produce crop
    out_crop = str(Path(args.input).with_name('test_focus_run.jpg'))
    crop_focus.crop_focus(args.input, out_crop, top_fraction=args.top_fraction, center_fraction=args.center_fraction, h_offset=args.h_offset, pad_px=args.pad_px, keep=args.keep)
    print('Cropped to', out_crop)

    res = compare_crop_to_db(out_crop, args.combat_json, out_path=args.out)
    print(json.dumps(res, ensure_ascii=False, indent=2))


if __name__ == '__main__':
    main()
