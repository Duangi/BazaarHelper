use image::{DynamicImage, GenericImageView};
use imageproc::corners::corners_fast9;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

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

#[derive(Clone)]
struct TemplateCache {
    name: String,
    day: String,
    descriptors: Vec<([u8; 32], (u32, u32))>, // (BRIEF描述子, 坐标)
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

pub async fn preload_templates_async(resources_dir: PathBuf) -> Result<(), String> {
    let progress = Arc::new(Mutex::new(LoadingProgress {
        loaded: 0,
        total: 0,
        is_complete: false,
        current_name: "".to_string(),
    }));
    let _ = LOADING_PROGRESS.set(progress.clone());

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

    println!("开始加载 {} 个特征点模板 (FAST + BRIEF)...", total);

    let mut cache = Vec::new();
    for (name, day, path) in image_tasks {
        if let Ok(mut p) = progress.lock() { p.current_name = name.clone(); }
        if let Ok(img) = image::open(&path) {
            let descriptors = extract_features(&img);
            cache.push(TemplateCache { name, day, descriptors });
        }
        if let Ok(mut p) = progress.lock() { p.loaded += 1; }
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
        title.contains("the bazaar") || title.contains("thebazaar")
    });

    let start_capture = Instant::now();
    let screenshot = if let Some(window) = bazaar_window {
        println!("[Recognition] Found window: {}", window.title());
        window.capture_image().map_err(|e| e.to_string())?
    } else {
        println!("[Recognition] 'The Bazaar' window not found, falling back to monitor 0");
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
    for i in 0..3 {
        let start_slot = Instant::now();
        let x = region_x_start + (i as u32 * slot_w);
        let y = region_y;
        if x + slot_w > width || y + slot_h > height { continue; }

        let slice = img.crop_imm(x, y, slot_w, slot_h);
        
        // 保存调试图片到 target/debug/monster_debug
        let debug_dir = PathBuf::from("target/debug/monster_debug");
        if !debug_dir.exists() {
            let _ = std::fs::create_dir_all(&debug_dir);
        }
        let debug_path = debug_dir.join(format!("slot_{}.png", i + 1));
        let _ = slice.save(&debug_path);
        println!("[Debug] Saved slot {} image to: {:?}", i + 1, debug_path);

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
