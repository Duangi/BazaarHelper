import json
import os
import struct

try:
    import cv2
    import numpy as np
    HAS_CV2 = True
except ImportError:
    HAS_CV2 = False
    print("警告: 未安装opencv-python，将跳过特征生成")
    print("请运行: pip install opencv-python")

def pack_string(s):
    """Bincode string serialization: len(u64) + bytes"""
    encoded = s.encode('utf-8')
    return struct.pack('<Q', len(encoded)) + encoded

def pack_vec_u8(data):
    """Bincode Vec<u8> serialization: len(u64) + bytes"""
    return struct.pack('<Q', len(data)) + data

def generate_event_features():
    """为所有事件图片生成单一的ORB特征bin文件 (event_features_opencv.bin)"""
    if not HAS_CV2:
        print("无法生成特征文件，缺少opencv-python")
        return
    
    # 读取 event_encounters.json
    base_dir = os.path.dirname(os.path.abspath(__file__))
    resources_dir = os.path.abspath(os.path.join(base_dir, "../src-tauri/resources"))
    
    json_path = os.path.join(resources_dir, "event_encounters.json")
    with open(json_path, 'r', encoding='utf-8') as f:
        events = json.load(f)
    
    # 目标文件路径
    output_path = os.path.join(resources_dir, "event_features_opencv.bin")
    
    # 初始化 ORB 检测器
    orb = cv2.ORB_create(
        nfeatures=500,
        scaleFactor=1.2,
        nlevels=8,
        edgeThreshold=15,
        firstLevel=0,
        WTA_K=2,
        scoreType=cv2.ORB_HARRIS_SCORE,
        patchSize=31,
        fastThreshold=20
    )
    
    print("开始生成事件特征文件...")
    print("="*80)
    
    generated_count = 0
    failed_list = []
    
    # 收集所有模板数据
    temp_buffer = bytearray()
    valid_events_count = 0
    
    for event in events:
        if 'image_paths' not in event or not event['image_paths'].get('char'):
            continue
        
        # 获取事件ID
        event_id = event.get('Id', '')
        if not event_id:
            continue
        
        char_path = os.path.join(resources_dir, event['image_paths']['char'])
        
        # 提取事件名称
        name = None
        if 'Localization' in event and 'Title' in event['Localization']:
            name = event['Localization']['Title'].get('Text', '')
        if not name:
            name = event.get('InternalName', event_id)
        
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
            
        generated_count += 1
        print(f"✓ {name}: 生成特征 ({len(keypoints)} 个特征点)")
        
        # 序列化单个 EventTemplateCache 结构
        # struct EventTemplateCache {
        #     id: String,
        #     name: String,
        #     descriptors: Vec<u8>,
        #     descriptor_rows: i32,
        #     descriptor_cols: i32,
        # }
        
        item_data = bytearray()
        item_data.extend(pack_string(event_id))
        item_data.extend(pack_string(name))
        item_data.extend(pack_vec_u8(descriptors.tobytes()))
        item_data.extend(struct.pack('<i', descriptors.shape[0])) # rows
        item_data.extend(struct.pack('<i', descriptors.shape[1])) # cols
        
        temp_buffer.extend(item_data)
        valid_events_count += 1
        
    # 最终写入文件
    # Vec<EventTemplateCache> 序列化: [len: u64] [item1] [item2] ...
    with open(output_path, 'wb') as f:
        f.write(struct.pack('<Q', valid_events_count))
        f.write(temp_buffer)
        
    print("\n" + "="*80)
    print("特征生成完成！")
    print(f"  ✓ 成功打包: {valid_events_count}")
    print(f"  ✗ 失败: {len(failed_list)}")
    
    if failed_list:
        print("\n失败列表：")
        for name, reason in failed_list:
            print(f"  - {name}: {reason}")
    
    print(f"\n单一特征文件保存在: {output_path}")

if __name__ == "__main__":
    generate_event_features()
