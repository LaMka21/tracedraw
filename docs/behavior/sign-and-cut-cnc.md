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
- `signcut_cnc::tests::gcode_jobs_by_outline_colour`,
  `signcut_cnc::tests::gcode_follows_the_selection_and_opens_groups`,
  `signcut_cnc::tests::hot_wire_path_is_one_open_curve_in_one_step`.
