# Imposition assistant (Arcano edition)

File > Imposition... puts the open drawing's pages on press sheets, as
a new drawing, with a preview of the first sheet. The work is in
`apps/tracedraw/src/imposicao.rs`; it drives the imposition engine
(`imposition.rs`).

## Products

- Book signatures: perfect bound, signatures of n sheets (default 4),
  creep.
- Saddle stitched (booklets, notebooks): one signature.
- Cards and invitations: the first page repeated over the sheet.
- Deck of cards: every page once, n-up.
- Labels and bookmarks: the first page repeated, gap between them.

## Sheets and values

- Sheets: A4, A3, SRA3 (320 × 450), 330 × 480, legal 216 × 330; the
  orientation that holds more is taken.
- Bleed 3, gap 4, margin 5 mm; marks on by default. For signatures the
  pages keep their size when the spread fits the sheet, and are fitted
  otherwise.
- The summary says how many per sheet and how many sheets; it says so
  when nothing fits.
- The choices are kept in the preferences (`imposicao`).
