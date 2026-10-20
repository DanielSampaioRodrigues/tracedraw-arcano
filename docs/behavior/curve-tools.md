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

## Bezier and Pen

- Click-drag places a smooth node and pulls its handle out
  symmetrically (the handle follows the pointer while the button is
  down). C while pulling makes it a cusp: the incoming handle stays where
  it is and only the outgoing one follows. C acts while it is held (also
  when it went down before the press) and the node stays a cusp after C
  is let go; pressing it again in the same pull changes nothing more.
  While a curve is being drawn C never aligns the selection. Alt while
  pulling moves the node itself, its handles with it.
- A plain click places a node with a straight segment to it: a node
  pulled before becomes a cusp, the segment leaving it straight (its
  incoming handle stays). Clicking the last node again (within the join
  distance) makes it a cusp without adding a node.
- Double-click or Enter finishes; with the Bezier tool Space finishes
  too and then, being the Pick tool's key, switches to Pick (checked by
  `pick_drag_tests::space_finishes_a_bezier_curve_then_picks`); Esc
  discards the curve being drawn.
  A press on the first node (within the join distance, 5 screen pixels
  by default, once the curve has three nodes; with the Pen and Auto
  add/delete on, within the larger of the join and pick distances, so
  the first node is never deleted instead) closes the curve: a click
  closes it with a straight segment from the last node; a drag from the
  first node pulls the closing segment's handle at the first node (the
  mirror of the pointer, as a pulled handle), and the first node's own
  outgoing handle stays as it was, with or without C held.
- With no curve being drawn and one curve selected, the Pen and Bezier
  tools edit it: a click on a segment (within the pick distance) adds a
  node there (the Pen only with Auto add/delete on), a press on a node
  that is not an end node drags it as the Shape tool does (end nodes
  continue the curve, see below); with the Pen and Auto add/delete on a
  click on such a node deletes it (one undo step, Delete Node; a curve
  keeps at least two nodes).
- Auto add/delete (Pen, on by default): also on the curve being drawn, a
  click on a segment (within the pick distance) adds a node there,
  splitting the segment so the curve keeps its shape, and a click on a
  node deletes it, its neighbours joined with their handles. The last
  node (a click makes it a cusp) and the first once the curve has three
  nodes (a click closes it) keep their meaning. A drag from a node or a
  segment places a node as usual. The Bezier tool has no Auto
  add/delete.
- The pointer: the cross hair with the tool's sign to start a new
  curve; with a node and a line going on from it over an end node of the
  selected curve (continue it); a ring over the first node (close); a
  plus over a segment where a click adds a node; a minus over a node a
  click deletes (Pen, Auto add/delete); a hollow node over a node of the
  selected curve otherwise, or over the last node of the curve being
  drawn (edit the node, or make it a cusp).
- While drawing (code: `pen_look.rs`), the segments placed so far are a
  hairline in the colour of the outline new objects get (black without
  one), as the curve will look; nodes placed before are small hollow
  squares and the last node a filled blue one; the last node's handles
  are blue dashed lines ending in arrow tips (a cusp shows its incoming
  handle as it was frozen). While the first node is dragged to close,
  the closing segment shows dashed in blue with its handle.
- Preview mode (Pen, on by default): while the button is up a dashed blue
  segment runs from the last node to the pointer (as the next node would
  take it, snapped and with Ctrl's steps), curved by the last node's
  outgoing handle, so the next segment shows before the click. The
  Bezier tool and the Pen with Preview mode off show nothing past the
  last node.
- Both options are on the Pen's property bar and in Options > Tools >
  Curve tools (kept in the preferences, `curves.pen_preview` and
  `curves.pen_auto_add_delete`).
- Snapping applies to every placed node and to a handle being pulled;
  the snap mark and its screen tip (Options > Snapping) show beside the
  pointer while nodes are placed and handles pulled, including "grid"
  and "guideline" when the pointer lands on a grid line or a guideline.
- Ctrl: the next node goes in steps of the constrain angle from the last
  one (keeping how far the pointer reaches along that direction), and a
  dragged handle in such steps around its node; the preview follows.

## B-spline

- Each click (or the start of a drag) places a control point; there are
  no handles. The curve is the cubic B-spline of the control points: it
  starts at the first and ends at the last, tangent there to the control
  polygon, and stays inside the points' hull (quadratic with three
  points, a line with two). While drawing, the control polygon shows
  dashed with the curve over it, both following the pointer.
- Double-click or Enter finishes, Esc discards; clicking the first
  point (once there are three) closes it into the periodic spline,
  smooth all round. The result is a curve object (its nodes are the
  Bezier segments, one a knot span); the control points are not kept.
- Checked by `geometry::tests::bsplines_start_and_end_on_their_ends_and_stay_inside`
  and `pick_drag_tests::bspline_clicks_place_control_points_of_a_smooth_curve`.

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

- `pick_drag_tests::a_pen_click_after_a_pulled_node_gives_a_straight_segment`
- `pick_drag_tests::clicking_the_last_node_again_makes_it_a_cusp`
- `pick_drag_tests::dragging_from_the_first_node_shapes_only_the_closing_segment`
- `pick_drag_tests::the_pen_adds_a_node_on_a_segment_and_drags_a_node_of_the_selected_curve`
- `pick_drag_tests::the_pen_deletes_a_node_of_the_selected_curve_and_the_toggle_turns_it_off`
- `pick_drag_tests::the_pen_adds_and_deletes_nodes_on_the_curve_being_drawn`
- `pick_drag_tests::c_held_while_pulling_makes_a_cusp_that_stays_and_does_not_align`
- `pick_drag_tests::the_snap_tip_shows_while_placing_and_pulling_with_the_pen`
- `pen_look::tests` (preview geometry, colours and node marks, the
  closing preview, splitting and deleting nodes of the curve being
  drawn, the defaults)
- `curve_join::tests` (from either end, closing, two subpaths made one,
  reversing curves, in the curve's own space with one undo step)
- `ui::cursors::tests::tools_get_their_cursors` (join and close signs)

- Given a freehand drag of 100 jittered points along a line, then the
  result has at most a handful of nodes and spans the same bounds.
- `recognize::tests::strokes_become_the_shapes_they_look_like` (level and
  tilted rectangles, a circle and a tilted ellipse, a triangle, a line,
  recognition off)
- `ui::drawing_bars::tests::both_bars_draw`
