import json
import os
import shutil

# 路径配置
event_json_path = "../src-tauri/resources/event_encounters.json"
source_assets_dir = r"D:\TheBazaarData\Assets"
target_char_dir = "../src-tauri/resources/EncEvent_CHAR"
target_bg_dir = "../src-tauri/resources/EncEvent_BG"

# 创建目标文件夹
os.makedirs(target_char_dir, exist_ok=True)
os.makedirs(target_bg_dir, exist_ok=True)

# 读取 JSON
with open(event_json_path, 'r', encoding='utf-8') as f:
    events = json.load(f)

# 提取有 choices 的事件名称
event_names = []
for event in events:
    if 'choices' in event and event['choices']:
        # 尝试多个来源获取名称
        name = None
        clean_name = None
        
        # 优先使用 Localization.Title.Text
        if 'Localization' in event and 'Title' in event['Localization']:
            name = event['Localization']['Title'].get('Text', '')
        
        # 如果没有，使用 InternalName
        if not name:
            name = event.get('InternalName', '')
        
        # 最后尝试 name 字段
        if not name:
            name = event.get('name', '')
            
        if name:
            # 去掉括号及其内容和前面的空格
            clean_name = name.split(' (')[0].strip()
            # 处理特殊字符（如单引号、空格等）
            # 移除特殊符号，保留字母数字
            clean_name = clean_name.replace("'", "").replace(" ", "")
            event_names.append((name, clean_name))

print(f"找到 {len(event_names)} 个包含 choices 的事件")

# 查找并复制图片
found_char = []
found_bg = []
missing_char = []
missing_bg = []

for original_name, clean_name in event_names:
    # 构造期望的文件名模式（支持多种大小写变体和前缀）
    prefixes = ["ENC_Merchant_", "ENC_Event_"]
    char_patterns = []
    bg_patterns = []
    
    for prefix in prefixes:
        char_patterns.extend([
            f"{prefix}{clean_name}_Char.png",
            f"{prefix}{clean_name}_CHAR.png",
            f"{prefix}{clean_name}_char.png",
        ])
        bg_patterns.extend([
            f"{prefix}{clean_name}_Bg.png",
            f"{prefix}{clean_name}_BG.png",
            f"{prefix}{clean_name}_bg.png",
        ])
    
    # 在 Assets 目录下搜索
    char_found = False
    bg_found = False
    
    for root, dirs, files in os.walk(source_assets_dir):
        for file in files:
            # 检查是否匹配任何 Char 模式
            if any(file == pattern for pattern in char_patterns):
                src_path = os.path.join(root, file)
                # 统一保存为标准命名格式
                dst_filename = f"ENC_Merchant_{clean_name}_Char.png"
                dst_path = os.path.join(target_char_dir, dst_filename)
                shutil.copy2(src_path, dst_path)
                found_char.append((original_name, file))
                char_found = True
                print(f"✓ 复制 Char: {file} -> {dst_filename}")
            
            # 检查是否匹配任何 Bg 模式
            elif any(file == pattern for pattern in bg_patterns):
                src_path = os.path.join(root, file)
                # 统一保存为标准命名格式
                dst_filename = f"ENC_Merchant_{clean_name}_Bg.png"
                dst_path = os.path.join(target_bg_dir, dst_filename)
                shutil.copy2(src_path, dst_path)
                found_bg.append((original_name, file))
                bg_found = True
                print(f"✓ 复制 Bg: {file} -> {dst_filename}")
    
    if not char_found:
        missing_char.append((original_name, char_patterns[0]))
    
    if not bg_found:
        missing_bg.append((original_name, bg_patterns[0]))

# 输出统计
print("\n" + "="*60)
print("统计结果:")
print(f"  找到 Char 图片: {len(found_char)} 个")
print(f"  找到 Bg 图片: {len(found_bg)} 个")
print(f"  缺失 Char 图片: {len(missing_char)} 个")
print(f"  缺失 Bg 图片: {len(missing_bg)} 个")

if missing_char:
    print("\n缺失的 Char 图片:")
    for orig_name, pattern in missing_char:
        print(f"  - {orig_name} (期望: {pattern})")

if missing_bg:
    print("\n缺失的 Bg 图片:")
    for orig_name, pattern in missing_bg:
        print(f"  - {orig_name} (期望: {pattern})")

print("\n完成！")
