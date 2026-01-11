# --- 配置区 ---
$NEW_VERSION = "0.5.4"  # <--- 下次发版改这里
$KEY_PATH = "src-tauri/tauri.key"
$GITHUB_USER = "Duangi"
$REPO_NAME = "BazaarHelper"

# 设置错误处理
$ErrorActionPreference = "Stop"
# 强制使用无 BOM 的 UTF8
$Utf8NoBom = New-Object System.Text.UTF8Encoding $False

Write-Host "--- Start Automation Workflow (Version: $NEW_VERSION) ---" -ForegroundColor Cyan

# 1. 更新 package.json
Write-Host "[1/6] Updating package.json..."
$pkg = Get-Content package.json -Raw | ConvertFrom-Json
$pkg.version = $NEW_VERSION
$pkg_json = $pkg | ConvertTo-Json -Depth 10
[System.IO.File]::WriteAllText("$(Get-Location)/package.json", $pkg_json, $Utf8NoBom)

# 2. 更新 tauri.conf.json
Write-Host "[2/6] Updating tauri.conf.json..."
$conf = Get-Content src-tauri/tauri.conf.json -Raw | ConvertFrom-Json
$conf.version = $NEW_VERSION
$conf_json = $conf | ConvertTo-Json -Depth 10
[System.IO.File]::WriteAllText("$(Get-Location)/src-tauri/tauri.conf.json", $conf_json, $Utf8NoBom)

# 3. 运行构建
Write-Host "[3/6] Running Tauri Build (Release)..." -ForegroundColor Yellow
# 先清理旧文件，防止找到上一个版本的包
if (Test-Path "src-tauri/target/release/bundle/nsis") {
    Remove-Item "src-tauri/target/release/bundle/nsis/*" -Recurse -Force
}
npm run tauri build
if ($LASTEXITCODE -ne 0) { throw "Build failed" }

# 4. 智能定位安装包 (核心修复)
$bundlePath = "src-tauri/target/release/bundle/nsis"
# 找最新生成的那个 setup.exe
$exeFile = Get-ChildItem -Path "$bundlePath/*.exe" | Where-Object { $_.Name -like "*setup.exe" } | Select-Object -First 1

if ($null -eq $exeFile) { throw "Installer file not found" }
Write-Host "Found installer: $($exeFile.Name)" -ForegroundColor Green

# 5. 执行签名
Write-Host "[4/6] Signing..."
# 如果私钥有密码，取消注释下行
# $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = "123456"
npx tauri signer sign --private-key-path $KEY_PATH $exeFile.FullName

# 6. 生成 update.json (自动匹配文件名)
$sigFile = $exeFile.FullName + ".sig"
if (!(Test-Path $sigFile)) { throw "Signature generation failed" }

$signature = (Get-Content -Path $sigFile -Raw).Trim()

$updateData = @{
    version = $NEW_VERSION
    notes = "Bazaar Helper v$NEW_VERSION Auto-Release"
    pub_date = (Get-Date -Format "yyyy-MM-ddTHH:mm:ssZ")
    platforms = @{
        "windows-x86_64" = @{
            signature = $signature
            # 使用 gh.ddlc.top 镜像加速（国内友好）
            url = "https://gh.ddlc.top/https://github.com/$GITHUB_USER/$REPO_NAME/releases/download/v$NEW_VERSION/$($exeFile.Name)"
        }
    }
}

$update_json = $updateData | ConvertTo-Json -Depth 10
[System.IO.File]::WriteAllText("$(Get-Location)/update.json", $update_json, $Utf8NoBom)

Write-Host "--- DONE ---" -ForegroundColor Cyan
Write-Host "请确保："
Write-Host "1. 在 GitHub 创建 Release，Tag 必须填: v$NEW_VERSION"
Write-Host "2. 上传的文件必须是: $($exeFile.Name)"