#!/bin/sh
set -e
wasm-pack build gochu-wasm --target web --out-dir ../docs/pkg
rm -f docs/pkg/.gitignore

# Write version info for the web UI (commit hash + date)
COMMIT=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
DATE=$(git log -1 --format=%cI 2>/dev/null || echo "unknown")
printf 'window.GOCHU_VERSION = { commit: "%s", date: "%s" };\n' "$COMMIT" "$DATE" > docs/version.js

echo "Build complete. docs/pkg/ ready for GitHub Pages."
