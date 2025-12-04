# EDR Development Guide

## Overview

EDR (Ethereum Development Runtime) is the Rust backend for Hardhat. It exposes a Node.js native addon via the `edr_napi` crate, published as `@nomicfoundation/edr` on npm.

## Project Structure

```
crates/
├── edr_napi/           # Node.js native addon (napi-rs)
│   ├── npm/            # Platform-specific package directories
│   │   ├── darwin-arm64/
│   │   ├── darwin-x64/
│   │   ├── linux-arm64-gnu/
│   │   ├── linux-arm64-musl/
│   │   ├── linux-x64-gnu/
│   │   ├── linux-x64-musl/
│   │   └── win32-x64-msvc/
│   ├── dist/           # Distribution files (install.js, package.json for releases)
│   ├── index.js        # Main entry point
│   └── package.json    # npm package config
├── edr_provider/       # Core provider implementation
│   └── src/
│       ├── data.rs     # State management (dump_state, load_state)
│       └── requests/   # RPC method handlers
└── state/
    └── api/
        └── src/
            └── diff.rs # StateDiff implementation
```

## Building

### Prerequisites

- Rust toolchain (rustup)
- Node.js 20+ with pnpm
- Docker (for cross-compilation)
- `cross` for cross-compilation: `cargo install cross`
- `gh` CLI for GitHub releases: `brew install gh`

### Local Development Build

```bash
cd crates/edr_napi
pnpm install
pnpm run build:dev  # Debug build with test features
```

### Release Build (Single Platform)

```bash
cd crates/edr_napi
npx napi build --platform --release --features op --target aarch64-apple-darwin
```

### Cross-Platform Build (All Platforms)

Run the build script from the repo root:

```bash
./scripts/build_all_platforms.sh
```

This builds for:
- macOS ARM64 (native)
- macOS x64 (cross-compile)
- Linux x64 GNU (via cross/Docker)
- Linux x64 MUSL (via cross/Docker)
- Linux ARM64 GNU (via cross/Docker)
- Linux ARM64 MUSL (via cross/Docker)

**Note:** Windows builds require actual Windows hardware or CI.

### Manual Cross-Compilation Commands

```bash
# macOS ARM64 (native on Apple Silicon)
cd crates/edr_napi
npx napi build --platform --release --features op --target aarch64-apple-darwin
cp edr.darwin-arm64.node npm/darwin-arm64/

# macOS x64
npx napi build --platform --release --features op --target x86_64-apple-darwin
cp edr.darwin-x64.node npm/darwin-x64/

# Linux x64 GNU (via cross)
cd ../..  # repo root
cross build --release -p edr_napi --features op --target x86_64-unknown-linux-gnu
cp target/x86_64-unknown-linux-gnu/release/libedr_napi.so crates/edr_napi/npm/linux-x64-gnu/edr.linux-x64-gnu.node

# Linux x64 MUSL
cross build --release -p edr_napi --features op --target x86_64-unknown-linux-musl
cp target/x86_64-unknown-linux-musl/release/libedr_napi.so crates/edr_napi/npm/linux-x64-musl/edr.linux-x64-musl.node

# Linux ARM64 GNU
cross build --release -p edr_napi --features op --target aarch64-unknown-linux-gnu
cp target/aarch64-unknown-linux-gnu/release/libedr_napi.so crates/edr_napi/npm/linux-arm64-gnu/edr.linux-arm64-gnu.node

# Linux ARM64 MUSL
cross build --release -p edr_napi --features op --target aarch64-unknown-linux-musl
cp target/aarch64-unknown-linux-musl/release/libedr_napi.so crates/edr_napi/npm/linux-arm64-musl/edr.linux-arm64-musl.node
```

## Creating a Release

### 1. Update Version

Update version in `crates/edr_napi/package.json` and all platform package.json files:

```bash
# Example: Set version to 0.12.0-state-dump
NEW_VERSION="0.12.0-state-dump"

# Update main package.json
cd crates/edr_napi
sed -i '' "s/\"version\": \".*\"/\"version\": \"$NEW_VERSION\"/" package.json

# Update platform packages
for dir in npm/*/; do
  sed -i '' "s/\"version\": \".*\"/\"version\": \"$NEW_VERSION\"/" "$dir/package.json"
done
```

### 2. Build All Platforms

```bash
cd /path/to/edr
./scripts/build_all_platforms.sh
```

### 3. Create Distribution Package

The distribution package includes an install script that downloads the correct platform binary:

```bash
cd crates/edr_napi

# Ensure dist directory exists with install.js and package.json
# Copy necessary files
cp index.js index.d.ts dist/

# Update dist/package.json version
sed -i '' "s/\"version\": \".*\"/\"version\": \"$NEW_VERSION\"/" dist/package.json

# Create the main distribution tarball
cd dist
npm pack
```

### 4. Create Platform Tarballs

```bash
cd crates/edr_napi

# Pack each platform package
for dir in npm/*/; do
  (cd "$dir" && npm pack)
done

# List all tarballs
ls -la npm/*/*.tgz dist/*.tgz
```

### 5. Create GitHub Release

```bash
# Tag the release
git tag "v$NEW_VERSION"
git push origin "v$NEW_VERSION"

# Create release with all artifacts
gh release create "v$NEW_VERSION" \
  --repo mezosphere/edr \
  --title "v$NEW_VERSION" \
  --notes "Release with custom features" \
  crates/edr_napi/dist/*.tgz \
  crates/edr_napi/npm/darwin-arm64/*.tgz \
  crates/edr_napi/npm/darwin-x64/*.tgz \
  crates/edr_napi/npm/linux-x64-gnu/*.tgz \
  crates/edr_napi/npm/linux-x64-musl/*.tgz \
  crates/edr_napi/npm/linux-arm64-gnu/*.tgz \
  crates/edr_napi/npm/linux-arm64-musl/*.tgz
```

### 6. Verify Release

```bash
# List release assets
gh release view "v$NEW_VERSION" --repo mezosphere/edr

# Test installation
cd /tmp && rm -rf edr-test && mkdir edr-test && cd edr-test
npm init -y
npm install "https://github.com/mezosphere/edr/releases/download/v$NEW_VERSION/mezosphere-edr-$NEW_VERSION.tgz"
node -e "const edr = require('@mezosphere/edr'); console.log('EDR loaded:', edr.L1_CHAIN_TYPE)"
```

## Using Custom EDR in Projects

### Direct Installation

```bash
npm install https://github.com/mezosphere/edr/releases/download/v0.12.0-state-dump/mezosphere-edr-0.12.0-state-dump.tgz
```

### In package.json (as @mezosphere/edr)

```json
{
  "dependencies": {
    "@mezosphere/edr": "https://github.com/mezosphere/edr/releases/download/v0.12.0-state-dump/mezosphere-edr-0.12.0-state-dump.tgz"
  }
}
```

### Override @nomicfoundation/edr in Hardhat Projects

For pnpm:
```json
{
  "pnpm": {
    "overrides": {
      "@nomicfoundation/edr": "https://github.com/mezosphere/edr/releases/download/v0.12.0-state-dump/mezosphere-edr-0.12.0-state-dump.tgz"
    }
  }
}
```

For npm:
```json
{
  "overrides": {
    "@nomicfoundation/edr": "https://github.com/mezosphere/edr/releases/download/v0.12.0-state-dump/mezosphere-edr-0.12.0-state-dump.tgz"
  }
}
```

## Testing

### Run All Tests

```bash
cargo test
```

### Run Provider Tests

```bash
cargo test -p edr_provider
```

### Run State API Tests

```bash
cargo test -p edr_state_api
```

## Key Files

- `crates/edr_provider/src/data.rs` - Core provider data, includes `dump_state()` and `load_state()`
- `crates/edr_provider/src/requests/hardhat/state.rs` - RPC handlers for hardhat_dumpState/loadState
- `crates/edr_provider/src/requests/hardhat/rpc_types/state.rs` - StateAccount and StateDump types
- `crates/state/api/src/diff.rs` - StateDiff implementation (handles account status tracking)
- `crates/edr_napi/dist/install.js` - Postinstall script for downloading platform binaries

## Distribution Package Structure

The `dist/` directory contains files for the distributable package:

- `package.json` - Package manifest with postinstall script
- `install.js` - Downloads correct platform binary from GitHub releases
- `index.js` - Main entry point (copied from parent)
- `index.d.ts` - TypeScript definitions (copied from parent)

The install.js script:
1. Detects the current platform (os + arch)
2. Maps to the correct binary name (e.g., `edr.darwin-arm64.node`)
3. Downloads from GitHub release assets
4. Extracts to the package directory

## Troubleshooting

### Cross build fails

Ensure Docker is running:
```bash
docker ps
```

### Binary not found after install

Run the install script manually:
```bash
node node_modules/@mezosphere/edr/install.js
```

### Permission denied on Linux binaries

The `.so` files need execute permission:
```bash
chmod +x crates/edr_napi/npm/*/*.node
```
