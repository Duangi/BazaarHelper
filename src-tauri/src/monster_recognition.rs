use image::{DynamicImage, GenericImageView, GenericImage, ImageBuffer, RgbaImage};
use imageproc::corners::corners_fast9;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use rayon::prelude::*;

// 识别结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonsterRecognitionResult {
    pub position: u8,
    pub name: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadingProgress {
    pub loaded: usize,
    pub total: usize,
    pub is_complete: bool,
    pub current_name: String,
}

#[derive(Clone, Serialize, Deserialize)]
struct TemplateCache {
    name: String,
    day: String,
    descriptors: Vec<([u8; 32], (u32, u32))>, // (BRIEF描述子, 坐标)
    // 保存一份样本 PNG，用于调试时做可视化对比（序列化到二进制缓存）
    sample_png: Vec<u8>,
    sample_w: u32,
    sample_h: u32,
}

#[derive(Deserialize)]
struct MonsterEntry {
    image: Option<String>,
    available: Option<String>,
}

static TEMPLATE_CACHE: OnceLock<Vec<TemplateCache>> = OnceLock::new();
static LOADING_PROGRESS: OnceLock<Arc<Mutex<LoadingProgress>>> = OnceLock::new();

pub fn get_loading_progress() -> LoadingProgress {
    LOADING_PROGRESS
        .get()
        .and_then(|p| p.lock().ok())
        .map(|p| p.clone())
        .unwrap_or(LoadingProgress {
            loaded: 0,
            total: 0,
            is_complete: false,
            current_name: "".to_string(),
        })
}

fn compute_brief(img: &image::GrayImage, x: u32, y: u32) -> Option<[u8; 32]> {
    if x < 16 || y < 16 || x > img.width() - 17 || y > img.height() - 17 {
        return None;
    }
    let mut desc = [0u8; 32];
    for i in 0..256 {
        let p1 = img.get_pixel(x + (i % 15) - 7, y + (i / 15 % 15) - 7);
        let p2 = img.get_pixel(x + (i % 13) - 6, y + (i / 13 % 13) - 6);
        if p1.0[0] > p2.0[0] {
            desc[(i / 8) as usize] |= 1 << (i % 8);
        }
    }
    Some(desc)
}

fn extract_features(img: &DynamicImage) -> Vec<([u8; 32], (u32, u32))> {
    let gray = img.to_luma8();
    // 使用 30 作为阈值，在“灵敏度”和“抗噪”之间取得平衡
    let corners = corners_fast9(&gray, 30);
    let mut features = Vec::new();
    for corner in corners {
        if let Some(desc) = compute_brief(&gray, corner.x, corner.y) {
            features.push((desc, (corner.x, corner.y)));
        }
        // 关键改进：将上限提升到 1000。
        // 这样可以确保即便有很多边缘点，程序也能继续扫到中间的怪物图案。
        if features.len() > 1000 { break; }
    }
    features
}

pub async fn preload_templates_async(resources_dir: PathBuf, cache_dir: PathBuf) -> Result<(), String> {
    let progress = Arc::new(Mutex::new(LoadingProgress {
        loaded: 0,
        total: 0,
        is_complete: false,
        current_name: "".to_string(),
    }));
    let _ = LOADING_PROGRESS.set(progress.clone());

    // 1. 尝试从二进制缓存加载 (极快)
    let cache_file = cache_dir.join("monster_features.bin");
    if cache_file.exists() {
        if let Ok(data) = std::fs::read(&cache_file) {
            if let Ok(cached_templates) = bincode::deserialize::<Vec<TemplateCache>>(&data) {
                println!("从缓存加载了 {} 个怪物特征点模板", cached_templates.len());
                if let Ok(mut p) = progress.lock() {
                    p.loaded = cached_templates.len();
                    p.total = cached_templates.len();
                    p.is_complete = true;
                }
                let _ = TEMPLATE_CACHE.set(cached_templates);
                return Ok(());
            }
        }
    }

    // 2. 如果缓存不存在或加载失败，从原始图片加载 (使用 Rayon 并行)
    let db_path = resources_dir.join("monsters_db.json");
    let json_content = std::fs::read_to_string(&db_path)
        .map_err(|e| format!("读取 monsters_db.json 失败: {}", e))?;

    let monsters: HashMap<String, MonsterEntry> = serde_json::from_str(&json_content)
        .map_err(|e| format!("解析 monsters_db.json 失败: {}", e))?;

    let mut image_tasks = Vec::new();
    for (key, entry) in monsters.iter() {
        if let (Some(rel_path), Some(day)) = (&entry.image, &entry.available) {
            let full_path = resources_dir.join(rel_path);
            if full_path.exists() {
                image_tasks.push((key.clone(), day.clone(), full_path));
            }
        }
    }

    let total = image_tasks.len();
    if let Ok(mut p) = progress.lock() { p.total = total; }

    println!("缓存未命中，开始并行计算 {} 个特征点模板...", total);

    // 使用 Rayon 并行处理所有图片
    let cache: Vec<TemplateCache> = image_tasks.into_par_iter().map(|(name, day, path)| {
        let mut descriptors = Vec::new();
        let mut sample_png = Vec::new();
        let mut sample_w = 0u32;
        let mut sample_h = 0u32;
        if let Ok(img) = image::open(&path) {
            descriptors = extract_features(&img);
            sample_w = img.width();
            sample_h = img.height();
            // 直接读取原始文件字节，后续可以用 image::load_from_memory 加载（支持 jpg/png 等）
            sample_png = std::fs::read(&path).unwrap_or_default();
        }
        
        // 更新进度 (原子锁)
        if let Some(p_arc) = LOADING_PROGRESS.get() {
            if let Ok(mut p) = p_arc.lock() {
                p.loaded += 1;
                p.current_name = name.clone();
            }
        }
        
        TemplateCache { name, day, descriptors, sample_png, sample_w, sample_h }
    }).collect();

    // 3. 保存到二进制缓存以备下次使用
    let _ = std::fs::create_dir_all(&cache_dir);
    if let Ok(encoded) = bincode::serialize(&cache) {
        let _ = std::fs::write(&cache_file, encoded);
        println!("特征点模板已保存到缓存: {:?}", cache_file);
    }

    if let Ok(mut p) = progress.lock() { p.is_complete = true; }
    let _ = TEMPLATE_CACHE.set(cache);
    println!("特征点模板加载完成");
    Ok(())
}

fn hamming_distance(a: &[u8; 32], b: &[u8; 32]) -> u32 {
    let mut dist = 0;
    for i in 0..32 {
        dist += (a[i] ^ b[i]).count_ones();
    }
    dist
}

pub fn recognize_monsters(day_filter: Option<String>) -> Result<Vec<MonsterRecognitionResult>, String> {
    use xcap::Window;
    use std::time::Instant;

    let start_total = Instant::now();

    let windows = Window::all().map_err(|e| e.to_string())?;
    let bazaar_window = windows.into_iter().find(|w| {
        let title = w.title().to_lowercase();
        let app_name = w.app_name().to_lowercase();
        
        // 排除常见的干扰窗口
        let is_excluded = 
            title.contains("visual studio code") || app_name.contains("visual studio code") ||
            title.contains("obs") || app_name.contains("obs") ||
            title.contains("mediaplayer") || app_name.contains("mediaplayer") ||
            title.contains("bazaarhelper") || app_name.contains("bazaarhelper");

        let is_bazaar = 
            title.contains("the bazaar") || title.contains("thebazaar") || 
            app_name.contains("the bazaar") || app_name.contains("thebazaar");

        is_bazaar && !is_excluded
    });

    let start_capture = Instant::now();
    let screenshot = if let Some(window) = bazaar_window {
        println!("[Recognition] Found window: '{}' (App: '{}'), Pos: {:?}, Size: {:?}", 
                 window.title(), window.app_name(), (window.x(), window.y()), (window.width(), window.height()));
        window.capture_image().map_err(|e| {
            println!("[Recognition] Error capturing window: {}. Ensure screen recording permission is granted.", e);
            e.to_string()
        })?
    } else {
        println!("[Recognition] 'The Bazaar' window not found among {} windows, falling back to monitor 0", Window::all().unwrap().len());
        use xcap::Monitor;
        let monitors = Monitor::all().map_err(|e| e.to_string())?;
        if monitors.is_empty() { return Err("No monitor found".into()); }
        monitors[0].capture_image().map_err(|e| e.to_string())?
    };
    println!("[Timer] 截图耗时: {:?}", start_capture.elapsed());

    let img = DynamicImage::ImageRgba8(screenshot);
    let (width, height) = img.dimensions();

    let full_cache = TEMPLATE_CACHE.get().ok_or("Templates not loaded")?;
    let cache: Vec<&TemplateCache> = if let Some(ref target_day) = day_filter {
        // 合并策略：如果用户选择了 Day 10+，则包含库中所有标记为 Day 10 和 Day 10+ 的怪物
        if target_day == "Day 10+" {
            full_cache.iter().filter(|t| t.day == "Day 10" || t.day == "Day 10+").collect()
        } else {
            full_cache.iter().filter(|t| t.day == *target_day).collect()
        }
    } else {
        full_cache.iter().collect()
    };
    println!("[Recognition] 开始匹配，库中共有 {} 个目标怪兽", cache.len());

    let mut results = Vec::new();
    let region_y = (height as f32 * 0.15) as u32;
    let region_h = (height as f32 * 0.35) as u32;
    let total_region_w = (width as f32 * (5.0 / 12.0)) as u32;
    let region_x_start = (width as f32 * (0.5 - 5.0 / 24.0)) as u32;

    let slot_w = total_region_w / 3;
    let slot_h = region_h;

    let start_match = Instant::now();
    // Debug output directory (开发时将输出用于比对的图片)
    let _ = std::fs::create_dir_all("target/debug/monster_debug");
    // 保存整张截图用于调试
    let _ = img.save("target/debug/monster_debug/screenshot.png");
    for i in 0..3 {
        let start_slot = Instant::now();
        let x = region_x_start + (i as u32 * slot_w);
        let y = region_y;
        if x + slot_w > width || y + slot_h > height { continue; }

        let slice = img.crop_imm(x, y, slot_w, slot_h);
        
        let scene_features = extract_features(&slice);
        
        if scene_features.is_empty() { continue; }

        let mut best_name = "Unknown".to_string();
        let mut max_matches = 0;

        for template in &cache {
            let mut matches = 0;
            for (scene_desc, _) in &scene_features {
                for (temp_desc, _) in &template.descriptors {
                    // Hamming 距离匹配逻辑
                    if hamming_distance(scene_desc, temp_desc) < 40 {
                        matches += 1;
                        break; 
                    }
                }
            }
            if matches > max_matches {
                max_matches = matches;
                best_name = template.name.clone();
            }
        }

        let confidence = (max_matches as f32 / 50.0).min(1.0);
        println!("[Slot {}] 识别得出: '{}', 匹配点数: {}, 耗时: {:?}", 
                 i + 1, best_name, max_matches, start_slot.elapsed());

        // 核心修正：降低判定门槛到 5。
        // 因为我们已经按天过滤了怪兽（只有8个目标），所以匹配点数为 6 已经具有极高的置信度。
        // Debug: 保存每个槽位的切片以及与最佳模板的并列对比图（若找到模板样本）
        let slot_scene_path = format!("target/debug/monster_debug/slot_{}_scene.png", i + 1);
        let _ = slice.save(&slot_scene_path);

        if let Some(found_template) = full_cache.iter().find(|t| t.name == best_name) {
            if !found_template.sample_png.is_empty() {
                if let Ok(tmpl_img) = image::load_from_memory(&found_template.sample_png) {
                    // 将模板缩放到槽位大小，以便直观对比
                    let resized_tmpl = tmpl_img.resize_exact(slot_w, slot_h, image::imageops::FilterType::Lanczos3);
                    let tmpl_path = format!("target/debug/monster_debug/slot_{}_template.png", i + 1);
                    let _ = resized_tmpl.save(&tmpl_path);

                    // 合成左右并排图像
                    let mut pair: RgbaImage = ImageBuffer::new(slot_w * 2, slot_h);
                    let left: RgbaImage = slice.to_rgba8();
                    // resized_tmpl is an ImageBuffer; convert/cast to RgbaImage if necessary
                    let right: RgbaImage = resized_tmpl.to_rgba8();
                    let _ = pair.copy_from(&left, 0, 0);
                    let _ = pair.copy_from(&right, slot_w, 0);
                    let pair_path = format!("target/debug/monster_debug/slot_{}_pair.png", i + 1);
                    let _ = pair.save(&pair_path);
                }
            }
        }

        if max_matches >= 5 {
            results.push(MonsterRecognitionResult {
                position: (i + 1) as u8,
                name: best_name,
                confidence,
            });
        }
    }
    
    println!("[Timer] 特征提取与比对总耗时: {:?}", start_match.elapsed());
    println!("[Timer] 识别流程整体耗时: {:?}", start_total.elapsed());

    Ok(results)
}
