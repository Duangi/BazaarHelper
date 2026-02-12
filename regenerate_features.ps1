# Regenerate Feature Files Script
# This script deletes cache files so the app will regenerate them on next startup

Write-Host "Starting feature file regeneration..." -ForegroundColor Green
Write-Host ""

# 1. Delete feature files in resources directory
$resourcesDir = ".\src-tauri\resources"
Write-Host "Deleting feature files in resources directory..." -ForegroundColor Yellow
Remove-Item "$resourcesDir\monster_features_opencv_v2.bin" -ErrorAction SilentlyContinue
Remove-Item "$resourcesDir\card_features_small.bin" -ErrorAction SilentlyContinue
Remove-Item "$resourcesDir\card_features_medium.bin" -ErrorAction SilentlyContinue
Remove-Item "$resourcesDir\card_features_large.bin" -ErrorAction SilentlyContinue
Write-Host "Resources feature files deleted" -ForegroundColor Green
Write-Host ""

# 2. Delete feature files in target/debug/resources and target/release/resources
Write-Host "Deleting feature files in target directories..." -ForegroundColor Yellow
$debugResources = ".\src-tauri\target\debug\resources"
$releaseResources = ".\src-tauri\target\release\resources"

if (Test-Path $debugResources) {
    Remove-Item "$debugResources\monster_features_opencv_v2.bin" -ErrorAction SilentlyContinue
    Remove-Item "$debugResources\card_features_small.bin" -ErrorAction SilentlyContinue
    Remove-Item "$debugResources\card_features_medium.bin" -ErrorAction SilentlyContinue
    Remove-Item "$debugResources\card_features_large.bin" -ErrorAction SilentlyContinue
    Write-Host "Debug target feature files deleted" -ForegroundColor Green
}

if (Test-Path $releaseResources) {
    Remove-Item "$releaseResources\monster_features_opencv_v2.bin" -ErrorAction SilentlyContinue
    Remove-Item "$releaseResources\card_features_small.bin" -ErrorAction SilentlyContinue
    Remove-Item "$releaseResources\card_features_medium.bin" -ErrorAction SilentlyContinue
    Remove-Item "$releaseResources\card_features_large.bin" -ErrorAction SilentlyContinue
    Write-Host "Release target feature files deleted" -ForegroundColor Green
}
Write-Host ""

# 3. Delete feature files in AppData cache directory
$appDataDir = "$env:APPDATA\com.bazaarhelper.app"
if (Test-Path $appDataDir) {
    Write-Host "Deleting feature files in AppData cache directory..." -ForegroundColor Yellow
    Remove-Item "$appDataDir\monster_features_opencv_v2.bin" -ErrorAction SilentlyContinue
    Remove-Item "$appDataDir\card_features_opencv.bin" -ErrorAction SilentlyContinue
    Remove-Item "$appDataDir\card_features_small.bin" -ErrorAction SilentlyContinue
    Remove-Item "$appDataDir\card_features_medium.bin" -ErrorAction SilentlyContinue
    Remove-Item "$appDataDir\card_features_large.bin" -ErrorAction SilentlyContinue
    Write-Host "AppData cache feature files deleted" -ForegroundColor Green
}
else {
    Write-Host "AppData directory does not exist, skipping" -ForegroundColor Gray
}

Write-Host ""
Write-Host "Feature files deleted successfully!" -ForegroundColor Green
Write-Host ""
Write-Host "Next steps:" -ForegroundColor Cyan
Write-Host "1. Run the application (dev mode: npm run tauri dev OR built app)" -ForegroundColor White
Write-Host "2. The app will auto-generate features from:" -ForegroundColor White
Write-Host "   - Monsters: src-tauri/resources/images_monster_char/*.webp" -ForegroundColor Gray
Write-Host "   - Items: src-tauri/resources/images/*.webp" -ForegroundColor Gray
Write-Host "3. Generated files will be saved to AppData directory" -ForegroundColor White
Write-Host "4. To bundle: copy .bin files from AppData to src-tauri/resources/" -ForegroundColor White
Write-Host ""
Write-Host "AppData path: $appDataDir" -ForegroundColor Gray
