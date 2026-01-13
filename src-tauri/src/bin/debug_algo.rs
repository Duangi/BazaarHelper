use image::{DynamicImage, imageops::FilterType};
use std::path::PathBuf;

fn calculate_rmse(img1: &DynamicImage, img2: &DynamicImage) -> u64 {
    let small1 = img1.resize_exact(32, 32, FilterType::Triangle).to_rgb8();
    let small2 = img2.resize_exact(32, 32, FilterType::Triangle).to_rgb8();
    
    let pixels1 = small1.as_raw();
    let pixels2 = small2.as_raw();
    
    let mut diff: f64 = 0.0;
    let mut total_weight: f64 = 0.0;
    
    for y in 0..32 {
        for x in 0..32 {
            let idx = (y * 32 + x) * 3;
            let c1 = &pixels1[idx..idx+3];
            let c2 = &pixels2[idx..idx+3];
            
            // 权重：中心 (16, 12) 附近最高，向四周递减
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
    
    let tests = vec![
        ("slot_1.png", "初学学徒.jpg"),
        ("slot_2.png", "街头玩家.jpg"),
        ("slot_3.png", "变异烘焙师.jpg"),
    ];

    println!("{:<15} | {:<15} | {:<10} | {:<10}", "文件", "预期目标", "L1差异值", "置信度估算");
    println!("{:-<60}", "");

    for (test_file, expected_template) in tests {
        let test_path = test_dir.join(test_file);
        let template_path = monster_img_dir.join(expected_template);
        if !test_path.exists() || !template_path.exists() { continue; }

        let img_test = image::open(&test_path).unwrap();
        let img_template = image::open(&template_path).unwrap();
        
        // 取消二次裁剪，直接对比原始槽位图
        let cropped = img_test;
        
        // 保存对比图
        let debug_save_path = test_dir.join(format!("no_crop_{}", test_file));
        cropped.save(&debug_save_path).unwrap();

        let diff = calculate_rmse(&cropped, &img_template);
        // RMSE 通常在 0-255 之间，设定一个合理的置信度映射 (100以内认为是比较接近的)
        let conf = (1.0 - (diff as f32 / 120.0)).max(0.0);
        
        println!("{:<15} | {:<15} | {:<10} | {:<10.4}", test_file, expected_template, diff, conf);
        
        // 交叉比对
        let other_templates = vec!["暴徒.jpg", "幽灵辣椒.jpg", "产品演示员.jpg"];
        for other in other_templates {
            if other == expected_template { continue; }
            let other_path = monster_img_dir.join(other);
            if let Ok(other_img) = image::open(&other_path) {
                let d2 = calculate_rmse(&cropped, &other_img);
                println!("                | -> 错误目标: {:<10} | 差异: {:<8}", other, d2);
            }
        }
        println!("{:-<60}", "");
    }
}