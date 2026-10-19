# Sign & Cut: Cutting

Tools that prepare a job for a cutting plotter: weeding cuts, bridges,
the cut order, plotter output (HPGL), registration marks for print and
cut, and crop marks. The geometry is in `crates/signcut/src/cut/`; the
operations are in `apps/tracedraw/src/signcut_cut.rs` and the panel
section in `apps/tracedraw/src/ui/signcut_cut.rs`.

Open it from Tools > Sign & Cut > Cutting, or in the Sign & Cut panel
(Window > Panels > Sign & Cut) under Cutting. The submenu has Cutting
Panel, then Add Weeding Lines, Add Bridges, Optimize Cut Order (each runs
at once with the panel's settings and opens the panel), Plotter Output
(opens the panel), Add Registration Marks and Add Crop Marks (run at
once).

Lengths are millimetres in the model and shown in the document's
units. Coordinates are page space, y up, origin at the page's bottom
left.

## Which objects

Like the other Sign & Cut tools: the selection, or with nothing selected
every visible, unlocked object of the page's layers. An object's
outlines are its drawn outline in page space (a group by its members, a
clip frame by its frame, text by its glyphs).

The groups these tools make carry an object data field `signcut` with
the value `weeding`, `registration marks`, `cut contour` or `crop
marks`. Registration marks and crop marks are printed, never cut: the
plotter output, bridges and the cut order leave them out. Weeding, the
registration marks and the crop marks also leave out everything these
tools made (so weeding twice does not weed around the old weeding cuts).

## Weeding

Settings: Lines or Boxes (Lines); Gap from objects 2 mm; Shorten lines
by 1 mm; Lines per side 1 (0 to 20); Weeding frame on, margin 5 mm; box
margins 2 mm (width) and 2 mm (height).

- **Lines**: each object is grown by the gap: its closed outlines are
  offset outward by the gap (round corners), its open outlines widened
  to a band of twice the gap. For each object and each side of its
  bounding box, `n` lines run out of the box at `i / (n + 1)` of the side
  (i = 1..n), perpendicular to it. A line is cast from the box's centre
  line outward to the frame; it starts where it leaves its own grown
  object for the last time (or at the box side when it never meets it)
  and stops at the first other grown object or at the frame. A line
  starting inside another object's grown area is dropped. The start is
  pulled back by Shorten, and so is an end that stops at an object; an
  end on the frame touches it so the waste comes apart. Lines shorter
  than 0.5 mm are dropped; a line whose ends lie within 0.5 mm of
  another line's ends is the same line and kept once (two neighbours
  facing each other share one line). Lines never cross a grown object.
- **Boxes**: a rectangle around each object, its bounding box grown by
  the width margin left and right and the height margin below and
  above. Boxes that overlap are welded into one outline.
- **Frame**: the bounding box of all the objects (Lines) or of all the
  boxes (Boxes) grown by the margin, as a closed rectangle.

The cuts are open curves (boxes and the frame closed), hairline
(0.0762 mm) in light blue (RGB 0, 150, 255), no fill, grouped as one
object named "Weeding lines" on the active layer and selected. One undo
step ("Weeding Lines"). The status line gives the number of cuts.

## Bridges

Settings: Bridge length 2 mm (at least 0.01 mm); Bridges per outline 2
(1 to 1000), or One bridge every 100 mm; Only outlines longer than 0 mm.

- Each closed subpath of length `L > minimum` gets `n` bridges: the
  count, or `round(L / distance)` (at least 1). When `n x length > 0.9 L`
  the outline is left whole.
- Ideal centres at `(i + 0.5) L / n` along the arc length from the start
  node. Each moves to the nearest point of the middle half of a straight
  segment (a segment at least two bridges long; the bridge centre at
  least a bridge length from its ends) within a quarter of the spacing,
  preferring straight stretches over curves and corners. Two bridges
  that would come closer than 1.5 bridge lengths go back to their ideal
  places.
- The outline is cut into open pieces between the bridges (from the end
  of one bridge to the start of the next); curves keep their exact
  shape (segments are split at the arc length, accuracy 0.0001 mm).
  Open subpaths and outlines without bridges stay as they are.
- Rectangles, ellipses, polygons and curves without live effects take
  part, groups by their members; text, bitmaps, locked objects and
  registration or crop marks do not. A changed object becomes an open
  curve in its place. One undo step ("Bridges"); the status line gives
  the number of bridges.

## Cut order

Settings: Direction Keep, Clockwise or Counter-clockwise (Keep); Start
point Nearest node, Bottom-left node or Keep (Nearest node); Inner
outlines first (on); Show the route (off).

The knife starts at the page origin (0, 0). Each layer is planned on its
own, from the bottom layer up, each starting where the previous one
ended (the export cuts in stacking order, layer by layer). Locked
objects and layers keep their places.

1. **Inner first**: an object lies inside another when its bounding box
   lies in the box of one of the other's closed outlines, its box is
   smaller, and its first node lies inside that outline. It is cut
   before. Inside one object, a closed subpath inside another subpath
   (a hole) is cut before it.
2. **Nearest neighbour**: from where the knife is, the next object is
   the one whose cheapest allowed first cut starts nearest; inside an
   editable object its subpaths go the same way.
3. **2-opt**: a stretch of the order is reversed when that shortens the
   travel between the objects' entry and exit points and keeps rule 1
   (up to 20 passes, up to 2000 objects per layer). The result is kept
   only when its exact travel is shorter than the nearest-neighbour one.
4. **Start points** of closed outlines: the node nearest the knife, the
   node nearest the bottom-left corner of the outline's box, or the
   current start. Open paths are cut from their nearer end unless the
   direction is Keep.
5. **Direction** of closed outlines: as they are, all clockwise or all
   counter-clockwise as seen on the page (signed area of the flattened
   outline, positive = counter-clockwise).

Travel is the sum of the straight moves with the knife up: from the
origin to the first start and from each cut's end to the next start;
the move back to the origin is not counted. **Optimize** moves the
objects within their layer into the planned order (they take the
places the targets held, other objects stay where they are), and gives
the rectangles, ellipses, polygons and curves whose subpaths change
order, start or direction their new outline as a curve (closed when all
its subpaths close); objects whose outline needs no change keep their
kind. Groups and text keep their outlines and only move in the order.
One undo step ("Cut Order"); the status line and the panel give the
travel before and after. With **Show the route** the planned knife-up
moves are drawn on the canvas as thin lines and the panel shows the
travel now and planned (recomputed when the drawing, the objects or
the settings change).

## Plotter output

Profiles: a list (one "Cutting plotter" profile at first); New copies the
current one, Delete removes it (one always stays). Profiles last for the
session; they are not saved between runs.

| Setting | Default | Range |
| --- | --- | --- |
| Name | Cutting plotter | |
| Origin | Bottom left | the four corners |
| Units per mm | 40 (0.025 mm) | 1 to 10000 |
| Pen up, Pen down | `PU`, `PD` | text |
| Start string, End string | `IN;`, `PU0,0;` | text, sent as typed |
| Speed | 0 (not sent) | 0 to 10000, sent as `VS<v>;` |
| Force | 0 (not sent) | 0 to 10000, sent as `FS<v>;` |
| Knife offset | 0.25 mm | at least 0 |
| Corners sharper than | 15 degrees | 0 to 180 |
| Overcut | 1 mm | at least 0 |
| Mirror, Rotate 90 | off | |
| Passes | 1 | 1 to 10 |
| Cut a frame around the job | off, margin 5 mm | |
| New line after each command | off | |

- The objects' outlines (registration and crop marks left out) are
  taken in the route order of the cut order tool (Optimize the cut
  order, on; the drawing is not changed) or in stacking order, and
  flattened (0.02 mm).
- Rotate 90 turns them counter-clockwise, `(x, y) -> (-y, x)`; Mirror
  flips left and right, `x -> -x`. Then the chosen corner of the job's
  box (the frame included) becomes the plotter's 0,0 with both axes
  counting into the job: bottom left `(x - x0, y - y0)`, bottom right
  `(x1 - x, y - y0)`, top left `(x - x0, y1 - y)`, top right
  `(x1 - x, y1 - y)`. Coordinates are rounded to whole units
  (`mm x units per mm`).
- **Overcut**: a closed outline is cut on past its start along its first
  segments for the overcut length (at most one more lap).
- **Knife offset**: the tip of a swivel knife trails its axis by `d`. For
  a point `P` cut while moving in direction `u` the holder goes to
  `P + d u`. Where the direction turns by more than the corner limit the
  holder draws an arc of radius `d` around the node from the old
  direction to the new one, in steps of at most 10 degrees; smaller
  turns go straight on.
- With the frame option a rectangle around the job, the margin away, is
  cut last. Each outline is cut once per pass, all outlines of a pass
  before the next pass.
- Output: the start string, `VS`/`FS` when set, then per outline
  `PU x,y;` and `PD x,y,...;` (at most 64 points per command, repeated
  points dropped), then the end string.
- **Save as .plt...** asks for a file (`cut.plt`; in the browser it is
  downloaded). On the desktop **Send** writes the same bytes to the port
  or share typed (COM3, \\host\share, /dev/usb/lp0): an existing device
  or share is opened for writing as it is; a regular file is replaced;
  a path that does not exist becomes a file. The panel shows the
  outlines, the length to cut and the job's size after a save or send.
  Nothing in the drawing changes; no undo step.

## Registration marks

Settings: Around The objects or The page (inside) (The objects); Style
Square, Circle or L corner (Square); Mark size 5 mm; Offset 10 mm; Extra
marks every 400 mm (0: none); Corner length 20 mm and Corner width 1 mm
(L corner); Cut contour Outlines, Bounding box or None (Outlines).

- **Around the objects**: the box is the objects' bounding box. A
  square or circle mark at each corner lies outside the box, its nearest
  edge `offset` from the corner in x and in y (centre at `offset +
  size / 2` from the corner on both axes). **The page**: the box is the
  page and the marks lie inside it, `offset` in from both edges.
- **L corner**: two arms along the corner's sides, `corner length` long
  and `corner width` wide; around objects the L's inner edges are
  `offset` from the box, inside a page its outer edges are `offset` from
  the page edges.
- **Extra marks**: when the distance between the corner marks' centres
  along a side is longer than the spacing, it is split into
  `ceil(distance / spacing)` equal parts and a mark (a square for the L
  style) sits at each division, on both opposite sides, in line with the
  corner marks.
- Marks are filled black, no outline, grouped as "Registration marks".
- **Cut contour**: copies of the objects' outlines (one curve per
  object) or their bounding box, hairline, no fill, in the spot ink "Cut
  contour" (CMYK 0, 100, 0, 0; added to the drawing's inks when it is
  not there), grouped as "Cut contour". The ink and group name stay in
  English whatever the language, so later steps can find them.
- Both groups go on the active layer and are selected. One undo step
  ("Registration Marks"). The plotter output cuts the cut contour and
  leaves the marks out.

## Crop marks

Settings: Length 5 mm; Offset 3 mm; Outline width 0.25 mm; Around each
object (off: around all of them).

At each corner of the box two lines along the box's sides produced,
from `offset` to `offset + length` away from the corner. One curve per
corner (its two lines), in registration colour at the outline width,
grouped as "Crop marks" on the active layer and selected. One undo step
("Crop Marks").

## Checks

- `tracedraw_signcut::cut::weed::tests::*`: four lines and a frame
  around one object with the gap and shortening, one shared line between
  neighbours, lines never entering an object, a ring's lines starting
  outside it, boxes grown and welded, open curves as obstacles,
  degenerate and non-finite input.
- `tracedraw_signcut::cut::bridges::tests::*`: two bridges moved off a
  square's corners, bridges by distance and the minimum length, curves
  kept exact, open paths and crowded outlines left alone, non-finite
  input and settings.
- `tracedraw_signcut::cut::order::tests::*`: nearest neighbour on a
  zigzag, inner outlines first (between and inside objects), nearest and
  bottom-left start nodes, directions, fixed items and 2-opt, empty and
  non-finite items.
- `tracedraw_signcut::cut::hpgl::tests::*`: a square in plotter units
  with overcut, speed, force, passes and line breaks; origin corners,
  mirror and rotation; knife offset arcs at sharp corners only; the
  frame cut last; empty and non-finite input.
- `tracedraw_signcut::cut::marks::tests::*`: square marks outside the
  corners and inside a page, extra marks on long sides, L corners, crop
  marks, bad input.
- `signcut_cut::tests::*` (app): weeding as one group in one step and
  not around old weeding, bridges and undo, cut order reordering a layer
  with travel before and after and curves only when needed, the route
  preview cache, plotter output in route order without marks, sending
  to a file path, registration marks with the cut contour and its ink,
  crop marks around each object or all, nothing changing on an empty
  page.

## Not done

- Profiles are not kept between runs.
- Weeding lines run along the box sides' perpendiculars only (no
  diagonal or shape-following weeding).
- The cut order does not move objects between layers, and objects inside
  groups are ordered with their group.
