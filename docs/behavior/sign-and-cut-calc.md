# Sign & Cut: Calculations

How long a job takes to cut, what it costs and what it weighs. The
geometry is in `crates/signcut/src/calc/` (`time.rs`, `cost.rs`,
`weight.rs`, `csv.rs`, and `arrange.rs` for Set Size and Replace); the
settings and operations are in `apps/tracedraw/src/signcut_calc.rs` and
the windows in `ui/signcut_win_calc.rs` (see Windows below). This page
also describes the windows of Perimeter, Area, Set Size, Reference Scale
and Replace, whose operations are in `apps/tracedraw/src/signcut.rs`.

Set Size, Reference Scale and Replace change the drawing, each in one
undo step.

The calculations only read the drawing: nothing they do adds, changes
or deletes an object, and nothing is an undo step.

## Opening it

- The Sign & Cut toolbar's Measure and Calculate button, or the Sign &
  Cut panel: Perimeter, Area, Cost, Cutting Time, Volume and Weight
  (Set Size and Replace are on the Arrange button, Reference Scale on
  CAD and Machining).
- Tools > Sign & Cut > Calculations > **Cutting Time** or **Volume and
  Weight**: opens the window, measures the objects and sums the results
  up in the status line ("Cutting time 0:01:23, cost 12.30 EUR, weight
  3.570 kg"); **Cost** opens the Cost window and adds the objects as a
  row.

## Which objects

Like the rest of Sign & Cut: the selection, in stacking order; with
nothing selected, every visible, unlocked object of the page. Outlines are taken as Measure takes them (a group by
its members, a clip frame by its frame, text by its glyphs). Each
object is one **block** for the cost (a group is one block); an object
without a single segment has no block.

Opening a calculator window, or **Update**, measures the objects once:
length and areas through `tracedraw_signcut::measure` (the same numbers
as Measure), each object's bounding box, and the head's travel between
outlines with each move. Time, cost and weight are then worked out from
that measurement and the current settings every time the window is
drawn, so a new price, profile or thickness shows at once. When the
drawing, the objects or the travel order change, the results stay, grey,
and the window says to press Update.

## Cutting time

Profiles: a name and five values. Two come with TraceDraw:

| Profile | Cutting speed | Travel speed | Lift and lower | Acceleration factor | Passes |
|---|---|---|---|---|---|
| Standard | 300 mm/s | 600 mm/s | 0.2 s | 1.2 | 1 |
| Thick film, two passes | 150 mm/s | 500 mm/s | 0.3 s | 1.3 | 2 |

Ranges: speeds 0.1 to 100 000 mm/s (shown and typed in the document's
units per second), lift and lower 0 to 60 s, acceleration factor 1 to
10, passes 1 to 100. A value that is not a number takes the Standard
value. **Edit profile** edits the profile in use; **Add** copies it as
"New profile"; **Delete** removes it (the last profile stays).

Outlines are the subpaths with at least one segment, open or closed.
The head starts at the page's origin (bottom-left corner); the way back
at the end is not counted. Travel order:

- **Nearest first** (default): always the nearest outline next. A
  closed outline is entered at its nearest node and left there; an open
  one at its nearer end and left at the other end. When outlines x nodes
  exceeds 20 000 000 the order falls back to As drawn and the section
  says so.
- **As drawn**: the stacking order, each outline from its first node.

Nodes that are not finite numbers are left out; an outline without a
finite node is not counted.

Formulas (lengths in mm, speeds in mm/s, times in s):

- cutting = cut length / cutting speed x acceleration factor x passes
- travel = travel length / travel speed x acceleration factor x passes
- lifts = outlines x lift and lower time x passes
- total = cutting + travel + lifts

The table shows Cut length, Cutting, Travel (length and time), Lifts
(count x time), Passes and Total; times as h:mm:ss and in seconds.

## Cost

Settings: **Material** (one of the material list, or No material),
**Technical reserve** (5 mm, at least 0), **Minimum billed size** (width
and height, 0 mm each), **Copies** (1, 1 to 100 000), **Currency** (free
text up to 12 characters shown after amounts, empty at first) and the
**Services billed** (a check box per service).

Blocks (one per object, mm):

- billed width = max(box width + 2 x reserve, minimum width)
- billed height = max(box height + 2 x reserve, minimum height)
- billed area = sum of billed width x billed height (blocks may
  overlap; each is billed)

Material:

- **per m2**: billed area (m2) x copies x price.
- **per running metre of a roll** of a given width: the bounding box of
  all the objects, grown by the reserve on every side and at least the
  minimum size, is laid on the roll. When its width fits the roll, the
  roll used is its height; else when its height fits, it lies across
  the roll and the roll used is its width; else it is cut in strips of
  the roll's width side by side across its narrower side: strips =
  ceil(narrower side / roll width), roll used = strips x longer side.
  Cost = roll used (m) x copies x price. Copies follow one another along
  the roll. A roll width that is not a positive number fits anything.

Services (each selected one is a line):

| Billed per | Quantity |
|---|---|
| metre of cut | cut length (m, one pass) x copies |
| m2 | billed area (m2) x copies |
| piece | blocks x copies |
| hour | total cutting time (h, with the profile in use) x copies |

Each line: quantity x price; the total is the sum of the lines. Prices
and quantities that are not numbers or below zero count as zero. The
table shows Item, Quantity (in m2, m, pcs or h whatever the document's
units: prices are per metre), Unit price and Amount, then the Total
with the currency, the billed area and, for a roll, the roll used (and
"across the roll" or the number of strips).

While the section is open and the measurement is current, the billed
blocks (bounding boxes grown by the reserve) are outlined on the
drawing.

## Volume and weight

Settings: **Material** from the density list and **Thickness** (3 mm,
0 to 10 000 mm).

- volume (mm3) = net area (mm2, holes taken away, as Measure) x
  thickness; shown in cm3 and litres
- weight (g) = volume / 1000 x density (g/cm3); shown in g below
  1000 g, else in kg
- edge area (mm2) = outline length x thickness (for edge banding or
  painting the edges)

## The lists

Under **Price lists** (materials and services) and **Densities**: each
row is edited in place (name, unit, price, and the roll width of a roll
material; name and density); the cross deletes a row; **Add** adds
"New material" (per m2, price 0, roll width 1000 mm), "New service" (per
metre of cut, price 0, billed) or "New material" in the density list
(1 g/cm3). A list keeps at most 500 rows; names are one line of at most
120 characters; prices 0 to 1 000 000 000; densities 0.001 to 100 g/cm3.
Deleting the material in use bills no material.

The lists TraceDraw comes with hold example prices without a currency;
edit them to yours. The names of these rows are data and stay in
English whatever the interface language.

| Material | Unit | Price | Roll width |
|---|---|---|---|
| Self-adhesive vinyl 630 mm | roll | 6 | 630 mm |
| Self-adhesive vinyl 1260 mm | roll | 11 | 1260 mm |
| Reflective film 1220 mm | roll | 28 | 1220 mm |
| PVC foam board 3 mm | m2 | 18 | |
| Acrylic 3 mm | m2 | 38 | |
| Aluminium composite 3 mm | m2 | 30 | |

| Service | Billed per | Price | Billed at first |
|---|---|---|---|
| Cutting | metre of cut | 0.5 | yes |
| Weeding | m2 | 12 | yes |
| Application tape | m2 | 3 | no |
| Handling | piece | 0.5 | no |
| Machine time | hour | 40 | no |

Densities (g/cm3): Acrylic 1.19, PVC foam 0.55, Aluminium 2.70, Steel
7.85, MDF 0.75, Plywood 0.6, Polycarbonate 1.20.

### CSV

**Export CSV...** saves a list through the save dialog (a download in
the browser): comma-separated, decimal point, CR LF line ends, a header
row, fields with a comma, quote, semicolon, tab or line break quoted
(quotes doubled).

| List | Columns | Units |
|---|---|---|
| materials.csv | name, unit, price, roll_width_mm | `m2` or `roll` |
| services.csv | name, unit, price | `m`, `m2`, `piece` or `hour` |
| densities.csv | name, density_g_cm3 | |

**Import CSV...** replaces the list with the rows of a picked `.csv` or
`.txt` file:

- The separator is the one of tab, semicolon and comma found most often
  outside quotes on the first line with text (on a tie: tab, then
  semicolon, then comma). A byte order mark is skipped.
- Numbers may use a decimal comma and spaces or the other mark between
  thousands (`8,50`, `1 234,5`, `1.234,50`, `1,234.50`).
- Units are read without regard to case; also understood: `m²`, `sqm`,
  `area`, `m`, `lm`, `running` for materials (`m` means a roll);
  `metre`, `meter`, `lm`, `m²`, `sqm`, `pc`, `pcs`, `each`, `h`, `hr` for
  services.
- A row is left out when it has no name, an unknown unit, a price that
  is not a number or below zero, a roll without a width above zero, or a
  density not above zero. A first row that does not read is the header
  and is not counted; rows starting with `#` are comments; empty lines
  are skipped. Rows past 500 are left out.
- The status line gives the rows read and left out. A file without a
  usable row leaves the list as it was. Imported services are not
  billed until checked; the first imported material and density are
  chosen.

### Kept across restarts

The profiles, the lists, the choices and the cost and weight settings
are kept in `signcut_calc.json` in the preferences folder (the
browser's local storage in the web build), written after a change once
no field is being typed in, through a temporary file. A file that is
missing, unreadable, not such a file or larger than 4 MB gives the
defaults; a value that is missing or out of range keeps its default or
is brought into range; rows missing a name or with an unknown unit are
dropped. The measurement is not kept.

## Windows

Each tool of the Measure and Calculate button of the Sign & Cut toolbar,
and Set Size, Reference Scale and Replace, has a window of its own
(`ui/signcut_win_calc.rs` and `ui/signcut_win_calc/`). The calculators
(Perimeter, Area, Cost, Cutting Time, Volume and Weight) change nothing
in the drawing: Update at the bottom left, Close at the right. They
measure the objects (the selection, else the page) when the window
opens and with Update; when the drawing or the objects change after
that, the results turn grey and the window says "Changed since: press
Update." The calculators take sizes as drawn on the page (millimetres
of the page, whatever the drawing scale).

Their Units lists are their own (mm, cm, m, in, ft; areas and volumes
in the square and cube): Perimeter and Area start in m, Volume and
Weight in mm. The units, the prices of the windows and the price per
hour are kept with the lists in `signcut_calc.json`.

### Perimeter (310 x 250)

One group Perimeter: **Perimeter** (bold, the outline length in the
unit), a line with a **More** link at its right that opens the Area
window, **Price per <unit>** (1.00 at first, 0 to 1 000 000 000),
**Cost** (perimeter x price, with the currency of the cost lists) and
**Units**.

### Area (280 x 470)

A white list of five rows, each with a picture of what it counts (a
plate with a round hole and a small part beside it, filled where the
row counts), the name and the value in bold: **Object area** (outer
areas less holes), **Outer area**, **Inner area (holes)**, **Bounding
box area** (with the box's width x height under it) and **Perimeter**.
Under it **Objects** (how many) and **Units**.

### Cost (880 x 330, resizable)

A table with a row per **Add**: Add takes the objects (the selection,
else the page) as one row; objects without an outline give no row
("Nothing to bill"). Columns: **No.** with a swatch of the first plain
fill colour among the row's objects (a click selects the row's objects
that still exist; the number is grey when the drawing changed since the
row was measured), **Name** (free text), **Material** (a list: No
material or a material of the price list; a new row takes the material
chosen last), **Material qty** (m2 or running m), **Material cost**,
**Services** (the services billed), **Services cost**, **Total** and a
cross that deletes the row. The last row, **Total**, sums the costs, and
the quantities when every row has the same unit. Each row is worked out
as in Cost above with its own objects and material, the services
billed, the job settings and the cutting time of its objects with the
profile in use; a new price shows at once.

Bottom: **Update** measures every row again from its objects (objects
that are gone leave the row, a row with none left goes), **Add**,
**Price Lists** (shows or hides the lists under the table) and
**Close**. The lists have three tabs: **Materials** and **Services**
(edited in place as in The lists below; a service's check box says it
is billed) and **Job**: Technical reserve, Minimum billed width and
height, Copies and Currency. While the window is open the billed blocks
of the rows measured on the current drawing are outlined on the
drawing.

### Cutting Time (400 x 320, resizable)

Tabs:

- **Time**: the profile list with a button that opens Parameters;
  **Estimated time** (bold, "0 h 1 min 23 s"); **Cutting share** (cutting
  time / total time, %); **Price per hour** (100.00 at first) with the
  price per m2 of net area beside it; **Total** (total time in hours x
  price per hour, with the currency).
- **Parameters**: the profile's Name, Cutting speed, Travel speed, Lift
  and lower, Acceleration factor and Passes, the **Travel order**, and
  Add and Delete (as in Cutting time above).
- **Preview**: the outlines measured, the moves with the tool up in red
  and the start point (the page's origin) as a green cross; under it
  Cut length, Cutting, Travel (length and time) and Lifts (count and
  time).

A thin green bar above the buttons shows the cutting share.

### Volume and Weight (310 x 560)

- **Volume**: Volume (bold, net area x thickness), Area (net), Edge area
  (outline length x thickness), **Material thickness** (3 mm at first,
  in the window's unit) and **Units**.
- **Mass**: **Material** (the density list), **Density (kg/m3)** (the
  material's density, editable: it changes the list), **Units** (g, kg,
  t, lb), **Mass** (bold) and **Material Density List**, which shows or
  hides the density list (edited as in The lists below).
- **Price**: **Cost** (1.0000 at first), **Cost per** (the mass unit, m2
  of net area or m3 of volume) and **Price** = cost x the quantity.

### Set Size (300 x 660)

A command window: Close only. Group Set Size: **Change Width Only**,
**Change Height Only** and **Change Size**, each with a picture, apply
at once to the selected, unlocked objects (one undo step "Set Size";
the status line says how many changed, or "Nothing to change.").
**Reference position**: the point of each object's box that stays put
(Center at first, nine choices). Group Size from, one of:

- **Same as the largest**: the largest width and height among the
  selected objects (each taken on its own).
- **Same as the smallest**: the smallest width and height above zero.
- **Size**: **Width** and **Height** (100 mm each at first).
- **Change by**: the object's own size plus **Width change** and
  **Height change** (0 at first; a change that leaves nothing keeps
  that side).
- **Scale**: the object's own size x **Scale (%)** (100 at first).

**Keep proportions** (on at first): one factor for both sides; with
both sides given the object fits inside them; with one side only (the
Width Only or Height Only button) the other side follows. Off: Width
Only changes only the width. **Change** limits which objects change:
Every object, Only larger objects (shrink only), Only smaller objects
(grow only). **Rotate** with **Angle (°)** (counter-clockwise, -360 to
360) also turns each object about the same point of its new box. Then
**Units** (the document's units).

### Reference Scale (320 x 400)

Group Reference: **Measure** (Width, Height or Length; changing it
measures the reference again and takes that value as the real size),
**Use Selected Object as Reference** (one object selected), **Reference
measures** (its value now), **Real size** and **Scale factor** (real
size / measured value, and in %). Group Scale: **Objects** (Every object
on the page, or The selection and the reference) and **Fixed point**
(bottom left at first). Then **Units**. While the window is open the
reference's bounding box is outlined on the drawing.

Apply scales (one undo step "Reference Scale") and stays open; OK
scales and closes; Cancel closes. Apply and OK are greyed while there
is no reference or the factor is 1.

### Replace (400 x 290)

Tabs **One master** and **Several masters**. One master: **Set Master**
(one object selected; the button reads "! Set Master" while no master
is set) and **Replace with Master** (enabled with a master and another
object selected). Several masters: **Set Masters** takes every selected
object as a master, in selection order; **Replace with Masters** puts
in each place the master that **Option** picks: **In turn** (the first
object to replace gets the first master, the next the next, and round
again) or **Closest proportions** (the master whose width to height is
closest to the object's; a flat object or flat masters fall back to In
turn). Both tabs: **Preserve angle** (No/Yes: the copy takes each
object's rotation, as Take each object's rotation), **Size** (Keep the
master's size, Fit inside each object, Stretch to each object) and Keep
the replaced objects. One undo step ("Replace"); the copies are
selected. A grey line says how many masters are set; their boxes are
outlined on the drawing. Bottom: **Select Master** (selects the masters)
and Close.

### Left out

- Area: a raster-sampled area (the bitmap area of the reference
  layout): it is the same area as Object area within the raster's
  accuracy and adds nothing for cut work.
- Cost: a per-row second size column for the services; services are
  billed in different units (m, m2, pieces, hours), so the table shows
  their names and cost.
- Volume and Weight: a units list per result (the volume and area share
  the window's unit).
- Replace: a repeat button; Replace with Master is pressed again
  instead.
- Set Size: the reference's two "Specify size" entries are Size and
  Change by here.

## Limits

- The roll use takes the objects as they lie: no nesting, copies one
  after another along the roll.
- The travel is an estimate of the head's moves (nearest first or as
  drawn), not the order a cut file will use, and the way back is not
  counted. Acceleration is one factor, not a model of the machine.

## Checks

- `tracedraw_signcut::calc::tests::*`: a block per object and the
  measured length, an empty job, non-finite outlines.
- `tracedraw_signcut::calc::time::tests::*`: the default profile,
  cutting plus travel plus lifts times passes, values brought into
  range, nearest-first order, entry at the nearest node or end,
  subpaths as outlines, nothing to cut, non-finite nodes, h:mm:ss.
- `tracedraw_signcut::calc::cost::tests::*`: reserve and minimum size,
  material by area, a roll by the length used, turned and in strips,
  services times copies, bad numbers, material and service CSV round
  trips, semicolon CSV with decimal commas and bad rows.
- `tracedraw_signcut::calc::weight::tests::*`: the default densities,
  1 m2 of 3 mm acrylic weighs 3570 g, bad numbers, density CSV.
- `tracedraw_signcut::calc::csv::tests::*`: separators, quoted fields,
  numbers with decimal commas and thousands, written text reads back,
  header rows, an unclosed quote.
- `signcut_calc::tests::*` (app): cutting time of the selection with a
  profile and the out-of-date note, nothing changes in the drawing (no
  revision, no undo step), cost of material and services and a new
  price at once, weight of acrylic, CSV import and export, settings
  through JSON and bad files.
- `tracedraw_signcut::calc::time::tests::the_moves_add_up_to_the_travel`.
- `tracedraw_signcut::calc::arrange::tests::*`: as large as the largest,
  as small as the smallest with the proportions kept, width only with
  and without the lock, change by and scale, rotate about the fixed
  point, bad numbers and flat objects; masters picked in turn or by
  proportions.
- `signcut_calc::tests::calc_cost_rows_follow_the_lists_and_the_drawing`
  (rows by area and by roll, a deleted material, Update drops deleted
  objects, nothing to bill), `calc_time_and_weight_prices`,
  `calc_units_convert`.
- `signcut::tests::the_measurement_is_outdated_by_a_change_or_another_selection`,
  `resize_modes_and_sides_in_one_step` (one undo step, nothing to do is
  no step), `replace_with_several_masters_picks_by_proportions`.
- `ui::signcut_win_calc::tests::every_calc_window_draws_and_changes_nothing`,
  `a_window_counts_as_opened_after_a_frame_without_it`,
  `numbers_drop_trailing_zeros`.
- `ui::signcut_win_calc::measure::tests::perimeter_and_area_show_the_selection_in_the_chosen_unit`;
  `cost::tests::the_cost_window_draws_its_table_and_every_list` (billed
  blocks on the drawing, no change), `row_indices_follow_a_deleted_row`;
  `time::tests::the_time_window_draws_every_tab`,
  `times_read_as_hours_minutes_seconds`;
  `weight::tests::the_weight_window_draws_with_and_without_densities`;
  `arrange::tests::the_arrange_windows_draw_in_every_mode_and_tab`,
  `reference_scale_apply_is_one_step`.
- Not looked at on screen yet: the windows are checked by drawing them
  headless only.
