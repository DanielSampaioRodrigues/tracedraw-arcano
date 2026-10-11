# CDR format notes

What the reader assumes, and what has been confirmed. Status legend:

- **confirmed (public spec)**: matches the public, declarative description
  of the format (a Kaitai Struct specification built from observed files).
  We use that description as documentation only; no code is taken from it.
- **confirmed (file)**: checked against a named file in our corpus.
- **assumed**: from public reverse-engineering notes or our own reading,
  not yet checked.
- **wrong**: kept for history.

Versions in this document are the format's major numbers (7 to 21 and
up), as the RIFF form type encodes them (`CDR7` to `CDR9`, then `CDRA` =
10 onwards, skipping `I`). The public spec writes them as hundreds
(1300 = version 13); `vrsn` stores the same number.

## Containers

| Item | Status | Note |
|---|---|---|
| `RIFF <size> CDR<v>`; `v` is a version char: `1`..`9` = 1..9, `A` = 10 ... `H` = 17, `J` = 18, `K` = 19; `I` is not used | confirmed (public spec) | We used to map `J` to 19. |
| Lowercase form type `cdr` with `8` is a CDR 8 variant (spec: 801) | confirmed (public spec) | Treated as version 8. |
| `vrsn` chunk: u16 version in hundreds (1300 = version 13, 801 = CDR 8 variant); `fver` has u16 full version, patch, minor, major | confirmed (public spec) | We read `vrsn` and prefer it over the form-type letter when sane. |
| Chunk: fourcc, u32 size, payload, pad byte when the size is odd | confirmed (public spec) | |
| 14 and 15: ZIP with `content/riffData.cdr` holding the RIFF stream | confirmed (public spec) | |
| `LIST cmpr` (7 to version 13 with compression on): two (compressed, uncompressed) u32 pairs, then two `CPng` blocks (`CPng`, `01 00 04 00`, zlib). Block 1 inflates to the chunk stream, block 2 to a pool of u32 sizes; inside the inflated stream each chunk's size field is an index into the pool | confirmed (public spec) | We used to treat the body as plain sub-chunks and the inflated stream as byte-sized chunks. |
| Chunk sizes 4-byte aligned in some newer files | wrong | Not in the public spec; all chunks pad to 2 bytes. |

### version 16 and later (16+)

| Item | Status | Note |
|---|---|---|
| ZIP whose RIFF entry point is `content/root.dat`; `content/dataFileList.dat` lists the external stream members (one name per line), found under `content/data/` | confirmed (public spec) | `container.rs` |
| Any chunk whose declared size is exactly 16 bytes is a redirect record: `stream u32, length u32, offset u32, reserved u32`; the payload is `length` bytes at `offset` in stream `stream` (index into `dataFileList.dat`) | confirmed (public spec) | `riff.rs`, gated on version 16+. |
| Stream `0xFFFFFFFF` means the payload (at most 8 bytes) is stored inline right after the length field | confirmed (public spec) | |
| A redirected `LIST` body starts with its form type in the external stream | confirmed (public spec) | |

## Units and coordinates

| Item | Status | Note |
|---|---|---|
| Version 6+: coordinates are i32 in 1/254000 inch (0.0001 mm) | confirmed (public spec) | |
| Before version 6: coordinates are i16 in 1/1000 inch; header fields, counts, ids and offsets that are u32 later are u16 | confirmed (public spec) | New in the reader. |
| Angles: i32 in millionths of a degree (i16 in tenths of a degree before 6) | confirmed (public spec) | |
| Page origin is the page centre; shapes carry their own transform (`trfd`) | confirmed (file: banner, version 21) | A full-page picture sits at (-w/2, -h/2) after its translation. |
| Objects inside a layer are stored front to back (the first `obj ` in the file is the top-most) | confirmed (file: banner, version 21) | Text, then a patch, then the picture underneath. The reader reverses the order into its bottom-to-top layers. |
| `mcfg` holds the document page size even when the content bbox under `LIST page` is smaller; the `bbox` under a page is the content bounds, not the page | confirmed (file: banner, version 21) | Letter landscape page with a 247 x 164 mm picture; the master page has a near-empty bbox. |
| Version 21 files (`CDRM`, `vrsn` 2100) parse with the version 16+ rules: `root.dat`, redirect records, 0x05 bitmaps with `bmp ` in `Bitmaps.dat`, `txsm` layout version 1800 | confirmed (file: banner, version 21) | The file also carries `color/*.icc`, `styles/document.cdss` (JSON styles) and `META-INF/textinfo.xml` (plain text of every text object), which are not needed to read the drawing. |
| `color/docPalette.xml` colours in `CMY`, `HSB`, `HLS`, `LAB`, `YIQ`, `Gray` and `Registration`: each component a fraction of 255 (L a fraction of 100, a and b offset by 128), as their names spell them ("C60 M169 Y255", "L76 a-14 b66") | confirmed (file: public sample, version 17) | CMY is the complement of RGB. Spot colours (`PANTONE ...`) are skipped. |
| `color/profiles/cmyk/*.icc`: the CMYK profile the drawing's CMYK values are in (`color/color.xml` names it) | confirmed (file: public sample, version 19) | Converting a card's C0 M20 Y60 K20 through it gives the preview's colour (231, 214, 163) exactly; the drawing carries the profile and shows its CMYK colours through it. |
| Object style string `transparency`: `uniformTransparency` is the share that lets the background through (0.3 draws the object at 70 %) | confirmed (file: public sample, version 19) | The preview's colour of a card at 0.3 is its colour at 70 % over white. Without it, the mean of `startTransparency` and `endTransparency` (assumed). |
| Version 16+ pages and layers keep their `loda` in a `lgob` list | confirmed (files: versions 17 to 21) | A page's own size (argument 19130) and layer names come from it: a second page of 60 x 40 mm in a 50 x 50 mm document. |
| `color/docPalette.xml`: the document palette, `<palette name="Document Palette"><colors><page>` then one `<color cs="CMYK" name="Green" tints="1,0,1,0"/>` per colour, tints as fractions in the colour space's order | confirmed (file: banner, version 21) | Read into the document palette (CMYK, RGB, grey). `color/color.xml` names the RGB, CMYK and grayscale profiles, the colour model and the rendering intent. |
| `mcfg` page size: two coords (width, height) after 12 unknown bytes (version 13+), 4 (9 to 12), 28 (version 6), 0 (7, 8, and 4 to 5); before 4: 2 unknown bytes then two corners | confirmed (public spec) | We lacked the version 6 offset and the pre-4 layout. |
| A page's own `loda` may carry argument 19130: width, height coords | confirmed (public spec) | Used per page when sane. |

## Structure

| Item | Status | Note |
|---|---|---|
| `LIST page` > `LIST layr` > `LIST obj ` > `loda` + `trfd`; groups (`LIST grp `) nest `obj ` lists and other `grp ` lists | assumed (layer > obj confirmed, file: banner, version 21) | Spec types the chunks but not the nesting. `grp ` lists become groups; other lists under a layer are descended, so an object list one level down is still found. |
| `flgs`: 4 bytes; byte 3 is the record type (0x08 object, 0x10 group, 0x90 page, 0x98 layer); for pages byte 2 != 0 marks the master page; for layers byte 0 is the layer type (0 normal, 0x08 desktop, 0x0a guides, 0x1a grid) | confirmed (public spec) | Guides and grid layers are imported hidden and unprintable. |
| Layer name: `loda` argument 1000 in the layer's own `loda`: NUL-terminated single-byte text before 12 (system code page), UTF-16LE from 12 | confirmed (public spec) | Single-byte text is decoded as Windows-1252 (the spec's own default; the real code page is the saving system's). |
| `lobj` has the same layout as `loda` | confirmed (public spec) | |
| `bbox`: four coords (two corners); `obbx`: eight coords (four corners) | confirmed (public spec) | Not used; we compute bounds. |
| `spnd`: u32 (u16 before 6); `usdn`: u32 static object id | confirmed (public spec) | Not used. |

## `loda` argument table

| Item | Status | Note |
|---|---|---|
| Header: length, argument count, offset of the offsets table, offset of the types table, object type (u32 each; u16 before version 6) | confirmed (public spec) | |
| Offsets table has argument count + 1 entries; the extra one ends the last argument | confirmed (public spec) | Used to bound each argument. |
| Types table is stored in reverse order: offsets[0] pairs with types[count - 1] | confirmed (public spec) | We used to pair them in the same order. |
| Argument types: 10 outline id, 20 fill id, 30 coordinates, 100 transform (pre-4), 200 style id, 1000 name, 2000 palette, 8000 opacity, 8005 container, 11000 polygon, 12010 gradient, 12030 rotate, 19130 page size, 40050 layer GUID | confirmed (public spec) | We read 10, 20, 30, 200, 1000, 8000, 11000, 19130. |
| Fill id and outline id are u32 references into the `fild`/`outl` tables (version 4+); before 4 the styles are inline | confirmed (public spec) | Inline pre-4 styles are not read. |
| Opacity argument: 10 unknown bytes (14 from version 13), then u16 in thousandths | confirmed (public spec) | Applied to `Shape.opacity`. |
| The value is a transparency (0 = opaque); a value above 1.0 is percent in thousandths | assumed | The spec only says "value / 1000"; we pick transparency and compute `opacity = 1 - value`. |
| Style argument (200): style id (u32; u16 before 6) into the `stlt` table | confirmed (public spec) | Used as the default run style of text objects. |
| Object types: 0x01 rectangle, 0x02 ellipse, 0x03 line and curve, 0x04 artistic text, 0x05 bitmap, 0x06 paragraph text, 0x14 polygon, 0x25 path (version 16+), 0x26 spline | confirmed (public spec) | Spline is not modelled. |

## Geometry (argument type 30)

| Item | Status | Note |
|---|---|---|
| Point list: `n` points of (x, y) coords followed by `n` type bytes; `n` is clamped to what fits | confirmed (public spec) | |
| Point type byte: bits 7..6 operation (00 move to, 01 line to, 10 cubic Bezier end point, 11 control point); bit 3 closes the subpath (also set on the move-to of a closed subpath); bits 5..4 node continuity (angle, smooth, symmetrical); bit 2 "can modify"; bit 1 character start | confirmed (public spec) | We had 10 and 11 swapped and closed on the move-to. |
| Rectangle before version 15: width, height, radius (coords); from version 9 three more radii follow (four corners) | confirmed (public spec) | Each corner keeps its own radius. |
| The four radii run counter-clockwise from the corner at (width, 0): (width, 0), (width, height), (0, height), (0, 0); with a negative width or height the corners mirror with it | confirmed (file: public sample, version 17) | A rectangle with a negative height and the fourth radius set has its top-left corner rounded, as its preview shows. The corner type byte (version 15+) is not read: corners import round. |
| Rectangle version 15+: width, height as f64 in coordinate units, scale x, scale y (f64), scale-with flag u8, 7 unknown; then four radii as f64 each followed by a corner type byte and unknown bytes (16 bytes between radii) | confirmed (public spec) | |
| Version 15+ radii are in inches (2.76 = 70.1 mm) with the scale-with flag clear | confirmed (file: public sample, version 17) | The one rounded corner of a 55.9 x 36.8 mm rectangle runs down the whole height in its preview. The spec multiplies by 254000 only when the flag is set; a file with the flag set would show its meaning. |
| Rectangle spans 0..width and 0..height from its origin; the scale fields repeat the scale of the object's transform and are not applied again | confirmed (file: public sample, version 19) | A 70.04 x 52.90 mm rectangle with scale 0.454 in both its record and its `trfd` has a `bbox` of 31.83 x 24.04 mm. Corrects a reading that applied the scale twice (an assumed row of 2026-10-09). |
| Corner sizes: the two corners on a side together are at most as long as the side (every corner shrinks by the factor of the tightest side) | confirmed (file: public sample, version 17) | A single corner can take the whole shorter side; equal corners stop at half of it. |
| Ellipse: width, height, start angle, end angle, pie flag (u32; u16 before 6); the ellipse spans 0..width, 0..height | confirmed (public spec) | Arc when the angles differ. |
| Line and curve (0x03): u32 point count then a point list, in all versions | confirmed (public spec) | We used a u16 count before version 16. |
| Path (0x25): 4 unknown, two u16 counts (added), 16 unknown, point list | confirmed (public spec) | New. |
| Polygon (0x14): coordinates hold the base curve as a point list; argument 11000 holds: 4 unknown (before version 13), angle count u32, next point u32, u32, 4 unknown (version 13+), rx f64, ry f64, cx, cy coords | confirmed (public spec) | |
| The base curve is one segment of a shape centred on the origin: the top vertex, the middle of a side (a star's inner point), the next vertex clockwise, at a radius of 1000 mm; rx and ry scale it, cx and cy place its centre | confirmed (file: public sample, version 19) | A pentagon with rx = ry = 0.0445 is 44.5 mm in radius in its preview. An inner point closer than the middle of a side makes a star (sharpness 1 - inner / outer radius). Corrects a reading of rx and ry as coordinate units. |
| Bitmap (0x05): two corners (4 coords), 32 unknown, image id u32, unknown (8 before 4, 12 in version 8, else 20), u32 count, clipping point list | confirmed (file: banner, version 21) | The point list is the crop outline in the object's local space; when it is not the full rectangle the object is imported as a clip frame (frame = outline, contents = the bitmap). A patch cut from a larger picture confirmed it. |
| Artistic text (0x04): origin x, y; paragraph text (0x06): 4 unknown, width, height | confirmed (public spec) | Origin is the baseline start; the frame spans (0, 0) to (width, height) like our rectangles (assumed). |
| The `txsm` chunk of a text object is inside the object's `LIST obj ` | assumed | The spec does not state how a text object finds its content; the first `txsm` under the object list is used. |

## `trfd` transforms

| Item | Status | Note |
|---|---|---|
| Header: length, argument count, offset of the offsets table (u32 each; u16 before 6); no types table | confirmed (public spec) | We used to read a fourth header field. |
| Each record: 8 unknown bytes (version 13+), type u16; type 0x08 is a matrix: 6 unknown bytes (version 6+) then six f64 `a c tx b d ty` | confirmed (public spec) | We had 4 + 2 (before version 13) and 8 + 2 + 2 (version 13+) bytes before the matrix. |
| tx, ty are in coordinate units (1/254000 inch; 1/1000 inch before 6) | confirmed (public spec) | |
| Several matrices in one `trfd` | assumed | Only the first is used, with a warning. |

## `fild` fills (`fill` before version 7)

| Item | Status | Note |
|---|---|---|
| id u32; version 13+: tag u32 (1300) and body length u32; type u16; body | confirmed (public spec) | We skipped only 4 bytes on version 13+. |
| Fill types: 0 none, 1 uniform, 2 gradient, 6 PostScript, 7 two-colour pattern (older), 8 two-colour pattern (newer), 9 colour bitmap, 10 vector pattern, 11 texture | confirmed (public spec) | 6 and 10 become no fill with a warning. |
| Uniform before version 13: 2 unknown bytes then a colour | confirmed (public spec) | We read the colour 2 bytes early. |
| Uniform version 13+: tag u32, length u32, property list of (type u8, length u32, body) ending with type 0; type 1 = colour (12 bytes), 6 = Lab colour, 7 = palette GUID, 3/8/11/12 = special palette colour parts | confirmed (public spec) | Only the type 1 colour is used. |
| Gradient: 2 unknown (8 from version 13), type u8, unknown 11 (before 6) / 19 (6 to 12) / 17 (version 13+), edge offset (i32 in 6 to 12, else i16), angle, centre x, centre y (i32; i16 before 6), 2 unknown (6+), mode (u32; u16 before 6), mid point u8, 1 unknown, stop count (u32; u16 before 6; low 16 bits), 3 unknown (version 13+), stops, 3 unknown (version 13+), version 16+: four f64 (centre x, centre y, width, height relative to the object) when present | confirmed (public spec) | We read the angle as f64 and skipped the stops. |
| version 16+ transformation: the relative centre becomes `Fountain.offset` when the legacy centre offsets are zero; a width or height below 1 becomes `edge_pad = (1 - min(w, h)) / 2` when the legacy edge offset is zero; a fill larger than the object cannot be represented and is drawn at object size | assumed | The spec documents the fields, not their scale; "relative to the object" is read as a fraction of the object's size. |
| Gradient stop: colour, unknown 0 / 5 (version 13, version 14) / 26 (version 15+), offset (u32; u16 before 6; low 16 bits in percent), 3 unknown (version 13+) | confirmed (public spec) | Stop sizes 16 / 24 / 45 bytes. |
| Gradient type byte: 1 linear, 2 radial, 3 conical, 4 square | assumed | Not in the spec. |
| Centre offsets and edge offset are percentages | assumed | Scaling is not in the spec. |
| Two-colour pattern: 2 unknown (8 from version 13), pattern id u32, width, height (i32; i16 before 6), tile offsets u16 x2 (before 9) or 4 unknown, rcp offset u16, flags u8 (bit 2 = relative size before 9), unknown 1 (6 from version 13), colour 1, unknown 0 / 10 (version 13 to version 15) / 31 (version 16+), colour 2 | confirmed (public spec) | The tile comes from `bmpf` (below); without it a placeholder tile is drawn. |
| `bmpf`: pattern id u32, then a device-independent bitmap: 40-byte info header (size 40, width, height, planes 1, 1 bit per pixel), two BGRx palette entries, rows padded to 4 bytes, bottom-up (top-down when the height is negative); the darker palette entry is the front colour | assumed | Not in the public spec. The header is searched for in the first 64 bytes and validated; anything else keeps the placeholder with a warning. |
| Image fills (9, 10, 11): leading records (version 13+: a 0x640 word followed by 0x640 bytes, or a bare 0x514 word, repeated) or 2 unknown bytes, pattern id, width, height (u32; u16 before 6), tile offsets (before 9) or 4 unknown, rcp u16, flags u8, unknown 21 (17 from version 13), pattern id again (6+) | confirmed (public spec) | Types 9 and 11 are read; 10 is not. |
| The image fill's pattern id names a `bmp ` image (same table as bitmap objects) | assumed | Colour bitmap fills become a tiled PNG at the stored tile width; textures are drawn from that bitmap too (not regenerated), or fall back to a built-in texture with a warning when the bitmap is missing. |

## `outl` outlines

| Item | Status | Note |
|---|---|---|
| id u32; version 13+: records (id u32, length u32, body) until id 1, whose body is the outline | confirmed (public spec) | We skipped 4 fixed bytes. |
| line type u16, caps u16, join u16, 2 unknown (6 to 12), width coord, stretch u16 (percent), 2 unknown (6+), nib angle, unknown 0 / 52 (6 to 12) / 46 (version 13+), colour, unknown 10 (before 6) / 16, dash count u16, then a fixed area of 20 (before 6) / 22 bytes whose start holds the dash values (u16 each), start marker u32, end marker u32 | confirmed (public spec) | Dashes are stored in multiples of the width. We used to read the markers right after the dashes. |
| `arrw`: arrowhead id u32, point count u32, point list as in geometry | assumed | Not in the public spec. The outline is classified into our presets: three corners = arrow (open arrow when not closed), four = square, bar (aspect under 0.35) or diamond (not axis-aligned), closed all-Bezier = circle; anything else is kept as a custom arrowhead from the outline itself (assumed: tip towards +x); an undefined marker id = none, with a warning. |
| line type bit 0 = no outline, bit 4 = behind fill, bit 5 = scale with object; caps 0 butt, 1 round, 2 square; join 0 miter, 1 round, 2 bevel | assumed | Not in the spec. |

## Colours

| Item | Status | Note |
|---|---|---|
| Version 5+: model u16, palette u16, 4 unknown bytes, 4 value bytes (12 bytes) | confirmed (public spec) | We read the value 4 bytes early. |
| Version 4: model u16, C M Y K as u16 each, 2 unknown (12 bytes); before 4: model u8, 4 value bytes | confirmed (public spec) | |
| Models: 1 spot library, 2 CMYK percent, 3 CMYK 0..255, 4 CMY, 5 BGR, 6 HSB, 7 HLS, 8 black and white, 9 grayscale, 11 YIQ, 12 Lab (signed), 13 index, 14 spot library (hex), 15 six-colour process, 17 CMYK 0..255, 18 Lab (offset 128), 20 registration, 21 BGR with tint byte (tint already applied), 22 user ink, 25 spot, 26 multi-channel, 99 mixed | confirmed (public spec) | We map 2, 3, 4, 5, 9, 17, 20, 21; others become black with a warning. Model 17 was wrongly read as percent. |

## `bmp ` bitmaps

| Item | Status | Note |
|---|---|---|
| image id (u32; u16 before 6), unknown 14 (before 6) / 46 (6) / 50, colour model u32, 4, width u32, height u32, 4, bits per pixel u32, 4, bitmap size u32, 32; palette when bpp < 24 and model not 5 or 6: 2 unknown, count u16, BGR triplets; then the pixel bytes | confirmed (file) | Colour model 1 is 24-bit BGR (a version 21 file, 700 x 464, model 1, 24 bpp, size 974400 = 700 x 3 rounded to 4 x 464). Another public reader draws model 5 as greyscale, so the writer uses 1. |
| Pixel rows padded to 4 bytes, stored bottom-up; 32 and 24 bpp are BGR(x), 8 and 1 bpp index the palette | assumed | Device-independent bitmap convention. |

## Text and styles

| Item | Status | Note |
|---|---|---|
| `font`: id u16, encoding u16, style flags u32, 10 unknown, name to the end (UTF-16LE from 12, single-byte before) | confirmed (public spec) | Gives the family name per font id. |
| Text encodings: UTF-16LE from version 12; single-byte code page before (the `font` encoding byte names it: 0 Latin, 0xcc Cyrillic, 0xee Central European ...) | confirmed (public spec) | |
| Code page of text before version 12: the encoding field's low byte is a Windows character set value (0 Latin, 2 symbol, 128 Japanese, 129 Korean, 134 simplified Chinese, 136 traditional Chinese, 161 Greek, 162 Turkish, 163 Vietnamese, 177 Hebrew, 178 Arabic, 186 Baltic, 204 Cyrillic, 222 Thai, 238 Central European), read in the matching Windows code page (1252, byte values kept, 932, 949, 936, 950, 1253, 1254, 1258, 1255, 1256, 1257, 1251, 874, 1250) | assumed | A run's text uses its style's set (the `txsm` style's encoding, or the style table's font entry) or its font record's, a non-Latin set winning over a Latin one; font names use their record's set; layer and object names use the non-Latin set most font records have. Other values read as Windows-1252. Not yet checked on a file. |
| Style flags u32: low 18 bits name the face (one bit each: thin, thin italic, ..., bold 0x1000, bold italic 0x2000, ..., heavy italic 0x20000), bits 18..20 underline, 21..23 overline, 24..26 strike-through, 27..28 script | confirmed (public spec) | Bold = any weight bit from semi-bold up; italic = any italic bit. |
| `stlt` (7+): the body of `LIST stlt` is one record, not sub-chunks: record count u32, then (when the count is not 0) the mapping tables, then the records | confirmed (public spec) | `riff.rs` leaves the list unparsed from version 7. Before 7 the list holds untyped sub-chunks we skip. |
| `stlt` mapping tables, each `count u32` then entries: fills (id u32, 4, value u32, 48 more from version 13), outlines (12 bytes), fonts (id u32, 12 / 20 (10+) unknown, font id u16, encoding u16, 8, size coord, 8, style flags u32, 8 (10+)), alignments (12 bytes), intervals (52), set5 (152), tabs (784), bullets (variable), indents (16 + 3 coords), hyphens (32, 36 from version 13), drop caps (28), set11 (12; only from the CDR 8 variant 801) | confirmed (public spec) | Fills, outlines, fonts, alignments, intervals and indents are kept. Our major version cannot tell 800 from 801; version 8 reads set11 as absent. |
| `stlt` interval entry: id u32, 8 unknown, character spacing u32, 8 unknown, line spacing u32, 24 unknown; both in millionths (1 000 000 = 100 %) | confirmed (public spec) for the layout; the meaning of the values is assumed | Line spacing becomes the paragraph leading in percent; character spacing becomes tracking in percent of the em when within -2000..2000 %. |
| `stlt` indent entry: id u32, 12 unknown, right, first line, left (coords) | confirmed (public spec) | Converted to millimetres into the paragraph indents. |
| `stlt` record: group count u32, style id u32, parent id u32, 8 unknown, name length u32 (characters), name, fill ref u32, outline ref u32; when the group count > 1: font ref, align ref, interval ref, set5 ref (and set11 ref when that table exists); when > 2: tab, bullet, indent, hyphen, drop cap refs | confirmed (public spec) | Refs index the mapping tables; the entry's value is the real `fild`/`outl`/`font` id. Parents are followed for unset properties. |
| Alignment values: 0 none, 1 left, 2 right, 3 centre, 4 full justify, 5 force justify | assumed | A common order for alignment values; not in the spec. |
| `txsm` 7 to version 15: frame flag u32, 32 unknown, 1 (version 15), (before 8: on-path u32 and 32 bytes when set), frame count, frames, paragraph count, paragraphs | confirmed (public spec) | Versions before 7 are not described and are skipped with a warning. |
| `txsm` 7 to version 15 frame: id u32, 48 unknown, (8+: on-path u32; when set 4, 8 (version 13+), 28, 8 (version 15); when clear 8 (version 15)), then 36 (7) / 32 (8) / 34 (8 variant 801 to version 13) / 36 (version 14) / 40 (version 15) bytes when the frame flag is clear, 4 bytes (version 15) when set | confirmed (public spec) | Version 8 is read as 32 bytes (800); 801 files may misparse. |
| `txsm` 7 to version 15 paragraph: style id u32, 1 unknown, 1 more (version 13+ with the frame flag), style count u32, styles, char count u32, char descriptions (4 bytes; 8 from 12), byte count u32 (from 12; before: one byte per char), text, has-path u8, 24 bytes per char when set | confirmed (public spec) | |
| `txsm` 7 to version 15 style: char count u16, flags u8, fl3 u8 (8+); flags: 0x01 font id u16 + encoding u16, 0x02 style flags u32, 0x04 size coord, 0x08 / 0x10 / 0x20 4 bytes each, 0x40 fill id u32 (+48 from version 13), 0x80 outline id u32; fl3: 0x02 URL (len u32, len x 2 bytes), 0x08 locale (4 bytes; from version 13 len u32 + len x 2), 0x20 a peeked flag byte that, when set, starts a 4 (52 from version 15) byte block | confirmed (public spec) | Styles are run lengths over the paragraph text. Sizes are in coordinate units (0.0001 mm) and converted to points. |
| `txsm` version 16+: frame flag u32, 32 unknown, layout version u16 (1500 ... 1800), 3 unknown, frame count, frames (id u32, 48 unknown, on-path u32 (+40 when set), 8 unknown, and when the frame flag is clear: 16 unknown, length u32, that many bytes (x 2 before version 17)), paragraph count, paragraphs | confirmed (public spec) | |
| Text fitted to a path: the frame's on-path flag is set and the path is another object of the text's list (its group, or its layer) | assumed | In a group of the text and exactly one curve, that curve. Otherwise the curve along which the text's settings (below) put the point the offset names inside the text's `bbox` (1 mm to spare), the nearest to the box's centre when several do; a text whose path is not found stays straight, with a warning. Before version 16 the settings are not read: offset 0, no distance. |
| `txsm` version 16+ paragraph: style id u32, 1 unknown, flag u8 (frame flag set; 64 bytes follow when it is 1), paragraph style string (layout < 1700 and no frame flag), default style string, record count u32, records (3 x u16; URL when the second is 0x3fff and the third has 0x11; locale when the third has 0x04; a style string when the second is not 0 or the third has 0x04), char count, char descriptions (flags u16, override index u8 >> 1, 1, 4), byte count, text, has-path u8 (+24 bytes per char) | confirmed (public spec) | The override index selects a record whose string overrides the default. Text encoding: UTF-16LE up to version 18; from version 17 one byte per character (UTF-8 when valid, else Windows-1252), confirmed with a version 21 file where a 6-character run is 6 bytes. |
| Style string: length u32 in UTF-16 units (bytes from version 17), then the text | confirmed (public spec) | |
| Style string values are JSON strings, numbers included (`"justify":"2"`, `"weight":"1024"`) | confirmed (files: versions 19 to 21) | Read as numbers when they parse as numbers. |
| `paragraph` `justify`: 2 centres the lines on one another | confirmed (files: public sample version 19, versions 21 owned by the author) | Artistic text centres each line on its origin. 3 (right), 4 and 5 (justify) are assumed; 0 and 1 read as left. |
| Font `weight`: the style code of the style tables, a power of two counting weights (64 = 400, 1024 = 600, 4096 = 700; an odd exponent is the italic of the weight below) | confirmed (file: public sample, version 19) | The embedded faces of the file have OS/2 weights 400, 600 and 700 for the runs set at 64, 1024 and 4096. Weights from 600 draw bold. |
| Font `underline` and `strikeout`: 0 or 1 | confirmed (file: public sample, version 19) | `overline` is not read. |
| The object's style string (`loda` argument 201) under the text records' styles: its paragraph alignment and character settings apply where the records say nothing | confirmed (files: versions 19 and 21) | The centring of multi-line artistic text comes from it. |
| Line breaks inside a version 21 text run are carriage returns (`\r`) | confirmed (files owned by the author, version 21) | Read as new lines; CR LF, U+2028, U+2029 and a vertical tab too. |
| Text fitted to a path (version 21): the `txsm` frame's 40 bytes after the on-path flag, ten i32: 0, the placement (4 seen), 0, an unknown value (16082 and 18667 seen), the offset, 0, the distance, 1, a horizontal and a vertical mirror flag; then 24 bytes per character after the has-path byte (two small integers, then the character's angle in millionths of a degree) | assumed (file owned by the author, version 21: two texts on one circle, compared with the preview the file carries) | Placement 4 puts the text's middle at the offset. Offset and distance are in coordinate units: the distance moves the baseline to the left of the path's direction (outwards on the clockwise circle, negative inwards); the offset runs along that baseline from the path's start, in the path's direction, mirrored or not. Both mirror flags set the text on the other side of the path, reading the other way (letters' tops towards the inside of the circle). An ellipse's path starts at its top and runs clockwise. The text's `bbox` is the box of the fitted letters. The per-character bytes are not read (the angles of the first and last letters of the upper text, 44.2 and -43.3 degrees, agree with the placement). With these readings both texts land where the preview shows them. |
| Style string content: a JSON object with `character` (font id, size, fill id, outline id) and `paragraph` (justify) sections | assumed | Not in the spec. We parse JSON leniently and also accept `key:value;` lists, matching key names loosely (size, weight/bold, italic, fill, outline, justify/align, font/family). Sizes above 1000 are taken as coordinate units, smaller ones as points. |
| Version 21 style string content: `character` holds `fill` and `outline` as objects (not ids), and the font settings three times, `RTL`, `farEast` and `latin` (`font` family name, `size` in coordinate units, `weight` 64 for a regular face, `italic`, `underline`, `strikeout`); `frame` holds `columns`; `paragraph` holds `interLineSpacing` (1000000 in a default paragraph), `interCharSpacing`, `interWordSpacing` and underline, overline and strikeout geometry (`thinOverlineThickness`, `dblUnderline1Distance` ...) | confirmed (file: banner, version 21) | Only `latin` is read for the font. The fill object's `type` 1 is a uniform fill whose `primaryColor` reads `MODEL,PALETTE,v1,v2,...`: `CMYK255,USER,0,0,0,255,100,<uuid>` (0..255) and `CMYK,USER,0,0,0,100,100,<uuid>` (0..100) were seen; `RGB255` and `GRAY255` are read alike (assumed); the value after the components (100) is not used. `type` 0 is taken as no fill (assumed). The geometry keys are skipped (their names would otherwise read as outline references). |
| Version 21 object style string: `loda` argument 201 holds u32 1, a length u32, then that many bytes of a JSON object with `fill`, `outline`, `character`, `frame` and `paragraph` sections | confirmed (file: banner, version 21) | The artistic text's `frame` section reads `"columns":"623721,0"`: a width in coordinate units and the gutter after it. For that one-line text the width is the line's advance width (9916 font units at 12.8819 mm per 2048-unit em = 62.372 mm). |
| Version 21 artistic text: object type 0x04, the coordinates (argument 30) hold the origin (0, 8 units), and the `txsm` record has the frame layout (its first u32 is 0xFFFFFFFF); the transform takes the origin to the start of the first baseline, and the object's `bbox` chunk is the box of the letters | confirmed (file: banner, version 21, measured with the font the file embeds) | The box runs 10.56 mm above and 1.73 mm below the baseline, as the glyph outlines of the six letters at that size give (10.561 and 1.723 mm), and starts at the first letter's left side bearing. Read as artistic text: the frame layout of the `txsm` record does not make it paragraph text. Corrects a reading of 2026-10-11 that took the box for a paragraph frame (the text came out 2.7 mm low). |
| Paragraph text (object type 0x06) whose coordinates hold no frame size (version 16+) | assumed | The frame is read as the `bbox` taken back through the object's transform (place and height) and the `columns` of the style string (width), for transforms that neither turn nor slant; otherwise the text is laid out unframed with a warning. Not seen in a file. |
| The 48 bytes after the frame id in a version 16+ `txsm` frame record | confirmed (file: banner, version 21) | Six f64: the frame's matrix (a, b, tx, c, d, ty), the same as the object's `trfd`, translation in coordinate units. Not read. |
| `ftil`: six f64 fill transform; `uidr`: colour id, user id, 36 unknown, colour; `DISP`: a preview bitmap header; `urls`: text | confirmed (public spec) | Not implemented. |

## Embedded fonts

| Item | Status | Note |
|---|---|---|
| Version 16+ ZIP files keep each embedded font in a member `embed/embedding<n>` (`embedding0` first) | confirmed (file: banner, version 21) | `font/fontTable.dat` lists the fonts the drawing uses (family, script, PostScript name, version string, style name) whether embedded or not; it is not read. |
| Next to a font, newer files keep pictures of it and a sample (`embedding0.png`, `embedding0-Extended.png`, `embedding0.sample.curve`) | confirmed (file: public sample, version 19) | Left alone. A family can be embedded once per face (regular and bold of the same family): each face serves the text. |
| An `embed/` member: a header of its own (2705 bytes in the file seen; the font's version string in UTF-16 among fields not understood), then an Embedded OpenType (EOT) structure that runs to the end of the member | confirmed (file: banner, version 21) | The EOT start is found where a well-formed EOT header's size reaches the member's end (else the first well-formed header). |
| EOT structure: little-endian `EOTSize`, `FontDataSize`, `Version` (0x00010000, 0x00020001 or 0x00020002), `Flags`, PANOSE, charset, italic, weight, `fsType`, `MagicNumber` 0x504C at offset 34, ranges, checksum, reserved, then `FamilyName`, `StyleName`, `VersionName`, `FullName` (each a u16 padding, a u16 byte size and UTF-16LE text, the family size at offset 82), version 2.1 and 2.2 extras, and the font data as the last `FontDataSize` bytes | confirmed (public spec: W3C Member Submission "Embedded OpenType (EOT) File Format", 2008) | The file seen: version 0x00020002, flags 0x10000000, a 59712-byte TrueType font. |
| EOT flag 0x10000000: every font byte is XOR-ed with 0x50; flag 0x4: the data is compressed with MicroType Express | confirmed (public spec), XOR seen in the file | Compressed fonts are not read (a warning names them). Data that is not a TrueType or OpenType file after decoding is skipped. |
| Use of an embedded font | design | The font becomes the drawing's `fonts` entry (saved in `.tdraw`) and serves its text while no installed font has the family, when its OS/2 embedding bits allow use in documents (not restricted, outlines allowed). See `docs/behavior/text.md`. |

## Writing

`crates/cdr/src/write.rs` writes the version 12 layout (`CDRC`, 32-bit
fields): `vrsn` 1200; `doc ` with `mcfg` (4 unknown bytes, width,
height, padded to 48 bytes), `fntt`/`font`, `filt`/`fild` (types 0, 1 and
the version 6 to 12 gradient body), `otlt`/`outl` (version 9 to 12 layout)
and `bmpt`/`bmp ` (24-bit, model 1); then one `page` per page with `flgs`
0x90, a `gobj` list and one `layr` per layer (`flgs` 0x98, the layer's
`loda` with its name, then the objects front to back). Each object is an
`obj ` list with its `loda` (type, coordinates, fill id, outline id,
name, transparency), a `trfd` with one type 8 matrix (translation in
coordinate units, page centre as origin) and, for text, a `txsm` in the
7 to version 15 layout with one style per run. Rectangles, ellipses and text
keep their own types (a rectangle only when one round radius describes
it: corner styles, sizes per corner and fixed corners on a stretched
rectangle are written as the curve they draw, since the corner order
and type byte are not confirmed); every other object is a curve; groups are `grp `
lists; a clip frame holding one bitmap is written as that bitmap with the
frame as its crop path (the way files store cropped bitmaps), other
clip frames become a group of the frame outline and the
contents; effects are evaluated first. A `stlt` with one complete style
(id 0) accompanies the font table and text objects reference it. The file reads back in this crate (round
trip tests in `write.rs`) and another public reader of the layout
rendered the vector content, fills, outlines and plain bitmaps of the
test files; text and transformed bitmaps are known not to show there.

## How to confirm an item on a file

1. Take a small `.cdr` file of your own holding one object and note its
   version (`cargo run -p tracedraw-cli -- info file.cdr`).
2. `cargo run -p tracedraw-cli -- inspect file.cdr` dumps the chunk tree.
3. Compare the parsed values with what the file is known to hold
   (position in mm, size, colour values) and move the row to "confirmed
   (file)" with the file name. Add the finding, its source and the date
   to `CLEANROOM.md`.
