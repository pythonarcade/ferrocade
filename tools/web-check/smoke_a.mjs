// SPIKE: run spike/pyodide-wheel/smoke.html for a given wheel and print its console output.
import puppeteer from "puppeteer-core";
const wheel = process.argv[2];
const browser = await puppeteer.launch({
  executablePath: "C:/Program Files/Google/Chrome/Application/chrome.exe", headless: "new",
});
const page = await browser.newPage();
const lines = [];
page.on("console", (m) => lines.push(m.text()));
page.on("pageerror", (e) => lines.push(`PAGEERROR ${e.message}`));
await page.goto(`http://localhost:8000/spike/pyodide-wheel/smoke.html?wheel=${wheel}`);
const start = Date.now();
while (Date.now() - start < 120000 && !lines.some((l) => /A1-SMOKE-(OK|ERROR)|PAGEERROR/.test(l))) {
  await new Promise((r) => setTimeout(r, 250));
}
console.log(lines.join("\n"));
await browser.close();
