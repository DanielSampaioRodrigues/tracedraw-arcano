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
- Character and Paragraph are the Text panel's two parts, each as a
  panel of its own (Window > Panels lists them with Text).
- The drawing, the tools, the bars and the menus are the same in both;
  the status line names the workspace chosen.
- Window > Workspace > Reset this workspace's panels puts the preset
  back for the workspace in use.
- The workspace in use is kept in the preferences (`arcano_persona`).
