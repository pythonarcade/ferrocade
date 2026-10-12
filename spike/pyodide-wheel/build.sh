#!/usr/bin/env bash
# SPIKE: build the approach-A experiment wheel for Pyodide 314 inside WSL2.
#   bash spike/pyodide-wheel/build.sh          # A1: core data layer only
#   bash spike/pyodide-wheel/build.sh render   # A2: also link wgpu
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
PYODIDE_VERSION=314.0.7
source "$HOME/.cargo/env"
source "$HOME/emsdk/emsdk_env.sh" >/dev/null 2>&1
VENV="$HOME/pyodide-venv"
export PATH="$VENV/bin:$PATH"
# Keep build output off the slow /mnt/c filesystem.
export CARGO_TARGET_DIR="$HOME/ferrocade-a1-target"

pyodide xbuildenv install "$PYODIDE_VERSION" >/dev/null 2>&1 || pyodide xbuildenv use "$PYODIDE_VERSION"
echo "pyodide xbuildenv: $(pyodide config get version 2>/dev/null) python=$(pyodide config get python_version 2>/dev/null) emscripten=$(pyodide config get emscripten_version 2>/dev/null)"
echo "rustflags: $(pyodide config get rustflags 2>/dev/null)"

cd "$HERE/crate"
FEATURES=""
if [ "${1:-}" = "render" ]; then FEATURES="--features render"; fi
mkdir -p "$HERE/dist"
export CARGO_TARGET_WASM32_UNKNOWN_EMSCRIPTEN_RUSTFLAGS="$(pyodide config get rustflags)"
maturin build --release --target wasm32-unknown-emscripten -i python3.14 $FEATURES -o "$HERE/dist" 2>&1
ls -la "$HERE/dist"
