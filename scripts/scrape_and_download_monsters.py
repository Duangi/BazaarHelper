#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
从 bazaardb.gg 网站爬取所有怪物的图片和背景 URL
并下载分层图片（背景层和角色层）
"""
import os
import time
import json
import re
from pathlib import Path
import requests
from PIL import Image
import io
from bs4 import BeautifulSoup

ROOT = Path(__file__).resolve().parents[1]
MONSTERS_DB = ROOT / 'src-tauri' / 'resources' / 'monsters_db.json'
BG_DIR = ROOT / 'src-tauri' / 'resources' / 'images_monster_bg'
CHAR_DIR = ROOT / 'src-tauri' / 'resources' / 'images_monster_char'
MAP_FILE = ROOT / 'src-tauri' / 'resources' / 'images_monster_map.json'

BG_DIR.mkdir(parents=True, exist_ok=True)
CHAR_DIR.mkdir(parents=True, exist_ok=True)

SEARCH_URL = 'https://bazaardb.gg/search?c=monsters'


def load_json(p):
    if not p.exists():
        return {}
    with open(p, 'r', encoding='utf-8') as f:
        return json.load(f)


def save_json(p, data):
    with open(p, 'w', encoding='utf-8') as f:
        json.dump(data, f, ensure_ascii=False, indent=2)


def download_and_save(url, out_path):
    """
    Download image from URL or decode base64 data URI and save to out_path.
    """
    try:
        # 检查是否是 data URI
        if url.startswith('data:'):
            # 解析 data:image/webp;base64,xxxxx
            import base64
            match = re.match(r'data:image/[^;]+;base64,(.+)', url)
            if match:
                image_data = base64.b64decode(match.group(1))
                with open(out_path, 'wb') as f:
                    f.write(image_data)
                return True, None
            else:
                return False, "Invalid data URI format"
        else:
            # 普通 URL 下载
            headers = {'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36'}
            r = requests.get(url, headers=headers, timeout=20)
            r.raise_for_status()
            
            # 直接保存原始内容
            with open(out_path, 'wb') as f:
                f.write(r.content)
            return True, None
    except Exception as e:
        return False, str(e)


def scrape_monsters_from_bazaardb():
    """
    从 bazaardb.gg 爬取所有怪物数据
    返回格式: {monster_name: {'background_url': ..., 'character_url': ...}}
    """
    print(f"正在访问: {SEARCH_URL}")
    headers = {
        'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36'
    }
    
    try:
        response = requests.get(SEARCH_URL, headers=headers, timeout=30)
        response.raise_for_status()
        soup = BeautifulSoup(response.text, 'html.parser')
        
        monsters = {}
        # 查找所有符合结构的怪物容器
        # <div class="_ay _aB" style="aspect-ratio: 1 / 1; ...">
        monster_divs = soup.find_all('div', class_='_ay')
        
        print(f"找到 {len(monster_divs)} 个怪物元素")
        
        for div in monster_divs:
            try:
                # 提取纯背景 URL (第一个 img 的 src，是 base64 或 URL)
                bg_img = div.find('div', class_='_aC').find('img') if div.find('div', class_='_aC') else None
                bg_url = None
                if bg_img:
                    bg_src = bg_img.get('src', '')
                    # 如果是 data:image/webp;base64, 就保留；如果是 URL 也保留
                    bg_url = bg_src if bg_src else None
                
                # 提取角色 URL (img._aD)
                char_img = div.find('img', class_='_aD')
                char_url = None
                name = None
                if char_img:
                    char_url = char_img.get('src')
                    name = char_img.get('alt', '').strip()
                
                if name and (bg_url or char_url):
                    monsters[name] = {
                        'name_zh': name,  # 暂时用英文名，后续可以映射中文
                        'background_url': bg_url,
                        'character_url': char_url
                    }
                    print(f"  ✓ {name}")
                    
            except Exception as e:
                print(f"  ✗ 解析元素失败: {e}")
                continue
        
        print(f"\n成功提取 {len(monsters)} 个怪物数据")
        return monsters
        
    except Exception as e:
        print(f"爬取失败: {e}")
        return {}


def main():
    print("=" * 60)
    print("从 bazaardb.gg 爬取怪物图片")
    print("=" * 60)
    
    # 1. 爬取网站数据
    scraped_monsters = scrape_monsters_from_bazaardb()
    
    if not scraped_monsters:
        print("未能获取怪物数据，退出")
        return
    
    # 2. 加载现有的 monsters_db.json（如果有中文名映射）
    existing_db = load_json(MONSTERS_DB)
    
    # 3. 合并数据：保留现有的中文名等信息
    monsters_data = {}
    for name, data in scraped_monsters.items():
        # 查找是否有现有数据（通过英文名匹配）
        existing_entry = None
        for key, value in existing_db.items():
            if isinstance(value, dict):
                if value.get('name') == name or key == name:
                    existing_entry = value
                    break
        
        if existing_entry:
            # 合并现有数据和新抓取的 URL
            merged = existing_entry.copy()
            merged['background_url'] = data['background_url']
            merged['character_url'] = data['character_url']
            monsters_data[name] = merged
        else:
            # 新怪物
            monsters_data[name] = data
    
    print(f"\n开始下载 {len(monsters_data)} 个怪物的图片...")
    
    # 4. 下载图片
    results = {}
    count = 0
    
    for name, data in monsters_data.items():
        name_zh = data.get('name_zh', name)
        bg_url = data.get('background_url', '')
        ch_url = data.get('character_url', '')
        
        if not bg_url and not ch_url:
            print(f"[{name}] 无图片 URL，跳过")
            continue
        
        entry = {
            'background': None,
            'character': None,
            'background_url': bg_url,
            'character_url': ch_url
        }
        
        # 下载背景（WebP）
        if bg_url:
            # 使用英文名作为文件名，避免特殊字符问题
            safe_name = re.sub(r'[^\w\-]', '_', name)
            out_name = f"{safe_name}_bg.webp"
            out_path = BG_DIR / out_name
            
            if out_path.exists():
                entry['background'] = str(out_path.relative_to(ROOT / 'src-tauri' / 'resources'))
                print(f"[{name}] 背景已存在")
            else:
                ok, err = download_and_save(bg_url, out_path)
                if ok:
                    entry['background'] = str(out_path.relative_to(ROOT / 'src-tauri' / 'resources'))
                    print(f"[{name}] ✓ 下载背景")
                else:
                    entry['background_error'] = err
                    print(f"[{name}] ✗ 背景失败: {err}")
                time.sleep(0.5)
        
        # 下载角色图（WebP）
        char_path_rel = None
        if ch_url:
            safe_name = re.sub(r'[^\w\-]', '_', name)
            out_name = f"{safe_name}_char.webp"
            out_path = CHAR_DIR / out_name
            
            if out_path.exists():
                entry['character'] = str(out_path.relative_to(ROOT / 'src-tauri' / 'resources'))
                char_path_rel = entry['character']
                print(f"[{name}] 角色图已存在")
            else:
                ok, err = download_and_save(ch_url, out_path)
                if ok:
                    entry['character'] = str(out_path.relative_to(ROOT / 'src-tauri' / 'resources'))
                    char_path_rel = entry['character']
                    print(f"[{name}] ✓ 下载角色")
                else:
                    entry['character_error'] = err
                    print(f"[{name}] ✗ 角色失败: {err}")
                time.sleep(0.5)
        
        # 更新 image 字段为角色层图片（用于识图）
        if char_path_rel:
            data['image'] = char_path_rel
        
        results[name] = entry
        count += 1
    
    # 5. 保存结果
    save_json(MAP_FILE, results)
    save_json(MONSTERS_DB, monsters_data)
    
    print("\n" + "=" * 60)
    print(f"完成！处理了 {count} 个怪物")
    print(f"背景图保存在: {BG_DIR}")
    print(f"角色图保存在: {CHAR_DIR}")
    print(f"映射文件: {MAP_FILE}")
    print(f"数据库更新: {MONSTERS_DB}")
    print("识图将使用角色层 WebP 图片（保留透明通道）")
    print("=" * 60)


if __name__ == '__main__':
    main()
