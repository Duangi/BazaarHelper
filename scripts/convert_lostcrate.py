from PIL import Image
import os

source_path = r"D:\Projects\BazaarHelper\src-tauri\resources\EncEvent_CHAR\失落宝箱1.png"
target_path = r"D:\Projects\BazaarHelper\src-tauri\resources\EncEvent_CHAR\ENC_Merchant_TheLostCrate_Char.webp"

if os.path.exists(source_path):
    try:
        img = Image.open(source_path)
        img.save(target_path, "WEBP")
        print(f"Successfully converted {source_path} to {target_path}")
        
        # Optionally remove the source file if needed, but safe to keep for now
        # os.remove(source_path)
    except Exception as e:
        print(f"Error converting image: {e}")
else:
    print(f"Source file not found: {source_path}")
