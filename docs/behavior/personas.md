# Workspaces one click apart (Arcano edition)

At the right of the menu bar, while a drawing is open: Drawing and Page,
as a small switch (the one in use raised, with a green line under it).
The work is in `apps/tracedraw/src/personas.rs`.

- Each workspace keeps its own panels. Changing workspace keeps the
  panels of the one left (`settings.arcano_persona_layouts`, by id) and
  puts back those of the one chosen; the first time, its preset.
- Drawing's preset: Properties and Hints; Objects.
- Page's preset, in three stacked groups: Pages and Objects; Character,
  Paragraph and Text Styles; House Press and Align and Distribute.
  Rulers and guides turn on.
- Character and Paragraph are the Text panel's settings, each as a panel
  of its own (Window > Panels lists them with Text), laid out for page
  work (`ui/painel_tipo.rs`): a small picture before every field, two
  fields to a row. Character: font and style across; size and leading
  (typed in points, it sets the paragraph's line spacing in points),
  tracking and baseline shift, character angle and horizontal offset;
  buttons for small caps, superscript and subscript (the font's OpenType
  features `smcp`, `sups`, `subs`; superscript and subscript exclude each
  other), underline, strikethrough and ligatures; old-style figures,
  fractions and swash; the font's axes; then the fill, background and
  outline rows. Paragraph: the six alignments; left and right indents,
  first line indent and drop cap lines (one line is none), space before
  and after; hyphenation; buttons for the baseline grid, Bullets, Tabs and
  Drop Cap; word and letter spacing under a fold; widows, orphans and keep
  with next for paragraph text. Tests: `ui::painel_tipo::tests`.
- The drawing, the tools, the bars and the menus are the same in both;
  the status line names the workspace chosen.
- Window > Workspace > Reset this workspace's panels puts the preset
  back for the workspace in use.
- The workspace in use is kept in the preferences (`arcano_persona`).

## The user's workspaces, colours and pictures

- The small "+" after the workspaces saves the panels as they are as a new
  workspace: a window asks its name, its colour (twelve offered, or any
  from the colour button) and its picture (fifteen drawn ones, or none).
  It comes after Drawing and Page and is put in use
  (`settings.arcano_workspaces`: id `user-N`, name, colour, picture and the
  panels it was saved with).
- A right click on a workspace: Name, colour and picture...; Keep these
  panels in this workspace (the user's, in use: what Restore brings back);
  Reset this workspace's panels; Delete this workspace (the user's only;
  Drawing comes back when it was in use).
- Drawing is green and Page orange (`PAGE_COLOR`, #E06C12); both can take
  another colour, picture and name (`settings.arcano_workspace_looks`).
- While a workspace is in use the interface's accent takes its colour:
  selected buttons and tools, menu hover, selected rows, focus rings, links,
  drop bands and the icons' detail (`theme::set_accent`, `theme::rehue`:
  every colour made around the green is turned to the workspace's hue,
  its saturation and lightness scaled as the colour's differ from the
  green's, so the green becomes the colour exactly and its pale tints pale
  tints of it). Not in the High Contrast theme, which takes the system's.

Tests: `personas::tests`.

