#!/bin/sh
set -e

if [ "$#" -ne 1 ]; then
  echo "Usage: $0 NEW_VERSION" >&2
  echo "Example: $0 0.2.0" >&2
  exit 1
fi

NEW_VER="$1"

ROOT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT_DIR"

# Ensure git working tree is clean before making release changes.
if ! git diff --quiet || ! git diff --cached --quiet; then
  echo "Error: git working tree is not clean. Commit or stash changes first." >&2
  exit 1
fi

# Detect current version from gochu-core as the source of truth.
CURRENT_VER="$(grep '^version = \"' gochu-core/Cargo.toml | head -n1 | sed 's/.*\"\\(.*\\)\".*/\\1/')"

if [ -z "$CURRENT_VER" ]; then
  echo "Error: could not detect current version from gochu-core/Cargo.toml" >&2
  exit 1
fi

echo "Current version: $CURRENT_VER"
echo "New version:     $NEW_VER"

# Update crate versions.
sed -i "s/version = \"$CURRENT_VER\"/version = \"$NEW_VER\"/" gochu-core/Cargo.toml
sed -i "s/version = \"$CURRENT_VER\"/version = \"$NEW_VER\"/" gochu-wasm/Cargo.toml
sed -i "s/version = \"$CURRENT_VER\"/version = \"$NEW_VER\"/" gochu-ibus/Cargo.toml

# Run tests for safety.
cargo test --workspace

# Rebuild the web demo so docs/pkg and docs/version.js are in sync.
./build.sh

# Stage changes.
git add gochu-core/Cargo.toml gochu-wasm/Cargo.toml gochu-ibus/Cargo.toml docs/pkg docs/version.js

# Commit with a standard message.
git commit -m "Bump version to $NEW_VER and rebuild web demo"

# Create and push the tag. The release workflow triggers on v* tags.
TAG="v$NEW_VER"
git tag "$TAG"
git push origin main
git push origin "$TAG"

echo "Release $TAG created and pushed."

