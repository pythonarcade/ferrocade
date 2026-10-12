# Feature Specification: Phase 1b — Web Feasibility Spike

**Feature Branch**: `002-web-feasibility-spike`

**Created**: 2026-10-12

**Status**: Draft

**Input**: User description: "Phase 1b web feasibility spike: run the Phase 1 centered-ship example (examples/01_centered_ship.py) in a web browser, unchanged or nearly unchanged, using a Python-in-browser runtime. Determine which approach works (engine spec §9 options: A Pyodide + engine built as a Pyodide wheel, B Pyodide + separate engine WASM module, C RustPython). Deliverable is a decision backed by evidence: a minimal web/ loader if it works, and docs/research/web-spike.md documenting what was tried, what worked, what failed, and the recommended path. Windows remains the dev machine."

**Context**: This is roadmap Phase 1b (`docs/spec/ENGINE_SPEC.md` §10). The web is a required
platform (constitution III), but Ferrocade games are written in Python, so a browser has to run
a Python runtime alongside the engine. Engine spec §9 lists three candidate approaches
(referred to here as **A**, **B**, and **C**) and leans toward A. This is a *spike*: its main
output is a **decision backed by evidence**, not a shipped feature. It runs before the engine
grows, so any architecture changes it reveals stay cheap.

## User Scenarios & Testing *(mandatory)*

There are two kinds of users:
- The **project owner**, who needs a sound decision on how Ferrocade will run on the web.
- **Game developers**, who eventually want their unchanged games to run in a browser.

### User Story 1 - A web strategy decision backed by evidence (Priority: P1)

The project owner reads one research report. It explains which browser approaches were tried,
how far each one got, what blocked the ones that failed, and what each viable one costs
(download size, load time, maintenance burden, and the impact on the engine's design). It ends
with a clear recommendation, which the owner can accept and record in the engine spec.

**Why this priority**: the decision is the point of the spike. Even if nothing runs in a browser
yet, a well-evidenced "not this way, try that" is a successful outcome.

**Independent Test**: the owner reads the report alone (without asking the implementer) and can
state the recommended approach, the evidence for it, the known risks, and the next steps.

**Acceptance Scenarios**:

1. **Given** the spike is complete, **When** the owner reads the report, **Then** every
   candidate approach (A, B, C) has a status: *proven working*, *blocked* (with the specific
   blocker), or *not attempted* (with the reason).
2. **Given** an approach was attempted, **When** the report describes it, **Then** it includes
   the steps taken and the concrete results observed (errors, measurements, screenshots or
   console output). It does not rely on speculation alone.
3. **Given** the report's recommendation, **When** the owner accepts it, **Then** the engine
   spec's web section and decision log can be updated directly from the report's conclusions.

---

### User Story 2 - The centered ship runs in a browser (Priority: P2)

A game developer opens a local web page and, after a loading period, sees the Phase 1 example:
the ship in the center of the canvas, crisp, with transparency, exactly as on the desktop. The
game code is the same Python file used on the desktop, or differs only in small, documented
ways.

**Why this priority**: it is the strongest possible evidence for the decision, and it proves
that the engine's architecture (Python API over a portable core) holds up on the web.

**Independent Test**: start the local preview, open it in a browser, and compare the result with
the desktop example.

**Acceptance Scenarios**:

1. **Given** the web build and local preview are running, **When** the developer opens the page
   in a current Chromium-based browser on Windows, **Then** a canvas shows a dark gray background
   with the ship centered at its natural size and crisp edges.
2. **Given** the browser example, **When** compared with `examples/01_centered_ship.py`,
   **Then** the game code is identical, or every difference is listed in the report with the
   reason it was needed.
3. **Given** the page is loading, **When** the runtime and engine are still downloading,
   **Then** the page shows a loading indication rather than a blank or frozen page.
4. **Given** a loading or runtime failure, **When** it happens, **Then** the page shows a
   readable error message instead of failing silently.

---

### User Story 3 - A repeatable local web build on Windows (Priority: P3)

The project owner, on the Windows development machine, can rebuild the web version and launch the
local preview by following documented steps. This makes the result reproducible and gives later
phases a starting point.

**Why this priority**: a one-off success that can't be reproduced is weak evidence, and later
phases will need this workflow.

**Independent Test**: on the development machine, follow the documented steps from a clean
checkout. The browser example appears without undocumented steps.

**Acceptance Scenarios**:

1. **Given** the documented prerequisites are installed, **When** the owner runs the documented
   build and preview steps, **Then** the browser example appears.
2. **Given** a prerequisite tool can't run natively on Windows, **When** the steps require it,
   **Then** the documentation names the tool, explains why it's needed, and gives the supported
   workaround (for example, a Linux environment on the same machine).

---

### Edge Cases

- **Approach A is blocked early:** the spike continues with B. If B is also blocked, C is
  assessed. The report documents each blocker precisely enough that a future attempt can tell
  whether it has been resolved.
- **No modern graphics API in the browser:** if the browser lacks its newest graphics API,
  the report records whether the older fallback path works. It is acceptable if only one of the
  two is proven, as long as this is stated.
- **The example needs changes:** for example, asset loading may need to work differently
  because browsers have no filesystem. Each change is minimal and documented, and is classified
  as either a temporary spike shortcut or a needed engine change.
- **The game loop can't block in a browser:** if the desktop "run until closed" behavior can't
  work as-is, the report states what the developer-facing behavior becomes on the web.
- **Desktop regression:** spike changes to the engine must not break the desktop build or its
  tests.
- **Large download:** if the runtime makes the first load very large or slow, the report
  quantifies it, and that counts as a finding, not a failure.

## Requirements *(mandatory)*

### Functional Requirements

**Investigation**

- **FR-001**: The spike MUST attempt approach **A** first (the engine spec's current leaning),
  either to a working browser example or to a precisely documented blocker.
- **FR-002**: If A is blocked, the spike MUST attempt approach **B** the same way. If both are
  blocked, it MUST assess approach **C** (at least a documented feasibility assessment, and a
  hands-on attempt if practical).
- **FR-003**: If A succeeds, B and C MUST still receive a written comparison in the report
  (strengths, weaknesses, and why they weren't needed). Hands-on attempts at them are optional.
- **FR-004**: The spike MUST record, for each attempted approach:
  - the versions of the runtime and tools used
  - the steps taken
  - the results observed
  - the blockers hit, if any

**Browser example (if any approach succeeds)**

- **FR-005**: The repository MUST include a minimal web loader (in `web/`) that loads the
  Python runtime and the engine, runs the centered-ship game, and draws it on a page canvas.
- **FR-006**: The browser example MUST render the same scene as the desktop example: a dark
  gray background, the ship centered at its natural size, crisp (nearest filtering), with
  correct transparency.
- **FR-007**: The page MUST show a loading indication during startup and a readable error
  message if startup or the game fails.
- **FR-008**: Any difference between the browser game code and `examples/01_centered_ship.py`
  MUST be documented with its reason.

**Measurements**

- **FR-009**: For each working approach, the report MUST state:
  - the total first-load download size
  - the time from opening the page to the ship appearing (cold and warm cache)
  - which browser graphics paths were verified

**Engine impact**

- **FR-010**: The report MUST list every engine or binding change the chosen approach would
  require (for example, how files are loaded, how the game loop starts, and how windowing
  works). Each item is classified as *done in the spike*, *required before shipping web
  support*, or *nice to have*.
- **FR-011**: The desktop build and all existing automated tests MUST still pass after any
  engine changes made during the spike.
- **FR-012**: Spike-only shortcuts MUST be clearly marked as such in the code and the report, so
  they aren't mistaken for finished engine work.

**Decision and records**

- **FR-013**: The report (`docs/research/web-spike.md`) MUST end with a single recommended
  approach (or "none viable yet", with what would change that), the key risks, and concrete
  next steps.
- **FR-014**: The build and preview steps MUST be documented so they are reproducible on the
  Windows development machine, including any workaround needed for tools that require another
  environment.
- **FR-015**: The engine spec MUST NOT record the web approach as decided until the owner
  accepts the recommendation (constitution V). Until then, the report's recommendation is
  marked *Proposed*.

### Key Entities *(include if feature involves data)*

- **Candidate approach**: one of A, B, or C. Attributes: description, status (proven, blocked,
  or not attempted), evidence, measurements, blockers, engine impact.
- **Spike report**: the decision document. Summary, per-approach findings, comparison,
  recommendation, risks, next steps.
- **Web loader**: the minimal page and startup logic that host the runtime, the engine, and the
  game on a canvas.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: The report gives each of the 3 candidate approaches an explicit status and its
  evidence, so no approach is left without a verdict.
- **SC-002**: The owner can reach and record a web-strategy decision from the report alone, in
  one reading, without extra investigation.
- **SC-003**: If any approach works, the ship appears centered (within 1 pixel, at its natural
  size) in a current Chromium-based browser on the development machine.
- **SC-004**: If any approach works, the browser game code differs from the desktop example by
  no more than 5 lines, and every changed line is justified in the report (0 lines is the goal).
- **SC-005**: If any approach works, the ship appears within 30 seconds of opening the page on
  a cold cache and within 10 seconds on a warm cache, measured on the development machine over
  a local server. Longer times are recorded as a finding.
- **SC-006**: Following the documented steps from a clean checkout reproduces the browser
  example on the development machine without undocumented steps.
- **SC-007**: After the spike, 100% of the existing desktop tests still pass.

## Assumptions

- **Time-boxed:** this is a spike. Code quality may be prototype-level where marked (FR-012),
  but conclusions must rest on evidence.
- **Browsers:** a current Chromium-based browser (Edge or Chrome) on Windows is the primary
  target. Checking other browsers is a bonus, not a requirement.
- **Scope:** only the Phase 1 scene. Input, audio, multiple windows, publishing a package for
  the browser runtime, and production hosting are out of scope.
- **Build environment:** some browser build tools may not support Windows natively. A Linux
  environment on the same machine is acceptable if the steps are documented (FR-014).
- **Serving:** the preview runs on a local web server. Opening the HTML file directly from disk
  isn't required.
- **Decision authority:** the owner makes the final call. The spike recommends (constitution V).
- **Existing design constraints still apply:** WebGL2-level limits, a non-blocking main loop,
  async-friendly GPU init, and file access through the asset loader (constitution III). The
  spike tests whether they were enough.
