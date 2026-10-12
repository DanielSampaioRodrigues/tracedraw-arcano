# Dimension and callout tools

Parallel, Horizontal or Vertical, Angular and Segment dimensions, and
the 3-point callout. A dimension is a group of its lines, its arrowheads
and its label, with what it measures and its style in the group's object
data (`dimension`), so it is drawn again when its style changes or it is
moved, scaled, turned or skewed (`dimensions.rs`).

## Placing them

- Parallel and Horizontal or Vertical: drag from the first point to the
  second (or click both), then click where the dimension line goes (its
  distance from the measured points along their normal). Horizontal or
  Vertical measures the larger of the horizontal and the vertical span.
- Segment: a click on a segment of any object takes its two ends (a
  click away from every segment is the first point, as with Parallel);
  then click where the dimension line goes.
- Angular: the vertex, a point on the first side, a point on the second
  side, then where the arc goes (its radius; at least 1 mm). The angle is
  measured counterclockwise from the first side.
- While the last point is wanted, the dimension follows the pointer,
  drawn thin in blue; before that, a line from the last point. The
  property bar says what to click next; Escape starts again. Points snap
  as drawing does.
- The new dimension is selected; one undo step.

## Drawing

- Linear: extension lines from the measured points to 2 mm past the
  dimension line, the dimension line, filled arrowheads (3 mm long, 2 mm
  wide) at both ends. Angular: the two sides from the vertex to 2 mm past
  the arc, the arc, arrowheads along it at both ends.
- The label reads left to right: along the dimension line, turned to
  stay upright (a line pointing left or down reads the other way); for
  angles upright, out from the middle of the arc.
- Text position: Above the line (1 mm above it in reading direction; for
  angles outside the arc), In the line (centred on it, the line broken
  1 mm either side of the label), Below the line (for angles inside the
  arc).
- Lines are hairlines, arrowheads and label black, the label in the
  current font at the style's size.

## Property bar

Shown with the dimension tools and when the Pick tool's selection is all
dimensions (after the position and size fields). Changes apply to the
selected dimensions (one undo step) and to the ones made next; the style
is kept in the preferences (`tools.dimension_style`).

| Control | Default | Range |
|---|---|---|
| Precision | 1.23 (2 decimals) | 0 to 6 decimals |
| Units | Drawing units (the document's) | any unit of the Units list; lengths go through the drawing scale (at 1:100 a 10 mm line reads 1000 mm) |
| Show units | on | the unit's sign after the number; for angles, the degree sign |
| Prefix, Suffix | empty | text before and after (applied when the field is left or Enter is pressed) |
| Text | Above the line | Above the line, In the line, Below the line |
| Label size | 10 pt | 1 to 500 pt |

## Dynamic dimensioning

When a dimension is moved, scaled, turned or skewed, its transform goes
into its measured points and it is drawn again in the same undo step: a
dimension scaled by 2 reads twice the length, its label stays upright,
undistorted text of its size, and the group keeps no transform.

## 3-point callout

- Three clicks: the tip (what the callout points at), the elbow, and
  where the text starts. The callout is a two-segment line with an
  arrowhead at the tip, and a text object that opens for typing right
  away.

## Checks

- `dimensions::tests::labels_follow_the_style` (25.4 mm: `25.40 mm`,
  `W 1.000 " max` in inches with three decimals, a prefix and a suffix,
  `25` bare; a right angle `90.0°`)
- `dimensions::tests::dimensions_redraw_for_their_style_and_their_size`
- `dimensions::tests::previews_and_segments`
- `ui::dimension_bar::tests::the_bar_and_the_preview_draw`
