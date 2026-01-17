#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
备份并压缩怪物角色图片
"""
import os
import shutil
from pathlib import Path
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
CHAR_DIR = ROOT / 'src-tauri' / 'resources' / 'images'
BACKUP_DIR = ROOT / 'src-tauri' / 'resources' / 'images_backup'

def get_dir_size(path):
    """计算目录总大小（字节）"""
    total = 0
    for entry in os.scandir(path):
        if entry.is_file():
            total += entry.stat().st_size
        elif entry.is_dir():
            total += get_dir_size(entry.path)
    return total

def format_size(bytes_size):
    """格式化字节大小"""
    for unit in ['B', 'KB', 'MB', 'GB']:
        if bytes_size < 1024.0:
            return f"{bytes_size:.2f} {unit}"
        bytes_size /= 1024.0
    return f"{bytes_size:.2f} TB"

def main():
    print("=" * 60)
    print("备份并压缩怪物图片")
    print("=" * 60)
    
    # 1. 备份原始文件夹
    if BACKUP_DIR.exists():
        print(f"删除旧备份: {BACKUP_DIR}")
        shutil.rmtree(BACKUP_DIR)
    
    print(f"\n备份原始文件到: {BACKUP_DIR}")
    shutil.copytree(CHAR_DIR, BACKUP_DIR)
    
    # 计算原始大小
    original_size = get_dir_size(BACKUP_DIR)
    print(f"原始文件夹大小: {format_size(original_size)}")
    
    # 2. 压缩 images_monster_char 中的所有图片
    print(f"\n开始压缩 {CHAR_DIR} 中的图片...")
    compressed_count = 0
    error_count = 0
    
    for img_file in CHAR_DIR.glob('*.*'):
        if img_file.suffix.lower() in ['.webp', '.png', '.jpg', '.jpeg']:
            try:
                # 打开图片
                img = Image.open(img_file)
                
                # 转换为 RGBA（保留透明通道）
                if img.mode != 'RGBA':
                    img = img.convert('RGBA')
                
                # 保存为 WebP，质量 80%
                output_path = img_file.with_suffix('.webp')
                img.save(output_path, 'WEBP', quality=80, method=6)
                
                # 如果原文件不是 webp，删除原文件
                if img_file.suffix.lower() != '.webp':
                    img_file.unlink()
                
                compressed_count += 1
                if compressed_count % 10 == 0:
                    print(f"  已压缩 {compressed_count} 张...")
                    
            except Exception as e:
                print(f"  ✗ 压缩失败 {img_file.name}: {e}")
                error_count += 1
    
    print(f"\n压缩完成: {compressed_count} 张成功, {error_count} 张失败")
    
    # 3. 计算压缩后大小
    compressed_size = get_dir_size(CHAR_DIR)
    print(f"压缩后文件夹大小: {format_size(compressed_size)}")
    
    # 4. 对比结果
    saved_size = original_size - compressed_size
    saved_percent = (saved_size / original_size * 100) if original_size > 0 else 0
    
    print("\n" + "=" * 60)
    print("压缩结果对比")
    print("=" * 60)
    print(f"原始大小:   {format_size(original_size)}")
    print(f"压缩后大小: {format_size(compressed_size)}")
    print(f"节省空间:   {format_size(saved_size)} ({saved_percent:.1f}%)")
    print("=" * 60)
    print(f"\n备份保存在: {BACKUP_DIR}")
    print(f"压缩文件在: {CHAR_DIR}")

if __name__ == '__main__':
    main()
