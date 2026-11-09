# Shaping

Object > Combine Shapes, the property bar's shaping buttons (with two or more
objects selected) and the Combine Shapes panel.

## Operations

The target gives the result its fill, outline and opacity; the other
selected objects are the sources. With objects picked one by one
(click, Shift+click) the target is the one selected last; with a
selection made by a marquee or a freehand region, the target is the
bottom-most object of it (as long as the selection is not changed by a
click afterwards). Checked by
`ops::curves_and_target_tests::shaping_targets_the_last_clicked_object_but_the_bottom_one_of_a_marquee`.

With only one object selected, Weld, Trim and Intersect (menu, Shaping
panel, property bar) wait for the target: the status bar asks for it,
the pointer becomes the target arrow, and the next click on an object
makes it the target, the selected object acting on it (found under the
selected object too, where they overlap). Esc, a right click, or a
click on nothing or on the selected object cancels. The operation and
its result are one undo step. Checked by
`pick_drag_tests::trim_with_one_object_waits_for_the_target`.

| Operation | Needs | Result | What stays (menu, property bar) |
|---|---|---|---|
| Weld | 2 | one object covering all of them | nothing |
| Trim | 2 | the target without what the sources cover | the sources |
| Intersect | 2 | the area all of them share | the sources and the target |
| Simplify | 1 | each object without what objects above it cover | the objects, changed in place |
| Front Minus Back | 2 | the front object without the ones behind (its attributes) | nothing |
| Back Minus Front | 2 | the back object without the ones in front (its attributes) | nothing |
| Boundary | 1 | an outline around the selection (the target's attributes) | every object |
| Divide | 2 | every region where the objects meet, each an object | nothing |
| Outline Stroke | 1 | each object's outline, as a filled object | nothing |
| Merge | 2 | one object per fill colour | nothing |

The result goes on top of the active layer and becomes the selection; an
empty result changes nothing and the status bar says so. Each operation
is one undo step.

Open curves (#828, `ops::curves_and_target_tests::open_targets_are_cut_and_open_cutters_leave_shapes_alone`,
`shaping::tests::open_paths_are_cut_by_closed_regions`): an open curve as
the target of Weld, Trim or Intersect (or the object kept by Front Minus
Back and Back Minus Front) is cut where it crosses the closed sources and
stays an open curve with the target's attributes: Trim and the Minus
operations keep its parts outside every source, Intersect its parts
inside every source, Weld the sources welded together with the curve's
parts outside them, as one open curve. Each part is a piece of the
curve's own line or curve segment (`shaping::cut_open`).

An open curve acting on a closed object (#1005,
`ops::curves_and_target_tests::open_curves_split_closed_targets_into_pieces`,
`shaping::tests::closed_regions_are_split_along_open_cutters`): Trim,
Front Minus Back and Back Minus Front split the object along the curve,
after any closed sources have cut it. Each piece the curves divide it
into becomes an object of its own with the object's fill, outline and
opacity, and all of them are selected; nothing is lost, the pieces'
areas add up to the object's. A straight line, a polyline and a curve
cut alike, the pieces keeping the curves of both the object and the cut
(`shaping::slice_open`); several curves, or one crossing several times,
make more pieces. Only a stretch of a curve that runs from one edge of
the object to another cuts: a curve that ends inside or only touches it
cuts nothing there, and when that leaves the object whole and no closed
source cut it, nothing changes and the status bar says so. In a group
each object is split on its own. The open curves go or stay as the other
sources do (Trim keeps them). Weld and Intersect leave open sources out:
they stay as they are, and with no closed source left the operation does
nothing and the status bar says so. Simplify leaves open curves alone.

When the object whose attributes the result takes is a group: Trim, Front
Minus Back and Back Minus Front cut each object of the group on its own,
each keeping its fill, outline and opacity, the pieces a group (pieces
cut away entirely are left out); Weld and Intersect take the attributes
of the group's first object with a fill. The result took the group's own
attributes, none, and showed only in Wireframe (#539,
`back_minus_front_with_a_group_keeps_each_objects_colours`).

## Divide, Outline Stroke and Merge

Three operations build objects of their own from the selection and keep
none of the originals, so they never ask what to leave.

Divide (`ops::divide_shapes`, `shaping::divide`) cuts the outlines where
they cross and gives every region an object of its own, filled with the
fill of the topmost object that holds the region whole (the part of the
region left outside that object is empty): where two objects overlap, the
region they share takes the front one's fill. The region itself says
which object it came from, not a point of its box, which lands on the
boundaries where the regions meet. Nothing is lost and nothing is counted
twice, the regions tiling the union of the objects. Four objects make up
to fifteen regions and are the most worked out; with five or more the
status bar says nothing was divided.

Outline Stroke (`ops::outline_strokes`) replaces each object by a filled
object in the outline's colour, as wide as the outline is drawn: centred,
inside or outside, with its caps, joins, dashes (fitted to the corners
when the outline aligns them), nib or variable width, its width scaled
with the object when it scales (`shaping::outline_band`, and
`shaping::stroke_area` for a plain centred outline). A hairline, or an
object without an outline, contributes nothing and stays as it is; when
no object had an outline the status bar says so.

Merge (`ops::merge_shapes`) welds the objects of one fill colour into one
and removes what the colours in front cover, so the colour in front keeps
the overlap: red behind blue leaves red cut by blue and blue whole. An
object whose fill is not a plain colour takes no part and stays as it is.

Checked by `ui::shaping_docker::tests::dividing_two_squares_makes_three_shapes_in_their_colours`,
`ui::shaping_docker::tests::outlining_a_square_gives_the_band_of_its_outline`
and `ui::shaping_docker::tests::merging_two_colours_keeps_the_front_one_whole`.

## Curves

- The operations run on the outlines flattened to 0.02 mm (`i_overlay`),
  and the curves of the objects are then put back into the result
  (`shaping::recover_curves`): every vertex of the polygon result that
  lies on an object's outline (within 0.03 mm, the crossing of two
  flattened edges lies up to 0.02 mm off the curves) is placed on it, and
  a stretch of vertices running along one subpath becomes that stretch
  of the subpath itself, with its segments and handles. The result keeps
  the Bezier segments of the objects, with new nodes only where their
  outlines cross; two circles welded give a handful of curve segments,
  not hundreds of tiny lines.
- A vertex where two outlines cross is moved onto the exact crossing of
  the curves. Where they cross at a shallow angle, several vertices of
  the flattening lie near the crossing and all land on it; they become
  one node (vertices closer than 1e-7 mm are one), so the result has no
  lines of no length there (`shaping::tests::shallow_crossings_give_one_node_each`).
- An edge that does not follow an object's outline (a chord across an
  arc, as where a cut runs straight across a curve) stays a straight
  line: a stretch is taken from the source only when the edges lie
  within 0.04 mm of it, so the result is never worse than the polygon.
  Nodes of the objects on the result stay nodes.
- Checked by `shaping::tests::welding_circles_keeps_their_arcs`,
  `shaping::tests::lines_and_arcs_both_come_back` and
  `shaping::tests::a_chord_stays_a_line`.

## Panel

- Ten picture buttons choose the operation; under them its name, a
  large picture and what it does, and that the last object selected is
  the target.
- Weld, Trim and Intersect each keep their own choice of what to leave:
  "Leave original source object" and "Leave original target object"
  (Weld: neither, Trim: the sources, Intersect: both, as the menus do).
  The choices and the operation are kept in the preferences
  (`panels.shaping_op`, `panels.shaping_leave`).
- The button under them, named after the operation, applies it; it is
  pale until enough objects are selected, and a line says how many.

## Fill rule

`shaping::overlay` reads both inputs with the even-odd rule, how curves
fill: contours of one object that overlap make holes there.
`shaping::overlay_with` takes the rule (`FillRule::EvenOdd` or
`FillRule::NonZero`) for outlines meant for the nonzero rule, how text
fills (glyphs drawn as overlapping strokes, traced logos), so they need no
clean-up first; a contour turning the other way is a hole under both
rules. The result covers the same area under the even-odd rule
(`shaping::tests::overlay_reads_its_inputs_with_the_fill_rule_given`).
`shaping::nonzero_as_even_odd` turns one such outline into an even-odd one
on its own.

## Checks

- `ui::shaping_docker::tests::what_the_operations_leave_of_the_originals`
- `ui::shaping_docker::tests::the_docker_draws_every_operation`
- `ui::shaping_docker::tests::dividing_two_squares_makes_three_shapes_in_their_colours`
- `ui::shaping_docker::tests::outlining_a_square_gives_the_band_of_its_outline`
- `ui::shaping_docker::tests::merging_two_colours_keeps_the_front_one_whole`
- `ui::propbar::tests::shaping_and_grouping_show_for_several_objects_groups_and_parts`
