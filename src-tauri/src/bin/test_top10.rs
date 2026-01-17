use opencv::{prelude::*, core::Mat, features2d::BFMatcher};
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TemplateCache {
    name: String,
    day: String,
    #[serde(skip)]
    #[allow(dead_code)]
    keypoints: Vec<f32>,
    descriptors: Vec<u8>,
    descriptor_rows: i32,
    descriptor_cols: i32,
    sample_png: Vec<u8>,
    sample_w: u32,
    sample_h: u32,
}

fn extract_features_orb(image_path: &str) -> Result<Mat, String> {
    use opencv::{imgcodecs, imgproc, features2d::ORB, core::AlgorithmHint};
    
    // Read file content first to handle non-ASCII paths on Windows
    let content = std::fs::read(image_path)
        .map_err(|e| format!("Failed to read file {}: {}", image_path, e))?;
    
    let img = imgcodecs::imdecode(&Mat::from_slice(&content).unwrap(), imgcodecs::IMREAD_COLOR)
        .map_err(|e| format!("Failed to decode image {}: {}", image_path, e))?;
    
    if img.empty() {
        return Err(format!("Decoded image is empty: {}", image_path));
    }
    
    let mut gray = Mat::default();
    imgproc::cvt_color(&img, &mut gray, imgproc::COLOR_BGR2GRAY, 0, AlgorithmHint::ALGO_HINT_DEFAULT)
        .map_err(|e| format!("Failed to convert to gray: {}", e))?;
    
    let mut orb = ORB::create(500, 1.2f32, 8, 31, 0, 2, 
        opencv::features2d::ORB_ScoreType::HARRIS_SCORE, 31, 20)
        .map_err(|e| format!("Failed to create ORB: {}", e))?;
    
    let mut keypoints = opencv::core::Vector::new();
    let mut descriptors = Mat::default();
    
    orb.detect_and_compute(&gray, &Mat::default(), &mut keypoints, &mut descriptors, false)
        .map_err(|e| format!("Failed to detect: {}", e))?;
    
    Ok(descriptors)
}

fn match_orb_descriptors(desc1: &Mat, desc2: &Mat) -> Result<usize, String> {
    use opencv::core::{NORM_HAMMING, Vector};
    
    if desc1.empty() || desc2.empty() {
        return Ok(0);
    }
    
    let matcher = BFMatcher::create(NORM_HAMMING, false)
        .map_err(|e| format!("Failed to create matcher: {}", e))?;
    
    let mut knn_matches = Vector::<Vector<opencv::core::DMatch>>::new();
    matcher.knn_train_match(desc1, desc2, &mut knn_matches, 2, &Mat::default(), false)
        .map_err(|e| format!("knn_match failed: {}", e))?;
    
    let mut good_matches = 0;
    for m in knn_matches.iter() {
        if m.len() >= 2 {
            let m0 = m.get(0).unwrap();
            let m1 = m.get(1).unwrap();
            if m0.distance < 0.75 * m1.distance {
                good_matches += 1;
            }
        } else if m.len() == 1 {
            good_matches += 1;
        }
    }
    
    Ok(good_matches)
}

fn main() {
    println!("=== 怪物识别 Top10 测试 ===\n");
    
    // 直接从 images_monster_char 目录直接加载
    println!("正在从 images_monster_char 目录加载怪物模板...");
    let mut cache = Vec::new();
    
    // 假设在 src-tauri 目录下运行
    let monster_dir = PathBuf::from("resources/images_monster_char");

    // 检查目录是否存在
    let search_path = if monster_dir.exists() {
        monster_dir
    } else {
        // 尝试回退到 workspace 根目录查找
        let alt = PathBuf::from("src-tauri/resources/images_monster_char");
        if alt.exists() {
            alt
        } else {
             // 绝对路径尝试
             let abs = PathBuf::from("D:/Projects/BazaarHelper/src-tauri/resources/images_monster_char");
             if abs.exists() {
                 abs
             } else {
                 panic!("无法找到 images_monster_char 目录");
             }
        }
    };
    
    scan_dir_and_extract(&search_path, &mut cache);
    
    println!("成功加载了 {} 个怪物模板\n", cache.len());
    
    // 测试图片路径 (尝试多个可能的路径)
    let test_images_base = vec![
        "D:/Projects/BazaarHelper/src-tauri/target/debug/examples/final_left.jpg",
        "D:/Projects/BazaarHelper/src-tauri/target/debug/examples/final_mid.jpg",
        "D:/Projects/BazaarHelper/src-tauri/target/debug/examples/final_right.jpg",
    ];
    
    let total_start = std::time::Instant::now();
    
    for (i, base_name) in test_images_base.iter().enumerate() {
        println!("测试图片 {}: {}", i + 1, base_name);
        println!("========================================");
        
        let img_start = std::time::Instant::now();
        let mut path = PathBuf::from(base_name);
        // ... (path resolution logic)
        if !path.exists() {
             // 尝试带前缀的路径
             let alt = PathBuf::from("src-tauri").join(base_name);
             if alt.exists() {
                 path = alt;
             }
        }

        if !path.exists() {
            println!("测试图片不存在: {:?}\n", path);
            continue;
        }

        // 提取测试图片的特征
        let (scene_desc, scene_kp_count, scene_size) = match extract_features_orb_with_details(path.to_str().unwrap()) {
            Ok(res) => res,
            Err(e) => {
                println!("提取特征失败: {}\n", e);
                continue;
            }
        };
        
        println!("场景信息: 尺寸 {:?}, 特征点数量: {}", scene_size, scene_kp_count);
        
        if scene_desc.empty() {
            println!("未检测到特征点\n");
            continue;
        }
        
        // 对所有模板进行匹配
        let mut all_scores: Vec<(String, usize, f32, usize, (u32, u32))> = Vec::new();
        
        for template in &cache {
            if template.descriptors.is_empty() {
                continue;
            }
            
            // 重建模板描述符
            use opencv::core::CV_8U;
            let rows = template.descriptor_rows;
            let cols = template.descriptor_cols;
            
            let mut template_desc = match unsafe { Mat::new_rows_cols(rows, cols, CV_8U) } {
                Ok(mat) => mat,
                Err(_) => continue,
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
            
            // 匹配
            if let Ok(matches) = match_orb_descriptors(&scene_desc, &template_desc) {
                let scene_kp = scene_kp_count as f32;
                let template_kp = rows as f32;
                // 使用更合理的置信度计算：匹配数 / min(场景特征点, 模板特征点)
                // 注意：ORB默认提取500个点，如果图片小可能提不到500个
                let min_kp = scene_kp.min(template_kp);
                let confidence = if min_kp > 0.0 {
                    matches as f32 / min_kp * 100.0
                } else {
                    0.0
                };
                
                all_scores.push((template.name.clone(), matches, confidence, rows as usize, (template.sample_w, template.sample_h)));
            }
        }
        
        // 按匹配数排序
        all_scores.sort_by(|a, b| b.1.cmp(&a.1));
        
        let elapsed = img_start.elapsed();
        println!("处理耗时: {:.2}ms", elapsed.as_secs_f64() * 1000.0);
        
        // 输出 Top10
        for (rank, (name, matches, confidence, template_kp, template_size)) in all_scores.iter().take(10).enumerate() {
            println!("Top {:2}: {} - 匹配: {}, (场景KP:{}, 模板KP:{}), 置信度: {:.2}%, 模板尺寸: {:?}", 
                rank + 1, name, matches, scene_kp_count, template_kp, confidence, template_size);
        }
        
        println!("\n");
    }
    
    println!("总耗时: {:.2}ms", total_start.elapsed().as_secs_f64() * 1000.0);
}

fn extract_features_orb_with_details(image_path: &str) -> Result<(Mat, i32, (i32, i32)), String> {
    use opencv::{imgcodecs, imgproc, features2d::ORB, core::AlgorithmHint};
    
    // Read file content first to handle non-ASCII paths on Windows
    let content = std::fs::read(image_path)
        .map_err(|e| format!("Failed to read file {}: {}", image_path, e))?;
    
    let img = imgcodecs::imdecode(&Mat::from_slice(&content).unwrap(), imgcodecs::IMREAD_COLOR)
        .map_err(|e| format!("Failed to decode image {}: {}", image_path, e))?;
        
    let w = img.cols();
    let h = img.rows();
    
    if img.empty() {
        return Err(format!("Decoded image is empty: {}", image_path));
    }
    
    let mut gray = Mat::default();
    imgproc::cvt_color(&img, &mut gray, imgproc::COLOR_BGR2GRAY, 0, AlgorithmHint::ALGO_HINT_DEFAULT)
        .map_err(|e| format!("Failed to convert to gray: {}", e))?;
    
    // 增加特征点数量限制到 1000 看看能否提升
    let mut orb = ORB::create(1000, 1.2f32, 8, 31, 0, 2, 
        opencv::features2d::ORB_ScoreType::HARRIS_SCORE, 31, 20)
        .map_err(|e| format!("Failed to create ORB: {}", e))?;
    
    let mut keypoints = opencv::core::Vector::new();
    let mut descriptors = Mat::default();
    
    orb.detect_and_compute(&gray, &Mat::default(), &mut keypoints, &mut descriptors, false)
        .map_err(|e| format!("Failed to detect: {}", e))?;
    
    Ok((descriptors, keypoints.len() as i32, (w, h)))
}

fn scan_dir_and_extract(dir: &PathBuf, templates: &mut Vec<TemplateCache>) {
    for entry in std::fs::read_dir(dir).expect("读取目录失败") {
        if let Ok(entry) = entry {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("webp") {
                let filename = path.file_name().unwrap().to_str().unwrap().to_string();
                let name = filename.replace(".webp", "");
                
                if let Ok(desc) = extract_features_orb(path.to_str().unwrap()) {
                    let rows = desc.rows();
                    let cols = desc.cols();
                    
                    // 将 Mat 转换为 Vec<u8>
                    let mut desc_vec = vec![0u8; (rows * cols) as usize];
                    unsafe {
                        std::ptr::copy_nonoverlapping(
                            desc.data() as *const u8, 
                            desc_vec.as_mut_ptr(), 
                            desc_vec.len()
                        );
                    }
                    
                    templates.push(TemplateCache {
                        name,
                        day: "Unknown".to_string(),
                        keypoints: Vec::new(),
                        descriptors: desc_vec,
                        descriptor_rows: rows,
                        descriptor_cols: cols,
                        sample_png: Vec::new(),
                        sample_w: 0,
                        sample_h: 0,
                    });
                }
            }
        }
    }
}

