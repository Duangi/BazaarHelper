fn main() {
    // 告诉Cargo在DirectML相关文件变化时重新构建
    println!("cargo:rerun-if-changed=../resources/models");
    println!("cargo:rerun-if-changed=resources/onnxruntime.dll");
    println!("cargo:rerun-if-changed=resources/DirectML.dll");

    // Windows平台：设置DLL搜索路径
    #[cfg(target_os = "windows")]
    {
        let profile = std::env::var("PROFILE").unwrap_or_default();
        
        // 添加resources目录到DLL搜索路径
        println!("cargo:rustc-link-search=native=resources");
        
        if profile == "release" {
            println!("cargo:warning=Building for release - ONNX Runtime DLLs should be in resources/ directory");
        }
    }
    
    tauri_build::build()
}
