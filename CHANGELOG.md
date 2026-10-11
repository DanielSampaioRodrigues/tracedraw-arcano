# Changes

Versions follow the tags of this repository; every push to `main` also
publishes a nightly build. [`docs/features.md`](docs/features.md) has the
full, row-by-row status.

## Unreleased

- Pick tool: Shift scales from the centre, Ctrl scales in 100 % steps
  (dragging across gives the mirror image at the same size), Alt frees a
  corner, Ctrl keeps a move straight, a right click during a drag leaves
  the original, Space drops a copy, Esc cancels; in rotate mode the side
  handles skew and the centre of rotation can be moved.
- Pick tool: Ctrl+click chooses an object inside a group (round handles;
  it moves, takes colours and is deleted on its own), Alt+click digs to
  the object beneath, a marquee with Alt takes what it touches, a click
  on any selected object or on the centre mark turns the handles, and
  the centre mark can be dragged. The Freehand Pick has the Pick tool's
  handles and takes what its lasso holds wholly (with Alt, what it
  touches).
- Curves: Ctrl keeps lines and Bezier nodes to steps of the constrain
  angle; with Freehand a click and then another click draw a straight
  line. A line drawn from (or to) an end node of the selected curve
  continues that curve, closes it when it meets the other end, and the
  pointer shows when it would. The curve tools' property bar has the
  outline width, line style, arrowheads and Close Curve, and Freehand
  its smoothing (also in Options > Tools > Curve tools).
- Drag a colour from the palette onto an object: inside it fills it, on
  its outline it colours the outline; onto the document palette it joins
  it.
- The Text tool's pointer is the I-beam only over text; elsewhere the
  cross hair with its icon. The Shape tool shows the resize, turning and
  slant arrows over the handles framing selected nodes, and its right
  click chooses the node or segment under the pointer before the menu
  opens. Interactive Fill shows the move arrows over the fill's handles.
- Scale handles snap to other objects, guidelines, the grid and the page.
- Paragraph text: the frame shows as a dashed line (Text > Show Text
  Frames), the handles sit on the frame, and scaling it with the Pick
  tool resizes the frame so the text flows anew instead of stretching
  the letters.
- Shape tool: dragging a segment bends it (a straight one becomes a
  curve; smooth nodes stay smooth); a click on a segment marks the spot
  where Add Node (or `+`) puts the new node; C, S and Y make the chosen
  nodes cusp, smooth or symmetrical.
- `+` duplicates the selection in place with the Pick tool; with the
  Shape tool and nodes chosen, `+` and `-` add and delete nodes.
- A double click on the drawing with the Rectangle tool no longer adds a
  page frame (double-click the tool's button for that).
- The pointer shows what a press would do: resize arrows on the handles,
  move arrows on objects and the centre mark, arrows across guidelines,
  turning and slant arrows in rotate mode,
  and the Shape tool's own arrow with a mark over nodes, control points
  and segments. Drawing tools show their icon by the cross hair; the
  Eraser and the brushes show their nib at its size.
- Interface size 100 % now draws menus, bars and text as large as other
  programs at 100 % (a quarter smaller than in 0.3.0 on a monitor the
  system scales by 125 %); drawings keep their real size at zoom 100 %.
- Shelf panel: keep objects for any drawing and after restarts, place
  them by double click or by dragging onto the page (Window > Dockers >
  Shelf, or Add to Shelf on the object menu).
- Options > Global: Printing and File Formats set what the Print and
  Export dialogs start with (and can remember the last ones), where
  imported objects go, Bitmap Tracing sets Quick Trace's preset, and
  Warnings turns messages on and off.
- Opening a `.cdr` file with objects that could not be read, or a drawing
  with fonts that are not installed, now says so in a message (with a
  Report button for the file).
- The browser demo's loading page says only that it is loading; the demo
  message shows each time the page opens.

## 0.3.0 (2026-10-11)

The first release meant for everyone to try. It is an alpha: real
use will turn up bugs, and "Known limits" below lists what is known.

**Working in the window**
- Several drawings open in tabs; a Create a New Document dialog.
- The toolbox, palette, toolbars and status bar dock on any side or
  float, and the window remembers where they were.
- Keyboard shortcuts of your own for every menu command (Options >
  Customization > Commands), shown in the menus.
- Workspace files: export and import shortcuts, bars and toolbox,
  palette options and appearance, part by part; the bars shown come back
  after a restart.
- The interface size follows the scaling the system gives each monitor,
  like other programs (100 % by default, 80 % to 200 % relative to it).
- A splash screen, an About box, an interface size option, tool cursors,
  a Hints panel with a page per tool, typed number fields and a status bar
  with object details and coordinates.
- Right-button drag of objects with a drop menu; menus switch as the
  pointer moves along the menu bar.

**Drawing and shaping**
- Shape tools draw a live preview while dragging; rectangle corners
  (round, scalloped, chamfered) per corner; complex stars.
- Common Shapes: 71 shapes in five sets (basic, arrows, flowchart,
  banners, callouts).
- Shape tool node transforms, reflection and segment copy; Corners, Join
  Curves and Coordinates panels.
- Snapping modes with marks and screen tips; a drawing scale and ruler
  calibration.
- Linked object styles and default properties for graphics, artistic text
  and paragraph text.

**Text**
- Drop caps sit on their lines and hyphenation breaks where it should;
  character offsets and angles; the Shape tool on text.

**Bitmaps**
- Colour modes, bitmap effects kept as an editable list apart from the
  pixels, image adjustments with tone curves, Straighten Image, Manually
  Inflate Bitmap and Convert to Bitmap options.
- Vector Mosaic and Picture Mosaic.

**Print and PDF**
- Preflight checks before printing; imposition and printers' marks.
- Publish to PDF with bookmarks, document properties, links and
  passwords (AES-256); protected PDFs open after asking for their
  password.
- Time Tracker: time spent on a drawing, task by task.

**Platforms**
- The whole editor runs in the browser (WebAssembly), as a demo that
  says what it leaves out and links to the desktop app.
- Installers for Windows, macOS and Linux on every release, a Microsoft
  Store package in the build, and code signing ready to switch on.

**Known limits**
- Downloads are not code-signed yet: Windows SmartScreen asks for "More
  info > Run anyway", and macOS needs right-click > Open the first time.
- The `.cdr` reader is written from public notes and sample files; some
  files open with objects missing or looking different. Reports with a
  file are very welcome (there is an issue template for them).
- Gaps are listed in [`docs/blueprint-gaps.md`](docs/blueprint-gaps.md).

## 0.1.0 (2026-10-09)

First tagged version: the document model and command engine, the desktop
editor, the `.cdr` reader and writer, SVG, PDF and the other formats,
effects, text and the command-line tool.
