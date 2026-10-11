# Pick tool

| Action | Behaviour |
|---|---|
| Click | select the topmost object under the pointer (its fill counts only when filled; otherwise its outline within 3 px; text, groups, tables, bitmaps, clip frames and symbols count anywhere inside their bounds) |
| Shift+click | add or remove from the selection |
| Alt+click | select the object beneath the current one (dig) |
| Click on selected | toggle the rotate/skew handles: curved arrows at the corners, straight double arrows at the sides, the centre of rotation in the middle |
| Drag on empty | marquee; objects fully inside are selected; Alt makes touching count |
| Drag object | move with the pointer (snapped); Ctrl keeps the move horizontal or vertical, whichever is longer; Space drops a copy where the selection is and the move goes on |
| Corner handle | scale proportionally from the opposite corner; Shift from the centre; Ctrl in whole steps of 100 % (crossing the other side gives the mirror image at the same size); Alt scales freely |
| Side handle | stretch one axis from the opposite side (across it: mirrored); Shift from the centre; Ctrl in steps of 100 % |
| Rotate handle (rotate mode, corners) | rotate about the centre of rotation; Ctrl in steps of the constrain angle (15 degrees) |
| Skew handle (rotate mode, sides) | slant: the top and bottom handles sideways, the left and right ones up and down, the opposite side fixed; Shift about the centre line; Ctrl in steps of the constrain angle |
| Centre of rotation (rotate mode, a ring with a dot) | drag it to turn about another point (kept while the selection stays the same); double-click puts it back in the middle; a click on it keeps rotate mode |
| Right click during a move, scale, rotation or skew | the original stays and a copy takes the change (the copy becomes the selection); a second right click takes it back; the status bar says which |
| Esc during a move, scale, rotation or skew | cancels it; nothing changes |
| Arrows | nudge by the nudge distance (2.54 mm default); Shift by the super nudge distance (5.08 mm), Ctrl by the micro nudge distance (0.254 mm); all three in Document Options > Rulers |
| Tab / Shift+Tab | cycle selection in stacking order |
| Double-click | Shape tool on a curve; on text, the Text tool editing that text with the caret at the end |
| Esc | deselect |
| Double-click the tool button | select all objects |

The toolbox button of each flyout shows the tool last used from it; the
choice is kept between sessions (`toolbox` in the settings file).

Selecting inside a group: Ctrl+click.

The drag keeps going while the left button is held, also after a right
click (the windowing layer ends a drag when any button goes up); the
modifier keys are read at every step, so pressing or letting go of Shift,
Ctrl or Alt during the drag changes the result at once.

## The pointer

The pointer says what a press would do (code: `ui/cursors.rs`):

| Under the pointer | Pointer |
|---|---|
| A corner handle | diagonal resize arrows (top left and bottom right, or top right and bottom left) |
| A side handle | vertical (top, bottom) or horizontal (left, right) resize arrows |
| A selected object, or the centre of rotation | move arrows |
| A corner handle in rotate mode | a turning arrow |
| A side handle in rotate mode | two half arrows along the slant (sideways for top and bottom, up and down for the sides) |
| Anything else | the arrow (the cross hair with Options > Tools > Pick > Cross hair) |

While dragging, the pointer keeps the shape of what is being done (move
arrows, the handle's resize arrows, the turning arrow, the slant
arrows). Resize and move arrows are the system's own; the turning and
slant arrows are drawn by TraceDraw, black on a white halo.

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

- `pick_drag_tests::ctrl_keeps_a_move_straight`
- `pick_drag_tests::a_right_click_while_dragging_leaves_the_original`
- `pick_drag_tests::esc_cancels_a_drag`
- `pick_drag_tests::space_while_moving_drops_a_copy`
- `pick_drag_tests::shift_scales_from_the_centre_and_ctrl_mirrors`
- `pick_drag_tests::rotate_mode_skews_with_the_sides_and_turns_about_the_moved_centre`
- `pick_drag_tests::handle_modifiers_shape_the_scale`
- `pick_drag_tests::side_handles_skew`
- `pick_drag_tests::the_pointer_shows_what_a_press_would_do`
- `drop_menu::tests::move_here_and_copy_here`
- `drop_menu::tests::fill_outline_and_properties_go_onto_the_target`
- `drop_menu::tests::place_inside_makes_the_target_a_clip_frame`
- `drop_menu::tests::a_ctrl_drag_keeps_to_one_direction`
