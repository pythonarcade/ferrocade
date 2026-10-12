// SPIKE: load the web example in headless Chrome/Edge and report what happened.
// Usage: node check.mjs [url] [--browser=chrome|edge] [--fresh]
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

const pick = (prefix) => console_.find((l) => l.includes(prefix)) ?? null;
const summary = {
  url, browser: browserName, outcome,
  backend: pick("backend="), timeToShip: pick("ship-visible"), centering: pick("centering"),
  exampleUnchanged, drawCalls, bytes, errorText: errorText || null, screenshot: shot,
  console: console_.slice(0, 40),
};
console.log(JSON.stringify(summary, null, 2));
await browser.close();
process.exit(outcome === "ship-visible" ? 0 : 1);
