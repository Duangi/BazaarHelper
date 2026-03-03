#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
APP_NAME="${APP_NAME:-BazaarHelper.app}"
APP_BUNDLE_NAME="${APP_BUNDLE_NAME:-BazaarHelper}"
SRC_APP="${SRC_APP:-$ROOT_DIR/src-tauri/target/release/bundle/macos/$APP_NAME}"
TARGET_DIR="${TARGET_DIR:-/Applications}"
DEST_APP="$TARGET_DIR/$APP_NAME"

if [[ ! -d "$SRC_APP" ]]; then
  echo "[install] Missing app bundle: $SRC_APP"
  echo "[install] Run: npm run tauri build"
  exit 1
fi

echo "[install] Source: $SRC_APP"
echo "[install] Target: $DEST_APP"

if pgrep -x "$APP_BUNDLE_NAME" >/dev/null 2>&1; then
  echo "[install] App is running, asking it to quit..."
  osascript -e "tell application \"$APP_BUNDLE_NAME\" to quit" >/dev/null 2>&1 || true
  sleep 1
fi
pkill -f "/$APP_NAME/Contents/MacOS/" >/dev/null 2>&1 || true

USE_SUDO=0
if [[ ! -w "$TARGET_DIR" ]]; then
  USE_SUDO=1
fi

run_copy() {
  if [[ "$USE_SUDO" -eq 1 ]]; then
    sudo rm -rf "$DEST_APP"
    sudo ditto "$SRC_APP" "$DEST_APP"
    sudo xattr -dr com.apple.quarantine "$DEST_APP" >/dev/null 2>&1 || true
  else
    rm -rf "$DEST_APP"
    ditto "$SRC_APP" "$DEST_APP"
    xattr -dr com.apple.quarantine "$DEST_APP" >/dev/null 2>&1 || true
  fi
}

echo "[install] Installing..."
run_copy

echo "[install] Done."
echo "[install] Installed app: $DEST_APP"
