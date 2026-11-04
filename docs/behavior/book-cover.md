# Book cover (Arcano edition)

File > New Book Cover... lays out a book's open cover as a new drawing.
The work is in `apps/tracedraw/src/capa.rs`; the values are the
editora's own (its spine worksheet), kept in the preferences
(`capa`), and editable in the dialog's paper table.

## The spine

- One leaf is `grammage × volume / 1000` mm thick; a book of `p` pages
  has `ceil(p / 2)` leaves (an odd count warns: books have even pages).
- Paperback: spine = text block + 2 × cover card + adjustment.
- Hardcover: spine = text block + 2 × board + play (default 1 mm).
- Papers shipped: soft 70/80 (volume 1.30), natural 70/80 (1.25), offset
  75/90 (1.15), bold 70/90 (1.80). Cards 250/300/350 g (0.31, 0.37,
  0.43 mm). Boards 1.9, 2.5, 3.0 mm. The table adds, removes and resets
  them.
- A spine under 3 mm warns: too thin for spine text.

## The page

- Paperback, left to right: flap (optional), back, spine, front, flap.
- Hardcover: turn-in, back (width + overhang), hinge, spine, hinge,
  front, turn-in; height = trim height + 2 × overhang + 2 × turn-in.
  Defaults: overhang 3, hinge 7, turn-in 15 mm.
- The page is the open cover; the document's bleed (default 3 mm) is
  set. Guides stand on every fold and on the safety margin (default
  5 mm) inside back and front; horizontal guides on the turn-ins and
  the margin.
- The first layer is renamed Art. A layer Guides, locked and not
  printed, names each panel and its width in 7 pt grey text.
- Units become millimetres. The last values are kept for the next
  cover.
