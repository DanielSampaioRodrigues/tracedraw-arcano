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

A freehand stroke becomes the shape it looks like, as drawn: tilted
shapes stay tilted (`recognize.rs`). The stroke is closed when its ends
are within a fifth of its size (the diagonal of its box).

- Open: a straight line from end to end when no point is farther from
  that line than a share of its length; else a smoothed curve.
- Closed, the corners found by simplifying the stroke (from its point
  farthest from the middle) by a share of its size:
  - four corners all near right angles: a rectangle, its sides the means
    of opposite sides, turned as the first side runs (within 45 degrees;
    nearly level ones come out level);
  - otherwise, when the stroke's area is within a share of the ellipse
    of the same spread (its second moments, so any tilt): an ellipse,
    level when nearly level, never tilted when nearly round;
  - three corners: a triangle, and four other corners: a four-sided
    shape, through the corners as drawn;
  - anything else: a smoothed closed curve.
- The new object takes the default fill and outline (open curves no
  fill), is selected; one undo step.

Property bar: Shape recognition and Smoothing, each None, Lowest, Low,
Medium (first), High, Highest; kept in the preferences
(`tools.recognize`).

| Level | Corners (share of size) | Straight line | Right angles | Ellipse area | Smoothing (mm) |
|---|---|---|---|---|---|
| None | no shapes | no shapes | | | 0.05 |
| Lowest | 0.03 | 0.025 | 8 degrees | 5 % | 0.15 |
| Low | 0.045 | 0.04 | 12 degrees | 8 % | 0.3 |
| Medium | 0.06 | 0.06 | 16 degrees | 12 % | 0.6 |
| High | 0.08 | 0.08 | 20 degrees | 16 % | 1.0 |
| Highest | 0.11 | 0.11 | 25 degrees | 20 % | 1.6 |

## Sketch (S)

- The stroke is simplified by the bar's smoothing level (as Shape
  recognition's) and smoothed. With Join on, when an open curve is
  selected and the new stroke starts within the join distance (5 mm) of
  its end, the stroke is appended to it; otherwise a new curve is created
  and selected, so consecutive strokes chain.
- Property bar: Join to the selected curve, the distance, Smoothing; kept
  in the preferences (`tools.sketch`).

## Checks

- `curve_join::tests` (from either end, closing, two subpaths made one,
  reversing curves, in the curve's own space with one undo step)
- `ui::cursors::tests::tools_get_their_cursors` (join and close signs)

- Given a freehand drag of 100 jittered points along a line, then the
  result has at most a handful of nodes and spans the same bounds.
- `recognize::tests::strokes_become_the_shapes_they_look_like` (level and
  tilted rectangles, a circle and a tilted ellipse, a triangle, a line,
  recognition off)
- `ui::drawing_bars::tests::both_bars_draw`
