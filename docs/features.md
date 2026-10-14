# Features

TraceDraw's capabilities, grouped by area, each with a status. "works"
means it does what its page in `behavior/` describes, with a test or a
checked screenshot behind it.
"partial" names what is missing. "baked" means the result is applied once
instead of staying live and editable. The goal is every row at "works".

Counts: works 184, partial 3 (`.cdr` content coverage; Extrude's 3D rotation, several lights and bevels; CMX import), missing 1 (system windows), not applicable 4, icons close but not pixel-identical.

## 1. Selection and navigation

| Capability | Status |
|---|---|
| Pick: click, shift-click, marquee, Alt-marquee (touch), Alt-click dig, Ctrl-click inside groups (round handles; move, colour and delete one object of a group), Tab cycling | works |
| Second click rotate/skew handles: rotate about a movable centre of rotation, skew with the side handles, Ctrl steps, Shift about the centre line | works (see `behavior/pick-tool.md`) |
| Handle modifiers: Shift scales from the centre, Ctrl in 100 % steps (mirror at the same size), Alt frees a corner; Ctrl keeps a move straight; right click during a drag leaves the original; Space drops a copy; Esc cancels | works |
| Drag a palette colour onto an object (inside: fill, on the outline: outline colour, No Color removes) or onto the document palette (see `behavior/color-palettes.md`) | works |
| Pointer shapes by what is under it: resize arrows on handles, move arrows on the selection, turning and slant arrows in rotate mode; the Shape tool's arrow marks nodes, control points and segments; drawing tools show their icon by the cross hair, the Eraser and brushes their nib (see `behavior/toolbox.md`) | works |
| Right-button drag of objects: blue outline preview, then Move Here, Copy Here, Copy Fill Here, Copy Outline Here, Copy All Properties, Place Inside, Cancel (see `behavior/pick-tool.md`) | works |
| Freehand Pick (lasso, Alt for touching, Ctrl for a rectangle, the Pick tool's handles) | works |
| Selection groups 0 to 9: Ctrl+number stores the selection, the number selects it again (twice: and zooms to it), Alt+number adds to it; saved with the drawing (see `behavior/pick-tool.md`) | works |
| Free Transform (rotation, angle reflection, scale, skew modes, Apply to Duplicate) | works |
| Zoom tool, zoom levels, zoom to page/fit/selected, marquee zoom, Shift+F4 | works |
| Pan, mouse wheel, Ctrl and Shift wheel | works |
| Rulers with numbers from the origin without sign, the unit name, an origin button and pointer markers; draggable origin, double-click for ruler settings, guidelines from rulers, Guidelines panel; all 13 units | works |
| Grid: document (lines or dots, spacing or frequency, thinned when dense, through the origin), pixel (Pixels view from 800%, colour and opacity), baseline (spacing, start from top, colour); snapping with threshold | works |
| Snap to objects, page, guidelines, dynamic and alignment guides; Alt+Q snap off, Alt+Z objects, Ctrl+Y grid | works |
| Snapping modes (node, intersection, midpoint, quadrant, tangent, perpendicular, edge, center, text baseline) with snap location marks and screen tips; moves carry the grabbed point (Options > Snapping) | works (see `behavior/snapping.md`) |
| View modes: Wireframe, Normal, Enhanced, Pixels; full-screen preview (F9); preview selected only | works |
| Page sorter view | works |
| Navigator and page tabs | works |

## 2. Drawing

| Capability | Status |
|---|---|
| Rectangle, 3-point rectangle, page frame on double-click; Ctrl square, Shift from centre | works (see `behavior/shape-tools.md`) |
| Rectangle corners: Round, Scalloped, Chamfered; a size per corner; Edit Corners Together; Relative Corner Scaling (property bar, Properties panel, Options > Toolbox > Rectangle tool) | works (see `behavior/shape-tools.md`) |
| Ellipse, 3-point ellipse, pie and arc | works |
| Polygon, Star, Complex star (crossing sides, Star and Complex Star buttons on the Star tool's bar), Spiral (symmetric, logarithmic), Graph paper, Common shapes (71 in five sets: basic shapes, arrows, flowchart, banners, callouts; picked from a grid of outlines), Action Lines | works (see `behavior/shape-tools.md`) |
| Freehand, 2-point line, Bezier, Pen, B-spline, Polyline, 3-point curve; lines from an end node of the selected curve continue it (join, close, merge subpaths); Freehand smoothing; outline, arrowheads and Close Curve on the property bar | works (see `behavior/curve-tools.md`) |
| Shape Recognition (lines, rectangles and ellipses at any tilt, triangles, four-sided shapes; recognition and smoothing levels) and Sketch (stroke joining with distance and smoothing) (see `behavior/curve-tools.md`) | works |
| Brush Strokes: Preset, Brush, Sprayer, Calligraphic, Expression | works |
| Dimensions: parallel, horizontal/vertical, angular, segment (a click on a segment); live preview; the bar's precision, units, prefix, suffix, text position and size; dynamic (drawn again when moved or scaled) (see `behavior/dimensions.md`) | works |
| 3-point callout: a drag or two clicks for the leader, a click for the text (typed at once); its bar's outline width, line style, head, shape round the text (underline, rectangle, rounded, ellipse) and gap; leader and shape fitted to the text as it is typed or moved (see `behavior/dimensions.md`) | works |
| Connectors: straight, right-angle, rounded right-angle, attached (they follow the objects they join), with a bar (kind, corner radius, outline); Anchor Editing tool (add, move, delete anchors; connectors snap to them) (see `behavior/connectors-and-anchors.md`) | works |
| Text (artistic and paragraph), Table | works |
| Browser version (WebAssembly, WebGL 2): opens, edits and exports documents with uploads and downloads | works |
| Import of HPGL plotter files (PLT) and export of pages as pen strokes | works |
| Import of TXT, RTF and DOCX as paragraph text with run formatting | works |
| Export to PSD with one raster layer per document layer | works |
| Text editing in place: caret, selection by keyboard and mouse, word selection, character-level bold/italic/underline, clipboard | works |
| Crop (an area to move and resize before it is applied; open curves cut, groups member by member, bitmaps, tables and symbols in a clip frame), Knife (two-point or freehand cuts, a gap, pieces apart or as one object, open curves split), Segment delete (the stretch between crossings, by a click or every one inside a drag), Eraser (band subtraction with thickness and nib shape) | works (see `behavior/crop-knife-eraser.md`); the crop area does not turn |
| QR code, barcode (Code 128, EAN-13, EAN-8) and page number insertion | works |

## 3. Shape and node editing

| Capability | Status |
|---|---|
| Shape tool: move nodes and handles, marquee select nodes, Elastic mode (farther nodes move less) | works |
| Shape tool node selection (Ctrl+click toggles, Shift+click selects a run, Tab and Shift+Tab, rectangular or freehand marquee, double-click the tool selects all) | works (see `behavior/shape-tool.md`) |
| Shape tool node transforms: Stretch and Scale, Rotate and Skew handles around the selected nodes; Reflect nodes horizontally and vertically | works (see `behavior/shape-tool.md`) |
| Shape tool segments: Ctrl+C, Ctrl+X, Ctrl+D copy, cut and duplicate the segments between selected nodes; Extend Curve to Close; Curve smoothness slider | works (see `behavior/shape-tool.md`) |
| Shape tool on rectangles (corner nodes, all or one corner), ellipses (pie inside, arc outside) and polygons (mirrored nodes, twisted stars) | works (see `behavior/shape-tool.md`) |
| Node types: cusp, smooth, symmetrical; to line / to curve; reverse subpaths | works |
| Add, delete, join, break, extract subpath, close curve, align nodes, reduce nodes | works |
| Weld, Trim, Intersect, Simplify, Front minus back, Back minus front, Boundary (menu, panel and the property bar with several objects selected); Trim keeps the sources, Intersect keeps both; the panel's leave-original choices per operation (see `behavior/shaping.md`) | works |
| Combine, Break apart, Group, Ungroup, Ungroup all | works |
| Join Curves panel: Extend, Chamfer, Fillet, Bezier Curve joints, gap tolerance, nearest ends first, closes subpaths | works (see `behavior/corners-and-joins.md`) |
| Corners panel: fillet, scallop, chamfer (A and B distances) the corners of curves or of chosen nodes, with preview; corners that do not fit are skipped | works (see `behavior/corners-and-joins.md`) |
| Coordinates panel: rectangle, square, ellipse, circle, polygon, regular polygon, star, complex star, 2-point line and multipoint curve from typed coordinates (origin point, size, angle, bounding box, bounding circle, points list), set interactively by clicking or dragging, live preview, Create object and Replace object | works (see `behavior/coordinates.md`) |
| Convert to curves, Convert outline to object | works |
| Smear, Twirl, Attract, Repel, Smudge, Roughen, Smooth brushes, each with its bar (nib size; rate, pressure, speed and turn, attract or repel, spike frequency and size), one undo step per stroke, per subpath (see `behavior/brushes.md`) | works |
| Lock, hide, order (front/back of page and layer, one step, in front of, behind, reverse) | works |
| Align and Distribute: six aligns to the active object, the page's edges, its centre or the grid, with Use Outline; eight distributes (edges, centres, gaps along x and y) between the outermost objects or across the page; picture buttons in the panel (see `behavior/align-distribute.md`) | works |
| Transformations panel: Position, Rotate, Scale and Mirror, Size and Skew pages (Alt+F7 to Alt+F10, Object > Transformations) with a 3 x 3 reference point, relative or absolute position and centre, mirror buttons, proportional scale and size, skew about the anchor point, and copies in one undo step (see `behavior/transformations.md`) | works |
| Step and Repeat | works |
| Align with pixel grid | works |

## 4. Text

| Capability | Status |
|---|---|
| Artistic text and paragraph text frames (dashed frames, Show Text Frames, resizing a frame with the Pick tool reflows the text), in-place editing | works (see `behavior/text.md`) |
| Font family, size, bold, italic, alignment, underline, strikethrough | works |
| Properties docker: sections in the order Outline, Fill, Transparency, the kind's own, Summary, open by default with header bars and a row of buttons, closed ones kept; fill kinds as pictures; the Transparency tool's settings in its own section | works (see `behavior/workspaces.md`, Properties docker) |
| Font list: type to find a family, count and filter (all, installed, in this drawing, to download), the five used last, names drawn in their own fonts with an icon of how each is built, styles under each family, sample size slider, preview with guide lines | works (see `behavior/text.md`, Font list) |
| Leading, paragraph spacing, indents, tabs, columns, bullets, drop cap; vertical alignment of paragraph text in its frame (top, middle, bottom); Text docker with Character, Paragraph and Frame sections (see `behavior/text.md`, Text docker) | works |
| Tracking, baseline shift, OpenType features (ligatures, small caps, old-style figures, fractions, swash) | works |
| Character horizontal and vertical offsets and angle; character and word spacing; Shape tool character nodes (drag to shift) and interactive horizontal and vertical spacing arrows; Straighten Text and Align to Baseline for characters | works (see `behavior/text.md`); `.cdr` character offsets are not read yet |
| Hyphenation (Liang patterns en/pt, heuristic elsewhere) | works |
| Text on path (offset, distance, mirror), straighten text | works |
| Fit text to frame, wrap paragraph text around objects | works |
| Insert Placeholder Text: filler that fills each selected paragraph frame (see `behavior/text.md`) | works |
| Linked paragraph text frames (link, unlink, live re-flow) | works |
| Change case, insert formatting codes, show non-printing characters, Encode, Make Text Web Compatible | works |
| Text statistics, Find and Replace, Glyphs panel, font filter and sample | works |
| Writing tools: spell check (system Hunspell word lists), grammar, thesaurus (built-in or MyThes file), Autocorrect | works |
| Missing-font substitution with report: stand-ins drawn to the same widths (Liberation for Arial, Times and Courier, Carlito for Calibri, Caladea for Cambria, ...), else of the same style, named in the report | works (see `behavior/text.md`) |
| Letters a font lacks drawn with a font that has them (Chinese, Japanese, Korean, Arabic, Hebrew, Thai, Indic and symbols included); variable fonts drawn at their weight and slant; fonts found by their legacy family names too ("Arial Narrow", "Arial Black") | works (see `behavior/text.md`) |
| Browser build: about a hundred free font families in a font folder, fetched the first time text needs one | works (see `behavior/web.md`, `web/FONTS.md`) |
| Fonts carried by a drawing (embedded in an opened `.cdr`, saved in `.tdraw`): used for its text when not installed and the licence allows it | works (see `behavior/text.md`) |
| Convert text to table and table to text | works |
| Table: cell typing, Tab navigation, insert/delete rows and columns, merge/split, distribute; fill and border on the property bar | works |

## 5. Fills and outlines

| Capability | Status |
|---|---|
| Uniform fill in RGB, CMYK, Gray, HSB, HSL, Lab, YIQ, registration | works |
| Gradient fill: linear, radial, conical, square; multi-stop; angle, offset, edge pad; interactive drag | works |
| Pattern fill: two-colour (8 tiles), bitmap, full-colour vector (tile from the selection); texture fill (clouds, marble, noise, wood) | works; PostScript fills not planned (legacy) |
| Mesh fill: Mesh Fill tool and bar (rows and columns keeping colours, Smooth colour, node transparency, Clear Mesh), nodes dragged and coloured from the palette, grid lines added and removed (see `behavior/mesh-fill.md`) | works |
| Area Fill (enclosed region; fill and outline: default, own or none) (see `behavior/area-fill.md`) | works |
| Interactive Fill tool: property bar with fill type and gradient fields, axis and centre handles on the canvas | works |
| Outline: width, colour, caps, joins, dashes, nib, behind fill, scale with object, arrowheads (presets and custom from the selection) | works |
| Outline Pen and Outline Color (hidden flyout, Options toggle); calligraphic nib rendered and exported | works |
| Eyedroppers: colour (bitmap sampling, fill or outline) and attributes (properties, transformations, effects groups) | works |

## 6. Interactive effects

| Capability | Status |
|---|---|
| Drop shadow (offset, opacity, feather, colour), live | works |
| Contour (to centre, inside, outside; steps, offset, mitred, round or bevelled corners, colour direction, end colours, object and colour accelerations), live; the tool's drag and bar edit it like the panel (see `behavior/contour.md`) | works |
| Blend (steps, rotation, accelerations, path), live, both objects kept and the steps following either (see `behavior/blend.md`) | works |
| Distort (push and pull of the nodes, zipper with random, smooth and local, twister with turns and direction), live; a drag centred where it starts, the centre marker dragged, the bar editing the selected distortion; Center, Clear, Convert to Curves (see `behavior/distort.md`) | works |
| Envelope, live: Envelope tool and panel; modes Straight Line, Single Arc, Double Arc, Unconstrained (handles); mappings Horizontal, Original, Putty, Vertical; Keep Lines; presets; nodes added and removed; Add New, Copy, Clear (see `behavior/envelope.md`) | works |
| Perspective (Add Perspective, node drag) | works |
| Extrude, live: six types (small and big back and front toward a vanishing point locked to the object or the page, back and front parallel), depth, colour from the object, solid or a shade, one light; hidden faces left out, curved sides as one outlined surface, every subpath extruded; the tool's drag and handle, a bar and the panel editing the selected extrusion (see `behavior/extrude.md`); 3D rotation, several lights and bevels not done | partial |
| Bevel (soft edge, emboss) | works |
| Block shadow (depth, direction, colour, gap on the property bar) | works |
| Transparency: uniform, gradient, pattern, texture; merge modes; fill/outline/all | works |
| Lens (11 types, frozen, rate, palette rotation) | works |
| clip frame (place inside, extract, empty frame, text frame, edit in place, lock contents) | works |
| Symbols (Object > Symbol menu, create, insert, revert, Symbols panel, Ctrl+F3) | works |
| Copy effect (drop shadow, transparency, envelope, perspective, distortion, extrude, contour, blend settings, lens, bevel, block shadow; pinned points carried by the box), Clone effect (shadow and transparency follow the source until edited), Clear effect, Clear transformations, Symmetry (live, 1 to 12 mirror lines) | works (see `behavior/copy-effect.md`) |
| Rollover | not applicable: web-page interactivity with no HTML export target |

## 7. Colour

| Capability | Status |
|---|---|
| Colour models with conversions; Colour panel with a saturation and brightness viewer and hue strip, sliders per model, palette swatches, old and new colour, gamut warning, following the selected fill (see `behavior/color-docker.md`) | works |
| Palettes: the default CMYK palette (99 named colours), document palette (filled as colours are applied, eyedropper, add from selection or document, delete, reset; read from `.cdr`), scroll arrows and wheel, show all colours, click and hold for shades, Ctrl+click to mix, open/save, palette editor, palette manager, from document/selection; Options > Customization > Color Palette | works |
| Colour styles and harmonies | works |
| Object styles (fill, outline and transparency parts; objects stay linked and follow the style; break link) and default properties of new graphics, artistic text and paragraph text, saved with the drawing | works (see `behavior/object-styles.md`) |
| Overprint fill, outline and bitmap; Simulate Overprints preview | works |
| Proof colours (soft proofing: the page drawn through CMYK and back, with the CMYK profile in use or the built-in model; see `behavior/colour-management.md`) | works |
| CMYK shown through a built-in press model of coated stock when no profile is loaded (default palette within 2.5 Delta E); RGB to CMYK with black-only greys and pure black | works |
| ICC colour management (v2/v4 profiles, matrix/TRC and LUT, four intents, black point compensation, gamut check); PDF/X output intent embedding | works |
| A drawing's own CMYK profile (the one a `.cdr` file carries, saved in `.tdraw`): its CMYK colours shown through it while it is the active drawing | works (see `cdr-format.md`) |
| Separations | works (PDF separations export) |

## 8. Bitmaps and images

| Capability | Status |
|---|---|
| Import PNG, JPEG, BMP, GIF, WebP, TIFF; crop, resample | works |
| Straighten Image: correct lens distortion, rotate -15 to 15 degrees, vertical and horizontal perspective, quarter turns, grid with size and colour, zoom and pan, Crop image, Crop and resample to original size | works (see `behavior/straighten-image.md`) |
| Convert to bitmap (resolution list and value, colour mode, dithered, always overprint black, anti-aliasing, transparent background, uncompressed size) | works (see `behavior/bitmap-modes.md`) |
| Colour monochrome bitmaps from the palette (click: background, right-click: foreground) | works (see `behavior/bitmap-modes.md`) |
| Bitmap colour modes: Black and White (line art, ordered, halftone, cardinality-distribution, Jarvis, Stucki, Floyd-Steinberg), Grayscale, Duotone (one to four inks with tone curves, load and save), Paletted (uniform, VGA, adaptive, optimized, grayscale, system, document palette; ordered and error-diffusion dithering), RGB, Lab, CMYK; dialogs with before and after previews | works (see `behavior/bitmap-modes.md`); no overprint tab or range sensitivity |
| Bitmap effects, 98 in fifteen groups: Adjust (Auto Adjust, Image Adjustments, Contrast Enhancement, Local Equalization, Target Color Balance, Tone Curve, Brightness/Contrast/Intensity, Color Balance, Gamma, Hue/Saturation/Lightness, Selective Color, Replace Colors, Desaturate, Channel Mixer), Transform, Correction, 3D Effects, Art Strokes, Blur, Camera, Color Transform, Contour, Creative, Custom, Distort, Noise, Sharpen, Texture; each with its settings dialog and before and after previews; Ctrl+B, Ctrl+Shift+B, Ctrl+Shift+U; the Image Adjustments (rotate, pan, zoom, three preview modes, Auto adjust, white and black point pickers, histogram, undo, redo, reset, snapshots); Tone Curve styles (curve, straight, freehand, gamma), eyedropper nodes, Auto Balance Tone; Contrast Enhancement's interactive histogram and channel list; Target Color Balance channel list; eyedroppers for sample colours | works (see `behavior/bitmap-effects.md`); results close in look, not pixel-identical; Tone Curve presets are JSON files |
| Effects kept apart from the pixels: Properties panel FX section (show or hide, edit, reorder, delete, add effect), original kept until Flatten Effects; Auto inflate bitmaps for effects (Document Options and Bitmaps > Inflate Bitmap) | works (see `behavior/bitmap-effects.md`); reordering with buttons instead of dragging |
| Colour modes: 1-bit, grayscale, RGB, CMYK | works |
| Bitmap colour mask | works |
| Inflate bitmap: Auto Inflate Bitmap switch, Manually Inflate Bitmap (Inflate to pixels, Inflate by percent, Maintain aspect ratio) | works (see `behavior/bitmap-effects.md`) |
| Bitmap tracing: quick, centreline, outline; presets; smoothing, detail, colour count; before and after preview in the dialog (see `behavior/bitmap-tracing.md`) | works |
| Edit bitmap in the system editor, linked bitmaps (update from link, break link) | works |

## 9. Layout and document

| Capability | Status |
|---|---|
| Multiple pages, insert/duplicate/rename/delete/move, go to page, page sorter | works |
| Page size presets and orientation, custom sizes, bleed, printable area | works |
| Page background: solid or bitmap | works |
| Master layers (all, odd, even pages; their objects selectable on the pages they show on), layer visibility, lock, printable | works |
| Objects panel: the page and the Master Page (master layers, guides, document grid) as a tree; layers with collapse arrow, eye, printer, padlock and layer icon; the active layer in bold, picked by a click (new objects go there; a click on an object makes its layer active); objects by name (or what they are) with a fill and outline swatch; groups open to their objects; double-click renames a layer or an object; New Layer, New Master Layer (all, odd or even pages) and Delete at the bottom (see `behavior/objects-panel.md`) | works |
| Document properties and metadata, rendering resolution, baseline grid | works |
| Guidelines panel, presets (seven presets, user-defined margins, columns, grid), angled guides, per-guideline colour, style and lock, rotate handles, drag off to delete, right-click Lock/Unlock, Select All > Guidelines | works |
| Insert page number (active layer, all, odd, even) | works |
| Templates: save as template, new from template | works |

## 10. Import and export

| Capability | Status |
|---|---|
| Open `.cdr`: RIFF (versions 3 to 13), ZIP of versions 14 and 15, ZIP with redirected chunks (version 16 on) | works (layouts confirmed against the public format description; real-file corpus still being collected) |
| `.cdr` content: shapes, curves (with their node types), paths, polygons and stars (scaled base shapes), groups, bitmaps (cropped ones as clip frames), rectangles with their corners, text (artistic, paragraph, fitted to a path with its offset, distance and mirroring; line breaks, alignment, weights, italic, underline from style strings), line spacing and indents, fills, outlines, uniform transparency, preset and custom arrowheads, opacity, embedded fonts (EOT, not compressed, every face), the drawing's CMYK profile, page sizes per page, layer names, CMYK and cut-out pictures, invert lenses (version 13 on) | partial: splines, vector pattern and PostScript fills, tabs/bullets/drop caps from style tables, lenses other than invert, effects |
| Native `.tdraw` save/load | works |
| MCP server (`tracedraw --mcp`): open, new, save, export, run_script, document_info, undo, redo over JSON-RPC; the same tools on the open window's drawings through `tracedraw --mcp-window` when Options > Global > General lets agents in (local port, token) | works (see `behavior/mcp.md`) |
| Save Drawing dialog (folder bar, shortcuts, folder list, file name, type, `.cdr` version, title, subject, rating, replace question; in the browser name, type, version and properties); Save As `.cdr` in a chosen version, 7 to 22 (a drawing opened from a `.cdr` keeps its version; Options > Global > File Formats sets the one of new drawings): pages, layers, rectangles, ellipses, curves (with their node types), groups, text, bitmaps, solid and gradient fills, outlines, transparency | works (round trip in every version; see `cdr-format.md`, Writing, and `behavior/files.md`) |
| Import SVG and SVGZ (groups, clips, gradients, images); open SVG as a document | works |
| Export SVG, PDF, AI (PDF-compatible), EPS, PostScript and PRN (every page), DXF, EMF, WMF, HTML (inline SVG, page tabs), PNG, JPEG, WebP, GIF, BMP, TIFF; Send To (Desktop, Documents, mail) as PDF; bitmaps, clip frames and bitmap transparency in SVG and PDF | works |
| Export for Web and Office presets | works |
| Export dialog with a preview, the size in pixels and an estimated file size (see `behavior/export.md`) | works |
| PDF: fills rasterised when needed, arrowheads, separations, PDF/X-1a, X-3 and X-4 with output intent, flattening and bleed | works |
| Import PDF and AI (PDF-compatible): paths, images, text, clips, shadings, patterns, forms, annotations; protected files of every revision (RC4, AES-128, AES-256), asking for the password when one is needed | works (see `behavior/pdf-import.md`) |
| Links: a web address, e-mail address or page for any object (Links and Bookmarks panel), exported as link areas in PDF and as links around the object in SVG and HTML; named objects as bookmarks in the panel | works (see `behavior/links.md`) |
| Publish to PDF dialog: pages (all, current, selection), PDF/X level, bleed, title, author, subject and keywords, a bookmark per page, password protection (AES-256: a password to open, a password for the permissions to print, copy and change) | works (see `behavior/publish-to-pdf.md`) |
| Import DXF (lines, polylines, arcs, ellipses, splines, hatches, text, blocks) | works (see `behavior/dxf.md`) |
| Import EPS, PostScript and PRN print files (interpreter: paths, images, text, clips, shadings, patterns, prologs; one page per `showpage` at its `/PageSize`), and `.ai` files written as PostScript (the format's versions before the PDF-based ones) | works (see `behavior/eps-import.md`) |
| Import PSD/PSB (layers as bitmaps with opacity, visibility and masks) | works (see `behavior/psd-import.md`) |
| Import EMF and WMF (GDI paths, pens, brushes, text, DIB bitmaps, clips); export EMF and WMF | works (see `behavior/emf-wmf.md`) |
| Import CMX: pages, layers, groups, curves, rectangles, ellipses, uniform and fountain fills, outlines with dashes and arrowheads, embedded 24-bit images, clips, text (from the drawn characters); not yet lenses and other fill types | partial (see `behavior/cmx-import.md`) |
| Acquire image (scanner) | not applicable: no scanner stack in pure Rust; import the scanned file instead |

## 11. Productivity and customisation

| Capability | Status |
|---|---|
| Undo/redo with history panel, repeat (Ctrl+R) | works |
| Copy, paste, paste in view, Paste Special (system clipboard as text, bitmap or objects), duplicate, clone, copy properties from | works |
| Find and Replace (text and object attributes) | works |
| Scripts panel: JavaScript object model, run, record macro, load/save | works |
| AI Drawing panel and prompt bar (above the palette while a service is set up; Window > Toolbars; model list fetched from the service): a request in words drawn by an AI service the user sets up (Messages API or any chat completions service, local model servers included) with their own key; the reply's script runs without file access, all or nothing, as one undo step; a failing script is sent back once with its error | works (see `behavior/ai-drawing.md`) |
| Workspaces: Default, Lite, Classic, Illustration, Page Layout, Touch (the interface 1.4 times larger, two-finger pan and pinch zoom); toolbar toggles | works (see `behavior/workspaces.md`) |
| Options dialogs: Tools > Options > TraceDraw (General with start-up and undo levels, Display, Edit with constrain angle and precision, Nodes and Handles, clip frame auto-centre, Snapping radius, Save with backups and auto-backup, Text), Customization (Appearance with the interface size and desktop colour, Commands, Command Bars), Tools (Pick, Zoom/Pan, Curve tools, shape tools, Eraser), Global, Workspaces; Layout > Document Options (General with Fill open curves, Page Size, Layout, Background, Bleed, Rulers, Grid, Guidelines, Save as Default); Cancel restores | works |
| Options > Global > Printing (what the Print dialog starts with, remember the last print, preflight in the dialog), File Formats (what Export starts with, remember the last export, imported objects centred or as in the file), Bitmap Tracing (Quick Trace preset, what every trace starts with), Warnings (unread objects, missing fonts, Shelf removal; each message can be turned off) | works (see `behavior/options.md`) |
| Options pages for pen pressure, tablet mode, VBA and plug-ins | not applicable: no pen pressure input, own workspaces, JavaScript scripting, no plug-in host |
| Save settings as default | works |
| Preferences kept across restarts and saved as they change (desktop file, browser local storage): Options, bars, dockers, units, snapping, tool settings, what the window shows, dockers' choices (see `behavior/workspaces.md`, Kept across restarts) | works |
| Hints panel with per-tool help; Welcome Screen (recent drawings with thumbnails, templates, news, learn) | works |
| Border and Grommet (large-format finishing) | works |
| Vector Mosaic (vector mosaics): density, scale, screen angle, keep original, limit colors, uniform, opacity and luminosity tracking, merge adjacent, weld adjacent overlap, circle, square and custom tiles | works (see `behavior/vector-mosaic.md`); made at once rather than in the background |
| Picture Mosaic (bitmap mosaics from an image library folder): columns and rows from the shape, blending, duplicates with tile spacing, Single, Stack and Array composition, Stretch and Remove edges, document, custom, tile and output size priorities, keep original | works (see `behavior/picture-mosaic.md`); a library is one folder, read on worker threads |
| Bitmap plug-ins (third-party filters) | not applicable: no plug-in host; the built-in bitmap effects cover the stock filters |
| User interface in 18 languages with system fallback fonts | works |
| Keyboard shortcuts (every menu shortcut bound; Esc closes dialogs) | works (see `behavior/shortcuts.md`) |
| Shortcuts of your own for every menu command (Options > Customization > Commands: search, press the keys, conflicts named, shown in the menus) | works (see `behavior/shortcuts.md`) |
| Workspace export and import (`.tdws`: shortcuts, bars and toolbox, palette options, appearance; part by part) and the bars shown kept across restarts | works (see `behavior/workspaces.md`) |
| Reset all settings: Options > Global, F8 while starting, `--reset-settings` (see `behavior/about-and-startup.md`) | works |
| Shelf: objects kept for any drawing and after restarts (Add Selection, Add to Shelf on the object menu), placed by double click or by dragging onto the page, renamed, reordered and removed; symbols become objects, linked styles go along (see `behavior/shelf.md`) | works |
| Time Tracker: tasks per drawing saved in `.tdraw` files, a clock that stops after a set time without input, toolbar with the active task, its time and the total, dialog to add, rename, delete and edit tasks (time, start, finish), CSV and text export | works (see `behavior/time-tracker.md`) |

## 12. Print and prepress

| Capability | Status |
|---|---|
| Print (PDF to the system viewer), copies, range, bleed; imposition (as in document, several pages per sheet, booklet, step and repeat) on fitted or paper sheets; printers' marks (crop marks at every cut, registration targets, colour bars, densitometer scale, page information), mirror and invert of the whole sheet | works (see `behavior/printing.md`) |
| Print preview | works |
| Print merge from CSV (create/load, edit, perform) | works |
| Separations and overprint simulation | works |
| Preflight: thirteen checks (empty or overflowing text, missing fonts, small and rich black text, RGB in CMYK, objects off the page, low and very high bitmap resolution, thin outlines, white overprint, empty pages, objects that will not print) with thresholds, a list that selects the object, and a summary in the Print dialog | works (see `behavior/preflight.md`) |

## Workspace look

| Element | Status |
|---|---|
| Menu bar, standard toolbar (icons, Open/Undo/Redo lists, zoom box, view toggles, Snap Off, Snap To, Launch), property bar (stacked fields, page bar, object bar with object origin, shape parts, outline part, zoom bar), toolbox with flyouts, rulers, document tabs, bottom palette, panel tab strip, navigator, status bar | works |
| Icons | painted vector icons, one per tool, close in style; not pixel-identical |
| Panels (37): Properties, Objects, Hints, Transformations, Undo, Align and Distribute, Shaping, Step and Repeat, Text, Glyphs, Colour, Colour Styles, Object Styles, Find and Replace, Scripts, Palettes, Lens, Blend, Contour, Envelope, Extrude, Bevel, Brush Strokes, Bitmap Mask, Object Data, Links, Symbols, Pages, Guidelines, Fonts, Corners, Join Curves, Coordinates, Vector Mosaic, Picture Mosaic, Shelf, AI Drawing | works |
| Welcome Screen with tabs | works |
| About box with the banner, icon, facts (author, license, source code, co-author) and copyright; opens from the Welcome Screen too (see `behavior/about-and-startup.md`) | works |
| Splash screen while starting (artwork, status, progress), then the main window maximized or at its kept size; can be turned off (see `behavior/about-and-startup.md`) | works |
| Interface size relative to the system scaling of the monitor: 80 % to 200 %, 100 % by default (see `behavior/options.md`) | works |
| Drawing scale (typical scales 1:1 to 1:10000, 2:1 to 10:1, inch-to-foot scales, or any page and world distance): rulers, coordinates, sizes, offsets, grid, guidelines and dimension labels in world units; ruler calibration so zoom 100% is real size (see `behavior/rulers-grid-guidelines.md`, `behavior/options.md`) | works |
| Movable bars: the toolbox and the colour palette dock on any edge or float, the standard toolbar and the property bar dock at the top or the bottom or float, the status bar sits at the top or the bottom; dragging a grip or a floating title shows where the bar goes; the bar menu (right-click) has its places, the bars to show, Lock Toolbars and Reset Bar Positions; the layout is kept (see `behavior/toolbars.md`) | works |
| Right-click context menus (object, node, page, table) | works |
| Dialogs: New Document, Options, Export, Print, Print Merge, Colour Management, Font Manager, Document Properties, Convert to Bitmap, Resample, Trace, QR Code, Barcode, Change Case, Text Statistics, Tabs, Columns, Bullets, Drop Cap, Table create/split, Page Number Settings, Paste Special, Symmetry, Thesaurus, Grammar, Autocorrect, Encode, Border and Grommet | works |
| Several open drawings in tabs (unsaved marker, close button, New after the last tab), Window list, Close, Close All, exit asks about each unsaved drawing | works (see `behavior/documents.md`) |
| Window > Cascade, Tile Horizontally, Tile Vertically, Combine Windows, Dock Window: every drawing in a window with a title bar, the active one with its rulers and tools, the others showing their page as left (see `behavior/drawing-window.md`, Windows) | works |
| Window > New Window: another window (tab) of the same drawing, titled "name:1", "name:2"; each window has its own view, page and selection, the drawing, its undo history and its file are shared; closing one window keeps the others (see `behavior/drawing-window.md`, New Window) | works |
| A drawing in a system window of its own | missing |
