#!/usr/bin/env bash
# SPIKE: one-time approach-A toolchain setup inside WSL2 (owner approved 2026-10-12).
# Installs into $HOME only: rustup (stable + wasm32-unknown-emscripten), emsdk 5.0.3,
# uv-managed Python 3.14 venv with pyodide-build.
set -euo pipefail
EMSCRIPTEN_VERSION=5.0.3

if ! command -v rustup >/dev/null 2>&1 && [ ! -x "$HOME/.cargo/bin/rustup" ]; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
fi
source "$HOME/.cargo/env"
rustup toolchain install stable --profile minimal
rustup target add wasm32-unknown-emscripten

if [ ! -d "$HOME/emsdk" ]; then
  git clone --depth 1 https://github.com/emscripten-core/emsdk.git "$HOME/emsdk"
fi
cd "$HOME/emsdk"
git pull --ff-only || true
./emsdk install "$EMSCRIPTEN_VERSION"
./emsdk activate "$EMSCRIPTEN_VERSION"

# Call uv by full path: a pyenv shim named `uv` may shadow it on PATH.
UV="$HOME/.local/bin/uv"
if [ ! -x "$UV" ]; then
  curl -LsSf https://astral.sh/uv/install.sh | env UV_NO_MODIFY_PATH=1 sh
fi
"$UV" venv --allow-existing --python 3.14 "$HOME/pyodide-venv"
"$UV" pip install --python "$HOME/pyodide-venv/bin/python" pyodide-build maturin

echo "=== versions"
rustc --version
"$HOME/emsdk/upstream/emscripten/emcc" --version | head -1
"$HOME/pyodide-venv/bin/python" --version
"$HOME/pyodide-venv/bin/pyodide" --version 2>&1 | head -3 || true
"$HOME/pyodide-venv/bin/maturin" --version
echo "=== SETUP DONE"
