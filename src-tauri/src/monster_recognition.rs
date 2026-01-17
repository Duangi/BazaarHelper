use image::{DynamicImage, GenericImageView};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use rayon::prelude::*;
use opencv::{
    core::{Mat, Vector, KeyPoint, DMatch, NORM_HAMMING},
    features2d::{ORB, BFMatcher},
    imgcodecs::{imdecode, IMREAD_GRAYSCALE},
    prelude::*,
};
use crate::log_to_file;

#[tauri::command]
pub fn check_opencv_load() -> Result<String, String> {
    let mat = Mat::default();
    if mat.empty() {
        Ok("OpenCV loaded successfully (Mat created)".to_string())
    } else {
        Ok("OpenCV loaded (Mat not empty?)".to_string())
    }
}

// 识别结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonsterRecognitionResult {
    pub position: u8,
    pub name: String,
    pub confidence: f32,
    pub match_count: usize, // 新增匹配点数字段
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
    // 读取图片 (支持中文路径)
    let content = std::fs::read(image_path).map_err(|e| opencv::Error::new(opencv::core::StsError, format!("Read error: {}", e)))?;
    let img = imdecode(&Mat::from_slice(&content)?, IMREAD_GRAYSCALE)?;
    
    if img.empty() {
        return Ok((Vec::new(), Vec::new(), 0, 0));
    }

    // 初始化 ORB (nfeatures=1000)
    let mut orb = ORB::create(1000, 1.2f32, 8, 31, 0, 2, 
        opencv::features2d::ORB_ScoreType::HARRIS_SCORE, 31, 20)?;

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
        .map(|kp| (kp.pt().x, kp.pt().y))
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
    use image::ImageFormat;
    img.write_to(&mut std::io::Cursor::new(&mut bytes), ImageFormat::Png)
        .map_err(|e| opencv::Error::new(opencv::core::StsError, format!("图像转换失败: {}", e)))?;
    
    // 使用 OpenCV 解码
    use opencv::imgcodecs::imdecode;
    use opencv::core::_InputArray;
    let buf_mat = Mat::from_slice(&bytes)?;
    let input_array = _InputArray::from_mat(&buf_mat)?;
    let gray_img = imdecode(&input_array, IMREAD_GRAYSCALE)?;
    
    if gray_img.empty() {
        return Ok(Mat::default());
    }

    // 初始化 ORB (nfeatures=1000)
    let mut orb = ORB::create(1000, 1.2f32, 8, 31, 0, 2, 
        opencv::features2d::ORB_ScoreType::HARRIS_SCORE, 31, 20)?;

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
    // 使用 knn_train_match: query, train, output, k, mask, compactResult
    matcher.knn_train_match(desc1, desc2, &mut matches, 2, &Mat::default(), false)?;

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
    log_to_file(&format!("Start loading templates. Resource Dir: {:?}, Cache Dir: {:?}", resources_dir, cache_dir));
    let progress = Arc::new(Mutex::new(LoadingProgress {
        loaded: 0,
        total: 0,
        is_complete: false,
        current_name: "".to_string(),
    }));
    let _ = LOADING_PROGRESS.set(progress.clone());
    
    // Define both paths
    let cache_file = cache_dir.join("monster_features_opencv.bin");
    let bundled_cache = resources_dir.join("monster_features_opencv.bin");

    // 1. 优先从资源目录加载（预打包的缓存），并强制覆盖本地缓存
    if bundled_cache.exists() {
        log_to_file(&format!("Found bundled cache file at {:?}. Forcing overwrite of local cache.", bundled_cache));
        if let Ok(data) = std::fs::read(&bundled_cache) {
            // Force replace logic
            let _ = std::fs::create_dir_all(&cache_dir);
            if let Err(e) = std::fs::write(&cache_file, &data) {
                 log_to_file(&format!("Failed to write to local cache: {}", e));
            } else {
                 log_to_file("Local cache overwritten successfully.");
            }

            // Load directly from the bundled data
             if let Ok(cached_templates) = bincode::deserialize::<Vec<TemplateCache>>(&data) {
                 log_to_file(&format!("Loaded {} templates from BUNDLED cache", cached_templates.len()));
                 println!("从预打包缓存加载了 {} 个怪物特征点模板", cached_templates.len());

                 if let Ok(mut p) = progress.lock() {
                        p.loaded = cached_templates.len();
                        p.total = cached_templates.len();
                        p.is_complete = true;
                }
                let _ = TEMPLATE_CACHE.set(cached_templates);
                return Ok(());
             } else {
                 log_to_file("Failed to deserialize bundled cache (corruption?)");
             }
        }
    }

    // 2. 尝试从 AppData 缓存加载 (旧逻辑)
    // let cache_file = cache_dir.join("monster_features_opencv.bin");
    if cache_file.exists() {
        log_to_file(&format!("Found cache file at {:?}", cache_file));
        if let Ok(data) = std::fs::read(&cache_file) {
            if let Ok(cached_templates) = bincode::deserialize::<Vec<TemplateCache>>(&data) {
                if !cached_templates.is_empty() {
                    log_to_file(&format!("Loaded {} templates from OpenCV cache", cached_templates.len()));
                    println!("从 OpenCV 缓存加载了 {} 个怪物特征点模板", cached_templates.len());
                    if let Ok(mut p) = progress.lock() {
                        p.loaded = cached_templates.len();
                        p.total = cached_templates.len();
                        p.is_complete = true;
                    }
                    let _ = TEMPLATE_CACHE.set(cached_templates);
                    return Ok(());
                } else {
                    log_to_file("Cache file is empty (0 templates). Rebuilding from images...");
                }
            } else {
                log_to_file("Failed to deserialize cache file.");
            }
        } else {
             log_to_file("Failed to read cache file.");
        }
    } else {
        log_to_file("Cache file not found, rebuilding from images.");
    }

    // 2. 从原始图片加载 (使用 Rayon 并行)
    let db_path = resources_dir.join("monsters_db.json");
    if !db_path.exists() {
        log_to_file(&format!("Error: monsters_db.json not found at {:?}", db_path));
    }

    let json_content = std::fs::read_to_string(&db_path)
        .map_err(|e| format!("读取 monsters_db.json 失败: {}", e))?;

    let monsters: HashMap<String, MonsterEntry> = serde_json::from_str(&json_content)
        .map_err(|e| format!("解析 monsters_db.json 失败: {}", e))?;

    let mut image_tasks = Vec::new();
    for (key, entry) in monsters.iter() {
        if let (Some(rel_path), Some(day)) = (&entry.image, &entry.available) {
            // 使用角色图进行识别（images_monster_char）
            let char_path = rel_path.replace("images_monster/", "images_monster_char/");
            let full_path = resources_dir.join(&char_path);
            if full_path.exists() {
                image_tasks.push((key.clone(), day.clone(), full_path));
            } else {
                log_to_file(&format!("Missing image: {:?}", full_path));
            }
        }
    }

    let total = image_tasks.len();
    log_to_file(&format!("Found {} images to process.", total));
    
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
                log_to_file(&format!("Warning: extraction failed for {}: {}", name, e));
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
        log_to_file("Global cache saved.");
    }
    
    // 如果没有预打包缓存，提示用户可以复制生成的缓存
    if !bundled_cache.exists() {
        log_to_file(&format!("Suggestion: You can copy {:?} to {:?} to ship with the app.", cache_file, bundled_cache));
    }
    
    log_to_file(&format!("Template loading complete. Cache size: {}", cache.len()));

    if let Ok(mut p) = progress.lock() { p.is_complete = true; }
    let _ = TEMPLATE_CACHE.set(cache);
    println!("OpenCV ORB 特征点模板加载完成");
    Ok(())
}


// 公共函数：鼠标触发的怪物识别
pub fn scan_and_identify_monster_at_mouse() -> Result<Option<String>, String> {
    use xcap::{Window, Monitor};
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
    use windows::Win32::Foundation::POINT;
    
    // 1. 获取鼠标位置
    let mut point = POINT::default();
    unsafe { GetCursorPos(&mut point).map_err(|e| e.to_string())? };
    let mouse_x = point.x;
    let mouse_y = point.y;

    // 2. 查找窗口并截图
    let windows = Window::all().map_err(|e| e.to_string())?;
    // 优先只查找 "The Bazaar" 窗口
    let bazaar_window = windows.into_iter().find(|w| {
        let title = w.title().to_lowercase();
        let app_name = w.app_name().to_lowercase();
        // 简单匹配
        title.contains("the bazaar") || app_name.contains("the bazaar") || 
        title.contains("thebazaar") || app_name.contains("thebazaar")
    });

    let (screenshot, win_x, win_y) = if let Some(window) = bazaar_window {
        log_to_file(&format!("Found window: {}, App: {}", window.title(), window.app_name()));
        (window.capture_image().map_err(|e| e.to_string())?, window.x(), window.y())
    } else {
        log_to_file("Window not found, capturing primary monitor.");
        // 如果找不到窗口，就截全屏（主显示器）
        let monitors = Monitor::all().map_err(|e| e.to_string())?;
        if monitors.is_empty() { return Err("No monitor found".into()); }
        (monitors[0].capture_image().map_err(|e| e.to_string())?, monitors[0].x(), monitors[0].y())
    };

    let mut img = DynamicImage::ImageRgba8(screenshot);
    let (img_w, img_h) = img.dimensions();

    // 3. 计算裁剪区域 400x400
    // 鼠标在截图内的相对坐标
    let rel_x = mouse_x - win_x;
    let rel_y = mouse_y - win_y;
    
    // 定义裁剪框 (以鼠标为中心)
    let crop_size = 400;
    let half_size = crop_size / 2;
    
    // 确保不越界
    let crop_x = (rel_x - half_size).max(0) as u32;
    let crop_y = (rel_y - half_size).max(0) as u32;
    
    // 实际裁剪宽度（处理边缘情况）
    let crop_w = if crop_x + crop_size as u32 > img_w { img_w - crop_x } else { crop_size as u32 };
    let crop_h = if crop_y + crop_size as u32 > img_h { img_h - crop_y } else { crop_size as u32 };

    if crop_w < 50 || crop_h < 50 {
        return Err("裁剪区域太小".into());
    }

    let cropped_img = img.crop(crop_x, crop_y, crop_w, crop_h);
    // 可选：保存调试图片
    // cropped_img.save("debug_mouse_crop.png").ok();

    // 4. 提取特征并匹配
    let scene_desc = extract_features_from_dynamic_image(&cropped_img).map_err(|e| e.to_string())?;
    if scene_desc.empty() {
        return Ok(None);
    }
    
    // 5. 对比所有模板
    let cache = TEMPLATE_CACHE.get().ok_or("Templates not loaded")?;
    log_to_file(&format!("Scanning against {} templates", cache.len()));
    let mut results: Vec<(String, usize, f32)> = Vec::new(); // (Name, Matches, Confidence)

    for template in cache {
        if template.descriptors.is_empty() { continue; }

        use opencv::core::CV_8U;
        // 重建模板描述符
        let mut template_desc = unsafe { Mat::new_rows_cols(template.descriptor_rows, template.descriptor_cols, CV_8U).unwrap() };
        if template.descriptors.len() == (template.descriptor_rows * template.descriptor_cols) as usize {
            unsafe {
                std::ptr::copy_nonoverlapping(template.descriptors.as_ptr(), template_desc.data_mut() as *mut u8, template.descriptors.len());
            }
        } else {
            continue;
        }

        if let Ok(matches) = match_orb_descriptors(&scene_desc, &template_desc) {
            let temp_kp_count = template.descriptor_rows as f32;
            let scene_kp_count = scene_desc.rows() as f32;
            
            // 计算置信度
            let min_kp = temp_kp_count.min(scene_kp_count);
            let confidence = if min_kp > 0.0 {
                 matches as f32 / min_kp * 100.0
            } else { 0.0 };
            
            results.push((template.name.clone(), matches, confidence));
        }
    }
    
    // 6. 排序和阈值判断
    results.sort_by(|a, b| b.1.cmp(&a.1)); // 按匹配数降序

    if results.is_empty() { return Ok(None); }

    let top1 = &results[0];
    let top2_score = if results.len() > 1 { results[1].1 as f32 } else { 0.0 };
    
    // 阈值检查: 匹配数 > 25 且 Top1 > 1.5 * Top2
    if top1.1 > 25 && (top1.1 as f32 > 1.5 * top2_score) {
        // 7. 黑名单检查（重复出现的怪物）
        let ignored_monsters = vec!["快乐杰克南瓜", "绿洲守护神"];
        if ignored_monsters.contains(&top1.0.as_str()) {
            println!("识别到重复出现怪物 '{}'，跳过自动跳转。", top1.0);
            return Ok(None);
        }
        
        println!("鼠标指向识别成功: {} (匹配: {}, 2nd: {})", top1.0, top1.1, top2_score);
        return Ok(Some(top1.0.clone()));
    }

    Ok(None)
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
        println!("[OpenCV Recognition] 'The Bazaar' window not found, falling back to monitor 0");
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
            // 创建临时 Vec 并 clone 到 Mat
            use opencv::core::CV_8U;
            let rows = template.descriptor_rows;
            let cols = template.descriptor_cols;
            
            // 创建空 Mat 并拷贝数据
            let mut template_desc = match unsafe { Mat::new_rows_cols(rows, cols, CV_8U) } {
                Ok(mat) => mat,
                Err(e) => {
                    println!("创建 Mat 失败: {}", e);
                    continue;
                }
            };
            if template.descriptors.len() == (rows * cols) as usize {
                unsafe {
                    let src_ptr = template.descriptors.as_ptr();
                    let dst_ptr = template_desc.data_mut() as *mut u8;
                    std::ptr::copy_nonoverlapping(src_ptr, dst_ptr, template.descriptors.len());
                }
            } else {
                continue;
            }

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
                match_count: max_matches,
            });
        }
    }
    
    println!("[Timer] OpenCV 特征提取与比对总耗时: {:?}", start_match.elapsed());
    println!("[Timer] OpenCV 识别流程整体耗时: {:?}", start_total.elapsed());

    Ok(results)
}
