import json
import os
import re

# 读取 event_encounters.json
with open("../src-tauri/resources/event_encounters.json", 'r', encoding='utf-8') as f:
    events = json.load(f)

# 更新每个有 choices 的事件
updated_count = 0
for event in events:
    if 'choices' in event and event['choices']:
        # 提取事件名称
        name = None
        if 'Localization' in event and 'Title' in event['Localization']:
            name = event['Localization']['Title'].get('Text', '')
        if not name:
            name = event.get('InternalName', '')
        
        clean_name = name.split(' (')[0].strip().replace("'", "").replace(" ", "")
        
        # 构建图片路径
        char_path = f"EncEvent_CHAR/ENC_Merchant_{clean_name}_Char.webp"
        bg_path = f"EncEvent_BG/ENC_Merchant_{clean_name}_Bg.webp"
        
        # 检查文件是否存在
        char_exists = os.path.exists(f"../src-tauri/resources/{char_path}")
        bg_exists = os.path.exists(f"../src-tauri/resources/{bg_path}")
        
        # 添加图片路径到事件对象
        event['image_paths'] = {}
        if char_exists:
            event['image_paths']['char'] = char_path
        if bg_exists:
            event['image_paths']['bg'] = bg_path
        
        updated_count += 1

# 保存更新后的 JSON
with open("../src-tauri/resources/event_encounters.json", 'w', encoding='utf-8') as f:
    json.dump(events, f, ensure_ascii=False, indent=2)

print(f"✓ 成功更新 {updated_count} 个事件的图片路径")
print(f"✓ 已保存到 event_encounters.json")
