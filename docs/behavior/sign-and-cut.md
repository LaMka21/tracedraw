# Sign & Cut

Tools for sign making and cut files: how long and how large the work
is, the clean-up a plotter, router or laser file needs, and sizing. The
geometry is in the `tracedraw-signcut` crate; the panel and its
commands are in `apps/tracedraw/src/signcut.rs` and
`ui/signcut_docker.rs`.

Open it from Tools > Sign & Cut (each item opens the panel at its
section; Measure, Find Duplicates, Remove Duplicate Edges and Find
Crossings also run at once) or Window > Panels > Sign & Cut. The panel
has three sections: Measure (open at first), Clean Up and Size.

## Which objects

Measure and Clean Up work on the selection, in stacking order; with
nothing selected, on every visible, unlocked object of the page's
layers. The panel says which. An object's outlines are its drawn
outline in page space: a group by its members, a clip frame by its
frame only (its contents are cut off by it), text by its glyphs.

## Measure

- **Measure** fills a table: Objects, Length, Area, Outer area, Holes,
  Bounding box area, Bounding box (width x height), Paths (closed,
  open) and Nodes. Lengths are in the document's units, areas in square
  document units and in square metres.
- Length: the arc length of every segment (accuracy 0.0001 mm), the
  closing segment of a closed subpath included.
- Area: per object, even-odd: a closed subpath inside an odd number of
  larger closed subpaths of the same object is a hole, so the letter O
  counts its ring; an island inside a hole is material again. Outer
  area is the sum of the even ones, Holes of the odd ones, Area their
  difference. Open subpaths have length but no area. Objects are
  measured on their own and added up: an object lying on another is a
  separate piece.
- After the drawing changes the table stays and says it is out of date.

## Clean Up

Tolerance: 0.05 mm at first, at least 0.001 mm, shared by the three
tools.

- **Find Duplicates**: an object repeats an earlier one (lower in the
  stacking order) when the bounding boxes agree within the tolerance
  and every point of either outline lies within the tolerance of the
  other (outlines flattened to a quarter of the tolerance and sampled
  at least every tolerance). Start node, direction and object kind do
  not matter. With **Delete them** the repeats are deleted in one undo
  step ("Delete Duplicates"); otherwise they are selected. Locked
  objects are left alone.
- **Remove Duplicate Edges**: a stretch of outline that runs along an
  edge met earlier (in a lower object, or earlier in the same object)
  within the tolerance is taken out, so every edge is cut once. Straight
  segments lose only the shared stretch (both ends of the earlier
  straight segment within the tolerance of this one's line, the shared
  part longer than the tolerance); a curved segment goes when it
  follows an earlier segment from end to end. Pieces shorter than the
  tolerance are dropped. A changed object becomes an open curve (its
  remaining runs, a closed outline cut open joined across its start); an
  object whose every edge repeats is deleted. Rectangles, ellipses,
  polygons and curves without live effects take part, groups by their
  members; text, bitmaps and locked objects do not. One undo step
  ("Remove Duplicate Edges"); the status line gives the length taken
  out.
- **Find Crossings**: places where an outline crosses or touches
  itself, and with **Also between objects** where two objects' outlines
  cross. Outlines are flattened to a quarter of the tolerance;
  neighbouring segments of a subpath (and the last and first of a
  closed one) do not count, nor do segments lying along each other.
  Crossings closer than the tolerance for the same objects are one. The
  objects with crossings are selected and each crossing is marked with
  a circle of 6 px on the drawing while the panel shows and the drawing
  is unchanged.

## Size

The tool windows of Set Size, Reference Scale and Replace (and of
Perimeter and Area) are described in `sign-and-cut-calc.md`, Windows:
the Set Size window also sizes to the largest or smallest object, by a
change or a scale, and turns; Replace there also takes several masters.
The panel items below are the operations they share.

- **Set size**: Width and Height, each with a check box (Width on,
  100 mm; Height off, 100 mm); Keep proportions (on); Change: Every
  object, Only larger objects (maximum: shrink only) or Only smaller
  objects (minimum: grow only); Fixed point: one of nine points of the
  object's box (centre at first). Each selected object is sized on its
  own by its bounds, in one undo step ("Set Size"). With proportions
  kept and both sides given, the object fits inside them. A side of
  zero (a straight line) keeps its zero.
- **Reference scale**: Measure Width, Height or Length (outline length);
  **Use Selected Object as Reference** (one object selected) takes the
  object and its value, shown as "The reference measures ...". Enter its
  Real size; Apply scales uniformly by real size / measured value about
  the fixed point (bottom left at first) of the scaled objects' box:
  every unlocked object of the page's unlocked layers (Scale every object
  on the page, on at first), or the selection plus the reference. One
  undo step ("Reference Scale").
- **Replace**: **Use Selected Object as Master** (one object selected).
  Then select the objects to replace (the master may stay selected; it
  is skipped) and press Replace: a copy of the master is centred on each
  object, sized Keep the master's size, Fit inside each object
  (proportions kept, the default) or Stretch to each object; with Take
  each object's rotation (on) the copy turns by the difference of the
  objects' rotations and the sizes are taken along each object's own
  axes. The copies go on the replaced object's layer and are selected;
  the replaced objects are deleted unless Keep the replaced objects is
  on. One undo step ("Replace").

## Checks

- `tracedraw_signcut::measure::tests::*`: perimeter and area of a
  square and a circle, holes and islands, open lines, totals, empty
  input.
- `tracedraw_signcut::cleanup::tests::*`: duplicates whatever the start
  node and direction, a smaller object inside is not one, circles within
  the tolerance; two squares sharing a side cut it once, a partly shared
  side keeps its free part, an exact copy is removed, separate objects
  stay, a line drawn back over itself, non-finite input does not panic.
- `tracedraw_signcut::crossings::tests::*`: a figure eight, simple
  shapes, crossings between objects only when asked, an open line
  through a node twice.
- `tracedraw_signcut::sizing::tests::*`: anchors, proportional and free
  sizing, maximum and minimum, flat lines, reference factor, Replace
  fit and rotation.
- `signcut::tests::*` (app): measuring the selection and the page,
  duplicates selected or deleted and undone, a shared side taken out of
  the upper object, crossing marks until a change, Set Size per object,
  Reference Scale, Replace.
