# Sign & Cut: Sign Making

Tools for illuminated signs and built-up letters: LED modules, neon
tubes, sign boxes, letter returns, liquid acrylic moulds, smart divide
(tiling) and carriers. The geometry is in `crates/signcut/src/sign/`;
the operations and their settings are in
`apps/tracedraw/src/signcut_sign.rs`, the tool windows in
`apps/tracedraw/src/ui/signcut_win_sign.rs`.

Open a tool from the Sign & Cut toolbar (Sign Making: LED Modules, Neon
Tubes, Letter Returns, Acrylic Mould; Construct: Sign Box, Smart Divide,
Carrier), the Sign & Cut panel or Tools > Sign & Cut > Sign Making; each
opens the tool's own window (see "Windows" under each tool and
`sign-and-cut-windows.md` for how tool windows behave). Nothing runs at
once: every tool has settings.

## Common rules

- The tools work on the selected objects, in stacking order; locked
  objects are left out. Only the sign box works with nothing selected.
- An object's outlines are its drawn outline in page space (a group by
  its members, text by its glyphs), as for Measure. Closed outlines of
  one object form a region by the even-odd rule: a closed outline inside
  an odd number of larger ones is a hole.
- Lengths are entered and shown in the document's units and kept in
  millimetres. Every window has a Units row that changes the document's
  units.
- New objects go on the layer of the first selected object (when it is
  unlocked), else on the active layer. Each tool adds its result as one
  named group (Smart Divide: one group per tile) in one undo step whose
  label is given below. The selection stays as it was, except after
  Smart Divide, which selects the tiles, and Select All LEDs.
- Non-finite or negative settings are treated as zero or as the
  smallest sensible value; nothing panics on them.
- Settings are kept for the session. Letter Returns, Acrylic Mould and
  Sign Box have presets (tool keys `letter_returns`, `acrylic_mould`,
  `sign_box`).

## LED modules (undo: "LED Modules")

Catalogue (editable in the window; kept for the session): name, width W
and height H (a round diode: diameter), diodes, power (W), price,
colour. Defaults: 2-LED 40 x 12 mm 0.48 W; 3-LED 75 x 15 mm 0.72 W;
1-LED lens 30 x 15 mm 0.5 W; 3-LED high power 70 x 20 mm 1.5 W; round
diode 8 mm 0.06 W. Turn "Across the curve" swaps W and H.

Settings: distance between modules (60 mm, centre to centre, at least
the module width), distance between rows (40 mm, at least the module
height), gap from edge (5 mm), distance from the outline (15 mm, on an
offset curve), distance from the curve (10 mm, around the curve),
random seed (1), fix overlaps (no) with overlap distance (0 mm).

Methods on curves:

- **LEDs on the curve**: on every subpath of the selected objects, open
  or closed, flattened (0.05 mm). A closed one of length L gets
  `max(1, floor(L / spacing))` modules at the middles of equal
  stretches; an open one `floor(L / spacing) + 1` from end to end (one
  in the middle when shorter than the spacing). Each module is turned
  along the curve. No inside check.
- **LEDs around the curve**: the same positions moved the distance from
  the curve to both sides (along the normal): two rows.
- **LEDs on an offset curve**: the region is offset inwards by the
  distance from the outline (round joins); on each loop of length L,
  `max(1, floor(L / spacing))` modules spread evenly, turned along it.

Fill methods:

- **Matrix fill**: the region is shrunk by the edge distance (mitred
  offset). Rows of the module height are spread over its height,
  `floor((H_region - h) / row_spacing) + 1` rows, centred. In each row
  the band of the module's height is tested where it lies wholly inside
  (exact for the flattened outline); in each such stretch of length L,
  `floor((L - w) / spacing) + 1` modules are centred.
- **Matrix fill with shift**: as Matrix, every second row moved by half
  a module spacing; a module that no longer fits is dropped (a row that
  would be empty keeps the unshifted positions).
- **Island fill**: rings: the region offset inwards (round joins) by the
  edge distance plus half the module height, then again by the row
  spacing each time until nothing is left; modules spread along each
  ring as on an offset curve.
- **Centre line fill**: the region is sampled on a grid (a quarter of
  the smaller module side, 0.25 to 5 mm, at most 400 000 cells) and each
  cell's Euclidean distance to the outside computed. Ridge cells with
  room for half the module height plus the edge distance are walked from
  one far end of each ridge; a module is placed at least one spacing
  from every module placed, turned along the ridge.
- **Random fill**: up to `30 x box area / spacing^2 + 100` (at most
  200 000) random points in the region's box (xorshift from the seed,
  repeatable); a point is kept when the module fits and no kept centre
  is closer than the spacing.

In Offset, Island, Centre and Random a module is kept only when its four
corners and four side middles lie inside the region and at least the
edge distance from its outline (a round diode: its centre at least
radius + edge). With fix overlaps, a module whose footprint grown by
half the overlap distance meets a kept one's (turned rectangles,
separating axes; round diodes by centre distance) is left out, in
placing order, also across the selected objects. At most 20 000 modules
per object.

Report: modules, diodes, total power = modules x power, total price =
modules x price, and the power supply: total power x 1.2 rounded up to
30, 60, 100, 150, 200 or 300 W; above 300 W, `ceil(power x 1.2 / 300)`
units of 300 W. The modules are rectangles (or circles) filled with the
module's colour, outlined in half that colour, in a group "LED modules".

Tools:

- **Select all LEDs in the drawing**: selects the unlocked groups named
  "LED modules" on the visible, unlocked layers of the page.
- **Fix overlaps in the selected LEDs** (undo: "Fix LED Overlaps"): in
  each selected LED group, a module whose bounding box grown by half
  the overlap distance overlaps a kept one's (with positive area) is
  removed, in drawing order; one step for all groups.

### Window

About 640 x 480, not resizable. At the left, tabs Catalogue and Edit
module: the catalogue is a list (colour square, name, power) with the
chosen module's size, diodes, power and price under it; Edit module has
Name, Round diode, Width (Diameter), Height, Diodes, Power (W), Price,
Colour. Under them Turn (Along the curve / Across the curve), Fix
overlaps (No / Yes) with Overlap distance, Units, and Copy Module,
Delete Module. At the right, tabs On curve, Fill and Tools. On curve
and Fill each have a Method list (their methods above) and a Main group
with that method's fields, a hint, the last report and the command row:
Create LEDs (adds the layout, one step), Show (marks the layout on the
drawing while the window is open; worked out again only when the
drawing, the selection or a setting changes) and Undo Last (undoes the
last step when it was "LED Modules"). Switching to On curve or Fill
picks that tab's first method when the current one belongs to the other
tab. Tools has Search: Select all LEDs; Edit: Overlap distance and Fix
overlaps. Close at the bottom right.

## Neon tubes (undo: "Neon Tubes", "Divide Tubes", "Support Holes")

Neon type: **Centre line neon** (the selected curves, open or closed
subpaths, are the tubes' centre lines) or **Contour neon** (closed
shapes; the centre lines are the region offset inwards by gap from edge
+ d/2, then each further tube by d + gap between tubes, mitred or with
round corners (on), as many as tube count (1, at most 100) or as fit).
Settings: tube diameter d (standard 6, 8, 10, 12, 14, 15, 16, 18, 20 mm
or a custom value; 12 mm), gap from edge (0), tube count (1), gap
between tubes (50 mm), round corners (on), draw as outlines (off),
support distance (300 mm, 0 = none), maximum tube length (1500 mm),
minimum tube length (0 = none), support hole diameter (5 mm).

- A centre line shorter than the minimum tube length is left out.
- A line no longer than the maximum is one tube (a closed loop stays
  closed). A longer line of length L becomes `ceil(L / max)` tubes:
  each split aims at the even share of what is left, and may be
  anywhere that keeps the count; it moves to the nearest point of a
  straight stretch (straight segments, direction within 1 degree, at
  least three diameters long, the split kept one diameter from its
  ends), else stays at the even share.
- Support holes: on each tube of length l, `max(1, ceil(l / spacing))`
  holes at `(k + 1/2) l / n` along it, circles of the hole diameter.
- A tube is drawn as an open curve with a round-capped outline of the
  tube diameter in neon pink, or (as outlines) as the band of the
  diameter around the centre line with round ends, hairline (a closed
  loop gives two rings). Status: tubes, total length, holes.
- At most 10 000 tubes; a line that would need more is skipped.

Tools: **Divide into tubes** (the tubes only), **Make support holes**
(the holes only), **Convert tubes to outlines** (undo: "Tubes to
Outlines": every selected curve, or curve in a selected group, drawn
with an outline thicker than a hairline becomes the band of that width
with round ends, no fill, a hairline in the same colour; one step),
**Measure the selection** (tubes, length and holes the selection would
make; nothing changes).

### Window

About 330 x 520, not resizable. Tabs Neon, Tools and Options. Neon:
Neon type, a hint, Tube diameter (list) and Custom diameter (enabled for
Custom), for Contour neon Gap from edge, Tube count, Gap between tubes
and Round corners, Draw as outlines, and a full-width Apply that makes
the tubes and holes (one step). Tools: a Processing group with the four
commands as full-width buttons. Options: Support distance, Maximum and
Minimum tube length, Hole diameter, Units. The last report under the
tab; Close at the bottom right.

## Sign box (undo: "Sign Box")

The face W x H is the selection's bounds (with "Around the selection
when there is one", on), or the window's width and height (1000 x
500 mm) centred on the page. Settings: method (Classic, With corner
tabs), first fold (depth) per side (left, right, bottom, top; 50 mm;
0 = no flange on that side), second fold (lip) per side (0 = none),
corner gap g (0.5 mm, at most a quarter of the shorter side), corner
relief holes (diameter, 0 = none), tab length (20 mm), fixing holes
(off; spacing 300 mm, diameter 5 mm), legend (off; prefix).

- Flat pattern: the face with a flange of its side's depth on each
  side, each flange `side - 2g` long (the corner squares notched out).
  A lip is cut at 45 degrees at its ends (its outer edge `flange - 2L`
  long; L at most 99 % of half its flange). Relief holes are circles on
  the four face corners, cut out of the outline.
- With corner tabs: the bottom and top flanges carry a tab at each end
  in the corner notch, beside a side that has a flange: as long as the
  tab length (at most that side flange's depth + g), set in by
  `c = min(max(g, 0.5), depth / 4)` from the flange's edges, its free
  corners cut at 45 degrees by `min(tab / 2, (depth - 2c) / 4)`; a fold
  line where it joins the flange, and with fixing holes a hole in its
  middle.
- Fold lines: the face edges with a flange, the lip folds, the tab
  folds, as separate open curves, dashed (0.2 mm, 3 mm on, 2 mm off) in
  blue. The cut outline and holes are red hairlines.
- Fixing holes: in each flange at half its depth, `max(1, ceil(len /
  spacing))` holes evenly spread at `(k + 1/2) len / n`.
- Legend: the text "prefix W x H x D" (D the deepest flange) on the
  face, 6 % of its shorter side high (3 to 100 mm), near its left edge.
- No bend allowance: the sizes are flat sizes.

### Window

About 960 x 720, resizable. At the left the white preview of the
pattern (cut lines red, folds blue) with "Shape size: W x H" under it
and in the yellow strip; the pattern follows the settings and the
selection at once (it is cheap), so Apply only works it out again. At
the right: the preset row, Method, tabs Size (group Size: Around the
selection, Width, Height (greyed while the selection gives the face),
Get from document (copies the selection's size into Width and Height);
First fold and Second fold groups: Left, Right, Bottom, Top and a Set
button with an All field that copies All to the four sides) and Options
(Corners: Corner gap, Corner relief holes, Tab length for corner tabs;
Holes: Fixing holes, Hole spacing, Hole diameter; Legend: Write the size
on the face, Prefix; Units). A thin progress bar is full while the
preview is current. Apply, OK (adds the previewed pattern, or works it
out and adds it, one step, and closes), Cancel.

## Letter returns (undo: "Letter Returns")

Every closed outline of each selected object gets a strip. Methods:
**Notched strip** (V notches on the top edge at the corners),
**Strip with bend marks** (a mark line at each corner from the top and
from the bottom edge, mark length 5 mm, at most a third of the height),
**Plain strip** (no corners). Settings: depth (50 mm, the strip
height), material thickness t (1 mm), length on Inner face / Middle /
Outer face (Inner), overlap (10 mm), also make the letter faces (off),
corners under (135 degrees, 0 = none), notch size (3 mm), bend
compensation c (0 mm), split strips longer than (off; 2000 mm when
turned on), space between strips (10 mm), number the strips (on).

- Length = face length + overlap. Face length = `L + s 2 pi d` with L
  the outline's arc length, d = 0, t/2 or t for the faces, s = +1 for
  an outer outline and -1 for a hole.
- Corners: every node (not the first, which is the strip's end) where
  the corner angle (180 minus the turn) is under the limit, at the
  node's arc length scaled by face length / L. A notch is as wide and
  deep as the notch size (at most half the height).
- Bend compensation: the face length is `c` shorter at each corner and
  the n-th corner moves `c (n - 1/2)` towards the start (c at most the
  face length / (corners + 1)).
- Split: a strip whose face length + overlap is over the maximum is cut
  into `ceil(face / (max - overlap))` equal pieces, each with the
  overlap, each corner on the piece it falls in.
- Faces: each letter's region offset inwards by t (mitred), at the
  letter's place, red hairline.
- Layout: strips one under another, left edges at the selection's left,
  the first one the strip space under the selection, each the space
  apart. Each strip is labelled with a text number inside its left end
  ("2", "2.1" when the object has more than one outline, "2.1b" for the
  second piece of a split strip), 0.4 x the height (1 to 20 mm).

### Window

About 1000 x 700, resizable. At the left the preview: before Apply the
selected letters, each closed outline's start node as a red dot with a
short red line showing its direction, and its corners (where notches or
marks go) as yellow dots; after Apply the letters in grey with the
strips, faces (red) and marks (blue); the yellow strip says how many
objects are selected, or "Strips: n  Total length: L" (also under the
drawing). At the right: the preset row, Method, a Strip group with the
method's name as its heading and its fields (Depth, Material thickness,
Length on, Overlap, Also make the letter faces; for Notched and Marked
Corners under, Notch size or Mark length, Bend compensation; Split
strips longer than and Longest strip) and Options (Space between
strips, Number the strips, Units). Progress bar, Apply (works the strips
out into the preview; nothing changes), OK (adds the previewed strips,
or works them out and adds them, one step, and closes), Cancel. A
changed setting, drawing or selection makes the preview out of date: it
shows the letters again.

## Liquid acrylic mould (undo: "Acrylic Mould")

Settings: outside contour (wall) w (2 mm), inside contour (off; 2 mm
when turned on), fill depth (5 mm), overlap (10 mm), space between
strips (10 mm), number the strips (on), density (1.18 g/ml).

- Base: each letter's region offset outwards by w (round corners), at
  the letter's place, red hairline.
- Inside contour: each letter's region offset inwards by the inside
  offset (round corners), red hairline.
- Wall strip: as a letter return on the Middle face of a material w
  thick (length `L + s pi w + overlap`), as high as the fill depth, no
  notches; laid out under the bases.
- Output: perimeter (the letters' outline length), area (their area,
  holes taken away), volume = area x fill depth / 1000 ml, mass =
  volume x density g.

### Window

As Letter Returns (1000 x 700, resizable; preview before Apply: the
letters). At the right: the preset row, Contours (Outside contour,
Make an inside contour, Inside offset), Wall strip (Fill depth, Overlap,
Space between strips, Number the strips), Material (Density, Units) and
Output (after Apply: Perimeter, Area, Volume, Mass; before: a hint).
Apply, OK and Cancel as for Letter Returns.

## Smart divide (undo: "Smart Divide")

Methods: **By tile size** (tile width 1220 mm and height 2440 mm, grid
from Selection corner or Page origin) and **Table** (columns 2, rows 1).
Settings for both: overlap (10 mm), spread tiles apart by (0 mm), name
the tiles (on), add the tile frames (off).

- By tile size: columns start every `width - overlap` from the origin,
  rows every `height - overlap`; the grid takes the column and row
  holding the selection's left and bottom edges and as many as it takes
  to reach the right and top edges.
- Table: the tiles just cover the selection: each is
  `(W + (cols - 1) overlap) / cols` wide and `(H + (rows - 1) overlap)
  / rows` high, from the selection's bottom-left corner.
- More than 2500 tiles, or an overlap not under the tile size, makes no
  grid (the window says so).
- Each tile holds each drawn member of each object cut to the tile's
  rectangle: closed outlines by boolean intersection (holes kept), open
  lines clipped as polylines. A piece keeps its member's fill, outline
  and opacity and becomes a curve. Tiles with no piece are skipped.
- Names: a row letter counted from the top (A, B, ..., Z, AA, ...) and
  a column number from the left: A1 is the top left. With names on,
  each tile gets its name as text near its top-left corner; with frames
  on, its rectangle as a dashed grey hairline. The tile group is named
  "Tile A1" and so on.
- Spread: the tile in column c and row r (from the bottom) moves by
  (c x gap, r x gap).
- The originals are deleted; the tiles are selected. Objects holding a
  bitmap are left out.

### Window

About 350 x 430. Method, an Options group (Overlap; Tile width, Tile
height and Grid from, or Columns and Rows; Spread tiles apart by, Name
the tiles, Add the tile frames; Units), a line with the grid ("3 x 2
tiles of W x H", or why there is none) and the last division. The grid
is marked on the drawing while the window is open (worked out again
only when the drawing, the selection or a setting changes). Apply
divides (the result cannot be shown before: it replaces the objects),
OK divides and closes, Cancel closes.

## Carrier (undo: "Carrier")

Methods: **Contour** (distance 10 mm, rounding r 0, fill the holes on)
and **Rectangle** (margins left, right, bottom, top 10 mm; corner radius
0).

- Contour: each object's region is offset outwards by distance + r
  (round joins), all are welded, and the result offset inwards by r:
  inner corners and narrow gaps get radius r, and gaps under
  `2 (distance + r)` close. Fill the holes keeps only the outer
  outlines. Letters far apart give separate pieces (the status line
  gives the count).
- Rectangle: the bounds of all the selected outlines (open ones too)
  grown by the margins, its corners rounded by the radius (at most half
  the shorter side).
- The outline (green hairline, no fill) goes into the lowest selected
  object's layer just under it.

### Window

About 330 x 470. Method; for Contour a group with Distance, Rounding
and Fill the holes; for Rectangle a Margins group (Left, Right, Bottom,
Top, Set and All) and Corner radius; Options: Show on the drawing (marks
the carrier on the drawing, worked out again only on a change), Units.
Apply adds the carrier, OK adds it and closes, Cancel closes.

## Left out (compared with what such tools often offer)

- LED: a user drawing as the diode shape, fill along rays from a centre,
  selecting LEDs under an outline or shape, a cost dialog beyond the
  report line.
- Neon: dark (painted) segments, connecting tubes, double-backs,
  elevation changes, filleting curves, dividing or supporting at a
  clicked point (no canvas tools in this round), select by colour.
- Letter returns: bending radii, tool diameters for milled strips, the
  "fix shapes" repair.
- Mould: pouring holes and their help contour.
- Sign box: wall mounts (keyholes), locks, a second set of folds with
  separate tools, auto creation on every change of the selection.
- Smart divide: manual cut lines on the drawing, removing the outlines
  at the cut edges.
- Carrier: grouping with the letters, strokes and frames.

## Checks

- `tracedraw_signcut::sign::tests::*`: regions keep closed subpaths
  and drop non-finite ones, even-odd inside and edge distance, outer
  contours, `closed_subpaths_and_sides`.
- `tracedraw_signcut::sign::led::tests::*`: matrix count and centring,
  shifted rows, holes and narrow parts, stroke centre, outline spacing,
  round diodes, bad input, power supply sizes, band intervals, the
  catalogue, `on_curve_spreads_modules_from_end_to_end`,
  `around_curve_makes_two_rows`, `island_fills_rings_inside`,
  `random_is_repeatable_and_keeps_its_spacing`,
  `fix_overlaps_drops_touching_modules`, `curve_modes_survive_bad_input`.
- `tracedraw_signcut::sign::neon::tests::*`: one tube, even splits,
  straight stretches, closed loops, supports, bad input,
  `short_lines_are_left_out`, `contour_tubes_run_inside_the_outline`.
- `tracedraw_signcut::sign::sign_box::tests::*`: cross pattern area,
  gaps and lips, `each_side_has_its_own_depth_and_lip`,
  `corner_tabs_sit_in_the_notches`, fixing holes, relief holes,
  degenerate sizes.
- `tracedraw_signcut::sign::returns::tests::*`: square strip and
  notches, faces and holes, smooth curves, stacking, mould, bad input,
  `long_strips_are_split_into_pieces`,
  `bend_compensation_shortens_at_each_notch`,
  `marks_sit_at_the_corners_on_both_edges`,
  `the_face_is_inset_by_the_thickness`,
  `a_mould_can_have_an_inner_contour`.
- `tracedraw_signcut::sign::tiles::tests::*`: grid, labels, spread,
  clipping, `a_table_of_tiles_covers_the_bounds`.
- `tracedraw_signcut::sign::carrier::tests::*`: one outline, far
  letters and rounding, holes, nothing closed,
  `a_rectangle_goes_round_everything_with_its_margins`,
  `a_rectangle_of_nothing_is_empty`.
- `signcut_sign::tests::*` (app): LED layout, report and undo, a
  selection needed, `leds_follow_an_open_curve`,
  `led_groups_are_found_and_their_overlaps_fixed`, neon split and
  supports, `neon_tools_divide_make_holes_convert_and_measure`,
  `contour_neon_runs_inside_closed_shapes`, sign box from the selection
  and the window size, `sign_box_plan_changes_nothing_and_writes_a_legend`,
  returns under the letters, `returns_plan_follows_the_method`, mould,
  `mould_plan_works_out_the_resin`, smart divide and
  `smart_divide_by_table`, carrier and
  `a_rectangle_carrier_has_its_margins`, the menu opening a window.
- `ui::signcut_win_sign::tests::*`:
  `every_window_draws_every_method_and_tab` (every tool, method and tab,
  before and after Apply, with and without a selection),
  `apply_previews_and_changes_nothing`,
  `ok_applies_the_previewed_plan_in_one_step`,
  `a_changed_drawing_makes_the_preview_stale`,
  `the_led_preview_marks_the_drawing`.
