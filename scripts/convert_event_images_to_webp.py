import os
from PIL import Image

# 路径配置
char_dir = "../src-tauri/resources/EncEvent_CHAR"
bg_dir = "../src-tauri/resources/EncEvent_BG"

def convert_to_webp(directory):
    """转换目录中的所有PNG图片为WebP格式"""
    if not os.path.exists(directory):
        print(f"目录不存在: {directory}")
        return
    
    converted_count = 0
    for filename in os.listdir(directory):
        if filename.endswith('.png'):
            png_path = os.path.join(directory, filename)
            webp_path = os.path.join(directory, filename.replace('.png', '.webp'))
            
            try:
                img = Image.open(png_path)
                img.save(webp_path, 'WEBP', quality=80)
                print(f"✓ 转换: {filename} -> {filename.replace('.png', '.webp')}")
                converted_count += 1
                
                # 删除原PNG文件
                os.remove(png_path)
            except Exception as e:
                print(f"✗ 转换失败 {filename}: {e}")
    
    return converted_count

print("="*60)
print("转换 Char 图片...")
print("="*60)
char_count = convert_to_webp(char_dir)

print("\n" + "="*60)
print("转换 Bg 图片...")
print("="*60)
bg_count = convert_to_webp(bg_dir)

print("\n" + "="*60)
print(f"转换完成！")
print(f"  Char: {char_count} 个")
print(f"  Bg: {bg_count} 个")
print("="*60)
