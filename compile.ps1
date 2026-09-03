cargo build --release --target wasm32-unknown-unknown
wasm-bindgen --out-name esp-viewer --out-dir wasm/target --target web target/wasm32-unknown-unknown/release/esp-viewer.wasm
# wasm-opt wasm/target/esp-viewer_bg.wasm -O3 -o wasm/target/esp-viewer_bg.wasm