// Verify fc.color against Chrome's own CSS engine: every CSS_COLORS keyword must be a valid
// CSS color whose computed value equals ours, and there must be exactly 148 of them.
// Usage (repo root):  python -c "import json, ferrocade.color as c; print(json.dumps(c.CSS_COLORS))" > colors.json
//                     node tools/web-check/check_colors.mjs colors.json
import fs from "node:fs";
import puppeteer from "puppeteer-core";

const colors = JSON.parse(fs.readFileSync(process.argv[2], "utf8"));
const browser = await puppeteer.launch({
  executablePath: "C:/Program Files/Google/Chrome/Application/chrome.exe", headless: "new",
});
const page = await browser.newPage();
const computed = await page.evaluate((keywords) => {
  const div = document.createElement("div");
  document.body.appendChild(div);
  const out = {};
  for (const k of keywords) {
    div.style.color = "";
    div.style.color = k;
    out[k] = div.style.color ? getComputedStyle(div).color : null; // null = not a CSS color
  }
  return out;
}, Object.keys(colors));
await browser.close();

const problems = [];
for (const [keyword, [r, g, b]] of Object.entries(colors)) {
  const expected = `rgb(${r}, ${g}, ${b})`;
  if (computed[keyword] === null) problems.push(`${keyword}: not a CSS color keyword`);
  else if (computed[keyword] !== expected) problems.push(`${keyword}: ours ${expected}, Chrome ${computed[keyword]}`);
}
const count = Object.keys(colors).length;
if (count !== 148) problems.push(`expected 148 keywords, got ${count}`);
console.log(problems.length ? problems.join("\n") : `OK: all ${count} CSS colors match Chrome`);
process.exit(problems.length ? 1 : 0);
