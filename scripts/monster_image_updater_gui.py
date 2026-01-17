import tkinter as tk
from tkinter import ttk, messagebox, filedialog
import requests
from PIL import Image
import io
import os
from pathlib import Path
import json

# Define paths
ROOT = Path(__file__).resolve().parents[1]
RESOURCES = ROOT / 'src-tauri' / 'resources'
CHAR_DIR = RESOURCES / 'images_monster_char'
BG_DIR = RESOURCES / 'images_monster_bg'
MONSTERS_DB_PATH = RESOURCES / 'monsters_db.json'

# Ensure directories exist
CHAR_DIR.mkdir(parents=True, exist_ok=True)
BG_DIR.mkdir(parents=True, exist_ok=True)

class MonsterImageUpdater:
    def __init__(self, root):
        self.root = root
        self.root.title("怪兽图片更新工具 (BazaarHelper)")
        self.root.geometry("600x450")
        
        # UI Elements
        main_frame = ttk.Frame(root, padding="20")
        main_frame.pack(fill=tk.BOTH, expand=True)
        
        ttk.Label(main_frame, text="怪兽名称 (如: 恶蚊):", font=('Segoe UI', 10)).grid(row=0, column=0, sticky=tk.W, pady=5)
        self.name_var = tk.StringVar()
        self.name_entry = ttk.Entry(main_frame, textvariable=self.name_var, width=40)
        self.name_entry.grid(row=0, column=1, sticky=tk.W, pady=5, padx=5)
        
        ttk.Label(main_frame, text="图片来源 (URL 或 本地路径):", font=('Segoe UI', 10)).grid(row=1, column=0, sticky=tk.W, pady=5)
        self.source_var = tk.StringVar()
        self.source_entry = ttk.Entry(main_frame, textvariable=self.source_var, width=40)
        self.source_entry.grid(row=1, column=1, sticky=tk.W, pady=5, padx=5)
        
        btn_frame = ttk.Frame(main_frame)
        btn_frame.grid(row=2, column=1, sticky=tk.W, pady=10)
        
        self.browse_btn = ttk.Button(btn_frame, text="浏览本地文件", command=self.browse_file)
        self.browse_btn.pack(side=tk.LEFT, padx=2)
        
        self.update_btn = ttk.Button(main_frame, text="下载并替换 WebP", command=self.process_image)
        self.update_btn.grid(row=3, column=1, sticky=tk.W, pady=20)
        
        # Instructions
        instr = (
            "使用说明:\n"
            "1. 输入准确的怪物中文名。\n"
            "2. 粘贴来自 bazaardb.gg 的图片链接，或选择本地截图。\n"
            "3. 点击按钮，程序会自动将其保存为 .webp 格式并覆盖旧图。\n"
            "4. 替换后，程序识别时会自动加载新图 (可能需要手动删除 OpenCV 缓存 .bin 文件)。"
        )
        self.status_label = tk.Label(main_frame, text=instr, justify=tk.LEFT, fg="gray")
        self.status_label.grid(row=4, column=0, columnspan=2, sticky=tk.W, pady=20)

    def browse_file(self):
        filename = filedialog.askopenfilename(filetypes=[("Image files", "*.jpg *.jpeg *.png *.webp *.bmp")])
        if filename:
            self.source_var.set(filename)

    def process_image(self):
        name = self.name_var.get().strip()
        source = self.source_var.get().strip()
        
        if not name or not source:
            messagebox.showerror("错误", "请填写怪兽名称和图片来源")
            return
            
        try:
            img = None
            if source.startswith(('http://', 'https://')):
                response = requests.get(source, timeout=20)
                response.raise_for_status()
                img = Image.open(io.BytesIO(response.content))
            else:
                img = Image.open(source)
                
            if img:
                # Convert to RGBA if needed
                if img.mode != 'RGBA':
                    img = img.convert('RGBA')
                
                # Save as WebP in both locations (char and bg as fallback)
                char_path = CHAR_DIR / f"{name}.webp"
                bg_path = BG_DIR / f"{name}.webp"
                
                img.save(char_path, "WEBP")
                img.save(bg_path, "WEBP")
                
                messagebox.showinfo("成功", f"文件已保存至:\n{char_path}\n{bg_path}")
                
        except Exception as e:
            messagebox.showerror("处理失败", f"发生错误: {str(e)}")

if __name__ == "__main__":
    root = tk.Tk()
    app = MonsterImageUpdater(root)
    root.mainloop()
