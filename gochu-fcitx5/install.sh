#!/bin/sh
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

PREFIX="${PREFIX:-/usr}"
FCITX5_ADDON_DIR="${FCITX5_ADDON_DIR:-$PREFIX/lib/fcitx5}"
FCITX5_ADDON_CONF_DIR="${FCITX5_ADDON_CONF_DIR:-$PREFIX/share/fcitx5/addon}"
FCITX5_IM_CONF_DIR="${FCITX5_IM_CONF_DIR:-$PREFIX/share/fcitx5/inputmethod}"

echo "Building gochu-fcitx5 (release)..."
echo "Note: requires fcitx5 development headers (fcitx5 / fcitx5-dev package)."
cargo build --release -p gochu-fcitx5 --manifest-path "$REPO_ROOT/Cargo.toml"

echo "Installing shared library to $FCITX5_ADDON_DIR/libgochu_fcitx5.so"
install -Dm755 \
    "$REPO_ROOT/target/release/libgochu_fcitx5.so" \
    "$FCITX5_ADDON_DIR/libgochu_fcitx5.so"

echo "Installing addon config to $FCITX5_ADDON_CONF_DIR/gochu.conf"
install -Dm644 "$SCRIPT_DIR/data/gochu.conf" "$FCITX5_ADDON_CONF_DIR/gochu.conf"

echo "Installing input method config to $FCITX5_IM_CONF_DIR/gochu.conf"
install -Dm644 "$SCRIPT_DIR/data/gochu-im.conf" "$FCITX5_IM_CONF_DIR/gochu.conf"

echo ""
echo "Done. To activate:"
echo "  1. Restart fcitx5:  fcitx5 -r &"
echo "  2. Open fcitx5-configtool, click '+', search for 'Gochu Telex', add it."
echo "  3. Switch to it with Ctrl+Space (or your configured IM keybinding)."
