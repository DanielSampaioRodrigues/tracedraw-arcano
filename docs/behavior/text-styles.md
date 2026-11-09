# Text styles

Named paragraph and character styles, kept with the drawing
(`Document::text_styles`, saved only when there are some). The model is in
`crates/core/src/text_styles.rs`; the Text Styles panel (Window > Panels)
in `apps/tracedraw/src/estilos.rs`.

## The model

- A style has an id, a name, a kind (paragraph or character), an optional
  parent (`based_on`, same kind; a loop is never offered), a paragraph
  format (paragraph styles; `None` takes the parent's) and the character
  properties it sets (`CharFormat`: font, size, bold, italic, underline,
  strikethrough, the look of those two lines, tracking, baseline shift,
  colour, OpenType features; each one `None` when not set), the style the
  paragraph after it takes (`next`) and a keyboard shortcut
  (`shortcut`).
- What a style sets is its own settings over its parents', from the root
  down (`resolve`).
- A paragraph is linked through its paragraph format
  (`ParagraphFormat::style`); characters through `TextSpan::char_style`.

## Applying

- A paragraph style goes to every paragraph the characters touch (all of
  them for a whole text object): the paragraphs take its format and link,
  their characters its character properties, and characters linked to a
  character style take that style again on top.
- A character style goes to the characters given (the selection while a
  text is edited; a whole selected text object). With only a caret, the
  status bar asks for a selection.
- No style takes the link off: a paragraph keeps its look; characters give
  back what their character style set, taken from the paragraph's plain
  characters (the first ones without a character style) with the
  paragraph style on top.
- Each application is one undo step for every text it changes.

## Changing styles

`Command::SetTextStyles` replaces the list (free ids for id 0) and applies
the styles again to every text linked to them, on every page, the master
page, in groups and clip frames. What a style sets wins over formatting
given by hand; what it does not set stays. A link to a style that is gone
is dropped and the text keeps its look. Deleting a style in the panel gives
the styles based on it what it set and its parent.

## Next style

A paragraph style names the style of the paragraph after it (\`next\`):
when a paragraph ends with Enter, the paragraph it opens takes it. The
book set uses it: Chapter title gives a First paragraph, a First
paragraph gives Body text, a Subtitle or a Quotation gives Body text
back. A style that names nothing, itself, a character style or a style
that is gone keeps its own, so the paragraphs of a body of text stay in
the body style.

- The next paragraph is that style from its start: what is typed there
  comes out in its look, size and outline included. The empty paragraph
  after a break carries the style on its newline, so it is already there
  before the first character.
- Only Enter breaks a paragraph this way: Shift+Enter writes a line break
  inside the paragraph.
- Opening a file keeps the paragraph formats it has: the next style is
  not applied to what is already written.
- Checked by \`text_styles::tests::a_style_names_the_style_that_follows_it\`,
  \`text_styles::tests::an_empty_last_paragraph_takes_the_look_on_its_newline\`
  and \`estilos::tests::the_next_style_follows_a_paragraph_break\`.

## Shortcuts

A style can carry a keyboard shortcut (\`shortcut\`, written as the other
shortcuts are: "Ctrl+3"). A press of it applies the style to what the
panel would apply it to, the text being edited or the selected text
objects.

- A key without Ctrl or Alt is refused, so the typing keys stay free; the
  field says so and the shortcut is not kept.
- A shortcut the user gives a command (Options > Customization >
  Commands) is read first, as built-in shortcuts are.
- While a text is being typed the keys go to the text, as always: a style
  shortcut acts when a text object is selected.
- Checked by \`text_styles::tests::the_next_style_and_the_shortcut_are_saved\`
  and \`estilos::tests::a_style_shortcut_finds_the_style\`.

## Reset to base and the summary

Reset to base takes off everything the style sets, its paragraph format
and its character properties: it keeps its name, the style it is based
on, its next style and its shortcut, takes everything from the base, and
every text linked to it takes the base's look.

Under the settings the panel shows a summary of the style as the text on
the page looks: the characters (font, size, bold, italic, underlined,
struck through, colour), the paragraph (alignment, first line indent),
the look of the underline and of the strike line when they have one of
their own, and the style that follows it.

- Checked by
  \`text_styles::tests::reset_to_base_leaves_the_name_the_parent_and_the_next\`,
  \`estilos::tests::reset_to_base_gives_the_text_the_look_of_the_base\` and
  \`estilos::tests::the_summary_says_what_the_style_sets\`.

## Making styles from text

- New (paragraph): the format of the paragraph at the caret and all the
  properties of its first character.
- New (character): what the character at the caret has that the
  paragraph's plain look lacks (everything, when nothing differs).
- Redefine: the same from the text at the caret, keeping the name and the
  parent; with a parent, only what differs from it stays the style's own.
- Add Book Styles (an empty list only): Body text (justified, 5 mm first
  line indent, hyphenated, 11 pt), First paragraph (no indent), Chapter
  title (centred, 20 pt bold, 20 mm before, 12 mm after), Subtitle (centred,
  14 pt italic), Quotation (10 mm each side, 10 pt) based on Body text, and
  the character styles Emphasis (italic), Bold and Small caps (`smcp`). They
  set no font.

## The panel

Two lists, paragraph and character styles, each with New and [No style]
first. A row applies its style on a click, is marked when it is the style
at the caret, shows its parent after « and how many paragraphs (or
runs of characters) use it. Its + opens the name (kept when the
field is left), Based on, the next style and the shortcut (paragraph
styles), Redefine, Clear Overrides (apply again), Reset to base, Delete,
and a summary of what the style sets.

Tests: `text_styles::tests` (core), `estilos::tests` (the panel keeps its
width, the book set and a change reach the text, one undo step).
