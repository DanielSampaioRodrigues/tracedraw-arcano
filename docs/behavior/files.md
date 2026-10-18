# Saving files

File > Save (Ctrl+S) writes the drawing back to its file. A drawing that
has none (new, or opened from a format TraceDraw only reads or converts,
`.cdr` included), and File > Save As (Ctrl+Shift+S), open the Save
Drawing dialog.

## Save Drawing

One window, 760 px wide, the same on every system:

- **Folder** (desktop): an Up button, the path bar (a folder typed there
  opens with Enter; one that does not exist says so) and New folder
  (makes "New folder", "New folder (2)" ... and opens it). Down the left,
  shortcuts to the folders that exist: Home, Desktop, Documents,
  Downloads and Pictures (on Linux the names in
  `~/.config/user-dirs.dirs`), then the drives (Windows) or `/`. On the
  right, 250 px high and scrolling, the folder's contents: folders, then
  the files of the chosen type, by name ignoring case (hidden ones left
  out), with Name, Date modified (local time), Type ("File folder",
  "CDR file") and Size (KB, MB from 1 MB). A click on a file takes its
  name; a double click on a folder opens it.
- **File name**: the drawing's file name, else its title, else
  "Graphic1", with the type's extension; a name typed with either
  extension, or none, gets the chosen type's.
- **Save as type**: TraceDraw (`*.tdraw`, the native JSON format, which
  keeps everything) or CDR files (`*.cdr`). It starts at `.cdr` for a
  drawing that came from or was saved to a `.cdr` file.
- **Version** (`.cdr` only): 27 down to 7, each with its release name
  ("v13 (X3)", "v27 (2026)"); versions not yet checked in other programs
  say "experimental" (versions 7 to 14).
- **Embed colour profile (size)** with "Colour profile: name", shown when
  the drawing carries a CMYK profile: on, the profile is stored in the
  file; off, it is left out. Only version 16 and later store one, so the
  box is off and greyed for older versions. The choice stays for the
  next Save As and for Save.
- **Title**, **Subject** and **Rating** (five stars; clicking the lit
  last star takes it off): the document's properties, also in File >
  Document Properties; changed ones are set (one undo step each) before
  the file is written.
- **Save** (or Enter) writes the file; a file that exists asks "<name>
  already exists. Replace it?" with Replace and Cancel. Cancel closes
  without writing. The dialog starts in the drawing's folder, else the
  last folder saved to, else Documents, else home.
- In the browser, where every save is a download, the dialog keeps the
  file name, type, version and properties, and says the browser saves
  the file in its downloads folder.

## `.cdr` versions

A program opens `.cdr` files of its own version and older ones, so the
version decides who can open a file:

- Save As starts at the newest version, 27, whatever the drawing came
  from.
- The version chosen stays with the drawing: Save writes the same
  version again without asking. Each open drawing keeps its own.
- A drawing opened from a file older than version 7 is saved as version
  7 (the oldest written), one newer than 27 as version 27. Options >
  Global > File Formats (version 27 at first) is used for a drawing
  saved to a `.cdr` file without a version of its own.
- `tracedraw-cli convert in.svg out.cdr` writes version 12, or the
  version in `TRACEDRAW_CDR_VERSION`; the MCP `save` tool takes an
  optional `version` (`mcp.md`).

What each version's file holds is in `docs/cdr-format.md`, Writing.

## Checks

- Given a folder with a subfolder, a `.cdr` and a `.txt` file, the
  TraceDraw type lists the subfolder only and CDR the subfolder and the
  `.cdr` file; saving "poster" as CDR version 15 with a title, subject
  and rating writes `poster.cdr` there (version 15), sets the properties
  and remembers the folder; the next Save As starts there as `.cdr`
  version 27; an existing name asks first and Replace overwrites it; a
  TraceDraw file is written with its own extension
  (`ui::save_as::tests::save_as_chooses_folder_name_type_version_and_properties`).
- An empty name, a bare extension and a missing folder save nothing and
  say why; typed extensions become the type's
  (`ui::save_as::tests::names_and_folders_are_checked`).
- A version 9 file opened offers CDR version 27; a new drawing offers
  TraceDraw and 27; a copy saved as version 9 is version 9 and Ctrl+S
  writes version 9 again without asking; versions 3 and 99 are saved as
  7 and 27
  (`ui::save_as::tests::a_drawing_opened_from_a_cdr_file_keeps_its_version`).
- A drawing with a CMYK profile offers it by name and size; saved with
  the box on, the version 27 file holds the profile, with the box off it
  does not, and the next Save As keeps the box off
  (`ui::save_as::tests::the_cmyk_profile_goes_along_unless_left_out`).
- The dialog draws, folders open (`ui::save_as::tests::the_dialog_draws_and_opens_folders`),
  and the Linux folder names are read from the user's settings
  (`ui::save_as::tests::linux_folder_names_come_from_the_users_settings`).
- The MCP `save` tool writes version 21 when asked and refuses versions 3
  and 23 (`mcp::tests::export_and_save_write_files`).
- Every version from 7 to 22 reads back with its pages, layers, objects,
  text runs and alignment (`write::tests` in `crates/cdr`).
- Checked on screen: the dialog with its shortcuts, list, type list,
  version and stars; Save as CDR wrote the file in the folder shown.
