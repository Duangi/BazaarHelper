import json

# 读取数据库
with open('src-tauri/resources/monsters_db.json', encoding='utf-8') as f:
    db = json.load(f)

# 要查找的英文名
test_names = ['Pyro', 'Banannibal', 'Boarrior', 'Ahexa', 'Haunted Kimono']

print("查找英文名对应的中文名:")
for cn_name, data in db.items():
    en_name = data.get('name', '')
    if en_name in test_names:
        print(f"  {en_name} -> {cn_name}")
