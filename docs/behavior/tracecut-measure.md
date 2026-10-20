# traceCUT: Measure

traceCUT is TraceDraw's set of tools for cutting plotters, lasers and sign
making. Its geometry lives in the `tracedraw-tracecut` crate (depends on
`core` only); `apps/tracedraw/src/tracecut.rs` picks the objects and turns
them into page-space outlines. The Measure docker is its first tool.

## The docker

Window > Panels > traceCUT Measure, in the group with Transformations and
Coordinates. Help topic: `objects/measure`.

- **Targets:** the selection (objects picked inside groups included), or
  with nothing selected every visible, unlocked object of the page, in
  drawing order. A line under the title says which.
- **Rows:** Perimeter and Area (bold), Outer area, Holes; Bounding box
  (width x height) and its area; Objects, Paths ("n closed, m open"),
  Nodes.
- **Units:** lengths in the drawing's unit (`Units::world_from_mm`, so the
  drawing scale applies) with its short name; areas in its square
  ("mm²", "cm²"). Decimals: Options > Precision, at most 10.
- **Updates:** the measurement is cached per drawing, page, engine revision
  and target list, and taken again whenever one of them changes (undo
  included); nothing to press.

## Formulas

- **Outlines:** each object in page space (`Document::shape_on_page`);
  groups open into their members, a clip frame counts its frame only.
- **Perimeter:** the arc length of every segment (curves to 1e-4 mm), the
  closing segment of a closed subpath included; non-finite lengths are
  skipped.
- **Closed or open:** a subpath closes with a close element or by ending
  within 1e-6 mm of where it starts.
- **Areas, per object:** each closed subpath's absolute area; a subpath
  inside an odd number of larger ones (tested at the middle of its first
  segment) is a hole, else outer (islands in holes count as outer). Area =
  outer - holes. Open subpaths have no area. Objects are measured on their
  own and added up, so overlapping objects count twice.
- **Bounding box:** the union of the outlines' boxes; its area is width x
  height.
- **Nodes:** per subpath its start and the end of each segment; a closed
  subpath whose last segment returns to its start counts that point once
  (a rectangle has 4).

## Checks

- `tracedraw-tracecut`: a square (perimeter 40, area 100), a circle (2πr,
  πr²), a hole with an island, open lines, two objects sharing a box, empty
  input; a closed outline's start node counts once; groups open in page
  space; subpaths know whether they close.
- `tracecut::tests`: the selection or else the page; a change and its undo
  are measured again; hidden objects count only when selected.
- `ui::measure_docker::tests::values_follow_the_drawings_unit`.
- `ui::tests::window_draws_with_every_docker_tool_and_selection` draws the
  docker.
