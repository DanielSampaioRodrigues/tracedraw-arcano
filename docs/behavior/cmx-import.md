# CMX import

File > Open and File > Import read `.cmx` exchange files; so does
`tracedraw-cli` (`info`, `convert`). The file's layout is described in
`docs/cmx-format.md`.

## What comes in

- Every page at its size, its layers with their names, groups.
- Curves, polygons, rectangles and ellipses, at their place: the file's
  origin is the page's centre.
- Uniform fills, and fountain fills (linear, radial, conical, square)
  with their colours, angle, centre and edge pad.
- Outlines: colour, width, caps, join, behind fill, scaling with the
  object, dash patterns, and arrowheads (as custom heads, or the preset
  heads they match).
- Embedded 24-bit images, with their placement and crop.
- Clips.

## What does not, yet

Text, lenses, bitmap, texture and pattern fills, 16-bit coordinate files
and big-endian files. Whatever is skipped is named once in the import
report; opening never fails on it.

## Checks

- `cmx::tests` in `crates/cdr`: pages, layers and shapes; outlines,
  dashes and arrowheads; fountain fills; images after a jump; what is
  not read is reported; long instructions; other files and versions;
  broken files never panic.
- Public sample files compared with the previews they carry: most render
  within an RMS difference of 0.03 to 0.07; the outlines sample (0.23,
  arrowheads smaller than in the preview), a brushes sample (0.16,
  unread fill types), a text sample (0.16, text not read) and one more
  sample (0.22) are the known gaps.
