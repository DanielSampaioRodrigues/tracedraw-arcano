# Pick tool

| Action | Behaviour |
|---|---|
| Click | select the topmost object under the pointer (its fill counts only when filled; otherwise its outline within 3 px; text, groups, tables, bitmaps, clip frames and symbols count anywhere inside their bounds) |
| Shift+click | add or remove from the selection |
| Alt+click | select the object beneath the current one (dig) |
| Click on selected | toggle the rotate/skew handles |
| Drag on empty | marquee; objects fully inside are selected; Alt makes touching count |
| Drag object | move; Ctrl constrains to the axis; right-click while dragging leaves a copy; Space while dragging drops a copy |
| Corner handle | scale proportionally; Shift scales from the centre; Alt scales freely |
| Side handle | stretch one axis |
| Rotate handle | rotate about the centre marker; Ctrl constrains to 15 degrees |
| Skew handle | skew along the edge |
| Arrows | nudge by the nudge distance (2.54 mm default); Shift by the super nudge distance (5.08 mm), Ctrl by the micro nudge distance (0.254 mm); all three in Document Options > Rulers |
| Tab / Shift+Tab | cycle selection in stacking order |
| Double-click | Shape tool on a curve; on text, the Text tool editing that text with the caret at the end |
| Esc | deselect |
| Double-click the tool button | select all objects |

The toolbox button of each flyout shows the tool last used from it; the
choice is kept between sessions (`toolbox` in the settings file).

Selecting inside a group: Ctrl+click.

## Dragging with the right mouse button

Pressing the right button on an object (it is selected first if it was
not) and dragging shows the selection's outline in blue where it would
go, while the objects stay put; Ctrl keeps the drag horizontal or
vertical, Esc cancels it. On release a menu opens there:

| Item | Effect |
|---|---|
| Move Here | moves the selection by the drag |
| Copy Here | copies of the selection, moved by the drag, become the selection |
| Copy Fill Here | the object under the pointer takes the fill of the first selected object |
| Copy Outline Here | ... its outline |
| Copy All Properties | ... its fill, outline and transparency |
| Place Inside | the object under the pointer becomes the clip frame of the selection |
| Cancel | nothing |

The four items that act on the object under the pointer are dimmed when
there is none. A right click without a drag opens the context menu.

## Property bar

With nothing selected the bar shows the page fields (size, orientation,
units, nudge distance). With a selection it shows X, Y (lower left corner),
W, H, scale, angle, mirror buttons, the outline width, To Front, To Back
and Convert to Curves. Text selected on its own adds the text fields of the
Text tool after them. Bitmaps selected on their own replace Convert to
Curves with the bitmap commands: Edit Bitmap, Crop Bitmap (switches to the
Crop tool), Trace Bitmap (Quick Trace and the centreline and outline
presets), Straighten Image and Resample.

## Dialogs

Enter presses OK in every modal dialog (Esc cancels). In the two
multi-line fields (QR code text and document notes) Enter inserts a line
break instead.

## Checks

- `drop_menu::tests::move_here_and_copy_here`
- `drop_menu::tests::fill_outline_and_properties_go_onto_the_target`
- `drop_menu::tests::place_inside_makes_the_target_a_clip_frame`
- `drop_menu::tests::a_ctrl_drag_keeps_to_one_direction`
