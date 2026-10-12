# SPIKE: build the Phase 1b browser engine (approach B) and optionally serve it.
#   tools\build_web.ps1          # build into web\pkg
#   tools\build_web.ps1 -Serve   # build, then serve the repo root on http://localhost:8000
param([switch]$Serve, [int]$Port = 8000)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
try {
    cargo build -p ferrocade-web --target wasm32-unknown-unknown --release
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
    wasm-bindgen --target web --out-dir web/pkg target/wasm32-unknown-unknown/release/ferrocade_web.wasm
    if ($LASTEXITCODE -ne 0) { throw "wasm-bindgen failed (is wasm-bindgen-cli 0.2.129 installed?)" }
    Write-Host "Built web/pkg. Open http://localhost:$Port/web/ (add ?backend=webgl2 to force WebGL2)."
    if ($Serve) { python -m http.server $Port }
} finally {
    Pop-Location
}
