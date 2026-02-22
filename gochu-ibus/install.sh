#!/bin/sh
set -e

PREFIX="${PREFIX:-/usr/local}"
IBUS_COMPONENT_DIR="${IBUS_COMPONENT_DIR:-/usr/share/ibus/component}"

echo "Building gochu-ibus (release)..."
cargo build --release -p gochu-ibus

echo "Installing binary to $PREFIX/bin/gochu-ibus"
install -Dm755 target/release/gochu-ibus "$PREFIX/bin/gochu-ibus"

echo "Installing IBus component to $IBUS_COMPONENT_DIR/gochu.xml"
install -Dm644 gochu-ibus/data/gochu.xml "$IBUS_COMPONENT_DIR/gochu.xml"

echo ""
echo "Done. To activate:"
echo "  1. Restart IBus:  ibus restart"
echo "  2. Add 'Gochu Telex' in Settings > Keyboard > Input Sources"
echo "     or run: ibus engine gochu-telex"
