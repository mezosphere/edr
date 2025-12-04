#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

FEATURES="op"

echo "=== Building EDR for all platforms ==="
echo "This script builds native addons for macOS and Linux"
echo ""

# macOS ARM64 (native)
echo ">>> [1/6] Building for aarch64-apple-darwin (native)..."
cd crates/edr_napi
npx napi build --platform --release --features "$FEATURES" --target aarch64-apple-darwin
cp edr.darwin-arm64.node npm/darwin-arm64/
cd ../..

# macOS x64 (cross-compile)
echo ""
echo ">>> [2/6] Building for x86_64-apple-darwin..."
cd crates/edr_napi
npx napi build --platform --release --features "$FEATURES" --target x86_64-apple-darwin
cp edr.darwin-x64.node npm/darwin-x64/
cd ../..

# Linux x64 GNU (using cross/Docker)
echo ""
echo ">>> [3/6] Building for x86_64-unknown-linux-gnu (via cross)..."
cross build --release -p edr_napi --features "$FEATURES" --target x86_64-unknown-linux-gnu
cp target/x86_64-unknown-linux-gnu/release/libedr_napi.so crates/edr_napi/npm/linux-x64-gnu/edr.linux-x64-gnu.node

# Linux x64 MUSL (using cross/Docker)
echo ""
echo ">>> [4/6] Building for x86_64-unknown-linux-musl (via cross)..."
cross build --release -p edr_napi --features "$FEATURES" --target x86_64-unknown-linux-musl
cp target/x86_64-unknown-linux-musl/release/libedr_napi.so crates/edr_napi/npm/linux-x64-musl/edr.linux-x64-musl.node

# Linux ARM64 GNU (using cross/Docker)
echo ""
echo ">>> [5/6] Building for aarch64-unknown-linux-gnu (via cross)..."
cross build --release -p edr_napi --features "$FEATURES" --target aarch64-unknown-linux-gnu
cp target/aarch64-unknown-linux-gnu/release/libedr_napi.so crates/edr_napi/npm/linux-arm64-gnu/edr.linux-arm64-gnu.node

# Linux ARM64 MUSL (using cross/Docker)
echo ""
echo ">>> [6/6] Building for aarch64-unknown-linux-musl (via cross)..."
cross build --release -p edr_napi --features "$FEATURES" --target aarch64-unknown-linux-musl
cp target/aarch64-unknown-linux-musl/release/libedr_napi.so crates/edr_napi/npm/linux-arm64-musl/edr.linux-arm64-musl.node

echo ""
echo "=== All builds complete! ==="
echo ""
echo "Built artifacts:"
ls -lh crates/edr_napi/npm/*/*.node 2>/dev/null || true

echo ""
echo "To package for distribution:"
echo "  cd crates/edr_napi"
echo "  # Pack platform packages"
echo "  for dir in npm/*/; do (cd \"\$dir\" && npm pack); done"
echo "  # Pack main package"
echo "  npm pack"
