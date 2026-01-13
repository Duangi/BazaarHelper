use image::{DynamicImage, imageops::FilterType, GenericImageView};
use std::path::PathBuf;
use std::time::Instant;
use std::fs;

struct Template {
    name: String,
    thumb: Vec<u8>,
}

fn extract_thumb(img: &DynamicImage) -> Vec<u8> {
    // 模拟生产环境的裁剪 (10%, 10%, 80%, 55%)
    let (w, h) = img.dimensions();
    let cx = (w as f32 * 0.10) as u32;
    let cy = (h as f32 * 0.10) as u32;
    let cw = (w as f32 * 0.80) as u32;
    let ch = (h as f32 * 0.55) as u32;
    let cropped = img.crop_imm(cx, cy, cw, ch);
    cropped.resize_exact(32, 32, FilterType::Triangle).to_rgb8().into_raw()
}

fn calculate_weighted_rmse(data1: &[u8], data2: &[u8]) -> u64 {
    let mut diff: f64 = 0.0;
    let mut total_weight: f64 = 0.0;
    
    for y in 0..32 {
        for x in 0..32 {
            let idx = (y * 32 + x) * 3;
            let c1 = &data1[idx..idx+3];
            let c2 = &data2[idx..idx+3];
            
            let dx = (x as i32 - 16) as f64 / 16.0;
            let dy = (y as i32 - 12) as f64 / 12.0;
            let dist_sq = dx*dx + dy*dy;
            let weight = (1.2 - dist_sq).max(0.1); 
            
            let r_d = c1[0] as i32 - c2[0] as i32;
            let g_d = c1[1] as i32 - c2[1] as i32;
            let b_d = c1[2] as i32 - c2[2] as i32;
            
            diff += (r_d*r_d + g_d*g_d + b_d*b_d) as f64 * weight;
            total_weight += weight;
        }
    }
    
    (diff / total_weight).sqrt() as u64
}

fn main() {
    let base_dir = PathBuf::from(r"D:\Projects\BazaarHelper\src-tauri");
    let monster_img_dir = base_dir.join("resources").join("images_monster");
    let test_dir = base_dir.join("target").join("debug").join("monster_debug");
    
    // 1. 加载库中所有图片
    println!("正在加载怪物库模板...");
    let start_load = Instant::now();
    let mut library = Vec::new();
    if let Ok(entries) = fs::read_dir(&monster_img_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
                if ext == "jpg" || ext == "png" {
                    if let Ok(img) = image::open(&path) {
                        library.push(Template {
                            name: path.file_name().unwrap().to_string_lossy().to_string(),
                            thumb: extract_thumb(&img),
                        });
                    }
                }
            }
        }
    }
    println!("加载完成，共 {} 个模板，耗时: {:?}", library.len(), start_load.elapsed());

    let tests = vec![
        ("slot_1.png", "初学学徒.jpg"),
        ("slot_2.png", "街头玩家.jpg"),
        ("slot_3.png", "变异烘焙师.jpg"),
    ];

    println!("\n{:<15} | {:<25} | {:<10}", "测试文件", "匹配目标(Top 3)", "RMSE 差异");
    println!("{:-<60}", "");

    for (test_file, expected) in tests {
        let test_path = test_dir.join(test_file);
        if !test_path.exists() { continue; }

        let img_test = image::open(&test_path).unwrap();
        let test_thumb = extract_thumb(&img_test);
        
        let start_match = Instant::now();
        let mut scores = Vec::new();
        for item in &library {
            let rmse = calculate_weighted_rmse(&test_thumb, &item.thumb);
            scores.push((item.name.clone(), rmse));
        }
        
        // 排序
        scores.sort_by_key(|&(_, rmse)| rmse);
        let elapsed = start_match.elapsed();

        println!("{:<15} | 预期: {:<19} | (全库比对耗时: {:?})", test_file, expected, elapsed);
        
        // 查找预期目标的排名
        if let Some(pos) = scores.iter().position(|(name, _)| name == expected) {
            let (name, rmse) = &scores[pos];
            println!("   [!] 预期目标 '{}' 排名在第 {} 位，RMSE: {}", name, pos + 1, rmse);
        } else {
            println!("   [X] 错误: 在库中未找到预期目标 '{}'", expected);
        }

        println!("   --- Top 5 匹配结果 ---");
        for i in 0..5.min(scores.len()) {
            let (name, rmse) = &scores[i];
            println!("      #{}: {:<20} | RMSE: {}", i + 1, name, rmse);
        }
        println!("{:-<60}", "");
    }
}
