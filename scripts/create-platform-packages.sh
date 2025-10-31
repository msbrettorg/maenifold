#!/bin/bash
# Creates platform-specific npm package structures

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
PACKAGES_DIR="$REPO_ROOT/packages"

# Read version from main package.json
VERSION=$(node -p "require('$REPO_ROOT/package.json').version")
echo "Creating platform packages version $VERSION in $PACKAGES_DIR"

# Clean and create packages directory
rm -rf "$PACKAGES_DIR"
mkdir -p "$PACKAGES_DIR"

# Platform configurations: "package-name|runtime-id|binary-name"
PLATFORMS=(
  "maenifold-linux-x64|linux-x64|Maenifold"
  "maenifold-linux-arm64|linux-arm64|Maenifold"
  "maenifold-darwin-x64|osx-x64|Maenifold"
  "maenifold-darwin-arm64|osx-arm64|Maenifold"
  "maenifold-win32-x64|win-x64|Maenifold.exe"
  "maenifold-win32-arm64|win-arm64|Maenifold.exe"
)

for platform in "${PLATFORMS[@]}"; do
  IFS='|' read -r pkg_name runtime_id binary_name <<< "$platform"

  echo "Creating package: $pkg_name"

  PKG_DIR="$PACKAGES_DIR/$pkg_name"
  mkdir -p "$PKG_DIR"

  # Create package.json
  cat > "$PKG_DIR/package.json" <<EOF
{
  "name": "$pkg_name",
  "version": "$VERSION",
  "description": "maenifold binary for $runtime_id",
  "main": "index.js",
  "os": ["${runtime_id%%-*}"],
  "cpu": ["${runtime_id##*-}"],
  "publishConfig": {
    "access": "public"
  },
  "repository": {
    "type": "git",
    "url": "git+https://github.com/MSBrett/maenifold.git"
  },
  "keywords": [
    "maenifold",
    "binary",
    "$runtime_id"
  ],
  "author": "MSBrett",
  "license": "MIT",
  "files": [
    "**/*"
  ]
}
EOF

  # Create index.js (empty, just for package validity)
  cat > "$PKG_DIR/index.js" <<EOF
// This package contains the maenifold binary for $runtime_id
// The binary is executed by the main @ma-collective/maenifold package
module.exports = {};
EOF

  # Copy main README.md
  if [ -f "$REPO_ROOT/README.md" ]; then
    cp "$REPO_ROOT/README.md" "$PKG_DIR/README.md"
    echo "  ✓ Copied README.md"
  else
    echo "  ⚠ README.md not found"
  fi

  # Copy runtime payload (binary, assets, supporting libs)
  RUNTIME_DIR="$REPO_ROOT/bin/$runtime_id"
  if [ -d "$RUNTIME_DIR" ]; then
    cp -R "$RUNTIME_DIR/"* "$PKG_DIR/"
    find "$PKG_DIR" -name ".DS_Store" -delete
    echo "  ✓ Copied runtime payload from bin/$runtime_id"
  else
    echo "  ⚠ Runtime directory not found: $RUNTIME_DIR (run build first)"
  fi
done

echo ""
echo "Platform packages created successfully!"
echo "Next steps:"
echo "  1. Build all platforms: npm run build:all"
echo "  2. Run this script again to copy binaries: ./scripts/create-platform-packages.sh"
echo "  3. Publish platform packages: cd packages/[platform] && npm publish"
echo "  4. Publish main package: npm publish"
