#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import requests
import json
import re
import uuid
import os
from bs4 import BeautifulSoup
from PIL import Image
import io

def load_items_db():
    """加载现有的物品数据库"""
    try:
        with open('src-tauri/resources/items_db.json', 'r', encoding='utf-8') as f:
            return json.load(f)
    except:
        return {}

def load_combat_encounters():
    """加载战斗遭遇数据库，用于查找物品描述"""
    try:
        with open('src-tauri/resources/combat_encounters.json', 'r', encoding='utf-8') as f:
            return json.load(f)
    except:
        return {}

def load_monster_skills_db():
    """加载怪物技能数据库"""
    try:
        with open('src-tauri/resources/monster_skills_db.json', 'r', encoding='utf-8') as f:
            return json.load(f)
    except:
        return {}

def find_item_description_in_encounters(item_id, encounters_db):
    """在战斗遭遇数据库中查找物品描述"""
    for encounter_id, encounter in encounters_db.items():
        items = encounter.get('items', [])
        for item in items:
            if item.get('id') == item_id and item.get('description'):
                return item.get('description')
    return ''

def download_and_composite_images(background_url, character_url, filename, target_dir="src-tauri/resources/images"):
    """下载背景图和角色图，合成后保存为JPG格式"""
    if not background_url or not character_url:
        return False
        
    try:
        # 确保目标目录存在
        os.makedirs(target_dir, exist_ok=True)
        
        headers = {
            'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36'
        }
        
        # 下载背景图
        print(f"  正在下载背景图: {background_url}")
        bg_response = requests.get(background_url, headers=headers, timeout=10)
        bg_response.raise_for_status()
        background = Image.open(io.BytesIO(bg_response.content))
        
        # 下载角色图
        print(f"  正在下载角色图: {character_url}")
        char_response = requests.get(character_url, headers=headers, timeout=10)
        char_response.raise_for_status()
        character = Image.open(io.BytesIO(char_response.content))
        
        # 将两张图片都转换为RGBA模式以支持透明度
        if background.mode != 'RGBA':
            background = background.convert('RGBA')
        if character.mode != 'RGBA':
            character = character.convert('RGBA')
        
        # 调整角色图尺寸以匹配背景图
        if character.size != background.size:
            character = character.resize(background.size, Image.Resampling.LANCZOS)
        
        # 合成图片：将角色图叠加到背景图上
        composite = Image.alpha_composite(background, character)
        
        # 转换为RGB模式用于JPG保存
        if composite.mode == 'RGBA':
            # 创建白色背景
            rgb_composite = Image.new('RGB', composite.size, (255, 255, 255))
            rgb_composite.paste(composite, mask=composite.split()[-1])  # 使用alpha通道作为mask
            composite = rgb_composite
        
        # 保存为JPG格式
        target_path = os.path.join(target_dir, f"{filename}.jpg")
        composite.save(target_path, 'JPEG', quality=95)
        print(f"  ✓ 合成图片已保存: {target_path}")
        return True
        
    except Exception as e:
        print(f"  ✗ 图片合成失败: {e}")
        return False

def download_and_convert_image(url, filename, target_dir="src-tauri/resources/images", max_retries=3):
    """下载单张图片并转换为JPG格式（用于技能图片），支持重试"""
    if not url:
        return False
    
    # 检查图片是否已存在
    target_path = os.path.join(target_dir, f"{filename}.jpg")
    if os.path.exists(target_path):
        print(f"  ✓ 图片已存在，跳过下载: {filename}.jpg")
        return True
        
    for attempt in range(max_retries):
        try:
            # 确保目标目录存在
            os.makedirs(target_dir, exist_ok=True)
            
            headers = {
                'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36'
            }
            
            response = requests.get(url, headers=headers, timeout=15, stream=True)
            response.raise_for_status()
            
            # 使用PIL处理图片
            image = Image.open(io.BytesIO(response.content))
            
            # 如果是RGBA模式，转换为RGB
            if image.mode == 'RGBA':
                # 创建白色背景
                background = Image.new('RGB', image.size, (255, 255, 255))
                background.paste(image, mask=image.split()[-1])  # 使用alpha通道作为mask
                image = background
            elif image.mode != 'RGB':
                image = image.convert('RGB')
            
            # 保存为JPG格式
            image.save(target_path, 'JPEG', quality=95)
            print(f"  ✓ 图片已保存: {target_path}")
            return True
            
        except requests.exceptions.RequestException as e:
            if attempt < max_retries - 1:
                print(f"  ⚠ 下载失败 (尝试 {attempt + 1}/{max_retries})，重试中...")
                import time
                time.sleep(2)  # 等待2秒后重试
            else:
                print(f"  ✗ 图片下载失败 (已重试{max_retries}次): {filename}")
                return False
        except Exception as e:
            print(f"  ✗ 图片处理失败: {e}")
            return False
    
    return False

def update_items_db_image(item_name_zh, image_path):
    """更新items_db.json中物品的图片路径"""
    try:
        items_db_path = 'src-tauri/resources/items_db.json'
        with open(items_db_path, 'r', encoding='utf-8') as f:
            items_db = json.load(f)
        
        # 查找匹配的物品
        updated = False
        for item_id, item_data in items_db.items():
            if item_data.get('name_zh') == item_name_zh or item_data.get('name') == item_name_zh:
                # 只有当前没有图片路径或图片路径为空时才更新
                if not item_data.get('image') or item_data.get('image') == '':
                    item_data['image'] = image_path
                    updated = True
                    print(f"  ✓ 已更新items_db中 '{item_name_zh}' 的图片路径")
                    break
        
        if updated:
            with open(items_db_path, 'w', encoding='utf-8') as f:
                json.dump(items_db, f, ensure_ascii=False, indent=2)
        
        return updated
    except Exception as e:
        print(f"  ✗ 更新items_db失败: {e}")
        return False

def check_image_exists(filename, target_dir="src-tauri/resources/images"):
    """检查图片是否已存在"""
    image_path = os.path.join(target_dir, f"{filename}.jpg")
    return os.path.exists(image_path)

def is_chinese(text):
    """判断文本是否包含中文"""
    if not text:
        return False
    # 检查是否包含中文字符
    for char in text:
        if '\u4e00' <= char <= '\u9fff':
            return True
    return False

def rename_image_file(old_name, new_name, target_dir="src-tauri/resources/images"):
    """重命名图片文件"""
    try:
        old_path = os.path.join(target_dir, f"{old_name}.jpg")
        new_path = os.path.join(target_dir, f"{new_name}.jpg")
        
        if not os.path.exists(old_path):
            print(f"  ⚠ 原图片不存在: {old_name}.jpg")
            return False
        
        if old_name == new_name:
            return True
        
        # 如果目标文件已存在，先删除旧的源文件
        if os.path.exists(new_path):
            print(f"  ⚠ 目标图片已存在: {new_name}.jpg，删除源文件")
            os.remove(old_path)
            return True
        
        os.rename(old_path, new_path)
        print(f"  ✓ 图片已重命名: {old_name}.jpg -> {new_name}.jpg")
        return True
    except Exception as e:
        print(f"  ✗ 图片重命名失败: {e}")
        return False

def find_skill_by_name(name, items_db):
    """在物品数据库中查找技能，支持中英文名称匹配"""
    if not name:
        return None
    
    name = name.strip()
    # 遍历所有物品，寻找附魔中匹配的技能
    for item_id, item_data in items_db.items():
        enchantments = item_data.get('enchantments', [])
        for enchantment in enchantments:
            if (enchantment.get('name') == name or 
                enchantment.get('name_en') == name or
                enchantment.get('id') == name.lower()):
                return {
                    "id": enchantment.get('id', ''),
                    "name": enchantment.get('name', name),
                    "name_en": enchantment.get('name_en', name),
                    "tier": "Bronze",  # 默认等级
                    "description": enchantment.get('description', ''),
                    "image": f"images/{enchantment.get('name', name)}.jpg"
                }
    return None

def find_item_by_name(name, items_db, encounters_db=None):
    """在物品数据库中查找物品，支持中英文名称匹配"""
    if not name:
        return None
    
    name = name.strip()
    for item_id, item_data in items_db.items():
        if (item_data.get('name_zh') == name or 
            item_data.get('name_en') == name or
            item_data.get('name') == name):
            
            # 从战斗遭遇数据库中查找描述
            description = ''
            if encounters_db:
                description = find_item_description_in_encounters(item_id, encounters_db)
            
            return {
                "id": item_id,
                "name": item_data.get('name_zh', item_data.get('name', name)),
                "name_en": item_data.get('name_en', name),
                "tier": item_data.get('tier', 'Silver'),
                "socket": "Socket_0",  # 默认插槽，需要手动调整
                "description": description,
                "image": item_data.get('image', f"images/{item_data.get('name_zh', name)}.jpg")
            }
    return None

def scrape_monster_info(url):
    """爬取怪物信息"""
    headers = {
        'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36'
    }
    
    try:
        response = requests.get(url, headers=headers, timeout=10)
        response.raise_for_status()
        soup = BeautifulSoup(response.content, 'html.parser')
        
        # 保存HTML到文件用于调试
        with open('debug.html', 'w', encoding='utf-8') as f:
            f.write(soup.prettify())
        
        # 从URL中提取中文名
        monster_name_zh = ""
        url_match = re.search(r'/([^/]+)/zh-CN', url)
        if url_match:
            import urllib.parse
            monster_name_zh = urllib.parse.unquote(url_match.group(1))
        
        # 从meta description中提取基础信息
        meta_desc = soup.find('meta', {'name': 'description'})
        description_text = meta_desc.get('content', '') if meta_desc else ''
        print(f"Meta描述: {description_text}")
        
        # 从meta description中提取技能信息（Skills: Fiery Bronze ...)
        skill_info = {}
        skill_match = re.search(r'Skills?:\s*([^.]+)', description_text)
        if skill_match:
            skill_text = skill_match.group(1).strip()
            # 解析技能名称和等级，例如: "Fiery Bronze +1"
            skill_parts = skill_text.split()
            if len(skill_parts) >= 1:
                skill_info['name_en'] = skill_parts[0]  # Fiery
                skill_info['tier'] = skill_parts[1] if len(skill_parts) > 1 and skill_parts[1] in ['Bronze', 'Silver', 'Gold'] else 'Bronze'
            print(f"提取到技能信息: {skill_info}")
        
        # 从页面标题提取英文名
        title_tag = soup.find('title')
        title_text = title_tag.get_text() if title_tag else ''
        monster_name_en = title_text.split(' - ')[0].strip() if ' - ' in title_text else ''
        
        print(f"怪物名称: {monster_name_en} ({monster_name_zh})")
        
        # 抓取背景图和角色图URL
        background_url = ""
        character_url = ""
        
        # 查找背景图（在div的style属性中）
        bg_div = soup.find('div', {'class': '_aK'})
        if bg_div and bg_div.get('style'):
            style = bg_div.get('style')
            # 更宽松的正则匹配，处理有无引号的情况
            bg_match = re.search(r'background-image:\s*url\(([^)]+)\)', style)
            if bg_match:
                background_url = bg_match.group(1).strip('"\'')
                print(f"找到背景图: {background_url}")
        
        # 如果第一个div没找到，尝试查找其他包含background-image的div
        if not background_url:
            all_divs = soup.find_all('div', {'class': '_aK'})
            for div in all_divs:
                if div.get('style') and 'background-image' in div.get('style'):
                    style = div.get('style')
                    bg_match = re.search(r'background-image:\s*url\(([^)]+)\)', style)
                    if bg_match:
                        background_url = bg_match.group(1).strip('"\'')
                        print(f"找到背景图: {background_url}")
                        break
        
        # 查找角色图（在img标签中）
        char_img = soup.find('img', {'class': '_aL'})
        if char_img:
            character_url = char_img.get('src', '')
            print(f"找到角色图: {character_url}")
        
        # 如果没有找到指定class的图片，尝试其他方式
        if not background_url or not character_url:
            # 查找所有包含 encounter 的图片URL
            all_imgs = soup.find_all('img')
            for img in all_imgs:
                src = img.get('src', '')
                if 'encounter' in src and not character_url:
                    character_url = src
                    print(f"备用角色图: {character_url}")
                    break
        
        skills = []
        items = []
        
        # 查找所有文本，寻找中文技能和物品信息
        all_text = soup.get_text()
        print(f"页面总文本长度: {len(all_text)}")
        
        # 查找所有可能包含技能描述的文本
        text_elements = soup.find_all(['div', 'span', 'p', 'td', 'th'])
        
        for elem in text_elements:
            text = elem.get_text().strip()
            if not text or len(text) < 3:
                continue
            
            # 查找包含"灼烧"、"伤害"、"治疗"等关键词的描述
            game_effects = ['灼烧', '伤害', '治疗', '护甲', '生命', '充能', '攻击', '防御', '冷却']
            
            if any(effect in text for effect in game_effects):
                print(f"发现游戏效果文本: {text}")
                
                # 如果文本包含数字和游戏效果，可能是技能或物品描述
                if any(char.isdigit() for char in text) and len(text) < 200:
                    # 检查是否是技能描述（通常包含 +1 » +2 这样的升级描述）
                    if '»' in text or '+' in text:
                        # 提取技能名（通常在描述前面）
                        skill_name_match = re.search(r'^([^，。,.:]+)[，。,:]', text)
                        skill_name = skill_name_match.group(1).strip() if skill_name_match else text.split()[0]
                        
                        skills.append({
                            "id": str(uuid.uuid4()),
                            "name": skill_name,
                            "name_en": skill_name,  # 可以后续翻译
                            "tier": "Bronze",
                            "description": text,
                            "image": ""
                        })
                        print(f"找到技能: {skill_name} - {text}")
                    
                    # 检查是否是简单的物品描述（如"造成5灼烧"）
                    elif len(text) < 50 and not '»' in text:
                        # 这可能是物品描述，暂存待匹配
                        print(f"可能的物品描述: {text}")
        
        # 从meta描述中解析物品名称列表
        item_descriptions = {}  # 存储物品描述的映射
        
        if description_text:
            # 解析物品：Items: Cinders, Lighter, Cinders
            item_match = re.search(r'Items:\s*([^.]+)', description_text)
            if item_match:
                item_names = [i.strip() for i in item_match.group(1).split(',')]
                
                # 为每个物品查找对应的描述
                for item_name in item_names:
                    if item_name:
                        # 在页面中查找这个物品的描述
                        item_desc = ""  # 默认留空
                        
                        # 查找包含物品相关效果的文本
                        for elem in text_elements:
                            elem_text = elem.get_text().strip()
                            # 查找包含数字和游戏效果的短文本（可能是物品描述）
                            if (any(char.isdigit() for char in elem_text) and 
                                any(effect in elem_text for effect in game_effects) and
                                len(elem_text) < 50 and '»' not in elem_text):
                                item_desc = elem_text
                                print(f"为 {item_name} 找到描述: {elem_text}")
                                break
                        
                        items.append({
                            "name": item_name,
                            "description": item_desc
                        })
                        print(f"添加物品: {item_name}")
        
        # 如果没有从meta找到技能，尝试从页面寻找技能标题
        if not skills and description_text:
            skill_match = re.search(r'Skills:\s*([^.]+)', description_text)
            if skill_match:
                skill_names = [s.strip() for s in skill_match.group(1).split(',')]
                for skill_name in skill_names:
                    if skill_name:
                        # 查找这个技能的详细描述
                        skill_desc = ""  # 默认留空
                        
                        for elem in text_elements:
                            elem_text = elem.get_text().strip()
                            if ('»' in elem_text or '+' in elem_text) and len(elem_text) < 200:
                                skill_desc = elem_text
                                print(f"为 {skill_name} 找到描述: {elem_text}")
                                break
                        
                        skills.append({
                            "id": str(uuid.uuid4()),
                            "name": skill_name,
                            "name_en": skill_name,
                            "tier": "Bronze",
                            "description": skill_desc,
                            "image": ""
                        })
        
        monster_info = {
            "name": monster_name_en or monster_name_zh,
            "name_zh": monster_name_zh,
            "skills": skills,
            "items": items,
            "background_url": background_url,
            "character_url": character_url
        }
        
        print(f"最终提取结果: {len(skills)} 个技能, {len(items)} 个物品")
        return monster_info
        
    except Exception as e:
        print(f"爬取失败: {e}")
        return None

def generate_monster_json(url):
    """生成怪物JSON数据"""
    # 加载数据库
    items_db = load_items_db()
    encounters_db = load_combat_encounters()
    skills_db = load_monster_skills_db()
    print(f"加载了 {len(items_db)} 个物品数据")
    print(f"加载了 {len(encounters_db)} 个战斗遭遇数据")
    print(f"加载了 {len(skills_db)} 个怪物技能数据")
    
    # 爬取怪物信息
    print(f"正在爬取: {url}")
    monster_info = scrape_monster_info(url)
    
    if not monster_info:
        return None
    
    # 生成怪物key（使用中文名）
    monster_key = monster_info.get("name_zh", monster_info.get("name", "未知怪物"))
    
    # 处理技能列表
    skills = []
    for i, skill in enumerate(monster_info.get("skills", [])):
        skill_name = skill.get("name", f"技能{i+1}")
        
        # 先在怪物技能数据库中查找（优先级最高）
        found_skill = None
        for skill_id, skill_data in skills_db.items():
            if skill_data.get("name_en", "").lower() == skill_name.lower() or skill_data.get("name", "") == skill_name:
                found_skill = skill_data.copy()
                print(f"在怪物技能库找到匹配: {skill_name} -> {found_skill['name']}")
                break
        
        # 如果怪物技能库没有，再到items_db中查找
        if not found_skill:
            found_skill = find_skill_by_name(skill_name, items_db)
            if found_skill:
                print(f"在items_db找到匹配技能: {skill_name} -> {found_skill['name']}")
        
        if found_skill:
            # 优先使用数据库中的描述（这是正确的中文描述）
            # 只有当数据库没有描述时，才使用爬取的
            if not found_skill.get("description") or found_skill["description"].strip() == "":
                found_skill["description"] = skill.get("description", "")
            print(f"  使用描述: {found_skill['description'][:50]}...")
            skills.append(found_skill)
        else:
            print(f"未找到匹配技能: {skill_name}，创建新条目")
            skills.append({
                "id": str(uuid.uuid4()),
                "name": skill.get("name", f"技能{i+1}"),
                "name_en": skill.get("name_en", f"Skill {i+1}"),
                "tier": skill.get("tier", "Bronze"),
                "description": skill.get("description", ""),
                "image": ""
            })
    
    # 处理物品列表
    items = []
    for i, item in enumerate(monster_info.get("items", [])):
        item_name = item.get("name", "")
        
        # 在数据库中查找匹配的物品，同时传入战斗遭遇数据库
        found_item = find_item_by_name(item_name, items_db, encounters_db)
        
        if found_item:
            print(f"找到匹配物品: {item_name} -> {found_item['name']} (描述: {found_item['description'][:20]}...)")
            found_item["socket"] = f"Socket_{i}"  # 设置插槽位置
            items.append(found_item)
        else:
            print(f"未找到匹配物品: {item_name}，创建新条目")
            items.append({
                "id": str(uuid.uuid4()),
                "name": item_name,
                "name_en": item_name,
                "tier": "Silver",
                "socket": f"Socket_{i}",
                "description": item.get("description", ""),
                "image": ""
            })
    
    # 构建最终JSON
    monster_json = {
        monster_key: {
            "name": monster_info.get("name", "Unknown Monster"),
            "name_zh": monster_info.get("name_zh", "未知怪物"),
            "skills": skills,
            "items": items,
            "image": f"images/{monster_key}.jpg",  # 图片路径
            "background_url": monster_info.get("background_url", ""),
            "character_url": monster_info.get("character_url", "")
        }
    }
    
    return monster_json

def manual_input_monster():
    """手动输入怪物信息"""
    print("手动输入怪物信息模式")
    print("=" * 30)
    
    monster_name_zh = input("怪物中文名称: ").strip()
    monster_name_en = input("怪物英文名称: ").strip()
    
    skills = []
    print(f"\n输入 {monster_name_zh} 的技能信息 (直接回车结束):")
    skill_count = 0
    while True:
        skill_count += 1
        skill_name = input(f"技能{skill_count}名称: ").strip()
        if not skill_name:
            break
        skill_desc = input(f"技能{skill_count}描述: ").strip()
        skill_tier = input(f"技能{skill_count}等级 [Bronze/Silver/Gold] (默认Bronze): ").strip() or "Bronze"
        
        skills.append({
            "id": str(uuid.uuid4()),
            "name": skill_name,
            "name_en": skill_name,  # 可以后续修改
            "tier": skill_tier,
            "description": skill_desc,
            "image": ""
        })
    
    items = []
    print(f"\n输入 {monster_name_zh} 的物品信息 (直接回车结束):")
    item_count = 0
    while True:
        item_count += 1
        item_name = input(f"物品{item_count}名称: ").strip()
        if not item_name:
            break
        item_desc = input(f"物品{item_count}描述: ").strip()
        item_tier = input(f"物品{item_count}等级 [Bronze/Silver/Gold] (默认Silver): ").strip() or "Silver"
        socket_num = input(f"物品{item_count}插槽位置 (0-7, 默认{item_count-1}): ").strip() or str(item_count-1)
        
        items.append({
            "id": str(uuid.uuid4()),
            "name": item_name,
            "name_en": item_name,  # 可以后续修改
            "tier": item_tier,
            "socket": f"Socket_{socket_num}",
            "description": item_desc,
            "image": ""
        })
    
    return {
        "name": monster_name_en,
        "name_zh": monster_name_zh,
        "skills": skills,
        "items": items,
        "background_url": background_url,
        "character_url": character_url
    }

def enhance_with_manual_info(monster_info):
    """手动增强怪物信息"""
    print("\n=== 手动增强模式 ===")
    print("爬取到的信息可能不完整，请手动补充：")
    
    # 技能信息增强
    for i, skill in enumerate(monster_info["skills"]):
        print(f"\n技能 {i+1}: {skill['name']}")
        print(f"当前描述: {skill['description']}")
        
        new_name = input("中文技能名 (回车保持不变): ").strip()
        if new_name:
            skill["name"] = new_name
            
        new_desc = input("技能描述 (回车保持不变): ").strip()
        if new_desc:
            skill["description"] = new_desc
    
    # 物品信息增强
    for i, item in enumerate(monster_info["items"]):
        print(f"\n物品 {i+1}: {item['name']}")
        print(f"当前描述: {item.get('description', '无')}")
        
        new_desc = input("物品描述 (回车保持不变): ").strip()
        if new_desc:
            item["description"] = new_desc
    
    return monster_info

def main():
    """主函数"""
    print("怪物信息爬虫 - Bazaar Helper")
    print("=" * 50)
    
    url = input("请输入怪物页面URL (回车使用默认): ").strip()
    if not url:
        url = "https://bazaardb.gg/card/44ozvkobqiztq39ir867y62cp/%E7%81%AB%E7%81%B5/zh-CN"
    
    monster_json = generate_monster_json(url)
    
    if monster_json:
        # 获取怪物信息
        monster_key = list(monster_json.keys())[0]
        monster_info = monster_json[monster_key]
        
        # 读取现有的 combat_encounters.json，检查是否已存在
        encounters_file = "src-tauri/resources/combat_encounters.json"
        try:
            with open(encounters_file, 'r', encoding='utf-8') as f:
                existing_encounters = json.load(f)
        except:
            existing_encounters = {}
        
        # 检查怪物是否已存在（通过key或name_zh匹配）
        is_update = False
        existing_key = None
        
        # 先检查key是否直接匹配
        if monster_key in existing_encounters:
            is_update = True
            existing_key = monster_key
        else:
            # 检查是否有相同name_zh的怪物
            for key, data in existing_encounters.items():
                if data.get("name_zh") == monster_key or data.get("name_zh") == monster_info.get("name_zh"):
                    is_update = True
                    existing_key = key
                    print(f"\n发现已存在的怪物 (key不同): {key} -> name_zh: {data.get('name_zh')}")
                    break
        
        if is_update:
            print(f"\n发现已存在的怪物: {existing_key}，将进行更新...")
            # 使用现有的key，不要改变
            monster_key = existing_key
            # 合并现有数据和新数据
            existing_monster = existing_encounters[existing_key]
            # 保留现有的图片路径（如果有的话）
            if existing_monster.get("image") and not monster_info.get("image"):
                monster_info["image"] = existing_monster["image"]
        else:
            print(f"\n创建新怪物: {monster_key}")
        
        # 自动处理怪物图片合成
        # 重新获取最新的monster数据来确保获取到图片URL
        fresh_monster_info = scrape_monster_info(url)
        background_url = fresh_monster_info.get("background_url", "") if fresh_monster_info else ""
        character_url = fresh_monster_info.get("character_url", "") if fresh_monster_info else ""
        
        if background_url and character_url:
            print(f"\n正在合成{monster_key}的图片...")
            if download_and_composite_images(background_url, character_url, monster_key):
                monster_info["image"] = f"images/{monster_key}.jpg"
            else:
                if not monster_info.get("image"):  # 只有在没有现有图片路径时才清空
                    monster_info["image"] = ""
        else:
            print(f"\n未找到{monster_key}的图片URL，跳过图片处理")
        
        # 处理技能图片和中文化
        print(f"\n开始处理技能图片和中文化...")
        for i, skill in enumerate(monster_info.get("skills", [])):
            skill_name = skill.get("name", "")
            skill_name_en = skill.get("name_en", "")
            
            # 如果技能名已经是中文，跳过更新检查和中文化询问
            if is_chinese(skill_name):
                print(f"  ✓ 技能 {i+1} 已有中文名: {skill_name}")
                final_name = skill_name
            else:
                # 先从所有怪物中查找是否有相同技能的中文信息
                found_in_encounters = False
                for enc_key, enc_data in existing_encounters.items():
                    for enc_skill in enc_data.get("skills", []):
                        if (enc_skill.get("name_en") == skill_name_en or 
                            enc_skill.get("name_en") == skill_name):
                            if is_chinese(enc_skill.get("name", "")):
                                skill["name"] = enc_skill["name"]
                                print(f"  ✓ 技能 {i+1} 从其他怪物复用中文名: {enc_skill['name']}")
                                found_in_encounters = True
                            if is_chinese(enc_skill.get("description", "")):
                                skill["description"] = enc_skill["description"]
                                print(f"  ✓ 技能 {i+1} 从其他怪物复用中文描述: {enc_skill['description'][:30]}...")
                            if found_in_encounters:
                                break
                    if found_in_encounters:
                        break
                
                # 如果在其他怪物中没找到，再尝试从当前怪物的现有数据中获取（更新模式）
                if not found_in_encounters and is_update and existing_key:
                    existing_skills = existing_encounters.get(existing_key, {}).get("skills", [])
                    for ex_skill in existing_skills:
                        if (ex_skill.get("name_en") == skill_name_en or 
                            ex_skill.get("name") == skill_name or
                            ex_skill.get("name") == skill_name_en):
                            if is_chinese(ex_skill.get("name", "")):
                                skill["name"] = ex_skill["name"]
                                print(f"  ✓ 技能 {i+1} 保留已有中文名: {ex_skill['name']}")
                            if is_chinese(ex_skill.get("description", "")):
                                skill["description"] = ex_skill["description"]
                                print(f"  ✓ 技能 {i+1} 保留已有中文描述")
                            break
                
                # 先处理中文化，确定最终使用的名称
                final_name = skill.get("name", skill_name)
                if not is_chinese(final_name):
                    print(f"\n  技能 {i+1}: '{final_name}' 没有中文名")
                    zh_name = input(f"  请输入技能的中文名 (回车跳过): ").strip()
                    if zh_name:
                        skill["name"] = zh_name
                        final_name = zh_name
                    else:
                        final_name = skill_name_en if skill_name_en else skill_name
            
            # 检查描述是否需要中文化（先检查是否已有中文描述）
            if is_chinese(skill.get("description", "")):
                print(f"  ✓ 技能 {i+1} 已有中文描述")
            elif not is_chinese(skill.get("description", "")):
                print(f"  技能 '{skill.get('name')}' 的描述不是中文")
                zh_desc = input(f"  请输入技能的中文描述 (回车跳过): ").strip()
                if zh_desc:
                    skill["description"] = zh_desc
            
            # 处理图片（使用最终确定的名称）
            if not skill.get("image") or skill.get("image") == "":
                # 先检查images文件夹中是否已有该图片（用最终名称）
                if check_image_exists(final_name):
                    print(f"  ✓ 在images文件夹找到技能图片: {final_name}.jpg")
                    skill["image"] = f"images/{final_name}.jpg"
                # 如果用英文名能找到，重命名为中文名
                elif skill_name_en and check_image_exists(skill_name_en) and final_name != skill_name_en:
                    print(f"  ✓ 找到英文图片，重命名为中文名")
                    if rename_image_file(skill_name_en, final_name):
                        skill["image"] = f"images/{final_name}.jpg"
                # 如果本地没有，尝试下载
                elif skill_name_en:
                    image_url = f"https://cdn.bazaardb.gg/static/cards/{skill_name_en.lower().replace(' ', '-')}.webp"
                    print(f"  尝试下载技能图片: {skill_name_en} -> {final_name}.jpg")
                    success = download_and_convert_image(image_url, final_name)
                    if success:
                        skill["image"] = f"images/{final_name}.jpg"
                    else:
                        # 下载失败，询问用户是否手动输入URL
                        manual_url = input(f"  下载失败，输入图片URL手动下载 (回车跳过): ").strip()
                        if manual_url:
                            if download_and_convert_image(manual_url, final_name):
                                skill["image"] = f"images/{final_name}.jpg"
        
        # 处理物品图片和中文化
        print(f"\n开始处理物品图片和中文化...")
        for i, item in enumerate(monster_info.get("items", [])):
            item_name = item.get("name", "")
            item_name_en = item.get("name_en", "")
            
            # 如果物品名已经是中文，跳过更新检查和中文化询问
            if is_chinese(item_name):
                print(f"  ✓ 物品 {i+1} 已有中文名: {item_name}")
                final_name = item_name
            else:
                # 先从所有怪物中查找是否有相同物品的中文信息
                found_in_encounters = False
                for enc_key, enc_data in existing_encounters.items():
                    for enc_item in enc_data.get("items", []):
                        if (enc_item.get("name_en") == item_name_en or 
                            enc_item.get("name_en") == item_name):
                            if is_chinese(enc_item.get("name", "")):
                                item["name"] = enc_item["name"]
                                print(f"  ✓ 物品 {i+1} 从其他怪物复用中文名: {enc_item['name']}")
                                found_in_encounters = True
                            if is_chinese(enc_item.get("description", "")):
                                item["description"] = enc_item["description"]
                                print(f"  ✓ 物品 {i+1} 从其他怪物复用中文描述: {enc_item['description'][:30]}...")
                            if found_in_encounters:
                                break
                    if found_in_encounters:
                        break
                
                # 如果在其他怪物中没找到，再尝试从当前怪物的现有数据中获取（更新模式）
                if not found_in_encounters and is_update and existing_key:
                    existing_items = existing_encounters.get(existing_key, {}).get("items", [])
                    for ex_item in existing_items:
                        if (ex_item.get("name_en") == item_name_en or 
                            ex_item.get("name") == item_name or
                            ex_item.get("name") == item_name_en):
                            if is_chinese(ex_item.get("name", "")):
                                item["name"] = ex_item["name"]
                                print(f"  ✓ 物品 {i+1} 保留已有中文名: {ex_item['name']}")
                            if is_chinese(ex_item.get("description", "")):
                                item["description"] = ex_item["description"]
                                print(f"  ✓ 物品 {i+1} 保留已有中文描述")
                            break
                
                # 先处理中文化，确定最终使用的名称
                final_name = item.get("name", item_name)
                if not is_chinese(final_name):
                    print(f"\n  物品 {i+1}: '{final_name}' 没有中文名")
                    zh_name = input(f"  请输入物品的中文名 (回车跳过): ").strip()
                    if zh_name:
                        item["name"] = zh_name
                        final_name = zh_name
                    else:
                        final_name = item_name_en if item_name_en else item_name
            
            # 检查描述是否需要中文化（先检查是否已有中文描述）
            if is_chinese(item.get("description", "")):
                print(f"  ✓ 物品 {i+1} 已有中文描述")
            elif not is_chinese(item.get("description", "")):
                print(f"  物品 '{item.get('name')}' 的描述不是中文")
                zh_desc = input(f"  请输入物品的中文描述 (回车跳过): ").strip()
                if zh_desc:
                    item["description"] = zh_desc
            
            # 处理图片（使用最终确定的名称）
            if not item.get("image") or item.get("image") == "":
                # 先检查images文件夹中是否已有该图片（用最终名称）
                if check_image_exists(final_name):
                    print(f"  ✓ 在images文件夹找到物品图片: {final_name}.jpg")
                    item["image"] = f"images/{final_name}.jpg"
                # 如果用英文名能找到，重命名为中文名
                elif item_name_en and check_image_exists(item_name_en) and final_name != item_name_en:
                    print(f"  ✓ 找到英文图片，重命名为中文名")
                    if rename_image_file(item_name_en, final_name):
                        item["image"] = f"images/{final_name}.jpg"
                # 如果本地没有，尝试下载
                elif item_name_en:
                    image_url = f"https://cdn.bazaardb.gg/static/cards/{item_name_en.lower().replace(' ', '-')}.webp"
                    print(f"  尝试下载物品图片: {item_name_en} -> {final_name}.jpg")
                    success = download_and_convert_image(image_url, final_name)
                    if success:
                        item["image"] = f"images/{final_name}.jpg"
                    else:
                        # 下载失败，询问用户是否手动输入URL
                        manual_url = input(f"  下载失败，输入图片URL手动下载 (回车跳过): ").strip()
                        if manual_url:
                            if download_and_convert_image(manual_url, final_name):
                                item["image"] = f"images/{final_name}.jpg"
            
            # 更新items_db.json中的图片路径（使用最终的中文名）
            if item.get("image"):
                update_items_db_image(final_name, item["image"])
            
            # 检查描述是否需要中文化
            if not is_chinese(item.get("description", "")):
                print(f"  物品 '{item.get('name')}' 的描述不是中文")
                zh_desc = input(f"  请输入物品的中文描述 (回车跳过): ").strip()
                if zh_desc:
                    item["description"] = zh_desc
        
        # 更新 combat_encounters.json
        if is_update:
            # 更新现有怪物
            existing_encounters[monster_key] = monster_info
            updated_encounters = existing_encounters
        else:
            # 将新怪物数据插入到最前面
            updated_encounters = {}
            updated_encounters.update(monster_json)  # 新怪物数据在前
            updated_encounters.update(existing_encounters)  # 现有数据在后
        
        # 保存回 combat_encounters.json
        with open(encounters_file, 'w', encoding='utf-8') as f:
            json.dump(updated_encounters, f, ensure_ascii=False, indent=2)
        
        action_word = "更新" if is_update else "添加"
        print(f"\n生成成功! 怪物数据已{action_word}到: {encounters_file}")
        print("\n生成的JSON:")
        print(json.dumps(monster_json, ensure_ascii=False, indent=2))
        
        print("\n使用说明:")
        print(f"1. 怪物数据已{action_word} combat_encounters.json 文件")
        print("2. 查看debug.html了解网页结构")
        print("3. 手动调整Socket位置和物品匹配")
        print("4. 图片已自动下载并转换为JPG格式")
        print("5. 数据增强已自动完成")
    else:
        print("生成失败!")

if __name__ == "__main__":
    main()