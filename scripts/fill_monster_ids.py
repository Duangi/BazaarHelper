import json
import os
import uuid

def fill_monster_db_ids():
    path = r"d:\Projects\BazaarHelper\src-tauri\resources\monsters_db.json"
    if not os.path.exists(path):
        return
        
    with open(path, 'r', encoding='utf-8-sig') as f:
        monsters = json.load(f)
        
    changes = 0
    name_to_id_local = {}
    
    # Fill item/skill IDs
    for m_name, m_data in monsters.items():
        # Monster itself needs ID
        if "id" not in m_data or not m_data["id"]:
            m_data["id"] = str(uuid.uuid5(uuid.NAMESPACE_DNS, m_name))
            changes += 1
            
        for list_key in ["items", "skills"]:
            if list_key in m_data:
                for it in m_data[list_key]:
                    it_id = it.get("id")
                    name = it.get("name")
                    if not it_id and name:
                        new_id = str(uuid.uuid5(uuid.NAMESPACE_DNS, name))
                        it["id"] = new_id
                        changes += 1
                            
    if changes > 0:
        with open(path, 'w', encoding='utf-8') as f:
            json.dump(monsters, f, ensure_ascii=False, indent=2)
        print(f"Filled {changes} missing IDs in monsters_db.json")

if __name__ == "__main__":
    fill_monster_db_ids()
