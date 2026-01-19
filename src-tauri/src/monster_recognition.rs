use image::{DynamicImage, GenericImageView, imageops::FilterType};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use rayon::prelude::*;
use ndarray::Array;
use ort::{session::{builder::GraphOptimizationLevel, Session}, value::Value};
use opencv::{
    core::{Mat, Vector, KeyPoint, DMatch, NORM_HAMMING},
    features2d::{ORB, BFMatcher},
    imgcodecs::{imdecode, IMREAD_GRAYSCALE},
    prelude::*,
};
use tauri::Manager;
use crate::log_to_file;
use chrono;

// YOLO 检测结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YoloDetection {
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
    pub confidence: f32,
    pub class_id: usize,
}

static YOLO_SESSION: OnceLock<Mutex<Session>> = OnceLock::new();

pub fn get_yolo_session(model_path: &PathBuf) -> Result<impl std::ops::DerefMut<Target = Session> + '_, String> {
    if let Some(mutex) = YOLO_SESSION.get() {
        return mutex.lock().map_err(|e| e.to_string());
    }
    
    let session = Session::builder()
        .map_err(|e| e.to_string())?
        .with_optimization_level(GraphOptimizationLevel::Level3)
        .map_err(|e| e.to_string())?
        .with_intra_threads(4)
        .map_err(|e| e.to_string())?
        .commit_from_file(model_path)
        .map_err(|e| e.to_string())?;
        
    let _ = YOLO_SESSION.set(Mutex::new(session));
    YOLO_SESSION.get().unwrap().lock().map_err(|e| e.to_string())
}

pub fn run_yolo_inference(img: &DynamicImage, model_path: &PathBuf) -> Result<Vec<YoloDetection>, String> {
    let mut session = get_yolo_session(model_path)?;
    let (orig_w, orig_h) = img.dimensions();

    // 1. 预处理 (640x640)
    let resized = img.resize_exact(640, 640, FilterType::Lanczos3);
    let rgb_img = resized.to_rgb8();
    
    let mut input_array = Array::zeros((1, 3, 640, 640));
    for (x, y, pixel) in rgb_img.enumerate_pixels() {
        input_array[[0, 0, y as usize, x as usize]] = pixel[0] as f32 / 255.0;
        input_array[[0, 1, y as usize, x as usize]] = pixel[1] as f32 / 255.0;
        input_array[[0, 2, y as usize, x as usize]] = pixel[2] as f32 / 255.0;
    }

    // 2. 推理
    let input_shape = [1, 3, 640, 640];
    let input_vec = input_array.into_raw_vec();
    let input_tensor = Value::from_array((input_shape, input_vec)).map_err(|e: ort::Error| e.to_string())?;
    let outputs = session.run(vec![("images", input_tensor)]).map_err(|e: ort::Error| e.to_string())?;
    let output_value = &outputs["output0"];
    
    // 3. 后处理
    let (shape, data) = output_value.try_extract_tensor::<f32>().map_err(|e: ort::Error| e.to_string())?;
    
    // YOLOv8/v11 输出通常是 [1, 4 + num_classes, 8400]
    let num_elements = shape[1] as usize;
    let num_anchors = shape[2] as usize;

    let mut candidates = Vec::new();
    let conf_threshold = 0.25;

    for i in 0..num_anchors {
        let mut max_score = 0.0;
        let mut class_id = 0;
        for c in 4..num_elements {
            // output[[0, c, i]] -> data[c * num_anchors + i]
            let score = data[c * num_anchors + i];
            if score > max_score {
                max_score = score;
                class_id = c - 4;
            }
        }

        if max_score > conf_threshold {
            let xc = data[0 * num_anchors + i];
            let yc = data[1 * num_anchors + i];
            let w = data[2 * num_anchors + i];
            let h = data[3 * num_anchors + i];

            let x1 = (xc - w / 2.0) * (orig_w as f32 / 640.0);
            let y1 = (yc - h / 2.0) * (orig_h as f32 / 640.0);
            let x2 = (xc + w / 2.0) * (orig_w as f32 / 640.0);
            let y2 = (yc + h / 2.0) * (orig_h as f32 / 640.0);

            candidates.push(YoloDetection {
                x1: x1 as i32,
                y1: y1 as i32,
                x2: x2 as i32,
                y2: y2 as i32,
                confidence: max_score,
                class_id,
            });
        }
    }

    Ok(nms(candidates, 0.45))
}

fn nms(mut detections: Vec<YoloDetection>, iou_threshold: f32) -> Vec<YoloDetection> {
    detections.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
    let mut result = Vec::new();

    while !detections.is_empty() {
        let best = detections.remove(0);
        result.push(best.clone());
        detections.retain(|d| {
            calculate_iou(&best, d) < iou_threshold
        });
    }

    result
}

fn calculate_iou(a: &YoloDetection, b: &YoloDetection) -> f32 {
    let x1 = a.x1.max(b.x1);
    let y1 = a.y1.max(b.y1);
    let x2 = a.x2.min(b.x2);
    let y2 = a.y2.min(b.y2);

    let intersection_area = (x2 - x1).max(0) * (y2 - y1).max(0);
    let area_a = (a.x2 - a.x1) * (a.y2 - a.y1);
    let area_b = (b.x2 - b.x1) * (b.y2 - b.y1);

    if area_a + area_b - intersection_area == 0 {
        return 0.0;
    }

    intersection_area as f32 / (area_a + area_b - intersection_area) as f32
}

pub fn recognize_monsters_yolo(app: &tauri::AppHandle) -> Result<Vec<String>, String> {
    use xcap::Window;
    use std::time::Instant;

    let start_total = Instant::now();
    let resources_path = app.path().resource_dir().map_err(|e| e.to_string())?;
    let model_path = resources_path.join("resources").join("models").join("best.onnx");

    // 截图逻辑
    let windows = Window::all().map_err(|e| e.to_string())?;
    let bazaar_window = windows.into_iter().find(|w| {
        let title = w.title().to_lowercase();
        let app_name = w.app_name().to_lowercase();
        let is_bazaar = title.contains("the bazaar") || app_name.contains("the bazaar") || 
                        title.contains("thebazaar") || app_name.contains("thebazaar");
        is_bazaar && !title.contains("bazaarhelper")
    });

    let screenshot = if let Some(window) = bazaar_window {
        window.capture_image().map_err(|e| e.to_string())?
    } else {
        use xcap::Monitor;
        let monitors = Monitor::all().map_err(|e| e.to_string())?;
        if monitors.is_empty() { return Err("No monitor found".into()); }
        monitors[0].capture_image().map_err(|e| e.to_string())?
    };

    let img = DynamicImage::ImageRgba8(screenshot);
    let detections = run_yolo_inference(&img, &model_path)?;
    
    let mut identified_monsters = Vec::new();
    
    // 过滤出 class_id = 1 (event) 的结果
    for det in detections.iter().filter(|d| d.class_id == 1) {
        // 进行裁剪
        let x = det.x1.max(0) as u32;
        let y = det.y1.max(0) as u32;
        let w = (det.x2 - det.x1).max(0) as u32;
        let h = (det.y2 - det.y1).max(0) as u32;
        
        if w > 0 && h > 0 {
            let cropped = img.crop_imm(x, y, w, h);
            // 调用现有的 ORB 匹配逻辑
            if let Some(monster_name) = match_single_image_to_db(&cropped, None) {
                identified_monsters.push(monster_name);
            }
        }
    }

    println!("[YOLO Recognition] Identified {} monsters in {:?}", identified_monsters.len(), start_total.elapsed());
    Ok(identified_monsters)
}

fn match_single_image_to_db(img: &DynamicImage, day_filter: Option<String>) -> Option<String> {
    let full_cache = TEMPLATE_CACHE.get()?;
    let cache: Vec<&TemplateCache> = if let Some(ref target_day) = day_filter {
        full_cache.iter().filter(|t| t.day == *target_day).collect()
    } else {
        full_cache.iter().collect()
    };

    // 预处理图像：转换为 OpenCV Mat
    let gray_img_res = (|| -> Result<Mat, Box<dyn std::error::Error>> {
        let mut buff = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut buff), image::ImageFormat::Png)?;
        let mat = imdecode(&Mat::from_slice(&buff)?, opencv::imgcodecs::IMREAD_GRAYSCALE)?;
        Ok(mat)
    })();

    let gray_img = gray_img_res.ok()?;

    // 提取特征点
    let mut orb = ORB::create(1000, 1.2f32, 8, 31, 0, 2, opencv::features2d::ORB_ScoreType::HARRIS_SCORE, 31, 20).ok()?;
    let mut keypoints = Vector::<KeyPoint>::new();
    let mut descriptors = Mat::default();
    orb.detect_and_compute(&gray_img, &Mat::default(), &mut keypoints, &mut descriptors, false).ok()?;

    if descriptors.empty() { return None; }

    // 寻找最佳匹配
    let mut best_name = None;
    let mut max_matches = 0;

    for t in cache {
        // 使用 Mat::new_rows_cols_with_data 或 Mat::from_slice 重新创建描述符
        let t_desc_res = (|| -> Result<Mat, Box<dyn std::error::Error>> {
            let mut mat = unsafe { Mat::new_rows_cols(t.descriptor_rows, t.descriptor_cols, opencv::core::CV_8U)? };
            let data_ptr = mat.data_mut();
            unsafe {
                std::ptr::copy_nonoverlapping(t.descriptors.as_ptr(), data_ptr, t.descriptors.len());
            }
            Ok(mat)
        })();

        if let Ok(t_mat) = t_desc_res {
            if let Ok(matches) = match_orb_descriptors(&descriptors, &t_mat) {
                if matches > max_matches && matches > 15 { // 设定一个阈值
                    max_matches = matches;
                    best_name = Some(t.name.clone());
                }
            }
        }
    }

    best_name
}

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
    name_zh: Option<String>,
}

static TEMPLATE_CACHE: OnceLock<Vec<TemplateCache>> = OnceLock::new();
static CARD_TEMPLATE_CACHE: OnceLock<Vec<TemplateCache>> = OnceLock::new();
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
fn extract_features_orb(image_path: &str, n_features: i32) -> Result<(Vec<(f32, f32)>, Vec<u8>, i32, i32), opencv::Error> {
    // 读取图片 (支持中文路径)
    let content = std::fs::read(image_path).map_err(|e| opencv::Error::new(opencv::core::StsError, format!("Read error: {}", e)))?;
    let img = imdecode(&Mat::from_slice(&content)?, IMREAD_GRAYSCALE)?;
    
    if img.empty() {
        return Ok((Vec::new(), Vec::new(), 0, 0));
    }

    // 初始化 ORB
    let mut orb = ORB::create(n_features, 1.2f32, 8, 31, 0, 2, 
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
pub fn extract_features_from_dynamic_image(img: &DynamicImage, n_features: i32) -> Result<Mat, opencv::Error> {
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

    // 初始化 ORB (截图也同样使用 1000 个特征点)
    let mut orb = ORB::create(n_features, 1.2f32, 8, 31, 0, 2, 
        opencv::features2d::ORB_ScoreType::HARRIS_SCORE, 31, 20)?;

    let mut keypoints = Vector::<KeyPoint>::new();
    let mut descriptors = Mat::default();
    let mask = Mat::default();

    orb.detect_and_compute(&gray_img, &mask, &mut keypoints, &mut descriptors, false)?;

    Ok(descriptors)
}

pub fn match_card_descriptors(scene_desc: &Mat) -> Result<Option<serde_json::Value>, String> {
    let cache = CARD_TEMPLATE_CACHE.get().ok_or("Card templates not loaded")?;
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
        return Ok(Some(serde_json::json!(matches_found)));
    }
    Ok(None)
}

pub fn match_monster_descriptors_from_mat(scene_descriptors: &Mat) -> Result<Option<String>, String> {
    let cache = TEMPLATE_CACHE.get().ok_or("Monster templates not loaded")?;
    let mut best_name = None;
    let mut max_matches = 0;
    let mut best_score = 0.0f32;

    for template in cache {
        if template.descriptors.is_empty() { continue; }
        use opencv::core::CV_8U;
        let rows = template.descriptor_rows;
        let cols = template.descriptor_cols;
        
        let mut template_desc = match unsafe { Mat::new_rows_cols(rows, cols, CV_8U) } {
            Ok(mat) => mat,
            Err(_) => continue,
        };
        if template.descriptors.len() == (rows * cols) as usize {
            unsafe {
                std::ptr::copy_nonoverlapping(template.descriptors.as_ptr(), template_desc.data_mut() as *mut u8, template.descriptors.len());
            }
        } else {
            continue;
        }

        if let Ok(matches) = match_orb_descriptors(&scene_descriptors, &template_desc) {
            let scene_kp_count = scene_descriptors.rows() as f32;
            let template_kp_count = template.descriptor_rows as f32;
            let min_kp = scene_kp_count.min(template_kp_count);
            let score = if min_kp > 0.0 { matches as f32 / min_kp } else { 0.0 };

            if matches > max_matches {
                max_matches = matches;
                best_score = score;
                best_name = Some(template.name.clone());
            }
        }
    }

    if max_matches >= 10 || best_score > 0.15 {
        return Ok(best_name);
    }
    Ok(None)
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
            // 适度放宽比例阈值 (从 0.75 到 0.8)，增加某些特征点不明显怪物的匹配数
            if m0.distance < 0.8 * m1.distance {
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
    let cache_file = cache_dir.join("monster_features_opencv_v2.bin");
    let bundled_cache = resources_dir.join("monster_features_opencv_v2.bin");

    // 1. 优先从资源目录加载（预打包的缓存）
    if bundled_cache.exists() {
        log_to_file(&format!("Found bundled cache file at {:?}. Using it.", bundled_cache));
        if let Ok(data) = std::fs::read(&bundled_cache) {
            if let Ok(cached_templates) = bincode::deserialize::<Vec<TemplateCache>>(&data) {
                if !cached_templates.is_empty() {
                    log_to_file(&format!("Loaded {} templates from bundled cache", cached_templates.len()));
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
    }

    // 2. 尝试从 AppData 缓存加载
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

    // 预处理：建立一个“中文名 -> 图片路径”的映射，用于补全那些没有图片的词条
    let mut name_to_path: HashMap<String, PathBuf> = HashMap::new();
    for (key, entry) in monsters.iter() {
        // 1. 检查数据库中定义的 image 字段
        if let Some(rel_path) = &entry.image {
            let p = resources_dir.join(rel_path);
            if p.exists() {
                name_to_path.insert(key.clone(), p.clone());
            } else {
                let char_path = resources_dir.join(rel_path.replace("images_monster/", "images_monster_char/"));
                if char_path.exists() {
                    name_to_path.insert(key.clone(), char_path);
                }
            }
        }
        
        // 2. 检查以 key 为名的直接图片 (e.g. 绿洲守护神_Day9.webp)
        let char_path_key = resources_dir.join(format!("images_monster_char/{}.webp", key));
        if char_path_key.exists() {
            name_to_path.insert(key.clone(), char_path_key);
        }

        // 3. 检查以 name_zh 为名的直接图片 (e.g. 绿洲守护神.webp)
        if let Some(name_zh) = entry.name_zh.as_ref() {
            let char_path_name = resources_dir.join(format!("images_monster_char/{}.webp", name_zh));
            if char_path_name.exists() {
                name_to_path.insert(key.clone(), char_path_name);
            } else {
                // 特殊处理：如果带前缀（如 "毒素 吹箭枪陷阱"），尝试查找基础名称 "吹箭枪陷阱.webp"
                if let Some(space_pos) = name_zh.rfind(' ') {
                    let base_name = &name_zh[space_pos + 1..];
                    let base_path = resources_dir.join(format!("images_monster_char/{}.webp", base_name));
                    if base_path.exists() {
                        name_to_path.insert(key.clone(), base_path);
                    }
                }
            }
        }
    }

    let mut image_tasks = Vec::new();
    let mut seen_names = HashSet::new();

    for (key, entry) in monsters.iter() {
        if let Some(day) = &entry.available {
            let mut found_path = name_to_path.get(key).cloned();
            
            if found_path.is_none() {
                let clean_key = if key.contains("_Day") {
                    key.split("_Day").next().unwrap_or(key).to_string()
                } else {
                    key.clone()
                };
                found_path = name_to_path.get(&clean_key).cloned();
                if found_path.is_none() {
                    for (mapped_name, path) in name_to_path.iter() {
                        if mapped_name.starts_with(&clean_key) {
                            found_path = Some(path.clone());
                            break;
                        }
                    }
                }
            }

            if let Some(path) = found_path {
                let mut clean_name = if key.contains("_Day") {
                    key.split("_Day").next().unwrap_or(key).to_string()
                } else {
                    key.clone()
                };

                // 特殊处理陷阱类：将所有陷阱变体统一为基础名称（如 "毒素 吹箭枪陷阱" -> "吹箭枪陷阱"）
                // 这样它们会共享同一个 ORB 模板，避免前缀不同导致无法识别
                if clean_name.contains("陷阱") {
                    if let Some(space_pos) = clean_name.rfind(' ') {
                        clean_name = clean_name[space_pos + 1..].to_string();
                    }
                }

                // 去重：如果已经添加过同名怪物的特征提取任务，且路径相同，则跳过
                if seen_names.contains(&clean_name) {
                    continue;
                }

                if let Ok(metadata) = std::fs::metadata(&path) {
                    if metadata.len() > 0 {
                        seen_names.insert(clean_name.clone());
                        image_tasks.push((clean_name, day.clone(), path));
                    }
                }
            } else {
                 if day == "Day 10+" {
                     log_to_file(&format!("Missing Day 10+ monster image: {}", key));
                 }
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
        match extract_features_orb(path_str, 1000) {
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
    use xcap::Monitor;
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
    use windows::Win32::Foundation::POINT;
    
    // 1. 获取鼠标位置
    let mut point = POINT::default();
    unsafe { GetCursorPos(&mut point).map_err(|e| e.to_string())? };
    let mouse_x = point.x;
    let mouse_y = point.y;

    // 2. 查找窗口并截图
    let windows = xcap::Window::all().map_err(|e| e.to_string())?;
    // 优先查找包含鼠标且标题匹配 "The Bazaar" 的窗口
    let bazaar_window = windows.into_iter().find(|w| {
        let title = w.title().to_lowercase();
        let app_name = w.app_name().to_lowercase();
        let is_bazaar = title.contains("the bazaar") || app_name.contains("the bazaar") || 
                        title.contains("thebazaar") || app_name.contains("thebazaar");
        
        if is_bazaar {
            let wx = w.x();
            let wy = w.y();
            let ww = w.width();
            let wh = w.height();
            // 检查鼠标是否在窗口范围内
            mouse_x >= wx && mouse_x < wx + ww as i32 &&
            mouse_y >= wy && mouse_y < wy + wh as i32
        } else {
            false
        }
    });

    let (screenshot, win_x, win_y) = if let Some(window) = bazaar_window {
        log_to_file(&format!("Found matching window under mouse: {}, App: {}", window.title(), window.app_name()));
        (window.capture_image().map_err(|e| e.to_string())?, window.x(), window.y())
    } else {
        log_to_file("No matching Bazaar window under mouse, capturing monitor under cursor.");
        // Find monitor containing the mouse
        let monitors = Monitor::all().map_err(|e| e.to_string())?;
        if monitors.is_empty() { return Err("No monitor found".into()); }
        
        let target_monitor = monitors.into_iter().find(|m| {
             let mx = m.x();
             let my = m.y();
             let mw = m.width();
             let mh = m.height();
             mouse_x >= mx && mouse_x < mx + mw as i32 &&
             mouse_y >= my && mouse_y < my + mh as i32
        }).ok_or("Mouse is not within any monitor bounds")?;

        (target_monitor.capture_image().map_err(|e| e.to_string())?, target_monitor.x(), target_monitor.y())
    };

    let img = DynamicImage::ImageRgba8(screenshot);
    let (img_w, img_h) = img.dimensions();

    // 3. 计算裁剪区域 400x400
    // 鼠标在截图内的相对坐标
    let rel_x = mouse_x - win_x;
    let rel_y = mouse_y - win_y;
    
    // 定义裁剪框 (以鼠标为中心)
    let crop_size = 400;
    let half_size = crop_size / 2;
    
    // 确保不越界
    // 使用 saturating_sub 防止 usize/u32 减法溢出 (panic at img_w - crop_x)
    let crop_x = (rel_x - half_size).max(0) as u32;
    let crop_y = (rel_y - half_size).max(0) as u32;
    
    // 实际裁剪宽度（处理边缘情况）
    let crop_w = if crop_x + crop_size as u32 > img_w { img_w.saturating_sub(crop_x) } else { crop_size as u32 };
    let crop_h = if crop_y + crop_size as u32 > img_h { img_h.saturating_sub(crop_y) } else { crop_size as u32 };

    if crop_w < 50 || crop_h < 50 {
        log_to_file(&format!("Error: Crop area too small ({}x{}). Mouse: ({},{}), Win: ({},{}), Rel: ({},{}), Img: {}x{}", 
            crop_w, crop_h, mouse_x, mouse_y, win_x, win_y, rel_x, rel_y, img_w, img_h));
        return Err("裁剪区域太小或鼠标已移出窗口范围".into());
    }

    let cropped_img = img.crop_imm(crop_x, crop_y, crop_w, crop_h);
    // 可选：保存调试图片
    // cropped_img.save("debug_mouse_crop.png").ok();

    // 4. 提取特征并匹配
    let scene_desc = extract_features_from_dynamic_image(&cropped_img, 1000).map_err(|e| e.to_string())?;
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
        let mut template_desc = match unsafe { Mat::new_rows_cols(template.descriptor_rows, template.descriptor_cols, CV_8U) } {
            Ok(m) => m,
            Err(e) => {
                log_to_file(&format!("OpenCV Error creating Mat for template {}: {}", template.name, e));
                continue;
            }
        };
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
        println!("鼠标指向识别成功: {} (匹配: {}, 2nd: {})", top1.0, top1.1, top2_score);
        
        // 关键改进：处理“陷阱”类多重匹配
        // 如果识别结果包含“陷阱”，则寻找所有同类型的陷阱变体并一起作为结果返回
        let base_name = if top1.0.contains("_Day") {
            top1.0.split("_Day").next().unwrap_or(&top1.0).to_string()
        } else {
            top1.0.clone()
        };

        if base_name.contains("陷阱") {
            if base_name.contains("吹箭枪陷阱") {
                return Ok(Some("毒素 吹箭枪陷阱|黑曜石 吹箭枪陷阱|炽焰 吹箭枪陷阱".to_string()));
            } else if base_name.contains("铁蒺藜陷阱") {
                return Ok(Some("炽焰 铁蒺藜陷阱|黑曜石 铁蒺藜陷阱|毒素 铁蒺藜陷阱".to_string()));
            } else if base_name.contains("滚石陷阱") {
                return Ok(Some("毒素 滚石陷阱|黑曜石 滚石陷阱|炽焰 滚石陷阱".to_string()));
            }
        }

        return Ok(Some(base_name));
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
    let region_y = (height as f32 * 0.10) as u32;
    let region_h = (height as f32 * 0.50) as u32;
    let total_region_w = (width as f32 * 0.60) as u32;
    let region_x_start = (width as f32 * 0.20) as u32;

    let slot_w = total_region_w / 3;
    let slot_h = region_h;

    let start_match = Instant::now();
    save_debug_image(&img, "monster_full_screenshot");

    for i in 0..3 {
        let start_slot = Instant::now();
        let x = region_x_start + (i as u32 * slot_w);
        let y = region_y;
        if x + slot_w > width || y + slot_h > height { continue; }

        let slice = img.crop_imm(x, y, slot_w, slot_h);
        save_debug_image(&slice, &format!("monster_slot_{}", i + 1));
        
        // 使用 OpenCV 提取场景特征
        let scene_descriptors = match extract_features_from_dynamic_image(&slice, 1000) {
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

// --- Card Recognition ---

pub fn save_debug_image(img: &DynamicImage, name: &str) {
    // 自动保存到缓存目录下的 debug 文件夹
    let cache_dir = std::env::var("APPDATA")
        .map(|v| PathBuf::from(v).join("BazaarHelper"))
        .unwrap_or_else(|_| PathBuf::from("target/debug"));
        
    let debug_dir = cache_dir.join("debug_images");
    let _ = std::fs::create_dir_all(&debug_dir);
    
    let file_path = debug_dir.join(format!("{}_{}.png", chrono::Local::now().format("%H%M%S"), name));
    let _ = img.save(&file_path);
    println!("[DebugImage] 已保存截图至: {:?}", file_path);
}

pub async fn preload_card_templates_async(resources_dir: PathBuf, cache_dir: PathBuf) -> Result<(), String> {
    log_to_file(&format!("Start loading card templates. Resource Dir: {:?}, Cache Dir: {:?}", resources_dir, cache_dir));
    
    let cache_file = cache_dir.join("card_features_opencv.bin");
    let bundled_cache = resources_dir.join("card_features_opencv.bin");

    // 1. 优先从资源目录加载
    if bundled_cache.exists() {
        if let Ok(data) = std::fs::read(&bundled_cache) {
            if let Ok(cached_templates) = bincode::deserialize::<Vec<TemplateCache>>(&data) {
                if !cached_templates.is_empty() {
                    log_to_file(&format!("Loaded {} card templates from bundled cache", cached_templates.len()));
                    println!("[Card Templates] Loaded {} templates from bundled cache: {:?}", cached_templates.len(), bundled_cache);
                    let _ = CARD_TEMPLATE_CACHE.set(cached_templates);
                    return Ok(());
                }
            }
        }
    }

    // 2. 尝试从 AppData 缓存加载
    if cache_file.exists() {
        if let Ok(data) = std::fs::read(&cache_file) {
            if let Ok(cached_templates) = bincode::deserialize::<Vec<TemplateCache>>(&data) {
                if !cached_templates.is_empty() {
                    log_to_file(&format!("Loaded {} card templates from OpenCV cache", cached_templates.len()));
                    println!("[Card Templates] Loaded {} templates from cache: {:?}", cached_templates.len(), cache_file);
                    let _ = CARD_TEMPLATE_CACHE.set(cached_templates);
                    return Ok(());
                }
            }
        }
    }

    // 3. 从 items_db.json 加载并计算
    let db_path = resources_dir.join("items_db.json");
    if !db_path.exists() {
        return Err(format!("items_db.json not found at {:?}", db_path));
    }

    let json_content = std::fs::read_to_string(&db_path)
        .map_err(|e| format!("读取 items_db.json 失败: {}", e))?;

    // 我们只需要简单的结构
    #[derive(Deserialize)]
    struct RawItemSimple {
        id: String,
        name_cn: Option<String>,
    }
    
    let items: Vec<RawItemSimple> = serde_json::from_str(&json_content)
        .map_err(|e| format!("解析 items_db.json 失败: {}", e))?;

    let mut tasks = Vec::new();
    for item in items {
        let img_path = resources_dir.join("images").join(format!("{}.webp", item.id));
        if img_path.exists() {
            tasks.push((item.name_cn.unwrap_or_else(|| item.id.clone()), item.id, img_path));
        }
    }

    log_to_file(&format!("Building card cache for {} images...", tasks.len()));
    
    let cache: Vec<TemplateCache> = tasks.into_par_iter().filter_map(|(name, id, path)| {
        let path_str = path.to_str()?;
        // 用户要求特征点少一些, 用 300
        match extract_features_orb(path_str, 300) {
            Ok((keypoints, descriptors, rows, cols)) => {
                Some(TemplateCache {
                    name, // 这里存中文名
                    day: id, // 这里借用 day 字段存 ID
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
    }).collect();

    log_to_file(&format!("Successfully built cache for {} cards", cache.len()));
    
    // 保存到文件以便下次加速
    if let Ok(serialized) = bincode::serialize(&cache) {
        let _ = std::fs::write(&cache_file, &serialized);
        let _ = std::fs::write(&bundled_cache, &serialized);
        log_to_file(&format!("Saved card templates cache: appdata={:?}, resources={:?}", cache_file, bundled_cache));
        println!("[Card Templates] Cache saved: appdata={:?}, resources={:?}", cache_file, bundled_cache);
    }

    let _ = CARD_TEMPLATE_CACHE.set(cache);
    Ok(())
}

#[tauri::command]
pub async fn recognize_card_at_mouse() -> Result<Option<serde_json::Value>, String> {
    use xcap::{Window, Monitor};
    use enigo::{Enigo, Mouse, Settings};

    // 1. 获取鼠标位置
    let enigo = match Enigo::new(&Settings::default()) {
        Ok(e) => e,
        Err(e) => return Err(format!("Failed to init Enigo: {:?}", e)),
    };
    let (mouse_x, mouse_y) = match enigo.location() {
        Ok(loc) => loc,
        Err(e) => return Err(format!("Failed to get mouse location: {:?}", e)),
    };

    // 2. 截图
    let windows = Window::all().map_err(|e| e.to_string())?;
    let bazaar_window = windows.into_iter().find(|w| {
        let title = w.title().to_lowercase();
        let app_name = w.app_name().to_lowercase();
        title.contains("the bazaar") || app_name.contains("the bazaar")
    });

    let (screenshot, win_x, win_y) = if let Some(window) = bazaar_window {
        (window.capture_image().map_err(|e| e.to_string())?, window.x(), window.y())
    } else {
        let monitors = Monitor::all().map_err(|e| e.to_string())?;
        let target_monitor = monitors.into_iter().find(|m| {
             let mx = m.x(); let my = m.y(); let mw = m.width(); let mh = m.height();
             mouse_x >= mx && mouse_x < mx + mw as i32 && mouse_y >= my && mouse_y < my + mh as i32
        }).ok_or("Mouse not in monitor")?;
        (target_monitor.capture_image().map_err(|e| e.to_string())?, target_monitor.x(), target_monitor.y())
    };

    let img = DynamicImage::ImageRgba8(screenshot);
    let (img_w, img_h) = img.dimensions();
    let rel_x = mouse_x - win_x;
    let rel_y = mouse_y - win_y;
    
    // 4K 自适应：调整截图范围。
    // 竖直方向保持屏幕高度的 75%，水平方向缩小一半，设为屏幕高度的 50%
    let target_h = (img_h as f32 * 0.75).round() as u32;
    let target_w = (img_h as f32 * 0.50).round() as u32;
    
    let half_w = (target_w / 2) as i32;
    let half_h = (target_h / 2) as i32;
    
    let crop_x = std::cmp::max(rel_x - half_w, 0) as u32;
    let crop_y = std::cmp::max(rel_y - half_h, 0) as u32;
    let crop_w = if crop_x + target_w > img_w { img_w.saturating_sub(crop_x) } else { target_w };
    let crop_h = if crop_y + target_h > img_h { img_h.saturating_sub(crop_y) } else { target_h };

    if crop_w < 50 || crop_h < 50 { return Err("Invalid crop size".into()); }
    let mut cropped_img = img.crop_imm(crop_x, crop_y, crop_w, crop_h);
    
    // 4K 优化：针对高分辨率截图，缩减尺寸以加快特征提取和比对（由 512 提升至 800 以保留更多细节）
    if crop_w > 800 || crop_h > 800 {
        cropped_img = cropped_img.resize(800, 800, image::imageops::FilterType::Triangle);
    }
    
    save_debug_image(&cropped_img, "card_crop_adaptive");

    // 3. 提取特征
    let scene_desc = extract_features_from_dynamic_image(&cropped_img, 500).map_err(|e| e.to_string())?;
    if scene_desc.empty() { return Ok(None); }
    
    // 4. 比对
    let cache = CARD_TEMPLATE_CACHE.get().ok_or("Card templates not loaded")?;
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
    for i in 0..results.len().min(10) { // 先取前10个候选
        let (top, matches, confidence) = results[i];
        // 阈值：匹配点数 > 12 且 置信度 > 0.12
        if matches > 12 && confidence > 0.12 {
             matches_found.push(serde_json::json!({
                 "id": top.day, // ID 存储在 day 字段
                 "name": top.name,
                 "confidence": confidence,
                 "match_count": matches
             }));
        }
        if matches_found.len() >= 3 { break; }
    }

    if !matches_found.is_empty() {
        println!("[Card Recognition] Found {} matches", matches_found.len());
        return Ok(Some(serde_json::json!(matches_found)));
    }
    
    println!("[Card Recognition] No matches found above threshold.");
    Ok(None)
}
