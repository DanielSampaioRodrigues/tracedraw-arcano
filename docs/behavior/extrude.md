# Extrude

A body built from an object's outline, as a live effect
(`Effect::Extrude`): side faces between the outline and a copy of it,
and that copy as the far or the near face. The geometry is in
`tracedraw_core::extrude`, the tool in `extrude_tool.rs`, the bar and
the controls the Extrude panel shares in `ui/extrude_bar.rs`.

## Types

| Type | The copy | Where it is drawn |
|---|---|---|
| Small back | each point `p` to `p + (V - p) * d` | behind the object |
| Big back | `p + (p - V) * d` | behind the object |
| Small front | `p + (V - p) * d` | in front; the object behind |
| Big front | `p + (p - V) * d` | in front; the object behind |
| Back parallel | `p + v` | behind the object |
| Front parallel | `p + v` | in front; the object behind |

`V` is the vanishing point, `d` the depth over 100 (1 to 99), `v` the
offset of a parallel body. Older files without a type read as Back
parallel, or Small back when they have a vanishing point.

## Faces

- The outline is flattened (0.05 mm) subpath by subpath (at most 400
  points each); a subpath that ends where it started counts as closed.
  Every edge gives a quad to its copy.
- A face shows when its normal (pointing away from the filled side, so a
  hole's walls face into the hole) points along the way the body runs
  at that face (back types) or against it (front types). Open subpaths
  show both sides.
- Faces between corners (where the outline turns by more than 20
  degrees) make one surface: its faces are filled and edged with a
  hairline of their own colour, and the surface is outlined once with
  the object's outline (at most 0.1 mm wide), so a curved side is
  smooth, not a row of slats.
- Surfaces are drawn farthest first: along the offset for parallel
  bodies, by distance from the vanishing point for the others (nearer
  is farther for Small back and Big front, the other way for Big back
  and Small front).
- A back body draws its copy first (the far face, in the darkest
  colour), then the surfaces, then the object; a front body draws the
  object, the surfaces, then the copy looking like the object (its fill
  and outline).

## Colour and light

- Object fill: the object's colour; Solid: one colour; Shade: a linear
  fountain on each face from the first colour at the object's side to
  the second at the copy's. The far face takes the object's or the
  solid colour half way to black, or the shade's second colour.
- Each face is darkened by the light (one light; angle 0 = from the
  right, counterclockwise; intensity `k` = 0 to 100 %):
  `0.55 + 0.45 * k * (n . l + 1) / 2 + 0.45 * (1 - k)`, at least 0.2.

## The tool

- A drag on an object (inside it counts, filled or not): a parallel
  body runs along the drag; the others take the point the drag ends on
  as the vanishing point. One undo step per drag.
- The selected extrusion's handle is a square with a cross, joined to
  the object's middle by a dashed line: the vanishing point, or the
  middle of a parallel body's copy. A drag that starts on it (within 6
  pixels) moves it.
- A click selects the object under the pointer (or nothing). Groups and
  bitmaps are not extruded.

## Property bar and panel

| Control | Default | Range |
|---|---|---|
| Type | Small back | the six types |
| Offset (parallel) | 6, 6 mm | document units |
| Depth | 20 | 1 to 99 |
| Vanishing point | up and to the right of the object (Apply), or where a drag ended | page coordinates, document units |
| Locked to | Locked to object | object (moves with it) or page |
| Colour | Object fill | Object fill, Solid, Shade (two colours) |
| Light angle | 135 degrees | 0 to 360 |
| Light intensity | 50 | 0 to 100 |

- The controls show the selected extrusion and edit the selected
  extrusions live (a run of changes is one undo step). Apply (shown when
  a selected object has none) adds the extrusion; Clear Extrude removes
  it; Break Apart turns the effects into objects.
- Effects > Extrude opens the panel with the same controls. The values
  are kept in the preferences (`tools.extrude`).
- Not done: 3D rotation of the body, more than one light, bevels.

## Checks

- `extrude::tests::back_parallel_shows_the_faces_toward_the_vector`
- `extrude::tests::front_and_vanishing_bodies`
- `extrude::tests::every_subpath_extrudes_and_holes_show_their_far_walls`
- `extrude::tests::a_curved_side_is_one_outlined_surface`
- `extrude::tests::odd_values_give_no_faces`
- `extrude_tool::tests::a_drag_extrudes_toward_the_pointer`
- `extrude_tool::tests::the_bar_edits_the_selected_extrusion`
- `extrude_tool::tests::settings_read_older_effects_and_stay_in_range`
- `ui::extrude_bar::tests::the_bar_and_the_panel_draw_for_every_kind`
- Given a 20 mm square, Back parallel, offset (10, 10), then the far
  face is the square at (10, 10) and only the top and right faces show,
  the top one lighter with the light from the top left.
- Given that square, Small back toward (60, 60) at depth 25, then the
  far face is 15 mm wide starting at (15, 15).
