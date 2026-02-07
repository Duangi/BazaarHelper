#!/usr/bin/env python3
"""
将 images_monster_char 文件夹中的 PNG 图片转换为 80% 质量的 WebP 格式
"""
import os
from pathlib import Path
from PIL import Image

def convert_png_to_webp(png_path: Path, webp_path: Path, quality: int = 80):
    """
    将 PNG 文件转换为 WebP 格式
    
    Args:
        png_path: PNG 文件路径
        webp_path: WebP 输出路径
        quality: WebP 质量 (0-100)，默认 80
    """
    try:
        # 打开 PNG 图片
        with Image.open(png_path) as img:
            # 转换为 RGBA（如果不是的话，保留透明度）
            if img.mode != 'RGBA':
                if img.mode == 'P' and 'transparency' in img.info:
                    img = img.convert('RGBA')
                elif img.mode == 'RGB':
                    img = img.convert('RGBA')
            
            # 保存为 WebP
            img.save(webp_path, 'WEBP', quality=quality, method=6)
            
            # 获取文件大小对比
            original_size = png_path.stat().st_size
            webp_size = webp_path.stat().st_size
            compression_ratio = (1 - webp_size / original_size) * 100
            
            print(f"✓ {png_path.name:50s} -> {webp_path.name:50s} "
                  f"({original_size/1024:6.1f}KB -> {webp_size/1024:6.1f}KB, "
                  f"压缩 {compression_ratio:.1f}%)")
            return True
    except Exception as e:
        print(f"✗ {png_path.name}: {e}")
        return False

def main():
    # 获取项目根目录
    script_dir = Path(__file__).parent
    project_root = script_dir.parent
    
    # images_monster_char 文件夹路径
    source_dir = project_root / "src-tauri" / "resources" / "images_monster_char"
    
    if not source_dir.exists():
        print(f"错误: 源文件夹不存在: {source_dir}")
        return
    
    print(f"源文件夹: {source_dir}")
    print(f"质量设置: 80%")
    print("=" * 120)
    
    # 查找所有 PNG 文件
    png_files = list(source_dir.glob("*.png")) + list(source_dir.glob("*.PNG"))
    
    if not png_files:
        print("没有找到 PNG 文件")
        return
    
    print(f"找到 {len(png_files)} 个 PNG 文件\n")
    
    success_count = 0
    fail_count = 0
    
    for png_path in sorted(png_files):
        # 生成 WebP 文件名（同一目录）
        webp_path = png_path.with_suffix('.webp')
        
        # 如果 WebP 已存在，跳过
        if webp_path.exists():
            print(f"⊘ {png_path.name:50s} (WebP 已存在，跳过)")
            continue
        
        # 转换
        if convert_png_to_webp(png_path, webp_path, quality=80):
            success_count += 1
        else:
            fail_count += 1
    
    print("\n" + "=" * 120)
    print(f"转换完成: 成功 {success_count} 个, 失败 {fail_count} 个")
    
    if success_count > 0:
        print(f"\n提示: 转换成功后，可以手动删除原始 PNG 文件（如果不需要的话）")

if __name__ == "__main__":
    main()
