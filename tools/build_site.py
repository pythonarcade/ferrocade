"""Assemble the static web site (the browser build plus the examples) into `_site/`.

The browser loader (`web/loader.js`) fetches files relative to `web/` (`../examples/...`,
`../assets/...`, `../python/ferrocade/color.py`), so the site keeps the repository's layout.
GitHub Pages publishes `_site/` (see `.github/workflows/pages.yml`).

Build the engine first (`web/pkg`), then run from the repository root:
    python tools/build_site.py
    python -m http.server 8000 --directory _site      # preview at http://localhost:8000/
"""

import html
import shutil
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SITE = ROOT / "_site"

# Example file, title, and a one-line description for the landing page.
EXAMPLES = [
    ("06_showcase", "Performance showcase",
     "Thousands of balls bouncing off the walls and each other. Press = / - to add or remove "
     "1,000 balls, T to switch bouncing between the engine and Python, Space to pause. "
     "Stats print to the developer console."),
    ("05_collisions", "Collisions",
     "Steer the ship (arrow keys or WASD) through drifting asteroids; the ones you touch "
     "disappear. Click the game first."),
    ("04_keyboard_ship", "Keyboard control",
     "Steer the ship with the arrow keys or WASD; Space re-centers it. Click the game first."),
    ("02_bouncing_ship", "Bouncing ship", "A ship that moves, spins, and bounces off the edges."),
    ("01c_shapes", "Colors and shapes", "Rectangles and circles in CSS named colors, plus tinting."),
    ("01_centered_ship", "Centered ship", "The first milestone: a window with the ship centered."),
]

FILES = [
    "web/index.html",
    "web/loader.js",
    "web/python/ferrocade/__init__.py",
    "python/ferrocade/color.py",
    "python/ferrocade/key.py",
]


def landing_page():
    items = "\n".join(
        f"""    <li>
      <a class="title" href="web/?example={name}">{html.escape(title)}</a>
      <span class="alt">(<a href="web/?example={name}&amp;backend=webgl2">WebGL2</a>,
        <a href="https://github.com/pythonarcade/ferrocade/blob/main/examples/{name}.py">source</a>)</span>
      <p>{html.escape(text)}</p>
    </li>""" for name, title, text in EXAMPLES)
    return f"""<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Ferrocade examples</title>
  <link rel="icon" href="data:,">
  <style>
    body {{ margin: 0 auto; max-width: 760px; padding: 24px 16px; background: #111; color: #ddd;
           font: 16px/1.5 system-ui, sans-serif; }}
    a {{ color: #8cc8ff; }}
    h1 {{ margin-bottom: 4px; }}
    ul {{ list-style: none; padding: 0; }}
    li {{ margin: 18px 0; }}
    .title {{ font-size: 18px; font-weight: 600; }}
    .alt {{ font-size: 14px; opacity: 0.8; }}
    li p {{ margin: 4px 0 0; opacity: 0.9; }}
    .note {{ font-size: 14px; opacity: 0.75; }}
  </style>
</head>
<body>
  <h1>Ferrocade examples</h1>
  <p>Ferrocade is a 2D game engine: games are written in Python, and the engine is Rust. These
  examples run the same Python files as on the desktop, in your browser (Python via Pyodide,
  the engine via WebAssembly).</p>
  <ul>
{items}
  </ul>
  <p class="note">The first load downloads about 10 MB (mostly the Python runtime). WebGPU is used
  when available; the WebGL2 links force the fallback renderer.
  <a href="https://github.com/pythonarcade/ferrocade">Source on GitHub</a>.</p>
</body>
</html>
"""


def main():
    pkg = ROOT / "web" / "pkg"
    if not (pkg / "ferrocade_web_bg.wasm").exists():
        print("web/pkg is missing: build the engine first (see tools/build_web.ps1)")
        return 1
    if SITE.exists():
        shutil.rmtree(SITE)
    for rel in FILES:
        dest = SITE / rel
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(ROOT / rel, dest)
    shutil.copytree(pkg, SITE / "web" / "pkg", ignore=shutil.ignore_patterns(".gitignore"))
    shutil.copytree(ROOT / "assets", SITE / "assets", ignore=shutil.ignore_patterns("*.md"))
    (SITE / "examples").mkdir()
    for name, _, _ in EXAMPLES:
        shutil.copy2(ROOT / "examples" / f"{name}.py", SITE / "examples" / f"{name}.py")
    (SITE / "index.html").write_text(landing_page(), encoding="utf-8")
    (SITE / ".nojekyll").write_text("", encoding="utf-8")  # serve files as-is
    count = sum(1 for p in SITE.rglob("*") if p.is_file())
    print(f"Built {SITE} ({count} files)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
