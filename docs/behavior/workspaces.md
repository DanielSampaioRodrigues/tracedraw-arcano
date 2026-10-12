# Workspaces

A workspace is how the window is set up: which bars show and where they
sit, the toolbox, keyboard shortcuts, the colour palette options and the
interface's look. Window > Workspace picks a built-in one (Default, Lite,
Classic, Illustration, Page Layout, Touch); the user's own setup can be
kept in a file and brought back.

## Touch

For touch screens: the interface (bars, buttons, panels, menus and text)
is drawn `TOUCH_SCALE` = 1.4 times the interface size chosen (at most
400 %), and drawings keep their real size at zoom 100 % (the points per
inch shrink by as much). The standard toolbar, property bar and toolbox
show; rulers, panels and the status bar do not. On the drawing two
fingers pan and a pinch zooms about its centre, with any tool; the
gesture drops what the first finger had begun, so it neither drags an
object nor draws a marquee. Picking another workspace brings the
interface back to the size chosen.
Checks: `ui::tests::the_touch_workspace_is_larger_and_keeps_real_sizes`,
`pick_drag_tests::two_fingers_pan_without_dragging_anything`.

## Kept across restarts

Everything below is part of the preferences (`settings.json`, or the
browser's local storage) and comes back when TraceDraw starts again:
where each bar sits (`bars`), which bars show (`shown_bars`: standard
toolbar, property bar, toolbox, status bar, panels, rulers, Text, Zoom
and Transform toolbars), the tool each toolbox group shows, hidden
flyouts, shortcuts, palette options, interface size and desktop colour,
and the dockers (`dockers`: the dockers in the tab column, in order, and
the one in front). Preferences written before the bars shown were kept
have no `shown_bars`: the built-in workspace's bars show, as before;
without `dockers` the column holds Hints, Properties and Objects.
Picking a built-in workspace sets its own bars, as it always did.

## Dockers

The tab column holds the open dockers, as tabs; a click on a tab brings
its docker to the front, a click on the tab in front collapses the
docker area to the column. The docker's close button (x) and its tab's
"Close Docker" take the docker out of the column and bring the next one
forward; with none left the docker area closes. Collapse (the two
triangles) only folds the area. A docker opened from a menu or its
shortcut joins the column; Help > Hints opens Hints and, ticked, closes
it. The column is kept when TraceDraw closes, so a docker closed stays
closed the next time.
Checks: `app::tests::a_closed_docker_stays_closed_the_next_time`.

## Export and import

Options > Workspaces > Workspace > Export and import, or Window >
Workspace > Export Workspace... and Import Workspace...

| Part | What it holds |
|---|---|
| Keyboard shortcuts | the user's shortcuts for menu commands and tools (`Settings::shortcuts`) |
| Toolbars, toolbox and status bar | the built-in workspace it starts from, where the bars sit and whether they are locked, which bars show, the tool each toolbox group shows, hidden flyouts, the Outline flyout, what the status bar's first field shows |
| Color palette options | No Color well, right mouse button, document palette, the palette shown, colours joining the document palette |
| Appearance | interface size, desktop colour, page border |

- The options page has a check box per part (remembered in
  `workspace_parts`); Export writes the parts checked, Import applies the
  parts checked that the file has. The Window menu items use every part.
- An import from the options is part of the options: Cancel puts the
  previous setup back, OK keeps it. From the Window menu it is kept and
  saved at once.
- The status bar names the file and the parts applied, or says the file
  is not a workspace file or has none of the parts chosen.

## The file

JSON with the extension `.tdws`; parts not exported are left out.

```json
{
  "kind": "tracedraw-workspace",
  "version": 1,
  "base": "default",
  "shortcuts": [["menu.file.export", "Ctrl+Shift+X"], ["pick", "Q"]],
  "bars": {
    "layout": { "toolbox": "right", "palette": "bottom", "locked": true },
    "shown": { "status_bar": false, "zoom_toolbar": true },
    "toolbox": ["pick", "shape"],
    "toolbox_hidden": ["eraser"],
    "show_outline_flyout": false,
    "status_info": "CursorCoordinates"
  },
  "palette": { "show_no_color": true, "right_click_outline": true },
  "appearance": { "ui_size": 1.0, "desktop_rgb": [255, 255, 255], "page_border": true }
}
```

A file is never trusted:

- More than 4 MB, not JSON, a part of the wrong shape, another `kind`
  or version 0: refused, nothing changes. Later versions are read as far
  as their parts are understood; unknown fields are ignored.
- Shortcuts with an empty or overlong (over 64 characters) id or keys
  are dropped; the last keys given for an id win; at most 4096 are kept.
- Floating bar positions that are not finite or lie beyond 100000 points
  fall back to the default layout's; toolbox lists keep at most 256
  entries, and an overlong tool id counts as none.
- An interface size outside 50 % to 400 % becomes 100 %.

## Checks

- Given a workspace with custom shortcuts, bars on the right, the status
  bar hidden, palette options and an interface size of 125 %, when it is
  exported and imported into a fresh editor, then every part is the same.
- Given a file exported with shortcuts and palette only, when it is
  imported with only shortcuts checked, then the bars and palette stay as
  they were.
- Given garbage, an empty file, another kind, version 0 or a part of the
  wrong shape, when it is imported, then nothing changes and the status
  bar says it is not a workspace file.
- Given the status bar hidden and the Transform toolbar shown, when
  TraceDraw starts again, then they are still hidden and shown.
