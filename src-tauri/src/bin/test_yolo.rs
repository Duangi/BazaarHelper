use image::{imageops::FilterType, GenericImageView};
use ndarray::Array;
use ort::session::{builder::GraphOptimizationLevel, Session};
use std::time::Instant;
use opencv::{
    core::{Point, Scalar, Rect},
    highgui,
    imgcodecs,
    imgproc,
    prelude::*,
};

#[derive(Debug, Clone)]
struct Detection {
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    confidence: f32,
    class_id: usize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model_path = "D:\\Projects\\BazaarHelper\\src-tauri\\resources\\models\\best.onnx";
    let image_paths = [
        "D:\\Projects\\BazaarHelper\\src-tauri\\target\\debug\\examples\\monster.png",
        "D:\\Projects\\BazaarHelper\\src-tauri\\target\\debug\\examples\\test.png",
    ];

    println!("正在加载模型: {} ...", model_path);
    let start_load = Instant::now();
    
    // 初始化 ort session
    let mut session = Session::builder()?
        .with_optimization_level(GraphOptimizationLevel::Level3)?
        .with_intra_threads(4)?
        .commit_from_file(model_path)?;
    
    println!("📥 模型加载耗时: {} ms", start_load.elapsed().as_millis());

    for image_path in &image_paths {
        if !std::path::Path::new(image_path).exists() {
            println!("❌ 找不到测试图片: {}", image_path);
            continue;
        }

        println!("\n正在处理图片: {}", image_path);
        let img = image::open(image_path)?;
        let (orig_w, orig_h) = img.dimensions();

        let start_inference = Instant::now();
        
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
        let input_shape = vec![1, 3, 640, 640];
        let input_tensor = ort::value::Value::from_array((input_shape, input_array.into_raw_vec()))?;
        let outputs = session.run(vec![("images", input_tensor)])?;
        let output_tensor = &outputs["output0"];
        
        // 在 2.0.0-rc.9 中，try_extract_tensor 返回的是 ( &Shape, &[f32] )
        let (output_shape, output_data) = output_tensor.try_extract_tensor::<f32>()?;
        let num_elements = output_shape[1] as usize;
        let num_anchors = output_shape[2] as usize;
        
        println!("⚡ 推理耗时: {} ms", start_inference.elapsed().as_millis());

        // 3. 后处理
        let mut candidates = Vec::new();
        let conf_threshold = 0.25;

        for i in 0..num_anchors {
            let xc = output_data[0 * num_anchors + i];
            let yc = output_data[1 * num_anchors + i];
            let w = output_data[2 * num_anchors + i];
            let h = output_data[3 * num_anchors + i];

            let mut max_score = 0.0;
            let mut class_id = 0;
            for c in 4..num_elements {
                let score = output_data[c * num_anchors + i];
                if score > max_score {
                    max_score = score;
                    class_id = c - 4;
                }
            }

            if max_score > conf_threshold {
                let x1 = (xc - w / 2.0) * (orig_w as f32 / 640.0);
                let y1 = (yc - h / 2.0) * (orig_h as f32 / 640.0);
                let x2 = (xc + w / 2.0) * (orig_w as f32 / 640.0);
                let y2 = (yc + h / 2.0) * (orig_h as f32 / 640.0);

                candidates.push(Detection {
                    x1: x1 as i32,
                    y1: y1 as i32,
                    x2: x2 as i32,
                    y2: y2 as i32,
                    confidence: max_score,
                    class_id,
                });
            }
        }

        // NMS
        let detections = nms(candidates, 0.45);

        println!("检测到 {} 个目标:", detections.len());
        
        let mut mat = imgcodecs::imread(image_path, imgcodecs::IMREAD_COLOR)?;
        
        for det in &detections {
            println!("  - [ID: {}] 置信度: {:.2} | 坐标: ({}, {}) - ({}, {})", 
                det.class_id, det.confidence, det.x1, det.y1, det.x2, det.y2);

            let rect = Rect::new(det.x1, det.y1, det.x2 - det.x1, det.y2 - det.y1);
            imgproc::rectangle(&mut mat, rect, Scalar::new(0.0, 255.0, 0.0, 0.0), 2, imgproc::LINE_8, 0)?;
            
            let label = format!("ID: {} {:.2}", det.class_id, det.confidence);
            imgproc::put_text(&mut mat, &label, Point::new(det.x1, det.y1 - 5), 
                imgproc::FONT_HERSHEY_SIMPLEX, 0.5, Scalar::new(0.0, 255.0, 0.0, 0.0), 1, imgproc::LINE_AA, false)?;
        }

        highgui::imshow("YOLO Rust Result", &mat)?;
        println!("按任意键继续处理下一张图片/退出...");
        highgui::wait_key(0)?;
    }

    Ok(())
}

fn nms(mut detections: Vec<Detection>, iou_threshold: f32) -> Vec<Detection> {
    detections.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
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

fn calculate_iou(a: &Detection, b: &Detection) -> f32 {
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
