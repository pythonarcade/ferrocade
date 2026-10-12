// SPIKE: load the web example in headless Chrome/Edge and report what happened.
// Usage: node check.mjs [url] [--browser=chrome|edge] [--fresh] [--motion] [--keys] [--collisions]
// Prints a JSON summary (backend, time-to-ship, centering, bytes by kind) and saves a screenshot.
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import puppeteer from "puppeteer-core";

const args = process.argv.slice(2);
const url = args.find((a) => !a.startsWith("--")) ?? "http://localhost:8000/web/?diagnostics=1";
const browserName = (args.find((a) => a.startsWith("--browser=")) ?? "--browser=chrome").split("=")[1];
const executable = {
  chrome: "C:/Program Files/Google/Chrome/Application/chrome.exe",
  edge: "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe",
}[browserName];
const outDir = path.join(path.dirname(new URL(import.meta.url).pathname).replace(/^\/(\w:)/, "$1"), "out");
fs.mkdirSync(outDir, { recursive: true });
// A persistent profile gives a warm cache across runs; --fresh uses a new one (cold).
const profile = args.includes("--fresh")
  ? fs.mkdtempSync(path.join(os.tmpdir(), "ferrocade-check-"))
  : path.join(outDir, `profile-${browserName}`);

const browser = await puppeteer.launch({
  executablePath: executable,
  headless: "new",
  userDataDir: profile,
  args: ["--enable-unsafe-webgpu", "--enable-features=Vulkan", "--use-angle=d3d11", "--ignore-gpu-blocklist"],
});
const page = await browser.newPage();
await page.setViewport({ width: 1000, height: 800 });
const console_ = [];
page.on("console", (m) => console_.push(m.text()));
page.on("pageerror", (e) => console_.push(`PAGEERROR ${e.message}`));

const bytes = { pyodide: 0, engine: 0, assets: 0, other: 0 };
const cdp = await page.createCDPSession();
await cdp.send("Network.enable");
const urls = new Map();
cdp.on("Network.responseReceived", (e) => urls.set(e.requestId, e.response.url));
cdp.on("Network.loadingFinished", (e) => {
  const u = urls.get(e.requestId) ?? "";
  const kind = u.includes("pyodide") ? "pyodide" : u.includes("/pkg/") ? "engine"
    : u.includes("/assets/") ? "assets" : "other";
  bytes[kind] += e.encodedDataLength;
});

const started = Date.now();
await page.goto(url, { waitUntil: "load" });
let outcome = "timeout";
while (Date.now() - started < 90000) {
  if (console_.some((l) => l.includes("ship-visible"))) { outcome = "ship-visible"; break; }
  if (console_.some((l) => l.includes("ERROR") || l.startsWith("PAGEERROR"))) { outcome = "error"; break; }
  await new Promise((r) => setTimeout(r, 250));
}
await new Promise((r) => setTimeout(r, 1000)); // let a few more frames render
const shot = path.join(outDir, `${browserName}-${new URL(url).search.replace(/\W+/g, "_") || "default"}.png`);
await page.screenshot({ path: shot });
const exampleUnchanged = await page.evaluate(async () => {
  const name = new URLSearchParams(location.search).get("example") ?? "01_centered_ship";
  const original = await (await fetch(`../examples/${name}.py`, { cache: "no-store" })).text();
  return window.ferrocadeExampleSource === original;
});
const errorText = await page.$eval("#error", (e) => e.textContent);
const drawCalls = await page.evaluate(() => window.ferrocadeEngine?.draw_calls?.() ?? null);
// --motion: read the first sprite's position twice, 1 s apart, to prove the game loop moves it.
let motion = null;
if (args.includes("--motion")) {
  const read = () => page.evaluate(() => Array.from(window.ferrocadeEngine.get_sprite(0)).slice(0, 2));
  const before = await read();
  await new Promise((r) => setTimeout(r, 1000));
  const after = await read();
  const canvas = await page.evaluate(() => [parseFloat(document.getElementById("game").style.width),
                                            parseFloat(document.getElementById("game").style.height)]);
  const moved = Math.hypot(after[0] - before[0], after[1] - before[1]);
  const inside = after[0] >= 0 && after[0] <= canvas[0] && after[1] >= 0 && after[1] <= canvas[1];
  motion = { before, after, moved: Math.round(moved), inside };
  if (!(moved > 50 && inside)) outcome = "motion-failed";
}

// --keys: hold ArrowRight on the focused canvas; the first sprite must move right (and not up or
// down), and the page must not scroll even though it is taller than the viewport.
let keys = null;
if (args.includes("--keys")) {
  await page.evaluate(() => { document.body.style.minHeight = "3000px"; });
  await page.focus("#game");
  const read = () => page.evaluate(() => Array.from(window.ferrocadeEngine.get_sprite(0)).slice(0, 2));
  const before = await read();
  await page.keyboard.down("ArrowRight");
  await new Promise((r) => setTimeout(r, 1000));
  const during = await read();
  await page.keyboard.up("ArrowRight");
  await new Promise((r) => setTimeout(r, 300));
  const after = await read();
  await new Promise((r) => setTimeout(r, 300));
  const stopped = await read();
  const scrollY = await page.evaluate(() => window.scrollY);
  keys = { before, during, after, stopped, scrollY,
           dx: Math.round(during[0] - before[0]), dy: Math.round(during[1] - before[1]) };
  const ok = keys.dx > 100 && Math.abs(during[1] - before[1]) < 1 && scrollY === 0
    && after[0] === stopped[0];
  if (!ok) outcome = "keys-failed";
}

// --collisions: fly right for 2 s through the 05_collisions asteroid field; the hit count in the
// page title must go up.
let collisions = null;
if (args.includes("--collisions")) {
  await page.focus("#game");
  const hits = () => page.evaluate(() => Number((document.title.match(/hits: (\d+)/) ?? [0, -1])[1]));
  const before = await hits();
  await page.keyboard.down("ArrowRight");
  await new Promise((r) => setTimeout(r, 2000));
  await page.keyboard.up("ArrowRight");
  await new Promise((r) => setTimeout(r, 300));
  const after = await hits();
  collisions = { before, after, title: await page.evaluate(() => document.title) };
  if (!(before >= 0 && after > before)) outcome = "collisions-failed";
}

const pick = (prefix) => console_.find((l) => l.includes(prefix)) ?? null;
const summary = {
  url, browser: browserName, outcome,
  backend: pick("backend="), timeToShip: pick("ship-visible"), centering: pick("centering"),
  exampleUnchanged, drawCalls, motion, keys, collisions, bytes, errorText: errorText || null, screenshot: shot,
  console: console_.slice(0, 40),
};
console.log(JSON.stringify(summary, null, 2));
await browser.close();
process.exit(outcome === "ship-visible" ? 0 : 1);
