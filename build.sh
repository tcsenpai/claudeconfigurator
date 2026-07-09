#!/usr/bin/env bash
# Build the ClaudeConfigurator app bundle (and optionally a distributable
# installer) for the current platform.
#
# Usage:
#   ./build.sh              Build the platform's default bundle(s)
#   ./build.sh --dmg        macOS: build the .dmg installer (implies .app)
#   ./build.sh --app        macOS: build only the .app bundle (no .dmg)
#   ./build.sh --appimage   Linux: build the AppImage
#   ./build.sh --deb        Linux: build the .deb package
#   ./build.sh --rpm        Linux: build the .rpm package
#   ./build.sh --bundles a,b Pass an explicit comma/space list to tauri
#
# With no flag, macOS builds app+dmg and Linux builds deb+appimage.
set -euo pipefail
cd "$(dirname "$0")"

os="$(uname -s)"
bundles=""

case "${1:-}" in
  --dmg)      bundles="dmg" ;;
  --app)      bundles="app" ;;
  --appimage) bundles="appimage" ;;
  --deb)      bundles="deb" ;;
  --rpm)      bundles="rpm" ;;
  --bundles)  bundles="${2:?--bundles needs a value}" ;;
  "")         : ;;  # platform default below
  *) echo "unknown flag: $1" >&2; exit 2 ;;
esac

# Platform default when no explicit bundle list was given.
if [ -z "$bundles" ]; then
  case "$os" in
    Darwin) bundles="app,dmg" ;;
    Linux)  bundles="deb,appimage" ;;
    *) echo "unsupported OS: $os (build manually with: bun run tauri build)" >&2; exit 1 ;;
  esac
fi

command -v bun >/dev/null || { echo "bun not found (https://bun.sh)" >&2; exit 1; }

echo "==> installing frontend deps"
bun install --frozen-lockfile 2>/dev/null || bun install

echo "==> building bundles: $bundles"
bun run tauri build --bundles "$bundles"

echo
echo "==> artifacts:"
find src-tauri/target/release/bundle -maxdepth 2 -type f \
  \( -name '*.dmg' -o -name '*.app' -o -name '*.deb' -o -name '*.rpm' -o -name '*.AppImage' \) \
  2>/dev/null || true
# .app is a directory, list it separately.
find src-tauri/target/release/bundle -maxdepth 2 -type d -name '*.app' 2>/dev/null || true
