#!/usr/bin/env bash
# Install a built ClaudeConfigurator bundle onto the current system.
# Run ./build.sh first. This only copies/installs existing artifacts.
#
#   macOS: copies the .app into /Applications
#   Linux: installs the .deb (dpkg) or .rpm (rpm), else copies the AppImage
#          into ~/.local/bin
set -euo pipefail
cd "$(dirname "$0")"

os="$(uname -s)"
bundle_dir="src-tauri/target/release/bundle"

[ -d "$bundle_dir" ] || { echo "no build found. Run ./build.sh first." >&2; exit 1; }

case "$os" in
  Darwin)
    app="$(find "$bundle_dir/macos" -maxdepth 1 -type d -name '*.app' 2>/dev/null | head -1)"
    [ -n "$app" ] || { echo "no .app in $bundle_dir/macos. Run ./build.sh --app" >&2; exit 1; }
    dest="/Applications/$(basename "$app")"
    echo "==> installing $(basename "$app") -> /Applications"
    rm -rf "$dest"
    cp -R "$app" "$dest"
    echo "installed: $dest"
    ;;

  Linux)
    deb="$(find "$bundle_dir/deb" -maxdepth 1 -name '*.deb' 2>/dev/null | head -1)"
    rpm="$(find "$bundle_dir/rpm" -maxdepth 1 -name '*.rpm' 2>/dev/null | head -1)"
    appimage="$(find "$bundle_dir/appimage" -maxdepth 1 -name '*.AppImage' 2>/dev/null | head -1)"

    if [ -n "$deb" ] && command -v dpkg >/dev/null; then
      echo "==> installing $(basename "$deb") (sudo dpkg)"
      sudo dpkg -i "$deb" || sudo apt-get -f install -y
    elif [ -n "$rpm" ] && command -v rpm >/dev/null; then
      echo "==> installing $(basename "$rpm") (sudo rpm)"
      sudo rpm -i --replacepkgs "$rpm"
    elif [ -n "$appimage" ]; then
      dest="$HOME/.local/bin/claudeconfigurator.AppImage"
      mkdir -p "$HOME/.local/bin"
      echo "==> installing AppImage -> $dest"
      cp "$appimage" "$dest"
      chmod +x "$dest"
      echo "installed: $dest (ensure ~/.local/bin is on PATH)"
    else
      echo "no installable artifact found. Run ./build.sh --deb|--rpm|--appimage" >&2
      exit 1
    fi
    ;;

  *)
    echo "unsupported OS: $os" >&2
    exit 1
    ;;
esac
