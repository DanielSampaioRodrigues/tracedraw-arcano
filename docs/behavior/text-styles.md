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
  strikethrough, tracking, baseline shift, colour, OpenType features; each
  one `None` when not set).
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
field is left), Based on, Redefine, Clear Overrides (apply again) and
Delete.

Tests: `text_styles::tests` (core), `estilos::tests` (the panel keeps its
width, the book set and a change reach the text, one undo step).
