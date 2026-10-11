# Curve tools

Freehand, 2-point line, Bezier, Pen, B-spline, Polyline, 3-point curve,
Shape recognition and Sketch. All produce open (or closed, when the end
meets the start) `Path` objects with the toolbox defaults.

## Freehand (F5)

- The pointer is sampled every 0.2 mm of movement; on release the points
  are simplified with a tolerance of `0.2 + s / 100 x 2.2` screen pixels
  (`s` the Freehand smoothing, 0 to 100, 50 by default: 1.3 pixels; at
  least 0.02 mm), so a zoomed-in drawing keeps more detail, and fitted
  with smooth cubic segments (`geometry::smooth_path`). The smoothing is
  on the property bar and in Options > Tools > Curve tools.
- A click, then another click: a straight line between them, previewed
  to the pointer after the first click; Ctrl keeps it to steps of the
  constrain angle (15 degrees by default); Esc drops the first click; a
  drag in between draws freehand instead.

## Bezier, Pen and B-spline

- Click places a cusp node; click-drag places a smooth node and pulls
  its handle out symmetrically (the handle follows the pointer while the
  button is down).
- Double-click or Enter finishes; Esc discards the curve being drawn;
  clicking the first node (within the join distance, 5 screen pixels by
  default, once the curve has three nodes) closes the curve.
- Snapping applies to every placed node.
- Ctrl: the next node goes in steps of the constrain angle from the last
  one (keeping how far the pointer reaches along that direction), and a
  dragged handle in such steps around its node; the preview follows.

## Polyline and 2-point line

- Polyline: clicks add straight segments (no smooth handles);
  double-click or Enter finishes. 2-point line: the second click
  finishes the segment at once. Ctrl keeps each segment to steps of the
  constrain angle.

## Continuing the selected curve

With one open curve selected (lying on a layer), a line drawn with
Freehand (dragged, or click and click), Bezier, Pen, B-spline, Polyline
or 2-point line that starts or ends within the join distance (Options >
Tools > Curve tools, 5 screen pixels by default) of one of the curve's
end nodes becomes part of that curve (code: `curve_join.rs`):

- it runs on from the end node itself (a line ending there is turned
  round first; a line from the curve's start turns the curve round);
- when its other end lies on the curve's other end, the curve closes;
- when its other end lies on an end of another of the curve's
  subpaths, the two subpaths become one;
- the curve keeps its look and stays selected; the change is one undo
  step (Set kind) and the curve's own transform is kept.

Otherwise the line is a curve of its own. Closed curves, several
selected objects and objects inside groups are never continued.

The pointer shows a node with a line going on from it over such an end
node, and a ring over the first node of a curve being drawn when a
click would close it.

## Property bar

The selection's position, size, scale, angle and mirror buttons; with a
selection also its outline width, line style and start and end
arrowheads, and Close Curve (enabled for an open curve); for Freehand
the smoothing (0 to 100); with nothing selected a reminder of how to
finish a curve.

## 3-point curve

- Drag the chord, then click the point the curve passes through; the
  curve is the quadratic through the three points (the click at t = 0.5),
  written as a cubic.

## Shape recognition (Shift+S)

- The stroke is simplified at 1 mm. It is "closed" when its ends are
  closer than a quarter of its longest side. Recognised results:
  - open stroke with at most 3 simplified points: a straight line;
  - closed with 3 or 5 points: a triangle; 4 (or 5) points: a rectangle,
    in both cases the stroke's bounding box;
  - closed with more points whose area is within 20 % of the bounding
    ellipse's: an ellipse;
  - anything else: a smoothed curve (closed when the stroke was).

## Sketch (S)

- The stroke is simplified at 0.8 mm and smoothed. When an open curve is
  selected and the new stroke starts within 5 mm of its end, the stroke
  is appended to it; otherwise a new curve is created and selected, so
  consecutive strokes chain.

## Checks

- `curve_join::tests` (from either end, closing, two subpaths made one,
  reversing curves, in the curve's own space with one undo step)
- `ui::cursors::tests::tools_get_their_cursors` (join and close signs)

- Given a freehand drag of 100 jittered points along a line, then the
  result has at most a handful of nodes and spans the same bounds.
- Given a shape-recognition stroke tracing a square, then the result is a
  `Rect`; tracing a circle gives an `Ellipse`; a short open stroke gives
  a two-node line.
