# Stacked panels

Options > Customization > Appearance > Stacked panels (on by default in
the program; `Settings::stacked_panels`, off in the tests, which check the
side tabs). The work is in `apps/tracedraw/src/ui/panel_stack.rs`.

- An open docked column shows its panels in groups, one under the
  other. Each group has its tabs across its top (the one in front in the
  panel colour, the others dimmed; a cross while the pointer is over a
  tab closes it), the panel list (three dots) at the right and, on the
  first group, collapse. A divider under each group but the last shares
  the height between two groups (each keeps at least 90 px).
- A collapsed column is a strip 40 px wide: at its top a button that
  opens the whole column, then a picture for each of its panels (every
  panel has its own, `ui/panel_icons.rs`, with a detail in the icons'
  green) and the panel list.
- A click on a picture shows that panel beside the strip, over the
  drawing (300 px wide, as tall as the column up to 640 px), and the
  column stays collapsed. Its title has a button that opens the column
  with that panel in front, and one that hides it. A second click on the
  picture, Esc, or a click anywhere outside it and the strip hides it
  (not while a menu or picker it opened is up).
- A picture can be dragged: along the strip to put the panels in another
  order, onto another strip or column, or away to float. A panel dropped
  on a collapsed strip joins its pictures and the column stays collapsed.
- Dragging a tab over a stacked column shows a band: over a group's tabs
  or the upper half of its page the tab joins that group, over the lower
  half it starts a new group under it. Elsewhere the panel goes where it
  went before (another column, a second column, floating).
- The groups only divide the column's panels (`Stacks::follow` each
  frame): a panel that leaves the column leaves its group, one that comes
  joins the group of the panel in front (else the first), and the panel
  put in front (Window > Panels, a click) is in front of its group.
- The groups are kept with the panels (`DockerSetup::stacks`: the panels'
  keys, the one in front and the share of the height of each group).

## Floating panels in one window (Arcano edition)

- A panel dropped on the top of another floating window (its title or tab
  strip and a little under it, the window shaded while over it) becomes a
  tab of that window, in front (`FloatingDocker::host`; the window's own
  panel keeps which one is in front, `front`). A panel from a column joins
  the same way; a window of several panels dropped there brings them all.
- A window of several panels shows a tab strip instead of a title: a click
  brings a panel to the front, dragging a tab takes that panel out
  (anywhere: a column, another window, a window of its own), dragging the
  free part of the strip moves the window with all its panels (into a
  column they all go there), the cross closes the panel in front, a right
  click on a tab gives its places and Close.
- When the window's own panel leaves, the next one takes the window where
  it was. The windows and their tabs are kept with the panels
  (`FloatingSetup::host` and `front`).

Tests: `app::tests::floating_panels_join_one_window`.

Tests: `ui::panel_stack::tests`.
