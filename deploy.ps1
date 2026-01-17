# --- 配置区 ---
$NEW_VERSION = "1.2.0"  # <--- 版本号

# 【新增】这里写你想对用户说的话（支持换行 \n）
# 比如：这次更新了巨龟数据！\n如果不小心是花钱买的，快去退款买杯奶茶！
$ANNOUNCEMENT_TEXT = "🎉 v$NEW_VERSION 更新来了：\n 更新内容太多了。都去看最新视频！  \n"

$GITHUB_USER = "Duangi"
$REPO_NAME = "BazaarHelper"
$KEY_PATH = "src-tauri/tauri.key" # 密钥路径

# 设置错误处理
$ErrorActionPreference = "Stop"
# 强制使用无 BOM 的 UTF8
$Utf8NoBom = New-Object System.Text.UTF8Encoding $False

Write-Host "--- Start Build Workflow (Version: $NEW_VERSION) ---" -ForegroundColor Cyan

# 1. 更新 package.json
Write-Host "[1/5] Updating package.json..."
$pkg = Get-Content package.json -Raw | ConvertFrom-Json
$pkg.version = $NEW_VERSION
$pkg_json = $pkg | ConvertTo-Json -Depth 10
[System.IO.File]::WriteAllText("$(Get-Location)/package.json", $pkg_json, $Utf8NoBom)

# 2. 更新 tauri.conf.json
Write-Host "[2/5] Updating tauri.conf.json..."
$conf = Get-Content src-tauri/tauri.conf.json -Raw | ConvertFrom-Json
$conf.version = $NEW_VERSION
$conf_json = $conf | ConvertTo-Json -Depth 10
[System.IO.File]::WriteAllText("$(Get-Location)/src-tauri/tauri.conf.json", $conf_json, $Utf8NoBom)

# 3. 运行构建
Write-Host "[3/5] Running Tauri Build (Release)..." -ForegroundColor Yellow
# 清理旧包，防止混淆
if (Test-Path "src-tauri/target/release/bundle/nsis") {
    Remove-Item "src-tauri/target/release/bundle/nsis/*" -Recurse -Force
}

# 执行构建 (此时不强制签名，只生成exe)
npm run tauri build
if ($LASTEXITCODE -ne 0) { throw "Build failed" }

# 为方便测试，把 DLL 也复制到 release 目录下的 loose exe 旁边
$releaseDir = "src-tauri/target/release"
$dllSource = "C:\opencv\build\x64\vc16\bin\opencv_world4120.dll"
if (Test-Path $dllSource) {
    Copy-Item $dllSource -Destination "$releaseDir\opencv_world4120.dll" -Force
    Write-Host "✅ [Dev] Copied opencv DLL to target/release for testing." -ForegroundColor Cyan
}

# 4. 定位安装包
$bundlePath = "src-tauri/target/release/bundle/nsis"
$exeFile = Get-ChildItem -Path "$bundlePath/*.exe" | Where-Object { $_.Name -like "*setup.exe" } | Select-Object -First 1

if ($null -eq $exeFile) { throw "Installer file not found" }
Write-Host "Found installer: $($exeFile.Name)" -ForegroundColor Green

# 5. 【核心修复】手动执行签名 & 更新 update.json
Write-Host "[4/5] Signing manually..." -ForegroundColor Yellow

# 检查密钥是否需要密码
if (Test-Path $KEY_PATH) {
    $keyContent = Get-Content $KEY_PATH -Raw
    if ($keyContent -like "*encrypted secret key*") {
        # 如果还没设置密码变量，提示输入
        if (-not $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD) {
            $pwd = Read-Host "检测到加密私钥，请输入密码" -AsSecureString
            $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = [Runtime.InteropServices.Marshal]::PtrToStringAuto([Runtime.InteropServices.Marshal]::SecureStringToBSTR($pwd))
        }
    }
    
    # 暴力补签：直接调用 signer 工具
    # 这比依赖环境变量注入 build 过程稳一万倍
    npx tauri signer sign --private-key-path $KEY_PATH $exeFile.FullName
} else {
    throw "Error: 找不到私钥文件 $KEY_PATH"
}

# 检查签名是否生成
$sigFile = $exeFile.FullName + ".sig"
if (Test-Path $sigFile) {
    $signature = (Get-Content -Path $sigFile -Raw).Trim()
    Write-Host "✅ 签名生成成功" -ForegroundColor Green
} else {
    throw "Signature generation failed"
}

# 6. 生成最终的 update.json
Write-Host "[5/5] Generating update.json..."
$updateData = @{
    version = $NEW_VERSION
    notes = $ANNOUNCEMENT_TEXT
    pub_date = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
    platforms = @{
        "windows-x86_64" = @{
            signature = $signature
            # 如果你需要用 ghproxy 加速，可以在这里加前缀
            url = "https://gh.llkk.cc/https://github.com/$GITHUB_USER/$REPO_NAME/releases/download/v$NEW_VERSION/$($exeFile.Name)"
        }
    }
}

$update_json = $updateData | ConvertTo-Json -Depth 10
[System.IO.File]::WriteAllText("$(Get-Location)/update.json", $update_json, $Utf8NoBom)

Write-Host "----------------------------------------"
Write-Host "🎉 流程完成！" -ForegroundColor Green
Write-Host "1. 上传 $($exeFile.Name) 到 GitHub Release v$NEW_VERSION"
Write-Host "2. 提交 update.json 到 GitHub"
Write-Host "----------------------------------------"