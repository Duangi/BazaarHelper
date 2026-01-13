use image::{DynamicImage, GenericImageView, Luma};
use imageproc::corners::corners_fast9;
use std::path::PathBuf;

/// 简单的 BRIEF 描述子实现 (16个采样点对)
fn get_brief_descriptor(img: &image::ImageBuffer<Luma<u8>, Vec<u8>>, x: u32, y: u32) -> u32 {
    let (w, h) = img.dimensions();
    let mut desc: u32 = 0;
    // 使用固定的偏移点对进行比较 (简单的模式)
    let patterns = [
        (-2, -2, 2, 2), (-2, 2, 2, -2), (0, -3, 0, 3), (-3, 0, 3, 0),
        (-1, -1, 1, 1), (-1, 1, 1, -1), (0, -2, 0, 2), (-2, 0, 2, 0),
        (-3, -3, 3, 3), (-3, 3, 3, -3), (1, -2, 1, 2), (-2, 1, 2, 1),
        (-1, -3, 1, 3), (-3, -1, 3, 1), (2, -2, -2, 2), (0, -1, 0, 1)
    ];
    
    for (i, &(x1, y1, x2, y2)) in patterns.iter().enumerate() {
        let px1 = (x as i32 + x1).clamp(0, w as i32 - 1) as u32;
        let py1 = (y as i32 + y1).clamp(0, h as i32 - 1) as u32;
        let px2 = (x as i32 + x2).clamp(0, w as i32 - 1) as u32;
        let py2 = (y as i32 + y2).clamp(0, h as i32 - 1) as u32;
        
        if img.get_pixel(px1, py1)[0] > img.get_pixel(px2, py2)[0] {
            desc |= 1 << i;
        }
    }
    desc
}

fn count_matches(img1: &DynamicImage, img2: &DynamicImage) -> usize {
    let g1 = img1.to_luma8();
    let g2 = img2.to_luma8();
    
    // 降采样模板到插画的大致大小，提高匹配率
    let g2_resized = image::imageops::resize(&g2, img1.width(), img1.height(), image::imageops::FilterType::Triangle);

    let corners1 = corners_fast9(&g1, 20);
    let corners2 = corners_fast9(&g2_resized, 20);
    
    let mut desc1 = Vec::new();
    for c in corners1 {
        desc1.push(get_brief_descriptor(&g1, c.x, c.y));
    }
    
    let mut desc2 = Vec::new();
    for c in corners2 {
        desc2.push(get_brief_descriptor(&g2_resized, c.x, c.y));
    }
    
    let mut matches = 0;
    for &d1 in &desc1 {
        for &d2 in &desc2 {
            // 汉明距离 (位差异) 小于 2 则认为匹配
            if (d1 ^ d2).count_ones() <= 2 {
                matches += 1;
                break;
            }
        }
    }
    matches
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

    println!("{:<15} | {:<15} | {:<10}", "文件", "预期目标", "FAST匹配数");
    println!("{:-<50}", "");

    for (test_file, expected_template) in tests {
        let test_path = test_dir.join(test_file);
        let template_path = monster_img_dir.join(expected_template);
        if !test_path.exists() || !template_path.exists() { continue; }

        let img_test = image::open(&test_path).unwrap();
        let img_template = image::open(&template_path).unwrap();
        
        let (w, h) = img_test.dimensions();
        let cx = (w as f32 * 0.1) as u32;
        let cy = (h as f32 * 0.1) as u32;
        let cw = (w as f32 * 0.8) as u32;
        let ch = (h as f32 * 0.55) as u32;
        let cropped = img_test.crop_imm(cx, cy, cw, ch);

        let m1 = count_matches(&cropped, &img_template);
        println!("{:<15} | {:<15} | {:<10}", test_file, expected_template, m1);
        
        let other_templates = vec!["暴徒.jpg", "幽灵辣椒.jpg", "产品演示员.jpg", "街头玩家.jpg", "初学学徒.jpg"];
        for &other in &other_templates {
            if other == expected_template { continue; }
            let other_path = monster_img_dir.join(other);
            if let Ok(other_img) = image::open(&other_path) {
                let m2 = count_matches(&cropped, &other_img);
                println!("                | -> 目标: {:<10} | 匹配数: {}", other, m2);
            }
        }
        println!("{:-<50}", "");
    }
}
