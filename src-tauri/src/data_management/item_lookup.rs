use crate::{ItemData, ItemDb, SkillDb};

pub fn lookup_item(tid: &str, items_db: &ItemDb, skills_db: &SkillDb) -> Option<ItemData> {
    if let Some(&index) = items_db.id_map.get(tid) {
        return items_db.list.get(index).cloned();
    }
    if let Some(&index) = skills_db.id_map.get(tid) {
        return skills_db.list.get(index).cloned();
    }
    None
}
