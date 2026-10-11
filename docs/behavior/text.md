# Text

Artistic text: a single line (or several, with Enter) placed at a
baseline point; paragraph text: a frame with wrapping. Shortcut F8.

| Control | Default |
|---|---|
| Font | first sans-serif found (Arial, Liberation Sans, DejaVu Sans) |
| Size | 24 pt artistic, 12 pt paragraph |
| Leading | 100% of the font's line height (ascender - descender + line gap) |
| Paragraph spacing | before 0, after 0 |
| Kerning | font kerning on (GPOS kern, kern table) |
| Tracking (range kerning) | 0% of em |
| Baseline shift | 0 pt |
| Alignment | left; justify stretches spaces only, last line left |
| Hyphenation | off; when on, at least 2 letters before and 3 after a break, from a compact pattern set (a vowel and consonant rule for words it leaves whole); digraphs (ch, ck, gh, ph, sh, th, wh, qu) stay together; words with a capital letter (names, acronyms) are never broken |
| Columns | 1, gutter 5 mm |
| Bullets | off; indent 5 mm, bullet "*" from the paragraph font |
| Drop cap | off; 3 lines. The paragraph's first letter is scaled so it sits on the baseline of the last line it spans and its top meets the top of the first line's letters; the lines beside it are narrowed by its width plus 1 mm, so they wrap and justify inside the column |
| Tabs | every 12.7 mm (0.5 in) |
| Text on path | distance 0, offset 0, orientation "rotate letters" |

## Property bar

Text tool, and the Pick tool with only text selected (after the object
fields): font family, size in points, Bold, Italic, Underline, and the
alignment (Left, Center, Right, Justify). Each change applies to the
selected text objects at once (every span takes the font, size and
style; the object takes the alignment) as one "Text Style" undo step,
and becomes the default for the next text. Double-clicking text with the
Pick tool edits it in place.

## Editing in place

Clicking text with the Text tool (or double-clicking it with the Pick
tool) edits it where it is. The caret is a character position in the
concatenated spans; the layout's line boxes give its place on the page.

| Input | Behaviour |
|---|---|
| Typing, paste | inserts at the caret, replacing the selection, with the style of the character before the caret |
| Backspace / Delete | the selection, else the character before / after the caret |
| Enter / Shift+Enter | paragraph break / line break (U+2028) |
| Tab | a tab character |
| Left / Right, Ctrl+Left / Right | one character, one word; Shift extends the selection |
| Up / Down | the same x on the previous / next line (the x is remembered across moves) |
| Home / End, Ctrl+Home / End | line start / end, text start / end |
| Ctrl+A | select all |
| Ctrl+B / I / U | toggle bold, italic, underline on the selection, or on the whole text without one |
| Ctrl+C / X / V | copy, cut, paste the selected characters |
| Click / Shift+click / drag | place the caret / extend / select |
| Double-click | select the word |
| Esc | finish editing and return to the Pick tool; an empty text is removed |

The property bar's font, size and style buttons act on the selected
characters while a selection exists, and show the style at the caret. All
the keystrokes of one editing session are one "Edit Text" undo step.

## Paragraph text frames

- A paragraph text's bounds are its whole frame (with the letters, when
  they run over it): the selection handles sit on the frame, a click
  anywhere in it picks the text, and alignment and snapping use it.
- Frames show as dashed lines: grey, blue for the selected texts and the
  one being edited, red with a small tab holding a down arrow under the
  bottom edge's middle when the text does not fit (the layout says so;
  kept per document revision). The lines that do not fit are not drawn,
  printed or exported; they show again when the frame grows, and flow
  into the next frame of a linked chain. Text > Show Text Frames (on by default, kept between
  sessions) shows every frame; off, only the selected and edited ones
  show.
- Scaling a paragraph text with the Pick tool's handles, or with the
  property bar's size and scale fields, changes its frame and the text
  flows anew in it; the letters keep their size and the text's transform
  stays as it was (`text_frames.rs`). While the handle is dragged the
  frame follows and the letters stay. This holds for text that is
  neither turned nor slanted and a scale by positive amounts; turning,
  slanting, mirroring and moving act on the whole text, and a turned
  text scales as a whole. With other objects in the selection each is
  scaled as usual, all in one undo step.

## Character offsets, angle and spacing

- Each character can be shifted and turned: Horizontal character offset
  (percent of the font size, positive right), Vertical character offset
  (percent of the size, positive up; stored as the baseline shift in
  points) and Character angle (degrees, counter-clockwise, about the
  character's origin on its shifted baseline). The characters after a
  shifted one keep their place. The fields are in the Text panel's
  Character section and on the Shape tool's property bar; they act on
  the selected characters while editing, on the chosen character nodes
  with the Shape tool, else on every character of the selected texts.
- Paragraph: Character spacing (percent of the width of a space, -100 to
  2000, added after every character) and Word spacing (percent of the
  space's own width, 0 to 2000), besides line spacing.
- Text > Straighten Text puts the characters back (no offsets, no
  angle) and takes text fitted to a path off it; Text > Align to
  Baseline (Alt+F12) removes the vertical offsets only.

## Shape tool on text

- A node at the lower left of every character, on its (shifted)
  baseline; newlines have none. Click chooses one, Ctrl+click adds or
  removes, Shift+click adds, a marquee (rectangular or freehand) chooses
  the nodes inside. Chosen nodes are filled.
- Dragging a node moves the chosen characters (the one dragged when it
  was not chosen); Ctrl keeps the move horizontal or vertical. The move
  becomes their offsets (layout space, so rotated text moves along its
  own axes). One undo step per drag ("Move Characters").
- The Interactive horizontal spacing arrow, below the lower right
  corner, changes the character spacing so the end of the widest line
  follows the pointer: `+dx / n` per character for `n` characters on
  that line. With Shift it changes word spacing: `+dx / spaces` per
  space. The Interactive vertical spacing arrow, below the lower left
  corner, changes line spacing so the last baseline follows the pointer:
  `leading' = leading * (pitch - dy / (lines - 1)) / pitch`.
- Text fitted to a path shows no character nodes.

## Placeholder text

Text > Insert Placeholder Text fills each selected paragraph text frame
with filler text (the traditional pseudo-Latin passage, cycled), as much
as the frame holds without overflowing: the number of words is found by
doubling, then halving the gap, with a full layout each time (at most
20000 words). The filler takes the style of the frame's first
character, or the default font and size of an empty frame; paragraphs
end at the first full stop after 60 words and the text ends with a full
stop. Artistic text and other objects are left alone; a text being
edited is finished first. One undo step.

## Formulas

- Line height `= size * leading / 100 * (asc - desc + gap) / upm`.
- Justify: extra space `= (frame_width - line_width) / n_spaces`.
- Text on path: each glyph is placed at arc length `s + offset`
  where `s` is the glyph's x-advance midpoint, rotated to the tangent
  and raised by `distance` along the normal.
- Fit text to frame: scale size so the paragraph fills the frame height.

## Checks

- `text_frames::tests::scaling_paragraph_text_resizes_its_frame_and_keeps_the_letters`
- `text_frames::tests::frames_show_dashed_and_follow_a_scale_being_dragged`
- `text_frames::tests::a_frame_too_small_for_its_text_overflows`

- `text_nodes::tests::every_character_has_a_node_on_its_baseline`,
  `text_nodes::tests::dragging_chosen_characters_shifts_only_them`,
  `text_nodes::tests::a_marquee_chooses_character_nodes`,
  `text_nodes::tests::spacing_arrows_change_character_word_and_line_spacing`,
  `tests::shifted_and_rotated_characters_leave_the_others_in_place` and
  `tests::character_and_word_spacing_widen_the_text` (text crate).
- `tests::drop_cap_spans_its_lines_without_rising_above_the_text`,
  `hyphen::tests::breaks_fall_on_the_pattern_positions` and
  `hyphen::tests::names_and_acronyms_are_not_broken` (text crate).

- Given "ab" at 100 pt, then the width equals the sum of the two
  advances minus the kerning pair from the font.
- Given a paragraph frame narrower than one word, then the word breaks
  by character.
- Given a 40 x 20 mm and a 120 x 90 mm paragraph frame and an artistic
  text selected, when Insert Placeholder Text runs, then both frames hold
  filler that fits (the larger one more), eight more words would
  overflow the larger one, the artistic text is unchanged, and one undo
  brings the old text back.
