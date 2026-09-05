#!/bin/bash

set -euo pipefail

echo "Compiling Rust to WebAssembly..."
cargo build --release --target wasm32-unknown-unknown

echo "Compiling wasm-bindgen..."
wasm-bindgen --out-name esp-viewer --out-dir wasm/target --target web target/wasm32-unknown-unknown/release/esp-viewer.wasm

echo "Optimizing WebAssembly..."
wasm-opt wasm/target/esp-viewer_bg.wasm -Oz --strip-debug -o wasm/target/esp-viewer_bg.wasm
