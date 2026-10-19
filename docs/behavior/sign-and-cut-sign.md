# Sign & Cut: Sign Making

Tools for illuminated signs and built-up letters: LED modules, neon
tubes, sign boxes, letter returns, liquid acrylic moulds, smart divide
(tiling) and carriers. The geometry is in `crates/signcut/src/sign/`;
the operations and their settings are in
`apps/tracedraw/src/signcut_sign.rs`, the panel section in
`apps/tracedraw/src/ui/signcut_sign.rs`.

Open it from Tools > Sign & Cut > Sign Making (each item opens the Sign
& Cut panel at the Sign Making section with that tool's header open) or
from the panel itself. Nothing runs from the menu at once: every tool
has settings.

## Common rules

- The tools work on the selected objects, in stacking order; locked
  objects are left out. Only the sign box works with nothing selected.
- An object's outlines are its drawn outline in page space (a group by
  its members, text by its glyphs), as for Measure. Closed outlines of
  one object form a region by the even-odd rule: a closed outline inside
  an odd number of larger ones is a hole.
- Lengths are entered and shown in the document's units and kept in
  millimetres.
- New objects go on the layer of the first selected object (when it is
  unlocked), else on the active layer. Each tool adds its result as one
  named group (Smart Divide: one group per tile) in one undo step whose
  label is given below. The selection stays as it was, except after
  Smart Divide, which selects the tiles.
- Non-finite or negative settings are treated as zero or as the
  smallest sensible value; nothing panics on them.

## LED modules (undo: "LED Modules")

Catalogue (editable in the panel: pick, edit, Copy Module, Delete
Module; kept for the session): name, width W and height H (a round
diode: diameter), diodes, power (W), price, colour. Defaults: 2-LED
40 x 12 mm 0.48 W; 3-LED 75 x 15 mm 0.72 W; 1-LED lens 30 x 15 mm
0.5 W; 3-LED high power 70 x 20 mm 1.5 W; round diode 8 mm 0.06 W.
"Turn the modules a quarter" swaps W and H.

Settings: layout (Rows), row spacing (40 mm, centre to centre, at least
the module height), module spacing (60 mm, centre to centre, at least
the module width), distance from the edge (5 mm), distance from the
outline (15 mm, Along the outline only).

Layouts:

- **Rows (matrix)**: the region is shrunk by the edge distance
  (mitred offset). Rows of the module height are spread over its height,
  `floor((H_region - h) / row_spacing) + 1` rows, centred. In each row
  the band of the module's height is tested where it lies wholly inside
  (exact for the flattened outline: at the band's edges and at every
  vertex height in between); in each such stretch of length L,
  `floor((L - w) / spacing) + 1` modules are centred.
- **Shifted rows**: as Rows, every second row moved by half a module
  spacing; a module that no longer fits is dropped (a row that would be
  empty keeps the unshifted positions).
- **Along the stroke centre**: the region is sampled on a grid (a
  quarter of the smaller module side, 0.25 to 5 mm, at most 400 000
  cells) and each cell's exact Euclidean distance to the outside is
  computed. Ridge cells (farther from the outside than one neighbour
  and not nearer than the other, across one of four directions) with
  room for half the module height plus the edge distance are walked
  from one far end of each ridge; a module is placed at a cell at least
  one module spacing from every module placed, turned along the ridge
  (main axis of the ridge cells within half a spacing).
- **Along the outline**: the region is offset inwards by the distance
  from the outline (round joins); on each resulting loop of length L,
  `max(1, floor(L / spacing))` modules are spread evenly, turned along
  the loop.

In the last two layouts a module is kept only when its four corners and
four side middles lie inside the region and at least the edge distance
from its outline (a round diode: its centre at least radius + edge).
At most 20 000 modules per object.

Report: modules, diodes, total power = modules x power, total price =
modules x price, and the power supply: total power x 1.2 rounded up to
30, 60, 100, 150, 200 or 300 W; above 300 W, `ceil(power x 1.2 / 300)`
units of 300 W. The modules are rectangles (or circles) filled with the
module's colour, outlined in half that colour, in a group "LED modules".

## Neon tubes (undo: "Neon Tubes")

The selected curves (open or closed subpaths) are the tubes' centre
lines. Settings: tube diameter (12 mm), longest tube (1500 mm),
supports every (300 mm, 0 = none), hole diameter (5 mm), draw as
outlines (off).

- A line no longer than the longest tube is one tube (a closed loop
  stays closed). A longer line of length L becomes `ceil(L / max)`
  tubes: each split aims at the even share of what is left, and may be
  anywhere that keeps the count (between `L - (k - 1) max` and
  `start + max`); it moves to the nearest point of a straight stretch
  (consecutive straight segments, direction within 1 degree, at least
  three diameters long, the split kept one diameter from its ends),
  else stays at the even share. No tube is longer than the maximum.
- Support holes: on each tube of length l, `max(1, ceil(l / spacing))`
  holes at `(k + 1/2) l / n` along it, circles of the hole diameter.
- A tube is drawn as an open curve with a round-capped outline of the
  tube diameter in neon pink, or (as outlines) as the band of the
  diameter around the centre line with round ends, hairline (a closed
  loop gives two rings). Status: tubes, total length, holes.
- At most 10 000 tubes; a line that would need more is skipped.

## Sign box (undo: "Sign Box")

The face W x H is the selection's bounds, or with nothing selected the
panel's width and height (1000 x 500 mm) centred on the page. Settings:
depth D (50 mm), return lip L (0 = none), corner gap g (0.5 mm, at
most a quarter of the shorter side), corner relief holes (diameter,
0 = none), fixing holes (off; spacing 300 mm, diameter 5 mm).

- Flat pattern: the face with a flange of depth D on each side, each
  flange `side - 2g` long (the corner squares notched out). With a lip,
  each flange carries a lip of width L whose ends are cut at 45 degrees
  (its outer edge `flange - 2L` long; L at most 99 % of half the
  shortest flange). Relief holes are circles centred on the four face
  corners, cut out of the outline (boolean difference).
- Fold lines: the four face edges (and the four lip folds), each the
  flange's length, as separate open curves, dashed (0.2 mm, 3 mm on,
  2 mm off) in blue. The cut outline and holes are red hairlines.
- Fixing holes: in each flange at half the depth, `max(1, ceil(len /
  spacing))` holes evenly spread at `(k + 1/2) len / n`.
- No bend allowance: the sizes are flat sizes. While the header is
  open the pattern and folds show on the drawing and the panel gives
  the flat size.

## Letter returns (undo: "Letter Returns")

Every closed outline of each selected object gets a strip. Settings:
depth (50 mm, the strip height), overlap (10 mm), material thickness t
(1 mm), length on Inner face / Middle / Outer face (Inner), notch
corners under (135 degrees, 0 = none), notch size (3 mm), space between
strips (10 mm), number the strips (on).

- Length = face length + overlap. Face length = `L + s 2 pi d` with L
  the outline's arc length, d = 0, t/2 or t for the faces, s = +1 for
  an outer outline and -1 for a hole (exact for convex outlines with
  corners rounded by the material, close for others).
- Notches: at every node (not the first, which is the strip's end)
  where the corner angle (180 minus the turn between the incoming and
  outgoing directions) is under the limit, a V notch as wide and deep as
  the notch size (at most half the height) on the strip's top edge, at
  the node's arc length scaled by face length / L.
- Layout: strips one under another, left edges at the selection's left,
  the first one the strip space under the selection, each the space
  apart. (Stacked rather than side by side: return strips are long and
  narrow.) Each strip is labelled with a text number inside its left end
  ("2", or "2.1", "2.2" when the object has more than one outline),
  0.4 x the height (1 to 20 mm) in the document's text font.

## Liquid acrylic mould (undo: "Acrylic Mould")

Settings: wall thickness w (2 mm), fill depth (5 mm), overlap (10 mm);
strip space and numbering shared with Letter returns.

- Base: each letter's region offset outwards by w (round corners), at
  the letter's place, red hairline.
- Wall strip: as a letter return on the Middle face of a material w
  thick (length `L + s pi w + overlap`), as high as the fill depth, no
  notches; laid out under the bases.

## Smart divide (undo: "Smart Divide")

Settings: tile width (1220 mm) and height (2440 mm), overlap (10 mm),
spread tiles apart by (0 mm), grid from Selection corner (the
selection's bottom left) or Page origin, name the tiles (on), add the
tile frames (off).

- Columns start every `width - overlap` from the origin, rows every
  `height - overlap`; the grid takes the column and row holding the
  selection's left and bottom edges and as many as it takes to reach
  the right and top edges. More than 2500 tiles, or an overlap not
  under the tile size, makes no grid (the panel says so).
- Each tile holds each drawn member of each object cut to the tile's
  rectangle: closed outlines by boolean intersection (holes kept,
  curves flattened to 0.02 mm), open lines clipped as polylines (0.05
  mm). A piece keeps its member's fill, outline and opacity and becomes
  a curve. Tiles with no piece are skipped.
- Names: a row letter counted from the top (A, B, ..., Z, AA, ...) and
  a column number from the left: A1 is the top left. With names on,
  each tile gets its name as text near its top-left corner (4 % of the
  tile's smaller side, 3 to 50 mm); with frames on, its rectangle as a
  dashed grey hairline. The tile group is named "Tile A1" and so on.
- Spread: the tile in column c and row r (from the bottom) moves by
  (c x gap, r x gap).
- The originals are deleted; the tiles are selected. Objects holding a
  bitmap are left out (they stay whole and in place). While the header
  is open the grid shows on the drawing.

## Carrier (undo: "Carrier")

Settings: distance (10 mm), rounding r (0), fill the holes (on).

- Each object's region is offset outwards by distance + r (round
  joins), all are welded, and the result offset inwards by r: inner
  corners and narrow gaps get radius r, and gaps under
  `2 (distance + r)` close. Fill the holes keeps only the outer
  outlines. Letters far apart give separate pieces (the status line
  gives the count).
- The outline (green hairline, no fill) goes into the lowest selected
  object's layer just under it.

## Checks

- `tracedraw_signcut::sign::tests::*`: regions keep closed subpaths
  and drop non-finite ones, even-odd inside and edge distance, outer
  contours.
- `tracedraw_signcut::sign::led::tests::*`: matrix count and centring,
  shifted rows, holes and narrow parts, stroke centre on horizontal and
  upright bars, outline spacing, round diodes, too small shapes and
  non-finite input, power supply sizes, band intervals at a notch, the
  catalogue.
- `tracedraw_signcut::sign::neon::tests::*`: one tube, even splits,
  split moved to a straight stretch, closed loop and its two rings,
  round ends, no supports, bad input and too many tubes.
- `tracedraw_signcut::sign::sign_box::tests::*`: cross pattern area,
  gaps and lips, fixing holes, relief holes, degenerate sizes.
- `tracedraw_signcut::sign::returns::tests::*`: square strip and
  notches, faces and holes, smooth curves, stacking, mould, bad input.
- `tracedraw_signcut::sign::tiles::tests::*`: grid columns and origin,
  labels and spread, clipping with holes, open lines, overlaps adding
  up, bad sizes.
- `tracedraw_signcut::sign::carrier::tests::*`: one outline, far
  letters and rounding, holes filled or kept, nothing closed.
- `signcut_sign::tests::*` (app): LED layout group, report and undo,
  selection needed, neon split and supports (thick and outline),
  sign box from the selection and from the panel size, returns under the
  letters, mould, smart divide replacing objects with spread tiles and
  undo, carrier under the letters, menu opening the tool.
