# Sign & Cut: CNC and Laser

Tool paths for routers, lasers and hot wire foam cutters, and generators
of parts made on them. The geometry is in `tracedraw-signcut`
(`crates/signcut/src/cnc/`); the operations and their settings are in
`apps/tracedraw/src/signcut_cnc.rs`, the panel section in
`ui/signcut_cnc.rs`.

Open it from Tools > Sign & Cut > CNC and Laser (each item opens the
Sign & Cut panel at this section) or from the panel's CNC and Laser
section. Every value is in millimetres in the model and shown in the
document's units; feeds are in mm/min. Settings live for the session
(they are not saved with the document or between runs).

## Which objects

Like the rest of the panel: the selection, or with nothing selected
every visible, unlocked object of the page's layers. Groups are opened
into their members, a clip frame counts by its frame, text by its glyph
outlines. Each member keeps its own outline colour.

## G-code export

### Machines

A list of machines (Router, Laser, Hot wire at first); the one picked is
used. **+** adds a copy of the current one, **-** removes it (one always
stays). Machine settings:

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
  table placed at the zero point's corner) the panel says so in red;
  the file is still written.

### Tools

A list of tools (End mill 3 mm, End mill 6 mm, Laser beam, Hot wire at
first), **+** adds a copy. Each has a name, diameter, speed or power
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

**Export G-code...** saves the text through the save dialog (`.nc`,
also `.gcode`, `.ngc`, `.tap`; a download in the browser). **Show the
tool paths** draws the paths, lead-ins included, on the drawing while
the panel shows. The panel gives the number of tool paths and their
length over all passes. Nothing in the drawing changes; no undo step.

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
- **Make the Path** (also Tools > Sign & Cut > CNC and Laser > Hot Wire
  Path) adds it as one open curve with a red hairline outline, selected,
  in one undo step ("Hot Wire Path"); the status line gives its length.
- **Export Hot Wire G-code...** writes the same path in X and Y only for
  the machine in use (its Z axis ignored): header, `G0` to the start
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
- **Make the Box** adds the panels as closed curves with a red hairline
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
- **Make the Door** adds the outer and inner outlines (red hairline)
  and the grooves (blue hairline, for an engraving job of their own in
  the G-code export) as closed curves in one group, selected, one undo
  step ("Door Panel"). A frame that leaves no inner panel adds nothing
  and says so.

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
- `signcut_cnc::tests::door_panel_fills_the_selected_box_or_the_page_centre`,
  `signcut_cnc::tests::finger_joints_rewrite_two_objects_in_one_step`,
  `signcut_cnc::tests::finger_box_is_one_group_of_panels_in_one_step`,
  `signcut_cnc::tests::gcode_jobs_by_outline_colour`,
  `signcut_cnc::tests::gcode_follows_the_selection_and_opens_groups`,
  `signcut_cnc::tests::hot_wire_path_is_one_open_curve_in_one_step`.
