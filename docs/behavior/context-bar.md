# Contextual bar (Arcano edition)

A small floating bar with the actions that suit the selection. The
work is in `apps/tracedraw/src/ui/context_bar.rs`.

## Where

- Under the selection's bounds, centred, 18 px below; with nothing
  selected, at the foot of the drawing area. Kept inside the drawing
  area.
- A rounded panel-coloured bar with a soft shadow; AI actions carry
  their picture.

## What

| Selection | Buttons |
|---|---|
| Nothing | Image from text, Vectors from text, Import |
| One bitmap | Remove background, Expand, Upscale, Variations, More AI |
| One group | Ungroup, Duplicate, Align |
| Only text | Text styles, Fix typography, Convert to curves |
| Two or more objects | Group, Align, Shaping |
| One other object | Duplicate, Convert to curves, Shaping |

- The AI buttons open the Generate with AI panel at that action (see
  ai-actions.md); More AI opens it on the page of actions.
- Align, Shaping and Text styles open their panels; Fix typography runs
  the house microtypography on the selection (one undo step).

## When

- Shown with a drawing, the Pick tool, no text being typed, no move or
  transform under way and no panel being dragged; hidden while a mouse
  button is held anywhere but on the bar itself.
- The cross at its end turns it off (a status line says how to turn it
  back on); Options > Appearance > Contextual bar turns it on and off.
  On by default (off in tests).
