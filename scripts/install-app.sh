#!/usr/bin/env bash
set -e

# OrbitSwitch local macOS build & install script
# Builds the .app release bundle and installs it to /Applications/OrbitSwitch.app

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "Error: This script is only intended for macOS." >&2
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

echo "==> Building OrbitSwitch.app (Release mode)..."
export PATH="$HOME/.bun/bin:$PATH"
bun tauri build --bundles app -c '{"bundle":{"createUpdaterArtifacts":false}}'

APP_SOURCE="src-tauri/target/release/bundle/macos/OrbitSwitch.app"
DESTINATION="/Applications/OrbitSwitch.app"

if [[ ! -d "$APP_SOURCE" ]]; then
  echo "Error: Built app bundle not found at $APP_SOURCE" >&2
  exit 1
fi

echo "==> Closing running OrbitSwitch instance (if any)..."
killall OrbitSwitch 2>/dev/null || true
sleep 0.5

echo "==> Installing to $DESTINATION..."
rm -rf "$DESTINATION"
cp -R "$APP_SOURCE" "$DESTINATION"

echo "==> Successfully installed OrbitSwitch.app to /Applications!"
echo "    You can launch it via: open /Applications/OrbitSwitch.app"
