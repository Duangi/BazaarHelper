# GitHub Actions 自动打包说明

## 功能
当你推送带有 `v` 开头的 tag 时（如 `v1.4.1`），GitHub Actions 会自动：
1. 在 macOS 和 Windows 环境下构建应用
2. 打包生成 DMG（macOS）和 NSIS 安装包（Windows）
3. 自动创建 GitHub Release 并上传安装包

## 使用方法

### 1. 推送代码并创建 tag
```bash
# 提交你的代码更改
git add .
git commit -m "Release v1.4.2"

# 创建 tag
git tag v1.4.2

# 推送代码和 tag
git push origin main
git push origin v1.4.2
```

### 2. 查看构建进度
访问你的 GitHub 仓库 → Actions 标签页，可以看到构建进度。

### 3. 下载安装包
构建完成后，访问 Releases 页面即可下载生成的安装包。

## 依赖说明

### macOS 构建依赖（GitHub Actions 自动安装）
- OpenCV (通过 Homebrew)
- ONNX Runtime (通过 Homebrew)
- ONNX, Protobuf, RE2 (通过 Homebrew)

这些依赖会在 GitHub Actions 运行时自动安装，**用户下载 DMG 后无需额外安装任何库**，因为所有依赖都已打包进 .app 文件中。

### Windows 构建依赖（GitHub Actions 自动安装）
- OpenCV (通过 vcpkg)
- ONNX Runtime (静态链接)

Windows 安装包同样包含所有必要的 DLL 文件。

## 本地构建说明

如果你想在本地构建：

### macOS
```bash
# 安装依赖
brew install opencv onnxruntime onnx protobuf re2

# 设置环境变量
export LIBCLANG_PATH=/Library/Developer/CommandLineTools/usr/lib
export DYLD_FALLBACK_LIBRARY_PATH=/Library/Developer/CommandLineTools/usr/lib
export PKG_CONFIG_PATH=/opt/homebrew/opt/opencv/lib/pkgconfig
export ORT_DYLIB_PATH=/opt/homebrew/lib/libonnxruntime.dylib

# 构建
npm run tauri build

# 打包 ONNX Runtime 库
./src-tauri/scripts/bundle_onnxruntime_macos.sh
```

### Windows
参考原有的构建流程，确保 OpenCV 和 ONNX Runtime 已正确配置。

## 注意事项

1. **Tag 命名**：必须以 `v` 开头（如 `v1.4.1`、`v2.0.0`）
2. **版本号同步**：记得同时更新 `src-tauri/tauri.conf.json` 中的版本号
3. **签名问题**：macOS 版本未签名，用户首次打开需要右键 → 打开，或运行 `xattr -cr BazaarHelper.app`
4. **构建时间**：完整构建大约需要 10-15 分钟

## 更新 update.json

发布新版本后，记得更新仓库根目录的 `update.json` 文件以支持自动更新功能。
