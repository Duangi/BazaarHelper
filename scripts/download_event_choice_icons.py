import json
import os
import requests  # type: ignore
from PIL import Image
from io import BytesIO

# 读取 event_encounters.json
with open("../src-tauri/resources/event_encounters.json", 'r', encoding='utf-8') as f:
    events = json.load(f)

# 创建目录
icon_dir = "../src-tauri/resources/EncEvent_Icons"
os.makedirs(icon_dir, exist_ok=True)

# 收集所有需要下载的 icon_url
icons_to_download = {}
for event in events:
    if 'choices' in event and event['choices']:
        for choice in event['choices']:
            if 'icon_url' in choice and choice['icon_url']:
                icon_url = choice['icon_url']
                # 从URL中提取文件名
                icon_id = choice.get('icon', icon_url.split('/')[-1].split('.')[0])
                if icon_id and icon_url not in icons_to_download.values():
                    icons_to_download[icon_id] = icon_url

print(f"找到 {len(icons_to_download)} 个唯一的图标需要下载\n")

# 下载图标
downloaded = 0
skipped = 0
failed = []

for icon_id, icon_url in icons_to_download.items():
    output_path = os.path.join(icon_dir, f"{icon_id}.webp")
    
    # 如果文件已存在，跳过
    if os.path.exists(output_path):
        print(f"跳过 {icon_id} (已存在)")
        skipped += 1
        continue
    
    try:
        # 下载图片
        print(f"下载 {icon_id} 从 {icon_url}...")
        response = requests.get(icon_url, timeout=10)
        response.raise_for_status()
        
        # 打开图片并转换为 WebP
        img = Image.open(BytesIO(response.content))
        img.save(output_path, 'WEBP', quality=80)
        
        print(f"  ✓ 已保存为 {output_path}")
        downloaded += 1
        
    except Exception as e:
        print(f"  ✗ 失败: {e}")
        failed.append((icon_id, icon_url, str(e)))

print("\n" + "="*80)
print("下载完成！")
print(f"  ✓ 下载: {downloaded}")
print(f"  - 跳过: {skipped}")
print(f"  ✗ 失败: {len(failed)}")

if failed:
    print("\n失败列表：")
    for icon_id, icon_url, error in failed:
        print(f"  - {icon_id}: {error}")
        print(f"    URL: {icon_url}")
