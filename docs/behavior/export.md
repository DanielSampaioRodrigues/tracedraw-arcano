# Export

File > Export (Ctrl+E) and File > Export For > Web or Office open the
Export dialog.

## Layout

- On the left, a 260 x 300 point preview of what will be written: the
  page, or the selection with Selected only, rendered to fit, on a
  checkerboard when the format keeps transparency and Transparent
  background is on, else on white.
- On the right: Format, then the format's own settings, then Selected
  only and All pages, then the estimated file size.

| Format | Settings |
|---|---|
| PNG, WebP, GIF | resolution (36 to 2400 dpi, presets 72, 96, 150, 300), transparent background; WebP also quality |
| JPEG | resolution, quality (1 to 100) |
| BMP, TIFF, PSD | resolution |
| PDF | standard (PDF 1.4, PDF/X-1a, PDF/X-3, PDF/X-4), bleed, output condition for PDF/X |
| SVG, EPS, AI, DXF, HTML, EMF, WMF, PLT | none |

- Bitmap formats show the size in pixels: the page (or selection) size
  over 25.4 times the resolution.
- File size: vector formats (SVG, EPS, PDF, AI) are written in memory and
  measured; bitmap formats are estimated by encoding the preview the same
  way and scaling by the pixel count of the export (times the pages with
  All pages). It shows as "about 820 KB" or "about 3.4 MB".
- The preview and estimate are worked out again only when the drawing,
  the page, the selection or a setting changes.
- OK asks where to save; the file name is the drawing's with the format's
  extension.

## Checks

- `export::tests::the_export_preview_shows_the_page_and_estimates_the_size`
- `export::tests::sizes_read_in_kilobytes_and_megabytes`
