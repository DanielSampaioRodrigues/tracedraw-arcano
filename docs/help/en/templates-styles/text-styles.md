# Text styles

A **text style** is a named set of text formatting that stays linked to the
text that uses it: change the style once and every paragraph in the drawing
that uses it changes. Open the [[docker.text_styles]] panel from
[[menu.window > menu.window.dockers]].

There are two kinds:

- **Paragraph styles** set a whole paragraph: alignment, line and paragraph
  spacing, indents, tabs, hyphenation, and the characters' font, size and
  weight.
- **Character styles** set only some characters: italic for a book title,
  bold, small caps. They sit on top of the paragraph style.

## Applying a style {#apply}

Edit a text (or select text objects) and click a style in the panel:

- a paragraph style goes to the paragraphs the selection touches (the
  paragraph at the caret when nothing is selected);
- a character style goes to the selected characters.

The style at the caret is marked. [[tstyle.none]] takes the link off and
leaves the text as it looks.

## Making and changing styles {#edit}

- [[tstyle.new]] makes a style with the look of the text at the caret and
  applies it.
- [[tstyle.book_set]] adds a starting set for books: body text, first
  paragraph, chapter title, subtitle, quotation, emphasis, bold and small
  caps. They set no font, so the text keeps its own.
- The arrow beside a style opens its settings: its name, the style it is
  **based on** (what it does not set, it takes from that one and follows its
  changes), [[tstyle.redefine]] (the style takes the look of the text at the
  caret), [[tstyle.clear]] (apply it again, taking off changes made by hand)
  and [[tstyle.delete]].

What a style sets wins over formatting given by hand when the style changes;
what it does not set stays as you made it. Each change is one undo step, and
the styles are saved with the drawing.
