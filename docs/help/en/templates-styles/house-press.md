# House press

The [[docker.house_press]] panel checks and fixes a drawing with buttons,
without any AI service. Open it from [[menu.window > menu.window.dockers]].
With objects selected it works on them; otherwise on the whole drawing.

Each job has [[prelo.check]] (counts what would change, changing nothing)
and [[prelo.fix]] (makes the changes; one undo step takes them all back).

## Typography {#typography}

[[prelo.typography]]: curly quotes and apostrophes, the dash between words
and the en dash between numbers (pages 10–20), the ellipsis, double spaces
and spaces before punctuation. The characters keep their formatting.

## Local formatting {#local}

[[prelo.local]]: characters whose look was changed by hand away from their
paragraph style, such as a size or bold given to a few words. Fix gives
them the style back; characters with a character style keep it.

## House standard {#standard}

[[prelo.standard]]: the house's top, bottom, inner and outer margins, bleed
and safety distance. When a house library folder is chosen they are kept
there, so every computer of the house uses the same values.
[[prelo.apply_standard]] adds the book text styles when missing, sets the
bleed and puts guides at the margins and the safety distance on every page.
The inner margin is at the spine (at the left of right-hand pages, the
first page being one). Applying again replaces the house's guides and
leaves your own.
