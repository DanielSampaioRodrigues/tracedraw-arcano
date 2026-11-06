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
