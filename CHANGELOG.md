# Changes

Versions follow the tags of this repository; every push to `main` also
publishes a nightly build. [`docs/features.md`](docs/features.md) has the
full, row-by-row status.

## Unreleased

- With two or more objects selected (or a group, or a curve of several
  parts) the Pick tool's property bar shows Group, Ungroup, Ungroup All,
  Combine, Break Apart, the shaping operations (Weld, Trim, Intersect,
  Simplify, Front Minus Back, Back Minus Front, Boundary) and Align and
  Distribute.
- `.ai` files written as PostScript (the format's early versions) open
  through the PostScript interpreter instead of failing as PDF.
- `.cdr` files: fonts embedded in the file are used for its text when
  they are not installed (and their licence allows it), so text keeps its
  letters instead of falling back to another font; the drawing carries
  them, saved in `.tdraw` files, and the Fonts docker marks them
  "embedded". Coloured runs of text in files from recent versions take
  their colour (stored inline in those files).
- The missing-font check also looks at text inside groups, clip frames,
  table cells and symbols.
- `.cdr` files before version 12: text, font names and layer and object
  names written in Cyrillic, Greek, Turkish, Central European, Baltic,
  Hebrew, Arabic, Thai, Vietnamese, Japanese, Chinese or Korean read in
  their own code page (they came out as Western European letters).

- Ctrl+C, Ctrl+X and Ctrl+V copy, cut and paste objects in the desktop
  app (the windowing layer delivers them as clipboard events, not as
  key presses, so they did nothing); Ctrl+Shift+V pastes in the middle
  of the view.

## 0.4.0 (2026-10-11)

The Pick and Shape tools now behave as the keys and the pointer lead
you to expect: modifiers while dragging, choosing inside groups,
bending segments, a pointer that shows what a press will do.

**Selecting and transforming (Pick tool)**
- Shift scales from the centre, Ctrl scales in 100 % steps (dragging
  across gives the mirror image at the same size), Alt frees a corner,
  Ctrl keeps a move straight; a right click during a drag leaves the
  original, Space drops a copy, Esc cancels.
- In rotate mode the side handles skew, and the centre of rotation can
  be moved; it stays with the object (selecting it again finds it where
  it was put, and it moves with the object).
- Ctrl+click chooses an object inside a group (round handles; it moves,
  takes colours and is deleted on its own); Alt+click digs to the object
  beneath; a marquee with Alt takes what it touches.
- A click on any selected object, or on the centre mark, turns the
  handles; the centre mark can be dragged; scale handles snap to other
  objects, guidelines, the grid and the page; `+` duplicates in place.
- The Freehand Pick has the Pick tool's handles and takes what its lasso
  holds wholly (with Alt, what it touches).

**Curves and nodes**
- Shape tool: dragging a segment bends it (a straight one becomes a
  curve, smooth nodes stay smooth); a click on a segment marks where Add
  Node (or `+`) puts the new node; C, S and Y make the chosen nodes cusp,
  smooth or symmetrical; `-` deletes them; a right click chooses the
  node or segment under the pointer before the menu opens.
- A line drawn from (or to) an end node of the selected curve continues
  that curve, and closes it when it meets the other end.
- Ctrl keeps lines and Bezier nodes to steps of the constrain angle;
  with Freehand a click and then another click draw a straight line.
- The curve tools' property bar has the outline width, line style,
  arrowheads and Close Curve, and Freehand its smoothing (also in
  Options > Tools > Curve tools).

**The pointer**
- It shows what a press would do: resize arrows on the handles, move
  arrows on objects and the centre mark, turning and slant arrows in
  rotate mode, arrows across guidelines, the copy arrow when a move will
  leave the original.
- The Shape tool's own arrow marks nodes, control points and segments,
  and shows the resize, turning and slant arrows over the handles that
  frame selected nodes.
- Drawing tools show their icon by the cross hair, with a sign where a
  line would continue the selected curve or close the one being drawn;
  the Eraser and the brushes show their nib at its size; the Text tool
  shows the I-beam only over text; Interactive Fill shows the move
  arrows over the fill's handles.

**Text**
- Paragraph text frames show as dashed lines (Text > Show Text Frames;
  red with a down arrow when the text does not fit), the handles sit on
  the frame, and scaling it with the Pick tool resizes the frame so the
  text flows anew instead of stretching the letters. Lines that do not
  fit are no longer drawn over the others.

**Colour**
- Drag a colour from the palette onto an object: inside it fills it, on
  its outline it colours the outline; onto the document palette it joins
  it. Objects dropped on the document palette give it their colours.

**Interface**
- Interface size 100 % now draws menus, bars and text as large as other
  programs at 100 % (a quarter smaller than in 0.3.0 on a monitor the
  system scales by 125 %); drawings keep their real size at zoom 100 %.
- Touch workspace (Window > Workspace > Touch): a larger interface for
  touch screens; two fingers pan and pinch zooms.
- Shelf panel: keep objects for any drawing and after restarts, place
  them by double click or by dragging onto the page.
- Options > Global: Printing and File Formats set what the Print and
  Export dialogs start with (and can remember the last ones), where
  imported objects go, Bitmap Tracing sets Quick Trace's preset, and
  Warnings turns messages on and off.
- The status bar tells how far, how much and what angle while dragging.
- A double click on the drawing with the Rectangle tool no longer adds a
  page frame (double-click the tool's button for that).

**Files**
- Opening a `.cdr` file with objects that could not be read, or a
  drawing with fonts that are not installed, now says so in a message
  (with a Report button for the file).
- The browser demo's loading page says only that it is loading; the demo
  message shows each time the page opens.

**Known limits**
- Downloads are not code-signed yet: Windows SmartScreen asks for "More
  info > Run anyway", and macOS needs right-click > Open the first time.
- The `.cdr` reader is written from public notes and sample files; some
  files open with objects missing or looking different. Reports with a
  file are very welcome (there is an issue template for them).
- Gaps are listed in [`docs/blueprint-gaps.md`](docs/blueprint-gaps.md).

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
