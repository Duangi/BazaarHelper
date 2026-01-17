use std::collections::HashMap;
use std::time::Instant;
use opencv::{
    core::{Mat, Vector, KeyPoint, DMatch, NORM_HAMMING},
    features2d::{ORB, BFMatcher},
    imgcodecs::{imread, IMREAD_GRAYSCALE},
    prelude::*,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct MonsterEntry {
    image: Option<String>,
    #[allow(dead_code)]
    available: Option<String>,
}

struct MatchResult {
    name: String,
    confidence: f32,
    matches: usize,
}

// 使用 OpenCV ORB 提取特征点和描述符
fn extract_features_orb(image_path: &str) -> Result<Mat, opencv::Error> {
    let img = imread(image_path, IMREAD_GRAYSCALE)?;
    
    if img.empty() {
        return Ok(Mat::default());
    }

    let mut orb = ORB::create_def()?;
    let mut keypoints = Vector::<KeyPoint>::new();
    let mut descriptors = Mat::default();
    let mask = Mat::default();

    orb.detect_and_compute(&img, &mask, &mut keypoints, &mut descriptors, false)?;

    Ok(descriptors)
}

// ORB 匹配函数 - 使用 Lowe's Ratio Test
fn match_orb_descriptors(desc1: &Mat, desc2: &Mat) -> Result<usize, opencv::Error> {
    if desc1.empty() || desc2.empty() {
        return Ok(0);
    }

    let matcher = BFMatcher::create(NORM_HAMMING, false)?;
    let mut matches = Vector::<Vector::<DMatch>>::new();
    
    // 使用 knn_train_match: query, train, output, k, mask, compactResult
    matcher.knn_train_match(desc1, desc2, &mut matches, 2, &Mat::default(), false)?;

    let mut good_matches = 0;
    for m in matches.iter() {
        if m.len() == 2 {
            let m0 = m.get(0)?;
            let m1 = m.get(1)?;
            // Lowe's Ratio Test: 好的匹配应该显著优于第二好的匹配
            if m0.distance < 0.75 * m1.distance {
                good_matches += 1;
            }
        } else if m.len() == 1 {
            // 如果只有一个匹配，且距离较小，也认为是好匹配
            let m0 = m.get(0)?;
            if m0.distance < 50.0 {
                good_matches += 1;
            }
        }
    }

    Ok(good_matches)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== OpenCV ORB 图像识别性能测试 ===\n");

    // 测试图片路径
    let test_images = vec![
        ("Left", "D:\\Projects\\BazaarHelper\\src-tauri\\target\\debug\\examples\\final_left.jpg"),
        ("Mid", "D:\\Projects\\BazaarHelper\\src-tauri\\target\\debug\\examples\\final_mid.jpg"),
        ("Right", "D:\\Projects\\BazaarHelper\\src-tauri\\target\\debug\\examples\\final_right.jpg"),
    ];

    // 读取怪物数据库
    let resources_dir = "D:\\Projects\\BazaarHelper\\src-tauri\\resources";
    let db_path = format!("{}\\monsters_db.json", resources_dir);
    let json_content = std::fs::read_to_string(&db_path)?;
    let monsters: HashMap<String, MonsterEntry> = serde_json::from_str(&json_content)?;

    // 收集所有模板图片路径
    let mut template_paths = Vec::new();
    for (name, entry) in monsters.iter() {
        if let Some(rel_path) = &entry.image {
            let full_path = format!("{}\\{}", resources_dir, rel_path);
            if std::path::Path::new(&full_path).exists() {
                template_paths.push((name.clone(), full_path));
            }
        }
    }

    println!("加载了 {} 个怪物模板\n", template_paths.len());

    // 预加载所有模板特征
    println!("正在提取所有模板特征...");
    let template_start = Instant::now();
    let mut template_features = Vec::new();
    
    for (name, path) in &template_paths {
        match extract_features_orb(path) {
            Ok(desc) if !desc.empty() => {
                template_features.push((name.clone(), desc));
            }
            _ => {
                println!("警告: 无法提取 {} 的特征", name);
            }
        }
    }
    
    let template_time = template_start.elapsed();
    println!("✓ 模板特征提取完成，耗时: {:?} ({} 个模板)\n", template_time, template_features.len());

    // 测试每张图片
    let total_start = Instant::now();
    let mut individual_times = Vec::new();

    for (label, test_path) in &test_images {
        println!("==================================================");
        println!("测试图片: {} ({})", label, test_path);
        println!("==================================================");

        let image_start = Instant::now();

        // 提取测试图片特征
        let test_desc = match extract_features_orb(test_path) {
            Ok(desc) => desc,
            Err(e) => {
                println!("错误: 无法读取测试图片 - {}\n", e);
                continue;
            }
        };

        if test_desc.empty() {
            println!("警告: 测试图片未检测到特征点\n");
            continue;
        }

        println!("✓ 提取到 {} 个特征点", test_desc.rows());

        // 与所有模板进行匹配
        let mut results = Vec::new();
        let mut total_matches_count = 0;
        
        for (name, template_desc) in &template_features {
            match match_orb_descriptors(&test_desc, template_desc) {
                Ok(matches) => {
                    if matches > 0 {
                        total_matches_count += 1;
                    }
                    let scene_kp = test_desc.rows() as f32;
                    let template_kp = template_desc.rows() as f32;
                    let min_kp = scene_kp.min(template_kp);
                    let confidence = if min_kp > 0.0 {
                        matches as f32 / min_kp
                    } else {
                        0.0
                    };
                    
                    results.push(MatchResult {
                        name: name.clone(),
                        confidence,
                        matches,
                    });
                }
                Err(e) => {
                    println!("警告: 匹配 {} 失败 - {}", name, e);
                }
            }
        }

        println!("✓ 完成匹配，有 {} 个模板产生了匹配点", total_matches_count);

        // 排序并显示 Top 10
        results.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        
        println!("\n📊 Top 10 匹配结果:");
        println!("{:<5} {:<30} {:<12} {:<10}", "排名", "怪物名称", "匹配点数", "置信度");
        println!("{}", "-".repeat(65));
        
        for (i, result) in results.iter().take(10).enumerate() {
            println!("{:<5} {:<30} {:<12} {:.2}%", 
                i + 1, 
                result.name, 
                result.matches, 
                result.confidence * 100.0
            );
        }

        let image_time = image_start.elapsed();
        individual_times.push((label, image_time));
        println!("\n⏱️  本张图片识别耗时: {:?}", image_time);
        println!();
    }

    let total_time = total_start.elapsed();

    // 统计信息
    println!("==================================================");
    println!("📈 总体统计");
    println!("==================================================");
    println!("模板特征提取: {:?}", template_time);
    println!("识别总耗时: {:?}", total_time);
    println!("平均每张图片: {:?}", total_time / test_images.len() as u32);
    println!("\n各图片详细耗时:");
    for (label, time) in individual_times {
        println!("  {} : {:?}", label, time);
    }
    println!("\n总测试时间 (含模板加载): {:?}", template_start.elapsed());

    Ok(())
}
