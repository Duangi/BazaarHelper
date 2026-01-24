import json
import os
import struct

try:
    import cv2
    import numpy  # noqa: F401
    HAS_CV2 = True
except ImportError:
    HAS_CV2 = False
    print("警告: 未安装opencv-python，将跳过特征生成")
    print("请运行: pip install opencv-python")

def generate_event_features():
    """为所有事件图片生成ORB特征bin文件"""
    if not HAS_CV2:
        print("无法生成特征文件，缺少opencv-python")
        return
    
    # 读取 event_encounters.json
    json_path = "../src-tauri/resources/event_encounters.json"
    with open(json_path, 'r', encoding='utf-8') as f:
        events = json.load(f)
    
    # 创建特征文件夹
    features_dir = "../src-tauri/resources/event_features"
    os.makedirs(features_dir, exist_ok=True)
    
    # 初始化 ORB 检测器（与Rust侧参数一致）
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
    skipped_count = 0
    failed_list = []
    
    for event in events:
        if 'image_paths' not in event or not event['image_paths'].get('char'):
            continue
        
        # 获取事件ID和名称
        event_id = event.get('Id', '')
        if not event_id:
            continue
        
        char_path = os.path.join("../src-tauri/resources", event['image_paths']['char'])
        feature_path = os.path.join(features_dir, f"{event_id}.bin")
        
        # 提取事件名称用于显示
        name = None
        if 'Localization' in event and 'Title' in event['Localization']:
            name = event['Localization']['Title'].get('Text', '')
        if not name:
            name = event.get('InternalName', event_id)
        
        # 如果bin文件已存在，跳过
        if os.path.exists(feature_path):
            print(f"- {name}: 特征文件已存在，跳过")
            skipped_count += 1
            continue
        
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
        
        # 保存为bin文件
        # 格式: [num_descriptors: u32][descriptor_size: u32][descriptors: u8 array]
        with open(feature_path, 'wb') as f:
            # 写入描述符数量和大小
            f.write(struct.pack('I', descriptors.shape[0]))  # num_descriptors
            f.write(struct.pack('I', descriptors.shape[1]))  # descriptor_size (32)
            # 写入描述符数据
            f.write(descriptors.tobytes())
        
        generated_count += 1
        print(f"✓ {name}: 生成特征文件 ({len(keypoints)} 个特征点)")
    
    print("\n" + "="*80)
    print("特征生成完成！")
    print(f"  ✓ 新生成: {generated_count}")
    print(f"  - 已存在: {skipped_count}")
    print(f"  ✗ 失败: {len(failed_list)}")
    
    if failed_list:
        print("\n失败列表：")
        for name, reason in failed_list:
            print(f"  - {name}: {reason}")
    
    print(f"\n特征文件保存在: {os.path.abspath(features_dir)}")

if __name__ == "__main__":
    generate_event_features()
