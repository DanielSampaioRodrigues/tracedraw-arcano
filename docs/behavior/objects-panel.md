# Objects panel

The Objects tab of the docker lists the current page's layers, topmost
first, with "+ Layer" at the top right.

| Row | Contents |
|---|---|
| Layer (light grey) | collapse arrow, eye (show or hide), padlock (lock or unlock), layer icon, name in bold |
| Object | swatch of its fill (a red diagonal for none) framed in its outline colour, then its name, or what it is when it has none ("Rectangle", "Curve with 8 nodes", "Paragraph Text: ...") |
| Group | as an object, with an arrow that opens it to its objects, indented one step (16 px) |

- Rows are 22 px high. The selected objects show in `#CCE8FF` with a
  `#99D1FF` frame; the row under the pointer in `#E5F3FF`. Hidden
  objects show their name dimmed.
- Click an object to select it, Shift+click to add it.
- Double-click a layer to rename it; double-click an object to name it
  (Rename Object; an empty name goes back to the description).
- A layer's context menu: Rename, Move Up, Move Down, Delete Layer.
- Which layers are closed and which groups are open lasts for the
  session.

## Checks

- `ui::objects_docker::tests::objects_show_their_names_or_what_they_are`
- `ui::objects_docker::tests::the_docker_draws_layers_groups_and_selection`
