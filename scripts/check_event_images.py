import json
import os

# 读取 event_encounters.json
with open("../src-tauri/resources/event_encounters.json", 'r', encoding='utf-8') as f:
    events = json.load(f)

# 统计有 choices 的事件
events_with_choices = []
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
        
        events_with_choices.append({
            'id': event.get('Id'),
            'name': name,
            'clean_name': clean_name,
            'char_path': char_path if char_exists else None,
            'bg_path': bg_path if bg_exists else None,
            'choices_count': len(event['choices'])
        })

print(f"找到 {len(events_with_choices)} 个包含 choices 的事件\n")
print("="*80)

# 统计
has_both = sum(1 for e in events_with_choices if e['char_path'] and e['bg_path'])
has_char_only = sum(1 for e in events_with_choices if e['char_path'] and not e['bg_path'])
has_bg_only = sum(1 for e in events_with_choices if not e['char_path'] and e['bg_path'])
has_none = sum(1 for e in events_with_choices if not e['char_path'] and not e['bg_path'])

print("统计：")
print(f"  ✓ 同时有 Char 和 Bg: {has_both}")
print(f"  ✓ 只有 Char: {has_char_only}")
print(f"  ✓ 只有 Bg: {has_bg_only}")
print(f"  ✗ 都没有: {has_none}")

print("\n" + "="*80)
print("详细列表：\n")

for i, e in enumerate(events_with_choices, 1):
    status_char = "✓" if e['char_path'] else "✗"
    status_bg = "✓" if e['bg_path'] else "✗"
    print(f"{i:2}. {e['name']:40} [{status_char} Char] [{status_bg} Bg] ({e['choices_count']} 个选项)")

if has_none > 0:
    print("\n" + "="*80)
    print("缺失图片的事件：\n")
    for e in events_with_choices:
        if not e['char_path'] and not e['bg_path']:
            print(f"  - {e['name']} (期望: {e['clean_name']})")
