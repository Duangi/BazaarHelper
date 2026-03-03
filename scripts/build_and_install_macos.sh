#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
LAUNCH_AFTER="${LAUNCH_AFTER:-1}"
APP_NAME="${APP_NAME:-BazaarHelper.app}"

cd "$ROOT_DIR"

echo "[build-install] Building release bundle..."
npm run tauri build

echo "[build-install] Signing bundle..."
bash "$ROOT_DIR/scripts/sign_macos_app.sh"

echo "[build-install] Installing to /Applications..."
bash "$ROOT_DIR/scripts/install_macos_app.sh"

if [[ "$LAUNCH_AFTER" == "1" ]]; then
  echo "[build-install] Launching app..."
  open -a "/Applications/$APP_NAME" || true
fi

echo "[build-install] Completed."
