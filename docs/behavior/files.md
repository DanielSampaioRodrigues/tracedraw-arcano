# Saving files

File > Save (Ctrl+S) writes the drawing back to its file. A drawing that
has none (new, or opened from a format TraceDraw only reads or converts,
`.cdr` included) asks for one with File > Save As (Ctrl+Shift+S), whose
dialog offers two types: TraceDraw (`*.tdraw`, the native JSON format,
which keeps everything) and `.cdr` files.

## `.cdr` versions

A program opens `.cdr` files of its own version and older ones, so the
version decides who can open a file. Saving as `.cdr` therefore asks for
the version, 7 to 22, newest first in the list:

- Save As to a `.cdr` file shows a second dialog, "Save as .cdr", with
  the Version list and the hint to choose the version of the program
  that will open the file, or an older one. OK (or Enter) writes the file;
  Cancel writes nothing.
- The list starts at the drawing's version: the version of the `.cdr`
  file it was opened from or last saved as, else the one in Options >
  Global > File Formats. Every `.cdr` save also sets that option, so new
  drawings start with the version chosen last (version 12 at first,
  which most programs open).
- The version chosen stays with the drawing: Save writes the same
  version again without asking. Each open drawing keeps its own.
- A drawing opened from a file older than version 7 is saved as version
  7 (the oldest written), one newer than 22 as version 22.
- In the browser, where every save is a download and there is no system
  dialog to choose the type, Save and Save As show one dialog, "Save",
  with Format (TraceDraw or `.cdr`) and Version (for `.cdr`). The
  download takes the drawing's name with the format's extension.
- `tracedraw-cli convert in.svg out.cdr` writes version 12, or the
  version in `TRACEDRAW_CDR_VERSION`; the MCP `save` tool takes an
  optional `version` (`mcp.md`).

What each version's file holds is in `docs/cdr-format.md`, Writing.

## Checks

- Given a drawing with a rectangle, when Save As chooses `drawing.cdr`,
  then the version dialog opens at version 12; choosing 17 and pressing
  Enter writes a file that reads back as version 17, Save writes version
  17 again, and a new drawing then starts at 17
  (`ui::save_cdr::tests::saving_a_cdr_file_asks_for_the_version_and_keeps_it`).
- Given a version 9 file opened while Options says 21, then saving
  offers 9; a new drawing beside it offers 21 and switching back offers
  9; versions 3 and 99 are saved as 7 and 22
  (`ui::save_cdr::tests::a_drawing_opened_from_a_cdr_file_keeps_its_version`).
- The browser form downloads `Plan.tdraw` or `Plan.cdr` by the format
  chosen; both forms of the dialog draw
  (`ui::save_cdr::tests::a_download_chooses_its_format`).
- The MCP `save` tool writes version 21 when asked and refuses versions 3
  and 23 (`mcp::tests::export_and_save_write_files`).
- Every version from 7 to 22 reads back with its pages, layers, objects,
  text runs and alignment (`write::tests` in `crates/cdr`).
