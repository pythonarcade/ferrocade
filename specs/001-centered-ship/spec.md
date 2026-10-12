# Feature Specification: Phase 1 — Window and Centered Ship

**Feature Branch**: `001-centered-ship`

**Created**: 2026-10-12

**Status**: Draft

**Input**: User description: "Implement Phase 1 of the prototype."

**Context**: Phase 1 of the Ferrocade roadmap (`docs/spec/ENGINE_SPEC.md` §10). The
goal is to open a window and draw `ship.png` in the center of the screen. The class
design, coordinate system, and API style are already decided in engine spec §3. This
document states what Phase 1 must deliver and how to verify it.

## User Scenarios & Testing *(mandatory)*

The "user" throughout is a **game developer** writing a game in Python with Ferrocade.

### User Story 1 - Open a game window (Priority: P1)

A game developer writes a few lines of Python. They define a game class based on the
engine's window, give it a size and a title, and start it. A window appears with an
empty, cleared canvas and stays open until the developer closes it.

**Why this priority**: every later feature draws into this window. It proves that the
whole stack works, from Python code to pixels on screen, and that the engine can be
installed and imported.

**Independent Test**: run a script that only creates and runs a window subclass that
clears the screen. A window with the requested size, title, and background color
appears, and closing it ends the program cleanly.

**Acceptance Scenarios**:

1. **Given** the engine is installed in a Python environment, **When** the developer
   runs a script that creates an 800×600 window titled "Ferrocade" and starts it,
   **Then** a window of that size and title appears, showing a solid background color.
2. **Given** a running window, **When** the developer clicks the window's close button,
   **Then** the window closes and the script finishes without errors.
3. **Given** a window subclass that overrides the draw callback to clear the screen to
   a chosen color, **When** the window runs, **Then** the canvas shows that color, and
   the draw callback is called repeatedly (once per displayed frame).
4. **Given** a window subclass that overrides the update callback, **When** the window
   runs, **Then** the update callback is called repeatedly with the elapsed time step.

---

### User Story 2 - Draw an image centered on screen (Priority: P2)

The developer loads `ship.png`, creates a sprite from it positioned at the center of
the window, adds the sprite to a sprite list, and draws the list in the draw callback.
The ship appears in the exact center of the window, with its transparent areas showing
the background.

**Why this priority**: this is the Phase 1 milestone. It proves that images load,
that sprites position correctly in the engine's coordinate system, and that batched
drawing works.

**Independent Test**: run the Phase 1 example (`examples/01_centered_ship.py`). The ship
appears centered, at its natural size, with correct transparency.

**Acceptance Scenarios**:

1. **Given** an 800×600 window and a sprite created from `ship.png` with its center at
   (400, 300), **When** the sprite list is drawn, **Then** the ship's center is at the
   center of the window, and the ship is shown at the image's original pixel size.
2. **Given** `ship.png` has transparent regions, **When** the ship is drawn over a
   colored background, **Then** the background shows through the transparent regions.
3. **Given** a sprite created directly from a file path (without creating an image
   object first), **When** it is drawn, **Then** the result is identical to a sprite
   created from an explicitly loaded image.
4. **Given** a sprite positioned at (0, 0), **When** drawn, **Then** its center is at
   the bottom-left corner of the window (bottom-left origin, y up).
5. **Given** a window created with nearest filtering (as in the Phase 1 example),
   **When** the ship is drawn, **Then** its pixel edges are crisp (not blurred).

---

### User Story 3 - Reuse images across sprites and control placement (Priority: P3)

The developer creates several sprites that share one loaded image and gives each its
own position and scale. They can read each sprite's size and change its position after
creation, and the display reflects the changes.

**Why this priority**: not required for the centered-ship demo, but it validates the
core design choices (image sharing, live position updates) before Phase 2 builds
movement on top of them.

**Independent Test**: create 10 sprites from the same image at different positions and
scales, draw them, then move one. All 10 sprites appear correctly, the image is loaded
only once, and the moved sprite shows at its new location on the next frame.

**Acceptance Scenarios**:

1. **Given** one loaded image used by 10 sprites, **When** they are drawn, **Then** all
   10 appear, and the image file was read only once.
2. **Given** the same image file path used twice to load an image, **When** both loads
   complete, **Then** they refer to the same shared image.
3. **Given** a sprite with scale 2.0, **When** its width and height are read, **Then**
   they are twice the image's pixel dimensions, and it is drawn at twice the size.
4. **Given** a drawn sprite, **When** its position is changed (one coordinate at a
   time, or both together), **Then** the next displayed frame shows it at the new
   position.
5. **Given** sprites in one list, **When** drawn, **Then** sprites added later appear on
   top of sprites added earlier.

---

### Edge Cases

- **Missing image file:** loading a path that doesn't exist raises a clear Python error
  that names the file. The engine does not crash.
- **Unsupported or corrupt image:** raises a clear Python error naming the file and the
  problem. The engine does not crash.
- **Image loaded before the window exists:** allowed. The image becomes drawable once
  the window is running.
- **Window resized:** the engine keeps running without crashing. Coordinates stay in
  pixels with the origin at the bottom-left of the new size. Sprites keep their
  coordinates and are not stretched.
- **Window minimized and restored:** the engine does not crash or freeze. Drawing
  resumes when the window is restored.
- **Error inside a developer callback:** an exception raised in the draw or update
  callback stops the window and propagates to the developer's script with its normal
  Python traceback. It is not silently swallowed.
- **No filtering choice:** creating a window without choosing a filter mode fails
  immediately, with an error saying that a filter mode is required. There is no
  silent default.
- **Empty sprite list:** drawing a list with no sprites does nothing and raises no error.
- **Removing a sprite:** a sprite removed from a list no longer appears on the next frame.
- **Display scaling:** on displays with OS scaling (e.g. 150%), window size and sprite
  coordinates are in logical pixels, so an 800×600 window looks the same relative size
  as on a 100% display, and centering stays correct.

## Requirements *(mandatory)*

### Functional Requirements

**Window**

- **FR-001**: Developers MUST be able to create a game window by subclassing the
  engine's window class, giving a width, height, and title.
- **FR-002**: The window MUST expose its current width and height.
- **FR-003**: Starting the window MUST show it and run the game loop until the window
  is closed. The script then continues past the start call.
- **FR-004**: The window MUST call the developer's draw callback once per displayed
  frame, and an update callback repeatedly at a fixed rate (configurable, default
  60 times per second), passing the time step in seconds. Phase 1 must deliver
  working callbacks. Precise timing guarantees are verified in Phase 2.
- **FR-005**: Developers MUST be able to clear the canvas to a chosen color from the
  draw callback. A default color is used when none is given.
- **FR-006**: Closing the window (close button or OS equivalent) MUST end the game loop
  cleanly, with no error output.

**Images (textures)**

- **FR-007**: Developers MUST be able to load an image from a file path into a reusable
  image object that exposes its pixel width, height, and source path.
- **FR-008**: Loading the same file path more than once MUST return the same shared
  image rather than loading it again.
- **FR-009**: PNG files, including transparency, MUST be supported. JPEG, BMP, and the
  first frame of GIF files SHOULD also load (engine spec §3, "Image formats").
- **FR-010**: Missing, unsupported, or corrupt image files MUST raise a Python exception
  whose message names the file and the reason.
- **FR-011**: Images MUST be loadable before the window is created.

**Sprites**

- **FR-012**: Developers MUST be able to create a sprite from either an image object or
  a file path, with an optional center position and scale.
- **FR-013**: A sprite MUST expose readable and writable center x, center y, combined
  position (both coordinates set in one operation), scale, and image. It MUST also
  expose read-only width and height (image size × scale).
- **FR-014**: Sprite positions MUST use pixel coordinates with the origin at the
  bottom-left of the window and y increasing upward. The position refers to the
  sprite's center.
- **FR-015**: Changes to a sprite's position, scale, or image MUST appear on the next
  displayed frame.

**Sprite lists**

- **FR-016**: Developers MUST be able to create a sprite list and append, remove, and
  count sprites in it.
- **FR-017**: Drawing a sprite list MUST draw all its sprites with alpha transparency,
  in list order (later sprites on top).
- **FR-018**: A sprite MUST be able to belong to more than one sprite list at the same
  time.

**Delivery**

- **FR-019**: The engine MUST be buildable and installable from source into a Python
  environment on Windows, with documented commands.
- **FR-020**: The repository MUST include a runnable example, `examples/01_centered_ship.py`,
  that opens a window and draws `ship.png` centered. It MUST work when run from the
  repository root.
- **FR-021**: The public Python API MUST include type information so that editors can
  offer autocompletion for the window, image, sprite, and sprite list types.

**Image filtering**

- **FR-022**: Developers MUST choose an image filtering mode, either **nearest** (crisp
  pixels) or **linear** (smooth), when creating a window. There is no default, and
  leaving it out is an error. The chosen mode applies to every image that doesn't
  override it.
- **FR-023**: Developers MUST be able to override the filtering mode for an individual
  image when loading it. Overriding the filter MUST NOT cause the image file to be read
  again.

### Key Entities *(include if feature involves data)*

- **Window**: the game's on-screen canvas and the owner of the game loop. Attributes:
  size, title, update rate, filtering mode (required). Behaviors: draw and update callbacks, clear, run.
- **Texture (image)**: a loaded picture that is shared and reusable. Attributes: pixel
  width, height, source path, optional filtering override. The pixel data is loaded once
  per distinct file path.
- **Sprite**: something placed on screen. Attributes: texture, center position, scale,
  derived width and height. It references exactly one texture, and many sprites can
  share one texture.
- **Sprite list**: an ordered collection of sprites that is drawn together. A sprite can
  be in several lists, and list order defines layering.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A developer with the engine installed can open a window and display a
  centered image with a script of 20 lines or fewer, matching the example in engine
  spec §3.
- **SC-002**: Running the Phase 1 example shows the ship centered to within 1 pixel
  horizontally and vertically, at the image's original size, with correct
  transparency.
- **SC-003**: The window appears within 2 seconds of starting the example on the
  development machine.
- **SC-004**: Closing the window ends the program within 1 second, with a successful
  exit status and no error output.
- **SC-005**: Resizing the window 20 times, and minimizing and restoring it 10 times,
  causes no crash, freeze, or error output.
- **SC-006**: Every attempt to load a missing or invalid image file produces an error
  message that names the file (verified by automated tests).
- **SC-007**: An unchanging scene (the centered ship) keeps displaying smoothly at the
  monitor's refresh rate, with no visible flicker or tearing.
- **SC-008**: Loading the same image file for 1,000 sprites reads the file from disk
  exactly once.
- **SC-009**: A new developer can build, install, and run the example by following the
  documented steps, without help.

## Assumptions

- **Platform:** Phase 1 targets Windows only (the prototype platform). The macOS check
  and the browser feasibility spike are separate work (Phase 1b). Phase 1 must still
  follow the constitution's portability rules so those ports stay possible.
- **Out of scope:** motion and velocities (Phase 2), rotation rendering (the angle
  property may exist but need not rotate the image yet), keyboard and mouse input
  (Phase 4), collisions (Phase 5), text, sound, cameras, and views.
- **Display scaling:** window and sprite units are logical pixels, so the engine handles
  OS display scaling (common on Windows laptops) transparently.
- **Resizing:** resizing does not re-center or stretch content. Re-centering after a
  resize is the developer's responsibility (via window width and height). Resize
  callbacks are not part of Phase 1.
- **Errors in callbacks:** exceptions propagate to the developer's script, matching
  normal Python expectations, instead of being logged and ignored.
- **Distribution:** installation from source is sufficient. Published packages
  (PyPI wheels) come later.
- **Asset paths:** relative image paths are resolved from the current working directory.
  The example is run from the repository root and uses `assets/images/ship.png`.
- **Dependencies:** the native build toolchain (compiler and C++ build tools) is
  installed on the development machine (verified 2026-10-12). Python 3.13 is the development version
  and the **minimum supported version** (backward compatibility isn't a goal at this
  stage). See engine spec §6 for the rationale.
