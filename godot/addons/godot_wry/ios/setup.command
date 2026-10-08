#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ADDON_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
MAC_FRAMEWORK="$ADDON_DIR/bin/universal-apple-darwin/libgodot_wry.framework"

echo "[godot_wry] Preparing turnkey iOS support..."

# Files extracted from a browser-downloaded GitHub ZIP may inherit the
# com.apple.quarantine xattr. Remove it only from this addon copy; do not
# disable Gatekeeper globally.
xattr -dr com.apple.quarantine "$ADDON_DIR" 2>/dev/null || true

# Release CI always gives the universal framework a valid ad-hoc signature,
# and Developer ID signing may replace it on main when credentials exist.
# Re-sign locally after clearing quarantine so dlopen cannot inherit a stale
# code signature from archive extraction/copying.
if [[ -d "$MAC_FRAMEWORK" ]]; then
  codesign --force --deep --sign - --timestamp=none "$MAC_FRAMEWORK"
  codesign --verify --deep --strict "$MAC_FRAMEWORK"
  echo "[godot_wry] macOS host framework is ready."
fi

python3 "$SCRIPT_DIR/install.py" "$@"

echo
echo "[godot_wry] iOS turnkey setup complete."
echo "[godot_wry] Open the project in Godot 4.7 and export iOS normally."
echo "[godot_wry] Xcode is still required by Apple to build/sign iOS apps."
