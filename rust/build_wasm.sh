#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
RUST_DIR="$SCRIPT_DIR"
PKG_DIR="$RUST_DIR/pkg"
DEST_DIR="$RUST_DIR/../example/web/wasm"

PROFILE="${1:-dev}"

echo "Building WASM ($PROFILE)..."
cd "$RUST_DIR"
wasm-pack build --target no-modules "--$PROFILE"

echo "Copying to $DEST_DIR..."
mkdir -p "$DEST_DIR"
cp "$PKG_DIR/flutter_vulkan_native.js" "$DEST_DIR/"
cp "$PKG_DIR/flutter_vulkan_native_bg.wasm" "$DEST_DIR/"

echo "Done. WASM files:"
ls -lh "$DEST_DIR/"
