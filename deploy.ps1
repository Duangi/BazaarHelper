# --- 配置区 ---
$NEW_VERSION = "0.5.0"
$KEY_PATH = "src-tauri/tauri.key"
$GITHUB_USER = "Duangi"
$REPO_NAME = "BazaarHelper"

$ErrorActionPreference = "Stop"
# 强制定义无 BOM 的 UTF8 编码
$Utf8NoBom = New-Object System.Text.UTF8Encoding $False

Write-Host "--- Start Automation Workflow (Version: $NEW_VERSION) ---"

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
Write-Host "[3/6] Running Tauri Build (Release)..."
# 尝试直接运行，看是否还报错
npm run tauri build
if ($LASTEXITCODE -ne 0) { throw "Build failed" }

# 4. 定位安装包
$bundlePath = "src-tauri/target/release/bundle/nsis"
$exeFile = Get-ChildItem -Path "$bundlePath/*.exe" | Where-Object { $_.Name -like "*setup.exe" } | Select-Object -First 1
if ($null -eq $exeFile) { throw "Installer file not found" }

# 5. 执行签名
Write-Host "[4/6] Signing..."
npx tauri signer sign --private-key-path $KEY_PATH $exeFile.FullName

# 6. 生成 update.json
$sigFile = $exeFile.FullName + ".sig"
$signature = (Get-Content -Path $sigFile -Raw).Trim()

$updateData = @{
    version = $NEW_VERSION
    notes = "Bazaar Helper v$NEW_VERSION release"
    pub_date = (Get-Date -Format "yyyy-MM-ddTHH:mm:ssZ")
    platforms = @{
        "windows-x86_64" = @{
            signature = $signature
            url = "https://github.com/$GITHUB_USER/$REPO_NAME/releases/download/v$NEW_VERSION/$($exeFile.Name)"
        }
    }
}
$update_json = $updateData | ConvertTo-Json -Depth 10
[System.IO.File]::WriteAllText("$(Get-Location)/update.json", $update_json, $Utf8NoBom)

Write-Host "--- DONE ---"