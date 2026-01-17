use opencv::{
    core::{Mat, Vector, KeyPoint, DMatch, NORM_HAMMING},
    features2d::{ORB, BFMatcher},
    imgcodecs::{imread, IMREAD_GRAYSCALE},
    prelude::*,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== OpenCV ORB 匹配调试 ===\n");

    // 测试两张图片
    let img1_path = "D:\\Projects\\BazaarHelper\\src-tauri\\target\\debug\\examples\\final_left.jpg";
    let img2_path = "D:\\Projects\\BazaarHelper\\src-tauri\\resources\\images_monster\\02233aaf-9549-5768-a51f-6a3302525c39.jpg";

    println!("读取图片 1: {}", img1_path);
    let img1 = imread(img1_path, IMREAD_GRAYSCALE)?;
    println!("图片 1 尺寸: {}x{}", img1.cols(), img1.rows());

    println!("读取图片 2: {}", img2_path);
    let img2 = imread(img2_path, IMREAD_GRAYSCALE)?;
    println!("图片 2 尺寸: {}x{}\n", img2.cols(), img2.rows());

    // 提取特征
    println!("提取图片 1 的 ORB 特征...");
    let mut orb1 = ORB::create_def()?;
    let mut kp1 = Vector::<KeyPoint>::new();
    let mut desc1 = Mat::default();
    orb1.detect_and_compute(&img1, &Mat::default(), &mut kp1, &mut desc1, false)?;
    println!("  特征点: {}", kp1.len());
    println!("  描述符: {}x{}, type: {}\n", desc1.rows(), desc1.cols(), desc1.typ());

    println!("提取图片 2 的 ORB 特征...");
    let mut orb2 = ORB::create_def()?;
    let mut kp2 = Vector::<KeyPoint>::new();
    let mut desc2 = Mat::default();
    orb2.detect_and_compute(&img2, &Mat::default(), &mut kp2, &mut desc2, false)?;
    println!("  特征点: {}", kp2.len());
    println!("  描述符: {}x{}, type: {}\n", desc2.rows(), desc2.cols(), desc2.typ());

    if desc1.empty() || desc2.empty() {
        println!("错误: 某个描述符为空！");
        return Ok(());
    }

    // 尝试匹配
    println!("创建 BFMatcher (NORM_HAMMING)...");
    let matcher = BFMatcher::create(NORM_HAMMING, false)?;

    // 先尝试普通匹配
    println!("\n=== 尝试普通匹配 (match) ===");
    let mut normal_matches = Vector::<DMatch>::new();
    matcher.train_match(&desc1, &desc2, &mut normal_matches, &Mat::default())?;
    println!("  找到 {} 个普通匹配", normal_matches.len());
    
    if normal_matches.len() > 0 {
        println!("  前 5 个匹配:");
        for i in 0..normal_matches.len().min(5) {
            let m = normal_matches.get(i)?;
            println!("    Match {}: query_idx={}, train_idx={}, distance={:.2}", 
                i, m.query_idx, m.train_idx, m.distance);
        }
    }

    println!("\n=== 尝试 KNN 匹配 (k=2) ===");
    let mut knn_matches = Vector::<Vector::<DMatch>>::new();
    
    // 测试不同的参数顺序
    println!("尝试匹配...");
    match matcher.knn_train_match(&desc1, &desc2, &mut knn_matches, 2, &Mat::default(), false) {
        Ok(_) => {
            println!("  匹配成功！找到 {} 组匹配", knn_matches.len());
            
            let mut good_matches = 0;
            let mut single_matches = 0;
            let mut total_matches = 0;
            
            for (i, m) in knn_matches.iter().enumerate() {
                total_matches += 1;
                if i < 5 {
                    print!("  Match {}: len={}", i, m.len());
                    if m.len() > 0 {
                        let m0 = m.get(0)?;
                        print!(", dist[0]={:.2}", m0.distance);
                    }
                    if m.len() > 1 {
                        let m1 = m.get(1)?;
                        print!(", dist[1]={:.2}", m1.distance);
                    }
                    println!();
                }
                
                if m.len() == 2 {
                    let m0 = m.get(0)?;
                    let m1 = m.get(1)?;
                    if m0.distance < 0.75 * m1.distance {
                        good_matches += 1;
                    }
                } else if m.len() == 1 {
                    single_matches += 1;
                    let m0 = m.get(0)?;
                    if m0.distance < 50.0 {
                        good_matches += 1;
                    }
                }
            }
            
            println!("\n统计:");
            println!("  总匹配组: {}", total_matches);
            println!("  双匹配: {}", total_matches - single_matches);
            println!("  单匹配: {}", single_matches);
            println!("  好匹配 (Lowe's ratio): {}", good_matches);
        }
        Err(e) => {
            println!("  匹配失败: {}", e);
        }
    }

    Ok(())
}
