# House library (Arcano edition)

The Shelf panel (Biblioteca in Portuguese) can live in a folder that the
editora's computers sync (Google Drive, OneDrive, iCloud or any other).
The work is in `apps/tracedraw/src/biblioteca.rs` and `shelf.rs`.

- At the top of the panel: where the library is, Choose Folder...,
  This Computer Only, and Save as House Template.
- Choosing a folder makes `Biblioteca TraceDraw/` in it, with
  `biblioteca.json` (the shelf: objects and colours) and `Modelos/`.
  When the folder's library is new, what was on this computer's shelf
  goes into it; otherwise the folder's library is read.
- The shelf file is looked at every 2 s and read again when another
  computer changed it (not while an item is being renamed or dragged).
- Save as House Template writes the drawing, with its fonts carried, as
  `Modelos/<title>.tdt` (a free name: "Title 2.tdt"...). The house
  templates show in File > New From Template.
- The folder is a preference (`arcano_library`); empty means the shelf
  stays in the preferences folder, as upstream.
