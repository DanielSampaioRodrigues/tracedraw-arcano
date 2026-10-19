# Bitmap import

File > Import (Ctrl+I) of a PNG, JPEG, BMP, GIF, TIFF or WebP file
embeds the image as a bitmap object centred on the page and remembers
the file (Bitmaps > Update from Link reads it again).

## Size

The bitmap comes in at the resolution its file records, so a 300 dpi
scan 1200 pixels wide is 101.6 mm wide:

| Format | Where the resolution comes from |
|---|---|
| PNG | the `pHYs` chunk, when its unit is the metre (pixels per metre x 0.0254) |
| JPEG | the JFIF header's density in dots per inch or per centimetre; else the EXIF `XResolution`/`YResolution` with their `ResolutionUnit` |
| TIFF | `XResolution`/`YResolution` with `ResolutionUnit` (inch or centimetre) |
| BMP | the header's pixels per metre |

Width in mm = pixels x 25.4 / horizontal dpi, height likewise with the
vertical dpi. A file that records no resolution, only an aspect ratio,
or a value outside 1 to 100000 dpi comes in at 96 dpi. Pasted images
and generated pictures are placed at 96 dpi.

## Checks

- Each reader gives the stored value and nothing for missing, broken or
  out-of-range data (`image_dpi::tests`).
- Given a PNG of 4 pixels at 300 dpi, when imported, then it is
  4 x 25.4 / 300 mm wide; a PNG without `pHYs` is 4 x 25.4 / 96 mm wide
  (`image_dpi::tests::import_places_a_bitmap_at_its_resolution`).
