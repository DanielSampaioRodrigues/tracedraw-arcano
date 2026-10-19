# Convert to Curves

Object > Convert to Curves (Ctrl+Q), the property bar's button and the
context menu turn the selected objects into curve objects, keeping
their fill, outline, transparency and effects. One undo step.

| Object | Becomes |
|---|---|
| Rectangle, ellipse, polygon, star, other shapes | a curve with the same outline (corners, arcs and points included) |
| Artistic and paragraph text | a curve made of the letter outlines |
| Clip frame | still a clip frame: its frame becomes a curve and its contents are converted by these same rules (a group inside stays a group with its objects converted), keeping their ids, and stay clipped inside it |
| Table | a group of curves: one filled rectangle per cell (merged cells as one), the cell text as curves, and one curve holding every cell border with the table's outline |
| Symbol instance | a group holding the symbol's objects as curves, where the instance was, at its size and turn; a symbol inside a symbol is converted too |
| Curve, group | stays as it is |
| Bitmap | stays as it is: a bitmap has no outline to convert. The status bar says so |

The command is greyed when nothing in the selection would change
(only curves, groups and bitmaps selected, or clip frames whose frame
and contents are already curves).

## Checks

- Given a 2 x 2 table with a border, a cell fill and text in one cell,
  when it is converted, then it is a group of six curves (four cell
  fills, the text, the border) covering the same area, and Undo brings
  the table back (`ops::curves_and_target_tests::a_table_becomes_curves_with_its_cells_borders_and_text`).
- Given an instance of a symbol made of a rectangle and an ellipse,
  moved 5 mm, when it is converted, then it is a group of two curves at
  the moved place (`ops::curves_and_target_tests::a_symbol_instance_becomes_the_curves_of_its_symbol`).
- Given a clip frame of an ellipse holding a rectangle and a bitmap,
  when it is converted, then it is still a clip frame whose frame and
  rectangle are curves with the same ids and fill, the bitmap is left as
  it is, and then the command is greyed and a second conversion changes
  nothing
  (`ops::curves_and_target_tests::a_clip_frame_keeps_its_contents_as_curves`).
- Given a bitmap, the command is greyed and the bitmap is left as it is
  (`ops::curves_and_target_tests::a_bitmap_is_not_converted`).
