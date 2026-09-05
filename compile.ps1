# Stop the script for any PowerShell cmdlet errors
$ErrorActionPreference = "Stop"

# Stop the script if external programs/EXEs fail (Requires PowerShell 7.3+)
$PSNativeCommandUseErrorActionPreference = $true

cargo build --release --target wasm32-unknown-unknown

# print out that we're runnign this
Write-Host "Running wasm-bindgen" -ForegroundColor Green
wasm-bindgen --out-name esp-viewer --out-dir wasm/target --target web target/wasm32-unknown-unknown/release/esp-viewer.wasm

#Write-Host "Running wasm-opt" -ForegroundColor Green
#wasm-opt wasm/target/esp-viewer_bg.wasm -O3 -o wasm/target/esp-viewer_bg.wasm

Write-Host "Copying assets" -ForegroundColor Green
Remove-Item -Path "wasm/assets" -Recurse -Force -ErrorAction Ignore
Copy-Item -Path "assets" -Destination "wasm/assets" -Recurse

# copy to Z:\wasm
#Remove-Item -Path "Z:\wasm" -Recurse -Force
#Copy-Item -Path "wasm" -Destination "Z:\wasm" -Recurse