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
echo "  1. Restart IBus:  ibus restart"
echo "  2. Add 'Gochu Telex' in Settings > Keyboard > Input Sources"
echo "     or run: ibus engine gochu-telex"
