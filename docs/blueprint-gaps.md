# Open items

What is still open, by area. Rows move to `features.md` when done; this
file keeps only what is open.

## Toolbox (17 groups)

All 62 tools exist with their own icon, flyout position, property bar and
shortcut, and all of them are interactive (the Anchor Editing tool was the
last one; see `behavior/connectors-and-anchors.md`).

## Menu items

Every item of the twelve menus is wired, with these documented
exceptions, shown disabled so the layout stays complete:

| Menu | Item | Why |
|---|---|---|
| File | Acquire Image | no scanner (TWAIN/WIA/SANE) stack in pure Rust; import the scanned file |
| Object | Rollover | web-page interactivity with no HTML export target |
| Bitmaps | Plug-ins | no third-party filter host; the built-in effects cover the stock filters |
| Window | Cascade, Tile, Combine, Dock | document tabs by design (New Window opens a second window of the drawing) |

## Fills, colour and effects

- PostScript fills (legacy; vector pattern fills are done).

## File format

- `.cdr` generations: `WL` binary (v1, v2), RIFF (v3 to 13), ZIP with
  `content/riffData.cdr` (14, 15), ZIP with `content/root.dat` and
  `content/data/*.dat` ordered by `content/dataFileList.dat` (16 and
  later). TraceDraw reads all three container layouts (RIFF, the 14/15
  ZIP and the 16+ ZIP with redirected chunks);
  the `WL` binary of v1 and v2 is not read. Writing `.cdr` produces the
  version 12 RIFF layout (see `cdr-format.md`, "Writing").
- Coordinates: 1/1000 inch (16-bit versions, `V < 600`), 1/254000 inch
  otherwise.
- Compressed lists: two zlib parts; the second is a size table that the
  first one's chunk sizes index into.
- Read: pages, layers, rectangles, ellipses, curves, paths, polygons,
  bitmaps, artistic and paragraph text (`font`, `stlt`, both `txsm`
  layouts from version 7 on; family, size, bold, italic, underline,
  strike-through, run fill, frame size, alignment), uniform, gradient
  (with the version 16+ transformation), two-colour pattern (with the `bmpf`
  tile), colour bitmap and texture fills (the latter from the stored
  bitmap), outlines with dashes and arrowheads classified into our
  presets, object opacity.
- Done since: groups (`grp ` lists, nested), text fitted to a path
  (from the frame flag and the sibling curve), line spacing, character
  spacing and indents from `stlt`, custom arrowhead outlines kept as
  custom arrowheads, version 21 inline run colours, embedded fonts (EOT
  members of the ZIP container), the code pages of text before
  version 12, and the `arrx` arrowheads of newer files (one version 21
  sample).
- Still open in the reader: splines (0x26, layout not public), vector
  pattern (10) and PostScript (6) fills, text before version 7, tabs,
  bullets and drop caps from `stlt` (tables skipped by size), the `ftil`
  fill transform, lenses and other effects, fonts embedded with
  MicroType Express compression. Layouts marked "assumed" in
  `cdr-format.md` (`bmpf`, `arrw`, the version 16+ style string content, the
  opacity direction, the alignment codes) have no file behind them yet.
- Chunks with public descriptions: `vrsn`, `DISP`, `LIST cmpr`, `stlt`,
  `font`, `txsm`, `mcfg`, `loda`, `trfd`, `fild`, `outl`, `bmp `, `sumi`.

## Formats to add

Import: DWG (proprietary), CGM; CMX lenses and bitmap fills (CMX import
otherwise done, from sample files: there is no public specification).
Export: CMX.
Done: SVG, SVGZ, PDF, AI (PDF-compatible), EPS/PostScript, DXF, PSD,
EMF, WMF, PLT, TXT, RTF and DOCX import; SVG, PDF, AI (PDF-compatible),
EPS, DXF, EMF, WMF, PLT, PSD, HTML, PDF/X-1a, X-3, X-4, `.cdr` (version
12 layout) and the bitmap formats on export.

## Automation

The JavaScript object model covers documents, pages, layers, shapes,
fills, outlines, colours, `Curve` > `SubPaths` > `Nodes` (position,
type, move, delete, add) and `Effects` (list, add any effect by its
`.tdraw` form, `AddExtrude`, `AddBlockShadow`, remove, clear), and
import/export filters (`behavior/script-filters.md`). Open: a script
editor with breakpoints.
