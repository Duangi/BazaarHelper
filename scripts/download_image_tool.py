import os
import requests
from PIL import Image
from io import BytesIO

def download_and_convert():
    # 目标目录
    target_dir = r"D:\Projects\BazaarHelper\src-tauri\resources\images"
    
    if not os.path.exists(target_dir):
        print(f"Error: 目标目录不存在 - {target_dir}")
        return

    # 获取用户输入
    filename = input("请输入需要保存的文件名 (不含后缀，例如 '幽灵'): ").strip()
    if not filename:
        print("文件名不能为空")
        return

    url = input("请输入图片的 WebP 链接: ").strip()
    if not url:
        print("链接不能为空")
        return

    try:
        print(f"正在下载: {url}...")
        response = requests.get(url, timeout=30)
        response.raise_for_status()

        # 使用 Pillow 打开图片
        img = Image.open(BytesIO(response.content))
        
        # 转换为 RGB 格式 (JPG 不支持透明度)
        if img.mode in ("RGBA", "P"):
            img = img.convert("RGB")
        
        # 拼接最终路径
        save_path = os.path.join(target_dir, f"{filename}.jpg")
        
        # 保存为 JPG
        img.save(save_path, "JPEG", quality=95)
        print(f"成功保存到: {save_path}")

    except Exception as e:
        print(f"处理失败: {e}")

if __name__ == "__main__":
    # 检查依赖
    try:
        import requests
        from PIL import Image
    except ImportError:
        print("缺少依赖，请先运行: pip install requests Pillow")
    else:
        # 循环运行，方便连续下载
        while True:
            download_and_convert()
            cont = input("\n按回车键继续下载下一个，输入 'q' 退出: ").lower()
            if cont == 'q':
                break
