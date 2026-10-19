# Sign & Cut: Calculations

How long a job takes to cut, what it costs and what it weighs. The
geometry is in `crates/signcut/src/calc/` (`time.rs`, `cost.rs`,
`weight.rs`, `csv.rs`); the settings and operations are in
`apps/tracedraw/src/signcut_calc.rs` and the panel section in
`ui/signcut_calc.rs`.

The calculations only read the drawing: nothing here adds, changes or
deletes an object, and nothing is an undo step.

## Opening it

- Window > Panels > Sign & Cut, section **Calculations**.
- Tools > Sign & Cut > Calculations > **Cutting Time**, **Cost** or
  **Volume and Weight**: each measures the objects, opens the panel at
  the section and sums the results up in the status line ("Cutting time
  0:01:23, cost 12.30 EUR, weight 3.570 kg").

## Which objects

Like the rest of the panel: the selection, in stacking order; with
nothing selected, every visible, unlocked object of the page. The
section says which. Outlines are taken as Measure takes them (a group by
its members, a clip frame by its frame, text by its glyphs). Each
object is one **block** for the cost (a group is one block); an object
without a single segment has no block.

**Calculate** measures the objects once: length and areas through
`tracedraw_signcut::measure` (the same numbers as Measure), each
object's bounding box, and the head's travel between outlines. Time,
cost and weight are then worked out from that measurement and the
current settings every time the section is drawn, so a new price,
profile or thickness shows at once. When the drawing, the objects or the
travel order change, the results stay and the section says to calculate
again.

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
- `ui::signcut_calc::tests::*`: the section and every list editor draw
  before and after measuring and with empty lists, the billed block on
  the drawing; row indices after a deletion.
