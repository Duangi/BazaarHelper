import os
import requests  # type: ignore
from PIL import Image
from io import BytesIO

# 路径配置
char_dir = "../src-tauri/resources/EncEvent_CHAR"
bg_dir = "../src-tauri/resources/EncEvent_BG"

# 确保目录存在
os.makedirs(char_dir, exist_ok=True)
os.makedirs(bg_dir, exist_ok=True)

# 缺失的图片列表
missing_images = {
    "char": [
        "Dflek", "TakeFlight", "Seminar", "Cymon", "ReflectingPool",
        "ThievesGuild", "AdvancedTraining", "JulesCafe", "TinyFurryMonster",
        "ArtisanDunes", "DooleysCrib", "Flambe", "EatingContest",
        "StreetFestival", "Botul", "AStrangeMushroom", "TheLostCrate",
        "JungleRuins", "TheDocks"
    ],
    "bg": [
        "Dflek", "TakeFlight", "Seminar", "Cymon", "ReflectingPool",
        "TinyFurryMonster", "DooleysCrib", "Flambe", "Botul",
        "AStrangeMushroom", "TheLostCrate"
    ]
}

def download_image(url, save_path):
    """从URL下载图片并保存为WebP格式"""
    try:
        response = requests.get(url, timeout=30)
        response.raise_for_status()
        
        # 打开图片
        img = Image.open(BytesIO(response.content))
        
        # 转换为WebP
        img.save(save_path, 'WEBP', quality=80)
        return True
    except Exception as e:
        print(f"  ✗ 下载失败: {e}")
        return False

def process_missing_images():
    """处理缺失的图片"""
    print("="*60)
    print("下载缺失的事件图片")
    print("="*60)
    print("\n提示：如果某个图片不存在，直接按回车跳过")
    print("输入 'q' 或 'quit' 退出程序\n")
    
    # 处理 Char 图片
    print("\n" + "="*60)
    print("Char 图片 (角色图)")
    print("="*60)
    for name in missing_images["char"]:
        filename = f"ENC_Merchant_{name}_Char.webp"
        save_path = os.path.join(char_dir, filename)
        
        print(f"\n[Char] {name}")
        url = input("  请输入 WebP 图片地址: ").strip()
        
        if url.lower() in ['q', 'quit']:
            print("\n退出程序")
            return
        
        if not url:
            print("  - 跳过")
            continue
        
        print("  下载中...")
        if download_image(url, save_path):
            print(f"  ✓ 已保存: {filename}")
        else:
            print("  ✗ 保存失败")
    
    # 处理 Bg 图片
    print("\n" + "="*60)
    print("Bg 图片 (背景图)")
    print("="*60)
    for name in missing_images["bg"]:
        filename = f"ENC_Merchant_{name}_Bg.webp"
        save_path = os.path.join(bg_dir, filename)
        
        print(f"\n[Bg] {name}")
        url = input("  请输入 WebP 图片地址: ").strip()
        
        if url.lower() in ['q', 'quit']:
            print("\n退出程序")
            return
        
        if not url:
            print("  - 跳过")
            continue
        
        print("  下载中...")
        if download_image(url, save_path):
            print(f"  ✓ 已保存: {filename}")
        else:
            print("  ✗ 保存失败")
    
    print("\n" + "="*60)
    print("完成！")
    print("="*60)

if __name__ == "__main__":
    process_missing_images()
