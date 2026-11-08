# IDML import

File > Open reads IDML packages: a document is a zip of XML parts, with
`designmap.xml` naming the stories and the spreads, `Stories/*.xml` the
text, `Spreads/*.xml` the page items and `Resources/Graphic.xml` the
colours. Positions in the files are points with y down from the page's
top left; the drawing keeps millimetres with y up from its bottom left
(`crates/io/src/idml.rs`).

## What it takes now

- The page size, from `DocumentPreference` in
  `Resources/Preferences.xml` (A4 when the part is missing).
- The items of the **first spread**: text frames and rectangles.
  - A text frame's story is read from `Stories/`, the paragraphs ending
    with `\r` in the file and with `\n` in the drawing; the story's last
    `\r` does not make an empty paragraph.
  - The font family comes from the first `CharacterStyleRange`
    (`AppliedFont`, with the style word taken off: `Minion Pro\tRegular`
    gives `Minion Pro`) and its `PointSize`; the frame's
    `TextColumnCount` gives the columns.
  - A rectangle takes its `ItemTransform` and `GeometricBounds` and the
    solid colour named in `FillColor` (`Resources/Graphic.xml`: CMYK, RGB,
    Gray and LAB).
- Named colours of the file, as the drawing keeps them.

## What is left aside, with a warning

Every item the reader does not know is named in a warning (logged, and
the drawing opens anyway):

- spreads after the first;
- groups, tables, linked pictures (`Image`, `PDF`);
- rotation and skew in `ItemTransform` (only its translation is used);
- paragraph and character styles beyond the first font and size, and the
  text's alignment (left);
- masters, and everything the drawing keeps beyond a page.

Checks: a package built in the test, with one page, a story of two
paragraphs in two columns and a filled rectangle, gives a page of A4, the
text and the rectangle (`idml::tests::a_package_gives_a_page_with_its_text_and_rectangle`);
a file that is not a package is an error, and one missing its spread
opens with a warning (`idml::tests::a_broken_package_is_an_error_not_a_panic`).

## Format notes

The package layout above comes from the IDML interchange format's public
description and from packages we build ourselves for the tests; the test
package is written by hand in `crates/io/src/idml.rs` and carries no
document of anyone else's.
