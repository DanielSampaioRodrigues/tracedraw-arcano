# Changes

Versions follow the tags of this repository; every push to `main` also
publishes a nightly build. [`docs/features.md`](docs/features.md) has the
full, row-by-row status.

## Unreleased

- Selection handles: solid black squares just outside the selection box,
  which has no frame any more, so the objects' outlines stay in view; a
  handle scales from where it was grabbed. In rotate mode the arrows have
  solid heads and stand outside the box too.
- The Pick tool treats all objects as filled by default: a click inside
  an unfilled object selects it. Options > Tools > Pick and the property
  bar turn it off.
- Shape tool: a dragged node, control handle or shape node snaps to the
  grid, guidelines, the page and other objects like the drawing tools
  do; it ignored snapping (#3).
- Linux: after the splash screen the main window can be maximized and
  resized; it kept the splash screen's size as its maximum (#1).
- Linux: the interface is drawn a quarter larger at the default interface
  size (100 % is one interface point per pixel at the system's scaling);
  text was too small (#1). Tools > Options > Customization > Appearance >
  Interface size still sets it from 80 % to 200 %.
- Opening .cdr files: letters beyond Latin-1 (Czech, Polish, Turkish and
  other accented letters) in text of version 17 and later files read
  right; they showed as boxes and could break lines in the middle of a
  word. CMYK pictures show their own colours, converted through the
  colour profile the file carries; they looked inverted. Cut-out
  pictures keep their transparency instead of a white box. Paragraph
  text in a frame that was resized keeps its size and starts at the
  frame's top instead of being squashed or drawn above it. Rounded
  corners of rectangles in version 15 and later files have their real
  size, and shapes of version 14 and 15 files that have no outline of
  their own take their style's. Invert lenses of version 13 and later
  files show what lies beneath in its complementary colours instead of
  a solid box.
- Files with many large pictures open more than twice as fast (a 297 MB
  flyer: 20 to 8.6 seconds on two cores): pictures are decoded side by
  side and kept compressed more lightly, and a page keeps its decoded
  pictures between draws instead of decoding them again every time.
- Contour: the tool's drag draws a live contour (outside when dragged
  away from the object's middle, inside toward it); its bar used to wait
  for Apply and then made a static group of copies. The bar and the
  Contour panel share the controls: To centre, Inside, Outside, steps,
  offset, the corners (mitred, round or bevelled, round only before),
  the colour direction, the end colours and object and colour
  accelerations. To centre now spreads the colours over all its steps.
- Object > Copy Effect copies perspective, distortion, extrusion,
  contour, blend settings, lens, bevel and block shadow besides drop
  shadow, transparency and envelope; a distortion's centre, a
  perspective's corners and a vanishing point locked to the object sit
  the same way on the target.
- Blend keeps the end object as an object of its own: it can be moved,
  resized or recoloured and the steps follow, and selecting it picks the
  blend for the bar and panel. Blending used to delete the end object
  and keep a copy only inside the effect, so it could not be selected
  again; Clear Blend lost it.
- Segment Delete removes the stretch of an outline between the places
  where it crosses other outlines or itself, so overlapping lines can be
  trimmed back to their crossings; a drag removes every such stretch
  inside it. It used to remove one segment between two nodes, and a
  closed shape could lose its closing side by mistake.
- Crop: the area stays on the page with handles to move and resize it
  (and fields for its position and size) until a double-click inside,
  Enter or Crop applies it; Escape or Clear Crop Area drops it. It used
  to crop at once on release. Bitmaps are cropped (into a clip frame;
  Crop Bitmap led to a tool that skipped them), groups member by member
  (they were turned into one plain curve, losing their members), and
  open curves keep the parts inside instead of being closed.
- Extrude: six types (small and big, back and front, toward a vanishing
  point that can be locked to the object or the page, and back or
  front parallel), each with its depth; the body is coloured from the
  object, one solid colour or a shade per face, lit by one light. Only
  the faces that face the viewer are drawn, a curved side is one
  outlined surface instead of a row of slats, and every subpath is
  extruded (only the first was, so text and holes came out wrong). The
  tool's drag runs a parallel body along it or puts the vanishing point
  where it ends, its handle can be dragged, and the bar (its Apply used
  to add a separate static extrusion) and the Extrude panel share the
  same controls, showing and editing the selected extrusion.
- An outline that ends where it started (an ellipse) is treated as
  closed by Distort and Extrude.
- Distort: Push and Pull moves the nodes and bows the segments the other
  way (spikes or petals); it only scaled the object before. Zipper puts
  teeth on every segment, with Random, Smooth and Local; Twister takes
  whole turns, degrees and the direction. A drag is centred where it
  starts (the centre can be dragged afterwards), a Twister drag follows
  the turn of the pointer, and every distortion keeps the object's
  subpaths and open curves (they were merged into one closed outline).
  The bar shows and edits the selected distortion, with Center
  Distortion, Clear Distortion and Convert to Curves; its Apply button
  used to rewrite the object's outline instead of adding the effect.
- 3-point callout: a drag from the tip to the elbow works besides two
  clicks (the hint said so; only clicks were taken), the text goes on
  the side of the elbow it was placed on and opens with its sample text
  selected, so typing replaces it (it was typed after it). The leader
  and a new shape round the text (underline, rectangle, rounded
  rectangle or ellipse) follow the text as it is typed, resized or
  moved; a tip or elbow moved with the Shape tool stays. The tool has a
  property bar (outline width, line style, head at the tip, shape, gap),
  also for callouts selected with the Pick tool.
- Knife: a freehand cut besides the two-point line, a gap along the cut,
  and the pieces kept apart or as one object, from its property bar.
  Every separate piece becomes an object of its own, and open curves are
  split where the cut crosses them (they were closed into filled pieces).
- The Blend tool's bar sets the rotation and the object and colour
  accelerations besides the steps, for the next blend and live for a
  selected one, with Clear Blend and Break Apart. Dragging a slider in an
  effect panel (blend, contour, extrude and the others) or the drop and
  block shadow drags made an undo step per movement; a run of changes is
  now one step.
- Shape Recognition keeps shapes as drawn: a tilted rectangle or ellipse
  stays tilted (they came out level in the stroke's box), triangles and
  other four-sided shapes keep their corners (they became regular
  triangles and rectangles), and circles are told from squares by their
  area. Its property bar sets the recognition and smoothing levels;
  Sketch's sets joining to the selected curve, the distance and the
  smoothing.
- Connectors stay attached: move, scale or reshape an object and the
  connectors joined to it follow, in the same undo step; moved on its
  own, a connector is let go. They used to be plain lines. The connector
  tools have a property bar: straight, right angle or rounded (changing
  selected connectors too), the corner radius, and the outline of the
  selected ones.
- Dimensions: the tools have a property bar (precision, units, show
  units, prefix, suffix, text above, in or below the line, label size),
  shown also when dimensions are selected, and changing it redraws them.
  Dimensions are dynamic: scaled or skewed, they read the new length with
  upright text. The label is centred (it sat a quarter of the length to
  the left), extension lines run past the line on its side, and the one
  being placed follows the pointer. Parallel and Horizontal or Vertical
  take a drag as their hints said; Segment measures the segment clicked.
- The shaping brushes have their own bars: Attract and Repel, Smudge and
  Roughen showed only their name and could not change the nib size.
  Smooth has a rate, Smear and Smudge a pressure, Twirl a speed and its
  turn, Attract and Repel a speed and the mode, Roughen the spike
  frequency and size. A stroke is one undo step (two strokes in a row
  used to merge). The brushes work on each subpath they reach: before,
  brushing one part of a compound object joined all its parts into one
  outline, and Roughen closed open curves.
- Mesh Fill tool: its property bar sets the rows and columns (new meshes
  take them; the selected mesh changes and keeps its colours), Smooth
  colour, the selected node's transparency and Clear Mesh. A palette
  colour now colours the selected node; before, it replaced the whole
  mesh with a flat fill. Delete removes the grid lines through the
  selected node. Smooth colour now draws (it was only a checkbox): the
  colours flow through the nodes. The tool stays the Mesh Fill tool
  after making a mesh.
- Area Fill: the property bar chooses the fill (default, a colour, none)
  and the outline (default, a width and colour, none) of the objects it
  makes; objects with live effects count as drawn.
- Envelopes rebuilt. The Envelope tool keeps its own property bar
  (presets, the four modes, Add New, mapping, Keep Lines, Copy, Clear)
  instead of the distortion controls, and edits the frame itself. The
  modes now shape the frame (they were only buttons): Straight Line,
  Single Arc, Double Arc and Unconstrained, where the selected node's
  handles bend a segment alone. Double-clicking the frame adds a node,
  a node removes it. The mappings Horizontal, Original, Putty and
  Vertical change how the object fills the frame. Keep Lines and the
  mapping change the selected envelope at once. Envelopes from earlier
  files look as they did.
- Objects with live effects are picked and boxed where they are drawn:
  a click on the arch of an enveloped object selects it, and the
  selection box, the property bar's size and alignment take the warped
  outline and the objects the effects add.
- View > Proof Colors changes the drawing on screen: its colours go to
  CMYK and back, through the CMYK profile in use or the built-in model,
  so colours CMYK cannot print show duller. Before, the button only
  lit up.
- The object context menu's Frame Type works: Create Empty Clip Frame
  makes clip frames that stay in place when the object was turned (they
  were turned twice), No Frame turns empty clip frames back into their
  objects (it used to remove the outline), and Create Empty Text Frame
  replaces the object with an empty paragraph text frame of its size (it
  did nothing).
- Shaping: Trim keeps the objects that cut and Intersect keeps the
  originals, as their pictures always showed; Weld and the others
  replace them. The Shaping panel's "Leave original" boxes now do what
  they say (they were not used), each operation keeping its own choice,
  and the panel shows the operations as pictures with a large picture
  and a description of the one chosen.
- The Text docker has Character, Paragraph and Frame sections with
  header bars and buttons to open and close them (kept), the size list
  and bold, italic and underline as on the property bar, alignment
  pictures, and fields in rows. The new Frame section sets columns and
  their gutter, bullets, drop caps, hyphenation, fitting the text to the
  frame and where paragraph text sits from top to bottom: at the top, in
  the middle or at the bottom of its frame (new, saved with the drawing).
- Window > Cascade, Tile Horizontally and Tile Vertically put every open
  drawing in a window of its own, with a title bar, a button back to the
  tabs and a close button; the active one has its rulers and tools, the
  others show their page, and a click makes one active. Combine Windows
  and Dock Window bring the tabs back. Each drawing keeps its rendered
  page and its place in its window.
- Preferences are saved as they change (looked at once a second), not
  only when TraceDraw closes, so they last in the browser too (in its
  local storage) and after a crash. More of them are kept: the tool
  settings new objects take (polygon points, star sharpness, corners,
  spiral revolutions, table and graph paper size, eraser, text font and
  size, ...), what the window shows (grid, guidelines, page border,
  bleed, printable area, pixel and baseline grids, view mode, ...), all
  snapping toggles, and the Color and Align and Distribute dockers'
  choices.
- The Color docker has a color viewer (a saturation and brightness
  square beside a hue strip), the sliders and a palette's swatches, each
  shown or hidden by a button at the top; the selected object's colour
  sits beside the new one, and the new colour follows the selection. The
  HSB and HSL hue sliders count 0 to 360 degrees (they took the hue for
  a fraction of a turn before).
- The Align and Distribute docker has picture buttons. Align To (the
  active object, the page's edges, the page's centre, the grid) now does
  what it says, where before every choice aligned to the active object;
  Use Outline aligns the outer edges of the outlines. Distribute adds
  left, right, top and bottom edges to centres and gaps, and can spread
  the objects across the page (two objects are enough there); the
  Object menu lists the new ones too.
- The Transformations docker has picture buttons for its pages
  (Position, Rotate, Scale and Mirror, Size, Skew), a 3 x 3 reference
  point, Copies and one Apply: copies are made in one undo step, each the
  one before transformed again. Position and the rotation centre can be
  relative or absolute (from the ruler origin), the mirror buttons are
  applied with the scale, scale and size can stay proportional, skew can
  use the anchor point, and the size and absolute fields follow the
  selection. Alt+F8, Alt+F9 and Alt+F10 open Rotate, Scale and Mirror
  and Size as Alt+F7 opens Position, and Object > Transformations lists
  the five. Relative Position can be turned off again (it came back on).
- The Objects docker shows the page and the Master Page as a tree: the
  master layers, the guides (show, lock all) and the document grid (show)
  under the Master Page; each layer has a printer to turn printing and
  export on or off; the active layer, where new objects go, is in bold
  and is picked by clicking a layer (or an object on it); New Layer, New
  Master Layer (all pages, or odd or even from its context menu) and
  Delete (the selection, else the active layer) are at the bottom. A
  master layer can be moved, deleted and set to all, odd or even pages
  from its context menu, and its objects can be clicked, marquee
  selected and snapped to on every page it shows on.
- The Properties docker: its sections come open, in the order Outline,
  Fill, Transparency (new there: the Transparency tool's settings), the
  kind's own (corners, effects, character) and Summary, each with a header
  bar, and a row of buttons opens and closes them; the sections closed
  are kept. The fill kinds are pictures.
- The text property bar: the font size has a list of the usual sizes, the
  alignment is one button with its picture and a list, and Bullets, Drop
  Cap, Text Properties and Edit Text have buttons. The shape tools' bars
  (rectangle, ellipse, polygon, star) with an object selected end with
  Wrap Paragraph Text, To Front, To Back and Convert to Curves, as the
  Pick tool's bar does (Wrap Paragraph Text is new there too).
- The document palette stays a row under the drawing window wherever the
  colour palette is (on a side it was a column beside it). The colour
  palette starts at the bottom; moved to a side, along the top or
  floating, it stays there across restarts, in the browser too. Nightly
  builds that stood it at the right edge by default put it back at the
  bottom once.
- The browser version offers every family of the google/fonts collection
  (about 2,000 more), fetched from GitHub the first time a drawing's text
  needs one or the pointer rests on it in the font list; they show in
  the font lists with the download mark.
- A new font list in the property bar, the Text and Glyphs dockers and
  Options: type in the field to find a family (the name, then names
  starting with it, then the rest; Down, Up, Enter, Escape), the number
  of fonts and what to show (all, installed, in this drawing, to
  download), the five used last, every name drawn in its own font with
  an icon of how it is built (TrueType, OpenType, variable, came with the
  drawing, to download), the styles of each family under it, a slider for
  the sample size and a preview of "AaBbYyZz" with its guide lines.
- Dockers: the close button and a tab's Close Docker take the docker out
  of the tab column (Hints, Properties and Objects could not be closed),
  and the column with the docker in front is kept when TraceDraw closes,
  so a docker closed stays closed the next time. Help > Hints closes
  Hints when it is ticked.
- `.cdr` writing, made to match the sample files table by table: each
  page now lists the master page's guides, desktop and grid layers by the
  master layers' own GUIDs (they had GUIDs of their own, and the grid
  entry was missing), and the style table numbers its entries once and
  gives every style a fill, an outline, fonts for all three scripts,
  spacing and decoration settings that exist (styles named entry 0
  before). Text objects get numbers of their own and graphics a graphic
  style. These are the likeliest reasons written files closed the
  program that opened them.
- The browser version loads behind the desktop app's splash screen on
  the grey of the drawing window, with the download's progress in
  megabytes and percent on its bar, and under it what TraceDraw is (an
  open-source vector editor that opens `.cdr` files, alpha, written in
  Rust, MIT or Apache-2.0), which screens it suits and links to the
  desktop downloads, the code and the issues. Once up it simply runs:
  the demo message at the start, the menu bar's demo link and the
  Welcome Screen's demo note are gone.
- The `.cdr` issue form asks for the file's version and what the drawing
  uses (B-splines, lenses, live effects, pattern fills, paragraph
  settings, fitted text, pictures, pages).
- With two or more objects selected (or a group, or a curve of several
  parts) the Pick tool's property bar shows Group, Ungroup, Ungroup All,
  Combine, Break Apart, the shaping operations (Weld, Trim, Intersect,
  Simplify, Front Minus Back, Back Minus Front, Boundary) and Align and
  Distribute.
- `.ai` files written as PostScript (the format's early versions) open
  through the PostScript interpreter instead of failing as PDF.
- PostScript: `.prn` print files open and import (a printer job header
  before the program is skipped), multi-page `.ps` and `.prn` files open
  with one page per page of the file at its own size, and File > Export
  writes PostScript (`.ps`) and PRN (`.prn`) with every page.
- `.cdr` files: fonts embedded in the file are used for its text when
  they are not installed (and their licence allows it), so text keeps its
  letters instead of falling back to another font; the drawing carries
  them, saved in `.tdraw` files, and the Fonts docker marks them
  "embedded". Coloured runs of text in files from recent versions take
  their colour (stored inline in those files).
- The missing-font check also looks at text inside groups, clip frames,
  table cells and symbols.
- A missing font is drawn with a stand-in of the same widths when one is
  installed (Liberation Sans for Arial and Helvetica, Liberation Serif
  for Times New Roman, Liberation Mono for Courier New, Carlito for
  Calibri, Caladea for Cambria, Gelasio for Georgia, the TeX Gyre faces
  for the classic PostScript ones), else of the same style (serif,
  monospaced, casual or sans-serif), so text keeps its line breaks; the
  messages name the stand-in, as in "Arial (Liberation Sans)".
- `.cdr` files, checked against the previews files carry (public samples
  of versions 11 to 22 and files of the project's author): line breaks in
  artistic text of recent versions (they showed as boxes and the text ran
  on one line), centred lines, bold, semibold, italic and underlined runs
  from the style strings, rectangles of version 15 and later at their own
  size (the scale was applied twice) with corners in the right unit,
  polygons and stars at their size (they came out as a point), the size
  of each page, layer names, uniform transparency, and CMYK colours shown
  through the profile the file carries (the colours of the previews).
  Every face of an embedded family is used (a bold face next to the
  regular one), fonts are found by legacy names such as "Arial Narrow",
  and the palette keeps colours in the CMY, HSB, HLS, Lab and YIQ models.
- `.cdr` files: text fitted to a path follows its path when the two are
  not grouped (it was drawn straight, or off the page), at the offset and
  distance the file sets, mirrored text reading upright on the other side
  of the path. Objects of lists nested in a layer keep their stacking
  order.
- Text on a path: the letters stand on the path moved by the distance,
  spaced along it; on closed paths text runs on past the start, and
  mirrored text reads the right way round on the other side.
- Outlines thinner than a pixel are drawn one pixel wide on screen and in
  bitmap exports instead of fading out.
- `tracedraw-cli`: `TRACEDRAW_PAGE` chooses the page a bitmap or single-page
  export writes, and `TRACEDRAW_OBJECTS=1 tracedraw-cli info` lists every
  object the reader made.
- Letters a font lacks are drawn with a font that has them, so Chinese,
  Japanese, Korean, Arabic, Hebrew or Thai text set in a Latin font shows
  its letters instead of empty boxes.
- Variable fonts draw bold and italic text with their weight and slant
  axes instead of the default instance.
- The browser version offers about a hundred free font families (the
  stand-ins of the same widths for Calibri, Cambria, Georgia, Palatino,
  Century Gothic and others, popular families such as Open Sans, Roboto,
  Montserrat, Lato and Poppins, and fonts for Chinese, Japanese, Korean,
  Arabic, Hebrew and Thai). The font lists show them; a family's files
  are fetched the first time a drawing's text needs it.
- `.cdr` files before version 12: text, font names and layer and object
  names written in Cyrillic, Greek, Turkish, Central European, Baltic,
  Hebrew, Arabic, Thai, Vietnamese, Japanese, Chinese or Korean read in
  their own code page (they came out as Western European letters).
- File > Save As opens a Save Drawing dialog of its own: folder bar,
  shortcuts (home, desktop, documents, downloads, pictures, drives), the
  folder's contents with dates, types and sizes, file name, type, the
  `.cdr` version and the document's title, subject and rating, asking
  before replacing a file; in the browser it keeps the name, type,
  version and properties of the download.
- Save As `.cdr` writes any version from 7 to 22, chosen in that dialog
  (programs open files of their own version and older ones). Versions 7
  to 13 are RIFF files, 14 and 15 ZIP archives around one, 16 on ZIP
  archives with the chunk tree apart from its data, each with the
  records, tables, master page and layers of that version. A drawing
  opened from a `.cdr` file saves in its version unless another is
  chosen; Options > Global > File Formats sets the version of new
  drawings (12 at first). `tracedraw-cli` reads the
  version from `TRACEDRAW_CDR_VERSION` and the MCP `save` tool takes a
  `version`.
- `.cdr` writing: files of versions 21 and 22 carry the configuration
  record of those versions (page orientation, ruler origin, grid,
  units, nudge, duplicate offset, drawing scales) instead of an empty
  one, and every file has a font table for its styles.
- `.cdr` files: version 13 and later outlines read their nib angle and
  width scale from the right place, guide and grid layers stay hidden,
  and empty master layers no longer show up as layers of their own.
- File > Close All closes every drawing (it closed only the active one).
- AI Drawing (Tools > AI Drawing): describe what to draw or change and
  an AI service you set up with your own key (the Messages API, or any
  chat completions service, local model servers included) writes a
  script for the object model; it runs as one undo step, without file
  access and all or nothing, and a failing script is sent back once with
  its error. The key stays in memory unless you ask to remember it.
  Works on the desktop and in the browser. While a service is set up, a
  prompt bar above the colour palette takes requests (Enter draws) with
  a model list fetched from the service; Window > Toolbars hides it.
- Scripts: `CreatePath(d)` draws curves from SVG path data.
- AI agents can edit the drawings of the open window: with Options >
  Global > General > "Let AI agents on this computer edit the open
  drawing" on, an MCP client that starts `tracedraw --mcp-window` gets
  the tools of `tracedraw --mcp` on the window's drawings, with undo and
  shown at once. The window listens on this computer only, with a token.

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
