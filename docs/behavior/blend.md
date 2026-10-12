# Blend

Intermediate objects between a start and an end object.

| Control | Default | Range |
|---|---|---|
| Steps | 5 | 1..999 |
| Rotation | 0 degrees | -3600..3600 (the panel: -360..360) |
| Acceleration (objects, colours) | 0 (linear) | -1..1 (the bar: -100..100 %) |
| Path | none | any curve (the panel's New path), "rotate all objects" |

Not yet: spacing along a path, a colour direction other than direct, a
loop, mapping nodes, splitting and fusing.

## Property bar (Blend tool)

Steps, Rotation, Object acceleration, Color acceleration; with nothing
blended selected they are what the next blend takes (Steps is kept in the
preferences), with a blend selected they change it live (a run of changes
is one undo step), and Clear Blend and Break Apart follow; otherwise a
line on dragging from one object to another.

Interactive: drag from the start object to the end object; the start
and end are the two selected objects' order on drop.

## Start and end objects

Both objects stay objects of the drawing. The start object holds the
Blend effect, which names the end object and keeps a copy of it in page
space (its own effects left out) for drawing the steps; the end object
draws itself (`blend_link.rs`). After every change of the drawing, in
the same undo step:

- the end object moved, resized, turned or recoloured: the copy is
  brought up to date and the steps follow;
- the start object changed: the steps follow (they are drawn from it);
- the end object deleted: the blend goes with it (Undo brings both
  back).

Selecting either object picks the blend for the bar and the Blend
panel. Clear Blend removes the blend and leaves both objects; Break
Apart turns the steps into objects. Files from before this kept the end
only inside the effect; such a blend draws its end with the steps.

## Formulas

For step `k` of `n` (`t = (k) / (n + 1)` with acceleration `t' = t ^ (1 + 2*acc)`):
- Geometry: both paths are resampled to the same node count
  (the larger of the two, nodes mapped by arc length, or by the user's
  node map), then interpolated node by node, including control points.
  Before that the end outline is wound the way the start outline is,
  and its first node is the one whose direction from its own centre is
  closest to the start outline's first node's direction from its centre,
  so equal shapes blend without twisting through the middle whatever
  their relative position.
- Transform: interpolated as translation + rotation + scale (not matrix
  entries), so blends rotate the short way unless "rotation" adds turns.
- Colour: mixed directly (each colour's values from start to end).
- Along a path: step `k` is placed at arc length `t' * L` on the path,
  oriented to the tangent when "rotate all objects" is on.

## Checks

- `ui::effect_bars::tests::the_blend_bar_edits_the_selected_blend`
- `blend_link::tests::the_end_object_stays_and_the_steps_follow_it`

- Given two 10 mm circles 100 mm apart, 9 steps, then the 5th step is
  centred at 50 mm and has a 10 mm diameter.
- Given two equal ellipses, the second down and to the right, every step
  keeps the ellipse's width and height within 1 mm; reversing the second
  outline changes nothing.
- Given red to blue direct with 1 step, then the step is (128, 0, 128).
