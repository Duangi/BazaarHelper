// 按size分类的卡牌识别模块
use super::monster_recognition::*;
use opencv::prelude::*;
use serde::Deserialize;
use std::path::PathBuf;
use rayon::prelude::*;

// 按size分类加载卡牌特征
pub async fn preload_card_templates_by_size_async(resources_dir: PathBuf, cache_dir: PathBuf) -> Result<(), String> {
    log_to_file(&format!("Start loading card templates by size. Resource Dir: {:?}, Cache Dir: {:?}", resources_dir, cache_dir));
    
    // 定义三个缓存文件
    let small_cache_file = cache_dir.join("card_features_small.bin");
    let medium_cache_file = cache_dir.join("card_features_medium.bin");
    let large_cache_file = cache_dir.join("card_features_large.bin");
    
    let small_bundled = resources_dir.join("card_features_small.bin");
    let medium_bundled = resources_dir.join("card_features_medium.bin");
    let large_bundled = resources_dir.join("card_features_large.bin");
    
    // 尝试从缓存加载
    let mut loaded_from_cache = true;
    
    // Small
    if !try_load_cache(&small_bundled, &small_cache_file, &super::monster_recognition::CARD_SMALL_CACHE) {
        loaded_from_cache = false;
    }
    
    // Medium
    if !try_load_cache(&medium_bundled, &medium_cache_file, &super::monster_recognition::CARD_MEDIUM_CACHE) {
        loaded_from_cache = false;
    }
    
    // Large
    if !try_load_cache(&large_bundled, &large_cache_file, &super::monster_recognition::CARD_LARGE_CACHE) {
        loaded_from_cache = false;
    }
    
    if loaded_from_cache {
        log_to_file("All card size caches loaded successfully");
        return Ok(());
    }
    
    // 需要重新生成特征
    let db_path = resources_dir.join("items_db.json");
    if !db_path.exists() {
        return Err(format!("items_db.json not found at {:?}", db_path));
    }
    
    let json_content = std::fs::read_to_string(&db_path)
        .map_err(|e| format!("读取 items_db.json 失败: {}", e))?;
    
    #[derive(Deserialize)]
    struct RawItemWithSize {
        id: String,
        name_cn: Option<String>,
        size: Option<String>,
    }
    
    let items: Vec<RawItemWithSize> = serde_json::from_str(&json_content)
        .map_err(|e| format!("解析 items_db.json 失败: {}", e))?;
    
    // 按size分类
    let mut small_tasks = Vec::new();
    let mut medium_tasks = Vec::new();
    let mut large_tasks = Vec::new();
    
    for item in items {
        let img_path = resources_dir.join("images").join(format!("{}.webp", item.id));
        if !img_path.exists() { continue; }
        
        let task = (item.name_cn.unwrap_or_else(|| item.id.clone()), item.id.clone(), img_path);
        
        if let Some(size) = &item.size {
            if size.contains("Small") || size.contains("小型") {
                small_tasks.push(task);
            } else if size.contains("Medium") || size.contains("中型") {
                medium_tasks.push(task);
            } else if size.contains("Large") || size.contains("大型") {
                large_tasks.push(task);
            }
        }
    }
    
    log_to_file(&format!("Building card caches: Small={}, Medium={}, Large={}", 
                         small_tasks.len(), medium_tasks.len(), large_tasks.len()));
    
    // 并行构建特征
    let small_cache = build_feature_cache(small_tasks);
    let medium_cache = build_feature_cache(medium_tasks);
    let large_cache = build_feature_cache(large_tasks);
    
    log_to_file(&format!("Successfully built: Small={}, Medium={}, Large={}", 
                         small_cache.len(), medium_cache.len(), large_cache.len()));
    
    // 保存缓存
    save_cache(&small_cache, &small_cache_file, &small_bundled, "Small");
    save_cache(&medium_cache, &medium_cache_file, &medium_bundled, "Medium");
    save_cache(&large_cache, &large_cache_file, &large_bundled, "Large");
    
    // 设置到全局缓存
    let _ = super::monster_recognition::CARD_SMALL_CACHE.set(small_cache);
    let _ = super::monster_recognition::CARD_MEDIUM_CACHE.set(medium_cache);
    let _ = super::monster_recognition::CARD_LARGE_CACHE.set(large_cache);
    
    Ok(())
}

fn try_load_cache(bundled: &PathBuf, cache_file: &PathBuf, cache: &std::sync::OnceLock<Vec<TemplateCache>>) -> bool {
    // 优先从bundled加载
    if bundled.exists() {
        if let Ok(data) = std::fs::read(bundled) {
            if let Ok(templates) = bincode::deserialize::<Vec<TemplateCache>>(&data) {
                if !templates.is_empty() {
                    log_to_file(&format!("Loaded {} templates from {:?}", templates.len(), bundled));
                    let _ = cache.set(templates);
                    return true;
                }
            }
        }
    }
    
    // 尝试从cache_file加载
    if cache_file.exists() {
        if let Ok(data) = std::fs::read(cache_file) {
            if let Ok(templates) = bincode::deserialize::<Vec<TemplateCache>>(&data) {
                if !templates.is_empty() {
                    log_to_file(&format!("Loaded {} templates from {:?}", templates.len(), cache_file));
                    let _ = cache.set(templates);
                    return true;
                }
            }
        }
    }
    
    false
}

fn build_feature_cache(tasks: Vec<(String, String, PathBuf)>) -> Vec<TemplateCache> {
    tasks.into_par_iter().filter_map(|(name, id, path)| {
        let path_str = path.to_str()?;
        match extract_features_orb(path_str, 300) {
            Ok((keypoints, descriptors, rows, cols)) => {
                Some(TemplateCache {
                    name,
                    day: id,
                    keypoints,
                    descriptors,
                    descriptor_rows: rows,
                    descriptor_cols: cols,
                    sample_png: Vec::new(),
                    sample_w: 0,
                    sample_h: 0,
                })
            }
            Err(_) => None,
        }
    }).collect()
}

fn save_cache(cache: &Vec<TemplateCache>, cache_file: &PathBuf, bundled: &PathBuf, label: &str) {
    if let Ok(serialized) = bincode::serialize(cache) {
        let _ = std::fs::write(cache_file, &serialized);
        let _ = std::fs::write(bundled, &serialized);
        log_to_file(&format!("Saved {} card cache: appdata={:?}, resources={:?}", 
                             label, cache_file, bundled));
    }
}

// 根据size匹配卡牌
pub fn match_card_by_size(scene_desc: &Mat, size: &str) -> Result<Option<serde_json::Value>, String> {
    let cache = match size {
        "Small" => super::monster_recognition::CARD_SMALL_CACHE.get(),
        "Medium" => super::monster_recognition::CARD_MEDIUM_CACHE.get(),
        "Large" => super::monster_recognition::CARD_LARGE_CACHE.get(),
        _ => return Err(format!("Unknown size: {}", size)),
    }.ok_or(format!("{} card templates not loaded", size))?;
    
    println!("[Card Match] Matching against {} {} cards", cache.len(), size);
    
    let mut results: Vec<(&TemplateCache, usize, f32)> = Vec::new();
    
    for template in cache {
        if template.descriptors.is_empty() { continue; }
        use opencv::core::CV_8U;
        let mut template_desc = match unsafe { Mat::new_rows_cols(template.descriptor_rows, template.descriptor_cols, CV_8U) } {
            Ok(m) => m,
            Err(_) => continue,
        };
        unsafe { std::ptr::copy_nonoverlapping(template.descriptors.as_ptr(), template_desc.data_mut() as *mut u8, template.descriptors.len()); }
        
        if let Ok(matches) = match_orb_descriptors(&scene_desc, &template_desc) {
            let min_kp = (template.descriptor_rows as f32).min(scene_desc.rows() as f32);
            let confidence = if min_kp > 0.0 { matches as f32 / min_kp } else { 0.0 };
            results.push((template, matches, confidence));
        }
    }
    
    results.sort_by(|a, b| b.1.cmp(&a.1));
    
    let mut matches_found = Vec::new();
    for i in 0..results.len().min(10) {
        let (top, matches, confidence) = results[i];
        if matches > 12 && confidence > 0.12 {
            matches_found.push(serde_json::json!({
                "id": top.day,
                "name": top.name,
                "confidence": confidence,
                "match_count": matches
            }));
        }
        if matches_found.len() >= 3 { break; }
    }
    
    if !matches_found.is_empty() {
        println!("[Card Match] Found {} matches in {} category (best: {:?})", 
                 matches_found.len(), size, matches_found[0]);
        return Ok(Some(serde_json::json!(matches_found)));
    }
    Ok(None)
}
