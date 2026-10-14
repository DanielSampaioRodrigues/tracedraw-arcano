# The `.tdraw` format

A `.tdraw` file is the drawing's `Document` as JSON (`Document::to_json`
in `crates/core/src/document.rs`), with `format_version` at the top
level. Fonts the drawing carries are embedded in it.

## Versions

| Version | Change |
|---|---|
| 0 | Files written before versions were recorded (no `format_version`). |
| 1 | `format_version` recorded; the layout is the same as 0. |

## The promise

- Every `.tdraw` file TraceDraw has written opens in every later
  TraceDraw. A file of an older version is brought up to date on
  reading, one step per version (`migrate`; `STEPS[i]` takes version `i`
  to `i + 1`).
- A new field always comes with a default, so files without it read as
  before; adding one does not raise the version.
- A field renamed, moved or given another meaning raises
  `TDRAW_FORMAT_VERSION` and adds the step that rewrites older files.
- A file from a newer TraceDraw opens as far as this one knows it: fields
  it does not know are left out (saving it again drops them).

## Checks

- `document::tests::tdraw_files_carry_their_format_version_and_older_ones_open`
