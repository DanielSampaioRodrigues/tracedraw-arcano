# VBA macros

## Macro files and forms

The `tracedraw-vba-project` crate (`crates/vba-project`) reads the files
VBA macros come in, so their code and forms can run in TraceDraw. It
follows the public MS-OVBA and MS-OFORMS specifications.

### Project files

- A compound file holding a VBA project (`.gms` files, and any file with
  a `VBA` storage containing a `dir` stream, at any depth) is read with
  `read_gms`. The project's root is the storage holding `VBA`. Project
  files saved by drawing suites start with a short header ("GMS", a
  version byte and a few fields, 18 bytes) before the compound file; it
  is skipped (`compound_part`).
- Exported module files (`.bas`, `.cls`, `.frm`) are UTF-8 when they are,
  else in the code page they were exported in, guessed from their bytes
  (Cyrillic, Central European, Greek and the others).
- Checked on four published macro projects (60 modules and their
  forms, Russian comments in code page 1251): the code read from each
  project file is the same, line by line, as the project's exported
  source files, and the form controls land where the form shows them
  (`cargo run -p tracedraw-vba-project --example compare -- file.gms
  folder`; `--example dump -- file.gms` prints the modules and the
  forms' controls).
- The `dir` stream and the modules' code are stored compressed (MS-OVBA
  2.4.1). The crate expands them and also compresses (`ovba::compress`),
  so it can write projects; both follow the chunk and copy token rules of
  the specification, including chunks stored as is.
- From the `dir` stream: the project name, its code page, its
  description and help file, the references (their names and library
  ids) and, for each module, its name (the Unicode name when given), its
  stream, the offset of its source in that stream, whether it is
  procedural, read only or private.
- Each module's code is the compressed text from that offset to the end
  of its stream, decoded in the project's code page (Western, Central
  European, Cyrillic, Greek, Turkish, Hebrew, Arabic, Baltic,
  Vietnamese, Thai, Japanese, Simplified and Traditional Chinese,
  Korean, UTF-8 and others; Western when the page is unknown). A module
  whose source offset is at the end of its stream was saved as compiled
  code only: it comes back with `has_source` false, empty code and a
  warning ("saved as compiled code only, without its source"), and
  `Project::modules_without_source` lists it.
- The PROJECT stream gives each module's kind: `Module=` standard,
  `Class=` class, `BaseClass=` form, `Document=` the host document's
  module. Without it, procedural modules are standard and the others
  classes.
- Protection: the `CMG`, `DPB` and `GC` values are unscrambled (MS-OVBA
  2.4.3). A password (DPB holding more than its single zero byte), a
  project locked for viewing (GC zero) or the protection state's user
  bit set `Project::protected`, with the details in `protection`. The
  code of a protected project is stored in the clear and is read like
  any other.
- `Module::macros` lists a standard module's subs without arguments
  (public or without a scope): what runs as a macro.
  `Project::macros` lists them for the whole project as (module, sub).

### Exported module files

- `read_files` takes files and folders, `read_named` (file name, bytes)
  pairs (for the browser, which has no folders). A folder stands for the
  `.bas`, `.cls`, `.frm` and `.frx` files in it; a compound file among
  the files adds its modules.
- `.bas`: a standard module. `.cls`: a class, its `VERSION 1.0 CLASS`
  header and `BEGIN`/`END` block skipped. `.frm`: a form; its header of
  `Begin`/`End` blocks gives the form's caption and size (`ClientWidth`,
  `ClientHeight` in twips), and for classic forms its controls with
  their position, size, caption, text, colours (`&H...&`), fonts
  (`BeginProperty Font`), enabled, visible, tab order and the rest. A
  forms library form names its controls' file in `OleObjectBlob =
  "name.frx":offset`; the `.frx` file is a compound file holding the
  designer streams, read like a project's form. Without it the form
  keeps its header and the project warns.
- The module name is its `Attribute VB_Name`, else the form block's
  name, else the file name. Text is UTF-8 when it is, else the project's
  code page.

### Forms

- A form's designer storage sits next to `VBA` under the form module's
  stream name. Its `\x03VBFrame` text gives the caption and size; its
  `f` stream the form's own properties (caption, displayed size, back
  and fore colours, font, picture, enabled) and the list of controls,
  with each control's name, id, position (HIMETRIC, read as points),
  tab index, tag, tip, control and row sources, visible, tab stop,
  default and cancel; its `o` stream each control's properties one
  after the other, as many bytes as the control's site gives.
- Control kinds come from the class cache index (MS-OFORMS 2.4.1) or the
  class table's class id: label, text box, command button, check box,
  option button, toggle button, combo box, list box (these six share a
  structure whose display style picks the kind), frame, multi-page,
  page, image, spin button, scroll bar and tab strip. Others are kept as
  `Other(index)`.
- Read per kind: captions, values, group names, sizes, colours, fonts
  (TextProps and standard fonts), enabled, locked, multi-line, word
  wrap, password character, accelerator, maximum length, list style,
  column count, the minimum, maximum, position, steps and orientation
  of spin buttons and scroll bars, the tab captions of tab strips, and
  pictures as their stored bytes.
- A frame or a multi-page keeps its controls (a multi-page its pages,
  each with its own controls) in a storage `i<ID>` (two or more digits)
  under its container; they become the control's `children`.
- Every read is bounds checked: a damaged form keeps what could be read
  and lists the rest in `warnings`, never failing the project.
- `Form::raw` keeps the designer streams as stored, so a form is
  written back unchanged.

### Writing

- `export_module` and `export_project` give `.bas` files, `.cls` files
  (with the class header) and `.frm` files with their `.frx` (the
  designer streams as stored, or written from the form's model), in the
  project's code page with CRLF line ends; an `Attribute VB_Name` line is
  added when the code has none.
- `write_gms` writes a compound file with the VBA project storage:
  every module's code compressed at offset 0 with no compiled code (the
  `_VBA_PROJECT` header's version 0xFFFF, so a host compiles the code
  when it loads it), the `dir`, PROJECT (with the protection values) and
  `PROJECTwm` streams, and the forms' designer storages.

### Limits

- Compiled code (p-code) is not read: a module saved without its source
  has no code to run.
- Forms: the binary layouts were built from the specification and
  checked against forms this crate writes; unusual properties (mouse
  icons, list column widths, multi-page transition effects, tab strip
  tooltips and tags) are skipped, and pictures are kept as bytes in the
  format they were stored in.
- References are names and library ids only; the libraries themselves
  are not loaded.

### Checks (tests in `crates/vba-project`)

- The specification's example without copy tokens expands and compresses
  to the same bytes; a run of 73 bytes is one literal and one copy
  token; text and incompressible noise over several chunks go round
  trip; damaged containers are errors (`ovba::tests`).
- `dir` records with a Cyrillic code page give the names, offsets and
  kinds; a record running past the end is an error (`dir::tests`).
- Scrambled protection values come back for any seed; the module kinds
  and the password, locked and user protection states go round trip
  through a PROJECT stream (`project_stream::tests`).
- A form written with a label, text boxes (one multi-line with
  non-Latin text), a check box, an option button in a group and a
  button reads back with captions, values, positions, sizes, fonts,
  enabled states and group names; a frame's controls in its own storage
  become its children; cut or garbled streams leave warnings, not
  panics (`forms::tests`).
- Classic and forms library `.frm` headers, `.cls` and `.bas` files are
  read (`frm::tests`).
- A project laid out as a host saves it (compiled code before the
  source, references, a document module, a module saved without its
  source, a form) gives its modules, kinds, macros, references and form;
  protection is read while the code stays readable; a Japanese project
  in Shift-JIS keeps its names and code; files that are not projects or
  are damaged give errors, never panics; projects go round trip through
  `write_gms`, through exported module files and through a folder
  (`tests`).
