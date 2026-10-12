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
fields): the font list (below), the size in points (a field and an arrow
dropping 6, 7, 8, 9, 10, 11, 12, 14, 16, 18, 20, 24, 28, 32, 36, 40, 48,
60, 72, 96, 144 and 288), Bold, Italic, Underline, the alignment (a
button with the alignment's picture dropping Left, Center, Right and
Full justify), Bullets and Drop Cap (for paragraph text; they open their
dialogs), Text Properties (the Text docker, Ctrl+T) and Edit Text
(Ctrl+Shift+T). Each change applies to the
selected text objects at once (every span takes the font, size and
style; the object takes the alignment) as one "Text Style" undo step,
and becomes the default for the next text. Double-clicking text with the
Pick tool edits it in place.

## Font list

The family is picked from a font list, the same in the property bar, the
Text and Glyphs dockers and Options (default font):

- A field with the family and an arrow. A click in the field selects the
  name, so typing replaces it, and opens the list; typing narrows the
  list to the families holding the letters typed, ignoring case: the name
  itself first, then names starting with them, then the rest. Down and Up
  move through the list, Enter takes the highlighted family, Escape
  closes the list and puts the name back.
- At the top of the list, the number of families shown and what to show:
  all fonts, installed fonts, the fonts in this drawing, or (in the
  browser) the fonts to download.
- "Used last": the last five families picked in any font list, newest
  first (kept in the preferences, `recent_fonts`), while nothing is typed.
- Every family, its name drawn in its own font (in the interface font for
  a symbol font and for a family not here yet), with an icon of how it is
  built: T TrueType, O OpenType (PostScript outlines), V variable, D came
  with the drawing, a downward arrow downloaded the first time it is used
  (the tooltip says which). The family in use is marked and brought into
  view. A family with more than one face has an arrow that shows its
  styles under it (Regular, Italic, Bold, Light ..., each drawn in itself);
  picking a style sets Bold and Italic as its weight (600 or more is bold)
  and slant say.
- The sample size: a slider from 12 to 40 points (18 at first, kept in
  the preferences, `font_sample_px`); the rows grow with it.
- The preview: "AaBbYyZz" in the family (and style) under the pointer,
  else the one the keys reached, else the family in use, between its top,
  baseline and bottom lines, with "Family (Style)" under it.
- In the browser, resting the pointer on a family to download for 0.4 s
  fetches it, and its row and preview then draw in it.

Names are drawn by the text engine into pictures of the screen's
resolution, only for the rows in view, and kept until fonts are added.

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

## Text docker

Ctrl+T, the property bar's Text Properties button and Window > Panels >
Text. Three sections, each under a header bar with a triangle; the
buttons at the top ("A", the pilcrow and a frame) show which are open
and open or close them, as a click on a header does. All come open; the
ones closed are kept in the preferences (`text_sections_closed`).

- Character: the font list, the size list (as on the property bar),
  bold, italic and underline; then, for the first selected text,
  tracking (-50 to 200 %), the characters' horizontal and vertical
  offsets and angle, strikethrough and the OpenType features
  (ligatures, small capitals, old-style numerals, fractions, swashes).
- Paragraph: left, centre, right and full justification as picture
  buttons; then line spacing (0 to 2000 %), character spacing (-100 to
  2000 %), word spacing (0 to 2000 %), space before and after
  paragraphs, first-line, left and right indents in the ruler unit.
- Frame, for paragraph text: columns (1 to 8) and the gutter between
  them, the vertical alignment (top, middle, bottom: the lines move down
  together by the room left under them in the frame, half of it for the
  middle; text that overflows starts at the top), Bullets and Drop Cap
  (their dialogs; pushed in while the text has them), Hyphenate and Fit
  Text to Frame. With artistic text the section says it is for
  paragraph text.
- The font, size, style and alignment become those of new text too; the
  other fields change the first selected text, one undo step each.
- Labels in the sections show without a trailing colon.

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
- Text on path: the baseline is the path's first subpath (followed from
  its end back to its start when mirrored), moved `distance` to its left,
  towards the letters' tops (corners mitred, at most 4 times the
  distance out). Each glyph is centred at arc length `start + s` along
  the baseline, where `s` is the glyph's x-advance midpoint and `start`
  is `offset`, plus `(length - width) / 2` for centred text or
  `length - width` for right-aligned text, and turned to the baseline's
  direction there. On a closed path positions wrap round past the start;
  on an open one they stop at its ends.
- Fit text to frame: scale size so the paragraph fills the frame height.

## Checks

- The Text docker: `ui::text_docker::tests::the_sections_open_close_and_draw_for_any_text`,
  `ui::text_docker::tests::vertical_alignment_is_kept_with_the_text`;
  `tests::paragraph_text_sits_at_the_top_middle_or_bottom_of_its_frame`
  (text crate). Checked on screen: Bottom moved "Hello world" to the
  frame's bottom edge.
- The font list: typing finds the name, then names starting with it,
  then the rest (`ui::font_list::tests::typing_finds_the_name_then_names_starting_with_it_then_the_rest`);
  the fonts used last keep the newest five
  (`ui::font_list::tests::the_fonts_used_last_keep_the_newest_first`);
  names are drawn in their own font, and a family not installed or no
  text draws nothing (`ui::font_list::tests::names_are_drawn_in_their_own_font`);
  the list draws with every filter, a family's styles shown and typed text
  (`ui::font_list::tests::the_list_draws_in_every_state`); families tell
  how they are built, their style names, whether they came with a drawing
  or are to download (`tests::families_tell_how_they_are_built_and_their_styles`
  in `crates/text`). Checked on screen: the list open, typing "lib" left
  three families, Down and Enter took one, and it showed first among the
  fonts used last the next time.
- `document::tests::text_baselines_wrap_round_closed_paths_and_move_by_the_distance`,
  `document::tests::open_text_baselines_stop_at_their_ends_and_corners_keep_the_distance`
  (core), `tests::mirrored_text_on_a_path_reads_along_its_other_side` and
  `tests::text_on_a_closed_path_continues_past_its_start` (text crate).
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
