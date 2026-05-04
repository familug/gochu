#!/bin/sh
set -e

# Resolve repo root from script location so the script works regardless of CWD.
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

PREFIX="${PREFIX:-/usr/local}"
IBUS_COMPONENT_DIR="${IBUS_COMPONENT_DIR:-/usr/share/ibus/component}"

echo "Building gochu-ibus (release)..."
cargo build --release -p gochu-ibus --manifest-path "$REPO_ROOT/Cargo.toml"

echo "Installing binary to $PREFIX/bin/gochu-ibus"
install -Dm755 "$REPO_ROOT/target/release/gochu-ibus" "$PREFIX/bin/gochu-ibus"

echo "Installing IBus component to $IBUS_COMPONENT_DIR/gochu.xml"
sed "s|/usr/local/bin/gochu-ibus|$PREFIX/bin/gochu-ibus|g" \
    "$SCRIPT_DIR/data/gochu.xml" > /tmp/gochu-ibus-component.xml
install -Dm644 /tmp/gochu-ibus-component.xml "$IBUS_COMPONENT_DIR/gochu.xml"
rm -f /tmp/gochu-ibus-component.xml

echo ""
echo "Done. To activate:"

# Detect the active input method framework and print the appropriate instructions.
if command -v fcitx5 >/dev/null 2>&1; then
    echo "  Detected: fcitx5"
    echo ""
    echo "  fcitx5 can run IBus engines via the fcitx5-ibus compatibility layer."
    echo "  Make sure fcitx5-ibus is installed:"
    echo "    Arch/Manjaro:  sudo pacman -S fcitx5-ibus"
    echo "    Ubuntu/Debian: sudo apt install fcitx5-ibus"
    echo ""
    echo "  Then restart fcitx5 so it picks up the new engine:"
    echo "    fcitx5 -r &"
    echo ""
    echo "  Finally, open fcitx5-configtool, go to 'Input Method', click '+', search"
    echo "  for 'Gochu Telex' (it appears under the IBus category), and add it."
elif command -v ibus >/dev/null 2>&1; then
    echo "  Detected: IBus"
    echo "  1. Restart IBus:  ibus restart"
    echo "  2. Add 'Gochu Telex' in Settings > Keyboard > Input Sources"
    echo "     or run: ibus engine gochu-telex"
else
    echo "  IBus:"
    echo "    1. ibus restart"
    echo "    2. Add 'Gochu Telex' in Settings > Keyboard > Input Sources"
    echo ""
    echo "  fcitx5 (requires fcitx5-ibus installed):"
    echo "    1. fcitx5 -r &"
    echo "    2. Open fcitx5-configtool, add 'Gochu Telex' from the IBus category"
fi
