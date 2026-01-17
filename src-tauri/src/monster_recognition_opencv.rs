use image::{DynamicImage, GenericImageView, GenericImage, ImageBuffer, RgbaImage};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use rayon::prelude::*;
use opencv::{
    core::{Mat, Vector, KeyPoint, DMatch, NORM_HAMMING, Vec2i},
    features2d::{ORB, BFMatcher, ORB_Trait, DescriptorMatcher},
    imgcodecs::{imread, imdecode, IMREAD_GRAYSCALE},
    prelude::*,
};

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
    keypoints: Vec<(f32, f32)>, // (x, y) 坐标
    descriptors: Vec<u8>, // OpenCV Mat 序列化为字节数组
    descriptor_rows: i32,
    descriptor_cols: i32,
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

// 使用 OpenCV ORB 提取特征点和描述符
fn extract_features_orb(image_path: &str) -> Result<(Vec<(f32, f32)>, Vec<u8>, i32, i32), opencv::Error> {
    // 读取灰度图 (使用 imdecode 以支持中文路径)
    let content = std::fs::read(image_path).map_err(|e| opencv::Error::new(opencv::core::StsError, format!("Read error: {}", e)))?;
    let img = imdecode(&Mat::from_slice(&content)?, IMREAD_GRAYSCALE)?;
    
    if img.empty() {
        return Ok((Vec::new(), Vec::new(), 0, 0));
    }

    // 初始化 ORB (nfeatures=500, scaleFactor=1.2)
    let mut orb = <dyn ORB>::create(500, 1.2f32, 8, 31, 0, 2, 0, 31, true)?;

    // 提取特征点和描述符
    let mut keypoints = Vector::<KeyPoint>::new();
    let mut descriptors = Mat::default();
    let mask = Mat::default();

    orb.detect_and_compute(&img, &mask, &mut keypoints, &mut descriptors, false)?;

    if descriptors.empty() {
        return Ok((Vec::new(), Vec::new(), 0, 0));
    }

    // 转换 keypoints 为简单的 (x, y) 坐标
    let kp_coords: Vec<(f32, f32)> = keypoints
        .iter()
        .map(|kp| (kp.pt.x, kp.pt.y))
        .collect();

    // 将 Mat 描述符转换为字节数组以便序列化
    let rows = descriptors.rows();
    let cols = descriptors.cols();
    let mut desc_bytes = Vec::new();
    
    if !descriptors.empty() {
        // 将 Mat 数据拷贝到 Vec<u8>
        let size = (rows * cols) as usize;
        desc_bytes.reserve(size);
        unsafe {
            let ptr = descriptors.data() as *const u8;
            for i in 0..size {
                desc_bytes.push(*ptr.add(i));
            }
        }
    }

    Ok((kp_coords, desc_bytes, rows, cols))
}

// 从 DynamicImage 提取特征 (用于截图分析)
fn extract_features_from_dynamic_image(img: &DynamicImage) -> Result<Mat, opencv::Error> {
    // 将图像保存到临时缓冲区
    let mut bytes = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut bytes), image::ImageOutputFormat::Png)
        .map_err(|e| opencv::Error::new(opencv::core::StsError, format!("图像转换失败: {}", e)))?;
    
    // 使用 OpenCV 解码
    use opencv::imgcodecs::imdecode;
    let buf = Mat::from_slice(&bytes)?;
    let gray_img = imdecode(&buf, IMREAD_GRAYSCALE)?;
    
    if gray_img.empty() {
        return Ok(Mat::default());
    }

    // 初始化 ORB
    let mut orb = <dyn ORB>::create(500, 1.2f32, 8, 31, 0, 2, 0, 31, true)?;

    let mut keypoints = Vector::<KeyPoint>::new();
    let mut descriptors = Mat::default();
    let mask = Mat::default();

    orb.detect_and_compute(&gray_img, &mask, &mut keypoints, &mut descriptors, false)?;

    Ok(descriptors)
}

// ORB 匹配函数 - 使用 Lowe's Ratio Test
fn match_orb_descriptors(desc1: &Mat, desc2: &Mat) -> Result<usize, opencv::Error> {
    if desc1.empty() || desc2.empty() {
        return Ok(0);
    }

    // 创建 BFMatcher (Hamming 距离)
    let matcher = BFMatcher::create(NORM_HAMMING, false)?;
    
    // KNN 匹配，k=2
    let mut matches = Vector::<Vector::<DMatch>>::new();
    let mask = Mat::default();
    matcher.knn_match(desc1, desc2, &mut matches, 2, &mask, false)?;

    // Lowe's Ratio Test 过滤
    let mut good_matches = 0;
    for m in matches.iter() {
        if m.len() == 2 {
            let m0 = m.get(0)?;
            let m1 = m.get(1)?;
            if m0.distance < 0.75 * m1.distance {
                good_matches += 1;
            }
        }
    }

    Ok(good_matches)
}

pub async fn preload_templates_async(resources_dir: PathBuf, cache_dir: PathBuf) -> Result<(), String> {
    let progress = Arc::new(Mutex::new(LoadingProgress {
        loaded: 0,
        total: 0,
        is_complete: false,
        current_name: "".to_string(),
    }));
    let _ = LOADING_PROGRESS.set(progress.clone());

    // 1. 尝试从二进制缓存加载
    let cache_file = cache_dir.join("monster_features_opencv.bin");
    if cache_file.exists() {
        if let Ok(data) = std::fs::read(&cache_file) {
            if let Ok(cached_templates) = bincode::deserialize::<Vec<TemplateCache>>(&data) {
                // 如果缓存数量太少，可能是之前的 bug 导致的，强制重新加载
                if cached_templates.len() > 50 {
                    println!("从 OpenCV 缓存加载了 {} 个怪物特征点模板", cached_templates.len());
                    if let Ok(mut p) = progress.lock() {
                        p.loaded = cached_templates.len();
                        p.total = cached_templates.len();
                        p.is_complete = true;
                    }
                    let _ = TEMPLATE_CACHE.set(cached_templates);
                    return Ok(());
                } else {
                     println!("缓存中的模板数量过少 ({})，跳过缓存重新加载...", cached_templates.len());
                }
            }
        }
    }

    // 2. 从原始图片加载 (使用 Rayon 并行)
    let db_path = resources_dir.join("monsters_db.json");
    let json_content = std::fs::read_to_string(&db_path)
        .map_err(|e| format!("读取 monsters_db.json 失败: {}", e))?;

    let monsters: HashMap<String, MonsterEntry> = serde_json::from_str(&json_content)
        .map_err(|e| format!("解析 monsters_db.json 失败: {}", e))?;

    let mut image_tasks = Vec::new();
    for (key, entry) in monsters.iter() {
        if let Some(day) = &entry.available {
            let mut found_path = None;
            
            // 1. Try explicit path from DB
            if let Some(rel_path) = &entry.image {
                let p = resources_dir.join(rel_path);
                if p.exists() {
                    found_path = Some(p);
                }
            }
            
            // 2. Fallback: Try Character image (Chinese name)
            if found_path.is_none() {
                let char_path = resources_dir.join(format!("images_monster_char/{}.webp", key));
                if char_path.exists() {
                    found_path = Some(char_path);
                }
            }
            
            // 3. Fallback: Try Background image (Chinese name)
            if found_path.is_none() {
                let bg_path = resources_dir.join(format!("images_monster_bg/{}.webp", key));
                if bg_path.exists() {
                    found_path = Some(bg_path);
                }
            }

            if let Some(path) = found_path {
                image_tasks.push((key.clone(), day.clone(), path));
            }
        }
    }

    let total = image_tasks.len();
    if let Ok(mut p) = progress.lock() { p.total = total; }

    println!("缓存未命中，开始使用 OpenCV ORB 计算 {} 个特征点模板...", total);

    // 使用 Rayon 并行处理所有图片
    let cache: Vec<TemplateCache> = image_tasks.into_par_iter().filter_map(|(name, day, path)| {
        let path_str = path.to_str()?;
        
        // 使用 OpenCV 提取特征
        match extract_features_orb(path_str) {
            Ok((keypoints, descriptors, desc_rows, desc_cols)) => {
                // 读取原始图片数据用于调试
                let sample_png = std::fs::read(&path).unwrap_or_default();
                let (sample_w, sample_h) = if let Ok(img) = image::open(&path) {
                    (img.width(), img.height())
                } else {
                    (0, 0)
                };

                // 更新进度
                if let Some(p_arc) = LOADING_PROGRESS.get() {
                    if let Ok(mut p) = p_arc.lock() {
                        p.loaded += 1;
                        p.current_name = name.clone();
                    }
                }

                Some(TemplateCache {
                    name,
                    day,
                    keypoints,
                    descriptors,
                    descriptor_rows: desc_rows,
                    descriptor_cols: desc_cols,
                    sample_png,
                    sample_w,
                    sample_h,
                })
            }
            Err(e) => {
                println!("警告: 提取 {} 的特征失败: {}", name, e);
                None
            }
        }
    }).collect();

    // 3. 保存到二进制缓存
    let _ = std::fs::create_dir_all(&cache_dir);
    if let Ok(encoded) = bincode::serialize(&cache) {
        let _ = std::fs::write(&cache_file, encoded);
        println!("OpenCV 特征点模板已保存到缓存: {:?}", cache_file);
    }

    if let Ok(mut p) = progress.lock() { p.is_complete = true; }
    let _ = TEMPLATE_CACHE.set(cache);
    println!("OpenCV ORB 特征点模板加载完成");
    Ok(())
}

pub fn recognize_monsters(day_filter: Option<String>) -> Result<Vec<MonsterRecognitionResult>, String> {
    use xcap::Window;
    use std::time::Instant;

    let start_total = Instant::now();

    // 截图逻辑
    let windows = Window::all().map_err(|e| e.to_string())?;
    let bazaar_window = windows.into_iter().find(|w| {
        let title = w.title().to_lowercase();
        let app_name = w.app_name().to_lowercase();
        
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
        println!("[OpenCV Recognition] Found window: '{}' (App: '{}'), Pos: {:?}, Size: {:?}", 
                 window.title(), window.app_name(), (window.x(), window.y()), (window.width(), window.height()));
        window.capture_image().map_err(|e| {
            println!("[OpenCV Recognition] Error capturing window: {}. Ensure screen recording permission is granted.", e);
            e.to_string()
        })?
    } else {
        println!("[OpenCV Recognition] 'The Bazaar' window not found, attempting to capture monitor with mouse cursor (Dev Mode)");
        use xcap::Monitor;
        use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
        use windows::Win32::Foundation::POINT;

        let monitors = Monitor::all().map_err(|e| e.to_string())?;
        if monitors.is_empty() { return Err("No monitor found".into()); }

        let mut target_idx = 0;
        let mut point = POINT { x: 0, y: 0 };
        
        unsafe {
            if GetCursorPos(&mut point).as_bool() {
                let mx = point.x;
                let my = point.y;
                // Find which monitor contains the mouse
                if let Some(idx) = monitors.iter().position(|m| {
                    let x = m.x();
                    let y = m.y();
                    let w = m.width() as i32;
                    let h = m.height() as i32;
                    mx >= x && mx < x + w && my >= y && my < y + h
                }) {
                    target_idx = idx;
                    println!("[OpenCV Recognition] Mouse found at ({}, {}) on Monitor {}", mx, my, idx);
                }
            }
        }

        monitors[target_idx].capture_image().map_err(|e| e.to_string())?
    };
    println!("[Timer] 截图耗时: {:?}", start_capture.elapsed());

    let img = DynamicImage::ImageRgba8(screenshot);
    let (width, height) = img.dimensions();

    let full_cache = TEMPLATE_CACHE.get().ok_or("Templates not loaded")?;
    let cache: Vec<&TemplateCache> = if let Some(ref target_day) = day_filter {
        if target_day == "Day 10+" {
            full_cache.iter().filter(|t| t.day == "Day 10" || t.day == "Day 10+").collect()
        } else {
            full_cache.iter().filter(|t| t.day == *target_day).collect()
        }
    } else {
        full_cache.iter().collect()
    };
    println!("[OpenCV Recognition] 开始匹配，库中共有 {} 个目标怪兽", cache.len());

    let mut results = Vec::new();
    let region_y = (height as f32 * 0.15) as u32;
    let region_h = (height as f32 * 0.35) as u32;
    let total_region_w = (width as f32 * (5.0 / 12.0)) as u32;
    let region_x_start = (width as f32 * (0.5 - 5.0 / 24.0)) as u32;

    let slot_w = total_region_w / 3;
    let slot_h = region_h;

    let start_match = Instant::now();
    let _ = std::fs::create_dir_all("target/debug/monster_debug");
    let _ = img.save("target/debug/monster_debug/screenshot_opencv.png");

    for i in 0..3 {
        let start_slot = Instant::now();
        let x = region_x_start + (i as u32 * slot_w);
        let y = region_y;
        if x + slot_w > width || y + slot_h > height { continue; }

        let slice = img.crop_imm(x, y, slot_w, slot_h);
        
        // 使用 OpenCV 提取场景特征
        let scene_descriptors = match extract_features_from_dynamic_image(&slice) {
            Ok(desc) => desc,
            Err(e) => {
                println!("[Slot {}] 提取特征失败: {}", i + 1, e);
                continue;
            }
        };
        
        if scene_descriptors.empty() {
            println!("[Slot {}] 未检测到特征点", i + 1);
            continue;
        }

        let mut best_name = "Unknown".to_string();
        let mut max_matches = 0;
        let mut best_score = 0.0f32;

        // 遍历所有模板进行匹配
        for template in &cache {
            if template.descriptors.is_empty() {
                continue;
            }

            // 将模板描述符从字节数组重建为 Mat
            let template_desc = match Mat::from_slice_rows_cols(
                &template.descriptors,
                template.descriptor_rows,
                template.descriptor_cols,
            ) {
                Ok(mat) => mat,
                Err(_) => continue,
            };

            // 使用 ORB 匹配
            match match_orb_descriptors(&scene_descriptors, &template_desc) {
                Ok(matches) => {
                    if matches > max_matches {
                        max_matches = matches;
                        best_name = template.name.clone();
                        
                        // 计算置信度
                        let scene_kp_count = scene_descriptors.rows() as f32;
                        let template_kp_count = template.descriptor_rows as f32;
                        let min_kp = scene_kp_count.min(template_kp_count);
                        best_score = if min_kp > 0.0 {
                            matches as f32 / min_kp
                        } else {
                            0.0
                        };
                    }
                }
                Err(e) => {
                    println!("[警告] 匹配 {} 时出错: {}", template.name, e);
                }
            }
        }

        println!("[Slot {}] OpenCV ORB 识别得出: '{}', 匹配点数: {}, 置信度: {:.2}%, 耗时: {:?}", 
                 i + 1, best_name, max_matches, best_score * 100.0, start_slot.elapsed());

        // 保存调试图像
        let slot_scene_path = format!("target/debug/monster_debug/slot_{}_scene_opencv.png", i + 1);
        let _ = slice.save(&slot_scene_path);

        // 阈值判定：匹配数 >= 10 或 置信度 > 0.15
        if max_matches >= 10 || best_score > 0.15 {
            results.push(MonsterRecognitionResult {
                position: (i + 1) as u8,
                name: best_name,
                confidence: best_score,
            });
        }
    }
    
    println!("[Timer] OpenCV 特征提取与比对总耗时: {:?}", start_match.elapsed());
    println!("[Timer] OpenCV 识别流程整体耗时: {:?}", start_total.elapsed());

    Ok(results)
}
