# Clip frames

Object > Clip to Frame: objects placed inside a container that clips
them.

## Model

`ShapeKind::ClipFrame { frame, contents }`: the frame is any closed
object, kept whole with its own transform, fill and outline; the
contents keep their own page-space transforms. The frame's fill and
outline are drawn, the contents are clipped to the frame's path.

The clip frame keeps the id of the object it was made from; the frame
inside it gets an id of its own (Place Inside Frame), so it can be
found, painted and reshaped, and Extract Contents gives it the clip
frame's id back. A fill or outline given to the clip frame (palette,
Properties, the outline fields) goes to its frame, never to the
contents (`for_group_deep`); the status bar and the Properties docker
show the frame's fill and outline (`mixed::leaves`). With the Shape tool
the clip frame's nodes are its frame's: the corner nodes of a rectangle
frame round its corners, an ellipse frame takes a pie or arc, a curve
frame is edited node by node, and the clip frame stays one
(`App::shape_tool_shapes`, `node_target`).

## Commands

- Place Inside Frame: with contents selected, click the frame; the
  contents keep their position.
- Extract Contents: the contents become ordinary objects again; the frame
  stays (an empty frame shows a hatched preview).
- Edit Contents: enters the clip frame; only the contents are editable,
  the rest of the page is dimmed and the frame is drawn as an outline.
  Finish Editing Contents leaves it. While editing, the property bar
  shows Finish.
- Lock Contents to Frame (default on, stored as `clip_frame.unlocked` in
  the object data when off): with the lock on, moving, scaling or
  rotating the frame transforms the contents too. With it off, the frame
  moves alone and the contents stay where they are
  (`compensate_unlocked_clip_frames` undoes the transform on them).

## Picking

A click selects the clip frame (anywhere in its box). Ctrl+click
selects the object of the contents under the pointer, only inside the
frame where the contents show; further Ctrl+clicks go into a group
among the contents. Ctrl+Alt+click digs through the contents and then
the clip frame (see `pick-tool.md`).

## Opacity mask

Object > Clip to Frame > Opacity Mask (a check item) turns the selected clip
frames into opacity masks and back: the frame is no longer drawn and no
longer clips hard; its brightness (its fill, gradients and patterns
included, through the luminance of the colour) sets how much of the
contents shows, white fully, black not at all, nothing outside the
frame. Invert Mask swaps that (black shows). Place inside, extract, edit
in place and lock work as for any clip frame. One undo step each.

- Screen and bitmaps: the frame is drawn alone and its luminance masks
  the contents' layer (`Shape::opacity_mask`, `crates/render`).
- SVG: a `<mask>` holding the frame's fill (inverted through a
  `feColorMatrix` filter), on a group of the contents.
- PDF: the masked object goes in as a picture with its transparency
  (300 dpi, at most 40 million pixels); PDF/X-1a flattens it with what
  lies under it.

## Frame Type (object context menu)

- Create Empty Clip Frame: each selected object that is not a clip
  frame becomes one holding nothing, in place (turned or not), as Place
  Inside Frame with no contents does; one undo step. Pale when nothing
  selected can become one.
- No Frame: each selected empty clip frame becomes the object it was
  made from, with its id; one undo step. Pale without one.
- Create Empty Text Frame (one object selected): the object gives way to
  an empty paragraph text frame of its size (its bounds) in its place,
  in the font, size, style, alignment and hyphenation new text takes;
  one undo step. The frame stays selected (and stays when left empty)
  for the Text tool.

## Checks

- `tracedraw_core::command::tests::a_clip_frames_frame_is_found_and_painted`
- `pick_drag_tests::a_clip_frame_is_painted_and_reshaped_through_its_frame`

- Given a locked clip frame, when the frame is moved by (10, 0), then the
  contents move by (10, 0).
- Given an unlocked clip frame, when the frame is moved by (10, 0), then
  the contents' page bounds are unchanged.
- Given a `.cdr` bitmap with a crop outline smaller than the image, then
  the reader produces a clip frame of the bitmap inside the crop path
  (confirmed with the 2019 banner file).
- `render::tests::opacity_masks_show_the_contents_by_the_frame_brightness`,
  `svg::tests::opacity_masks_are_svg_masks`,
  `pdf::tests::opacity_masks_go_in_as_transparent_pictures`,
  `command::tests::only_clip_frames_take_an_opacity_mask`,
  `app::tests::clip_frames_turn_into_opacity_masks_and_back`.
- `frame_type::tests::empty_clip_frames_come_and_go_in_place`,
  `frame_type::tests::an_object_gives_way_to_an_empty_text_frame_of_its_size`.
