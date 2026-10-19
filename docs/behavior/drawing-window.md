# Drawing window

## Desktop and page

- The desktop (the area around the page) is white by default; the
  colour is a setting (`desktop_rgb`, Options > Customization).
- The page is white with a 1 px border `#AAAAAA` and a flat shadow
  `#BFBFBF`: the bordered page rectangle moved 6 px right and 4 px down,
  drawn behind the page, so a 6 px band shows on the right and a 4 px
  band at the bottom. The page rectangle is snapped to whole pixels.
  View > Page > Page Border hides the border and the shadow.
- Double-clicking the page border (within 3 px) or its shadow with the
  Pick tool, where no object is hit, opens the page size options
  (Layout > Document Options at Page Size).

## Scrollbars

- The vertical scrollbar runs down the right edge between the ruler and
  the bottom row; the horizontal one fills the right part of the bottom
  row. Both are 17 px: arrow buttons at the ends, a light track and a flat
  grey thumb that darkens when hovered or dragged.
- They cover the page plus one page size on every side, grown to include
  whatever is visible. Dragging the thumb pans; a click on the track pages
  by 90 % of the window towards the click; an arrow moves 40 px, then
  repeats every 50 ms after 0.4 s while held.

## Bottom row

From the left, after the ruler's width:

1. The document navigator: insert page before, first page, previous
   page, "n of m", next page, last page, insert page after. The insert
   buttons add a page of the current size next to the current page and
   make it current.
2. The page tabs: one slanted tab per page, the current one white; a click
   goes to the page, a double click renames it. A right click goes to the
   page and opens its menu: Insert Page Before, Insert Page After,
   Duplicate Page, Rename Page and Delete Page (greyed out when the
   drawing has one page), each acting on that page as the Layout menu's
   commands do, as one undo step.
3. A dotted splitter: dragging it shares the row between the page tabs and
   the horizontal scrollbar (45 % for the tabs by default).
4. The horizontal scrollbar.

The Navigator button (a magnifier over a cross) sits in the corner under
the vertical scrollbar; see `zoom-and-pan.md`.

## Windows

With drawings open, Window > Cascade, Tile Horizontally and Tile
Vertically put every drawing in a window of its own inside the drawing
area (`ui/windows.rs`); Combine Windows and Dock Window bring the tabs
back. The tabs stay above the area and still choose the active drawing.

- Tile Horizontally: the windows one above the other, each as wide as
  the area and an equal share of its height. Tile Vertically: side by
  side, each as tall as the area. Cascade: windows of 72 % of the area
  (at least 320 x 240 px, at most the area), each 28 px down and to the
  right of the one before, starting again at the top left when the next
  one would leave the area; the area behind them is `#BEBEBE`.
- A window has a 24 px title bar: the drawing's name (an asterisk while
  it has unsaved changes; bold on `#CEE3FF` for the active drawing, on
  `#E6E6E6` for the others), then a button that brings the tabs back with
  this drawing active (as a double click on the bar does) and one that
  closes the drawing (asking to save first when it has changes). The
  active window has a 1 px accent frame, the others a grey one.
- The active window is drawn on top with its rulers, scrollbars, page
  tabs and tools. The others show their page, grids, objects and
  guidelines as their own view left them (each drawing keeps its view
  and its rendered page), without the editing going on in the active
  window; a click on one makes its drawing the active one.
- A new arrangement, another number of windows, or the tabs again fit
  every page into its window. Otherwise a drawing stays where it was in
  its window when the window moves or the canvas begins elsewhere (the
  rulers shown or hidden, a bar docked on another side): its view moves
  with the canvas.

## New Window

Window > New Window (greyed with only the Welcome Screen) opens another
window of the active drawing: a tab right after the drawing's last
window, made active, showing the page, view and selection the active
window shows.

- All windows of a drawing show one document with one undo history: an
  edit, undo or redo in one is what every other shows. Each window keeps
  its own page, view (zoom and scroll) and selection; objects removed in
  another window drop out of a window's selection when it becomes active
  again.
- While a drawing has more than one window, its tabs, window title bars,
  Window menu entries and the main title read "name:1", "name:2" ... in
  the order the windows were opened (the asterisk for unsaved changes
  follows the number). Closing a window numbers the rest again from 1; a
  drawing left with one window shows its plain name.
- Save and Save As from any window save the drawing; every window then
  shows it saved, under the new name.
- Closing a window while the drawing has another open closes only that
  window, without asking. Closing the last window closes the drawing as
  before, asking to save when it has changes. Close All and quitting ask
  once per changed drawing; auto-backup writes one file per drawing.
- Tiled and cascaded windows (above) draw each window of a drawing with
  its own view.

## Checks

- `ui::windows::tests::windows_tile_and_cascade_inside_the_area`
- `ui::windows::tests::tiled_windows_draw_every_drawing_and_switch`
- `ui::windows::tests::the_view_stays_put_in_a_window_that_moves`
- `documents::tests::a_new_window_shares_the_drawing_and_its_undo_history`
- `documents::tests::each_window_keeps_its_own_view_page_and_selection`
- `documents::tests::window_titles_number_the_windows_of_a_drawing`
- `documents::tests::closing_one_window_keeps_the_drawing_in_the_other`
- `documents::tests::closing_a_parked_drawing_window_and_close_all`
- `documents::tests::saving_from_either_window_saves_the_one_drawing`
- `documents::tests::tiled_windows_draw_both_windows_of_a_drawing`
- `ui::menus::tests::window_new_window_opens_a_second_window`
- `ui::window_bars::tests::insert_page_before_and_after_the_current_one`
- `ui::window_bars::tests::the_page_border_and_shadow_are_hit_but_not_the_page_inside`
- `ui::window_bars::tests::a_right_click_on_a_page_tab_takes_the_page_and_opens_its_menu`
- `ui::window_bars::tests::a_page_tabs_menu_acts_on_its_page`
- `ui::window_bars::tests::the_last_page_is_not_deleted_from_its_tab`
