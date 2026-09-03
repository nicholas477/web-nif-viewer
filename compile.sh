#!/bin/bash

echo "Compiling Rust to WebAssembly..."
cargo build --release --target wasm32-unknown-unknown

echo "Compiling wasm-bindgen..."
wasm-bindgen --out-name esp-viewer --out-dir wasm/target --target web target/wasm32-unknown-unknown/release/esp-viewer.wasm

echo "Optimizing WebAssembly..."
wasm-opt wasm/target/esp-viewer_bg.wasm -O3 -o wasm/target/esp-viewer_bg.wasm
