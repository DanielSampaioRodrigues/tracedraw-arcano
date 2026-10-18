# Printing

File > Print (Ctrl+P) builds a print-ready PDF of the sheets and opens it
in the system viewer, where the printer is chosen (in the browser the PDF
is downloaded). `export::print` builds it; the sheets come from
`imposition.rs`.

## Print dialog

Two columns: the pages and the prepress options on the left, the layout
on the right; under them the preflight outcome (see `preflight.md`) and
Print, Print Preview and Cancel. The dialog starts with the choices of
Options > Global > Printing, which can also keep those of the last print
and turn the preflight outcome off (see `options.md`).

- Copies (handed to the viewer), All pages (off: the current page only),
  Fit to page (off: a scale in percent for fitted sheets).
- Prepress: Print separations (one grey page per ink, C, M, Y and K, for
  every sheet), Bleed (the document's bleed around every page), Crop
  marks, Registration marks, Color bars, Densitometer scale, Page
  information, Mirror (the whole sheet, marks included, flipped for
  emulsion-down film), Invert (every colour of the sheet, for negatives).
- Layout:
  - Imposition: As in document (one page per sheet), Several pages per
    sheet (a grid of columns x rows, 2 x 2 by default, filled left to
    right and top to bottom), Booklet (two pages per sheet side in
    saddle-stitch order), Step and repeat (each page repeated over the
    grid, for cards and labels).
  - Sheet: Fit to pages (the sheet takes the size of the pages, the gaps
    and the margins) or A5, A4, A3, SRA3, A2, Letter, Legal, Tabloid,
    portrait or Landscape. On a fixed sheet the pages shrink to fit (never
    grow) and the grid is centred.
  - Gap between pages (5 mm by default) for the grid layouts.
  - A sketch of the first sheet (the pages numbered, blank booklet pages
    grey) and the number of sheets.

## Print Preview

File > Print Preview, or Print Preview in the Print dialog (which hands
over its settings), fills the window with the sheets exactly as Print
sends them: the imposition, the marks, Mirror and Invert, built by the
same function (`export::print_sheets`).

- Top bar: Print (prints these settings and closes), Close (Esc), the
  sheet shown with previous and next (Page Up, Page Down), the Plate
  (Composite, then cyan, magenta, yellow, black and each spot ink) while
  Print separations is on, and Zoom from 25 to 400 % of the fitted size
  with Fit.
- Left: the Print dialog's options (all pages, fit, scale, prepress and
  layout with its sketch); every change rebuilds the sheets at once.
- A plate shows the ink's coverage in grey, black = full ink, as the
  separations print; registration colour shows on every plate.

## Imposition

- Pages keep their aspect ratio; each cell is as large as the largest
  page on its sheet and a smaller page sits in its middle.
- A booklet's page count is rounded up to a multiple of four with blank
  pages at the back. The sheet sides pair the pages as a folded booklet
  needs: for eight pages 8 and 1, 2 and 7, 6 and 3, 4 and 5 (left and
  right). Its two pages meet at the spine with no gap and no bleed between
  them.
- Every page is clipped to its trim box plus the bleed, so nothing spills
  onto its neighbours or the margin. Hidden and non-printing layers are
  left out; master layers print with every page they show on; a page
  background prints under its page.

## Marks

With any mark on, a 12 mm margin (plus the bleed) surrounds the pages.

- Crop marks: 5 mm lines in registration colour, 0.1 mm wide, starting
  1.5 mm beyond the bleed, at every cut line of the grid, outside the
  pages only. A booklet also gets a dashed fold mark at the spine.
- Registration marks: a target (circle and cross) centred on each side
  of the pages.
- Color bars: patches of 4 mm of cyan, magenta, yellow, black, red,
  green, blue (two-ink overprints) and 50 % tints of the four inks, left
  of the top target.
- Densitometer scale: black from 0 to 100 % in steps of ten, right of the
  top target.
- Page information: the drawing's name, the pages on the sheet, "sheet n
  of n" and the date and time, in 6 pt under the pages.

## Checks

- `ui::print_preview::tests::plates_split_the_sheet_by_ink`
- `ui::print_preview::tests::the_settings_key_ignores_copies_and_preflight`

- `imposition::tests::booklets_pair_the_pages_for_folding`
- `imposition::tests::n_up_fills_sheets_in_reading_order_and_scales_to_fit`
- `imposition::tests::fitted_sheets_grow_with_marks_bleed_and_gutters`
- `imposition::tests::imposing_places_and_clips_every_page`
- `export::tests::the_print_dialog_settings_make_the_layout`
- `export::tests::colours_are_mapped_inside_groups_frames_and_text`: the
  separations and Invert reach into groups, clip frames and character
  fills.
