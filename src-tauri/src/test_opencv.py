import os
import sys
import json
import argparse
import time
import cv2
import numpy as np
from PIL import Image
import imagehash


def load_image(path):
    try:
        with open(path, 'rb') as f:
            data = f.read()
        img = cv2.imdecode(np.frombuffer(data, np.uint8), cv2.IMREAD_COLOR)
        if img is not None:
            return img
    except Exception:
        pass
    return cv2.imread(path)


def build_hash_db(db_dir):
    db = []
    exts = ('.png', '.jpg', '.jpeg', '.bmp', '.webp')
    for root, _, files in os.walk(db_dir):
        for fn in files:
            if not fn.lower().endswith(exts):
                continue
            p = os.path.join(root, fn)
            try:
                h = imagehash.phash(Image.open(p))
                db.append({'path': p, 'hash': h})
            except Exception:
                continue
    return db


def phash_candidates(input_path, db_dir, phash_thresh=10, top_k=50):
    inp = load_image(input_path)
    if inp is None:
        return {"error": "failed_load_input"}
    inp_rgb = cv2.cvtColor(inp, cv2.COLOR_BGR2RGB)
    inp_pil = Image.fromarray(inp_rgb)
    inp_hash = imagehash.phash(inp_pil)

    db = build_hash_db(db_dir)
    candidates = []
    for item in db:
        try:
            d = inp_hash - item['hash']
        except Exception:
            continue
        if d <= phash_thresh:
            candidates.append((d, item['path']))

    candidates.sort(key=lambda x: x[0])
    return [p for _, p in candidates[:top_k]]


def verify_candidates(input_path, candidates, tpl_thresh=0.86, downscale=1.0):
    inp = load_image(input_path)
    if inp is None:
        return {"error": "failed_load_input"}
    if downscale != 1.0:
        inp = cv2.resize(inp, (0, 0), fx=downscale, fy=downscale, interpolation=cv2.INTER_AREA)
    inp_gray = cv2.cvtColor(inp, cv2.COLOR_BGR2GRAY)
    H, W = inp_gray.shape[:2]

    matches = []
    for p in candidates:
        tpl = load_image(p)
        if tpl is None:
            continue
        tpl_gray = cv2.cvtColor(tpl, cv2.COLOR_BGR2GRAY)
        th, tw = tpl_gray.shape[:2]
        if th > H or tw > W:
            # skip templates larger than input; could resize instead
            continue
        try:
            res = cv2.matchTemplate(inp_gray, tpl_gray, cv2.TM_CCOEFF_NORMED)
            _, maxv, _, maxloc = cv2.minMaxLoc(res)
        except Exception:
            continue
        if maxv >= tpl_thresh:
            matches.append({
                'file': os.path.relpath(p).replace('\\', '/'),
                'score': float(maxv),
                'location': {'x': int(maxloc[0]), 'y': int(maxloc[1])}
            })

    matches.sort(key=lambda x: x['score'], reverse=True)
    return matches


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--input', '-i', help='Input image path (default: search for test.jpg in repo root)')
    parser.add_argument('--db-dir', default=os.path.normpath(os.path.join(os.path.dirname(__file__), '..', 'resources', 'images')),
                        help='Directory with template images')
    parser.add_argument('--phash-thresh', type=int, default=10)
    parser.add_argument('--tpl-thresh', type=float, default=0.86)
    parser.add_argument('--downscale', type=float, default=1.0)
    parser.add_argument('--top-k', type=int, default=50)
    parser.add_argument('--match-db', action='store_true', help='Run DB matching (pHash prefilter + template verify)')
    args = parser.parse_args()

    inp = args.input
    if inp is None:
        cwd = os.getcwd()
        candidates = []
        for root, _, files in os.walk(cwd):
            for f in files:
                if f.lower() == 'test.jpg':
                    candidates.append(os.path.join(root, f))
        inp = candidates[0] if candidates else None

    if args.match_db:
        if inp is None:
            print(json.dumps({"error": "no_input_found"}))
            sys.exit(2)
        start = time.time()
        cand = phash_candidates(inp, args.db_dir, phash_thresh=args.phash_thresh, top_k=args.top_k)
        cand_time = time.time()
        matches = verify_candidates(inp, cand, tpl_thresh=args.tpl_thresh, downscale=args.downscale)
        end = time.time()
        out = {
            'input': inp,
            'db_dir': args.db_dir,
            'phash_threshold': args.phash_thresh,
            'template_threshold': args.tpl_thresh,
            'candidates_count': len(cand),
            'candidates': cand,
            'matches': matches,
            'timings': {
                'phash_seconds': round(cand_time - start, 3),
                'verify_seconds': round(end - cand_time, 3),
                'total_seconds': round(end - start, 3),
            }
        }
        print(json.dumps(out, ensure_ascii=False, indent=2))
        return

    # fallback: simple image load test
    base = os.path.dirname(__file__)
    default_img = os.path.normpath(os.path.join(base, '..', 'resources', 'images', '巨蚊.jpg'))
    test_img = inp or default_img
    img = load_image(test_img)
    if img is None:
        print('FAILED to load image:', test_img)
        sys.exit(2)
    print('cv2 version:', cv2.__version__)
    print('Loaded image shape:', img.shape)
    gray = cv2.cvtColor(img, cv2.COLOR_BGR2GRAY)
    print('Gray mean:', float(gray.mean()))


if __name__ == '__main__':
    try:
        main()
    except Exception as e:
        print('Error during test:', e)
        sys.exit(1)
import os
import sys
import json
import argparse
import cv2
import numpy as np


def load_image(path):
    try:
        with open(path, 'rb') as f:
            data = f.read()
        img = cv2.imdecode(np.frombuffer(data, np.uint8), cv2.IMREAD_COLOR)
        if img is not None:
            return img
    except Exception:
        pass
    return cv2.imread(path)


def match_db(input_path, db_dir, threshold=0.86, downscale=1.0, scales=(1.0, 0.8, 0.6)):
    inp = load_image(input_path)
    if inp is None:
        print('FAILED to load input image:', input_path)
        return {"error": "failed_load_input"}

    if downscale != 1.0:
        inp = cv2.resize(inp, (0, 0), fx=downscale, fy=downscale, interpolation=cv2.INTER_AREA)

    inp_gray = cv2.cvtColor(inp, cv2.COLOR_BGR2GRAY)
    inp_h, inp_w = inp_gray.shape[:2]

    matches = []
    exts = ('.png', '.jpg', '.jpeg', '.bmp', '.webp')
    for root, _, files in os.walk(db_dir):
        for fn in files:
            if not fn.lower().endswith(exts):
                continue
            tpl_path = os.path.join(root, fn)
            tpl = load_image(tpl_path)
            if tpl is None:
                continue
            best_val = -1.0
            best_scale = None
            for s in scales:
                tpl_resized = tpl
                if s != 1.0:
                    tpl_resized = cv2.resize(tpl, (0, 0), fx=s, fy=s, interpolation=cv2.INTER_AREA)
                tpl_gray = cv2.cvtColor(tpl_resized, cv2.COLOR_BGR2GRAY)
                th, tw = tpl_gray.shape[:2]
                if th >= inp_h or tw >= inp_w:
                    continue
                try:
                    res = cv2.matchTemplate(inp_gray, tpl_gray, cv2.TM_CCOEFF_NORMED)
                    min_val, max_val, min_loc, max_loc = cv2.minMaxLoc(res)
                except Exception:
                    continue
                if max_val > best_val:
                    best_val = float(max_val)
                    best_scale = s
                if best_val >= threshold:
                    break

            if best_val >= threshold:
                matches.append({
                    "file": os.path.relpath(tpl_path).replace('\\', '/'),
                    "score": best_val,
                    "scale": best_scale,
                })

    matches.sort(key=lambda x: x['score'], reverse=True)
    return {"input": input_path, "matches": matches}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--input', '-i', help='Input image path (default: search for test.jpg in repo root)')
    parser.add_argument('--db-dir', default=os.path.normpath(os.path.join(os.path.dirname(__file__), '..', 'resources', 'images')),
                        help='Directory with template images')
    parser.add_argument('--threshold', type=float, default=0.86)
    parser.add_argument('--downscale', type=float, default=1.0)
    parser.add_argument('--match-db', action='store_true', help='Run DB matching')
    args = parser.parse_args()

    # locate input
    inp = args.input
    if inp is None:
        # look for test.jpg in repo
        cwd = os.getcwd()
        candidates = []
        for root, _, files in os.walk(cwd):
            for f in files:
                if f.lower() == 'test.jpg':
                    candidates.append(os.path.join(root, f))
        inp = candidates[0] if candidates else None

    if args.match_db:
        if inp is None:
            print(json.dumps({"error": "no_input_found"}))
            sys.exit(2)
        out = match_db(inp, args.db_dir, threshold=args.threshold, downscale=args.downscale)
        print(json.dumps(out, ensure_ascii=False, indent=2))
        return

    # backward compatible single-image test
    base = os.path.dirname(__file__)
    default_img = os.path.normpath(os.path.join(base, '..', 'resources', 'images', '巨蚊.jpg'))
    test_img = inp or default_img
    img = load_image(test_img)
    if img is None:
        print('FAILED to load image:', test_img)
        sys.exit(2)
    print('cv2 version:', cv2.__version__)
    print('Loaded image shape:', img.shape)
    gray = cv2.cvtColor(img, cv2.COLOR_BGR2GRAY)
    print('Gray mean:', float(gray.mean()))


if __name__ == '__main__':
    try:
        main()
    except Exception as e:
        print('Error during test:', e)
        sys.exit(1)
