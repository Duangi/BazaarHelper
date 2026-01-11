#!/usr/bin/env python
"""
compare_retired.py
Compare two images (default: src-tauri/src/退休人员.png and src-tauri/resources/images/退休人员.jpg)
Outputs JSON with pHash distance, SSIM (if available), MSE, template match scores and ORB feature-match stats.
"""
import os
import sys
import json
import argparse
import cv2
import numpy as np
from PIL import Image
import imagehash


def load_image(path):
    if not os.path.exists(path):
        return None
    try:
        with open(path, 'rb') as f:
            data = f.read()
        img = cv2.imdecode(np.frombuffer(data, np.uint8), cv2.IMREAD_COLOR)
        if img is not None:
            return img
    except Exception:
        pass
    return cv2.imread(path)


def phash_distance(a_path, b_path):
    a = load_image(a_path)
    b = load_image(b_path)
    if a is None or b is None:
        return None
    a_rgb = cv2.cvtColor(a, cv2.COLOR_BGR2RGB)
    b_rgb = cv2.cvtColor(b, cv2.COLOR_BGR2RGB)
    ha = imagehash.phash(Image.fromarray(a_rgb))
    hb = imagehash.phash(Image.fromarray(b_rgb))
    return int(ha - hb)


def compute_mse(a_gray, b_gray):
    a = a_gray.astype('float32')
    b = b_gray.astype('float32')
    if a.shape != b.shape:
        b = cv2.resize(b, (a.shape[1], a.shape[0]), interpolation=cv2.INTER_AREA)
    err = np.mean((a - b) ** 2)
    return float(err)


def compute_ssim(a_gray, b_gray):
    try:
        from skimage.metrics import structural_similarity as ssim_func
        if a_gray.shape != b_gray.shape:
            b_gray = cv2.resize(b_gray, (a_gray.shape[1], a_gray.shape[0]), interpolation=cv2.INTER_AREA)
        return float(ssim_func(a_gray, b_gray))
    except Exception:
        return None


def template_match_score(a_path, b_path):
    a = load_image(a_path)
    b = load_image(b_path)
    if a is None or b is None:
        return [None, None]
    a_gray = cv2.cvtColor(a, cv2.COLOR_BGR2GRAY)
    b_gray = cv2.cvtColor(b, cv2.COLOR_BGR2GRAY)
    # match b inside a (resized if necessary)
    def match(A_gray, B_gray):
        if B_gray.shape[0] > A_gray.shape[0] or B_gray.shape[1] > A_gray.shape[1]:
            B_res = cv2.resize(B_gray, (A_gray.shape[1], A_gray.shape[0]), interpolation=cv2.INTER_AREA)
        else:
            B_res = B_gray
        try:
            res = cv2.matchTemplate(A_gray, B_res, cv2.TM_CCOEFF_NORMED)
            _, maxv, _, _ = cv2.minMaxLoc(res)
            return float(maxv)
        except Exception:
            return None

    s1 = match(a_gray, b_gray)
    s2 = match(b_gray, a_gray)
    return [s1, s2]


def orb_match_stats(a_path, b_path, ratio_thresh=0.75):
    a = load_image(a_path)
    b = load_image(b_path)
    if a is None or b is None:
        return None
    gray1 = cv2.cvtColor(a, cv2.COLOR_BGR2GRAY)
    gray2 = cv2.cvtColor(b, cv2.COLOR_BGR2GRAY)
    orb = cv2.ORB_create(500)
    k1, d1 = orb.detectAndCompute(gray1, None)
    k2, d2 = orb.detectAndCompute(gray2, None)
    if d1 is None or d2 is None or len(d1) == 0 or len(d2) == 0:
        return {'k1': len(k1), 'k2': len(k2), 'good_matches': 0, 'match_ratio': 0.0}
    bf = cv2.BFMatcher(cv2.NORM_HAMMING, crossCheck=False)
    try:
        matches = bf.knnMatch(d1, d2, k=2)
    except Exception:
        return {'k1': len(k1), 'k2': len(k2), 'good_matches': 0, 'match_ratio': 0.0}
    good = []
    for m_n in matches:
        if len(m_n) != 2:
            continue
        m, n = m_n
        if m.distance < ratio_thresh * n.distance:
            good.append(m)
    good_count = len(good)
    denom = max(1, min(len(k1), len(k2)))
    return {'k1': len(k1), 'k2': len(k2), 'good_matches': good_count, 'match_ratio': float(good_count) / denom}


def find_default_paths():
    repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
    candidates = [
        os.path.join(repo_root, 'src-tauri', 'src', '退休人员.png'),
        os.path.join(repo_root, 'src-tauri', 'resources', 'images', '退休人员.jpg')
    ]
    return candidates


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--a', help='First image path (png)', default=None)
    parser.add_argument('--b', help='Second image path (jpg)', default=None)
    parser.add_argument('--out', help='Write JSON to file instead of stdout', default=None)
    args = parser.parse_args()

    a_path = args.a
    b_path = args.b
    if not a_path or not b_path:
        a_default, b_default = find_default_paths()
        a_path = a_path or a_default
        b_path = b_path or b_default

    a_path = os.path.normpath(a_path)
    b_path = os.path.normpath(b_path)

    res = {'a': a_path, 'b': b_path}
    a_img = load_image(a_path)
    b_img = load_image(b_path)
    if a_img is None or b_img is None:
        res['error'] = {'a_exists': os.path.exists(a_path), 'b_exists': os.path.exists(b_path)}
        out = json.dumps(res, ensure_ascii=False, indent=2)
        if args.out:
            open(args.out, 'w', encoding='utf-8').write(out)
        else:
            print(out)
        sys.exit(2)

    # pHash
    try:
        phash = phash_distance(a_path, b_path)
        res['phash_distance'] = phash
    except Exception as e:
        res['phash_distance'] = None

    # grayscale and pixel metrics
    a_gray = cv2.cvtColor(a_img, cv2.COLOR_BGR2GRAY)
    b_gray = cv2.cvtColor(b_img, cv2.COLOR_BGR2GRAY)
    res['mse'] = compute_mse(a_gray, b_gray)
    res['ssim'] = compute_ssim(a_gray, b_gray)

    # template scores
    res['template_scores'] = template_match_score(a_path, b_path)

    # ORB stats
    res['orb'] = orb_match_stats(a_path, b_path)

    out = json.dumps(res, ensure_ascii=False, indent=2)
    if args.out:
        open(args.out, 'w', encoding='utf-8').write(out)
    else:
        print(out)


if __name__ == '__main__':
    main()
