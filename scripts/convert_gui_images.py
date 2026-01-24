from PIL import Image
import os

# 转换images_GUI目录下的所有PNG到WebP
gui_dir = "../src-tauri/resources/images_GUI"
for filename in os.listdir(gui_dir):
    if filename.endswith('.png'):
        png_path = os.path.join(gui_dir, filename)
        webp_path = os.path.join(gui_dir, filename.replace('.png', '.webp'))
        
        img = Image.open(png_path)
        img.save(webp_path, 'WEBP', quality=80)
        print(f"Converted {filename} to WebP")

print("All images converted successfully!")
