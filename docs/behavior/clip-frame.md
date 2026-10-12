# Clip frames

Object > Clip to Frame: objects placed inside a container that clips
them.

## Model

`ShapeKind::ClipFrame { frame, contents }`: the frame is any closed
object, kept whole with its own transform, fill and outline; the
contents keep their own page-space transforms. The frame's fill and
outline are drawn, the contents are clipped to the frame's path.

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

- Given a locked clip frame, when the frame is moved by (10, 0), then the
  contents move by (10, 0).
- Given an unlocked clip frame, when the frame is moved by (10, 0), then
  the contents' page bounds are unchanged.
- Given a `.cdr` bitmap with a crop outline smaller than the image, then
  the reader produces a clip frame of the bitmap inside the crop path
  (confirmed with the 2019 banner file).
- `frame_type::tests::empty_clip_frames_come_and_go_in_place`,
  `frame_type::tests::an_object_gives_way_to_an_empty_text_frame_of_its_size`.
