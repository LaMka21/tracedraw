# Sign & Cut: Nesting

Placing objects on sheets or a roll with the least waste, and filling a
sheet with copies. The geometry is in `crates/signcut/src/nest/`
(`rects.rs`, `shapes.rs`, `fill.rs`, with `poly.rs` for polygons and the
cell grid); the operations and settings are in
`apps/tracedraw/src/signcut_nest.rs`, the panel section in
`apps/tracedraw/src/ui/signcut_nest.rs`.

Open it from Tools > Sign & Cut > Nesting (Rectangle Nesting, Shape
Nesting, Fill Sheet: each opens the panel at the Nesting section with
that tool chosen; nothing runs from the menu) or from the Nesting
header of the Sign & Cut panel. The tool is picked with the three
buttons at the top of the section.

## Which objects

The selection, each selected object moved as one piece (a group with
its members). Locked objects are left alone; an object picked inside a
selected group goes with its group. With nothing selected the tools do
nothing and say so. An object's outline is what a cutter would follow
(`tracedraw_signcut::outlines`: a group by its members, a clip frame by
its frame).

## The material

Shared by the three tools, in the document's units:

| Setting | Default | Range |
| --- | --- | --- |
| Sheets or Roll | Sheets | |
| Page size (sheets) | on: the sheet is the page | |
| Width x Height (sheets, page size off) | 1220 x 2440 mm | at least 1 mm |
| Roll width (roll) | 1220 mm | at least 1 mm |
| Length runs (roll) | to the right | to the right, upward |
| Margin | 0 mm | 0 and up |
| Spacing (nesting only) | 2 mm | 0 and up |
| Start corner | bottom left | the four corners |
| Add sheet frames (nesting only) | on | |

- **Sheets** are laid side by side: sheet 1 from the page's origin
  (bottom-left of the page), each next sheet to the right of the last,
  20 mm apart (`SHEET_GAP`). Nothing is added as pages.
- A **roll** starts at the page's origin. Its length runs to the right
  (the roll width up the page) or upward (the roll width to the right).
  For a roll to the right only the bottom/top half of the start corner
  counts (which edge of the roll is filled first); for a roll upward
  only its left/right half.
- The **start corner** is where filling begins: rows fill across from
  it, then away from it.
- **Sheet frames**: after a nesting run, a group named "Sheets" of
  hairline rectangles (no fill, outline RGB 0, 0.55, 0.85), one per sheet
  used, named "Sheet 1", "Sheet 2" and so on; on a roll one rectangle of
  the roll width by the length used. It goes on the active layer, in
  the same undo step. Deselect it before nesting again.
- While the section is open the drawing shows the first sheet (or the
  roll, as long as the last run's length, else as long as the page's
  longer side) as a preview outline.

## Rectangle nesting

Objects are placed by their bounding boxes (of the outlines, not the
outline width).

- Order: largest box area first (ties in selection order).
- Each sheet keeps the list of its largest free rectangles (MaxRects).
  A box goes into the free rectangle that leaves the shortest side over
  (best short side fit); on a roll it goes where its top ends lowest,
  then leftmost. Every free rectangle the box covers is split into the
  parts around it; free rectangles inside others are dropped.
- **Allow 90 degree turns** (on): a box is also tried turned, and takes
  whichever turn scores better.
- Spacing: each box and the usable area grow by the spacing, so boxes
  end up at least the spacing apart and the margin from the edges.
- A box takes the first sheet it fits on; when none has room a new sheet
  is opened. A box that does not fit an empty sheet (within the margins,
  in either turn) is left where it is.
- Each placed object gets one page-space transform (a move, and a
  quarter turn about its box when turned). One undo step:
  "Nest Rectangles".

## Shape nesting

Objects are placed by their true outlines.

| Setting | Default | Range |
| --- | --- | --- |
| Turns | every 90 degrees | none; 0 and 180; every 90; every 45 |
| Accuracy | 0.5 mm | 0.01 to 50 mm |
| Place parts in holes | on | |
| Improve the layout | on | |
| Rounds | 100 | 1 to 100000 |
| Time limit | 5 s | 0.1 to 600 s |
| Seed | 1 | 0 to 999999 |

1. **Regions.** Each path of an object becomes a region: closed
   subpaths flattened to accuracy / 20, even-odd inside the path (the
   letter O keeps its hole); open subpaths count as their bounding box
   grown by 0.01 mm. The paths of a group are welded. With **Place parts
   in holes** off, holes are filled. The region is offset outward by
   `d = spacing / 2 + 0.3 x accuracy` with round corners
   (`tracedraw_core::shaping::offset`) and simplified within accuracy / 4.
   The extra `0.3 x accuracy` covers the flattening and the
   simplification, so the region always holds the outline grown by at
   least half the spacing: two objects end up at least the spacing apart
   (straight edges up to `0.6 x accuracy` more), and every outline at
   least the margin from the sheet's edges.
2. **Turns.** Each allowed turn of the region is drawn on a grid of
   square cells, `max(accuracy, usable width / 256)` wide; a cell is set
   when any part of the region (area or edge) lies in it, so the drawn
   cells always cover the region.
3. **Bottom-left fill.** Objects go largest region area first. For each
   open sheet in turn and each turn, the grid gives the lowest row, then
   the leftmost cell, where none of the object's cells meets a set cell of
   the sheet (a blocked run skips straight past the cell blocking it).
   The candidate is checked with the exact polygon test, then slides down
   and left in true geometry: each slide is a binary search (12 steps)
   over up to three cells, accepting only places the exact test passes,
   repeated (at most 8 times) until it no longer moves. The exact test:
   two regions overlap when any of their edges meet (touching counts) or
   a ring of one lies inside the other's area (holes are free). The turn
   whose top ends lowest wins (then the leftmost). An object fitting on no
   open sheet opens a new one; one fitting on no empty sheet is left
   where it is.
4. **Improvement loop** (when on): a seeded random step changes the
   order (two objects swap, or one moves to another place in the order,
   one time in three), the fill runs again and the new order is kept when
   the layout is no worse: fewer objects left out, then fewer sheets,
   then less length on the last sheet, then a lower sum of the objects'
   tops. It stops after the rounds set or when the time limit is reached
   (checked before every round and between objects; a round cut short is
   dropped). The first fill always finishes.
   The layout is the same for the same objects, settings and seed as long
   as the rounds finish within the time limit; when the time limit stops
   the loop the report says so, and the result is the best layout found
   until then (which may differ from run to run).
5. Each placed object gets one page-space transform (a move and a turn
   about the origin). One undo step: "Nest Shapes".

Work is bounded: the first fill is one pass over the objects, each
round one more pass, and the loop stops at the time limit. As a guide, a
release build nests 150 curved parts (circles, rings and triangles)
on 1220 x 2440 sheets in about 90 ms per round.

## Report

After a run the section and the status line say how many objects were
placed of how many, on how many sheets, the utilisation and the length:

- Utilisation = net area of the objects placed (outer areas minus holes,
  as Measure counts it) / material used. Material used is sheets x sheet
  width x sheet height, or roll width x length used.
- Length used = the farthest edge of an object from the start of the
  roll (or of the last sheet), plus the margin.
- Shape nesting also says how many improvement rounds ran and whether
  the time limit stopped them.

## Fill Sheet (Multiply)

Fills the sheet (or the page, with Page size on) with copies of the
selection as one block: the bounding box of the selected objects'
outlines, copied in a grid.

| Setting | Default | Range |
| --- | --- | --- |
| Gap | 2 mm | 0 and up |
| Most copies (0: all that fit) | 0 | 0 to 10000 |
| Turn copies 90 degrees where more fit | on | |

- A grid of `nx = floor((W - 2 margin + gap) / (w + gap))` by
  `ny = floor((H - 2 margin + gap) / (h + gap))` blocks, from the start
  corner, rows across first.
- With turning on, six plans are compared: all upright, all turned, and
  each of those with the strip left over beside its grid, or above it,
  filled the other way round. The plan with the most copies wins (fewer
  turned copies, then the earlier plan, on a tie).
- The selection itself moves to the first place; the other places get
  copies (each selected object copied, on its own layer). The roll
  setting is not used: Fill Sheet always fills one sheet. At most 10000
  places are filled (`MAX_COPIES`), whatever the limit. One undo step:
  "Fill Sheet". The originals and copies are selected afterwards.

## Checks

- `tracedraw_signcut::nest::tests::*`: the frames map the start corner,
  the roll runs along x, sheets without a size are refused, the random
  generator repeats for a seed.
- `tracedraw_signcut::nest::poly::tests::*`: net area with holes, the
  overlap test (crossing, touching, containment, inside a hole), rings
  with bad points dropped, the raster covers a shape and keeps its hole
  free, blocked runs jump past the blocking cell.
- `tracedraw_signcut::nest::rects::tests::*`: four squares fill a sheet,
  spacing kept, a box turns when it only fits turned, extra sheets go to
  the right, a roll uses the least length (both directions), start corner
  and margin, bad input left out, the layout repeats.
- `tracedraw_signcut::nest::shapes::tests::*`: two triangles share a
  square their boxes cannot, a small part goes into a hole only when
  allowed, spacing and margin kept on straight and curved outlines, a
  roll filled along its length, eight turns fit a bar across the
  diagonal, the same seed gives the same layout and rounds never make it
  worse, running out of time keeps the first layout, curves, open lines
  and bad input. `timing_of_a_larger_job` (ignored) times a larger job
  by hand.
- `tracedraw_signcut::nest::fill::tests::*`: the grid inside the margin,
  turned copies filling the strip left over, the limit and the corner,
  nothing fits or bad input.
- `signcut_nest::tests::*` (app): rectangle nesting moves the selection
  in one undo step with sheet frames, shape nesting with the time limit,
  a roll's frame of the length used, Fill Sheet copies in one step with a
  limit, nothing selected or nothing fitting changes nothing, the section
  draws each tool, the page is the sheet by default.
- `ui::menus::tests::every_menu_item_runs_without_panicking` clicks the
  three menu items.

## Not done

- Placing by no-fit polygons: candidates come from the cell grid and the
  slides, so very narrow gaps (under about two cells) between already
  placed objects are reached only by sliding.
- Mirrored placement, grain direction, per-object quantities and
  priorities, and nesting into the free area of a sheet already holding
  objects (the sheet is taken as empty).
- New pages for extra sheets (they go to the right of the page).
