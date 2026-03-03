use image::{DynamicImage, GenericImageView, imageops::FilterType};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
#[cfg(target_os = "windows")]
use std::sync::atomic::{AtomicBool, Ordering};
use rayon::prelude::*;
use ndarray::Array;
use ort::{
    session::{builder::GraphOptimizationLevel, Session},
    value::Value
};
#[cfg(target_os = "windows")]
use ort::execution_providers::DirectMLExecutionProvider;
use opencv::{
    core::{Mat, Vector, KeyPoint, DMatch, NORM_HAMMING},
    features2d::{ORB, BFMatcher},
    imgcodecs::{imdecode, IMREAD_GRAYSCALE},
    prelude::*,
};
use tauri::Manager;
use crate::data_management::resource_paths::resolve_existing_resource;
use crate::log_to_file;
use chrono;
use device_query::{DeviceQuery, DeviceState};

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

fn crop_focus(img: &DynamicImage, top_fraction: f32, center_fraction: f32, h_offset: f32, pad_px: i32, keep: &str) -> DynamicImage {
    let (w, h) = img.dimensions();
    
    // compute top crop height
    let top_h = (h as f32 * top_fraction).max(1.0) as u32;
    
    // compute centered width
    let center_w = (w as f32 * center_fraction).max(1.0) as u32;
    
    // center x with offset
    let center_x = (w as f32 / 2.0 + h_offset * w as f32) as i32;
    
    let mut left = center_x - center_w as i32 / 2;
    let mut right = left + center_w as i32;
    
    // clamp
    if left < 0 {
        left = 0;
        right = center_w as i32;
    }
    if right > w as i32 {
        right = w as i32;
        left = (w as i32 - center_w as i32).max(0);
    }
    
    // apply padding
    left = (left - pad_px).max(0);
    right = (right + pad_px).min(w as i32);
    
    // final crop: top/bottom portion then horizontal slice
    let (crop_y, crop_h) = if keep == "bottom" {
        let y = (h as i32 - top_h as i32).max(0) as u32;
        (y, top_h.min(h - y))
    } else {
        (0, top_h.min(h))
    };
    
    let crop_x = left as u32;
    let crop_w = (right as u32 - left as u32).min(w - crop_x);
    
    img.crop_imm(crop_x, crop_y, crop_w, crop_h)
}

static YOLO_SESSION_CPU: OnceLock<Mutex<Session>> = OnceLock::new();
#[cfg(target_os = "windows")]
static YOLO_SESSION_DML: OnceLock<Mutex<Session>> = OnceLock::new();
#[cfg(target_os = "windows")]
static YOLO_DML_DISABLED: AtomicBool = AtomicBool::new(false);

fn build_yolo_session(
    model_path: &PathBuf,
    #[allow(unused_variables)] use_gpu: bool,
) -> Result<Session, String> {
    if use_gpu {
        log_to_file("[YOLO] Initializing session with DirectML execution provider...");
    } else {
        log_to_file("[YOLO] Initializing session with CPU execution provider...");
    }

    let builder = Session::builder()
        .map_err(|e| format!("创建Session Builder失败: {}", e))?;

    #[cfg(target_os = "windows")]
    let builder = if use_gpu {
        builder
            .with_execution_providers([DirectMLExecutionProvider::default().build()])
            .map_err(|e| format!("DirectML执行提供者加载失败: {}. 请确保已安装GPU驱动和DirectML.dll在程序目录中。", e))?
    } else {
        builder
    };

    #[cfg(not(target_os = "windows"))]
    let _ = use_gpu;

    let session = builder
        .with_optimization_level(GraphOptimizationLevel::Level3)
        .map_err(|e| format!("设置优化级别失败: {}", e))?
        .with_intra_threads(4)
        .map_err(|e| format!("设置线程数失败: {}", e))?
        .commit_from_file(model_path)
        .map_err(|e| format!("加载ONNX模型失败: {}. 模型路径: {:?}", e, model_path))?;

    if use_gpu {
        log_to_file("[YOLO] Session initialized successfully with DirectML");
    } else {
        log_to_file("[YOLO] Session initialized successfully with CPU");
    }
    Ok(session)
}

pub fn get_yolo_session(
    model_path: &PathBuf,
    #[allow(unused_variables)] use_gpu: bool,
) -> Result<MutexGuard<'static, Session>, String> {
    #[cfg(target_os = "windows")]
    {
        if use_gpu && !YOLO_DML_DISABLED.load(Ordering::Relaxed) {
            if YOLO_SESSION_DML.get().is_none() {
                match build_yolo_session(model_path, true) {
                    Ok(session) => {
                        let _ = YOLO_SESSION_DML.set(Mutex::new(session));
                    }
                    Err(e) => {
                        YOLO_DML_DISABLED.store(true, Ordering::Relaxed);
                        log_to_file(&format!(
                            "[YOLO] DirectML init failed once, disabling GPU session retry and using CPU afterwards: {}",
                            e
                        ));
                        return Err(e);
                    }
                }
            }
            let mutex = YOLO_SESSION_DML
                .get()
                .ok_or_else(|| "failed to initialize DirectML YOLO session".to_string())?;
            return mutex.lock().map_err(|e| e.to_string());
        }
    }

    if YOLO_SESSION_CPU.get().is_none() {
        let session = build_yolo_session(model_path, false)?;
        let _ = YOLO_SESSION_CPU.set(Mutex::new(session));
    }
    let mutex = YOLO_SESSION_CPU
        .get()
        .ok_or_else(|| "failed to initialize CPU YOLO session".to_string())?;
    mutex.lock().map_err(|e| e.to_string())
}

pub fn run_yolo_inference(img: &DynamicImage, model_path: &PathBuf, use_gpu: bool) -> Result<Vec<YoloDetection>, String> {
    let mut session = get_yolo_session(model_path, use_gpu)?;
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
    let detections = run_yolo_inference(&img, &model_path, true)?; // 默认使用GPU
    
    let mut identified_monsters = Vec::new();

    // 1. 分离 Detected Objects
    // names: ['day', 'event', 'item', 'monstericon', 'randomicon', 'shopicon', 'skill']
    // event_id = 1, monstericon_id = 3
    let events: Vec<&YoloDetection> = detections.iter().filter(|d| d.class_id == 1).collect();
    let monster_icons: Vec<&YoloDetection> = detections.iter().filter(|d| d.class_id == 3).collect();

    // 2. 判定逻辑: Event + MonsterIcon Overlap > 50%
    for event in events {
        let mut is_monster_event = false;
        
        for icon in &monster_icons {
            let overlay_area = intersection_area_val(event, icon);
            let icon_area = ((icon.x2 - icon.x1) * (icon.y2 - icon.y1)) as f32;
            
            if icon_area > 0.0 && (overlay_area / icon_area) >= 0.5 {
                 is_monster_event = true;
                 break;
            }
        }

        if is_monster_event {
            // 进行裁剪和识别
            let x = event.x1.max(0) as u32;
            let y = event.y1.max(0) as u32;
            let w = (event.x2 - event.x1).max(0) as u32;
            let h = (event.y2 - event.y1).max(0) as u32;
            
            if w > 0 && h > 0 {
                let cropped = img.crop_imm(x, y, w, h);
                // 调用现有的 ORB 匹配逻辑
                if let Some(monster_name) = match_single_image_to_db(&cropped, None) {
                    identified_monsters.push(monster_name);
                }
            }
        }
    }
    
    println!("[YOLO Recognition] Identified {} monsters in {:?}", identified_monsters.len(), start_total.elapsed());
    Ok(identified_monsters)
}

fn intersection_area_val(a: &YoloDetection, b: &YoloDetection) -> f32 {
    let x1 = a.x1.max(b.x1);
    let y1 = a.y1.max(b.y1);
    let x2 = a.x2.min(b.x2);
    let y2 = a.y2.min(b.y2);
    (x2 - x1).max(0) as f32 * (y2 - y1).max(0) as f32
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
// 按size分类的卡牌特征缓存
static CARD_SMALL_CACHE: OnceLock<Vec<TemplateCache>> = OnceLock::new();
static CARD_MEDIUM_CACHE: OnceLock<Vec<TemplateCache>> = OnceLock::new();
static CARD_LARGE_CACHE: OnceLock<Vec<TemplateCache>> = OnceLock::new();
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

#[derive(Debug, Clone, Serialize)]
pub struct RecognitionCacheMemoryStat {
    pub key: String,
    pub count: usize,
    pub estimated_bytes: u64,
    pub note: Option<String>,
}

fn estimate_template_dynamic_bytes(entry: &TemplateCache) -> u64 {
    (entry.name.capacity() as u64)
        + (entry.day.capacity() as u64)
        + (entry.keypoints.capacity() as u64 * std::mem::size_of::<(f32, f32)>() as u64)
        + (entry.descriptors.capacity() as u64)
        + (entry.sample_png.capacity() as u64)
}

fn estimate_template_cache_bytes(cache: &Vec<TemplateCache>) -> u64 {
    let outer = std::mem::size_of::<Vec<TemplateCache>>() as u64
        + (cache.capacity() as u64 * std::mem::size_of::<TemplateCache>() as u64);
    outer
        + cache
            .iter()
            .map(estimate_template_dynamic_bytes)
            .sum::<u64>()
}

fn estimate_event_template_dynamic_bytes(entry: &EventTemplateCache) -> u64 {
    (entry.id.capacity() as u64) + (entry.name.capacity() as u64) + (entry.descriptors.capacity() as u64)
}

fn estimate_event_template_cache_bytes(cache: &Vec<EventTemplateCache>) -> u64 {
    let outer = std::mem::size_of::<Vec<EventTemplateCache>>() as u64
        + (cache.capacity() as u64 * std::mem::size_of::<EventTemplateCache>() as u64);
    outer
        + cache
            .iter()
            .map(estimate_event_template_dynamic_bytes)
            .sum::<u64>()
}

pub fn collect_recognition_cache_memory_stats() -> Vec<RecognitionCacheMemoryStat> {
    let mut stats = Vec::new();

    let push_template =
        |key: &str, cache: Option<&Vec<TemplateCache>>, stats: &mut Vec<RecognitionCacheMemoryStat>| {
            if let Some(c) = cache {
                stats.push(RecognitionCacheMemoryStat {
                    key: key.to_string(),
                    count: c.len(),
                    estimated_bytes: estimate_template_cache_bytes(c),
                    note: None,
                });
            } else {
                stats.push(RecognitionCacheMemoryStat {
                    key: key.to_string(),
                    count: 0,
                    estimated_bytes: 0,
                    note: Some("not_loaded".to_string()),
                });
            }
        };

    push_template("cache:monster_templates", TEMPLATE_CACHE.get(), &mut stats);
    push_template("cache:card_templates", CARD_TEMPLATE_CACHE.get(), &mut stats);
    push_template("cache:card_templates_small", CARD_SMALL_CACHE.get(), &mut stats);
    push_template("cache:card_templates_medium", CARD_MEDIUM_CACHE.get(), &mut stats);
    push_template("cache:card_templates_large", CARD_LARGE_CACHE.get(), &mut stats);

    if let Some(event_cache) = EVENT_TEMPLATE_CACHE.get() {
        stats.push(RecognitionCacheMemoryStat {
            key: "cache:event_templates".to_string(),
            count: event_cache.len(),
            estimated_bytes: estimate_event_template_cache_bytes(event_cache),
            note: None,
        });
    } else {
        stats.push(RecognitionCacheMemoryStat {
            key: "cache:event_templates".to_string(),
            count: 0,
            estimated_bytes: 0,
            note: Some("not_loaded".to_string()),
        });
    }

    stats.push(RecognitionCacheMemoryStat {
        key: "cache:yolo_session".to_string(),
        count: {
            #[cfg(target_os = "windows")]
            let mut loaded = if YOLO_SESSION_CPU.get().is_some() { 1 } else { 0 };
            #[cfg(not(target_os = "windows"))]
            let loaded = if YOLO_SESSION_CPU.get().is_some() { 1 } else { 0 };
            #[cfg(target_os = "windows")]
            {
                if YOLO_SESSION_DML.get().is_some() {
                    loaded += 1;
                }
            }
            loaded
        },
        estimated_bytes: 0,
        note: Some("native_session_unmeasured".to_string()),
    });

    stats
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
    let mut results = Vec::new();

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

            results.push((template.name.clone(), matches, score));
        }
    }
    
    results.sort_by(|a, b| b.1.cmp(&a.1));

    // Print Top 3 Monster Candidates
    println!("[Monster Recognition] Top 3 Candidates:");
    for i in 0..results.len().min(3) {
         let (name, matches, score) = &results[i];
         println!("  {}. {} - Matches: {}, Score: {:.4}", i+1, name, matches, score);
    }

    if let Some((best_name, max_matches, best_score)) = results.first() {
        if *max_matches >= 10 || *best_score > 0.15 {
            return Ok(Some(best_name.clone()));
        }
    }
    
    Ok(None)
}

// 从 Mat 匹配事件描述符，返回事件ID
pub fn match_event_descriptors_from_mat(scene_descriptors: &Mat) -> Result<Option<String>, String> {
    let cache = EVENT_TEMPLATE_CACHE.get().ok_or("Event templates not loaded")?;
    let mut results = Vec::new();
    
    println!("[Event Recognition] Scene has {} descriptors", scene_descriptors.rows());

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

            results.push((template.id.clone(), template.name.clone(), matches, score, template_kp_count as i32));
        }
    }

    results.sort_by(|a, b| b.2.cmp(&a.2)); // Sort by matches desc

    // Print Top 5 Candidate Events with more details
    println!("[Event Recognition] Top 5 Candidates:");
    for i in 0..results.len().min(5) {
         let (_id, name, matches, score, template_kp) = &results[i];
         println!("  {}. {} - Matches: {}/{} (Scene: {}), Score: {:.4}", 
                  i+1, name, matches, template_kp, scene_descriptors.rows(), score);
    }
    
    if let Some((best_id, best_name, max_matches, best_score, _)) = results.first() {
        // 事件识别阈值：匹配点数 >= 12 且 得分 > 0.12 (降低阈值以提高召回率)
        if *max_matches >= 12 && *best_score > 0.12 {
             println!("[Event Recognition] ✓ Matched: {} (Matches: {}, Score: {:.4})", best_name, max_matches, best_score);
             return Ok(Some(best_id.clone()));
        } else {
             println!("[Event Recognition] ✗ No match above threshold (Best: {} with {} matches, {:.4} score)", best_name, max_matches, best_score);
        }
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
            if let Some(p) = resolve_existing_resource(&resources_dir, rel_path) {
                name_to_path.insert(key.clone(), p);
            } else {
                let alt_rel = rel_path.replace("images_monster/", "assets/monsters/characters/");
                if let Some(char_path) = resolve_existing_resource(&resources_dir, &alt_rel) {
                    name_to_path.insert(key.clone(), char_path);
                }
            }
        }
        
        // 2. 检查以 key 为名的直接图片 (e.g. 绿洲守护神_Day9.webp)
        let char_path_key = resolve_existing_resource(&resources_dir, &format!("assets/monsters/characters/{}.webp", key));
        if let Some(path) = char_path_key {
            name_to_path.insert(key.clone(), path);
        }

        // 3. 检查以 name_zh 为名的直接图片 (e.g. 绿洲守护神.webp)
        if let Some(name_zh) = entry.name_zh.as_ref() {
            let char_path_name = resolve_existing_resource(&resources_dir, &format!("assets/monsters/characters/{}.webp", name_zh));
            if let Some(path) = char_path_name {
                name_to_path.insert(key.clone(), path);
            } else {
                // 特殊处理：如果带前缀（如 "毒素 吹箭枪陷阱"），尝试查找基础名称 "吹箭枪陷阱.webp"
                if let Some(space_pos) = name_zh.rfind(' ') {
                    let base_name = &name_zh[space_pos + 1..];
                    if let Some(base_path) = resolve_existing_resource(&resources_dir, &format!("assets/monsters/characters/{}.webp", base_name)) {
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


// 跨平台获取鼠标位置
fn get_mouse_position() -> (i32, i32) {
    let result = std::panic::catch_unwind(|| {
        let device_state = DeviceState::new();
        let mouse = device_state.get_mouse();
        (mouse.coords.0, mouse.coords.1)
    });
    match result {
        Ok(pos) => pos,
        Err(_) => {
            log::warn!("[Mouse] Failed to read mouse position (likely missing accessibility permission on macOS).");
            (0, 0)
        }
    }
}

// 公共函数：鼠标触发的怪物识别
pub fn scan_and_identify_monster_at_mouse() -> Result<Option<String>, String> {
    use xcap::Monitor;

    // 1. 获取鼠标位置（跨平台）
    let (mouse_x, mouse_y) = get_mouse_position();

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
    let (_width, _height) = img.dimensions();

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
    
    // Apply the crop_focus sequence:
    // 1) Keep top 50%
    let step1 = crop_focus(&img, 0.5, 1.0, 0.0, 0, "top");
    save_debug_image(&step1, "monster_step1_top50");
    
    // 2) From top 50%, keep bottom 70% (effectively 35% of original height)
    let step2 = crop_focus(&step1, 0.7, 1.0, 0.0, 0, "bottom");
    save_debug_image(&step2, "monster_step2_bottom70");
    
    // 3) Final horizontal crop to center 5/12 ≈ 41.67%
    let final_crop = crop_focus(&step2, 1.0, 5.0/12.0, 0.0, 0, "top");
    save_debug_image(&final_crop, "monster_final_crop");
    
    let (crop_width, crop_height) = final_crop.dimensions();
    let slot_w = crop_width / 3;
    let slot_h = crop_height;

    let start_match = Instant::now();

    for i in 0..3 {
        let start_slot = Instant::now();
        let x = i as u32 * slot_w;
        let y = 0;
        if x + slot_w > crop_width || y + slot_h > crop_height { continue; }

        let slice = final_crop.crop_imm(x, y, slot_w, slot_h);
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
    // Set recognition flag to prevent focus monitor from hiding overlays during screenshot/processing
    use std::sync::atomic::Ordering;
    struct RecognitionGuard;
    impl Drop for RecognitionGuard { 
        fn drop(&mut self) { 
            crate::core::recognition_state::IS_RECOGNIZING.store(false, Ordering::Relaxed);
            crate::core::recognition_state::update_last_recog_time(); // Add grace period
        } 
    }
    crate::core::recognition_state::IS_RECOGNIZING.store(true, Ordering::Relaxed);
    let _guard = RecognitionGuard;

    use xcap::Monitor;
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

    // 2. 截图：直接以“鼠标所在显示器”为基准，避免窗口坐标系(DPI/Retina)不一致导致裁剪越界。
    let monitors = Monitor::all().map_err(|e| e.to_string())?;
    let mut target_monitor = monitors.into_iter().find(|m| {
        let mx = m.x();
        let my = m.y();
        let mw = m.width() as i32;
        let mh = m.height() as i32;
        mouse_x >= mx && mouse_x < mx + mw && mouse_y >= my && mouse_y < my + mh
    });

    // Retina/缩放兼容：如果首轮没命中，尝试按 0.5/2.0 比例重新匹配。
    if target_monitor.is_none() {
        let monitors_retry = Monitor::all().map_err(|e| e.to_string())?;
        let candidates = [
            (mouse_x / 2, mouse_y / 2),
            (mouse_x.saturating_mul(2), mouse_y.saturating_mul(2)),
        ];
        target_monitor = monitors_retry.into_iter().find(|m| {
            let mx = m.x();
            let my = m.y();
            let mw = m.width() as i32;
            let mh = m.height() as i32;
            candidates.iter().any(|(cx, cy)| *cx >= mx && *cx < mx + mw && *cy >= my && *cy < my + mh)
        });
    }

    let target_monitor = target_monitor.ok_or("Mouse not in monitor")?;
    let screenshot = target_monitor.capture_image().map_err(|e| e.to_string())?;
    let win_x = target_monitor.x();
    let win_y = target_monitor.y();
    let win_w = target_monitor.width() as i32;
    let win_h = target_monitor.height() as i32;

    let img = DynamicImage::ImageRgba8(screenshot);
    let (img_w, img_h) = img.dimensions();
    let rel_x_raw = mouse_x - win_x;
    let rel_y_raw = mouse_y - win_y;
    let scale_x = if win_w > 0 { img_w as f32 / win_w as f32 } else { 1.0 };
    let scale_y = if win_h > 0 { img_h as f32 / win_h as f32 } else { 1.0 };
    let rel_x = (rel_x_raw as f32 * scale_x).round() as i32;
    let rel_y = (rel_y_raw as f32 * scale_y).round() as i32;
    let rel_x = rel_x.clamp(0, (img_w.saturating_sub(1)) as i32);
    let rel_y = rel_y.clamp(0, (img_h.saturating_sub(1)) as i32);
    
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

    if crop_w < 50 || crop_h < 50 {
        log::warn!(
            "[Card Recognition] Invalid crop (w={}, h={}) mouse=({},{}), monitor=({},{} {}x{}), rel_raw=({},{}), rel_scaled=({},{}), img={}x{}",
            crop_w, crop_h, mouse_x, mouse_y, win_x, win_y, win_w, win_h, rel_x_raw, rel_y_raw, rel_x, rel_y, img_w, img_h
        );
        return Ok(None);
    }
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

    // Print raw top 3 candidates for debugging
    println!("[Card Recognition] Top 3 Candidates:");
    for i in 0..results.len().min(3) {
        let (top, matches, confidence) = results[i];
        println!("  {}. {} (ID: {}) - Matches: {}, Conf: {:.4}", i+1, top.name, top.day, matches, confidence);
    }

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

// ===== 事件识别功能 =====

static EVENT_TEMPLATE_CACHE: OnceLock<Vec<EventTemplateCache>> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EventTemplateCache {
    id: String,
    name: String,
    descriptors: Vec<u8>,
    descriptor_rows: i32,
    descriptor_cols: i32,
}

// 加载事件特征模板
#[tauri::command]
pub async fn load_event_templates(app: tauri::AppHandle) -> Result<(), String> {
    if EVENT_TEMPLATE_CACHE.get().is_some() {
        println!("Event templates already loaded");
        return Ok(());
    }
    
    println!("开始加载事件特征模板...");
    log_to_file("Loading event templates...");

    // 1. 尝试从资源目录加载 (Bundled)
    let feature_bin_path = app.path().resolve("resources/event_features_opencv.bin", tauri::path::BaseDirectory::Resource);
    if let Ok(bin_path) = feature_bin_path {
         if bin_path.exists() {
             match std::fs::read(&bin_path) {
                 Ok(data) => {
                     match bincode::deserialize::<Vec<EventTemplateCache>>(&data) {
                         Ok(cached_templates) => {
                             if !cached_templates.is_empty() {
                                 println!("Loaded {} event templates from binary package (Resource)", cached_templates.len());
                                 log_to_file(&format!("Loaded {} event templates from binary package (Resource)", cached_templates.len()));
                                 
                                 let _ = EVENT_TEMPLATE_CACHE.set(cached_templates);
                                 return Ok(());
                             }
                         },
                         Err(e) => {
                             log_to_file(&format!("Failed to deserialize resource cache: {}", e));
                         }
                     }
                 },
                 Err(e) => {
                    log_to_file(&format!("Failed to read resource cache: {}", e));
                 }
            }
        }
    }

    // 2. 尝试从 AppCache 加载 (Generated)
    if let Ok(cache_dir) = app.path().app_cache_dir() {
        let cached_bin = cache_dir.join("event_features_opencv.bin");
        if cached_bin.exists() {
            match std::fs::read(&cached_bin) {
                Ok(data) => {
                    match bincode::deserialize::<Vec<EventTemplateCache>>(&data) {
                        Ok(cached_templates) => {
                             if !cached_templates.is_empty() {
                                 println!("Loaded {} event templates from generated cache", cached_templates.len());
                                 log_to_file(&format!("Loaded {} event templates from generated cache", cached_templates.len()));
                                 
                                 let _ = EVENT_TEMPLATE_CACHE.set(cached_templates);
                                 return Ok(());
                             }
                        },
                         Err(e) => {
                             log_to_file(&format!("Failed to deserialize generated cache: {}", e));
                        }
                    }
                },
                 Err(e) => {
                    log_to_file(&format!("Failed to read generated cache: {}", e));
                 }
            }
        }
    }

    log_to_file("Event cache not found or invalid. Starting generation from source images...");

    // 3. 生成特征
    // 初始化 ORB (增加特征点数量以提高匹配率)
    let mut orb = ORB::create(
        1000, // nfeatures (从500提升到1000，提取更多特征点)
        1.2f32, // scaleFactor
        8, // nlevels
        31, // edgeThreshold
        0, // firstLevel
        2, // WTA_K
        opencv::features2d::ORB_ScoreType::HARRIS_SCORE, // scoreType
        31, // patchSize
        20 // fastThreshold
    ).map_err(|e| format!("Failed to create ORB detector: {}", e))?;
    
    // 读取 event_encounters.json
    let event_json_path = app.path().resolve("resources/event_encounters.json", tauri::path::BaseDirectory::Resource)
        .map_err(|e| format!("Failed to resolve event_encounters.json: {}", e))?;
    
    let json_data = std::fs::read_to_string(&event_json_path)
        .map_err(|e| format!("Failed to read event_encounters.json: {}", e))?;
    
    let events: Vec<serde_json::Value> = serde_json::from_str(&json_data)
        .map_err(|e| format!("Failed to parse event_encounters.json: {}", e))?;
    
    // 资源根目录
    let resources_dir = app.path().resolve("resources", tauri::path::BaseDirectory::Resource)
        .map_err(|e| format!("Failed to resolve resources dir: {}", e))?;
    
    let mut templates = Vec::new();
    let mut loaded_count = 0;
    let mut missing_count = 0;
    
    for event in events {
        // 只处理有 choices 的事件
        if let Some(choices) = event.get("choices") {
            if let Some(arr) = choices.as_array() {
                if arr.is_empty() {
                    continue;
                }
            } else {
                continue;
            }
        } else {
            continue;
        }
        
        let id = event.get("Id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        
        if id.is_empty() {
            continue;
        }

        // 获取图片路径，支持多种策略查找
        let mut potential_paths = Vec::new();
        
        // 1. 优先使用 JSON 中的 image_paths
        if let Some(p) = event.get("image_paths")
            .and_then(|paths| paths.get("char"))
            .and_then(|c| c.as_str()) {
            let resolved_path = resolve_existing_resource(&resources_dir, p)
                .unwrap_or_else(|| resources_dir.join(p));
            potential_paths.push(resolved_path.clone());
            
            // 尝试替换扩展名 (应对 webp/png 不一致)
            if let Some(stem) = resolved_path.file_stem() {
                 if let Some(parent) = resolved_path.parent() {
                     potential_paths.push(parent.join(stem).with_extension("png"));
                     potential_paths.push(parent.join(stem).with_extension("jpg"));
                 }
            }
        }

        // 2. 尝试 ID 组合猜测
        potential_paths.push(resources_dir.join(format!("assets/events/characters/{}.png", id)));
        potential_paths.push(resources_dir.join(format!("assets/events/characters/{}.webp", id)));
        potential_paths.push(resources_dir.join(format!("assets/events/characters/{}.jpg", id)));
        potential_paths.push(resources_dir.join(format!("assets/events/characters/{}_Char.png", id)));
        potential_paths.push(resources_dir.join(format!("assets/events/characters/{}_Char.webp", id)));

        potential_paths.push(resources_dir.join(format!("EncEvent_CHAR/{}.png", id)));
        potential_paths.push(resources_dir.join(format!("EncEvent_CHAR/{}.webp", id)));
        potential_paths.push(resources_dir.join(format!("EncEvent_CHAR/{}.jpg", id)));
        potential_paths.push(resources_dir.join(format!("EncEvent_CHAR/{}_Char.png", id)));
        potential_paths.push(resources_dir.join(format!("EncEvent_CHAR/{}_Char.webp", id)));
        
        // 3. 尝试 BG 目录
        potential_paths.push(resources_dir.join(format!("assets/events/backgrounds/{}.png", id)));
        potential_paths.push(resources_dir.join(format!("assets/events/backgrounds/{}_BG.png", id)));
        potential_paths.push(resources_dir.join(format!("EncEvent_BG/{}.png", id)));
        potential_paths.push(resources_dir.join(format!("EncEvent_BG/{}_BG.png", id)));

        let img_path_final = potential_paths.into_iter().find(|p| p.exists());

        if img_path_final.is_none() {
            missing_count += 1;
            // log_to_file(&format!("Missing image for event: {}", id));
            continue;
        }
        
        let valid_path = img_path_final.unwrap();
        let img_path_str = valid_path.to_str().unwrap_or_default();
        
        // OpenCV 读取图片 (Grayscale)
        let img = match opencv::imgcodecs::imread(img_path_str, opencv::imgcodecs::IMREAD_GRAYSCALE) {
            Ok(img) => img,
            Err(e) => {
                log_to_file(&format!("Failed to load image for {}: {}", id, e));
                continue;
            }
        };

        if img.empty() {
             // log_to_file(&format!("Loaded empty image for {}", id));
             continue;
        }

        // 计算特征
        let mut keypoints = Vector::<KeyPoint>::new();
        let mut descriptors = Mat::default();
        
        if let Err(e) = orb.detect_and_compute(&img, &Mat::default(), &mut keypoints, &mut descriptors, false) {
             log_to_file(&format!("ORB compute failed for {}: {}", id, e));
             continue;
        }
        
        if descriptors.rows() == 0 || descriptors.cols() == 0 {
             // log_to_file(&format!("No descriptors found for {}", id));
             continue;
        }

        let name = event.get("Localization")
            .and_then(|l| l.get("Title"))
            .and_then(|t| t.get("Text"))
            .and_then(|t| t.as_str())
            .unwrap_or(&id)
            .to_string();

        // 将 Mat 转换为 Vec<u8>
        let mut descriptors_vec = Vec::new();
        if descriptors.is_continuous() {
             if let Ok(data_slice) = descriptors.data_typed::<u8>() {
                 descriptors_vec = data_slice.to_vec();
             }
        } else {
             for i in 0..descriptors.rows() {
                  for j in 0..descriptors.cols() {
                       let val = descriptors.at_2d::<u8>(i, j).unwrap_or(&0);
                       descriptors_vec.push(*val);
                  }
             }
        }

        templates.push(EventTemplateCache {
            id: id.clone(),
            name: name.clone(),
            descriptors: descriptors_vec,
            descriptor_rows: descriptors.rows(),
            descriptor_cols: descriptors.cols(),
        });
        
        loaded_count += 1;
    }
    
    println!("Generated event templates: {} success, {} missing", loaded_count, missing_count);
    log_to_file(&format!("Event templates generated: {} success, {} missing", loaded_count, missing_count));
    
    if !templates.is_empty() {
        // 保存生成的缓存到 AppCache
        if let Ok(cache_dir) = app.path().app_cache_dir() {
            if let Err(e) = std::fs::create_dir_all(&cache_dir) {
                 log_to_file(&format!("Failed to create cache dir: {}", e));
            } else {
                let cache_path = cache_dir.join("event_features_opencv.bin");
                match bincode::serialize(&templates) {
                    Ok(data) => {
                        if let Err(e) = std::fs::write(&cache_path, data) {
                             log_to_file(&format!("Failed to write generated cache to file: {}", e));
                        } else {
                             log_to_file(&format!("Saved generated event cache to {:?}", cache_path));
                        }
                    },
                    Err(e) => {
                        log_to_file(&format!("Failed to serialize generated templates: {}", e));
                    }
                }
            }
        }
    }

    let _ = EVENT_TEMPLATE_CACHE.set(templates);
    Ok(())
}

// 识别事件（从鼠标位置）
#[tauri::command]
pub async fn recognize_event_at_mouse() -> Result<Option<serde_json::Value>, String> {
    use xcap::Monitor;

    // 1. 获取鼠标位置（跨平台）
    let (mouse_x, mouse_y) = get_mouse_position();

    // 2. 截图
    let windows = xcap::Window::all().map_err(|e| e.to_string())?;
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
            mouse_x >= wx && mouse_x < wx + ww as i32 &&
            mouse_y >= wy && mouse_y < wy + wh as i32
        } else {
            false
        }
    });

    let (screenshot, win_x, win_y) = if let Some(window) = bazaar_window {
        (window.capture_image().map_err(|e| e.to_string())?, window.x(), window.y())
    } else {
        let monitors = Monitor::all().map_err(|e| e.to_string())?;
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
    let rel_x = mouse_x - win_x;
    let rel_y = mouse_y - win_y;
    
    // 裁剪 400x400 区域
    let crop_size = 400;
    let half_size = crop_size / 2;
    
    let crop_x = (rel_x - half_size).max(0) as u32;
    let crop_y = (rel_y - half_size).max(0) as u32;
    
    let crop_w = if crop_x + crop_size as u32 > img_w { img_w.saturating_sub(crop_x) } else { crop_size as u32 };
    let crop_h = if crop_y + crop_size as u32 > img_h { img_h.saturating_sub(crop_y) } else { crop_size as u32 };

    if crop_w < 50 || crop_h < 50 {
        return Err("裁剪区域太小或鼠标已移出窗口范围".into());
    }

    let cropped_img = img.crop_imm(crop_x, crop_y, crop_w, crop_h);
    
    // 3. 提取特征
    let scene_desc = extract_features_from_dynamic_image(&cropped_img, 500).map_err(|e| e.to_string())?;
    if scene_desc.empty() { return Ok(None); }
    
    // 4. 与事件模板比对
    let cache = EVENT_TEMPLATE_CACHE.get().ok_or("Event templates not loaded")?;
    let mut results: Vec<(&EventTemplateCache, usize, f32)> = Vec::new();

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

    // 取最佳匹配（阈值：匹配点数 > 15 且置信度 > 0.15）
    if let Some((best, matches, confidence)) = results.first() {
        if *matches > 15 && *confidence > 0.15 {
            println!("[Event Recognition] Matched: {} (confidence: {:.2}, matches: {})", best.name, confidence, matches);
            return Ok(Some(serde_json::json!({
                "id": best.id,
                "name": best.name,
                "confidence": confidence,
                "match_count": matches
            })));
        }
    }
    
    println!("[Event Recognition] No event matches found above threshold.");
    Ok(None)
}

// ============== 按size分类的卡牌识别 ==============

/// 按size分类加载卡牌特征
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
    let mut all_loaded = true;
    
    // Small
    if !try_load_size_cache(&small_bundled, &small_cache_file, &CARD_SMALL_CACHE) {
        all_loaded = false;
    }
    
    // Medium
    if !try_load_size_cache(&medium_bundled, &medium_cache_file, &CARD_MEDIUM_CACHE) {
        all_loaded = false;
    }
    
    // Large
    if !try_load_size_cache(&large_bundled, &large_cache_file, &CARD_LARGE_CACHE) {
        all_loaded = false;
    }
    
    if all_loaded {
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
    let small_cache = build_size_feature_cache(small_tasks);
    let medium_cache = build_size_feature_cache(medium_tasks);
    let large_cache = build_size_feature_cache(large_tasks);
    
    log_to_file(&format!("Successfully built: Small={}, Medium={}, Large={}", 
                         small_cache.len(), medium_cache.len(), large_cache.len()));
    
    // 保存缓存
    save_size_cache(&small_cache, &small_cache_file, &small_bundled, "Small");
    save_size_cache(&medium_cache, &medium_cache_file, &medium_bundled, "Medium");
    save_size_cache(&large_cache, &large_cache_file, &large_bundled, "Large");
    
    // 设置到全局缓存
    let _ = CARD_SMALL_CACHE.set(small_cache);
    let _ = CARD_MEDIUM_CACHE.set(medium_cache);
    let _ = CARD_LARGE_CACHE.set(large_cache);
    
    Ok(())
}

fn try_load_size_cache(bundled: &PathBuf, cache_file: &PathBuf, cache: &OnceLock<Vec<TemplateCache>>) -> bool {
    // 优先从bundled加载
    if bundled.exists() {
        if let Ok(data) = std::fs::read(bundled) {
            if let Ok(templates) = bincode::deserialize::<Vec<TemplateCache>>(&data) {
                if !templates.is_empty() {
                    log_to_file(&format!("Loaded {} templates from {:?}", templates.len(), bundled));
                    println!("[Card Size] Loaded {} templates from {:?}", templates.len(), bundled);
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
                    println!("[Card Size] Loaded {} templates from {:?}", templates.len(), cache_file);
                    let _ = cache.set(templates);
                    return true;
                }
            }
        }
    }
    
    false
}

fn build_size_feature_cache(tasks: Vec<(String, String, PathBuf)>) -> Vec<TemplateCache> {
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

fn save_size_cache(cache: &Vec<TemplateCache>, cache_file: &PathBuf, bundled: &PathBuf, label: &str) {
    if let Ok(serialized) = bincode::serialize(cache) {
        let _ = std::fs::write(cache_file, &serialized);
        let _ = std::fs::write(bundled, &serialized);
        log_to_file(&format!("Saved {} card cache: appdata={:?}, resources={:?}", 
                             label, cache_file, bundled));
        println!("[Card Size] Saved {} cache: appdata={:?}, resources={:?}", label, cache_file, bundled);
    }
}

/// 根据size匹配卡牌
pub fn match_card_by_size(scene_desc: &Mat, size: &str) -> Result<Option<serde_json::Value>, String> {
    let cache = match size {
        "Small" => CARD_SMALL_CACHE.get(),
        "Medium" => CARD_MEDIUM_CACHE.get(),
        "Large" => CARD_LARGE_CACHE.get(),
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
        println!("[Card Match] Found {} matches in {} category (best match: {}, confidence: {:.2})", 
                 matches_found.len(), size, matches_found[0]["name"], matches_found[0]["confidence"]);
        return Ok(Some(serde_json::json!(matches_found)));
    }
    Ok(None)
}

#[tauri::command]
pub async fn recognize_monster_at_mouse() -> Result<Option<String>, String> {
    // 1. 设置识别标志 (防止窗口隐藏)
    use std::sync::atomic::Ordering;
    struct RecognitionGuard;
    impl Drop for RecognitionGuard { 
        fn drop(&mut self) { 
            crate::core::recognition_state::IS_RECOGNIZING.store(false, Ordering::Relaxed);
            crate::core::recognition_state::update_last_recog_time(); 
        } 
    }
    crate::core::recognition_state::IS_RECOGNIZING.store(true, Ordering::Relaxed);
    let _guard = RecognitionGuard;

    use xcap::{Window, Monitor};
    use enigo::{Enigo, Mouse, Settings};

    // 2. 获取鼠标位置
    let enigo = Enigo::new(&Settings::default()).map_err(|e| format!("{:?}", e))?;
    let (mouse_x, mouse_y) = enigo.location().map_err(|e| format!("{:?}", e))?;

    // 3. 截屏 (围绕鼠标)
    let windows = Window::all().map_err(|e| e.to_string())?;
    let bazaar_window = windows.into_iter().find(|w| {
        let title = w.title().to_lowercase();
        let name = w.app_name().to_lowercase();
        title.contains("the bazaar") || name.contains("the bazaar")
    });
    
    let (screenshot, win_x, win_y) = if let Some(w) = bazaar_window {
         (w.capture_image().map_err(|e| e.to_string())?, w.x(), w.y())
    } else {
         let monitors = Monitor::all().map_err(|e| e.to_string())?;
         let m = monitors.into_iter().find(|m| {
             let mx = m.x(); let my = m.y(); let mw = m.width(); let mh = m.height();
             mouse_x >= mx && mouse_x < mx + mw as i32 && mouse_y >= my && mouse_y < my + mh as i32
         }).ok_or("Mouse off screen")?;
         (m.capture_image().map_err(|e| e.to_string())?, m.x(), m.y())
    };

    let img = DynamicImage::ImageRgba8(screenshot);
    let (img_w, img_h) = img.dimensions();
    
    // 裁剪参数: 怪物卡牌通常比物品卡大，或者主要是上半部分
    // 使用 600x600 (增大识别范围)
    let crop_size = 600;
    let half = crop_size / 2;
    let rel_x = mouse_x - win_x;
    let rel_y = mouse_y - win_y;
    
    let cx = (rel_x - half).max(0) as u32;
    let cy = (rel_y - half).max(0) as u32;
    let cw = if cx + crop_size as u32 > img_w { img_w.saturating_sub(cx) } else { crop_size as u32 };
    let ch = if cy + crop_size as u32 > img_h { img_h.saturating_sub(cy) } else { crop_size as u32 };
    
    if cw < 50 || ch < 50 { return Err("Crop too small".into()); }
    
    let cropped = img.crop_imm(cx, cy, cw, ch);
    save_debug_image(&cropped, "monster_mouse_crop");
    
    // 4. 提取特征
    let scene_desc = extract_features_from_dynamic_image(&cropped, 800).map_err(|e| e.to_string())?;
    if scene_desc.empty() { return Ok(None); }
    
    // 5. 比对 TEMPLATE_CACHE (怪兽库)
    let cache = TEMPLATE_CACHE.get().ok_or("Monster templates not loaded")?;
    
    let mut results: Vec<(&TemplateCache, usize, f32)> = Vec::new(); // (Template, Matches, Confidence)

    for template in cache {
        if template.descriptors.is_empty() { continue; }
        use opencv::core::CV_8U;
        // Rebuild Mat
        let mut t_desc = unsafe { Mat::new_rows_cols(template.descriptor_rows, template.descriptor_cols, CV_8U).unwrap() };
        unsafe { std::ptr::copy_nonoverlapping(template.descriptors.as_ptr(), t_desc.data_mut() as *mut u8, template.descriptors.len()); }
        
        if let Ok(matches) = match_orb_descriptors(&scene_desc, &t_desc) {
             let min_kp = (template.descriptor_rows as f32).min(scene_desc.rows() as f32);
             let conf = if min_kp > 0.0 { matches as f32 / min_kp } else { 0.0 };
             if matches > 15 && conf > 0.15 {
                 results.push((template, matches, conf));
             }
        }
    }
    
    results.sort_by(|a, b| b.1.cmp(&a.1));
    
    if let Some((best, matches, conf)) = results.first() {
        println!("[Monster Mouse Recog] Matched {} (Day: {}, Matches: {}, Conf: {:.2})", best.name, best.day, matches, conf);
        // Returns format: "Day|MonsterName" for frontend parsing
        let result = format!("{}|{}", best.day, best.name);
        return Ok(Some(result));
    }

    Ok(None)
}
