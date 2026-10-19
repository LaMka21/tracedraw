# Sign & Cut: Effects and Patterns

Patterns for engraving, cutting and decoration: hatching lines inside
shapes, grids and dividing lines, jigsaw puzzles, circle fills, flat cast
shadows and random copies. The geometry is in
`crates/signcut/src/fx/` (`tracedraw_signcut::fx`); the operations and
their settings are in `apps/tracedraw/src/signcut_fx.rs` (`app.sc_fx`),
the panel section in `apps/tracedraw/src/ui/signcut_fx.rs`.

Open it from Window > Panels > Sign & Cut, section Effects and Patterns,
or from Tools > Sign & Cut > Effects and Patterns: Effects and Patterns
Panel, Hatch Selected Shapes, Create Grid, Create Lines, Make Puzzle,
Fill with Circles, Add Cast Shadow and Random Copies. Each menu item
opens the panel at the section and runs at once with the panel's
settings. The settings last for the session (they are not saved).

## Common rules

- Units: millimetres, y up; lengths show in the document's units.
- Hatching, puzzle, circle fill, shadow and random copies work on the
  selected objects in stacking order (objects picked inside groups
  after); locked objects are left out. With nothing selected they do
  nothing and the status line says "Select one or more objects first".
  Grid and Create Lines do not need a selection.
- An object's outlines are its drawn outline in page space (a group by
  its members, a clip frame by its frame, text by its glyphs). The
  inside is taken by the even-odd rule over all closed subpaths of the
  object, so holes stay empty; open subpaths have no inside. Curves are
  flattened within 0.02 mm for these tests.
- Every operation is one undo step with the label given below, and
  selects what it made. When nothing can be made (no closed outline, or
  the settings leave no room) nothing changes and the status line says
  so.
- Random features take a seed (0 to 999 999 999, 1 at first; **New**
  picks another). The same seed and settings give the same result on
  every machine: the generator is SplitMix64, written for this crate.
- New objects without a stated colour get the document's default fill
  and outline for graphics.

## Hatching lines

Settings: Style Lines, Snake or Grid (Lines); Angle (0 degrees, counter-
clockwise from the x axis); Spacing (2 mm, at least 0.05 mm); Inset
(0 mm).

- The region is turned by -angle so the lines run along x. With an inset
  the region is first offset inward by it (mitred corners); an inset
  that swallows the shape gives nothing.
- Lines in the turned frame, region height h, spacing s:
  n = floor(h / s) lines at y = y0 + (h - n s) / 2 + s / 2 + k s, so the
  lines are centred and the outer ones lie s / 2 (plus half the rest)
  from the edges. With h < s one line goes through the middle. The
  spacing is widened when needed so one direction has at most 20 000
  lines.
- Each line is cut where it crosses the outlines; the even pieces inside
  are kept.
- **Snake**: the pieces are joined into zigzags. Line by line, a piece
  continues the zigzag that stopped on the line before at the nearest of
  its two ends, when the straight step from that zigzag's end to it does
  not cross the outline and its middle is inside or on the outline (a
  square's sides). Otherwise it starts a new zigzag. A convex shape
  becomes one continuous line; a U shape becomes one line per arm.
- **Grid**: lines at the angle and at the angle + 90 degrees.
- Result: per object one open curve holding all its lines as subpaths,
  drawn as a hairline (0.0762 mm) in the object's outline colour, else
  its plain fill colour, else black. The curves of all objects form one
  group on the layer of the first selected object. Undo label
  "Hatching". Status: the number of lines.

## Grid and lines

Grid settings: Columns (4) and Rows (3), 1 to 1000 each and at most
100 000 cells; Fill the selection's box (off); Cell width and Cell
height (20 mm each); Bottom left (x, y) (10 mm, 10 mm); As lines or As
rectangles (lines); Show on the drawing (off: draws the grid's lines on
the page while the section is open).

- Without Fill the selection's box (or with nothing selected) the grid
  starts at the bottom-left point with the given cells. With it, the
  grid covers the selection's bounding box: cell width = box width /
  columns, cell height = box height / rows.
- As lines: columns + 1 vertical lines (left to right) and rows + 1
  horizontal lines (bottom up), one open curve per line. As rectangles:
  one rectangle per cell, row by row from the bottom.
- One group on the active layer. Undo label "Grid".

Lines across: Count (3, 1 to 10 000), Horizontal or Vertical
(horizontal). n lines divide the selection's bounding box (or the page
with nothing selected) into n + 1 equal strips: at y0 + h i / (n + 1)
for i = 1..n (x likewise). One curve per line, one group on the active
layer. Undo label "Create Lines".

## Puzzle

Settings: Columns (4) and Rows (3), 1 to 100 each; Tab size (20 % of the
shorter side of a cell, 0 to 35 %); Randomness (50 %, 0 to 100 %); Tabs
Knob or Wave (knob); Seed; Cut lines only (default) or Separate pieces.

- The object's bounding box is divided into columns x rows equal cells.
  Every inner edge between two cells gets one tab of size
  t = tab size x shorter cell side:
  - Knob: a round head on a narrower neck, 0.8 t wide at its base and
    0.95 t high, made of four cubic curves.
  - Wave: a smooth bump 1.2 t wide and 0.6 t high.
- Per edge three random numbers are drawn, in a fixed order (the
  horizontal edges row by row from the bottom, each from the left, then
  the vertical edges column by column from the left, each from the
  bottom): the tab's centre at (0.5 + 0.2 r u) of the edge, its size
  t (1 + 0.2 r v) (u, v from -1 to 1, r the randomness) and the side it
  points to (either, half the time each). The tab keeps 5 % of the edge
  from either end; an edge too short for its tab stays straight. A tab
  size of 0 gives straight edges.
- Cut lines only: rows - 1 horizontal curves across the whole width and
  columns - 1 vertical curves across the whole height, in a group over
  the object; the object stays. Separate pieces: each cell's closed
  outline from its four edges (the outer sides straight); the pieces
  take the object's fill and outline (a group's pieces get the default
  ones), form a group in the object's place, and the object is deleted.
- When the object is an upright rectangle (one closed outline whose area
  equals its box) the curves are kept as they are. Otherwise cut lines
  are clipped to the inside of the outline and pieces are intersected
  with it (boolean intersection); both are then flattened within
  0.02 mm, and a piece cut in two by the outline gives two objects.
- One group per object, all in one step. Undo label "Puzzle". Status:
  the number of cut lines or pieces.

## Circle fill

Settings: Smallest radius (2 mm), Largest radius (10 mm) (exchanged if
given the wrong way round), Gap (1 mm), At most (200 circles, up to
20 000), Seed.

- min(60 x count + 2000, 400 000) random points are tried in the
  object's bounding box until the count is reached. At a point p inside
  the object a radius u is drawn between the smallest and the largest
  and shrunk to
  r = min(u, d - gap, min over earlier circles j of (|p - c_j| - r_j - gap)),
  where d is the distance from p to the outline. The circle is kept when
  r is at least the smallest radius. So circles never overlap, keep the
  gap from each other and from the outline (holes included), and stay
  inside.
- Result: the circles as ellipse objects with the default fill and
  outline, one group per object on its layer. Undo label "Circle Fill".

## Cast shadow

Settings: On a wall or On the floor (wall); wall: Direction (-45
degrees, down to the right) and Distance (3 mm); floor: Height (30 % of
the object's height, 1 % to 500 %), Slant (45 degrees, -80 to 80) and
Falls in front (on); Colour (dark grey, RGB 64 64 64); Opacity (50 %).

- On a wall: the outline moved by the distance in the direction:
  (x, y) -> (x + d cos a, y + d sin a). The object stands off a wall
  behind it. (No shear: the wall shadow is the outline's shape.)
- On the floor: the outline flattened and slanted, standing on the
  bottom edge y0 of the object's bounds:
  (x, y) -> (x + k tan(slant) (y - y0), y0 - k (y - y0)) in front
  (below the bottom edge), or y0 + k (y - y0) behind (above it, mostly
  hidden by the object), with k the height fraction.
- The shadow's outline: the object's outline when it is one path of
  closed subpaths (curves kept), otherwise the closed subpaths of all
  its outlines welded into one (flattened 0.02 mm). Objects with no
  closed outline get no shadow.
- The shadow is a closed curve filled with the colour, no outline, at
  the opacity, placed directly behind the object on its layer (behind
  its group when the object is inside one). One shadow per selected
  object, all in one step; the shadows are selected. Undo label "Cast
  Shadow".

## Random copies

Settings: Use Selected Object as Container (one object selected) or Use
the Page (the default: no container); Count (20, up to 5000); Rotation
from and to (0 to 360 degrees); Scale from and to (80 % to 120 %, at
least 1 %); Minimum distance (0 mm); Seed.

- The selected objects other than the container are copied together.
  They stand for their bounding circle: centred on the selection's box,
  radius R = half its diagonal, so whatever the turn a copy lies within
  it.
- A place is a random point p in the container's box with a random turn
  a and scale k from the ranges. It is kept when p is inside the
  container (the page rectangle, or the container's closed outlines
  even-odd), the outline is farther than R k from p (the circle is
  inside), and every earlier copy's centre q is at least
  R k + R_q k_q + minimum distance away (the circles do not overlap).
  Up to 200 tries per copy wanted; the search stops after 20 000 misses
  in a row. A copy's transform is translate(p) rotate(a) scale(k)
  translate(-centre), applied to each object in page space.
- Each place gets one copy of each object (grouped when there are
  several), on the active layer; the copies are selected. The status
  line says "n of count copies placed": fewer when no more fit. Undo
  label "Random Copies".

## Checks

Geometry (`cargo test -p tracedraw-signcut`):

- `fx::rng::tests::*`: the same seed repeats, values stay in their
  ranges and spread evenly, non-finite ranges.
- `fx::region::tests::*`: even-odd inside with holes and area, open and
  non-finite outlines make no region, edge distances and crossings,
  polylines cut to the inside.
- `fx::hatch::tests::*`: centred parallel lines in a square, holes stay
  empty, angle and inset, snake joins a circle and a square into one
  line and a U shape into one per arm without leaving it, grid,
  degenerate and non-finite input.
- `fx::grid::tests::*`: grid lines and cells, filling a box, dividing
  lines, bad and oversized grids.
- `fx::puzzle::tests::*`: pieces tile the box exactly, cut lines are the
  inner edges and longer than straight, the seed repeats and another
  changes it, wave tabs on a circle cut to the shape (pieces cover the
  disc), degenerate input and caps.
- `fx::circles::tests::*`: circles stay inside, apart and in range,
  holes stay empty and the count is kept, the seed repeats, bad settings
  and shapes.
- `fx::shadow::tests::*`: wall move, floor shadow standing on the bottom
  edge in front and behind, extreme and bad settings, the outline keeps
  curves or welds pieces.
- `fx::scatter::tests::*`: copies land inside and apart, the seed
  repeats and the ranges hold, too big or bad input places nothing.

Application (`cargo test -p tracedraw --bin tracedraw -- signcut`):

- `signcut_fx::tests::hatching_adds_one_group_in_one_step`
- `signcut_fx::tests::grids_come_as_lines_or_cells_and_fill_the_selection`
- `signcut_fx::tests::lines_divide_the_selection_or_the_page`
- `signcut_fx::tests::a_puzzle_adds_cut_lines_or_replaces_the_shape_with_pieces`
- `signcut_fx::tests::circle_fill_puts_circles_inside`
- `signcut_fx::tests::a_shadow_goes_right_behind_its_object`
- `signcut_fx::tests::random_copies_land_inside_the_container_and_repeat_with_the_seed`
- `ui::signcut_fx::tests::the_section_draws_in_every_mode_and_previews_the_grid`
- The menu items run in `ui::menus::tests::every_menu_item_runs_without_panicking`.
