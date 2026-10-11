# About box and start-up

## About TraceDraw

Help > About TraceDraw opens the About box from the Welcome Screen as well
as from a drawing. It is a chrome dialog (white title bar with the close
button) 480 points wide:

- A banner across the top: a diagonal gradient from `#244475` (top left)
  through `#1A3158` (top right) to `#0E192D` (bottom), faint light blue
  (`#5CAEFF`) curves on the right with one selected node (a square node,
  two handles along the tangent, round handle ends), the 76 px
  application icon, the name in the bold face at 29 px ("Trace" white,
  "Draw" `#7FC3FF`), the tagline and the version in a rounded chip.
- The description at 13.5 px with 20 px lines, a light separator, then
  the facts as two columns with dim labels: Author, License, Source code
  (a link that opens the repository), Developed with.
- The copyright line in small dim text and a Close button (Enter or Esc).

## Checks

- `ui::about::tests::the_embedded_icon_decodes`
- `ui::about::tests::cubic_helpers_follow_the_curve`
