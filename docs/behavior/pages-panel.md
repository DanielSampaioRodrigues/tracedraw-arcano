# Pages panel and master pages (Arcano edition)

The Pages panel (`apps/tracedraw/src/ui/pages_panel.rs`) works as a layout
program's: master pages on top, the drawing's pages below as miniatures.
The masters themselves are `MasterPage` records in the document
(`crates/core/src/document.rs`), the page operations are in
`apps/tracedraw/src/masters.rs` and the miniatures in
`apps/tracedraw/src/page_thumbs.rs`.

## Master pages

- Every drawing has A ("A-Master"); other masters are made from the
  panel's menu (New Master...: prefix, name, based on another master) or
  from a page (Save as Master: the page's objects copied onto the new
  master's layer). A master's objects are on the master layers whose
  `master_page` is its id; master layers scoped to odd or even pages are
  its right-hand and left-hand pages.
- A page shows the layers of its master and of the masters that one is
  based on, the parents' behind (`layers_for_page`); [None] leaves a page
  with no master objects. A master based on itself, directly or through
  others, is refused.
- Master Options (a double click on a master, or its right-click menu)
  changes its prefix, name and parent; Delete Master (never A) deletes its
  layers too and leaves its pages with no master; masters based on it take
  its parent.
- A double click on a master (or Edit Master in its right-click menu)
  opens it for editing in place of the pages: the current page shows only
  that master's objects and its parents' (`layers_for_page_in` with a
  `MasterView`, a view the program keeps and exports never see), its
  number fields show the master's prefix, and a bar over the drawing names
  the master and offers Back to Pages. With facing pages both pages of
  the master's spread are at hand: its other page shows beside the one
  edited, and a press there makes it the one edited where it is on
  screen and goes on there (drawing, picking: `App::master_press`).
  New objects go on the master's layer for the side shown (made, named
  "B-Master (left)" or "(right)", the first time a side is edited: on
  left-hand or right-hand pages only), or on its layer for every page
  without facing pages. Going to a page (a double click on it in the
  panel, the page tabs) leaves the master
  (`what_is_drawn_on_a_master_shows_on_its_pages_on_its_side`).
- The Objects panel shows the master layers of the current page's master
  (and its parents) as shown, the others as not shown. A new master layer
  made on a page belongs to that page's master.
- Pages made after a page (New Page, Insert Page, Duplicate Page, the
  pages Flow Into New Pages adds) take its master in the same undo step.

## Pages

- Miniatures in spreads when Facing Pages is on (page 1 alone on the
  right, then pairs), one page a row otherwise; under each its number (the
  section's numbering), in its outer top corner its master's prefix. The
  current page has an accent outline, picked pages an accent background.
- With facing pages the Multipage View shows one spread a row (page 1
  alone on the right on top), each page labelled with its number (a name
  of its own after it), the shadows drawn before the pages so none falls
  across the spine.
- Click picks a page, Shift+click a run, Ctrl+click one more; a double
  click goes to the page. Dragging pages moves them before or after the
  page they are let go on (a bar shows where), or to the end on the space
  under the pages; one undo step.
- Dragging a master (or [None]) onto a page applies it to that page, or
  to all the picked pages when the page is one of them; onto the space
  under the pages it adds a page with that master.
- Miniatures are drawn again only when what they show changed: the layers
  drawn on the page (its own and its masters', number fields filled in)
  are compared with the copy last drawn, and at most 8 ms of new pictures
  are drawn per frame (`a_miniature_is_drawn_again_only_when_its_page_changed`).
- The footer: the count of pages (and spreads), New Page (after the
  picked page, with its master), Delete (the picked pages, never all) and
  the panel's menu.

## Menus

The panel's menu and the pages' right-click menu: Insert Pages... (how
many, after or before a page, at the start or the end, with which
master), Move Pages... (page list such as "1-4, 7", where), Duplicate
Pages, Delete Pages, Apply Master to Pages..., Save as Master, New
Master..., Numbering and Section Options..., Facing Pages, Thumbnail Size
(small, medium, large; kept in the settings). Each is one undo step
(`a_master_on_pages_and_new_pages_after_them_keep_it`,
`the_panel_draws_and_a_master_dropped_on_a_page_applies_it`).

## Colours

Each master has a colour (one by its id unless chosen in Master Options:
eight ready ones or any colour). Its pages show its prefix on a tag of that
colour in their outer top corner, and its row in the panel a swatch of it.

## Threaded text frames

Page > Threaded Text Frames... (also in the panel's menus): on the pages
from one to another, a paragraph text frame inside the margins (top,
bottom, inside, outside; the house standard's to start with; inside and
outside swap on left-hand pages with facing pages, the first page a
right-hand one), with columns and gutter, each linked to the one before.
Text put in the first runs on through the pages, and Text > Text Frame >
Flow Into New Pages carries it on past the last. One undo step
(`threaded_frames_sit_in_the_margins_and_carry_the_text_on`).

## The panel beside a folded column

- Shown beside a column folded to pictures, the panel folds away at a
  click on the drawing; a click in a window it opened (Master Options,
  Insert Pages) does not fold it, and those windows stay up whether the
  panel shows or not. The pin in its title keeps it open (kept in the
  settings) until its cross hides it.
- Each panel group's three-dot button has a menu of its own (they shared
  one, and opening one closed it at once).

## Not yet

- Overriding a master object on one page.
- Several page sizes in one spread, and spreads of more than two pages.
