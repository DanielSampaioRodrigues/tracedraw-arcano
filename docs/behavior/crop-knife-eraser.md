# Crop, Knife, Segment delete, Eraser

## Crop

- Drag a rectangle. Every object on unlocked layers (or the selection,
  when there is one) is intersected with it: curves become their clipped
  outline (shaping intersect on flattened paths, 0.02 mm), bitmaps keep
  their pixels inside a clip frame frame of the crop rectangle, groups
  are cropped member by member. Objects entirely outside are deleted.

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

- `knife::tests::straight_cuts_split_closed_objects`
- `knife::tests::freehand_cuts_and_open_curves`

- Given a 40 x 20 mm rectangle and a 4 mm vertical band through its
  middle, when erased, then the object is a path with two subpaths and
  the same bounds.
- Given a dot 100 mm away, then the object is unchanged.
- Given a 100 mm band across the whole rectangle, then the object is
  deleted.
