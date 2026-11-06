# House press (Arcano edition)

Window > Panels > House Press: buttons that check and fix a drawing
without any AI service. The work is in `apps/tracedraw/src/prelo_casa.rs`.
With a selection the jobs work on the selected texts (in groups and clip
frames too), else on every text of the drawing. Check counts; Fix changes,
one undo step for all.

- Typography (`typography`): `"` becomes an opening or closing curly
  quote by what is before it (start, a space or an opening bracket or
  dash: opening); `'` an apostrophe inside a word, else an opening or
  closing single quote; `...` the ellipsis; `--` and a hyphen between
  spaces the em dash; a hyphen between digits the en dash; a second space
  goes; a space before `, . ; : ! ? )` goes. Each span keeps its
  formatting; the character before a span is the last of the span before.
- Local formatting (`local_overrides`): a run of characters in a
  paragraph with a paragraph style, without a character style, whose look
  differs from the style where the style sets it. Fix applies the styles
  again (`text_styles::refresh_text`).
- House standard (`HousePrefs`: top 20, bottom 25, inner 22, outer 18,
  bleed 3, safety 5 mm by default), kept in
  `Biblioteca TraceDraw/padrao-da-casa.json` when a house library is
  chosen, else in the preferences. Apply adds the book styles when there
  is no Body text style, sets the bleed and replaces the guides it put
  before (marked by their colour) with guides at the margins (inner at
  the left of pages 1, 3, 5...) and at the safety distance from the page's
  edges.

Tests: `prelo_casa::tests`.

## Breaks and justification

A check with a list (`apps/tracedraw/src/prelo_quebras.rs`): with a
selection the selected paragraph frames, else every paragraph text on the
pages (master layers aside). It looks at the lines as they are laid out,
frame by frame and, in a frame of several columns, column by column (a
line higher than the one before starts a column). A frame goes on from the
one before when that one's text does not end a paragraph, and into the next
when it is linked and its own does not.

| Case | When |
|---|---|
| Orphan | a paragraph's first line is the last of a frame or column and the paragraph goes on |
| Widow | a paragraph's last line is the first of a frame or column and the paragraph began before |
| Word alone | the last line of a paragraph of more lines holds one word |
| Hyphens in a row | more than 2 lines in a row end inside a word |
| Hyphen at the foot | the last line of a frame or column ends inside a word |
| Title at the foot | a paragraph kept with the next ends a frame or column with the next paragraph elsewhere |
| Loose line | a justified line (not a paragraph's last) whose spaces are more than 1.33 times as wide as on the same line set flush left |
| Tight line | the same, less than 0.80 times |

The list names each case, its page and the start of its line; a click goes
to the page and edits the text with the line selected. Nothing changes.
Loose lines are common in narrow justified columns while justification
only widens word spaces.

## Openings and front matter

A checklist, each line ticked or crossed; a click goes to its page:

- at least four pages; page 1 (half title) has text; page 2 is blank
  (nothing printable on its layers); page 3 (title page) and page 4
  (credits) have text; an even number of pages;
- every paragraph of the Chapter title paragraph style (the book set's)
  is on a right-hand page (1, 3, 5...), and the next paragraph with
  words has a drop cap. Without that style the chapters are not checked
  and the list says so.

Tests: `prelo_quebras::tests`.

