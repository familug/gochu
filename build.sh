#!/bin/sh
set -e
wasm-pack build gochu-wasm --target web --out-dir ../docs/pkg
rm -f docs/pkg/.gitignore
echo "Build complete. docs/pkg/ ready for GitHub Pages."
