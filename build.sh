#!/usr/bin/env bash
# Build the wasm module and bundle it into a single offline HTML file.
set -euo pipefail
cd "$(dirname "$0")"

cargo build --release --target wasm32-unknown-unknown
wasm-bindgen --target no-modules --no-typescript --out-dir pkg \
  target/wasm32-unknown-unknown/release/qr_logo.wasm
if command -v wasm-opt >/dev/null 2>&1; then
  wasm-opt -Oz -o pkg/qr_logo_bg.wasm pkg/qr_logo_bg.wasm
fi
python3 tools/bundle.py www/index.html pkg/qr_logo.js pkg/qr_logo_bg.wasm dist/index.html
echo "Wrote dist/index.html ($(stat -c %s dist/index.html) bytes)"
