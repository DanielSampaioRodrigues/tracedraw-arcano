# Gradient fill

## Controls and defaults

| Control | Default | Range |
|---|---|---|
| Type | Linear | Linear, Radial (elliptical), Conical, Square, Shaped |
| Stops | 2: colour 0% = object colour or black, 100% = white | 2..64 |
| Angle (linear, conical, square) | 0 (left to right, counter-clockwise positive) | -360..360 |
| Edge pad | 0% | 0..49% |
| Centre offset (radial, conical, square) | 0, 0 | -100..100% of half the bounding box |
| Steps (display) | 256 | 2..256 |
| Repeat | None | None, Sawtooth, Mirror |
| Cycles (with a repeat) | 2 when a repeat is first chosen | 1..50 |
| Bilinear | off | |
| Invert colours | off | |

The editor shows the whole run as drawn (repeats, mirror and inversion
in) in a strip under the type. The Interactive Fill property bar and the
Transparency tool's gradient list offer the Shaped type too; a shaped
gradient has no handles on the canvas.

Shortcut: F11 opens the fill editor; G is the Interactive Fill tool.
Dragging with Interactive Fill on an object (anywhere inside it, filled
or not) puts the start node at the press point and the end node at the
release point, keeping the kind and colours of a gradient the object
already has (a linear black to white one otherwise); the skew handle
goes square to the line, as far from the start as the end. A click
gives the default fill; Ctrl constrains the angle to 15 degree steps;
dragging a palette colour onto a stop recolours it; dragging a colour
onto the fill line adds a stop.

Property bar (Interactive Fill): the fill type (None, Uniform, Gradient,
Pattern, Texture, Mesh), then the type's fields: the colour for a
uniform fill; for a gradient the kind (Linear, Radial, Conical, Square),
the start and end colours, the angle and the edge pad. With a selection
the fields edit it; with nothing selected they set the default fill for
the next object.

Handles on the canvas (Interactive Fill, gradient-filled selection):
three nodes placed freely, as the reference program shows them. The
start node and the end node are squares in the first and last colours,
joined by a dashed line with an arrow at the end; the skew handle is a
small round node joined to the start by a thin line. Radial fills draw
the dashed ellipse their frame makes, square fills the parallelogram.

- Start: moves the fill. A linear fill keeps its end where it was (the
  line gets shorter or longer and turns); radial, conical and square
  fills move whole, end and skew handle with the start.
- End: sets the length and the angle; the skew handle turns with the
  line and keeps its length.
- Skew handle: sets the second axis on its own, so a radial fill
  becomes an ellipse and a linear fill's bands slant.

Until a node is moved the fill has no stored nodes and is placed by its
angle and offset as below; the first move stores the three nodes
(`Fountain::handles`, relative to the object's bounding box, so they
follow when the object is moved or scaled) and later moves keep them.
A drag on a node is one undo step.

## Formulas

Bounding box `b` of the object in page space, centre `c`.

- Linear: axis from `c - h*(cos a, sin a)` to `c + h*(cos a, sin a)` with
  `h = (w*|cos a| + h*|sin a|) / 2`; `t = projection of p on the axis,
  normalised 0..1`.
- Radial: `t = |p - (c + offset)| / (max(w, h) / 2 * sqrt 2)`.
- Conical: `t = ((atan2(p - c') - a) mod 2pi) / 2pi`, mirrored so the
  colour at 0 and 1 is the same (`t = 1 - |2t - 1|`).
- Square: `t = max(|dx|, |dy|)` in the frame rotated by `a`, normalised
  by half the diagonal.
- Free nodes (start `s`, end `e`, skew handle `k`): the frame maps
  `(u, v)` to `s + u (e - s) + v (k - s)`; for a page point `p` take its
  `(u, v)` in that frame, then Linear `t = u`, Radial `t = hypot(u, v)`,
  Conical the mirrored angle of `(u, v)`, Square `t = max(|u|, |v|)`. A
  frame with no area is ignored (the angle and offset place the fill).
- Shaped: `t = d(p) / max d`, where `d` is the distance from `p` to the
  object's edge (0 outside) and `max d` the distance of the point
  deepest inside. The distances are exact Euclidean distances on a grid
  of at most 384 pixels across the object (`edge_distance` in the
  renderer, two passes of the 1-D lower envelope of parabolas), drawn
  smoothed; they do not change with zoom.
- The stop parameter `s` of `t` (`Fountain::stop_param`), in this order:
  1. Bilinear: `t = 1 - |2t - 1|` (first colour at both ends, last in
     the middle).
  2. Repeat with `n` cycles: `x = t * n`; Sawtooth `t = frac(x)` (1 at
     the very end); Mirror `t = frac(x)` on even cycles, `1 - frac(x)` on
     odd ones. No repeat: `n` is ignored.
  3. Edge pad `e`: `t = clamp((t - e) / (1 - 2e), 0, 1)` (per cycle).
  4. Invert colours: `s = 1 - t`.
- Colour at `s` is a linear interpolation in RGB between the two stops
  around it; stops are sorted by position; equal positions give a hard
  edge.
- Gradient shaders and exports that know one run of stops take
  `Fountain::resolved_stops`: the same colours as stops over 0..1, with
  a sawtooth's jumps as hard edges (two stops at one position).

## Checks

- Given a 100x10 mm rectangle with stops black 0%, red 50%, white 100%
  and angle 0, when rendered at 1 px/mm, then pixel (50, 5) is red
  within 16/255 and pixel (2, 5) is black, pixel (97, 5) is white.
- Given a conical fill, when sampled at two angles 180 degrees apart,
  then the colours differ.
- Given a PDF export of a 3-stop fill, then the shading uses a
  stitching function with one bound at 0.3.
- Given a 40 x 20 mm rectangle with a 0 degree linear fill, then the
  start and end nodes are at (0, 10) and (40, 10) and the skew handle is
  square to the line; moving the start to (10, 10) leaves the end at
  (40, 10) and keeps the skew handle's offset from the start
  (`fill_tool::tests::linear_nodes_move_as_the_reference_moves_them`).
- Given a radial fill, moving its centre node moves the whole fill
  (`fill_tool::tests::the_centre_of_a_radial_fill_moves_it_whole`).
- Given a radial fill on a square, a drag from (5, 5) to (15, 5) keeps
  it radial and red to white, puts the start and end nodes at those
  points and the skew handle at (5, 15)
  (`fill_tool::tests::a_drag_puts_the_nodes_where_it_began_and_ended`).
- Given free nodes, the renderer, SVG (`gradientTransform`) and PDF (a
  unit shading drawn through the frame) place the fill as the nodes say
  (`free_fill_nodes_place_the_gradient`,
  `free_fill_nodes_go_out_as_a_gradient_transform`,
  `free_fill_nodes_draw_a_unit_shading_through_their_frame`).
- Given a 100 x 10 mm black to white linear fill with Sawtooth, 2 cycles,
  rendered at 1 px/mm, then pixel 48 is light and pixel 51 dark; with
  Mirror, 2 cycles (or Bilinear), the middle is white and both ends black;
  with Invert, the left end is white
  (`tests::linear_fountains_repeat_mirror_and_invert`).
- Given a shaped black to white fill on that strip, then the edge is
  dark, the middle light, and points at the same distance from the edge
  have the same colour; on a circle, rings of equal colour
  (`tests::shaped_fountains_follow_the_outline`).
- `style::tests::resolved_stops_draw_the_same_colours_in_one_run` checks
  every combination of repeat, cycles, bilinear, invert and edge pad
  against `color_at`.

## Files and exports

The options are saved in `.tdraw` files (`repeat`, `repeats`,
`bilinear`, `invert`, kind `shaped`); they are left out when off, so
files without them read as before and the format version stays.

| Format | Repeat, bilinear, invert | Shaped |
|---|---|---|
| SVG | worked into the stops | a radial gradient from the centre to the nearer sides, the inside colour at the centre |
| PDF | worked into the stitching function (hard edges leave out the empty segment) | the renderer's pixels at 150 dpi, clipped by the path |
| EPS, EMF | worked into the stops | a square gradient from the centre out |
| `.cdr` | the closest plain gradient (`Fountain::to_plain`): the options worked into at most 64 stops, edge pad 0 | written as a square gradient with the stops reversed (inside at the centre) |

The `.cdr` byte layout does not change; a gradient without the options
is written exactly as before
(`write::tests::gradient_options_are_written_as_the_closest_plain_gradient`).

## Importing a gradient

The fountain fill editor's Import Gradient button reads a `.ggr` file
(`tracedraw_io::ggr`) and replaces the stops with it. Plain linear
segments with their middle halfway give their two ends; segments with
another blending (curved, sine, sphere, step), a middle elsewhere or an
HSV colouring are sampled into 17 stops. End colours given as the
foreground or background colour read as black and white. Transparency
in the file is not kept (fountain stops are opaque).

## Built-in gradients

The fill editor's Gradients section starts with Current colours: the
last fill colour picked to the last outline colour picked (the palette's
left and right clicks, black and white until then; `App::current_colors`,
checked by `app::tests::palette_picks_set_the_current_colours`). Then
come 29 swatches (Black to white,
Sunset, Ocean, Rainbow, Gold, Silver, Copper, Chrome, Fire, Ice, Forest,
Sky, Dusk, Aurora, Pastel, Neon, Peach, Mint, Berry, Lavender, Desert,
Steel, Sepia, Charcoal, then five landscapes: Dawn over the sea, Desert
dusk, Forest horizon, Night horizon, Snowy mountains;
`gradient_presets.rs`). A click replaces the stops and keeps the type,
angle and centre. The landscapes run from the ground or sea (0) through
a horizon near the middle to the sky (1); their swatches are drawn
bottom to top, and a click on a linear fill also sets the angle to 90
degrees so the sky is on top. Checked by
`gradient_presets::tests::every_preset_runs_from_0_to_1_in_order` and
`gradient_presets::tests::landscapes_run_ground_to_sky_at_ninety_degrees`.
