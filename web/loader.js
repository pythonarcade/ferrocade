// SPIKE: Phase 1b web loader (approach B).
// Loads Pyodide and the Ferrocade engine (WebAssembly), copies assets and the browser
// `ferrocade` package into Pyodide's filesystem, then runs examples/01_centered_ship.py
// unchanged. Serve the repository root and open /web/.
performance.mark("start");

const PYODIDE_VERSION = "314.0.7";
const params = new URLSearchParams(location.search);
const backend = params.get("backend") ?? "auto";
const diagnostics = params.has("diagnostics");
// Which example to run (letters, digits, underscore only).
const example = /^[A-Za-z0-9_]+$/.test(params.get("example") ?? "")
  ? params.get("example") : "01_centered_ship";
const statusEl = document.getElementById("status");
const errorEl = document.getElementById("error");
const canvas = document.getElementById("game");

function setStatus(text) {
  statusEl.textContent = text;
  console.log(`[ferrocade] ${text}`);
}

function showError(text) {
  errorEl.textContent = text;
  errorEl.style.display = "block";
  statusEl.textContent = "Error";
  console.error(`[ferrocade] ERROR\n${text}`);
}
window.ferrocadeReportError = showError;

async function fetchBytes(url) {
  const response = await fetch(url);
  if (!response.ok) throw new Error(`could not load ${url}: HTTP ${response.status}`);
  return new Uint8Array(await response.arrayBuffer());
}

function writeFile(pyodide, path, bytes) {
  const dir = path.split("/").slice(0, -1).join("/");
  if (dir) pyodide.FS.mkdirTree(dir);
  pyodide.FS.writeFile(path, bytes);
}

// Count opaque-ish non-background pixels to find the ship's bounding box (W4).
function measureCentering() {
  const copy = document.createElement("canvas");
  copy.width = canvas.width;
  copy.height = canvas.height;
  const ctx = copy.getContext("2d");
  ctx.drawImage(canvas, 0, 0);
  const { data, width, height } = ctx.getImageData(0, 0, copy.width, copy.height);
  let x0 = width, y0 = height, x1 = -1, y1 = -1;
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const i = (y * width + x) * 4;
      if (data[i] !== 32 || data[i + 1] !== 32 || data[i + 2] !== 32) {
        if (x < x0) x0 = x; if (x > x1) x1 = x;
        if (y < y0) y0 = y; if (y > y1) y1 = y;
      }
    }
  }
  const scale = canvas.width / parseFloat(canvas.style.width || canvas.width);
  const result = {
    canvas: [width, height], devicePixelRatio: scale,
    bbox: [x0, y0, x1, y1],
    size: [(x1 - x0 + 1) / scale, (y1 - y0 + 1) / scale],
    centerOffset: [((x0 + x1 + 1) / 2 - width / 2) / scale, ((y0 + y1 + 1) / 2 - height / 2) / scale],
  };
  console.log(`[ferrocade] centering ${JSON.stringify(result)}`);
  return result;
}

async function main() {
  setStatus("Loading Python runtime…");
  const { loadPyodide } = await import(
    `https://cdn.jsdelivr.net/pyodide/v${PYODIDE_VERSION}/full/pyodide.mjs`);
  const pyodide = await loadPyodide();

  setStatus("Starting graphics…");
  const wasm = await import("./pkg/ferrocade_web.js");
  await wasm.default();
  const engine = await wasm.Engine.create(canvas, backend);
  console.log(`[ferrocade] backend=${engine.backend_name} pyodide=${pyodide.version}`);

  // After the first rendered frame: record time-to-ship (and optionally check centering).
  const endFrame = engine.end_frame.bind(engine);
  let firstFrame = true;
  engine.end_frame = () => {
    endFrame();
    if (firstFrame) {
      firstFrame = false;
      performance.mark("ship-visible");
      const ms = performance.measure("time-to-ship", "start", "ship-visible").duration;
      console.log(`[ferrocade] ship-visible ms=${ms.toFixed(0)}`);
      setStatus(`Running (${engine.backend_name})`);
      if (diagnostics) window.ferrocadeCentering = measureCentering();
    }
  };
  window.ferrocadeEngine = engine;

  setStatus("Loading assets…");
  // SPIKE: hard-coded asset list; a real build needs an asset manifest or bundler.
  const assets = ["assets/images/ship.png"];
  for (const path of assets) writeFile(pyodide, path, await fetchBytes(`../${path}`));
  // The browser package: its own __init__.py, plus the desktop package's color.py (one source).
  writeFile(pyodide, "/home/pyodide/ferrocade/__init__.py",
            await fetchBytes("python/ferrocade/__init__.py"));
  writeFile(pyodide, "/home/pyodide/ferrocade/color.py",
            await fetchBytes("../python/ferrocade/color.py"));

  const source = new TextDecoder().decode(await fetchBytes(`../examples/${example}.py`));
  window.ferrocadeExampleSource = source; // lets the check script verify it is unchanged (W5)
  try {
    await pyodide.runPythonAsync(source, { filename: `examples/${example}.py` });
  } catch (err) {
    showError(String(err));
  }
}

main().catch((err) => showError(`${err?.message ?? err}`));
