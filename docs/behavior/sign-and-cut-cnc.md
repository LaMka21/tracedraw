# Sign & Cut: CNC and Laser

Tool paths for routers, lasers and hot wire foam cutters, and generators
of parts made on them. The geometry is in `tracedraw-signcut`
(`crates/signcut/src/cnc/`); the operations and their settings are in
`apps/tracedraw/src/signcut_cnc.rs`, the windows in
`ui/signcut_win_cnc.rs` and `ui/signcut_win_cnc/gcode.rs`.

Each tool has a window of its own (see "Windows" below), opened from the
Sign & Cut toolbar (Sign Making > G-code; Construct > Centre Line; CAD
and Machining > Hot Wire Path, Finger-Joint Box, Finger Joints, Door
Panels), from the Sign & Cut panel, or from Tools > Sign & Cut > CNC and
Laser: Export G-code..., Hot Wire Path, Finger-Joint Box..., Finger
Joints on Edges..., Door Panel... and Centre Line. Every value is in
millimetres in the model and shown in the document's units; feeds are
in mm/min. The machine, the finger-joint box and the door can be saved
as presets (kept between runs, see `sign-and-cut-windows.md`); the
other settings live for the session (they are not saved with the
document or between runs).

## Which objects

Like the other Sign & Cut tools: the selection, or with nothing selected
every visible, unlocked object of the page's layers. Groups are opened
into their members, a clip frame counts by its frame, text by its glyph
outlines. Each member keeps its own outline colour.

## G-code export

### Machines

The machine is chosen with the preset row of the G-code window: the
presets Router, Laser and Hot wire are offered whenever the machine
preset list is empty, Router at first; Save in the preset row keeps the
machine as it is under a name. Machine settings:

| Setting | Router | Laser | Hot wire |
|---|---|---|---|
| Table (width x height) | 1300 x 2500 | 1300 x 900 | 1000 x 1000 |
| Zero point | bottom left of the page | same | same |
| Z axis | on | off | off |
| Header lines | `G21` `G90` `G17` | `G21` `G90` | `G21` `G90` |
| Footer lines | `M5` `M30` | `M5` `M2` | `M2` |
| Tool change | `T{tool} M6` `M3 S{rpm}` | none | none |
| Beam on / off (no Z axis) | `M3 S{rpm}` / `M5` | same | same |
| Comments | `( )` | `;` | `( )` |
| Decimals | 3 | 3 | 2 |

- Zero point: bottom left, bottom right, top left, top right or centre
  of the page. Machine coordinates are page coordinates (y up) minus
  that point; axes are not turned or mirrored.
- In the header, footer, tool change and beam lines `{tool}` (tool
  number from 1), `{name}`, `{diameter}`, `{rpm}` and `{feed}` are
  replaced by the tool's values. Empty lines are skipped.
- Comments: `(text)` (parentheses in the text become brackets),
  `; text`, or none. The file starts with the machine's name as a
  comment and each tool change with `Tool n: name, diameter d`.
- Decimals: 0 to 6, fixed (`X10.000`); `-0` is written `0`. Feeds and
  speeds are written as whole numbers (one decimal if they have one).
- When the tool paths leave the table (in machine coordinates, the
  table placed at the zero point's corner) the window's strip says so;
  the file is still written.

### Tools

A list of tools (End mill 3 mm, End mill 6 mm, Laser beam, Hot wire at
first). Add a copy adds a copy of the shown tool; Remove deletes it (one
always stays; jobs that used it take the first tool, the others keep
theirs). Each has a name, diameter, speed or power
(`{rpm}`), feed and plunge (mm/min), deepest pass (0: no limit), safe
height, and an optional park position (machine coordinates) visited at
the end. End mills: 18000 rpm, feed 1200, plunge 300, deepest pass half
the diameter (at least 0.5), safe height 5.

### Jobs by outline colour

**Find Colours** adds a job for each outline colour of the objects that
has none (an object without an outline counts by its plain fill colour,
else black). The menu item does it when the list is empty. Jobs are cut
in list order (arrows move a job, Remove deletes it); a switched-off job
is skipped. Each job:

| Setting | Default | Meaning |
|---|---|---|
| Tool | first | from the tool list |
| Depth | 3 mm | total depth, 0 for a laser |
| Passes | 1 | the depth is split evenly: pass k of n at Z = -depth k / n; when depth / deepest pass needs more, n = ceil(depth / deepest pass) |
| Cut side | Outside | On the line; Outside: closed outlines offset by the tool radius so the part keeps its size (its holes offset inwards); Inside: the other way, the opening keeps its size. Offsets use the shaping offset with round corners. Open curves are always cut on the line |
| Direction | Climb | for a spindle turning clockwise: Climb keeps the material on the right of the travel (outer outline of a part clockwise, a hole counter-clockwise), Conventional the other way. Open curves keep their direction |
| Inner outlines first | on | an outline inside another (by its first point, smaller area) is cut before it |
| Lead-in | None | Straight: a line of the lead length square to the first segment, from the waste side; Arc: a quarter circle of radius the lead length that meets the outline along its first segment. Closed outlines only |
| Bridges | 0 | per closed outline, centred at (i + 0.5) L / n along it; each leaves the bridge length (4 mm) uncut plus the tool diameter of lifted travel; skipped when they would take 80 % of the outline or more |
| Bridge height | 1 mm | with a Z axis the tool rises to Z = -depth + height over a bridge, on the passes deeper than that; without one the beam is switched off and the bridge crossed with `G0` |

Order: among the outlines that are ready (inner ones done), the one
whose start is nearest to the tool goes next; a closed outline starts
at its node nearest to the tool, an open one at its nearer end. The tool
starts at the machine's zero.

### Writing

For each cut: retract to the safe height (`G0 Z`), `G0` to the entry
point (lead-in start or first point), `G1 Z` down at the plunge feed,
the lead-in and the outline at the feed (`G1 X Y`, F written when it
changes), back to the start on a closed outline. Further passes of a
closed outline without a lead-in step down where they are; others
retract and come back. Without a Z axis the beam lines replace the
plunge and retract. At the end: retract, park position if set, footer.
Only `G0` and `G1` are written; lead-in arcs are 8 short lines. Curves
are flattened within 0.02 mm.

**Save G-code...** saves the text through the save dialog (`.nc`,
also `.gcode`, `.ngc`, `.tap`; a download in the browser). **Show the
tool paths** (on at first) draws the paths, lead-ins included, on the
drawing while the G-code window is open; they are made again only when
the drawing, the selection or a setting changes. The window gives the
number of tool paths, their length over all passes and the time they
take at the tools' feeds (length of each path times its passes over the
feed; travel, plunges and acceleration not counted). Nothing in the
drawing changes; no undo step.

## Hot wire path

For foam cutting with a wire that cannot be lifted: one open curve
through every closed outline of the objects (open curves are ignored).

- Start point: X 0, Y 0 (the page's bottom left) at first, in page
  coordinates.
- The outlines (flattened within 0.05 mm; each subpath on its own, so a
  hole is an outline of its own) are joined into a tree: starting from
  the start point, the outline with the shortest straight connection to
  the start point or to an outline already joined is joined next
  (Prim). Connections run between the nearest points (node to segment
  either way).
- The path goes from the start point to the entry of the first outline,
  follows each outline once in its drawn direction from its entry round
  to it again, and where a connection leaves an outline goes out along
  it, cuts that outline (and its own branches) and comes back the same
  way. It ends at the start point. Its length is the sum of the
  outlines plus twice the sum of the connections.
- OK adds it as one open curve with a red hairline outline, selected,
  in one undo step ("Hot Wire Path"); the status line gives its length.
- **Save G-code...** writes the same path in X and Y only for the
  machine chosen in the window (Hot wire at first; any machine preset;
  its Z axis ignored): header, `G0` to the start
  point, beam (heat) on with the chosen tool (Hot wire at first: feed
  300 mm/min, `{rpm}` 50), `G1` along the path, beam off, footer.

## Finger-joint box

The flat panels of a box W x D x H joined by finger joints on every
edge. Defaults: 200 x 150 x 100 mm outside, material thickness T 3 mm,
finger width 10 mm, kerf 0.1 mm, gap 5 mm, closed top.

- **Inside sizes**: the sizes are inside the box; outside = inside + 2 T
  (+ T only for the height when the top is open).
- Panels: front and back W x H, left and right D x H, bottom and top
  W x D (no top with **Open top**: five panels).
- Fingers: along an edge of outside length L the length between the
  two corner cubes, L - 2 T, is split into n equal fingers, n odd and
  the finger width L' / n nearest to the wanted width (n = round(L' / F),
  made odd towards the nearer width; at least 1). Both panels of an
  edge use the same n, so they always mate. The fingers go in turn to
  the two panels, the first and last to the panel of lower rank (bottom
  and top, then left and right, then front and back); a finger a panel
  does not own is cut T deep into it. The eight corner cubes belong to
  the front and back. An edge with no panel (the open top) is straight.
  Each piece of the box's shell is in exactly one panel: the panels'
  areas times T equal the shell's volume.
- Kerf: each outline is offset outwards by half the kerf (sharp
  corners), so fingers come out wider and slots narrower by the kerf.
- Layout: front, back, left and right in a row above bottom and top,
  starting at the page's bottom left plus the gap, panels a gap (plus
  the kerf) apart.
- OK adds the panels as closed curves with a red hairline
  outline in one group, selected, one undo step ("Finger-Joint Box").
  Sizes that cannot make a box (a side not longer than 2 T, a thickness
  or finger width of 0) add nothing and say so.

## Finger joints on edges

Cuts fingers into straight edges of existing closed outlines so two
flat parts interlock at a corner. Defaults: finger width 10 mm, depth
3 mm (the thickness of the mating part), minimum edge length 30 mm,
end fingers kept.

- Edges: straight segments (lines, or curves whose handles lie on
  their chord within 0.000001 mm) of closed subpaths, at least the
  minimum length long. Curved segments, open subpaths and shorter
  edges are kept as they are.
- An edge of length L takes n fingers, n odd and L / n nearest to the
  finger width (as for the box). Every other finger is cut in by the
  depth, square to the edge and into the material (the material side
  from the subpath's direction and even-odd nesting: a hole's material
  is outside it). Normally the slots are fingers 2, 4, ...; with **Cut
  the end fingers** they are 1, 3, ..., n, the mating pattern.
- **Two objects: join their nearest edges** (the default): exactly two
  objects must be selected (or be the targets); among their long
  straight edges the pair whose middles lie nearest is joined, both
  with the finger count of the shorter edge (each edge split over its
  own length), the second object with the opposite pattern of the
  first.
- **Every straight edge at least the minimum length**: each object gets
  fingers on all its long straight edges, with the chosen pattern.
- Rectangles, ellipses, polygons and curves without live effects take
  part (groups opened); text, bitmaps and locked objects do not. A
  changed object becomes a closed curve in the same place, keeping its
  fill and outline. One undo step ("Finger Joints").
- Limits: no kerf allowance and no check that the slots fit the part
  (a depth larger than the part's width cuts through it); fingers run
  square to the edge, so parts meet at right angles.

## Door panels

Outlines for a cabinet door or panel. Defaults: 400 x 600 mm, frame
60 mm, square inner panel, corner radius 20 mm, arch rise 60 mm, no
grooves (first groove 10 mm inside, then every 5 mm).

- Outer outline: with exactly one object selected, its bounding box
  (size and place); otherwise a W x H rectangle centred on the page.
- Inner panel: the outer rectangle inset by the frame width, with
  - Square corners;
  - Rounded corners: radius at most half the inner panel's shorter
    side;
  - Arched top: the top side replaced by a circular arc through the two
    top corners lowered by the rise and the middle of the top (radius
    (c^2 + h^2) / 2h for half width c and rise h); the rise is at most
    half the width (a half circle) and the panel's height.
- Groove lines: n closed outlines offset inwards from the inner panel
  by the first distance, then each by the spacing more (sharp corners
  for a square panel, round otherwise); one that vanishes ends the
  row. Grooves with a distance that is not positive are skipped.
- OK adds the outer and inner outlines (red hairline)
  and the grooves (blue hairline, for an engraving job of their own in
  the G-code export) as closed curves in one group, selected, one undo
  step ("Door Panel"). A frame that leaves no inner panel adds nothing
  and says so.

## Centre line from shapes

An approximate centre line (medial axis) of closed shapes such as thick
strokes and letters, for single-line engraving and neon. Defaults:
tolerance 0.2 mm, remove branches shorter than 2 mm. Each drawn object
is treated on its own, its subpaths by even-odd (holes).

- Sampling: the closed subpaths are flattened within a quarter of the
  tolerance and sampled at a spacing of the tolerance, coarser for
  large shapes so that one shape has at most 6000 samples (spacing =
  max(tolerance, perimeter / 6000)). Each sample is moved by at most a
  thousandth of the spacing (a fixed pattern) so that points on one
  circle do not make the triangulation ambiguous.
- The samples are triangulated (Delaunay, Bowyer-Watson with a sweep
  along x). Triangles whose centroid lies inside the shape count;
  slivers (three samples following each other along the outline, or a
  triangle less high than 5 % of the spacing) are left out. Each
  triangle stands for its circumcentre (its centroid when the
  circumcentre lies outside the shape or farther from it than its
  longest side); triangles sharing a side are joined.
- Pruning: a branch from a free end to a junction is removed when it
  is shorter than the prune length or than 1.5 times the half width at
  the junction (this takes off the branches into convex corners, about
  1.41 half widths long); a junction always keeps one branch. Repeated
  until nothing changes.
- The rest is split into lines between ends and junctions (round a
  hole: a closed loop) and simplified within the tolerance
  (Douglas-Peucker); lines shorter than the tolerance or the prune
  length are dropped.
- Results: lines end about half a stroke width before square ends (as
  a medial axis does); round a reflex corner the line keeps its
  distance from it.
- OK adds the lines as open curves (closed ones round holes)
  with a blue hairline, grouped, selected, one undo step ("Centre
  Line"); the status line gives their number.

## Windows

Every window has the dialog look of the Sign & Cut windows, can be
resized, and works on the objects it sees when a button is pressed.
The windows with Apply, OK and Cancel share one layout: a white preview
at the left with the yellow status strip under it, the settings column
at the right (group boxes, then the Units row), a thin progress bar that
is full while the preview shows the drawing and the settings as they
are, and Apply, OK, Cancel at the bottom right.

- **Apply** works the result out into the preview (and puts the strip's
  text in the status line); nothing in the drawing changes. A preview is
  made from the drawing, the selection, the page and the settings; when
  any of them changes it is out of date. Hot Wire Path, Finger Joints,
  Finger-Joint Box and Door Panels are quick: their preview is worked
  out again as soon as it is out of date. Centre Line is slow on large
  shapes: its preview waits for Apply and, out of date, shows the
  objects only.
- **OK** applies the previewed result when it is current, else works it
  out first, in one undo step (labels as above), and closes.
- **Cancel** (or the close button) closes; nothing changes.

In the previews, the objects are grey, cut lines red, grooves, centre
lines and the edges that take fingers blue.

### G-code

About 1000 x 700. At the left the preview: the objects in grey, the
tool paths in their jobs' colours (too light colours in blue), the
machine's table as a grey rectangle and its zero point as a red circle;
under it the size of the tool paths. The strip: "Tool paths: n  Cut
length: l  About t min", with the off-table warning after it, or why
there is nothing to cut. At the right the machine preset row (see
Machines), the tabs Machine, Tools and Jobs, and Units.

- Machine: group Machine (Name, Table width, Table height, Zero point,
  Z axis), group Output (Decimals 0 to 6, Comments), group G-code lines
  (Header, Footer, Tool change, Beam on and Beam off, the beam lines
  greyed while the machine has a Z axis).
- Tools: group Tools (the tool shown, Add a copy, Remove), group Tool
  (Name, Diameter, Speed or power, Feed, Plunge, Deepest pass, Safe
  height, park position with X and Y).
- Jobs: a table of the jobs (switched on, colour swatch and `#rrggbb`,
  tool, depth; a click on the colour picks the job), Find Colours, move
  up, move down, Remove; then group Job #rrggbb for the picked job
  (Tool, Depth, Passes, Cut side, Direction, Inner outlines first,
  Lead-in and its length, Bridges with their length and height).
- When the window first draws and there are no jobs, it finds the
  outline colours.
- Bottom: Save G-code... and Show the tool paths at the left, Close at
  the right.

### Hot Wire Path

About 820 x 520. Preview: the objects, the path in red and the start
point as a red circle; under it the path's length. Strip: "Closed
outlines: n  Path length: l". Settings: group Start point (X, Y), group
G-code (Machine: one of the machine presets, Hot wire at first; Tool:
Hot wire at first). Bottom left: Save G-code... (the path in X and Y).

### Finger Joints

About 820 x 520. Preview: the objects that can take fingers in grey,
their new outlines in black and the edges that take fingers in blue.
Strip: "Objects changed: n  Edges: e", or why nothing would change (two
objects needed, no edge long enough). Settings: group Edges (Method:
Two objects or Every long edge, with the method's description; Minimum
edge length), group Fingers (Finger width, Depth (mating thickness),
Cut the end fingers).

### Centre Line

About 820 x 520. Preview: the objects and, after Apply, the centre
lines in blue. Strip: "Closed outlines: n. Press Apply..." before
Apply, "Centre lines: n" after. Settings: group Centre line
(Tolerance, Remove branches shorter than).

### Finger-Joint Box

About 900 x 600. Preview: the panels as they will be laid out; under it
the outside size W x D x H and the fingers per edge (along the width,
the depth and the height). Strip: "Panels: n  Laid out: w x h". Settings:
the preset row (tool key `finger_joint_box`), group Box size (Width,
Depth, Height, Inside sizes, Open top), group Material (Material
thickness, Finger width, Kerf), group Layout (Gap between parts).

### Door Panels

About 900 x 600. Preview: the selected object (when exactly one is
selected) in grey, the outer and inner outlines red, the grooves blue;
under it the door's size. Strip: "Door: w x h  Grooves: n". Settings:
the preset row (tool key `door_panels`), group Door size (Width and
Height, greyed while one object gives the size), group Frame and panel
(Frame width, Inner panel: Square corners, Rounded corners with the
Corner radius, or Arched top with the Arch rise), group Grooves (Groove
lines, First groove inside, Groove spacing, greyed without grooves).

## Limits

- G-code: no arcs (`G2`/`G3`), no pocketing or ramped plunges; tools
  and jobs are not kept between runs (machines are, as presets).
- The hot wire path keeps each outline's drawn direction and does not
  avoid crossing other outlines with its straight connections (they
  are the shortest ones, which normally run through waste).
- Finger joints work on straight edges only, at right angles, without
  kerf allowance (the box has one).
- The centre line is an approximation within about the sample spacing;
  very thin parts (thinner than a few sample spacings) may lose their
  line.

## Checks

- `tracedraw_signcut::cnc::tests::*`: flattening with non-finite points,
  nesting of holes and islands, simplification.
- `tracedraw_signcut::cnc::gcode::tests::*`: golden G-code for a square
  on the line (`golden_square_on_the_line`), two passes stepping down in
  place with the top-left zero and a park position
  (`golden_two_passes_step_down_in_place_with_origin_and_park`), a laser
  with a bridge as a gap (`golden_laser_without_z_and_bridges_as_gaps`),
  a hot wire path (`golden_hot_wire_xy`); pass depths, bridges only on the
  passes below their top, tool radius offsets, inner outlines first and
  directions, lead-ins from the waste side, tool changes and the table
  check, non-finite input.
- `tracedraw_signcut::cnc::hotwire::tests::*`: one square from a start
  point, three squares joined by their shortest connections cut twice, a
  hole entered from its outline, nothing closed or non-finite input.
- `tracedraw_signcut::cnc::fingerbox::tests::*`: odd finger counts,
  the panels fill the shell exactly (`panels_fill_the_shell_exactly`),
  open top with inside sizes, the front panel's corners and slots,
  kerf, impossible sizes.
- `tracedraw_signcut::cnc::joints::tests::*`: odd counts, long edges
  only with their material side (holes too), every other strip taken
  out (and the flipped pattern), two parts with mating patterns on
  their nearest edges, curves kept and bad input.
- `tracedraw_signcut::cnc::door::tests::*`: square panel with grooves,
  rounded corners by area, the arch by its crown and area (and the half
  circle limit), a frame too wide and non-finite values.
- `tracedraw_signcut::cnc::centerline::tests::*`: a bar has one line
  down its middle, a square ring and a round ring have one closed loop
  half way between, an L keeps both arms, a long bar within the sample
  cap, empty and non-finite input.
- `signcut_cnc::tests::centre_lines_of_a_bar_and_a_ring_in_one_step`,
  `signcut_cnc::tests::door_panel_fills_the_selected_box_or_the_page_centre`,
  `signcut_cnc::tests::finger_joints_rewrite_two_objects_in_one_step`,
  `signcut_cnc::tests::finger_box_is_one_group_of_panels_in_one_step`,
  `signcut_cnc::tests::gcode_jobs_by_outline_colour`,
  `signcut_cnc::tests::gcode_follows_the_selection_and_opens_groups`,
  `signcut_cnc::tests::hot_wire_path_is_one_open_curve_in_one_step`,
  `signcut_cnc::tests::gcode_preview_has_job_colours_and_a_time`,
  `signcut_cnc::tests::removing_a_tool_keeps_the_jobs_on_their_tools`,
  `signcut_cnc::tests::settings_round_trip_as_presets`.
- Windows: `ui::signcut_win_cnc::tests::every_window_draws_every_tab_and_method`
  (every tab of G-code, both joint methods, the three inner panel
  shapes, and every window on an empty page),
  `gcode_preview_shows_tool_paths_and_the_table`,
  `machine_presets_offer_the_built_in_machines`,
  `apply_previews_and_changes_nothing`, `ok_applies_in_one_undo_step`,
  `ok_without_apply_works_it_out_and_cancel_changes_nothing`.
