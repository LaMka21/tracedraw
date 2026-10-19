# Sign & Cut: Cutting

Tools that prepare a job for a cutting plotter: weeding cuts, bridges,
the cut order, plotter output (HPGL), registration marks for print and
cut, and crop marks. The geometry is in `crates/signcut/src/cut/`; the
operations are in `apps/tracedraw/src/signcut_cut.rs` and the tool
windows in `apps/tracedraw/src/ui/signcut_win_cut.rs` (with its
submodules in `ui/signcut_win_cut/`). The clean-up windows Find
Duplicates, Remove Duplicate Edges and Find Crossings (operations in
`sign-and-cut.md`) are drawn there too.

Each tool has a window (see **Windows** below), opened from the Sign &
Cut toolbar, the Sign & Cut panel or Tools > Sign & Cut > Cutting. The
submenu has Cutting Panel (opens Plotter Output), then Add Weeding
Lines, Add Bridges, Optimize Cut Order, Plotter Output, Add
Registration Marks and Add Crop Marks, each opening its window.

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
One undo step ("Cut Order"); the status line and the window give the
travel before and after. With **Show the route** the knife-up moves the
window's preview shows are also drawn on the canvas as thin lines while
the window is open and the preview is current.

## Plotter output

Profiles are the presets of the Plotter Output window: a profile holds
every setting below, Optimize the cut order and the port. They are kept
by name in `signcut_presets.json` in the settings folder (tool key
`plotter_output`; in memory only in the browser).

| Setting | Default | Range |
| --- | --- | --- |
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
  a path that does not exist becomes a file. Nothing in the drawing
  changes; no undo step.

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

## Windows

All lengths are in the document's units; the Units row at the bottom
of the settings changes them. The windows with a preview are resizable;
the preview is white with the drawing fitted in it, y up, and a yellow
status strip under it. Each window works on the selection, or with
nothing selected on the whole page (a line under the settings says
which). What a window works out is kept with the drawing's revision, the
objects and the settings it was worked out for; when one of them
changes the result is out of date.

### Plotter Output (1000 x 700)

- Preview: the job as the plotter cuts it, turned and mirrored as the
  profile says: the outlines in cut order (dark), a red dot and arrow at
  each outline's start showing its direction, the knife-up moves as
  thin red lines from the origin, the weeding frame (blue) when on, and
  the plotter's 0,0 in green: a dot with arrows along both plotter axes
  pointing into the job. Under the drawing the job's size. Strip:
  "Outlines: n    Cut length: l    Travel: t" (all passes, from the
  plotter job). The preview is worked out again whenever the drawing,
  the objects, the profile or the cut order settings change (never
  every frame).
- Settings: the preset row (the plotter profiles); tabs:
  - **Job**: Cut order (Optimize the cut order, with a note that the
    direction and start points come from the Cut Order window);
    Placement (Origin, Mirror, Rotate 90); Weeding frame (Add a weeding
    frame, Margin).
  - **Plotter**: Commands (Units per mm, Pen up, Pen down, Start string,
    End string, New line after each command); Speed and force; Output
    (Port or share with the examples; desktop only).
  - **Knife**: Knife offset, Arc at corners sharper than (degrees),
    Overcut; Passes.
- Buttons: Save as .plt... and Send (desktop, greyed while no port is
  typed or nothing is to cut) at the left, Close at the right. Nothing in
  the drawing changes.

### Weeding Lines, Bridges and Cut Order (900 x 620)

Preview at the left, settings at the right, a thin progress bar (full
while the preview is current), Apply, OK and Cancel. Apply works the
result out and shows it; nothing in the drawing changes. Before Apply,
or when the result is out of date, the preview shows the objects in
grey and the strip "Objects: n. Press Apply to preview the result."
OK applies the previewed result when it is current, or works it out and
applies it, in one undo step, and closes. Cancel closes.

- **Weeding Lines**: Method (Lines or Boxes); Weeding lines (Gap from
  objects, Shorten lines by, Lines per side) or Weeding boxes (Box
  margin, width and height); Weeding frame (Add a weeding frame,
  Margin). Preview: the objects dark, the cuts in blue. Strip "Weeding
  cuts: n    Objects: m". OK: "Weeding Lines".
- **Bridges**: Bridge length; Bridges per outline (count) or One bridge
  every (distance), as radio buttons with their fields; Only outlines
  longer than. Preview: the outlines in grey, the bridged pieces dark
  over them (the gaps show grey) and a red ring around each bridge.
  Strip "Bridges: n    Outlines with bridges: m". OK: "Bridges".
- **Cut Order**: Cut order (Direction, Start point, Inner outlines
  first); Preview (Show the route, on the canvas too); the travel of
  the last Optimize. Preview: the cuts dark, knife-up moves red, a red
  dot and arrow at each start: the planned route, or with **Show
  Original** (bottom left) the route as drawn. Strip "Travel now: a
  Planned: b    Shorter by p %". OK: "Cut Order".

### Print and Cut Marks and Crop Marks (900 x 620)

The preview is always current (the marks are worked out again when the
drawing, the objects or a setting changes), so these windows have OK and
Cancel only. OK adds the marks in one undo step and closes.

- **Print and Cut Marks**: Marks (Around, Style, Corner length and
  width for L corners, Mark size, Offset, Extra marks every); Cut contour
  (Made from Outlines, Bounding box or None). Preview: the page (when
  the marks go inside it) or the objects' box in light blue, the objects
  in grey, the contour in magenta, the marks in black. Strip "Marks: n
  Cut contour outlines: m". OK: "Registration Marks".
- **Crop Marks**: Crop marks (Length, Offset, Outline width, Around each
  object). Preview: the objects in grey, the boxes in light blue, the
  marks in black. Strip "Crop mark corners: n". OK: "Crop Marks".

### Clean-up windows (360 x 230)

A group with the Tolerance (shared by the three) and the tool's
options, the line saying which objects are used, and the last result
in bold (before the first run, what to press).

- **Find Duplicates**: Delete them (else select them). Apply runs it
  (selects or deletes; deleting is one undo step "Delete Duplicates")
  and keeps the window; OK runs it and closes; Cancel closes.
- **Remove Duplicate Edges**: the explanation of what happens. Apply,
  OK and Cancel as above ("Remove Duplicate Edges").
- **Find Crossings**: Also between objects. It changes nothing in the
  drawing, so it has Find at the left and Close at the right. The
  crossings found are marked on the canvas with small circles while the
  window is open and the drawing has not changed; the result line then
  says none are left.

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
  with travel before and after and curves only when needed,
  `route_compute_changes_nothing_and_shows_both_routes`,
  `previews_change_nothing_and_commit_in_one_step`,
  `plotter_settings_read_back_as_a_preset`, plotter output in route
  order without marks, sending to a file path, registration marks with
  the cut contour and its ink, crop marks around each object or all,
  nothing changing on an empty page.
- `tracedraw_signcut::cut::hpgl::tests::the_layout_marks_the_origin_corner_and_the_frame`,
  the bridge centres in `a_square_gets_two_bridges_in_the_middle_of_its_sides`
  and the moves as drawn in `nearest_neighbour_shortens_a_zigzag`.
- Windows (`ui::signcut_win_cut::...`):
  `tests::every_window_draws_with_nothing_on_the_page`,
  `tests::start_marks_follow_the_first_segment`;
  `plotter::tests::plotter_window_draws_every_tab`,
  `the_preview_shows_cuts_moves_starts_frame_and_origin`,
  `nothing_to_cut_gives_an_empty_preview`;
  `prepare::tests::weeding_window_draws_both_methods_and_previews`,
  `weeding_ok_adds_the_previewed_cuts_in_one_step`,
  `bridges_window_draws_both_counts_and_previews`,
  `cut_order_window_shows_the_planned_and_the_drawn_route`;
  `marks::tests::print_and_cut_window_draws_every_style_and_previews_live`,
  `crop_window_previews_each_object_or_all`;
  `cleanup::tests::duplicates_window_draws_and_finds_in_one_step`,
  `edges_window_draws_and_removes_in_one_step`,
  `crossings_window_marks_what_it_found`.

## Not done

- Weeding lines run along the box sides' perpendiculars only (no
  diagonal or shape-following weeding).
- The cut order does not move objects between layers, and objects inside
  groups are ordered with their group.
- The previews draw outlines only: filled marks show as their outlines.
- The windows were checked by headless drawing tests, not looked at on
  screen yet.
