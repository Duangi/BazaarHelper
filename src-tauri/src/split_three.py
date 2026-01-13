#!/usr/bin/env python3
"""
split_three.py
Split an image into three equal vertical slices and save them.

Usage:
  python split_three.py --input final.jpg --out-prefix final
"""
import os
import argparse
from PIL import Image


def split_three(input_path, out_prefix=None):
    if not os.path.exists(input_path):
        raise FileNotFoundError(input_path)
    im = Image.open(input_path)
    w, h = im.size

    third = w // 3
    base, ext = os.path.splitext(input_path)
    if out_prefix is None:
        out_prefix = base

    left_box = (0, 0, third, h)
    mid_box = (third, 0, third * 2, h)
    right_box = (third * 2, 0, w, h)

    left_path = f"{out_prefix}_left{ext}"
    mid_path = f"{out_prefix}_mid{ext}"
    right_path = f"{out_prefix}_right{ext}"

    # ensure output dir exists (if provided)
    outdir = os.path.dirname(left_path)
    if outdir:
        os.makedirs(outdir, exist_ok=True)

    im.crop(left_box).save(left_path)
    im.crop(mid_box).save(mid_path)
    im.crop(right_box).save(right_path)

    return left_path, mid_path, right_path


def main():
    p = argparse.ArgumentParser()
    p.add_argument('--input', '-i', required=True, help='Input image path')
    p.add_argument('--out-prefix', '-p', default=None, help='Output filename prefix (default uses input basename)')
    args = p.parse_args()

    left, mid, right = split_three(args.input, args.out_prefix)
    print(f'Saved: {left}, {mid}, {right}')


if __name__ == '__main__':
    main()
