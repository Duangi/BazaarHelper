#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
APP_NAME="${APP_NAME:-BazaarHelper.app}"
SRC_APP="${SRC_APP:-$ROOT_DIR/src-tauri/target/release/bundle/macos/$APP_NAME}"
SIGNING_IDENTITY="${SIGNING_IDENTITY:-}"

if [[ ! -d "$SRC_APP" ]]; then
  echo "[sign] Missing app bundle: $SRC_APP"
  echo "[sign] Run: npm run tauri build"
  exit 1
fi

if [[ -z "$SIGNING_IDENTITY" ]]; then
  SIGNING_IDENTITY="$(security find-identity -v -p codesigning 2>/dev/null \
    | sed -n 's/.*"\(Apple Development:.*\)"/\1/p' \
    | head -n1 || true)"
fi

if [[ -z "$SIGNING_IDENTITY" ]]; then
  echo "[sign] No code signing identity found. Keeping existing signature."
  exit 0
fi

echo "[sign] Signing app with identity: $SIGNING_IDENTITY"

codesign --force --deep --sign "$SIGNING_IDENTITY" --timestamp=none "$SRC_APP"

echo "[sign] Verifying signature..."
codesign --verify --deep --strict --verbose=2 "$SRC_APP"
codesign -dv --verbose=4 "$SRC_APP" 2>&1 | sed -n '1,20p'

echo "[sign] Done."
