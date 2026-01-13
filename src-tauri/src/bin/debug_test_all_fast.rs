use image::DynamicImage;
use imageproc::corners::corners_fast9;
use std::path::PathBuf;
use std::time::Instant;
use std::fs;

struct Template {
    name: String,
    descriptors: Vec<([u8; 32], (u32, u32))>,
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
    // 提高阈值并限制点数，以提高匹配效率
    let corners = corners_fast9(&gray, 35);
    let mut features = Vec::new();
    for corner in corners {
        if let Some(desc) = compute_brief(&gray, corner.x, corner.y) {
            features.push((desc, (corner.x, corner.y)));
        }
        if features.len() > 300 { break; } // 到达上限即停止，保证速度
    }
    features
}

fn hamming_distance(a: &[u8; 32], b: &[u8; 32]) -> u32 {
    let mut dist = 0;
    for i in 0..32 {
        dist += (a[i] ^ b[i]).count_ones();
    }
    dist
}

fn main() {
    let base_dir = PathBuf::from(r"D:\Projects\BazaarHelper\src-tauri");
    let monster_img_dir = base_dir.join("resources").join("images_monster");
    let test_dir = base_dir.join("target").join("debug").join("monster_debug");
    
    println!("正在加载怪物库特征点 (FAST+BRIEF)...");
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
                            descriptors: extract_features(&img),
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

    println!("\n{:<15} | {:<25} | {:<10}", "测试文件", "匹配排名(Top 3)", "匹配特征点数");
    println!("{:-<70}", "");

    for (test_file, expected) in tests {
        let test_path = test_dir.join(test_file);
        if !test_path.exists() { continue; }

        let img_test = image::open(&test_path).unwrap();
        let scene_features = extract_features(&img_test);
        println!("   测试文件特征点提取完成: {} 个点", scene_features.len());
        
        let start_match = Instant::now();
        let mut scores = Vec::new();
        
        let total_library = library.len();
        for (idx, item) in library.iter().enumerate() {
            if idx % 20 == 0 { print!("({}/{}) ", idx, total_library); }
            let mut matches = 0;
            // 每一个场景特征点，在模板中找最近匹配
            for (sd, _) in &scene_features {
                for (td, _) in &item.descriptors {
                    // Hamming 距离 40 以下算初步匹配
                    if hamming_distance(sd, td) < 40 {
                        matches += 1;
                        break; 
                    }
                }
            }
            scores.push((item.name.clone(), matches));
        }
        println!("(100% DONE)");
        
        scores.sort_by_key(|&(_, m)| std::cmp::Reverse(m));
        let elapsed = start_match.elapsed();

        println!("{:<15} | 预期: {:<19} | (全库耗时: {:?})", test_file, expected, elapsed);
        for i in 0..5.min(scores.len()) {
            let (name, m) = &scores[i];
            let pointer = if name == expected { ">>" } else { "  " };
            println!("   {} #{}: {:<20} | 匹配数: {}", pointer, i + 1, name, m);
        }
        println!("{:-<70}", "");
    }
}
