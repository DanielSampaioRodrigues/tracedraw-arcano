# Clean-room record

This file records where TraceDraw's knowledge of each file format comes
from, so that anyone can check that the format support was written
independently. The details of each layout, and whether a file confirmed
it, live in `docs/cdr-format.md` and in the pages of `docs/behavior/`.

## Rules

- No program is ever decompiled or disassembled, and no decompiled code,
  disassembly or memory dump is used anywhere: not in the repository, not
  in issues, not in conversations with AI tools.
- No code is ported or copied from other implementations of a format,
  whatever their license. The documentation of open-source readers may be
  read to confirm a layout; the code that reads it here is written from
  that understanding and from the files themselves.
- Only files the contributor owns or is allowed to use are inspected.
  Personal files used for checking are not committed.
- Proprietary documentation is never copied into the repository.
- Every new piece of format knowledge gets a line in the log below:
  date, what was learned, and from which source.

## Sources

| Format | Sources |
|---|---|
| `.cdr` | Publicly available, unofficial format notes describing the RIFF chunks, the ZIP containers and the coordinate units; inspection of files owned by the author with `tracedraw-cli inspect` and a hex viewer; round trips of files TraceDraw writes, read back by TraceDraw and by another open-source reader. The libcdr documentation was consulted only to confirm layouts; none of its code (MPL 2.0) was ported. Embedded fonts: the W3C Member Submission "Embedded OpenType (EOT) File Format" (2008). |
| SVG | The W3C SVG 1.1 and SVG 2 specifications. |
| PDF, AI (PDF-compatible) | ISO 32000 (PDF 1.7 and 2.0) and the PDF/X standards. Password protection follows ISO 32000-2 section 7.6 (standard security handler, revisions 2 to 6); RC4 follows its published description and test vectors. Other PDF libraries and tools were used only to check files: as test oracles that open, protect or check them, never as code to port. |
| EPS, PostScript | The public PostScript language reference and the EPSF specification. |
| DXF | The publicly available DXF reference. |
| PSD | The publicly available PSD file format specification. |
| EMF, WMF | The published [MS-EMF] and [MS-WMF] open specifications. |
| PLT (HPGL) | Public HPGL command references. |
| DOCX, RTF, TXT | ECMA-376 (Office Open XML) and the RTF specification. |
| ICC profiles | The ICC.1 specification (v2 and v4). |

## Log

| Date | Finding | Source |
|---|---|---|
| 2026-10-08 | `.cdr` RIFF container, chunk walker, compressed lists (two zlib parts and a size table), coordinates in 1/254000 inch | public format notes |
| 2026-10-08 | Object, fill and outline records of the RIFF layout (rectangles, ellipses, curves, solid and gradient fills, outlines) | public format notes |
| 2026-10-09 | ZIP containers: `content/riffData.cdr` (versions 14 and 15), `content/root.dat` with `dataFileList.dat` and 16-byte redirect records (version 16 and later) | public format notes, cross-checked against the public format description |
| 2026-10-09 | Page origin at the page centre, objects stored front to back, bitmaps with a crop outline, the document palette in `color/docPalette.xml`, one byte per character in text from version 17 | a real banner file owned by the author (format version 21, not committed) |
| 2026-10-09 | Groups, text fitted to a path, paragraph spacing and indents, custom arrowhead outlines | public format notes; marked "assumed" in `docs/cdr-format.md` until a file confirms them |
| 2026-10-09 | Writing the version 12 RIFF layout (pages, layers, shapes, groups, text, bitmaps, fills, outlines, transparency, clipped bitmaps as crop paths) | the same notes; verified by round trip and by another open-source reader rendering the written files |
| 2026-10-11 | Version 21 paragraph text: object type 0x04 with an origin only, the frame being the object's `bbox` taken back through its transform and the `columns` of the object style string (`loda` argument 201, JSON); inline fills in run style strings (`primaryColor` as `CMYK255`/`CMYK` lists), Latin font settings, paragraph line spacing; the frame matrix in `txsm` frame records | a real banner file owned by the author (format version 21, not committed); measured with `tracedraw-cli inspect` and a hex dump |
| 2026-10-11 | Embedded fonts in version 16+ ZIP files: `embed/embedding<n>` members holding a header of their own and then an Embedded OpenType structure (sizes, flags, names, XOR-ed font data); version 21 text of object type 0x04 with the frame layout in `txsm` is artistic text whose `bbox` is the box of its letters (correcting the paragraph frame reading above) | the W3C Member Submission "Embedded OpenType (EOT) File Format" (2008) for the EOT layout; the author's version 21 banner file (not committed) for the member layout, checked by decoding its font and comparing the glyph outlines with the stored box; the decoded font was used locally only and is not committed |
| 2026-10-11 | Code pages of text before version 12: the low byte of the font and style encoding fields as a Windows character set value, mapped to its Windows code page | public format notes for the encoding fields; the published Windows character set values and code page tables (decoded with the `encoding_rs` library); not yet checked on a file |
| 2026-10-11 | PDF password protection: the standard security handler, revision 6 for writing (AES-256 file key, `/U` `/UE` `/O` `/OE` `/Perms`, algorithm 2.B hash, `/P` bits) and revisions 2 to 6 for reading (algorithms 1, 1.A, 2, 2.A, 4 to 6, crypt filters, object streams) | ISO 32000-2 section 7.6; checked against files protected and read by independent open-source tools |
| 2026-10-11 | Version 15+ rectangles (own size, the scale after it repeating the transform's; radii in inches; a single corner taking a whole side); polygons as a scaled base shape; page sizes and layer names in `lgob` lists; style strings (numbers as strings, alignment code 2 centring, weights as style codes, underline and strikeout, the object's style under the text's, uniform transparency); the CMYK profile under `color/profiles/cmyk/`; other palette colour models; pictures kept next to embedded fonts | public sample files in SDK example repositories on GitHub (Aspose.Imaging and GroupDocs examples, versions 11 to 22) and the LibreOffice test data, each compared with the preview it carries; files owned by the author (versions 20 and 21); all used locally, none committed |
| 2026-10-11 | Version 21 artistic text: line breaks as carriage returns; text fitted to a path keeps 40 bytes of path settings and 24 bytes per character (marked "assumed") | files owned by the author (version 21, not committed) |
| 2026-10-11 | Version 21 text fitted to a path: the meaning of the 40 path setting bytes (placement, offset along the baseline, distance to the left of the path, mirror flags), an ellipse's path starting at its top and running clockwise, the fitted text's `bbox` locating its path among the objects of its layer (marked "assumed") | a file owned by the author (version 21, not committed), compared with the preview it carries |
