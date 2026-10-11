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

## Fonts that come with a drawing

A drawing can carry font files: the fonts embedded in a `.cdr` file it
was opened from (see `docs/cdr-format.md`, Embedded fonts), saved with it
in `.tdraw` files. When such a drawing opens (or a `.cdr` file is
imported), each carried font whose family is not installed serves the
text for the rest of the session and joins the font lists; an installed
font of the same family wins, since an embedded copy may hold only the
letters the drawing used. A font is used only when its licence allows
use in documents (the OS/2 embedding bits: not "restricted", outlines
allowed); others are skipped with a log line and their text falls back
like any missing font. The Fonts docker marks carried families
"embedded" under "Fonts in document". Saving a drawing as `.tdraw` adds
the carried fonts its text uses (text imported or pasted from another
drawing), so the file keeps its letters on machines without them.

The missing-font check (status bar, Font Manager, preflight) looks at
text everywhere in the drawing: on layers, in groups and clip frames,
in table cells and in symbols; a family is missing when it is neither
installed nor carried.

## Family names

A font is found by the family name its file lists (the typographic
family, "Arial") and by its legacy family name when that differs
("Arial Narrow", "Arial Black", "Segoe UI Semibold", the name a drawing
from another program usually gives); the font lists show both. A
legacy name picks the face of that name nearest the text's weight and
style.

A drawing that carries several faces of one family (regular and bold)
has each face serve its text; an installed face of the same family,
weight and style wins over the carried one.

## Missing fonts

Text set in a family that is neither installed nor carried keeps its
font name (saving writes it back unchanged) and is drawn with a stand-in,
the first installed of:

1. Fonts drawn to the same widths, so lines break where they did:
   Liberation Sans, Arimo, Nimbus Sans or TeX Gyre Heros for Arial and
   Helvetica; Liberation Serif, Tinos, Nimbus Roman or TeX Gyre Termes
   for Times and Times New Roman; Liberation Mono, Cousine, Nimbus Mono
   or TeX Gyre Cursor for Courier and Courier New; Liberation Sans
   Narrow for Arial Narrow; Carlito for Calibri; Caladea for Cambria;
   Gelasio for Georgia; Selawik for Segoe UI; TeX Gyre Pagella for
   Palatino and Book Antiqua, Bonum for Bookman, Schola for Century
   Schoolbook, Adventor for Century Gothic and Avant Garde, Chorus for
   Zapf Chancery (or their URW counterparts); DejaVu Sans for Verdana
   and Tahoma; Comic Neue for Comic Sans; EB Garamond for Garamond;
   DejaVu Sans Mono for the programmers' faces.
2. Fonts of the same style, judged by the name: monospaced ("mono",
   "courier", "code", ...), serif ("serif", "roman", "garamond",
   "bodoni", ... and not "sans" or "gothic"), casual ("comic",
   "marker", ...), else sans-serif.
3. DejaVu Sans, Arial, the system sans-serif, any font.

Within a run, letters the face lacks are drawn with a face that has
them: the fonts made for their script first (Noto Sans CJK, WenQuanYi
and the system faces for Chinese, Japanese and Korean; Noto Sans Arabic,
Hebrew, Thai, Devanagari and Bengali; DejaVu Sans and Noto Sans for the
rest), then any installed face that has the letter, upright faces of the
run's weight first. Spaces, controls and combining marks stay with the
letter before them. A letter no face has is drawn with the run's face.

Variable fonts draw at weight 400, or 700 for bold, on their weight
axis (within its range), and slant on their italic or slant axis for
italic when the face is upright; only what no axis gives is made
synthetically (bold by drawing the letters twice, italic by slanting
them).

The browser build offers the families of its font folder (see
`web/FONTS.md`): the font lists show them, and text asks for one the
first time it needs it (text set in it, in a family it stands in for,
or holding letters of a script only it has); the page fetches the files
and draws again when they arrive.

Names are compared in lower case without spaces, punctuation, style
words and foundry endings ("Arial-BoldMT" and "Arial MT" are Arial,
"TimesNewRomanPSMT" is Times New Roman), so names written by PostScript
and PDF producers match too. The status bar, the message on opening, the
Font Manager and the Fonts docker (on the "missing" mark) name the
stand-in next to each missing family: "Arial (Liberation Sans)".

## Formulas

- Line height `= size * leading / 100 * (asc - desc + gap) / upm`.
- Justify: extra space `= (frame_width - line_width) / n_spaces`.
- Text on path: each glyph is placed at arc length `s + offset`
  where `s` is the glyph's x-advance midpoint, rotated to the tangent
  and raised by `distance` along the normal.
- Fit text to frame: scale size so the paragraph fills the frame height.

## Checks

- `tests::fonts_are_found_by_their_legacy_family_names`,
  `tests::a_drawing_brings_every_face_of_a_family`,
- `tests::missing_fonts_get_stand_ins_with_the_same_widths_or_style`,
  `tests::letters_a_font_lacks_are_drawn_with_a_font_that_has_them`,
  `tests::variable_fonts_draw_bold_with_their_weight_axis` and
  `tests::fonts_the_application_offers_are_asked_for_once_and_then_draw`
  (text crate), `documents::tests::fonts_that_arrive_later_refresh_the_lists`,
  `font_folder::tests::the_font_list_reads_families_and_keeps_files_in_the_folder`
  and
  `prefs::tests::opening_tells_what_was_left_out_as_the_warnings_say`.
- `documents::tests::a_drawing_brings_its_fonts_along`,
  `tests::fonts_a_document_brings_are_loaded_once_unless_installed` and
  `tests::a_font_licence_decides_whether_a_document_may_use_it` (text
  crate), `container::tests::embedded_fonts_come_with_the_document` (cdr
  crate) and `document::tests::carried_fonts_are_saved_and_shared_between_copies`
  (core).
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
