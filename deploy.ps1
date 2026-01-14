# --- 配置区 ---
$NEW_VERSION = "0.8.1"  # <--- 以后只改这里
$GITHUB_USER = "Duangi"
$REPO_NAME = "BazaarHelper"

$ErrorActionPreference = "Stop"
$Utf8NoBom = New-Object System.Text.UTF8Encoding $False

Write-Host "--- Start Build Workflow (Version: $NEW_VERSION) ---" -ForegroundColor Cyan

# 1. 更新 package.json
Write-Host "[1/3] Updating package.json..."
$pkg = Get-Content package.json -Raw | ConvertFrom-Json
$pkg.version = $NEW_VERSION
$pkg_json = $pkg | ConvertTo-Json -Depth 10
[System.IO.File]::WriteAllText("$(Get-Location)/package.json", $pkg_json, $Utf8NoBom)

# 2. 更新 tauri.conf.json
Write-Host "[2/3] Updating tauri.conf.json..."
$conf = Get-Content src-tauri/tauri.conf.json -Raw | ConvertFrom-Json
$conf.version = $NEW_VERSION
$conf_json = $conf | ConvertTo-Json -Depth 10
[System.IO.File]::WriteAllText("$(Get-Location)/src-tauri/tauri.conf.json", $conf_json, $Utf8NoBom)

# 3. 运行构建
Write-Host "[3/3] Running Tauri Build (Release)..." -ForegroundColor Yellow
# 清理旧包
if (Test-Path "src-tauri/target/release/bundle/nsis") {
    Remove-Item "src-tauri/target/release/bundle/nsis/*" -Recurse -Force
}

# 这里的参数 --no-bundle 是可选的。
# 如果你还需要 setup.exe，就不要加 --no-bundle。
# 如果你只需要那个绿色的 tauri-app.exe，可以加。
npm run tauri build

if ($LASTEXITCODE -ne 0) { throw "Build failed" }

# 4. 找到文件告诉你
$bundlePath = "src-tauri/target/release/bundle/nsis"
$exeFile = Get-ChildItem -Path "$bundlePath/*.exe" | Where-Object { $_.Name -like "*setup.exe" } | Select-Object -First 1

Write-Host "----------------------------------------"
Write-Host "🎉 构建完成！" -ForegroundColor Green
Write-Host "安装包位置: $($exeFile.FullName)"
Write-Host "现在你可以把它上传到百度网盘和 GitHub 了。"
Write-Host "----------------------------------------"