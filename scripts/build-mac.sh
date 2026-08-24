#!/usr/bin/env bash
# Build a Universal macOS DMG (Intel + Apple Silicon). Must run on macOS.
# Expect roughly 2x build time and bundle size vs single-arch.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

npm ci
npm run build
rustup target add aarch64-apple-darwin x86_64-apple-darwin
npx tauri build -- --target universal-apple-darwin

version="$(node -p "require('./package.json').version")"
product_name="多多解密"
expected_name="${product_name}_${version}_universal.dmg"
dmg_dir="$root/src-tauri/target/universal-apple-darwin/release/bundle/dmg"

if [[ ! -d "$dmg_dir" ]]; then
  dmg_dir="$root/src-tauri/target/release/bundle/dmg"
fi

if [[ ! -d "$dmg_dir" ]]; then
  echo "[error] DMG output directory not found under src-tauri/target"
  exit 1
fi

shopt -s nullglob
dmgs=("$dmg_dir"/*.dmg)
shopt -u nullglob

if ((${#dmgs[@]} == 0)); then
  echo "[error] No DMG artifact found under $dmg_dir"
  exit 1
fi

dmg_path="${dmgs[0]}"
target_path="$dmg_dir/$expected_name"

if [[ "$(basename "$dmg_path")" != "$expected_name" ]]; then
  cp -f "$dmg_path" "$target_path"
  echo "[ok] Copied $(basename "$dmg_path") -> $expected_name"
else
  echo "[ok] DMG already named $expected_name"
fi

echo "[ok] macOS Universal bundle: $target_path"
