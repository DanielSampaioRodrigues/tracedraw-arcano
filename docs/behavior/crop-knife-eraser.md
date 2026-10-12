# Crop, Knife, Segment delete, Eraser

## Crop

- A drag draws the crop area. It stays on the page until it is applied
  or cleared: the drawing outside it is dimmed, its edge is dashed and
  it has eight handles (corners and side middles). A drag inside moves
  it, a drag on a handle (within 6 pixels) resizes it (past the
  opposite side it turns round), a drag outside draws a new one. Points
  snap as drawing does.
- Applied by a double-click inside it, Enter or the bar's Crop button;
  cleared by Escape (which then does nothing else), a click outside it,
  Clear Crop Area, or another tool. An area smaller than 0.1 mm is
  dropped.
- The objects cropped are the selection, or with nothing selected every
  unlocked object on the page's unlocked, visible layers (`crop_tool.rs`):
  - wholly inside: left as they are; wholly outside: deleted;
  - closed outlines (a subpath that ends where it started counts) are
    intersected with the area; open curves are cut where they cross its
    edge and the pieces inside are kept, still open; text becomes its
    outlines first; the result is a curve in the object's own space,
    with its fill, outline and effects;
  - bitmaps, tables and symbols go into a clip frame of the area (no
    fill or outline), so their content stays whole and can be taken
    out again (Extract Contents);
  - groups are cropped member by member (a group left empty is
    deleted); a clip frame by its frame.
- One undo step; the deleted objects leave the selection.
- The bar (with an area): X and Y of the area's point at the object
  origin, W and H (sized about that point), Crop, Clear Crop Area.
- Not done: turning the crop area.

## Knife

- Two-point line (first choice): drag a straight cut; it runs on past
  both ends across the objects. Freehand: drag a line of any shape; the
  cut follows it as drawn.
- The objects cut are the selection, or with nothing selected every
  visible object on the page's unlocked layers; bitmaps, groups and text
  are not cut (`knife.rs`).
- Closed objects lose a band along the cut: Split (first choice) takes a
  hair (0.01 mm), so the pieces meet; Gap takes the width beside it.
  Each separate piece becomes an object of its own (Keep as one object:
  the pieces of each object stay one object of several parts); a cut
  that leaves one piece changes nothing. Open curves are split where the
  cut crosses them and stay open.
- Pieces keep the fill, outline and opacity; the first keeps the
  object's place. The pieces are selected; one undo step.
- The bar: Two-point line, Freehand; Split, Gap and its width; Keep as
  one object; kept in the preferences (`tools.knife`).

## Segment delete

- Click a segment between two nodes (hit distance 5 screen pixels): the
  segment is removed, splitting the subpath in two open subpaths; a
  closed path becomes open. The object is deleted when nothing is left.

## Eraser (X)

- Thickness: default 6.35 mm (property bar, in the document units).
  Nib: round (default) or square.
- Drag: the band swept by the nib (`shaping::stroke_band`, round caps
  and joins for the round nib, square caps and mitre joins for the
  square one) is subtracted from the selected objects, or from the
  object under the start point when nothing is selected. Objects become
  curves; an object that vanishes completely is deleted. Objects whose
  bounds the band does not touch are left alone.
- Click: erases a dot of the nib's size.
- Double-click an object: deletes it whole.
- Bitmaps, groups and clip frames are not erased (erasing bitmaps by
  masking is not implemented).

## Checks

- `crop_tool::tests::the_area_waits_and_moves_until_it_is_applied`
- `crop_tool::tests::open_curves_groups_bitmaps_and_text`
- `crop_tool::tests::handles_and_resizing_stay_sane`
- `ui::crop_bar::tests::the_bar_and_the_area_draw`
- `knife::tests::straight_cuts_split_closed_objects`
- `knife::tests::freehand_cuts_and_open_curves`

- Given a 40 x 20 mm rectangle and a 4 mm vertical band through its
  middle, when erased, then the object is a path with two subpaths and
  the same bounds.
- Given a dot 100 mm away, then the object is unchanged.
- Given a 100 mm band across the whole rectangle, then the object is
  deleted.
