# Sign & Cut: toolbar, panel and tool windows

How the Sign & Cut tools are reached and how their windows behave. The
tools themselves are described in `sign-and-cut.md` (Measure, Clean Up,
Size) and `sign-and-cut-cut.md`, `-nest.md`, `-calc.md`, `-sign.md`,
`-cnc.md`, `-fx.md`.

## Toolbar

Window > Toolbars > Sign & Cut (on at first) shows a bar under the
other toolbars while a drawing is open. It has nine buttons, each with a
picture and a small arrow; a click opens a menu of its tools:

| Button | Tools |
|---|---|
| Nesting | Shape Nesting, Rectangle Nesting, Fill Sheet |
| Cutting | Plotter Output, Weeding Lines, Separate Curves, Find Crossings, Find Duplicates, Remove Duplicate Edges, Print and Cut Marks, Crop Marks |
| Measure and Calculate | Perimeter, Area, Cost, Cutting Time, Volume and Weight |
| Sign Making | LED Modules, Grommets, Neon Tubes, Variable Data, Letter Returns, Acrylic Mould, G-code |
| Effects and Patterns | Effects, Hatching Lines, Grid |
| Construct | Sign Box, Smart Divide, Carrier, Centre Line, Join Curves, Reduce Nodes, Fillet Corners |
| CAD and Machining | DXF Export, Bridges, Reference Scale, Hot Wire Path, Cut Order, Finger-Joint Box, Finger Joints, Door Panels |
| Arrange | Align and Distribute, Set Size, Replace |
| Sign & Cut Panel | shows or hides the panel (no menu) |

Separate Curves runs Break Apart; Grommets opens Border and Grommet;
Variable Data opens Print Merge; Join Curves, Fillet Corners and Align
and Distribute open their panels; DXF Export opens Export. Every other
tool opens its window.

## Panel

Window > Panels > Sign & Cut lists every tool under its group's name
with the group's picture; a click does what the toolbar menu does. The
tools whose window is open show selected. The Tools > Sign & Cut menu
opens the same windows.

## Tool windows

- Several windows can be open at once. They do not block the drawing:
  the selection can change while a window is open, and the window works
  on the selection it sees when a button is pressed. Choosing an open
  tool again brings its window to the front.
- The window has the dialog look: a white title bar with the close
  button, a light grey body. It can be moved by its body and, where it
  has a preview, resized. What a window marks on the drawing (a route,
  a sheet, crossings) is drawn on the canvas while the window is open.
- Settings are in titled group boxes with the label at the left and the
  field at the right. Length fields show the document's units; the
  Units row at the bottom of the settings changes the document's units.
- **Presets**: windows with many settings have a preset row at the top
  of the settings: a list of saved presets and a gear button that shows
  a name field with Save (stores the settings under the name, replacing
  one of the same name), Delete and Defaults (back to the built-in
  values). Choosing a preset loads it. Presets are kept per tool in
  `signcut_presets.json` in the settings folder (in memory only in the
  browser).
- Buttons at the bottom right: Apply, OK and Cancel for tools that
  change the drawing (Apply works the result out and shows it in the
  window's preview where the window has one, or applies it where it
  has none; OK applies and closes; Cancel closes), Update and Close for
  calculators, Close for windows that only run commands.

## Nesting windows

Shape Nesting, Rectangle Nesting and Fill Sheet share one window layout
(about 1000 x 700, resizable):

- At the left a white preview: the sheets (the bottom edge in red) and
  the objects. Before Apply, or with Show Original pressed, the objects
  where they are now and the first sheet; after Apply, the layout. Under
  the drawing in large type the size of the placed objects with the
  usage, and "Usage n %. Sheets n". Under the preview a yellow strip:
  "Sheet count: n  Shapes nested: n of n", or how many objects are
  selected before Apply.
- At the right: the preset row; tabs Main and Layout.
  - Main: Sheet size (sheet width and height, or the roll width; Use
    active page size, on at first, with the swap button greyed while on;
    Margin). Shape and Rectangle Nesting: Nesting options (Minimum
    distance; Shape Nesting adds Allow inside, Search for best result
    No/Yes with Search time and Search rounds, Rotation, Accuracy;
    Rectangle Nesting has Turn by 90 degrees; both have Origin point).
    Fill Sheet: Copies (Gap, Turn copies where more fit, At most).
  - Layout: Material (Sheets or Roll, Roll direction, Add sheet frames);
    Shape Nesting adds Search (Random seed). Fill Sheet always fills one
    sheet.
  - Units.
- A thin progress bar shows full when the preview holds a layout of the
  drawing as it is.
- Apply works the layout out (Shape Nesting within the search time) and
  shows it; nothing in the drawing changes. Changing a setting or the
  drawing makes the preview out of date: it shows the objects again.
  OK applies the previewed layout when it is still current, or works one
  out and applies it, in one undo step ("Nest Shapes", "Nest Rectangles",
  "Fill Sheet"), and closes. Cancel closes.

## Reduce Nodes

A small window: Tolerance (0.1 mm at first) and Apply, OK, Cancel.
Removes the nodes of the selected curves that the outline does not need
within the tolerance (rectangles, ellipses and other shapes are left
alone) in one undo step ("Reduce Nodes").

## The other windows

Every tool has a window of its own; each area's page describes its
windows in its "Windows" section: `sign-and-cut-cut.md` (Plotter Output,
Weeding Lines, Bridges, Cut Order, Print and Cut Marks, Crop Marks, Find
Duplicates, Remove Duplicate Edges, Find Crossings), `sign-and-cut-calc.md`
(Perimeter, Area, Cost, Cutting Time, Volume and Weight, Set Size,
Reference Scale, Replace), `sign-and-cut-sign.md` (LED Modules, Neon Tubes,
Letter Returns, Acrylic Mould, Sign Box, Smart Divide, Carrier),
`sign-and-cut-cnc.md` (G-code, Hot Wire Path, Finger Joints, Centre Line,
Finger-Joint Box, Door Panels) and `sign-and-cut-fx.md` (Effects, Hatching
Lines, Grid). The items of Tools > Sign & Cut open the same windows.

## Checks

- `signcut_tools::tests::every_tool_is_on_exactly_one_button`,
  `every_tool_and_group_has_a_name`, `choosing_a_tool_opens_its_window_once`
- `ui::signcut_windows::tests::every_tool_window_draws`,
  `reduce_nodes_removes_nodes_on_a_straight_line`
- `ui::signcut_win_nest::tests::apply_previews_and_changes_nothing`,
  `ok_applies_the_previewed_layout_in_one_step`,
  `fill_sheet_previews_its_copies`, `the_window_draws_every_tool_and_tab`
- Looked at: the toolbar, its menus and the Shape Nesting window on
  Windows.
