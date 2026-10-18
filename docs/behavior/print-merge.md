# Print Merge

File > Print Merge > Create/Load Print Merge opens the dialog; Edit Print
Merge reopens the records kept with the session, Perform Merge runs them
again. `print_merge.rs` holds the model and the merge.

## Records

The dialog: "Add columns or records manually or import data from a file"
with Import File; under "Add or edit records" Add Column, Configure
Columns and View Single Record; the records; + and - (add a record, delete
the current one) and the navigation (first, previous, next, last, Go to
Record); Save Data Source, Finish and Cancel.

- Import File reads a CSV whose first line names the fields (comma or
  semicolon, whichever the first line has more of). Save Data Source
  writes the records back as CSV, counters written out.
- Add Column asks for a name and a type and adds the field to every
  record (and a first record when there is none).
- Configure Columns shows each field's name (edited in place), type,
  counter settings and the x that removes it.
- View Single Record shows the current record as a form; View All
  Records goes back to the grid, where a click on a record's number makes
  it the current one.
- Types:
  - Text: put in place of `<Field>` in every text object of the page.
  - Number: a counter, not typed in: Start, then Step more for each
    record, padded with zeros to Digits digits (start 7, step 3, digits 3
    gives 007, 010, 013).
  - Picture: the cell is the path of a picture (relative paths start at
    the data file's folder). A text object holding only `<Field>` becomes
    the picture, as large as fits in the object's box (the frame of
    paragraph text) keeping its proportions, rotated with it.
  - QR code: a text object holding only `<Field>` becomes a black QR
    code of the cell's text, square, as large as the box allows.
  An empty cell, or a picture that does not load, removes the field's
  object from that record's page.
- The placeholder buttons insert `<Field>` into the text being edited.

## Merge

Finish (or File > Print Merge > Perform Merge) copies the current page once per record, right after it,
in the records' order, names the copies "<page> 1", "<page> 2", ... and
fills them. It is two undo steps (the copies, then the fills).

## Checks

- `print_merge::tests::counters_and_short_records_are_filled_in`
- `print_merge::tests::fields_and_records_are_added_and_removed`
- `print_merge::tests::csv_round_trips_with_counters_written_out`
- `print_merge::tests::merge_fills_text_qr_and_counters`
