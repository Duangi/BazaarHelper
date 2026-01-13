#!/usr/bin/env python3
"""
compare.py
Compute matching confidence between three slices and two templates using ORB feature matching.

Usage:
  python compare.py --slices final_left.jpg final_mid.jpg final_right.jpg --templates 巨蚊.jpg 恶蚊.jpg
"""
import argparse
import cv2
import os
import numpy as np


def load_gray(path):
    img = cv2.imread(path, cv2.IMREAD_GRAYSCALE)
    if img is None:
        raise FileNotFoundError(path)
    return img


def orb_confidence(img1, img2, nfeatures=500, ratio=0.75):
    orb = cv2.ORB_create(nfeatures)
    kp1, des1 = orb.detectAndCompute(img1, None)
    kp2, des2 = orb.detectAndCompute(img2, None)

    if des1 is None or des2 is None or len(kp1) == 0 or len(kp2) == 0:
        return 0.0

    bf = cv2.BFMatcher(cv2.NORM_HAMMING)
    matches = bf.knnMatch(des1, des2, k=2)

    good = 0
    for m_n in matches:
        if len(m_n) != 2:
            continue
        m, n = m_n
        if m.distance < ratio * n.distance:
            good += 1

    # normalize by smaller number of keypoints to keep score in [0,1]
    denom = max(1, min(len(kp1), len(kp2)))
    return float(good) / denom


def main():
    p = argparse.ArgumentParser()
    p.add_argument('--slices', '-s', nargs=3, required=True, help='Three slice image paths (left mid right)')
    p.add_argument('--templates', '-t', nargs=2, required=True, help='Two template image paths')
    args = p.parse_args()

    slices = args.slices
    templates = args.templates

    # load images
    imgs = [load_gray(p) for p in slices]
    tmpls = [load_gray(p) for p in templates]

    results = {}
    for i, s_path in enumerate(slices):
        for j, t_path in enumerate(templates):
            score = orb_confidence(imgs[i], tmpls[j])
            results[(os.path.basename(s_path), os.path.basename(t_path))] = score

    # print results in a simple table
    for (s, t), score in results.items():
        print(f'{s} vs {t}: {score:.4f}')


if __name__ == '__main__':
    main()
