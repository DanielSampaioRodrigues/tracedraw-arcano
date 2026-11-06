# Imposition assistant (Arcano edition)

File > Imposition... puts the open drawing's pages on press sheets, as
a new drawing, with a preview of the first sheet. The work is in
`apps/tracedraw/src/imposicao.rs`; it drives the imposition engine
(`imposition.rs`).

## Functions

The dialog asks what to do with the pages, each with a line saying what
it is for:

- Repeat each page: each page repeated over the sheet (cards,
  invitations, labels, bookmarks). Bleed and gap are typed; a gap of 0
  and no bleed put them side by side (what Labels did; old preferences
  set to Labels open as Repeat with gap and bleed 0).
- Pages in sequence: every page once, n-up (a deck of cards).
- Stapled booklet: one signature, saddle stitched, creep.
- Book signatures: perfect bound, signatures of n sheets (default 4).
- Tiling: each page cut into pieces that each fit a sheet, with an
  overlap (default 10 mm) along the joins, for posters and banners. The
  pieces split the page evenly, left to right and top to bottom, one
  sheet each, centred, with corner marks; the sheet is turned the way
  that needs fewer pieces. A page that fits takes one sheet. The summary
  says how many pieces across and down.

For Repeat and Sequence the columns and rows are as many as fit, or the
ones typed with Type them on (what does not fit runs off the sheet, and
the overflow warning says so).

## Sheets and values

- Sheets: A4, A3, SRA3 (320 × 450), 330 × 480, legal 216 × 330, or a
  custom size typed in millimetres (330 × 480 at first); the
  orientation that holds more is taken. Fitted to the pages is offered
  for booklets only.
- The left column keeps a steady height, so that the centred window
  does not jump when the function changes.
- Bleed 3, gap 4, margin 5 mm; marks on by default. For signatures the
  pages keep their size when the spread fits the sheet, and are fitted
  otherwise.
- The summary says how many per sheet and how many sheets; it says so
  when nothing fits.
- The choices are kept in the preferences (`imposicao`).
