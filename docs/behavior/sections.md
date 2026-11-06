# Sections

Page > Sections (`apps/tracedraw/src/secoes.rs`; the model in
`crates/core/src/document.rs`, `Section`, `Document::sections`,
`Document::field_text`).

- A section has its first page, a numbering style (arabic, upper or lower
  roman, upper or lower letters), the number of its first page and a
  title. It goes on until the next section's first page; pages before the
  first section number as without sections.
- Page number fields drawn on a page (`layers_drawn_for_page`) show
  `field_text`: with a section, the page's place in it from the section's
  start, in its style, with the field's prefix and suffix; a field whose
  `show` is 1 shows the section's title (a running head).
- Page > Insert Running Head inserts such a field at the top centre: on
  the active master layer (one field for every page), else a text on each
  page with that page's section title written in.
- The window: a row per section (first page, numbering, start at, title,
  remove), Add at the Active Page, Book Numbering (lower roman from page
  1, arabic from the active page) and Insert Running Head. Each change is
  one undo step (`Command::SetSections`).

Tests: `secoes::tests`.
