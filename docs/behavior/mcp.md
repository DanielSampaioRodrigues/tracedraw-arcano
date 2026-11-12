# MCP server (`tracedraw --mcp`)

`tracedraw --mcp` runs the editor without a window and speaks the Model
Context Protocol over stdin and stdout: JSON-RPC 2.0, one message per
line, protocol version 2024-11-05, the `tools` capability. An AI agent
(or any MCP client) connected to it opens, edits, inspects, exports and
saves documents with the same commands and undo history a user has.

Client configuration, for example:

```json
{ "mcpServers": { "tracedraw": { "command": "tracedraw", "args": ["--mcp"] } } }
```

## The open window (`tracedraw --mcp-window`)

The same tools can work on the drawings of a TraceDraw window the user
has open. Options > Global > General > "Let AI agents on this computer
edit the open drawing (MCP)" (off by default, desktop only) makes the
window listen on a free port of `127.0.0.1` and write the port, a random
128-bit token and its process id to `mcp-window.json` in the preferences
folder (removed when the option is turned off or TraceDraw closes). The
client is configured with the bridge as its command:

```json
{ "mcpServers": { "tracedraw-window": { "command": "tracedraw", "args": ["--mcp-window"] } } }
```

The bridge reads the file and forwards each line of standard input as an
HTTP `POST /mcp` with `Authorization: Bearer <token>` (never through a
proxy) and writes the answer to standard output. The window answers on
its own thread between frames, as if the user had run the command: the
tools act on the active drawing (`open` and `new_document` open tabs),
each change is one undo step, and the window shows it at once. The
status bar says when agents are let in and on which port, and the option
page shows the port.

Refused: requests without the token or with another one (403), requests
carrying an `Origin` header, which web pages send (403), other methods
(405) and paths (404), bodies over 16 MB (413). Notifications are
answered `202 Accepted` with no body. A request waits up to 120 s for the
window. Without a window that lets agents in, the bridge answers every
request with a JSON-RPC error (-32000) saying how to turn the option on.

## Methods

| Method | Reply |
| --- | --- |
| `initialize` | protocol version, `tools` capability, server name and version |
| `ping` | `{}` |
| `tools/list` | the tools below with their JSON schemas |
| `tools/call` | `content: [{type: "text", text}]`, `isError` on failure |
| notifications (no id) | no reply |
| anything else | error -32601 |

## Tools

| Tool | Arguments | Effect |
| --- | --- | --- |
| `open` | `path` | Opens any supported file (`.cdr`, `.tdraw`, SVG, PDF, AI, EPS, DXF, PSD, EMF, WMF, images) and returns the document description |
| `new_document` | `width_mm`, `height_mm`, `title` (all optional; A4 default) | A fresh document |
| `save` | `path`, `version` (optional, `.cdr` only: 7 to 27) | Native `.tdraw`, or `.cdr` through the writer in `version` (default: the version of the `.cdr` the document came from or was last saved in, else the one in Options > Global > File Formats); marks the document saved |
| `export` | `path`, `page_index` (0), `dpi` (96) | SVG, PDF, EPS, PS, PRN, DXF, EMF, WMF, PLT, PSD, HTML or PNG by extension |
| `run_script` | `source` | Runs the JavaScript object model (`Application`, `ActiveDocument`, `ActivePage`, `ActiveLayer`, `ActiveSelection`, `Shapes`, `CreateRectangle`, `CreateEllipse`, `CreateArtisticText`, `CreateCurve`, `Fill`, `Outline`, `Move`, `Rotate`, `Stretch`, `Duplicate`, `Delete`, ...) and returns what it printed; the whole script is one undo step |
| `document_info` | | Title, file, modified flag, pages with size, layers with visibility and lock, every object with id, type, name, bounds in mm, fill, outline, opacity, text and font, group children, effect count; the selection |
| `render_page` | `page_index` (default: the page being edited), `max_px` (1024; 256 to 2048) | A PNG picture of the page on white, `max_px` on its longer side, as MCP image content, with a line giving the page, its size and the resolution; for the agent to look at what it drew |
| `undo`, `redo` | | One step back or forward, then the description |

New objects go to the active layer: the layer picked in the Objects
docker while it shows on the page, visible and unlocked, else the
topmost layer that is visible and unlocked (a hidden layer never
receives objects; with every layer hidden, the topmost unlocked one).
The same rule applies to drawing in the window.

## Checks

- `initialize` answers with the protocol version and server name; a
  notification gets no reply; an unknown method gets -32601.
- `new_document` 100 x 50, then a script creating a red rectangle and
  printing `ActivePage.Shapes.Count`, prints `1`; `document_info` lists
  one rectangle with fill `R:255 G:0 B:0`; `undo` leaves none.
- `export` to svg, pdf, png and emf and `save` to cdr and tdraw each
  write a non-empty file; the saved `.cdr` opens again with its ellipse.
- `save` with `version` 21 writes a file that reads back as version 21;
  versions 3, 6, 28, -1 and a string are refused and write nothing.
- The open window: with the option on, `initialize`, a notification and
  a script drawing an ellipse sent through the bridge get the server's
  name, no answer and `ok`, the window's drawing has one more object and
  Undo takes it back; the link file goes when the link does
  (`mcp_link::tests::agents_edit_the_drawing_in_the_window`).
- Requests without the token, with a wrong one or with an `Origin`
  header get 403, a GET 405, another path 404, the right token 200
  (`mcp_link::tests::strangers_and_web_pages_are_refused`).
- Without a window, or with one that went away, the bridge answers with
  an error naming the option (`mcp_link::tests::without_a_window_the_bridge_says_so`);
  the option starts and stops listening
  (`mcp_link::tests::the_option_starts_and_stops_listening`).
- Checked on screen: `tracedraw --mcp-window` drew a star and a line of
  text in the open window.
- `render_page` answers with an image of the page: red where a script
  drew, white elsewhere, at the asked size; a missing page is an error
  (`mcp::tests::render_page_shows_the_page_as_a_picture`).

## Arcano edition: working beside the user

The edition's tools are in `apps/tracedraw/src/mcp_plus.rs` (asked
first by `mcp.rs`) and the bridge's own in `mcp_link.rs`. An agent works
in the window the user works in, and always knows which one.

**Which window.** Each listening window also writes
`mcp-windows/<pid>.json` in the preferences folder. The window that has
the keyboard points `mcp-window.json` at itself (checked at most once a
second), so the bridge follows the window the user used last, not the
one opened last. When that window closes, the link passes to the other
window used last; when it cannot be reached, the bridge tries the others.
Every answer from a window ends with a line naming it:
`[TraceDraw window <pid> · drawing "<title>" · page n of m]`.

| Tool | Arguments | Effect |
| --- | --- | --- |
| `windows` (bridge) | | The windows that let agents in: process id, drawing in front, open drawings, which one the tools work in and which one the user used last |
| `use_window` (bridge) | `window` (pid; 0 follows the user again) | Work in that window; when it closes, the bridge follows the user again and says so |
| `window_info` | | The answering window's pid, drawing, file, open drawings, objects on the page, selected count |
| `document_info` | `page_index` (optional) | Upstream's description plus `window`, `open_documents` (index, title, active), `active_page_index`, `active_layer`, `view` (the part of the page shown, mm, and the zoom) and the coordinate convention |
| `switch_document` | `index` | Brings that drawing's tab to the front |
| `get_selection` | | The selected objects in full and the selection's bounds |
| `select` | `ids`, `add` | Selects as a click would (the objects' page comes to the front); an unknown id or objects on two pages are refused |
| `render_area` | `area` (`selection`, default when something is selected, `view`, `page`), or `ids` or `rect_mm`; `padding_mm` (3), `max_px` (1024; 128 to 2048), `page_index`, `save_to` (a .png path, for `compare_snapshots`) | A PNG on white of just that part of the page, drawn at that size (not cut from a page picture), and a line giving the area in mm and the resolution |
| `font_metrics` | `family`, `size_pt` (12), `bold`, `italic`, `text` | The face's ascent, descent, line height, cap height and x-height in mm, from its tables; with text, its advance width and its letters' ink bounds from the baseline start. A font that is not installed is an error naming similar families, never a substitute's measures |
| `list_fonts` | `filter` | Up to 200 families with their faces, and how many matched |
| `api_reference` | `model` | The object model as `run_script` defines it, with the conventions (mm, lower-left origin, y up) |
| `run_script` | `source`, `model` (`tracedraw` default, or `vba`: the model macros use) | One undo step named "AI agent". All or nothing: a script that throws keeps none of its changes (upstream keeps what ran before the error) |

### Checks (Arcano edition)

- Two windows: the one opened last is followed until the other takes the
  keyboard; `tools/list` through the bridge carries `windows`,
  `use_window` and `render_area`; every answer names its window;
  `windows` lists both; `use_window` sends the tools to the other one,
  an unknown pid is refused, 0 follows the user again; closing the
  followed window passes the link on, closing both removes it
  (`mcp_link::tests::agents_follow_the_window_in_use_and_can_pick_one`).
- A script that throws leaves the page as it was; a good one is one undo
  step (`mcp_plus::tests::a_failed_script_leaves_nothing_behind`).
- The macro model: `cdrMillimeter`, `CreateEllipse2` and `RGBAssign`
  draw a red circle at the right place; an unknown model is refused
  (`mcp_plus::tests::scripts_in_the_macro_model`).
- `api_reference` gives both models (`mcp_plus::tests::the_reference_names_the_models`).
- Selecting by id, reading the selection and its bounds, refusing an
  unknown id, clearing (`mcp_plus::tests::selection_is_read_and_set`).
- `render_area` of a 10 mm selected square at 200 px is that square
  filling the picture; a rectangle of empty page is white; with nothing
  selected the page; asking for the selection then is an error
  (`mcp_plus::tests::render_area_looks_at_objects_closely`).
- `font_metrics` of an installed font at 72 pt has an ascent between 10
  and 30 mm, twice that at 144 pt, and a width for text; a font that is
  not installed is an error (`mcp_plus::tests::fonts_are_measured_not_guessed`).

## Arcano edition: book and page tools

The tools a press works a book with, in `apps/tracedraw/src/mcp_prelo.rs`
(reading and checks) and `mcp_prelo_fix.rs` (composing and fixing),
asked after `mcp_plus.rs`'s. A story is a chain of linked paragraph
frames, named by the id of its first frame (`story` takes any of its
frames); without it a tool works on the main story, the one with the
most characters. Pages are indices (`from_page`, `to_page`, 0 = the
first); answers give each page's number as the drawing numbers it too.
Lengths are mm, page lower-left (0, 0), y up. The checks only diagnose;
each tool that changes the drawing is one undo step and none saves it
over its file. The tools' files go to Documents/TraceDraw
(`checkpoints`, `scripts`, `projects`).

| Tool | What it does |
| --- | --- |
| `document_report` | Format, facing pages, bleed, house margins, masters with their primary frames, stories, styles, images, missing fonts, and the preflight's problems |
| `page_layout` | Per page (10 at a time, at most 50): size, side, master, content area (the primary frame's, else the house margins, mirrored), columns; each object with id, type, layer, bounds, for text frames story, next and previous frame, overflow, primary mark, for images their effective ppi |
| `story_map` | The stories with frames, pages, characters and overflow, the main one told from short texts; the paragraph styles of the main story with counts and how many start a page; the likely opening styles |
| `export_lines` | Every line laid out: page, frame, column, paragraph, first and last, baseline and ends, measure, style, alignment, size, hyphen, the space ratio of justified lines; `save_to` writes JSON |
| `audit_justification` | Loose (over 133 % of a normal space) and tight (under 80 %) lines, measured against the same line set flush left; rivers (gaps overlapping on 3 or more lines of a column); hyphens in a row, at the foot, on the last word, one-letter syllables, proper names split |
| `audit_breaks` | Widows, orphans, a single word on the last line, titles at the foot, stacks of the same first or last word on 3 lines, spreads ending at different heights (not before an `opening_styles` paragraph) |
| `audit_rag` | Ragged paragraphs: wedges, holes, alternation, a last line under a fifth of the longest, a line ending in a short function word, a number apart from its unit, a proper name split; `poetry` only points |
| `audit_density` | Characters per mm of full lines against the median of their style and size: light and dark lines (10 %), patches of 3, pages off tone (2.5 %) |
| `audit_microtypography` | Straight quotes, hyphens as dashes, dashes spaced two ways, double spaces, spaces before punctuation, punctuation stuck to the next word; suggestions: three dots, acronyms in capitals, long numbers |
| `audit_structure` | Off the page, beyond the bleed, text frames outside the margins, overflow, empty frames, frames over twice their text, loose frames of running text on pages one after another |
| `audit_local_formatting` | Runs that differ from their paragraph style: off the style, the author's marks by hand, composition settings, with the local and the style's values |
| `check_openings` | The House Press's front matter and openings checklist |
| `check_fidelity` | Characters in italic, bold, underline, strike-through, superscript, subscript and small caps, and paragraphs, in the .docx and in the story; what went missing |
| `font_available` | Whether named families are installed, with their faces, or similar names |
| `house_standard` | The house margins, bleed and safety distance, new text's settings, the drawing's styles and the house rules |
| `craft_reference` | The craft base (`docs/craft.md`): chapters, a chapter, or the sections holding all the words searched |
| `create_document` | A new drawing in the house standard: size, pages, facing pages, margins, book styles, bleed and guides, a primary frame on master A threaded through every page; nothing to undo, not modified |
| `place_text` | Text or a .txt from a page: the page's empty primary frame or a frame in its content area, frames linked on the next pages, pages added at the end (Flow Into New Pages) until it fits; a style for all and one for the first paragraph |
| `thread_frames` | Links frames in order (each joins the end of the chain before) |
| `fit_frame` | The height that just holds the text (and `padding_mm`), the top kept; linked frames are left |
| `apply_paragraph_style` | A style by name to a story or its paragraphs `from` to `to`, optionally clearing local formatting |
| `clear_local_formatting` | Style properties back to the style (and composition ones when asked); the author's marks never touched, checked character by character |
| `fix_typography` | The House Press's typography fix on the story or all texts, refused if a word would change |
| `fix_justification` | Per paragraph with loose or tight lines, the paragraph's tracking from -5 to +5 thousandths of an em: the most even spacing that keeps its line count, the frame's fitted text and no more lines out of range |
| `fix_breaks` | Single words: tracking up to +15 thousandths bringing one more word down, the paragraph ending where it did; widows and orphans: a paragraph of the frame before (widow) or of the frame (orphan) gains or loses a line, kept only when the story's widows and orphans go down; what cannot be fixed is listed |
| `checkpoint` | A .tdraw copy with date and time in its name; the open drawing untouched |
| `save_as` | Saves under a new name and opens it; refuses the open file and existing files |
| `run_script_batch` | Scripts one after another, each an undo step, after a checkpoint; stops at an error and at new objects off the page or overflowing |
| `script_library` | Scripts kept by name with description, arguments, model and where they were used: list, search, read, save |
| `start_project`, `save_briefing`, `read_briefing` | The short interview per kind of job, the briefing saved as JSON and Markdown, read back or listed |
| `compare_snapshots` | Two PNGs of the same area (from `render_area` with `save_to`): the share that changed, regions boxed in red, in mm with `dpi` |
| `measure_contrast` | WCAG contrast of the letters of texts against what is behind them (drawn without them): median, worst 10 %, shares under 3:1 and 4.5:1, by fifth of the width; `suggest_position` tries moves and gives the best |

### Checks (book and page tools)

- Every tool has a description and an answer, and no name is shared with
  the other lists (`mcp_prelo::tests::every_book_tool_has_a_name_a_description_and_an_answer`).
- The report, layout and map read a drawing (`the_report_layout_and_map_read_the_drawing`).
- Lines come with their place and space ratio (`lines_come_with_their_place_and_spacing`).
- Micro-typography cases, breaks, structure and rag (`the_checks_find_their_cases`);
  rivers (`rivers_are_gaps_lined_up_over_three_lines`); local formatting
  groups (`local_formatting_is_told_apart_from_the_authors_marks`);
  fonts, craft base, house standard (`fonts_and_the_craft_base_answer_by_name`).
- A new book in the house standard takes its text through new pages, one
  undo step (`mcp_prelo_fix::tests::a_new_book_gets_the_house_standard_and_its_text_runs_through`);
  threading and fitting (`frames_thread_and_fit_their_text`); clearing
  keeps italic (`clearing_local_formatting_keeps_the_authors_marks`);
  typography (`typography_is_fixed_without_touching_words`); justification
  keeps the pagination (`justification_fixes_keep_the_pagination`);
  files never over a file (`files_go_to_the_work_folder_and_never_over_a_file`);
  scripts, batches and briefings (`scripts_are_kept_and_found_and_batches_stop_at_errors`);
  contrast (`contrast_is_measured_on_what_is_behind_the_letters`);
  snapshots (`snapshots_compare_by_pixel`).
