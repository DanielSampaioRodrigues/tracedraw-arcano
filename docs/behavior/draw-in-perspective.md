# Draw in Perspective

View > Draw in Perspective shows a perspective grid over the page. While
it is on, every object a drawing tool makes (rectangles, ellipses,
polygons, curves, brush strokes, text frames...) is laid onto the active
plane of the grid. Objects made with the Pick tool (pasted, duplicated,
moved) are left as they are.

## The bar

At the top left of the drawing window:

- Finish: leaves the mode (the grid goes; the objects keep their
  perspective).
- 1-Point, 2-Point, 3-Point: the kind of grid (a new grid of that kind,
  sized to the page).
- The planes: Ground, Left, Right (and Top with 1-Point). The active
  plane's grid lines are strong, the others faint.
- Grid: show the grid lines.
- Depth (5 to 98 %): how far towards the vanishing points the grid
  reaches.

## Defaults

- Horizon at 55 % of the page height.
- 1-Point: the vanishing point in the middle of the horizon; the near
  face from 15 % of the width, 70 % of the width wide, 8 % up from the
  bottom and 75 % of the height tall; depth 70 %.
- 2-Point: vanishing points at -35 % and 135 % of the width on the
  horizon; the near corner in the middle, 8 % up; depth 45 %.
- 3-Point: as 2-Point, with the vertical vanishing point at 220 % of the
  height above the corner; the walls' vertical edges lean to it.

## Handles

Drag on the page: the vanishing points (circles; with 2- and 3-Point the
two side points share the horizon), the near bottom corner and the near
top corner (squares; with 1-Point the top corner also sets the width).

## How objects are laid down

Each plane is the projective image of a unit square (`homography`, the
same as the Perspective effect's). The drawn object's box is taken onto
the plane: each side of the box becomes the mean of its two corners'
plane coordinates, giving a rectangle on the plane, whose corners on the
page are the object's Perspective effect (Effects > Add Perspective),
so the result is exact and the Shape tool edits its four corners. A box
drawn past a plane's vanishing line is pulled towards the grid until it
fits, and one that lies wholly beyond it stays flat. Drawing the object
and laying it down are one undo step.
