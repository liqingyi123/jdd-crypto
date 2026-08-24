#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

npm ci
npm run build
npx tauri build

version="$(node -p "require('./package.json').version")"
product_name="多多解密"
expected_name="${product_name}_${version}_aarch64.dmg"
dmg_dir="$root/src-tauri/target/release/bundle/dmg"

if [[ ! -d "$dmg_dir" ]]; then
  echo "[error] DMG output directory not found: $dmg_dir"
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

echo "[ok] macOS bundle: $target_path"
