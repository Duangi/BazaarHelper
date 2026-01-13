#!/usr/bin/env python3
"""
crop_focus.py
Crop an image's top portion and centered horizontal region with user-controlled parameters.

Usage examples:
  python src-tauri/src/crop_focus.py --input src-tauri/src/test.jpg --output src-tauri/src/test_focus_custom.jpg --top-fraction 0.5 --center-fraction 0.6

Parameters:
  --top-fraction    Fraction (0-1) of image height to keep from the top. Default 0.5.
  --center-fraction Fraction (0-1) of image width to keep centered. Default 0.6.
  --h-offset        Horizontal offset as fraction of width to shift crop center (negative=left, positive=right). Default 0.0.
  --pad-px          Add padding (pixels) to final crop on each side (can be negative). Default 0.
"""
import os
import sys
import argparse
from PIL import Image


def crop_focus(input_path, output_path, top_fraction=0.5, center_fraction=0.6, h_offset=0.0, pad_px=0, keep='top'):
    if not os.path.exists(input_path):
        raise FileNotFoundError(input_path)
    im = Image.open(input_path)
    w, h = im.size

    # compute top crop height
    top_h = max(1, int(round(h * float(top_fraction))))

    # compute centered width
    center_w = max(1, int(round(w * float(center_fraction))))

    # center x with offset
    center_x = int(round(w / 2 + float(h_offset) * w))

    left = center_x - center_w // 2
    right = left + center_w
    # clamp
    if left < 0:
        left = 0
        right = min(w, center_w)
    if right > w:
        right = w
        left = max(0, w - center_w)

    # apply padding
    left = max(0, left - int(pad_px))
    right = min(w, right + int(pad_px))

    # final crop: top/bottom portion then horizontal slice
    if keep == 'bottom':
        top_y = max(0, h - top_h)
        crop = im.crop((left, top_y, right, h))
    else:
        crop = im.crop((left, 0, right, top_h))
    # ensure output directory exists (only if a dir component is present)
    outdir = os.path.dirname(output_path)
    if outdir:
        os.makedirs(outdir, exist_ok=True)
    crop.save(output_path)
    return output_path, crop.size


def main():
    p = argparse.ArgumentParser()
    p.add_argument('--input', '-i', required=True, help='Input image path')
    p.add_argument('--output', '-o', required=True, help='Output image path')
    p.add_argument('--top-fraction', type=float, default=0.5, help='Fraction of height to keep from top (0-1)')
    p.add_argument('--keep', choices=['top','bottom'], default='top', help='Keep top or bottom portion')
    p.add_argument('--center-fraction', type=float, default=0.6, help='Fraction of width to keep centered (0-1)')
    p.add_argument('--h-offset', type=float, default=0.0, help='Horizontal offset as fraction of width (-1..1)')
    p.add_argument('--pad-px', type=int, default=0, help='Extra padding in pixels to add/subtract from horizontal crop')
    args = p.parse_args()

    out_path, size = crop_focus(args.input, args.output, args.top_fraction, args.center_fraction, args.h_offset, args.pad_px, keep=args.keep)
    print(f'Saved: {out_path} size={size[0]}x{size[1]}')


if __name__ == '__main__':
    main()