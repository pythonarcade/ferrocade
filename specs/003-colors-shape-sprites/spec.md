# Feature Specification: Phase 1c — CSS Colors and Shape Sprites

**Feature Branch**: `003-colors-shape-sprites`

**Created**: 2026-10-12

**Status**: Draft

**Input**: User description: "Phase 1c: CSS colors and shape sprites"

**Context**: This is roadmap Phase 1c (`docs/spec/ENGINE_SPEC.md` §10). The owner already decided
these points (engine spec §3, "Colors" and "Shape sprites"):
- **Colors:** `fc.color` provides every CSS named color, with UPPER_SNAKE names. CSS values win on
  conflicts (`GREEN` = (0,128,0)). The default background becomes `DEFAULT_BACKGROUND` (32,32,32).
- **Shapes:** sprites can be solid-color rectangles or circles with no image. Each sprite gets a
  color tint, where white means unchanged. Shapes share built-in textures, so they draw efficiently together.

This spec defines what users get and how to verify it. Two API questions the engine spec left open
are marked below.

## User Scenarios & Testing *(mandatory)*

The user is a **game developer** writing Python with Ferrocade.

### User Story 1 - Use any CSS color by name (Priority: P1)

A developer picks colors by familiar names, such as `fc.color.CORNFLOWER_BLUE` or
`fc.color.DARK_SLATE_GRAY`, for clearing the screen and (with US2) for shapes. Every color they
find on a CSS color chart exists under the expected name, with the same values.

**Why this priority**: it's the smallest independently useful piece, and US2 builds on it.

**Independent Test**: compare `fc.color` with the published list of CSS named colors. Clear a window
to a few named colors and check the displayed pixels.

**Acceptance Scenarios**:

1. **Given** the CSS named color `cornflowerblue` (100, 149, 237), **When** the developer reads
   `fc.color.CORNFLOWER_BLUE`, **Then** it is `(100, 149, 237)`.
2. **Given** every CSS named color, **When** each is looked up by its UPPER_SNAKE name, **Then**
   all of them exist with exactly the CSS values (including both the `GRAY` and `GREY` spellings).
3. **Given** `fc.color.GREEN`, **When** it's read, **Then** it is `(0, 128, 0)` (the CSS value), and
   `fc.color.LIME` is `(0, 255, 0)`.
4. **Given** a frame that doesn't clear explicitly, **When** it's drawn, **Then** the background is
   `fc.color.DEFAULT_BACKGROUND` (32, 32, 32), the same as Phase 1.
5. **Given** `fc.color.TRANSPARENT`, **When** it's read, **Then** it is `(0, 0, 0, 0)`.

---

### User Story 2 - Draw solid rectangles and circles without images (Priority: P2)

A developer creates a 50×20 red rectangle and a blue circle with radius 15, places them like any
sprite, adds them to a sprite list, and sees them drawn. They don't need any image files. Many
shapes of different sizes and colors still draw quickly.

**Why this priority**: it's the main new capability. It lets later phases (bouncing, collisions)
use simple shapes, and lets beginners make games without art.

**Independent Test**: run the Phase 1c example. Rectangles and circles of various sizes and colors
appear where expected, with correct sizes and colors, and without loading any image file.

**Acceptance Scenarios**:

1. **Given** a rectangle sprite with width 50, height 20 and color `RED`, centered at (100, 100),
   **When** drawn, **Then** a solid red 50×20 rectangle appears centered at (100, 100).
2. **Given** a circle sprite with radius 15 and color `BLUE` at (200, 100), **When** drawn,
   **Then** a solid blue disc 30 pixels across appears centered at (200, 100), with a round outline.
3. **Given** a shape with a semi-transparent color (alpha 128), **When** drawn over something,
   **Then** what's underneath shows through.
4. **Given** shape sprites, **When** their position or scale changes, **Then** they behave exactly
   like image sprites (same coordinates, layering in list order, and sprite-list membership).
5. **Given** 1,000 shapes of mixed sizes, colors, and kinds in one list, **When** drawn, **Then**
   the frame rate stays at the display's refresh rate on the development machine.
6. **Given** a game written for the browser build, **When** it creates the same shapes,
   **Then** they look the same as on desktop.

---

### User Story 3 - Tint and fade any sprite (Priority: P3)

A developer changes a sprite's color to tint it (for example, flashing an image sprite red when hit)
or lowers its alpha to fade it out. Changing a shape's color recolors the shape.

**Why this priority**: it comes almost free with the shape design and is commonly needed, but it
isn't required for the shapes themselves.

**Independent Test**: set an image sprite's color to red and to half transparent, and confirm how
it looks. Set it back to white, and confirm it's unchanged from the original.

**Acceptance Scenarios**:

1. **Given** an image sprite created normally, **When** its color is read, **Then** it is white
   `(255, 255, 255, 255)`, and the sprite looks exactly as in Phase 1.
2. **Given** an image sprite, **When** its color is set to `(255, 0, 0)`, **Then** it's drawn
   tinted red (color channels multiplied), and its transparent areas stay transparent.
3. **Given** any sprite, **When** its color alpha is set to 128, **Then** it's drawn at about half
   opacity.
4. **Given** a rectangle sprite, **When** its color changes from red to green, **Then** the next
   frame shows it green.

---

### Edge Cases

- **Invalid color values** (components outside 0-255, or the wrong number of components): rejected
  with a clear error, exactly like `clear()`.
- **Zero or negative shape size** (width, height, or radius ≤ 0, or non-finite): rejected with a
  clear error.
- **Very large or very small shapes** (e.g. 1×1, or 4000×4000): drawn correctly. Rectangles keep
  sharp edges at any size, and circles keep a round outline.
- **Scaling a shape**: a circle scaled by 2 is a circle with twice the diameter, and a 50×20
  rectangle scaled by 2 becomes 100×40.
- **Name collisions with Phase 1 constants**: the meaning of `GREEN` and `DARK_GRAY` changes (to CSS
  values). Phase 1 code that relied on the old values is updated (the default background constant
  becomes `DEFAULT_BACKGROUND`).
- **Mixing shapes and images in one list**: the drawing order is still exactly list order.
- **Circle edges with nearest filtering**: stepped by default, or smooth with `filter=fc.LINEAR` (FR-012). Rectangles look identical with either filter.

## Requirements *(mandatory)*

### Functional Requirements

**Colors**

- **FR-001**: `fc.color` MUST provide every CSS named color (CSS Color Module Level 4, 148 names),
  named as the UPPER_SNAKE form of the CSS keyword (e.g. `cornflowerblue` → `CORNFLOWER_BLUE`,
  `rebeccapurple` → `REBECCA_PURPLE`), with exactly the CSS values as RGB tuples of 0-255 integers.
- **FR-002**: Where CSS has both spellings, both names MUST exist (e.g. `DARK_GRAY` and `DARK_GREY`)
  with identical values.
- **FR-003**: `fc.color` MUST provide `TRANSPARENT = (0, 0, 0, 0)` and
  `DEFAULT_BACKGROUND = (32, 32, 32)`. The default clear color MUST remain (32, 32, 32).
- **FR-004**: Where an earlier constant conflicts with CSS, the CSS value MUST win (`GREEN` is
  (0, 128, 0), and `DARK_GRAY` is (169, 169, 169)).
- **FR-005**: The desktop and browser versions of `fc.color` MUST be identical (from one source).

**Sprite color (tint)**

- **FR-006**: Every sprite MUST have a readable and writable `color` (RGB or RGBA, 0-255). If alpha
  is omitted it defaults to 255. The default color for new image sprites is white
  `(255, 255, 255, 255)`. Reading `color` returns a 4-tuple.
- **FR-007**: A sprite MUST be drawn with its image's colors multiplied by `color` (each channel
  scaled by color/255, including alpha). White leaves the image unchanged.
- **FR-008**: Changing `color` MUST show on the next displayed frame, with the same efficiency rules
  as other sprite properties (only the changed sprite is updated).

**Shape sprites**

- **FR-009**: Developers MUST be able to create a **rectangle** sprite without providing an
  image, using a dedicated sprite type: `fc.RectangleSprite(width, height, color, center_x=0,
  center_y=0)`. It's a kind of `fc.Sprite`: it works everywhere a sprite does, and game classes can
  extend it (`class Wall(fc.RectangleSprite)` calling `super().__init__(...)`). *(Owner decision:
  dedicated classes, 2026-10-12.)*
- **FR-010**: Developers MUST be able to create a **circle** sprite without providing an image:
  `fc.CircleSprite(radius, color, center_x=0, center_y=0, *, filter=None)`. It's a kind of
  `fc.Sprite`, and extendable the same way (`class Ball(fc.CircleSprite)`).
- **FR-011**: A shape sprite's size MUST be its given width × height (or diameter × diameter) times
  its scale. Its `width` and `height` properties report this.
- **FR-012**: A circle's outline MUST look round at any size. Its edge follows the **window's
  filter** by default: crisp and stepped with `NEAREST`, smooth with `LINEAR`. A circle can
  override this with `filter=fc.LINEAR` or `filter=fc.NEAREST` at creation, the same rule as a
  `Texture` filter override. *(Owner decision: follow the window filter, with a per-sprite override,
  2026-10-12.)*
- **FR-013**: Shape sprites MUST behave like every other sprite in sprite lists (append, remove,
  draw order, multiple lists, position, scale, color) and MUST draw efficiently together. Shapes
  that differ only in size or color must not each cost a separate draw step.
- **FR-014**: Shape sprites MUST work in the browser build with the same API and appearance.

**Delivery**

- **FR-015**: `examples/01c_shapes.py` MUST show several rectangles and circles of different sizes
  and colors (using named colors), plus an image sprite tinted with `color`.
- **FR-016**: The type information for editors MUST cover the new colors, `Sprite.color`, and the
  shape-creation API.
- **FR-017**: All Phase 1 behavior MUST stay unchanged (existing automated tests keep passing, and
  the centered-ship example looks identical).

### Key Entities *(include if feature involves data)*

- **Named color**: an RGB (or RGBA) value under a CSS-derived name. A fixed set.
- **Sprite color (tint)**: a per-sprite RGBA value that multiplies the sprite's image. White by default.
- **Shape sprite**: a `RectangleSprite` or `CircleSprite`, which are kinds of `Sprite` whose image
  is a built-in solid shape. It has its own base size (width × height, or diameter) and, for
  circles, an optional filter override. In every other respect it's an ordinary sprite.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of the 148 CSS named colors are available with exact CSS values, verified
  automatically against the CSS specification's list.
- **SC-002**: A developer can draw a colored rectangle and a colored circle with at most 2 lines of
  code each (creation plus adding to a list), with no image files.
- **SC-003**: Drawn shapes match the requested size within 1 pixel and the requested color exactly
  (for opaque colors, away from anti-aliased edges), verified by pixel checks.
- **SC-004**: 1,000 mixed shapes in one list render at the display refresh rate on the development
  machine, and take no more draw steps than 1,000 image sprites that share one image would.
- **SC-005**: Image sprites with the default color render pixel-identical to Phase 1 (the existing
  pixel tests pass unchanged).
- **SC-006**: The Phase 1c example looks the same on desktop and in the browser (the same shapes,
  positions, and colors in screenshots).

## Assumptions

- **CSS list source:** CSS Color Module Level 4 (148 named colors, including `rebeccapurple`), plus
  the `transparent` keyword.
- **Shape size after creation:** width and height (or radius) are set at creation. Resizing later uses
  `scale`. Changing a shape's base size after creation is out of scope for Phase 1c.
- **No outlines or rounded rectangles:** filled shapes only. Outlines, rounded corners, polygons,
  and lines are later "shapes/primitives" backlog items.
- **Tint math:** standard multiply (image × color / 255 per channel, including alpha).
- **Collision shapes:** later (Phase 5). Shape sprites don't imply a collision shape yet.
- **Browser parity:** the web shim (accepted approach B) gets the same API in this phase.
