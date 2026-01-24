import json
import cv2
import os
import base64

# 读取 event_encounters.json
with open("../src-tauri/resources/event_encounters.json", 'r', encoding='utf-8') as f:
    events = json.load(f)

# 初始化 ORB 检测器
orb = cv2.ORB_create(nfeatures=500, scaleFactor=1.2, nlevels=8, edgeThreshold=15, firstLevel=0, WTA_K=2, scoreType=cv2.ORB_HARRIS_SCORE, patchSize=31, fastThreshold=20)

print("开始提取事件图片特征...")
print("="*80)

updated_count = 0
failed_list = []

for event in events:
    if 'image_paths' in event and event['image_paths'].get('char'):
        char_path = os.path.join("../src-tauri/resources", event['image_paths']['char'])
        
        # 提取事件名称
        name = None
        if 'Localization' in event and 'Title' in event['Localization']:
            name = event['Localization']['Title'].get('Text', '')
        if not name:
            name = event.get('InternalName', '')
        
        # 读取图片
        if not os.path.exists(char_path):
            print(f"✗ {name}: 图片不存在 {char_path}")
            failed_list.append((name, "图片文件不存在"))
            continue
        
        img = cv2.imread(char_path, cv2.IMREAD_GRAYSCALE)
        if img is None:
            print(f"✗ {name}: 无法读取图片")
            failed_list.append((name, "无法读取图片"))
            continue
        
        # 提取 ORB 特征
        keypoints, descriptors = orb.detectAndCompute(img, None)
        
        if descriptors is None or len(descriptors) == 0:
            print(f"✗ {name}: 未检测到特征")
            failed_list.append((name, "未检测到ORB特征"))
            continue
        
        # 将特征转换为 base64 字符串
        descriptor_bytes = descriptors.tobytes()
        descriptor_b64 = base64.b64encode(descriptor_bytes).decode('utf-8')
        
        # 保存到事件对象
        event['orb_features'] = {
            'descriptors': descriptor_b64,
            'shape': descriptors.shape,  # (num_keypoints, 32)
            'keypoints_count': len(keypoints)
        }
        
        updated_count += 1
        print(f"✓ {name}: {len(keypoints)} 个特征点")

print("\n" + "="*80)
print("特征提取完成！")
print(f"  ✓ 成功: {updated_count}")
print(f"  ✗ 失败: {len(failed_list)}")

if failed_list:
    print("\n失败列表：")
    for name, reason in failed_list:
        print(f"  - {name}: {reason}")

# 保存更新后的 JSON
with open("../src-tauri/resources/event_encounters.json", 'w', encoding='utf-8') as f:
    json.dump(events, f, ensure_ascii=False, indent=2)

print("\n✓ 已保存到 event_encounters.json")
