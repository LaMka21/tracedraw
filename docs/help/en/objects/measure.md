# Measuring outlines and areas

The [[docker.tracecut_measure]] panel shows how long the outlines of objects
are and how much area they cover: what a cutting plotter or laser follows and
what a sign uses up in material. It is part of traceCUT, TraceDraw's tools for
cutting and sign making. Open it from [[menu.window > menu.window.dockers]].

## What is measured {#what}

- With objects selected, the panel measures them.
- With nothing selected, it measures every visible, unlocked object on the
  page.

The values follow every change: pick other objects, move a node or resize
something, and the panel shows the new values at once.

## The values {#values}

- [[tracecut.perimeter]]: the length of every outline, curves included. A
  letter O counts its outer and its inner ring.
- [[tracecut.area]]: the material the objects cover, holes taken away.
- [[tracecut.outer_area]]: the area inside the outer outlines, holes not
  taken away.
- [[tracecut.hole_area]]: the area of the holes.
- [[tracecut.bbox]] and [[tracecut.bbox_area]]: the width and height of the
  box round all measured objects, and its area.
- [[tracecut.objects]], [[tracecut.paths]] and [[tracecut.nodes]]: how many
  objects, closed and open outlines and nodes were measured.

Lengths are in the drawing's unit and areas in its square, with the number
of decimals set in the options. Objects lying on each other are measured
each on its own, and open lines have a length but no area.
