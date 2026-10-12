//! Application state. The UI modules under `ui/` draw it; `interaction.rs`
//! turns pointer and keyboard input into commands. Every document change
//! goes through `Engine::run`, so the UI stays thin and replayable.

use crate::theme;
use crate::tools::Tool;
use crate::view::View;
use egui::Pos2;
use std::path::PathBuf;
use tracedraw_core::{
    document::{paper, Shape, ShapeKind, TextSpan},
    geometry::{Affine, Point, Rect, Size, Vec2},
    Color, Command, Engine, Fill, LayerId, PageId, ShapeId, Stroke,
};

/// What a drag with the Free Transform tool does. Modifier keys override
/// the mode: Alt scales, Ctrl skews, Shift reflects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FreeTransformMode {
    Rotation,
    Reflection,
    Scale,
    Skew,
}

impl FreeTransformMode {
    pub const ALL: [FreeTransformMode; 4] = [
        FreeTransformMode::Rotation,
        FreeTransformMode::Reflection,
        FreeTransformMode::Scale,
        FreeTransformMode::Skew,
    ];

    pub fn label_key(self) -> &'static str {
        match self {
            FreeTransformMode::Rotation => "toolbar.ft_rotation",
            FreeTransformMode::Reflection => "toolbar.ft_reflection",
            FreeTransformMode::Scale => "toolbar.ft_scale",
            FreeTransformMode::Skew => "toolbar.ft_skew",
        }
    }
}

/// How inserted page numbers look and where they go.
#[derive(Debug, Clone, PartialEq)]
pub struct PageNumberSettings {
    pub start_at: i64,
    pub prefix: String,
    pub suffix: String,
    /// 0 arabic, 1 upper roman, 2 lower roman, 3 upper alpha, 4 lower alpha.
    pub style: u8,
    /// 0 bottom centre, 1 bottom outer corner, 2 top centre, 3 top outer corner.
    pub position: u8,
    pub size_pt: f64,
}

impl Default for PageNumberSettings {
    fn default() -> Self {
        PageNumberSettings {
            start_at: 1,
            prefix: String::new(),
            suffix: String::new(),
            style: 0,
            position: 0,
            size_pt: 12.0,
        }
    }
}

impl PageNumberSettings {
    /// Text for the page at `index` (0-based).
    pub fn label(&self, index: usize) -> String {
        let n = self.start_at + index as i64;
        let body = match self.style {
            1 => roman(n),
            2 => roman(n).to_lowercase(),
            3 => alpha(n),
            4 => alpha(n).to_lowercase(),
            _ => n.to_string(),
        };
        format!("{}{}{}", self.prefix, body, self.suffix)
    }
}

fn roman(mut n: i64) -> String {
    if n <= 0 {
        return n.to_string();
    }
    const T: [(i64, &str); 13] = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut out = String::new();
    for (v, s) in T {
        while n >= v {
            out.push_str(s);
            n -= v;
        }
    }
    out
}

fn alpha(n: i64) -> String {
    if n <= 0 {
        return n.to_string();
    }
    // A..Z, then AA, AB... like spreadsheet columns.
    let mut n = n - 1;
    let mut out = Vec::new();
    loop {
        out.push((b'A' + (n % 26) as u8) as char);
        n /= 26;
        if n == 0 {
            break;
        }
        n -= 1;
    }
    out.iter().rev().collect()
}

thread_local! {
    /// Dots per inch of the pixel unit: the active drawing's resolution.
    static PIXEL_DPI: std::cell::Cell<f64> = const { std::cell::Cell::new(96.0) };
}

/// Set the resolution pixels are measured at (each frame, from the active
/// drawing; the Create a New Document dialog uses its own while drawn).
pub fn set_pixel_dpi(dpi: f64) {
    if dpi.is_finite() && dpi > 0.0 {
        PIXEL_DPI.with(|c| c.set(dpi));
    }
}

/// The resolution pixels are measured at.
pub fn pixel_dpi() -> f64 {
    PIXEL_DPI.with(|c| c.get())
}

thread_local! {
    /// World millimetres per page millimetre: the active drawing's scale
    /// (Document Options > Rulers > Drawing scale), 1 at 1:1.
    static DRAWING_SCALE: std::cell::Cell<f64> = const { std::cell::Cell::new(1.0) };
}

/// Set the drawing scale distances are shown at (each frame, from the
/// active drawing). Values that are not positive and finite mean 1:1.
pub fn set_drawing_scale(world_per_page: f64) {
    let s = if world_per_page.is_finite() && world_per_page > 0.0 {
        world_per_page
    } else {
        1.0
    };
    DRAWING_SCALE.with(|c| c.set(s));
}

/// The drawing scale: world millimetres per page millimetre.
pub fn drawing_scale() -> f64 {
    DRAWING_SCALE.with(|c| c.get())
}

/// Units of measure, in the order the Units list shows them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Units {
    Inches,
    #[default]
    Millimeters,
    Picas,
    Points,
    Pixels,
    Ciceros,
    Didots,
    Feet,
    Yards,
    Miles,
    Centimeters,
    Meters,
    Kilometers,
}

/// One didot point, mm.
const DIDOT_MM: f64 = 0.376_065;

impl Units {
    pub fn id(self) -> &'static str {
        match self {
            Units::Inches => "in",
            Units::Millimeters => "mm",
            Units::Picas => "pc",
            Units::Points => "pt",
            Units::Pixels => "px",
            Units::Ciceros => "cc",
            Units::Didots => "dd",
            Units::Feet => "ft",
            Units::Yards => "yd",
            Units::Miles => "mi",
            Units::Centimeters => "cm",
            Units::Meters => "m",
            Units::Kilometers => "km",
        }
    }
    pub fn from_id(id: &str) -> Self {
        Units::ALL
            .into_iter()
            .find(|u| u.id() == id)
            .unwrap_or(Units::Millimeters)
    }
    pub const ALL: [Units; 13] = [
        Units::Inches,
        Units::Millimeters,
        Units::Picas,
        Units::Points,
        Units::Pixels,
        Units::Ciceros,
        Units::Didots,
        Units::Feet,
        Units::Yards,
        Units::Miles,
        Units::Centimeters,
        Units::Meters,
        Units::Kilometers,
    ];
    pub fn label(self) -> String {
        crate::i18n::tr(match self {
            Units::Inches => "units.inches",
            Units::Millimeters => "units.millimeters",
            Units::Picas => "units.picas",
            Units::Points => "units.points",
            Units::Pixels => "units.pixels",
            Units::Ciceros => "units.ciceros",
            Units::Didots => "units.didots",
            Units::Feet => "units.feet",
            Units::Yards => "units.yards",
            Units::Miles => "units.miles",
            Units::Centimeters => "units.centimeters",
            Units::Meters => "units.meters",
            Units::Kilometers => "units.kilometers",
        })
    }
    pub fn short(self) -> &'static str {
        match self {
            Units::Inches => "\"",
            Units::Millimeters => "mm",
            Units::Picas => "pc",
            Units::Points => "pt",
            Units::Pixels => "px",
            Units::Ciceros => "c",
            Units::Didots => "dd",
            Units::Feet => "ft",
            Units::Yards => "yd",
            Units::Miles => "mi",
            Units::Centimeters => "cm",
            Units::Meters => "m",
            Units::Kilometers => "km",
        }
    }
    /// Millimetres per unit. A pixel is one dot at the active drawing's
    /// resolution (see [`set_pixel_dpi`]).
    pub fn mm(self) -> f64 {
        match self {
            Units::Inches => 25.4,
            Units::Millimeters => 1.0,
            Units::Picas => 25.4 / 6.0,
            Units::Points => 25.4 / 72.0,
            Units::Pixels => 25.4 / pixel_dpi(),
            Units::Ciceros => 12.0 * DIDOT_MM,
            Units::Didots => DIDOT_MM,
            Units::Feet => 304.8,
            Units::Yards => 914.4,
            Units::Miles => 1_609_344.0,
            Units::Centimeters => 10.0,
            Units::Meters => 1000.0,
            Units::Kilometers => 1_000_000.0,
        }
    }
    pub fn from_mm(self, v: f64) -> f64 {
        v / self.mm()
    }
    pub fn to_mm(self, v: f64) -> f64 {
        v * self.mm()
    }
    /// Page millimetres per unit of a distance in the drawing: [`mm`]
    /// through the drawing scale (a world metre at 1:100 is 10 page mm).
    ///
    /// [`mm`]: Units::mm
    pub fn world_mm(self) -> f64 {
        self.mm() / drawing_scale()
    }
    /// A distance in the drawing (a position, a size, an offset, a length)
    /// from page millimetres to this unit, through the drawing scale. Page
    /// sizes, outline widths and text sizes use [`from_mm`] instead: they
    /// are as printed.
    ///
    /// [`from_mm`]: Units::from_mm
    pub fn world_from_mm(self, v: f64) -> f64 {
        v / self.world_mm()
    }
    /// A distance in the drawing typed in this unit, as page millimetres.
    pub fn world_to_mm(self, v: f64) -> f64 {
        v * self.world_mm()
    }
}

/// What the modifier keys do while a handle of the selection is dragged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HandleMods {
    /// Shift: from the centre of the selection instead of the opposite
    /// side.
    pub from_center: bool,
    /// Ctrl: scales in whole steps of 100 % (so crossing the other side
    /// mirrors at the same size), skews in steps of the constrain angle.
    pub steps: bool,
    /// Alt: corner handles scale freely instead of keeping the
    /// proportions.
    pub free: bool,
}

impl HandleMods {
    pub fn of(mods: egui::Modifiers) -> Self {
        HandleMods {
            from_center: mods.shift,
            steps: mods.ctrl,
            free: mods.alt,
        }
    }
}

/// Which handle of the selection box the pointer grabbed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Handle {
    N,
    S,
    E,
    W,
    NE,
    NW,
    SE,
    SW,
}

impl Handle {
    pub fn is_corner(self) -> bool {
        matches!(self, Handle::NE | Handle::NW | Handle::SE | Handle::SW)
    }
}

#[derive(Debug, Clone)]
pub enum Drag {
    None,
    /// Drawing a new box shape from `start` (page space); with Shift the
    /// box grows from `start` as its centre.
    Box {
        start: Point,
        current: Point,
        from_center: bool,
    },
    /// Base segment of a 3-point rectangle, ellipse or curve.
    ThreePointBase {
        start: Point,
        current: Point,
    },
    /// Moving the selection.
    Move {
        /// Where the drag began.
        start: Point,
        last: Point,
        total: Vec2,
        start_bounds: Rect,
        /// The selection's snap point under the pointer when the move
        /// began: it snaps to other objects' points.
        source: Option<Point>,
    },
    /// Scaling the selection with a handle; `anchor` stays fixed (the
    /// centre with Shift).
    Scale {
        handle: Handle,
        anchor: Point,
        start_bounds: Rect,
        current: Point,
        mods: HandleMods,
    },
    /// Skewing the selection with a side handle in rotate mode: the
    /// opposite side stays (the centre line with Shift).
    Skew {
        handle: Handle,
        start_bounds: Rect,
        start: Point,
        current: Point,
        mods: HandleMods,
    },
    /// Moving the centre of rotation in rotate mode.
    Pivot,
    /// Rotating the selection about `center`.
    Rotate {
        center: Point,
        start_angle: f64,
        current_angle: f64,
    },
    /// Rubber-band selection; with `touching` (Alt) objects it touches
    /// count too.
    Marquee {
        start: Point,
        current: Point,
        touching: bool,
    },
    /// Freehand Pick lasso; with `touching` (Alt) objects it touches count
    /// too.
    Lasso {
        points: Vec<Point>,
        touching: bool,
    },
    /// Freehand stroke being drawn.
    Freehand {
        points: Vec<Point>,
    },
    /// Dragging the selected nodes (Shape tool).
    Node {
        last: Point,
    },
    /// Dragging a control handle (Shape tool).
    Handle {
        shape: ShapeId,
        index: usize,
        which: tracedraw_core::nodes::Which,
    },
    /// Dragging a node of a rectangle, ellipse or polygon (Shape tool):
    /// the object as it was when the drag began; `single` changes one
    /// rectangle corner only; `begun` once the first step is recorded.
    KindNode {
        shape: ShapeId,
        node: crate::kind_nodes::KindNode,
        start: Box<tracedraw_core::Shape>,
        single: bool,
        begun: bool,
        /// Degrees an ellipse's node has turned through.
        turn: f64,
    },
    /// Moving the selected nodes (Shape tool): the curves as they were,
    /// where the drag began and the node it began on.
    NodeMove {
        start: Vec<crate::node_edit::StartCurve>,
        from: Point,
        grab: Point,
        begun: bool,
    },
    /// Bending a segment (Shape tool): the curve as it was when the drag
    /// began (local path, closed flag), the segment (the element ending
    /// it), the parameter of the point grabbed, where the drag began
    /// (page); `begun` once the first step is recorded.
    SegmentBend {
        shape: ShapeId,
        path: tracedraw_core::BezPath,
        closed: bool,
        index: usize,
        t: f64,
        from: Point,
        begun: bool,
    },
    /// Dragging a node transform handle (Shape tool).
    NodeTransform {
        handle: Handle,
        start: Vec<crate::node_edit::StartCurve>,
        bounds: Rect,
        from: Point,
        begun: bool,
    },
    /// Moving chosen characters of a text (Shape tool): the text as it
    /// was, the characters, where the drag began.
    TextChars {
        start: Box<tracedraw_core::Shape>,
        chars: Vec<usize>,
        from: Point,
        begun: bool,
    },
    /// Dragging a text's spacing arrow (Shape tool): horizontal for
    /// character (or, with `word`, word) spacing, else line spacing.
    TextSpacing {
        start: Box<tracedraw_core::Shape>,
        horizontal: bool,
        word: bool,
        from: Point,
        begun: bool,
    },
    /// Freehand selection of nodes (Shape tool).
    NodeLasso {
        points: Vec<Point>,
    },
    /// Rubber-band selection of nodes (Shape tool).
    NodeMarquee {
        start: Point,
        current: Point,
    },
    /// Interactive fill: linear gradient from start to current.
    FillGradient {
        shape: ShapeId,
        start: Point,
        current: Point,
    },
    /// Moving a fountain handle of the selected object (Interactive Fill).
    FountainHandle {
        handle: crate::fill_tool::FountainHandle,
    },
    /// Zoom tool rubber band.
    ZoomBox {
        start: Point,
        current: Point,
    },
    /// Contour tool drag sets the offset.
    ContourDrag {
        start: Point,
    },
    /// Connector tool drag from one object to another.
    Connector {
        from: ShapeId,
        start: Point,
        current: Point,
    },
    /// Moving a custom connector anchor (Anchor Editing tool).
    Anchor {
        shape: ShapeId,
        index: usize,
    },
    /// Dragging a drop shadow offset (Drop Shadow tool).
    Shadow {
        shape: ShapeId,
        start: Point,
    },
    /// Dragging a selection across the text being edited (Text tool).
    TextSelect,
    /// Dragging a paragraph text frame (Text tool).
    TextFrame {
        start: Point,
        current: Point,
    },
    /// Dragging a new guideline out of a ruler.
    NewGuide {
        horizontal: bool,
        pos: Point,
    },
    /// Dragging an existing guideline: where the press was, the guideline
    /// as it was and as it is now (committed on release).
    MoveGuide {
        index: usize,
        press: Point,
        start: tracedraw_core::document::Guide,
        guide: tracedraw_core::document::Guide,
    },
    /// Turning a guideline about its pivot with a rotation handle.
    RotateGuide {
        index: usize,
        pivot: Point,
        guide: tracedraw_core::document::Guide,
    },
    /// Dragging the ruler origin out of the corner between the rulers.
    RulerOrigin {
        pos: Point,
    },
}

/// Bezier / polyline tool in progress.
#[derive(Debug, Clone, Default)]
pub struct CurveInProgress {
    /// Nodes placed so far with their outgoing handle (dragged with the
    /// Bezier/Pen tool); the incoming handle mirrors the previous node's.
    pub nodes: Vec<(Point, Option<Point>)>,
    /// Polyline tools make straight segments; curve tools make curves.
    pub smooth: bool,
    /// Pointer is being dragged to set the handle of the last node.
    pub dragging_handle: bool,
}

impl CurveInProgress {
    /// Path through the nodes, optionally extended to a preview point.
    pub fn path(&self, preview: Option<Point>) -> tracedraw_core::BezPath {
        let mut path = tracedraw_core::BezPath::new();
        let mut nodes: Vec<(Point, Option<Point>)> = self.nodes.clone();
        if let Some(p) = preview {
            nodes.push((p, None));
        }
        for (i, (p, _)) in nodes.iter().enumerate() {
            if i == 0 {
                path.move_to(*p);
                continue;
            }
            let (prev, prev_out) = nodes[i - 1];
            let cur_in = nodes[i].1.map(|out| *p - (out - *p));
            match (self.smooth, prev_out, cur_in) {
                (false, _, _) | (true, None, None) => path.line_to(*p),
                (true, po, ci) => path.curve_to(po.unwrap_or(prev), ci.unwrap_or(*p), *p),
            }
        }
        path
    }
}

/// Artistic text being typed.
pub use crate::text_editing::TextEdit;

#[derive(Debug, Clone, Default)]
pub struct Clipboard {
    pub shapes: Vec<Shape>,
    /// The object styles of the drawing copied from, so that pasted
    /// objects keep their links only to the same styles.
    pub styles: Vec<tracedraw_core::ObjectStyle>,
}

/// Centres of rotation the user moved. One object's is kept in the
/// object's own space, so it travels with the object when it is moved,
/// scaled or turned; that of several objects together as a place within
/// their bounds (keyed by the objects, in id order). Both stay after the
/// selection changes, so selecting the same objects again finds them.
#[derive(Debug, Clone, Default)]
pub struct Pivots {
    pub single: std::collections::HashMap<ShapeId, Point>,
    pub several: std::collections::HashMap<Vec<ShapeId>, (f64, f64)>,
}

pub struct App {
    pub engine: Engine,
    pub page: PageId,
    pub view: View,
    pub tool: Tool,
    pub previous_tool: Tool,
    pub selection: Vec<ShapeId>,
    /// Second click on a selected object switches to rotate/skew handles.
    pub rotate_mode: bool,
    /// Where the user moved the centre of rotation (otherwise the
    /// selection's centre), kept for each object and each set of objects
    /// after the selection changes (see `Pivots`).
    pub pivots: Pivots,
    pub drag: Drag,
    /// A right click during a move, scale, rotation or skew: on release
    /// the change goes to copies and the originals stay.
    pub drag_copy: bool,
    pub curve: Option<CurveInProgress>,
    pub text_edit: Option<TextEdit>,
    pub file: Option<PathBuf>,
    /// The `.cdr` version the active drawing is saved in: the version of
    /// the `.cdr` file it came from, or the one last chosen when saving
    /// it. `None`: the version of Options > Global > File Formats.
    pub cdr_version: Option<u16>,
    pub status: String,
    pub fit_pending: bool,
    pub units: Units,
    pub nudge_mm: f64,
    pub show_rulers: bool,
    pub show_grid: bool,
    pub show_status_bar: bool,
    pub show_dockers: bool,
    pub docker_tab: DockerTab,
    pub flyout_open: Option<usize>,
    /// The tool each toolbox group shows: the one last used from its flyout.
    pub toolbox_last: Vec<Tool>,
    pub clipboard: Option<Clipboard>,
    pub duplicate_offset: Vec2,
    // Defaults for new objects (no fill, black hairline).
    pub polygon_points: u32,
    pub star_sharpness: f64,
    /// Star tool: draw complex stars (sides crossing), their points and
    /// sharpness.
    pub star_complex: bool,
    pub complex_points: u32,
    pub complex_sharpness: u32,
    /// Corners of new rectangles (style, sizes, relative scaling).
    pub rect_corners: tracedraw_core::Corners,
    /// The property bar's lock: one corner size edits them all.
    pub corners_together: bool,
    /// The Shape tool's chosen rectangle corner: dragging it changes that
    /// corner only.
    pub rect_corner_selected: Option<(tracedraw_core::ShapeId, usize)>,
    pub ellipse_arc: Option<tracedraw_core::EllipseArc>,
    pub contour_steps: u32,
    pub contour_offset: f64,
    pub contour_direction: crate::tools2::ContourDirection,
    pub contour_color: Color,
    pub spiral_revolutions: u32,
    pub spiral_logarithmic: bool,
    pub common_shape: crate::tools2::CommonShape,
    pub table_rows: u32,
    pub table_cols: u32,
    /// Cell fill and border of the next table drawn.
    pub table_fill: Fill,
    /// Block shadow colour and gap for the next drag.
    pub block_shadow_color: Color,
    pub block_shadow_gap: f64,
    pub table_border: Option<Stroke>,
    pub graph_rows: u32,
    pub graph_cols: u32,
    pub action_lines_count: u32,
    pub action_lines_radial: bool,
    /// 3-point tools: the base segment already dragged, waiting for the third click.
    pub three_point_base: Option<(Point, Point)>,
    pub media_width: f64,
    pub media_angle: f64,
    pub dimension_points: Vec<Point>,
    pub blend_steps: u32,
    pub extrude_depth: Vec2,
    pub distort_mode: crate::tools2::DistortMode,
    pub distort_amount: f64,
    pub distort_frequency: u32,
    pub brush_radius: f64,
    /// Eraser nib: thickness in mm and shape.
    pub eraser_width: f64,
    pub eraser_square: bool,
    pub shadow_default: tracedraw_core::Shadow,
    pub text_font: String,
    pub text_size_pt: f64,
    pub text_bold: bool,
    pub text_italic: bool,
    pub text_underline: bool,
    pub text_align: tracedraw_core::TextAlign,
    pub eyedropper_color: Option<Color>,
    pub eyedropper_attrs: Option<crate::eyedropper::SampledAttrs>,
    pub eyedropper_groups: crate::eyedropper::AttrGroups,
    /// Apply mode (after a sample) versus Select mode.
    pub eyedropper_apply: bool,
    /// Bitmap sample box side in pixels: 1, 2 or 5.
    pub eyedropper_sample: u32,
    pub canvas_rect: egui::Rect,
    pub pointer_page: Option<Point>,
    /// The palette shown in the bottom strip (the first visible one).
    pub palette: Vec<(String, Color)>,
    /// First swatch shown in the bottom strip (its scroll position).
    pub palette_scroll: usize,
    /// The same for the document palette row.
    pub doc_palette_scroll: usize,
    /// The document palette swatch clicked last (Delete color acts on it).
    pub doc_palette_current: Option<usize>,
    /// Pop-up of a palette colour's shades (click and hold a swatch): the
    /// colour and where the swatch is.
    pub palette_shades: Option<(Color, egui::Pos2)>,
    /// All colours of a palette at once (the expand button): which row.
    pub palette_expanded: Option<crate::ui::palette::Row>,
    pub raster: std::cell::RefCell<crate::raster::Raster>,
    /// Whether each paragraph text overflows its frame, by document
    /// revision (laying text out every frame would be slow).
    pub overflow_cache: std::cell::RefCell<(u64, std::collections::HashMap<ShapeId, bool>)>,
    pub wireframe: bool,
    /// Families the font lists show: installed, carried by drawings, and
    /// (browser build) offered by the page's font folder.
    pub font_families: Vec<String>,
    /// `tracedraw_text::fonts().generation()` when `font_families` was
    /// taken (see `App::sync_fonts`).
    pub font_generation: u64,
    /// The carried CMYK profile the colour engine was installed with
    /// (its data's address), see `App::sync_document_colors`.
    pub color_profile_key: Option<usize>,
    pub transform_tab: TransformTab,
    /// Selected nodes: (shape, element index).
    pub node_selection: Vec<(ShapeId, usize)>,
    /// The Shape tool's spot on a segment (a click there): the curve, the
    /// segment (the element ending it) and where along it; Add Node puts
    /// the node there.
    pub segment_spot: Option<(ShapeId, usize, f64)>,
    pub snap: crate::snap::SnapSettings,
    /// The Corners docker's operation and sizes.
    pub corners: crate::corners::CornersSettings,
    /// The Join Curves docker's joint, gap tolerance and radius.
    pub join_settings: crate::corners::JoinSettings,
    /// Outlines a docker previews on the canvas this frame (page space).
    pub docker_preview: Vec<tracedraw_core::geometry::BezPath>,
    /// A point a docker marks on the drawing (the Coordinates docker's
    /// origin point), page space.
    pub docker_preview_point: Option<Point>,
    /// The point the pointer snapped to this frame, for its mark.
    pub snap_mark: std::cell::Cell<Option<crate::snap_points::Target>>,
    pub dialog: crate::ui::dialogs::Dialog,
    /// Layout > Page Number Settings.
    pub page_numbers: PageNumberSettings,
    /// Last loaded print-merge data, for Edit / Perform without reloading.
    pub merge_state: Option<crate::ui::dialogs::PrintMergeState>,
    pub show_welcome: bool,
    /// Object > Clip to Frame > Place Inside Frame is waiting for a click on the frame.
    pub pending_clip_frame: bool,
    /// Clip frame being edited in place: (frame id, content ids).
    pub clip_frame_edit: Option<(ShapeId, Vec<ShapeId>)>,
    pub show_guides: bool,
    /// Selected guidelines of the active page (indices).
    pub selected_guides: Vec<usize>,
    /// The guideline showing rotation handles (second click on it).
    pub guide_rotate: Option<usize>,
    /// Rotation centre of that guideline, where it was clicked.
    pub guide_pivot: Point,
    /// The Guidelines docker's entry fields.
    pub guide_form: crate::ui::layout_options::GuideForm,
    /// Document Options > Guidelines presets section.
    pub guide_presets: crate::ui::layout_options::PresetForm,
    /// Document Options > Grid shows lines per unit instead of spacing.
    pub grid_frequency: bool,
    pub transform_values: [f64; 4],
    // ----- added with the full menu structure -----
    pub settings: crate::settings::Settings,
    pub welcome_tab: WelcomeTab,
    pub fullscreen_preview: bool,
    pub preview_selected_only: bool,
    pub page_sorter: bool,
    pub view_mode: ViewMode,
    pub proof_colors: bool,
    pub show_page_border: bool,
    pub simulate_overprints: bool,
    pub rasterize_complex_effects: bool,
    pub show_bleed: bool,
    pub show_printable_area: bool,
    pub show_pixel_grid: bool,
    pub show_baseline_grid: bool,
    pub options_page: crate::ui::dialogs::OptionsPage,
    /// What an open options dialog may change, for its Cancel button.
    pub options_snapshot: Option<crate::ui::options::OptionsSnapshot>,
    /// Options > Customization > Commands: the commands listed and the one
    /// waiting for its keys.
    pub shortcut_editor: crate::ui::options::ShortcutEditor,
    /// A menu command to run this frame (its key), from a shortcut of the
    /// user's.
    pub pending_command: Option<String>,
    /// When the last auto-backup ran (set on the first frame).
    pub last_auto_backup: Option<web_time::Instant>,
    /// Page size changes apply to every page (the page bar's All pages).
    pub page_size_all: bool,
    /// The property bar's object origin: (column, row) of the 3 x 3
    /// reference point selector, (1, 1) the centre.
    pub object_origin: (u8, u8),
    /// The property bar keeps width and height proportional.
    pub scale_locked: bool,
    pub pending_copy_properties: bool,
    pub pending_copy_effect: Option<EffectKind>,
    pub pending_clone_effect: Option<EffectKind>,
    /// Document Options > Page Size state.
    pub options_labels: bool,
    /// Navigator pop-up (corner of the scrollbars).
    pub navigator_open: bool,
    /// A right-button drag of the selection and the menu where it ended
    /// (see drop_menu.rs).
    pub right_drag: Option<crate::drop_menu::RightDrag>,
    /// A bar being moved (see ui/bars.rs).
    pub bar_drag: Option<crate::ui::bars::BarDrag>,
    /// The Time Tracker's clock (see time_tracker.rs).
    pub tracker: crate::time_tracker::Clock,
    pub drop_menu: Option<crate::drop_menu::DropMenu>,
    /// Objects docker: layers shown closed, groups shown open.
    pub objects_collapsed: std::collections::HashSet<tracedraw_core::LayerId>,
    pub objects_open_groups: std::collections::HashSet<ShapeId>,
    pub navigator_tex: Option<egui::TextureHandle>,
    pub options_label_index: usize,
    pub options_current_only: bool,
    /// Selected custom anchor index (Anchor Editing tool).
    pub anchor_sel: Option<usize>,
    /// Object > Order > In Front Of / Behind is waiting for a click: Some(in_front).
    pub pending_order: Option<bool>,
    pub last_repeatable: Option<Command>,
    pub show_non_printing: bool,
    pub text_hyphenation: bool,
    /// Macro recording in progress: the commands run since Record started.
    pub recording: Option<Vec<Command>>,
    pub workspace: Workspace,
    pub palettes: Vec<crate::palette::Palette>,
    pub visible_palettes: Vec<usize>,
    pub show_standard_toolbar: bool,
    pub show_property_bar: bool,
    pub show_toolbox: bool,
    pub show_text_toolbar: bool,
    pub show_zoom_toolbar: bool,
    pub show_transform_toolbar: bool,
    /// Right-click context menu anchor (screen position, page position).
    pub context_menu: Option<(egui::Pos2, Point)>,
    pub scripts_output: Vec<String>,
    pub script_source: String,
    /// AI Drawing: the request, the key in use, the conversation.
    pub ai: crate::ai::AiSession,
    /// AI agents editing the open drawing (MCP), while allowed.
    #[cfg(not(target_arch = "wasm32"))]
    pub mcp_link: Option<crate::mcp_link::Link>,
    pub find_state: crate::ui::dialogs::FindReplaceState,
    pub step_repeat: crate::ops2::StepRepeat,
    pub align_to: crate::ops2::AlignTo,
    pub lens: crate::lens::LensSettings,
    pub envelope_mode: crate::effects_ui::EnvelopeMode,
    pub bevel: crate::effects_ui::BevelSettings,
    pub extrude: crate::effects_ui::ExtrudeSettings,
    pub transparency_default: crate::effects_ui::TransparencySettings,
    pub glyph_filter: String,
    pub table_edit: Option<crate::table::TableEdit>,
    pub missing_fonts: Vec<String>,
    /// Thumbnails of recent drawings loaded for the Welcome Screen (none
    /// when a drawing has no thumbnail).
    pub recent_thumbs: std::collections::HashMap<PathBuf, Option<egui::TextureHandle>>,
    /// The Shelf docker's items, kept across drawings and sessions.
    pub shelf: crate::shelf::Shelf,
    /// A colour dragged from a palette (`Some(None)`: No Color).
    pub palette_drag: Option<Option<Color>>,
    /// Where the document palette is this frame (a drop target).
    pub doc_palette_rect: Option<egui::Rect>,
    /// The pointer on screen, wherever it is (drops outside the drawing).
    pub pointer_screen: Option<egui::Pos2>,
    pub shaping_keep_source: bool,
    pub shaping_keep_target: bool,
    pub color_model: usize,
    pub mixer_color: Color,
    pub lens_synced_to: Option<ShapeId>,
    pub extrude_synced_to: Option<ShapeId>,
    pub pending_blend_path: bool,
    /// The document palette's eyedropper waits for a click on the drawing.
    pub pending_palette_sample: bool,
    pub envelope_keep_lines: bool,
    pub media_mode: crate::media::MediaMode,
    pub media_preset: usize,
    pub media_spacing: f64,
    pub media_pressure: f64,
    pub media_smoothing: f64,
    pub mask_colors: Vec<Color>,
    pub mask_tolerance: u8,
    pub font_filter: String,
    /// Envelope/perspective node being dragged with the Shape tool: (shape, index).
    pub effect_node_drag: Option<(ShapeId, usize)>,
    /// Dockers added to the tab strip beyond the defaults.
    pub open_dockers: Vec<DockerTab>,
    pub free_transform_last: Point,
    pub free_transform_mode: FreeTransformMode,
    pub free_transform_duplicate: bool,
    pub free_transform_reflected: bool,
    pub roughen_amount: f64,
    /// Properties docker section to expand: 0 fill, 1 outline.
    /// A section of the Properties docker to expand on the next frame
    /// (0 fill, 1 outline), set by F11 and F12.
    pub properties_open: Option<usize>,
    pub area_fill_color: Color,
    pub mesh_rows: u32,
    pub mesh_cols: u32,
    /// Context menu > Frame Type > Text: next click/drag adds a text frame.
    pub pending_text_frame: bool,
    /// Shape tool elastic mode: dragging one node pulls its neighbours.
    pub elastic_mode: bool,
    /// Shape tool: the node transform mode on the property bar.
    pub node_transform: crate::node_edit::NodeTransformMode,
    /// Shape tool: Reflect Nodes horizontally, vertically.
    pub reflect_nodes: (bool, bool),
    /// Shape tool: the freehand marquee for nodes (else rectangular).
    pub node_lasso: bool,
    /// Shape tool: the last run of nodes Shift+click selected.
    pub node_run: Option<crate::node_edit::NodeRun>,
    /// The Coordinates docker's fields, the "Set ... interactively" pick
    /// it waits for and where that pick's drag began.
    pub coords: crate::coords::CoordsState,
    /// The Vector Mosaic docker's settings.
    pub vector_mosaic: crate::vector_mosaic::VectorMosaicSettings,
    /// The Picture Mosaic docker's settings and indexed library.
    pub picture_mosaic: crate::picture_mosaic::PictureMosaicSettings,
    /// A Picture Mosaic library being indexed in the background.
    pub picture_mosaic_job: Option<crate::picture_mosaic::IndexJob>,
    pub coord_pick: Option<crate::coords::CoordPick>,
    pub coord_drag: Option<Point>,
    /// Shape tool: the Curve smoothness slider and the curves it started from.
    pub curve_smoothness: f64,
    pub smoothing: Option<Vec<crate::node_edit::StartCurve>>,
    /// Last clicked effect/mesh node (palette clicks colour a mesh node).
    pub selected_effect_node: Option<(ShapeId, usize)>,
    /// Open drawings, one tab each (see `documents.rs`); the active one's
    /// state lives in the fields above.
    pub docs: Vec<crate::documents::DocSlot>,
    pub active_doc: usize,
    /// Last "Untitled-N" number handed out this session.
    pub untitled_counter: u32,
    /// Window > Close All (or Exit) is going through the open drawings.
    pub closing_all: bool,
    /// Quit once every drawing is closed (File > Exit with unsaved work).
    pub quit_after_closing: bool,
    /// Close the window on the next frame.
    pub quit_now: bool,
    /// Share of the drawing window's bottom row given to the page tabs
    /// (the rest is the horizontal scrollbar); set with the splitter.
    pub page_tabs_fraction: f32,
    /// The Hints docker's page and history.
    pub hints: crate::ui::hints::HintsState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WelcomeTab {
    GetStarted,
    Workspace,
    News,
    Learn,
    Templates,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Wireframe,
    Normal,
    Enhanced,
    Pixels,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectKind {
    Shadow,
    Transparency,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Workspace {
    Default,
    Lite,
    Classic,
    Illustration,
    PageLayout,
    /// For touch screens: larger bars and buttons, no rulers, panels or
    /// status bar; two fingers pan and pinch zooms.
    Touch,
}

impl Workspace {
    /// Every built-in workspace, in the order the menus list them.
    pub const ALL: [Workspace; 6] = [
        Workspace::Default,
        Workspace::Lite,
        Workspace::Classic,
        Workspace::Illustration,
        Workspace::PageLayout,
        Workspace::Touch,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Workspace::Default => "default",
            Workspace::Lite => "lite",
            Workspace::Classic => "classic",
            Workspace::Illustration => "illustration",
            Workspace::PageLayout => "page_layout",
            Workspace::Touch => "touch",
        }
    }
    pub fn from_id(id: &str) -> Self {
        match id {
            "lite" => Workspace::Lite,
            "classic" => Workspace::Classic,
            "illustration" => Workspace::Illustration,
            "page_layout" => Workspace::PageLayout,
            "touch" => Workspace::Touch,
            _ => Workspace::Default,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockerTab {
    Properties,
    Objects,
    Hints,
    Transformations,
    Undo,
    AlignDistribute,
    Shaping,
    StepAndRepeat,
    Text,
    Glyphs,
    Color,
    ColorStyles,
    ObjectStyles,
    FindReplace,
    Scripts,
    Palettes,
    Lens,
    Blend,
    Contour,
    Envelope,
    Extrude,
    Bevel,
    BrushStrokes,
    BitmapMask,
    ObjectData,
    Links,
    Symbols,
    Pages,
    Guidelines,
    Fonts,
    Corners,
    JoinCurves,
    Coordinates,
    VectorMosaic,
    PictureMosaic,
    Shelf,
    AiDrawing,
}

impl DockerTab {
    pub const ALL: [DockerTab; 37] = [
        DockerTab::Properties,
        DockerTab::Objects,
        DockerTab::Hints,
        DockerTab::Transformations,
        DockerTab::Undo,
        DockerTab::AlignDistribute,
        DockerTab::Shaping,
        DockerTab::StepAndRepeat,
        DockerTab::Text,
        DockerTab::Glyphs,
        DockerTab::Color,
        DockerTab::ColorStyles,
        DockerTab::ObjectStyles,
        DockerTab::FindReplace,
        DockerTab::Scripts,
        DockerTab::Palettes,
        DockerTab::Lens,
        DockerTab::Blend,
        DockerTab::Contour,
        DockerTab::Envelope,
        DockerTab::Extrude,
        DockerTab::Bevel,
        DockerTab::BrushStrokes,
        DockerTab::BitmapMask,
        DockerTab::ObjectData,
        DockerTab::Links,
        DockerTab::Symbols,
        DockerTab::Pages,
        DockerTab::Guidelines,
        DockerTab::Fonts,
        DockerTab::Corners,
        DockerTab::JoinCurves,
        DockerTab::Coordinates,
        DockerTab::VectorMosaic,
        DockerTab::PictureMosaic,
        DockerTab::Shelf,
        DockerTab::AiDrawing,
    ];

    /// i18n key of the docker's title.
    pub fn key(self) -> &'static str {
        match self {
            DockerTab::Properties => "docker.properties",
            DockerTab::Objects => "docker.objects",
            DockerTab::Hints => "docker.hints",
            DockerTab::Transformations => "docker.transformations",
            DockerTab::Undo => "docker.undo",
            DockerTab::AlignDistribute => "docker.align_distribute",
            DockerTab::Shaping => "docker.shaping",
            DockerTab::StepAndRepeat => "docker.step_and_repeat",
            DockerTab::Text => "docker.text",
            DockerTab::Glyphs => "docker.glyphs",
            DockerTab::Color => "docker.color",
            DockerTab::ColorStyles => "docker.color_styles",
            DockerTab::ObjectStyles => "docker.object_styles",
            DockerTab::FindReplace => "docker.find_replace",
            DockerTab::Scripts => "docker.scripts",
            DockerTab::Palettes => "docker.palettes",
            DockerTab::Lens => "docker.lens",
            DockerTab::Blend => "docker.blend",
            DockerTab::Contour => "docker.contour",
            DockerTab::Envelope => "docker.envelope",
            DockerTab::Extrude => "docker.extrude",
            DockerTab::Bevel => "docker.bevel",
            DockerTab::BrushStrokes => "docker.brush_strokes",
            DockerTab::BitmapMask => "docker.bitmap_mask",
            DockerTab::ObjectData => "docker.object_data",
            DockerTab::Links => "docker.links",
            DockerTab::Symbols => "docker.symbols",
            DockerTab::Pages => "docker.pages",
            DockerTab::Guidelines => "docker.guidelines",
            DockerTab::Fonts => "docker.fonts",
            DockerTab::Corners => "docker.corners_docker",
            DockerTab::JoinCurves => "docker.join_curves",
            DockerTab::Coordinates => "docker.coordinates",
            DockerTab::VectorMosaic => "docker.vector_mosaic",
            DockerTab::PictureMosaic => "docker.picture_mosaic",
            DockerTab::Shelf => "docker.shelf",
            DockerTab::AiDrawing => "docker.ai_drawing",
        }
    }

    pub fn shortcut(self) -> &'static str {
        match self {
            DockerTab::Properties => "Alt+Enter",
            DockerTab::AlignDistribute => "Ctrl+Shift+A",
            DockerTab::Text => "Ctrl+T",
            DockerTab::Glyphs => "Ctrl+F11",
            DockerTab::Color => "Shift+F11",
            DockerTab::Contour => "Ctrl+F9",
            DockerTab::Envelope => "Ctrl+F7",
            DockerTab::Lens => "Alt+F3",
            DockerTab::Symbols => "Ctrl+F3",
            DockerTab::Blend => "Ctrl+F8",
            DockerTab::Extrude => "Ctrl+F10",
            DockerTab::Fonts => "Ctrl+F12",
            DockerTab::FindReplace => "Ctrl+F",
            DockerTab::StepAndRepeat => "Ctrl+Shift+D",
            DockerTab::Transformations => "Alt+F7",
            DockerTab::Undo => "",
            _ => "",
        }
    }

    /// Dockers shown in the tab strip by default (the rest open from menus).
    pub fn default_strip() -> Vec<DockerTab> {
        vec![DockerTab::Hints, DockerTab::Properties, DockerTab::Objects]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformTab {
    Position,
    Rotate,
    Scale,
    Size,
    Skew,
}

impl App {
    /// The editor in the window of `ctx`, whose fonts are already
    /// installed (see `theme::install_fonts` and the splash screen).
    pub fn new(ctx: &egui::Context, open: Option<PathBuf>) -> Self {
        // egui quits on Ctrl+Q by default; here Ctrl+Q is Convert to Curves
        // and the application closes through File > Exit (Alt+F4).
        ctx.options_mut(|o| {
            o.quit_shortcuts.clear();
            // Ctrl+plus and Ctrl+minus zoom the drawing, not the interface;
            // the interface size is an option (see `ui_size`).
            o.zoom_with_keyboard = false;
        });
        ctx.set_visuals(theme::visuals());
        ctx.all_styles_mut(|style| {
            style.spacing.item_spacing = egui::vec2(4.0, 3.0);
            style.spacing.button_padding = egui::vec2(5.0, 2.0);
            // Interface text at 13 px, the size labels, lists, menus and
            // the status bar use in this font;
            // headings in the bold face.
            use egui::{FontFamily, FontId, TextStyle};
            style.text_styles = [
                (
                    TextStyle::Small,
                    FontId::new(11.0, FontFamily::Proportional),
                ),
                (TextStyle::Body, FontId::new(13.0, FontFamily::Proportional)),
                (
                    TextStyle::Button,
                    FontId::new(13.0, FontFamily::Proportional),
                ),
                (
                    TextStyle::Monospace,
                    FontId::new(13.0, FontFamily::Monospace),
                ),
                (TextStyle::Heading, theme::bold(17.0)),
            ]
            .into();
        });
        Self::build(open)
    }

    /// An app without a window, for tests, scripts and the MCP server.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub fn headless() -> Self {
        if std::env::var_os("TRACEDRAW_CONFIG_DIR").is_none() {
            std::env::set_var(
                "TRACEDRAW_CONFIG_DIR",
                std::env::temp_dir().join("tracedraw-headless"),
            );
        }
        let mut app = Self::build(None);
        if !app.has_document() {
            app.new_document();
        }
        app.show_welcome = false;
        app
    }

    fn build(open: Option<PathBuf>) -> Self {
        let engine = Engine::default();
        let page = engine.document().pages[0].id;
        let mut app = App {
            engine,
            page,
            view: View::default(),
            tool: Tool::Pick,
            previous_tool: Tool::Pick,
            selection: Vec::new(),
            rotate_mode: false,
            pivots: Pivots::default(),
            drag: Drag::None,
            drag_copy: false,
            curve: None,
            text_edit: None,
            file: None,
            cdr_version: None,
            status: String::new(),
            fit_pending: true,
            units: Units::Millimeters,
            nudge_mm: 2.54,
            show_rulers: true,
            show_grid: false,
            show_status_bar: true,
            show_dockers: true,
            docker_tab: DockerTab::Hints,
            flyout_open: None,
            toolbox_last: crate::tools::GROUPS.iter().map(|g| g.tools[0]).collect(),
            clipboard: None,
            duplicate_offset: Vec2::new(6.35, 6.35),
            polygon_points: 5,
            star_sharpness: 0.5,
            star_complex: false,
            complex_points: 9,
            complex_sharpness: 2,
            rect_corners: tracedraw_core::Corners::default(),
            corners_together: true,
            rect_corner_selected: None,
            ellipse_arc: None,
            contour_steps: 3,
            contour_offset: 2.0,
            contour_direction: crate::tools2::ContourDirection::Outside,
            contour_color: Color::WHITE,
            spiral_revolutions: 4,
            spiral_logarithmic: false,
            common_shape: crate::tools2::CommonShape::RightArrow,
            table_rows: 3,
            table_cols: 4,
            table_fill: Fill::None,
            block_shadow_color: Color::cmyk_pct(0.0, 0.0, 0.0, 60.0),
            block_shadow_gap: 0.0,
            table_border: Some(Stroke::hairline(Color::BLACK)),
            graph_rows: 4,
            graph_cols: 3,
            action_lines_count: 12,
            action_lines_radial: false,
            three_point_base: None,
            media_width: 3.0,
            media_angle: 45.0,
            dimension_points: Vec::new(),
            blend_steps: 5,
            extrude_depth: Vec2::new(8.0, -8.0),
            distort_mode: crate::tools2::DistortMode::PushPull,
            distort_amount: 20.0,
            distort_frequency: 12,
            brush_radius: 10.0,
            eraser_width: 6.35,
            eraser_square: false,
            shadow_default: tracedraw_core::Shadow::default(),
            text_font: "Arial".into(),
            text_size_pt: 24.0,
            text_bold: false,
            text_italic: false,
            text_underline: false,
            text_align: tracedraw_core::TextAlign::Left,
            eyedropper_color: None,
            eyedropper_attrs: None,
            eyedropper_groups: Default::default(),
            eyedropper_apply: false,
            eyedropper_sample: 1,
            canvas_rect: egui::Rect::NOTHING,
            pointer_page: None,
            palette: crate::palette::default_cmyk(),
            palette_scroll: 0,
            doc_palette_scroll: 0,
            doc_palette_current: None,
            palette_shades: None,
            palette_expanded: None,
            raster: std::cell::RefCell::new(crate::raster::Raster::default()),
            overflow_cache: Default::default(),
            wireframe: false,
            font_families: tracedraw_text::fonts().available_families(),
            font_generation: tracedraw_text::fonts().generation(),
            color_profile_key: None,
            transform_tab: TransformTab::Position,
            node_selection: Vec::new(),
            segment_spot: None,
            snap: crate::snap::SnapSettings::default(),
            snap_mark: std::cell::Cell::new(None),
            corners: Default::default(),
            join_settings: Default::default(),
            docker_preview: Vec::new(),
            docker_preview_point: None,
            dialog: crate::ui::dialogs::Dialog::None,
            merge_state: None,
            page_numbers: PageNumberSettings::default(),
            show_welcome: false,
            pending_clip_frame: false,
            clip_frame_edit: None,
            show_guides: true,
            selected_guides: Vec::new(),
            guide_rotate: None,
            guide_pivot: Point::ZERO,
            guide_form: Default::default(),
            guide_presets: Default::default(),
            grid_frequency: false,
            transform_values: [0.0, 0.0, 100.0, 100.0],
            settings: crate::settings::Settings::default(),
            welcome_tab: WelcomeTab::GetStarted,
            fullscreen_preview: false,
            preview_selected_only: false,
            page_sorter: false,
            view_mode: ViewMode::Enhanced,
            proof_colors: false,
            show_page_border: true,
            simulate_overprints: false,
            rasterize_complex_effects: true,
            show_bleed: false,
            show_printable_area: false,
            show_pixel_grid: true,
            show_baseline_grid: false,
            options_page: crate::ui::dialogs::OptionsPage::General,
            options_snapshot: None,
            shortcut_editor: Default::default(),
            pending_command: None,
            last_auto_backup: None,
            recent_thumbs: Default::default(),
            shelf: Default::default(),
            palette_drag: None,
            doc_palette_rect: None,
            pointer_screen: None,
            page_size_all: true,
            object_origin: (1, 1),
            scale_locked: false,
            pending_copy_properties: false,
            pending_copy_effect: None,
            pending_clone_effect: None,
            options_labels: false,
            navigator_open: false,
            right_drag: None,
            bar_drag: None,
            tracker: Default::default(),
            drop_menu: None,
            objects_collapsed: Default::default(),
            objects_open_groups: Default::default(),
            navigator_tex: None,
            options_label_index: 0,
            options_current_only: false,
            anchor_sel: None,
            pending_order: None,
            last_repeatable: None,
            show_non_printing: false,
            text_hyphenation: false,
            recording: None,
            workspace: Workspace::Default,
            palettes: crate::palette::builtin_palettes(),
            visible_palettes: vec![0],
            show_standard_toolbar: true,
            show_property_bar: true,
            show_toolbox: true,
            show_text_toolbar: false,
            show_zoom_toolbar: false,
            show_transform_toolbar: false,
            context_menu: None,
            scripts_output: Vec::new(),
            script_source: String::new(),
            ai: Default::default(),
            #[cfg(not(target_arch = "wasm32"))]
            mcp_link: None,
            find_state: Default::default(),
            step_repeat: Default::default(),
            align_to: Default::default(),
            lens: Default::default(),
            envelope_mode: Default::default(),
            bevel: Default::default(),
            extrude: Default::default(),
            transparency_default: Default::default(),
            glyph_filter: String::new(),
            table_edit: None,
            missing_fonts: Vec::new(),
            shaping_keep_source: false,
            shaping_keep_target: false,
            color_model: 1,
            mixer_color: Color::cmyk_pct(0.0, 0.0, 0.0, 100.0),
            lens_synced_to: None,
            extrude_synced_to: None,
            pending_blend_path: false,
            pending_palette_sample: false,
            envelope_keep_lines: false,
            media_mode: crate::media::MediaMode::Calligraphic,
            media_preset: 0,
            media_spacing: 8.0,
            media_pressure: 0.5,
            media_smoothing: 25.0,
            mask_colors: Vec::new(),
            mask_tolerance: 20,
            font_filter: String::new(),
            effect_node_drag: None,
            open_dockers: Vec::new(),
            free_transform_last: Point::ZERO,
            free_transform_mode: FreeTransformMode::Rotation,
            free_transform_duplicate: false,
            free_transform_reflected: false,
            roughen_amount: 2.0,
            properties_open: None,
            area_fill_color: Color::cmyk_pct(0.0, 0.0, 100.0, 0.0),
            mesh_rows: 2,
            mesh_cols: 2,
            pending_text_frame: false,
            elastic_mode: false,
            node_transform: Default::default(),
            reflect_nodes: (false, false),
            node_lasso: false,
            node_run: None,
            coords: Default::default(),
            vector_mosaic: Default::default(),
            picture_mosaic: Default::default(),
            picture_mosaic_job: None,
            coord_pick: None,
            coord_drag: None,
            curve_smoothness: 0.0,
            smoothing: None,
            selected_effect_node: None,
            docs: Vec::new(),
            active_doc: 0,
            untitled_counter: 0,
            closing_all: false,
            quit_after_closing: false,
            quit_now: false,
            page_tabs_fraction: 0.45,
            hints: Default::default(),
        };
        app.load_settings();
        app.apply_runtime_settings();
        // Start-up: the file given on the command line, else what Options >
        // General asks for: the Welcome Screen alone (no drawing open), a
        // new drawing, or the last drawing edited.
        if let Some(p) = open {
            app.open_path(p);
        }
        if !app.has_document() {
            match app.settings.startup {
                crate::settings::Startup::WelcomeScreen => app.show_welcome = true,
                crate::settings::Startup::NewDocument => app.new_document(),
                crate::settings::Startup::LastDocument => {
                    match app.settings.recent_files.first().cloned() {
                        Some(p) if crate::files::exists(&p) => app.open_path(p),
                        _ => {}
                    }
                    if !app.has_document() {
                        app.show_welcome = true;
                    }
                }
            }
        }
        app
    }

    /// A new document with localised default page and layer names.
    pub fn localized_document(title: impl Into<String>, size: Size) -> tracedraw_core::Document {
        let mut doc = tracedraw_core::Document::new(title, size);
        for (i, p) in doc.pages.iter_mut().enumerate() {
            p.name = crate::i18n::trf("doc.page_n", &[("n", &(i + 1).to_string())]);
            for (k, l) in p.layers.iter_mut().enumerate() {
                l.name = crate::i18n::trf("docker.layer_n", &[("n", &(k + 1).to_string())]);
            }
        }
        doc
    }

    /// F11, and a double click on the status bar's fill: the fill
    /// settings (the Properties docker's Fill section).
    pub fn open_fill_editor(&mut self) {
        self.show_dockers = true;
        self.docker_tab = DockerTab::Properties;
        self.properties_open = Some(0);
    }

    /// F12, and a double click on the status bar's outline: the outline
    /// settings (the Properties docker's Outline section).
    pub fn open_outline_editor(&mut self) {
        self.show_dockers = true;
        self.docker_tab = DockerTab::Properties;
        self.properties_open = Some(1);
    }

    /// The resolution of the active drawing, for the pixel unit.
    pub fn document_dpi(&self) -> f64 {
        let dpi = self.doc().metadata.resolution_dpi;
        if dpi > 0.0 {
            dpi
        } else {
            self.settings.default_dpi.max(1.0)
        }
    }

    /// Layout > Page Setup and a double click on the page border or
    /// shadow: the Options dialog at the page size page.
    pub fn open_page_options(&mut self) {
        self.options_page = crate::ui::dialogs::OptionsPage::PageSize;
        self.dialog = crate::ui::dialogs::Dialog::Options;
    }

    /// Settings that act on the editor directly (undo levels), applied
    /// after loading the preferences and when an options dialog closes.
    pub fn apply_runtime_settings(&mut self) {
        let levels = self.settings.undo_levels.max(1);
        self.engine.set_max_history(levels);
        for slot in &mut self.docs {
            slot.engine.set_max_history(levels);
        }
    }

    /// Where auto-backups go unless Options > Save names a folder: a
    /// `TraceDraw` folder in the temporary folder (none in a browser).
    pub fn auto_backup_dir_default() -> Option<PathBuf> {
        if crate::files::WEB {
            return None;
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            Some(std::env::temp_dir().join("TraceDraw"))
        }
        #[cfg(target_arch = "wasm32")]
        {
            None
        }
    }

    /// Double-click on a ruler: Document Options at the Rulers page.
    pub fn open_ruler_options(&mut self) {
        self.options_page = crate::ui::dialogs::OptionsPage::Rulers;
        self.dialog = crate::ui::dialogs::Dialog::Options;
    }

    /// The ruler origin, page coordinates in mm.
    pub fn ruler_origin(&self) -> Point {
        let r = self.doc().metadata.rulers.clone();
        Point::new(r.origin_x, r.origin_y)
    }

    /// A page position as the rulers count it (mm from the ruler origin).
    pub fn to_ruler(&self, p: Point) -> Point {
        let o = self.ruler_origin();
        Point::new(p.x - o.x, p.y - o.y)
    }

    /// A position counted from the ruler origin, back to page coordinates.
    pub fn from_ruler(&self, p: Point) -> Point {
        let o = self.ruler_origin();
        Point::new(p.x + o.x, p.y + o.y)
    }

    /// Move the ruler origin (undoable, saved with the drawing).
    pub fn set_ruler_origin(&mut self, origin: Point) {
        let mut metadata = self.doc().metadata.clone();
        if metadata.rulers.origin_x == origin.x && metadata.rulers.origin_y == origin.y {
            return;
        }
        metadata.rulers.origin_x = origin.x;
        metadata.rulers.origin_y = origin.y;
        if let Err(e) = self
            .engine
            .run_with_label(&Command::SetMetadata { metadata }, "Ruler Origin")
        {
            self.status = format!("Ruler Origin: {e}");
        }
    }

    /// True while the ruler origin is being dragged out of the corner.
    pub fn drag_is_origin(&self) -> bool {
        matches!(self.drag, Drag::RulerOrigin { .. })
    }

    /// The colour around the page (Options > Customization), white by
    /// default.
    pub fn desktop_color(&self) -> egui::Color32 {
        let [r, g, b] = self.settings.desktop_rgb;
        egui::Color32::from_rgb(r, g, b)
    }

    /// A fallback title (a file without a usable name).
    pub fn untitled_name() -> String {
        crate::i18n::trf("doc.untitled_n", &[("n", "1")])
    }

    /// The name the next new drawing will get (without using it up).
    pub fn peek_untitled_name(&self) -> String {
        crate::i18n::trf(
            "doc.untitled_n",
            &[("n", &(self.untitled_counter + 1).to_string())],
        )
    }

    /// Apply persisted settings (language, workspace, snapping, units).
    pub fn load_settings(&mut self) {
        self.apply_settings(crate::settings::Settings::load());
    }

    /// Apply a set of preferences to the running editor.
    pub fn apply_settings(&mut self, s: crate::settings::Settings) {
        self.ai.key = crate::ai::remembered_key(&s.ai);
        let lang = if s.language.is_empty() {
            crate::i18n::system_language().to_string()
        } else {
            s.language.clone()
        };
        crate::i18n::set_language(&lang);
        self.workspace = Workspace::from_id(&s.workspace);
        self.set_workspace(self.workspace);
        // The bars as they were left, over the workspace's own.
        if let Some(shown) = s.shown_bars {
            self.set_shown_bars(shown);
        }
        self.snap.grid = s.snap.grid;
        self.snap.guides = s.snap.guides;
        self.snap.objects = s.snap.objects;
        self.snap.page = s.snap.page;
        self.nudge_mm = s.nudge_mm;
        self.apply_toolbox_picks(&s.toolbox);
        self.duplicate_offset = Vec2::new(s.duplicate_offset_mm[0], s.duplicate_offset_mm[1]);
        self.units = Units::from_id(&s.units);
        self.settings = s;
        self.apply_color_settings();
    }

    /// Install the colour engine from the settings: ICC profiles loaded from
    /// disk when paths are set, the built-in sRGB and generic CMYK model
    /// otherwise. Errors fall back to the built-in model with a status line.
    pub fn apply_color_settings(&mut self) {
        use tracedraw_core::color::{engine, IccEngine};
        use tracedraw_core::icc::{Intent, Profile};
        let c = &self.settings.color;
        let load = |path: &str| -> Option<Profile> {
            if path.is_empty() {
                return None;
            }
            match crate::files::read(std::path::Path::new(path)).map_err(|e| e.to_string()) {
                Ok(bytes) => Profile::parse(&bytes).map_err(|e| e.to_string()).ok(),
                Err(_) => None,
            }
        };
        let rgb = load(&c.rgb_profile_path);
        // A drawing that carries its CMYK profile (a `.cdr` file's) shows
        // its CMYK colours through it; others through the settings' one.
        let carried = self.doc().cmyk_profile.clone();
        let cmyk = carried_cmyk_profile(self.doc()).or_else(|| load(&c.cmyk_profile_path));
        let intent = Intent::from_name(&c.intent).unwrap_or_default();
        let bpc = c.black_point_compensation;
        self.color_profile_key = carried.map(|p| std::sync::Arc::as_ptr(&p.data) as usize);
        if rgb.is_none() && cmyk.is_none() {
            engine::clear();
            self.raster.borrow_mut().invalidate();
            return;
        }
        engine::install(IccEngine::new(rgb, cmyk, intent, bpc));
        self.raster.borrow_mut().invalidate();
    }

    /// Show the active drawing through the CMYK profile it carries, or the
    /// settings' one when it carries none: called every frame, it installs
    /// the colour engine again only when the drawing's profile changed
    /// (another tab, a drawing opened or reverted).
    pub fn sync_document_colors(&mut self) {
        let key = self
            .doc()
            .cmyk_profile
            .as_ref()
            .map(|p| std::sync::Arc::as_ptr(&p.data) as usize);
        if key != self.color_profile_key {
            self.apply_color_settings();
        }
    }

    /// Pick an `.icc`/`.icm` file for the RGB or CMYK slot.
    pub fn load_icc_profile(&mut self, cmyk: bool) {
        crate::files::Dialog::new()
            .add_filter("ICC", &["icc", "icm"])
            .pick_file(self, move |app, path| {
                let path = crate::files::keep(&path);
                app.use_icc_profile(cmyk, path);
            });
    }

    fn use_icc_profile(&mut self, cmyk: bool, path: PathBuf) {
        let bytes = match crate::files::read(&path) {
            Ok(b) => b,
            Err(e) => {
                self.status = crate::i18n::trf("status.icc_failed", &[("e", &e.to_string())]);
                return;
            }
        };
        match tracedraw_core::icc::Profile::parse(&bytes) {
            Ok(p) => {
                let name = p
                    .description()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| path.display().to_string());
                let c = &mut self.settings.color;
                if cmyk {
                    c.cmyk_profile = name.clone();
                    c.cmyk_profile_path = path.display().to_string();
                } else {
                    c.rgb_profile = name.clone();
                    c.rgb_profile_path = path.display().to_string();
                }
                self.apply_color_settings();
                self.status = crate::i18n::trf("status.icc_loaded", &[("name", &name)]);
            }
            Err(e) => {
                self.status = crate::i18n::trf("status.icc_failed", &[("e", &e.to_string())]);
            }
        }
    }

    /// Every preference back to how TraceDraw comes (history kept: see
    /// `Settings::reset_keeping_history`), applied and saved: shortcuts,
    /// bars, toolbox, palette options, interface size and the rest.
    /// Drawings are not touched.
    pub fn reset_settings(&mut self) {
        let fresh = self.settings.reset_keeping_history();
        self.toolbox_last = crate::tools::GROUPS.iter().map(|g| g.tools[0]).collect();
        self.apply_settings(fresh);
        self.show_page_border = true;
        self.shortcut_editor = Default::default();
        self.recent_thumbs.clear();
        self.apply_runtime_settings();
        self.save_settings();
        self.status = crate::i18n::tr("status.settings_reset");
    }

    pub fn save_settings(&mut self) {
        self.sync_settings();
        self.settings.save();
    }

    /// Copy the editor's state (workspace, snapping, toolbox, units...)
    /// into the preferences, without writing them.
    pub fn sync_settings(&mut self) {
        self.settings.workspace = self.workspace.id().into();
        self.settings.snap.grid = self.snap.grid;
        self.settings.snap.guides = self.snap.guides;
        self.settings.snap.objects = self.snap.objects;
        self.settings.snap.page = self.snap.page;
        self.settings.nudge_mm = self.nudge_mm;
        self.settings.toolbox = self
            .toolbox_last
            .iter()
            .map(|t| t.id().to_string())
            .collect();
        self.settings.duplicate_offset_mm = [self.duplicate_offset.x, self.duplicate_offset.y];
        self.settings.units = self.units.id().into();
        self.settings.language = crate::i18n::language();
        self.settings.shown_bars = Some(self.shown_bars());
    }

    /// The tool each toolbox group shows, from tool ids (one per group, in
    /// group order); ids that name no tool of their group are skipped.
    pub fn apply_toolbox_picks(&mut self, ids: &[String]) {
        for (gi, id) in ids.iter().enumerate() {
            let group = crate::tools::GROUPS.get(gi);
            let tool = Tool::ALL.iter().find(|t| t.id() == id);
            if let (Some(g), Some(t), Some(slot)) = (group, tool, self.toolbox_last.get_mut(gi)) {
                if g.tools.contains(t) {
                    *slot = *t;
                }
            }
        }
    }

    /// The bars shown now (the palette's visibility is a palette option).
    pub fn shown_bars(&self) -> crate::settings::ShownBars {
        crate::settings::ShownBars {
            standard_toolbar: self.show_standard_toolbar,
            property_bar: self.show_property_bar,
            toolbox: self.show_toolbox,
            status_bar: self.show_status_bar,
            dockers: self.show_dockers,
            rulers: self.show_rulers,
            text_toolbar: self.show_text_toolbar,
            zoom_toolbar: self.show_zoom_toolbar,
            transform_toolbar: self.show_transform_toolbar,
        }
    }

    /// Show and hide the bars as `shown` says.
    pub fn set_shown_bars(&mut self, shown: crate::settings::ShownBars) {
        self.show_standard_toolbar = shown.standard_toolbar;
        self.show_property_bar = shown.property_bar;
        self.show_toolbox = shown.toolbox;
        self.show_status_bar = shown.status_bar;
        self.show_dockers = shown.dockers;
        self.show_rulers = shown.rulers;
        self.show_text_toolbar = shown.text_toolbar;
        self.show_zoom_toolbar = shown.zoom_toolbar;
        self.show_transform_toolbar = shown.transform_toolbar;
    }

    /// Tools > Save Settings as Default: persist current tool defaults.
    pub fn save_defaults(&mut self) {
        // New drawings start with this drawing's grid, rulers and
        // guideline settings.
        if self.has_document() {
            self.settings.document_defaults = crate::settings::DocumentDefaults::of(self.doc());
        }
        self.save_settings();
        self.status = crate::i18n::tr("status.settings_saved");
    }

    pub fn set_workspace(&mut self, ws: Workspace) {
        self.workspace = ws;
        // The interface size follows at once (the Touch workspace draws it
        // larger).
        self.settings.workspace = ws.id().into();
        match ws {
            Workspace::Default | Workspace::Classic => {
                self.show_standard_toolbar = true;
                self.show_property_bar = true;
                self.show_toolbox = true;
                self.show_status_bar = true;
                self.show_dockers = true;
                self.show_rulers = true;
            }
            Workspace::Lite => {
                self.show_standard_toolbar = true;
                self.show_property_bar = true;
                self.show_toolbox = true;
                self.show_status_bar = true;
                self.show_dockers = false;
                self.show_rulers = false;
            }
            Workspace::Illustration => {
                self.show_standard_toolbar = true;
                self.show_property_bar = true;
                self.show_toolbox = true;
                self.show_status_bar = true;
                self.show_dockers = true;
                self.docker_tab = DockerTab::Properties;
                self.show_rulers = true;
            }
            Workspace::PageLayout => {
                self.show_standard_toolbar = true;
                self.show_property_bar = true;
                self.show_toolbox = true;
                self.show_status_bar = true;
                self.show_dockers = true;
                self.docker_tab = DockerTab::Pages;
                self.show_rulers = true;
                self.show_guides = true;
            }
            Workspace::Touch => {
                self.show_standard_toolbar = true;
                self.show_property_bar = true;
                self.show_toolbox = true;
                self.show_status_bar = false;
                self.show_dockers = false;
                self.show_rulers = false;
            }
        }
    }

    pub fn document_title(&self) -> String {
        let name = self.document_name();
        if self.engine.is_dirty() {
            format!("{name}*")
        } else {
            name
        }
    }

    /// The active drawing's name: its file's name without the extension,
    /// else the document title (an Untitled name for a new drawing).
    pub fn document_name(&self) -> String {
        // Imported files (.cdr, .svg) have no native path yet; their title
        // comes from the document.
        self.file
            .as_ref()
            .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()))
            .unwrap_or_else(|| {
                let t = self.doc().title.trim();
                if t.is_empty() || t == "Untitled" {
                    App::untitled_name()
                } else {
                    t.to_string()
                }
            })
    }

    /// File > Close: close the active drawing, asking to save unsaved
    /// changes first.
    pub fn close_document(&mut self) {
        if !self.has_document() {
            return;
        }
        if self.engine.is_dirty() {
            self.dialog = crate::ui::dialogs::Dialog::ConfirmClose;
            return;
        }
        self.close_active_document();
    }

    pub fn save_as_template(&mut self) {
        crate::files::Dialog::new()
            .add_filter(crate::i18n::tr("file.template"), &["tdt"])
            .set_file_name("Template1.tdt")
            .save_file(self, |app, path| app.save_template_to(path));
    }

    fn save_template_to(&mut self, path: PathBuf) {
        let written = tracedraw_text::fonts()
            .with_carried_fonts(self.engine.document())
            .to_json()
            .map_err(|e| e.to_string())
            .and_then(|json| crate::files::write(&path, json).map_err(|e| e.to_string()));
        match written {
            Ok(()) => {
                self.status = crate::i18n::trf(
                    "status.saved_template",
                    &[("p", &path.display().to_string())],
                )
            }
            Err(e) => {
                self.status = crate::i18n::trf("status.save_failed", &[("e", &e.to_string())])
            }
        }
    }

    /// Edit > Repeat: run the last repeatable command on the current selection.
    pub fn repeat_last(&mut self) {
        let Some(cmd) = self.last_repeatable.clone() else {
            return;
        };
        let shapes = self.selection.clone();
        let cmd = match cmd {
            Command::TransformShapes { transform, .. } => {
                Command::TransformShapes { shapes, transform }
            }
            Command::SetFill { fill, .. } => Command::SetFill { shapes, fill },
            Command::SetStroke { stroke, .. } => Command::SetStroke { shapes, stroke },
            Command::SetOpacity { opacity, .. } => Command::SetOpacity { shapes, opacity },
            other => other,
        };
        self.run(cmd);
    }

    /// Edit > Paste in View: paste centred on the visible area.
    pub fn paste_in_view(&mut self) {
        let before: Vec<_> = self.selection.clone();
        self.paste();
        if self.selection == before {
            return;
        }
        if let Some(b) = self.selection_bounds() {
            let view_rect = self.view.visible_page_rect(self.canvas_rect);
            let d = view_rect.center() - b.center();
            let shapes = self.selection.clone();
            self.run(Command::TransformShapes {
                shapes,
                transform: Affine::translate(d),
            });
        }
    }

    pub fn select_all_of(&mut self, pred: impl Fn(&Shape) -> bool) {
        let ids: Vec<ShapeId> = self
            .doc()
            .page(self.page)
            .map(|p| {
                p.layers
                    .iter()
                    .filter(|l| l.visible && !l.locked)
                    .flat_map(|l| &l.shapes)
                    .filter(|s| !s.locked && s.visible && pred(s))
                    .map(|s| s.id)
                    .collect()
            })
            .unwrap_or_default();
        self.select(ids);
    }

    pub fn zoom_step(&mut self, zoom_in: bool) {
        let levels = [
            10.0, 25.0, 50.0, 75.0, 100.0, 150.0, 200.0, 300.0, 400.0, 800.0, 1600.0,
        ];
        let cur = self.zoom_percent();
        let next = if zoom_in {
            levels
                .iter()
                .cloned()
                .find(|l| *l > cur + 0.5)
                .unwrap_or(cur * 2.0)
        } else {
            levels
                .iter()
                .rev()
                .cloned()
                .find(|l| *l < cur - 0.5)
                .unwrap_or(cur / 2.0)
        };
        self.set_zoom_percent(next);
    }

    // ----- document helpers --------------------------------------------------

    pub fn doc(&self) -> &tracedraw_core::Document {
        self.engine.document()
    }

    pub fn page_size(&self) -> Size {
        self.doc()
            .page(self.page)
            .map(|p| p.size)
            .unwrap_or(paper::A4)
    }

    pub fn page_rect(&self) -> Rect {
        Rect::from_origin_size((0.0, 0.0), self.page_size())
    }

    /// The layer new objects go to: the topmost visible, unlocked layer
    /// (hidden layers cannot take new objects); with none visible, the
    /// topmost unlocked one.
    pub fn active_layer(&self) -> Option<LayerId> {
        let page = self.doc().page(self.page).ok()?;
        page.layers
            .iter()
            .rev()
            .find(|l| !l.locked && l.visible)
            .or_else(|| page.layers.iter().rev().find(|l| !l.locked))
            .map(|l| l.id)
    }

    /// The selected objects, each in page space (an object inside a group
    /// with its group's transform taken in).
    pub fn selected_shapes(&self) -> Vec<Shape> {
        let doc = self.doc();
        self.selection
            .iter()
            .filter_map(|id| doc.shape_on_page(*id))
            .collect()
    }

    /// Some selected object lies inside a group (chosen with Ctrl+click or
    /// in the Objects docker).
    pub fn selection_nested(&self) -> bool {
        let doc = self.doc();
        self.selection.iter().any(|id| doc.is_nested(*id))
    }

    /// Turn the rotate and skew handles on or off (a moved centre of
    /// rotation stays where it was put).
    pub fn toggle_rotate_mode(&mut self) {
        self.rotate_mode = !self.rotate_mode;
    }

    pub fn selection_bounds(&self) -> Option<Rect> {
        self.selected_shapes()
            .iter()
            .map(Shape::bounds)
            .reduce(|a, b| a.union(b))
    }

    pub fn run(&mut self, cmd: Command) {
        self.break_clone_links_for(&cmd);
        if let Err(e) = self.engine.run(&cmd) {
            self.status = format!("{}: {e}", cmd.label());
            return;
        }
        self.sync_effect_clones_for(&cmd);
        if matches!(
            cmd,
            Command::TransformShapes { .. }
                | Command::SetFill { .. }
                | Command::SetStroke { .. }
                | Command::SetOpacity { .. }
        ) {
            self.last_repeatable = Some(cmd.clone());
        }
        if let Some(rec) = &mut self.recording {
            rec.push(cmd);
        }
    }

    /// Open the docker (or hide the column when it is already the active
    /// tab), as the docker shortcuts do.
    pub fn toggle_docker(&mut self, tab: DockerTab) {
        if self.show_dockers && self.docker_tab == tab {
            self.show_dockers = false;
        } else {
            self.show_dockers = true;
            self.docker_tab = tab;
        }
    }

    pub fn set_tool(&mut self, tool: Tool) {
        if self.tool != tool {
            self.finish_curve();
            self.finish_text();
            self.table_edit = None;
            self.three_point_base = None;
            self.previous_tool = self.tool;
            self.tool = tool;
            self.rotate_mode = false;
            self.flyout_open = None;
            if let Some(gi) = crate::tools::GROUPS
                .iter()
                .position(|g| g.tools.contains(&tool))
            {
                if let Some(slot) = self.toolbox_last.get_mut(gi) {
                    *slot = tool;
                }
            }
            if !tool.implemented() {
                self.status =
                    crate::i18n::trf("status.tool_not_implemented", &[("t", &tool.name())]);
            }
        }
    }

    pub fn select(&mut self, ids: Vec<ShapeId>) {
        self.selection = ids;
        self.rotate_mode = false;
        self.node_selection.clear();
        self.segment_spot = None;
        self.rect_corner_selected = None;
    }

    // ----- object creation ---------------------------------------------------

    pub fn new_shape(&mut self, kind: ShapeKind) -> Option<ShapeId> {
        let layer = self.active_layer()?;
        let id = self.engine.new_shape_id();
        let mut shape = Shape::new(id, kind);
        // The fill and outline of its kind (Default Properties).
        self.doc().defaults.apply_to(&mut shape);
        self.run(Command::AddShape { layer, shape });
        Some(id)
    }

    /// The rectangle of a box drag: corner to corner, or centred on the
    /// start point when drawn with Shift.
    pub fn box_rect(start: Point, current: Point, from_center: bool) -> Rect {
        if from_center {
            let d = current - start;
            Rect::from_points(start - d, start + d)
        } else {
            Rect::from_points(start, current)
        }
    }

    pub fn create_box_shape(&mut self, a: Point, b: Point) {
        let rect = Rect::from_points(a, b);
        if rect.width() < 0.05 || rect.height() < 0.05 {
            return;
        }
        let kind = match self.tool {
            Tool::Rectangle | Tool::ThreePointRectangle => {
                ShapeKind::rect_with_corners(rect, self.rect_corners)
            }
            Tool::Ellipse | Tool::ThreePointEllipse => ShapeKind::Ellipse {
                rect,
                arc: self.ellipse_arc,
            },
            // The dragged box is the polygon's own bounding box.
            Tool::Polygon => ShapeKind::Polygon {
                rect: tracedraw_core::geometry::polygon_rect_for_bounds(
                    rect,
                    self.polygon_points,
                    0.0,
                ),
                points: self.polygon_points,
                sharpness: 0.0,
                complex: None,
                twist: 0.0,
            },
            // A complex star's vertices are those of a polygon.
            Tool::Star if self.star_complex => ShapeKind::Polygon {
                rect: tracedraw_core::geometry::polygon_rect_for_bounds(
                    rect,
                    self.complex_points,
                    0.0,
                ),
                points: self.complex_points,
                sharpness: 0.0,
                complex: Some(self.complex_sharpness),
                twist: 0.0,
            },
            Tool::Star => ShapeKind::Polygon {
                rect: tracedraw_core::geometry::polygon_rect_for_bounds(
                    rect,
                    self.polygon_points,
                    self.star_sharpness,
                ),
                points: self.polygon_points,
                sharpness: self.star_sharpness,
                complex: None,
                twist: 0.0,
            },
            Tool::GraphPaper => {
                self.create_graph_paper(rect);
                return;
            }
            Tool::ActionLines => {
                self.create_action_lines(rect);
                return;
            }
            _ => return,
        };
        if let Some(id) = self.new_shape(kind) {
            self.select(vec![id]);
        }
    }

    /// Graph Paper: a grid of `graph_rows` x `graph_cols` rectangles, grouped.
    pub fn create_graph_paper(&mut self, rect: Rect) {
        let (rows, cols) = (self.graph_rows.max(1), self.graph_cols.max(1));
        let Some(layer) = self.active_layer() else {
            return;
        };
        let cw = rect.width() / cols as f64;
        let ch = rect.height() / rows as f64;
        let mut cmds = Vec::new();
        let mut ids = Vec::new();
        for r in 0..rows {
            for c in 0..cols {
                let cell = Rect::new(
                    rect.x0 + cw * c as f64,
                    rect.y0 + ch * r as f64,
                    rect.x0 + cw * (c + 1) as f64,
                    rect.y0 + ch * (r + 1) as f64,
                );
                let id = self.engine.new_shape_id();
                let mut s = Shape::new(
                    id,
                    ShapeKind::Rect {
                        rect: cell,
                        radius: 0.0,
                        corners: None,
                    },
                );
                s.fill = self.default_fill();
                s.stroke = self
                    .default_stroke()
                    .or_else(|| Some(Stroke::hairline(Color::BLACK)));
                ids.push(id);
                cmds.push(Command::AddShape { layer, shape: s });
            }
        }
        cmds.push(Command::Group { shapes: ids });
        let _ = self.engine.run_batch("Graph Paper", &cmds);
        if let Some(id) = self
            .doc()
            .page(self.page)
            .ok()
            .and_then(|p| p.layers.iter().find(|l| l.id == layer))
            .and_then(|l| l.shapes.last())
            .map(|s| s.id)
        {
            self.select(vec![id]);
        }
    }

    /// Action lines: speed lines across `rect`, parallel (left to right, random
    /// lengths) or radial (from the centre); one combined curve object.
    pub fn create_action_lines(&mut self, rect: Rect) {
        let path = crate::tools2::action_lines_path(
            rect,
            self.action_lines_count,
            self.action_lines_radial,
        );
        if let Some(id) = self.new_shape(ShapeKind::Path {
            path,
            closed: false,
        }) {
            let stroke = self
                .default_stroke()
                .unwrap_or_else(|| Stroke::hairline(Color::BLACK));
            self.run(Command::SetStroke {
                shapes: vec![id],
                stroke: Some(stroke),
            });
            self.run(Command::SetFill {
                shapes: vec![id],
                fill: Fill::None,
            });
            self.select(vec![id]);
        }
    }

    /// 3-point tools: the base segment `a -> b` was dragged, `c` is the third
    /// click. Rectangle and ellipse take the base as one side and `c` as the
    /// height; the curve passes through `c`.
    pub fn finish_three_point(&mut self, a: Point, b: Point, c: Point) {
        use tracedraw_core::geometry::BezPath;
        let base = b - a;
        let len = base.hypot();
        if len < 0.05 {
            return;
        }
        let angle = base.atan2();
        let dir = base / len;
        let normal = Vec2::new(-dir.y, dir.x);
        let height = (c - a).dot(normal);
        match self.tool {
            Tool::ThreePointRectangle | Tool::ThreePointEllipse => {
                if height.abs() < 0.05 {
                    return;
                }
                let local = Rect::new(0.0, height.min(0.0), len, height.max(0.0));
                let kind = if self.tool == Tool::ThreePointRectangle {
                    ShapeKind::rect_with_corners(local, self.rect_corners)
                } else {
                    ShapeKind::Ellipse {
                        rect: local,
                        arc: self.ellipse_arc,
                    }
                };
                if let Some(id) = self.new_shape(kind) {
                    let t = Affine::translate(a.to_vec2()) * Affine::rotate(angle);
                    self.run(Command::TransformShapes {
                        shapes: vec![id],
                        transform: t,
                    });
                    self.select(vec![id]);
                }
            }
            Tool::ThreePointCurve => {
                // Quadratic through c at t = 0.5, written as a cubic.
                let ctrl = Point::new(2.0 * c.x - 0.5 * (a.x + b.x), 2.0 * c.y - 0.5 * (a.y + b.y));
                let c1 = a + (ctrl - a) * (2.0 / 3.0);
                let c2 = b + (ctrl - b) * (2.0 / 3.0);
                let mut path = BezPath::new();
                path.move_to(a);
                path.curve_to(c1, c2, b);
                if let Some(id) = self.new_shape(ShapeKind::Path {
                    path,
                    closed: false,
                }) {
                    self.run(Command::SetFill {
                        shapes: vec![id],
                        fill: Fill::None,
                    });
                    self.select(vec![id]);
                }
            }
            _ => {}
        }
    }

    /// The curve being drawn is done: it continues the selected curve
    /// when it starts or ends on one of its end nodes, otherwise it becomes
    /// a curve of its own (see `curve_join.rs`).
    pub fn finish_curve(&mut self) {
        let Some(c) = self.curve.take() else { return };
        if c.nodes.len() < 2 {
            return;
        }
        self.add_drawn_line(c.path(None));
    }

    /// Start artistic text at a point, or paragraph text in a frame.
    /// Start editing an existing text object in place.
    pub fn begin_text_edit(&mut self, id: ShapeId) {
        let len = match self.doc().find_shape(id).map(|s| &s.kind) {
            Some(ShapeKind::Text { spans, .. }) => tracedraw_core::spans_char_count(spans),
            _ => return,
        };
        self.finish_text();
        self.text_edit = Some(TextEdit::at_end(id, len));
        self.select(vec![id]);
        self.sync_text_defaults_from(id);
    }

    pub fn start_text(&mut self, at: Point, frame: Option<Size>) {
        self.finish_text();
        let span = TextSpan {
            bold: self.text_bold,
            italic: self.text_italic,
            ..TextSpan::new("", self.text_font.clone(), self.text_size_pt)
        };
        let mut para = tracedraw_core::ParagraphStyle::default();
        para.hyphenate = self.text_hyphenation;
        if let Some(id) = self.new_shape(ShapeKind::Text {
            spans: vec![span],
            origin: at,
            frame,
            align: self.text_align,
            para,
            on_path: None,
        }) {
            self.text_edit = Some(TextEdit::at_end(id, 0));
            self.select(vec![id]);
        }
    }

    /// Property bar defaults follow the text object being edited.
    pub fn sync_text_defaults_from(&mut self, id: ShapeId) {
        let kind = self.doc().shape(id).map(|(_, s)| s.kind.clone());
        if let Ok(ShapeKind::Text { spans, align, .. }) = &kind {
            // While editing, the style at the caret (of the character before it).
            let at = self
                .text_edit
                .as_ref()
                .filter(|te| te.shape == id)
                .map(|te| te.caret.saturating_sub(1));
            let span_at = at.and_then(|at| {
                let mut seen = 0usize;
                spans.iter().find(|sp| {
                    let n = sp.char_count();
                    let hit = at < seen + n;
                    seen += n;
                    hit
                })
            });
            if let Some(sp) = span_at.or_else(|| spans.first()) {
                self.text_font = sp.font_family.clone();
                self.text_size_pt = sp.size_pt;
                self.text_bold = sp.bold;
                self.text_italic = sp.italic;
                self.text_underline = sp.underline;
            }
            self.text_align = *align;
        }
    }

    pub fn finish_text(&mut self) {
        let empty = self.edit_text().trim().is_empty();
        if let Some(te) = self.text_edit.take() {
            if empty {
                let _ = self.engine.run(&Command::DeleteShapes {
                    shapes: vec![te.shape],
                });
                self.selection.retain(|s| *s != te.shape);
            }
        }
    }

    // ----- editing -----------------------------------------------------------

    pub fn delete_selection(&mut self) {
        if self.selection.is_empty() {
            return;
        }
        let shapes = std::mem::take(&mut self.selection);
        self.run(Command::DeleteShapes { shapes });
    }

    pub fn select_all(&mut self) {
        if let Ok(p) = self.doc().page(self.page) {
            let ids = p
                .layers
                .iter()
                .filter(|l| !l.locked)
                .flat_map(|l| &l.shapes)
                .map(|s| s.id)
                .collect();
            self.select(ids);
        }
    }

    pub fn copy(&mut self) {
        let shapes = self.selected_shapes();
        if !shapes.is_empty() {
            self.clipboard = Some(Clipboard {
                shapes,
                styles: self.doc().object_styles.clone(),
            });
            self.status = crate::i18n::trf(
                "status.objects_copied",
                &[("n", &self.selection.len().to_string())],
            );
        }
    }

    pub fn cut(&mut self) {
        self.copy_with_system();
        self.delete_selection();
    }

    pub fn paste(&mut self) {
        let Some(cb) = self.clipboard.clone() else {
            return;
        };
        let Some(layer) = self.active_layer() else {
            return;
        };
        let mut ids = Vec::new();
        let mut cmds = Vec::new();
        let here = self.doc().object_styles.clone();
        for s in cb.shapes {
            let id = self.engine.new_shape_id();
            let mut shape = reid(s, id, &mut self.engine);
            crate::styles::keep_links_to(&mut shape, &cb.styles, &here);
            ids.push(id);
            cmds.push(Command::AddShape { layer, shape });
        }
        if let Err(e) = self.engine.run_batch("Paste", &cmds) {
            self.status = e.to_string();
        }
        self.select(ids);
    }

    pub fn duplicate(&mut self) {
        let shapes = self.selected_shapes();
        let Some(layer) = self.active_layer() else {
            return;
        };
        let mut ids = Vec::new();
        let mut cmds = Vec::new();
        for s in shapes {
            let id = self.engine.new_shape_id();
            let mut shape = reid(s, id, &mut self.engine);
            shape.transform = Affine::translate(self.duplicate_offset) * shape.transform;
            ids.push(id);
            cmds.push(Command::AddShape { layer, shape });
        }
        if cmds.is_empty() {
            return;
        }
        if let Err(e) = self.engine.run_batch("Duplicate", &cmds) {
            self.status = e.to_string();
        }
        self.select(ids);
    }

    pub fn group_selection(&mut self) {
        if self.selection.len() < 2 {
            return;
        }
        let shapes = std::mem::take(&mut self.selection);
        self.run(Command::Group { shapes });
        let doc = self.doc();
        if let Ok(p) = doc.page(self.page) {
            if let Some(g) = p
                .layers
                .iter()
                .flat_map(|l| &l.shapes)
                .filter(|s| matches!(s.kind, ShapeKind::Group { .. }))
                .max_by_key(|s| s.id)
            {
                self.selection = vec![g.id];
            }
        }
    }

    pub fn ungroup_selection(&mut self) {
        let groups: Vec<ShapeId> = self
            .selected_shapes()
            .iter()
            .filter(|s| matches!(s.kind, ShapeKind::Group { .. }))
            .map(|s| s.id)
            .collect();
        if groups.is_empty() {
            return;
        }
        // The members come out selected.
        let mut members: Vec<ShapeId> = Vec::new();
        for s in self.selected_shapes() {
            if let ShapeKind::Group { children } = &s.kind {
                members.extend(children.iter().map(|c| c.id));
            }
        }
        self.selection.clear();
        for g in groups {
            self.run(Command::Ungroup { group: g });
        }
        let doc = self.doc();
        members.retain(|id| doc.find_shape(*id).is_some());
        self.select(members);
    }

    pub fn convert_to_curves(&mut self) {
        let shapes = self.selected_shapes();
        let cmds: Vec<Command> = shapes
            .iter()
            .filter(|s| !matches!(s.kind, ShapeKind::Path { .. } | ShapeKind::Group { .. }))
            .map(|s| Command::SetShapeKind {
                shape: s.id,
                kind: ShapeKind::Path {
                    path: s.local_path(),
                    closed: true,
                },
            })
            .collect();
        if !cmds.is_empty() {
            if let Err(e) = self.engine.run_batch("Convert To Curves", &cmds) {
                self.status = e.to_string();
            }
        }
    }

    /// Order: 0 = to front, 1 = forward one, 2 = back one, 3 = to back.
    pub fn order(&mut self, op: u8) {
        let doc = self.doc();
        let mut cmds = Vec::new();
        for id in &self.selection {
            if let Ok((layer_id, idx)) = doc.locate(*id) {
                let len = doc.layer(layer_id).map(|l| l.shapes.len()).unwrap_or(1);
                let target = match op {
                    0 => len.saturating_sub(1),
                    1 => (idx + 1).min(len.saturating_sub(1)),
                    2 => idx.saturating_sub(1),
                    _ => 0,
                };
                if target != idx {
                    cmds.push(Command::Reorder {
                        shape: *id,
                        layer: layer_id,
                        index: target,
                    });
                }
            }
        }
        if !cmds.is_empty() {
            if let Err(e) = self.engine.run_batch("Order", &cmds) {
                self.status = e.to_string();
            }
        }
    }

    pub fn apply_fill(&mut self, fill: Fill) {
        // Monochrome bitmaps take the colour as their background.
        if let Fill::Solid(c) = &fill {
            if self.recolor_mono_bitmaps(false, *c) {
                return;
            }
        }
        if self.selection.is_empty() {
            self.request_default_change(crate::defaults::DefaultChange::Fill(fill));
        } else {
            let shapes = self.selection.clone();
            let used = match &fill {
                Fill::Solid(c) => Some(*c),
                _ => None,
            };
            let before = self.engine.revision();
            self.run(Command::SetFill { shapes, fill });
            if let Some(c) = used.filter(|_| self.engine.revision() != before) {
                self.remember_color(c);
            }
        }
    }

    /// Add a colour to the document palette as a step of its own (its
    /// eyedropper, Add from selection); nothing when it is there already.
    pub fn add_to_document_palette(&mut self, c: Color) {
        if self.doc().palette.contains(&c) {
            return;
        }
        let mut colors = self.doc().palette.clone();
        colors.push(c);
        self.run(Command::SetDocumentPalette { colors });
    }

    /// The document palette collects the colours applied to objects; the addition joins the step that applied it.
    pub fn remember_color(&mut self, c: Color) {
        if !self.settings.palette.auto_update_document
            || matches!(c, Color::Registration)
            || self.doc().palette.contains(&c)
        {
            return;
        }
        let mut colors = self.doc().palette.clone();
        colors.push(c);
        if let Err(e) = self.engine.amend(&Command::SetDocumentPalette { colors }) {
            self.status = e.to_string();
        }
    }

    pub fn apply_outline_color(&mut self, color: Option<Color>) {
        // Monochrome bitmaps take the colour as their foreground.
        if let Some(c) = color {
            if self.recolor_mono_bitmaps(true, c) {
                return;
            }
        }
        let make = |existing: Option<&Stroke>| -> Option<Stroke> {
            color.map(|c| {
                let mut s = existing.cloned().unwrap_or_default();
                s.color = c;
                s
            })
        };
        if self.selection.is_empty() {
            self.request_default_change(crate::defaults::DefaultChange::OutlineColor(color));
        } else {
            let shapes = self.selected_shapes();
            let cmds: Vec<Command> = shapes
                .iter()
                .map(|s| Command::SetStroke {
                    shapes: vec![s.id],
                    stroke: make(s.stroke.as_ref()),
                })
                .collect();
            if let Err(e) = self.engine.run_batch("Outline Color", &cmds) {
                self.status = e.to_string();
            } else if let Some(c) = color {
                self.remember_color(c);
            }
        }
    }

    pub fn apply_outline_width(&mut self, width: f64) {
        let shapes = self.selected_shapes();
        if shapes.is_empty() {
            self.request_default_change(crate::defaults::DefaultChange::OutlineWidth(width));
            return;
        }
        let cmds: Vec<Command> = shapes
            .iter()
            .map(|s| {
                let mut st = s.stroke.clone().unwrap_or_default();
                st.width = width;
                Command::SetStroke {
                    shapes: vec![s.id],
                    stroke: Some(st),
                }
            })
            .collect();
        if let Err(e) = self.engine.run_batch("Outline Width", &cmds) {
            self.status = e.to_string();
        }
    }

    pub fn transform_selection(&mut self, t: Affine) {
        if self.selection.is_empty() {
            return;
        }
        let shapes = self.selection.clone();
        // Paragraph text takes a scale as a new frame (see text_frames.rs).
        if !self.transform_with_frames(&shapes, t) {
            self.run(Command::TransformShapes {
                shapes: shapes.clone(),
                transform: t,
            });
        }
        self.compensate_unlocked_clip_frames(&shapes, t);
        self.apply_hinting(&shapes);
        self.reflow_chains_of(&shapes);
    }

    pub fn nudge(&mut self, d: Vec2) {
        self.transform_selection(Affine::translate(d));
    }

    pub fn mirror(&mut self, horizontal: bool) {
        let Some(b) = self.selection_bounds() else {
            return;
        };
        let c = b.center();
        let t = if horizontal {
            Affine::translate((c.x, 0.0))
                * Affine::scale_non_uniform(-1.0, 1.0)
                * Affine::translate((-c.x, 0.0))
        } else {
            Affine::translate((0.0, c.y))
                * Affine::scale_non_uniform(1.0, -1.0)
                * Affine::translate((0.0, -c.y))
        };
        self.transform_selection(t);
    }

    pub fn undo(&mut self) {
        self.finish_text();
        match self.engine.undo() {
            Ok(l) => self.status = crate::i18n::trf("status.undo_n", &[("l", l)]),
            Err(e) => self.status = e.to_string(),
        }
        self.selection
            .retain(|id| self.engine.document().find_shape(*id).is_some());
    }

    pub fn redo(&mut self) {
        match self.engine.redo() {
            Ok(l) => self.status = crate::i18n::trf("status.redo_n", &[("l", l)]),
            Err(e) => self.status = e.to_string(),
        }
        self.selection
            .retain(|id| self.engine.document().find_shape(*id).is_some());
    }

    // ----- files -------------------------------------------------------------

    /// A new drawing in its own tab, with the settings last used in the
    /// Create a New Document dialog.
    pub fn new_document(&mut self) {
        let name = self.next_untitled_name();
        let mut doc = self.settings.new_document.build(name);
        self.settings.document_defaults.apply(&mut doc);
        self.open_document(doc, None);
        self.status.clear();
    }

    /// File > New, Ctrl+N, the New button and the "+" tab: the Create a
    /// New Document dialog, unless it was turned off, then a new drawing
    /// with the last used settings.
    pub fn request_new_document(&mut self) {
        if self.settings.show_new_document_dialog {
            self.dialog = crate::ui::dialogs::Dialog::NewDocument(
                crate::new_document::NewDocState::from_settings(
                    &self.settings.new_document,
                    self.peek_untitled_name(),
                ),
            );
        } else {
            self.new_document();
        }
    }

    pub fn open_dialog(&mut self) {
        crate::files::Dialog::new()
            .add_filter(
                crate::i18n::tr("file.all_supported"),
                &[
                    "cdr", "tdraw", "svg", "svgz", "pdf", "ai", "eps", "ps", "prn", "dxf", "psd",
                    "psb", "emf", "wmf", "plt", "hpgl", "hgl", "txt", "rtf", "docx",
                ],
            )
            .add_filter("HPGL plotter (*.plt, *.hpgl)", &["plt", "hpgl", "hgl"])
            .add_filter("Text (*.txt, *.rtf, *.docx)", &["txt", "rtf", "docx"])
            .add_filter("SVG (*.svg, *.svgz)", &["svg", "svgz"])
            .add_filter("PDF, AI (*.pdf, *.ai)", &["pdf", "ai"])
            .add_filter("PostScript (*.ps, *.eps, *.prn)", &["ps", "eps", "prn"])
            .add_filter("DXF (*.dxf)", &["dxf"])
            .add_filter("Windows Metafile (*.emf, *.wmf)", &["emf", "wmf"])
            .add_filter("Photoshop (*.psd, *.psb)", &["psd", "psb"])
            .add_filter(crate::i18n::tr("file.cdr_files"), &["cdr"])
            .add_filter("TraceDraw (*.tdraw)", &["tdraw"])
            .pick_file(self, |app, p| app.open_path(p));
    }

    pub fn open_path(&mut self, path: PathBuf) {
        self.open_path_with_password(path, "");
    }

    /// The kind of file `path` holds, by its extension, except that an
    /// `.ai` file written as PostScript (the format's versions before the
    /// PDF-based ones) reads as EPS.
    pub(crate) fn file_kind(path: &std::path::Path) -> String {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if ext == "ai" && crate::files::read(path).is_ok_and(|b| b.starts_with(b"%!")) {
            return "eps".into();
        }
        // Print files: PostScript, often after a printer job header.
        if ext == "prn" {
            return "ps".into();
        }
        ext
    }

    /// Open `path`; `password` opens a protected PDF (empty: none).
    pub fn open_path_with_password(&mut self, path: PathBuf, password: &str) {
        let ext = Self::file_kind(&path);
        // Objects of a `.cdr` file left out because they could not be read.
        let mut unread = 0usize;
        // The version of an opened `.cdr` file: saving keeps it.
        let mut cdr_version = None;
        let result = if ext == "svg" || ext == "svgz" {
            Self::read_svg_text(&path)
                .and_then(|text| {
                    tracedraw_io::svg_import::parse(
                        &text,
                        &mut tracedraw_core::id::IdSource::default(),
                    )
                })
                .map(|imported| {
                    let title = path
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(App::untitled_name);
                    let mut doc = App::localized_document(title, imported.size);
                    let mut ids = doc.ids().clone();
                    let n = imported.shapes.len();
                    for s in imported.shapes {
                        let shape = reid_with(s, &mut ids);
                        doc.pages[0].layers[0].shapes.push(shape);
                    }
                    doc.set_ids(ids);
                    (doc, opened_objects("SVG", n))
                })
        } else if ext == "psd" || ext == "psb" {
            crate::files::read(&path)
                .map_err(|e| e.to_string())
                .and_then(|bytes| {
                    tracedraw_io::psd::parse(&bytes, &mut tracedraw_core::id::IdSource::default())
                })
                .map(|imported| {
                    let title = path
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(App::untitled_name);
                    for w in &imported.warnings {
                        log::warn!("psd: {w}");
                    }
                    let mut doc = App::localized_document(title, imported.size);
                    let mut ids = doc.ids().clone();
                    let n = imported.shapes.len();
                    for s in imported.shapes {
                        let shape = reid_with(s, &mut ids);
                        doc.pages[0].layers[0].shapes.push(shape);
                    }
                    doc.set_ids(ids);
                    (doc, format!("PSD: {n} layer(s)"))
                })
        } else if ext == "emf" || ext == "wmf" {
            crate::files::read(&path)
                .map_err(|e| e.to_string())
                .and_then(|bytes| {
                    tracedraw_io::emf::parse(&bytes, &mut tracedraw_core::id::IdSource::default())
                })
                .map(|imported| {
                    let title = path
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(App::untitled_name);
                    for w in &imported.warnings {
                        log::warn!("metafile: {w}");
                    }
                    let mut doc = App::localized_document(title, imported.size);
                    let mut ids = doc.ids().clone();
                    let n = imported.shapes.len();
                    for s in imported.shapes {
                        let shape = reid_with(s, &mut ids);
                        doc.pages[0].layers[0].shapes.push(shape);
                    }
                    doc.set_ids(ids);
                    (doc, opened_objects(&ext.to_ascii_uppercase(), n))
                })
        } else if ext == "eps" || ext == "ps" {
            crate::files::read(&path)
                .map_err(|e| e.to_string())
                .and_then(|bytes| {
                    tracedraw_io::eps_import::parse(
                        &bytes,
                        &mut tracedraw_core::id::IdSource::default(),
                    )
                })
                .map(|imported| {
                    let title = path
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(App::untitled_name);
                    for w in &imported.warnings {
                        log::warn!("eps: {w}");
                    }
                    // Every page of a PostScript file (print files have
                    // several), each at its own size.
                    let pages = imported.pages();
                    let first = pages
                        .first()
                        .map(|p| p.1)
                        .unwrap_or(tracedraw_core::document::paper::A4);
                    let mut doc = App::localized_document(title, first);
                    let layer_name = doc.pages[0].layers[0].name.clone();
                    let mut ids = doc.ids().clone();
                    let count = pages.len();
                    let mut n = 0;
                    for (i, (shapes, size)) in pages.into_iter().enumerate() {
                        n += shapes.len();
                        let shapes: Vec<_> =
                            shapes.into_iter().map(|s| reid_with(s, &mut ids)).collect();
                        if i == 0 {
                            doc.pages[0].layers[0].shapes = shapes;
                            continue;
                        }
                        let mut layer =
                            tracedraw_core::document::Layer::new(ids.layer(), layer_name.clone());
                        layer.shapes = shapes;
                        doc.pages.push(tracedraw_core::document::Page {
                            id: ids.page(),
                            name: crate::i18n::trf("doc.page_n", &[("n", &(i + 1).to_string())]),
                            size,
                            layers: vec![layer],
                            guides: Vec::new(),
                            background: None,
                        });
                    }
                    doc.set_ids(ids);
                    let format = if ext == "eps" { "EPS" } else { "PostScript" };
                    let msg = if count > 1 {
                        crate::i18n::trf(
                            "status.opened_pages",
                            &[
                                ("format", format),
                                ("pages", &count.to_string()),
                                ("objects", &n.to_string()),
                            ],
                        )
                    } else {
                        opened_objects(format, n)
                    };
                    (doc, msg)
                })
        } else if matches!(ext.as_str(), "plt" | "hpgl" | "hgl") {
            crate::files::read(&path)
                .map_err(|e| e.to_string())
                .and_then(|bytes| {
                    tracedraw_io::plt::parse(&bytes, &mut tracedraw_core::id::IdSource::default())
                })
                .map(|imported| {
                    let title = path
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(App::untitled_name);
                    for w in &imported.warnings {
                        log::warn!("plt: {w}");
                    }
                    let mut doc = App::localized_document(title, imported.size);
                    let mut ids = doc.ids().clone();
                    let n = imported.shapes.len();
                    for s in imported.shapes {
                        let shape = reid_with(s, &mut ids);
                        doc.pages[0].layers[0].shapes.push(shape);
                    }
                    doc.set_ids(ids);
                    (doc, format!("HPGL: {n} stroke(s)"))
                })
        } else if matches!(ext.as_str(), "txt" | "rtf" | "docx") {
            Self::parse_text_file(&path).map(|imported| {
                let title = path
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(App::untitled_name);
                for w in &imported.warnings {
                    log::warn!("text: {w}");
                }
                let n = imported.text().chars().count();
                let mut doc = App::localized_document(title, tracedraw_core::document::paper::A4);
                let mut ids = doc.ids().clone();
                let mut shape =
                    imported.to_shape(tracedraw_io::text_import::Placement::a4(), &mut ids);
                shape.id = ids.shape();
                shape.fill = Fill::Solid(Color::BLACK);
                shape.stroke = None;
                doc.pages[0].layers[0].shapes.push(shape);
                doc.set_ids(ids);
                (doc, format!("Text: {n} character(s)"))
            })
        } else if ext == "dxf" {
            crate::files::read(&path)
                .map_err(|e| e.to_string())
                .and_then(|bytes| {
                    let text = String::from_utf8_lossy(&bytes).into_owned();
                    tracedraw_io::dxf::parse(&text, &mut tracedraw_core::id::IdSource::default())
                })
                .map(|imported| {
                    let title = path
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(App::untitled_name);
                    for w in &imported.warnings {
                        log::warn!("dxf: {w}");
                    }
                    let n = imported.shapes.len();
                    let doc = tracedraw_io::dxf::to_document(imported, &title);
                    let mut doc2 = App::localized_document(title, doc.pages[0].size);
                    let mut ids = tracedraw_core::id::IdSource::default();
                    let pid = ids.page();
                    doc2.pages[0].id = pid;
                    doc2.pages[0].layers.clear();
                    for l in &doc.pages[0].layers {
                        let mut layer =
                            tracedraw_core::document::Layer::new(ids.layer(), l.name.clone());
                        for s in &l.shapes {
                            layer.shapes.push(reid_with(s.clone(), &mut ids));
                        }
                        doc2.pages[0].layers.push(layer);
                    }
                    doc2.set_ids(ids);
                    (doc2, opened_objects("DXF", n))
                })
        } else if ext == "pdf" || ext == "ai" {
            use tracedraw_io::pdf_import::ImportError;
            let parsed = crate::files::read(&path)
                .map_err(|e| ImportError::Failed(e.to_string()))
                .and_then(|bytes| {
                    tracedraw_io::pdf_import::parse_with_password(
                        &bytes,
                        &mut tracedraw_core::id::IdSource::default(),
                        password,
                    )
                });
            if matches!(parsed, Err(ImportError::NeedsPassword)) {
                self.ask_pdf_password(path, false, !password.is_empty());
                return;
            }
            parsed.map_err(|e| e.to_string()).map(|imported| {
                let title = path
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(App::untitled_name);
                for w in &imported.warnings {
                    log::warn!("pdf: {w}");
                }
                let pages = imported.pages.len();
                let n: usize = imported.pages.iter().map(|p| p.shapes.len()).sum();
                let title = imported.title.clone().unwrap_or(title);
                let first = imported
                    .pages
                    .first()
                    .map(|p| p.size)
                    .unwrap_or(tracedraw_core::document::paper::A4);
                let mut doc2 = App::localized_document(title, first);
                let layer_name = doc2.pages[0].layers[0].name.clone();
                doc2.pages.clear();
                let mut ids = tracedraw_core::id::IdSource::default();
                for (i, p) in imported.pages.into_iter().enumerate() {
                    let pid = ids.page();
                    let lid = ids.layer();
                    let mut layer = tracedraw_core::document::Layer::new(lid, layer_name.clone());
                    for s in p.shapes {
                        layer.shapes.push(reid_with(s, &mut ids));
                    }
                    doc2.pages.push(tracedraw_core::document::Page {
                        id: pid,
                        name: crate::i18n::trf("doc.page_n", &[("n", &(i + 1).to_string())]),
                        size: p.size,
                        layers: vec![layer],
                        guides: Vec::new(),
                        background: None,
                    });
                }
                doc2.set_ids(ids);
                (
                    doc2,
                    crate::i18n::trf(
                        "status.opened_pages",
                        &[
                            ("format", "PDF"),
                            ("pages", &pages.to_string()),
                            ("objects", &n.to_string()),
                        ],
                    ),
                )
            })
        } else if ext == "cdr" {
            Self::open_cdr(&path)
                .map(|(doc, report)| {
                    let ver = report.version.map(|v| v.name()).unwrap_or_default();
                    for w in &report.warnings {
                        log::warn!("cdr: {w}");
                    }
                    unread = report.skipped_objects;
                    cdr_version = report.version.map(|v| writable_cdr_version(v.0));
                    (
                        doc,
                        crate::i18n::trf(
                            "status.opened_cdr",
                            &[
                                ("version", &ver.to_string()),
                                ("pages", &report.pages.to_string()),
                                ("objects", &report.shapes.to_string()),
                                ("skipped", &report.skipped_objects.to_string()),
                            ],
                        ),
                    )
                })
                .map_err(|e| e.to_string())
        } else {
            crate::files::read_to_string(&path)
                .map_err(|e| e.to_string())
                .and_then(|json| {
                    tracedraw_core::Document::from_json(&json).map_err(|e| e.to_string())
                })
                .map(|d| (d, crate::i18n::tr("status.opened")))
        };
        match result {
            Ok((doc, msg)) => {
                self.open_document(doc, None);
                self.cdr_version = cdr_version;
                self.file = if matches!(
                    ext.as_str(),
                    "cdr"
                        | "svg"
                        | "svgz"
                        | "pdf"
                        | "ai"
                        | "eps"
                        | "ps"
                        | "dxf"
                        | "psd"
                        | "psb"
                        | "emf"
                        | "wmf"
                        | "plt"
                        | "hpgl"
                        | "hgl"
                        | "txt"
                        | "rtf"
                        | "docx"
                ) {
                    None
                } else {
                    Some(path.clone())
                };
                self.status = format!("{}: {msg}", path.display());
                self.fit_pending = true;
                self.settings.touch_recent(&path);
                self.settings.last_dir = path.parent().map(|p| p.to_path_buf());
                self.settings.save();
                self.store_thumbnail(&path);
                self.show_welcome = false;
                self.check_missing_fonts();
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                self.show_open_notes(name, unread);
            }
            Err(e) => {
                self.status = crate::i18n::trf(
                    "status.could_not_open",
                    &[("p", &path.display().to_string()), ("e", &e.to_string())],
                )
            }
        }
    }

    pub fn save(&mut self, save_as: bool) {
        // In a browser every save is a download, so there is no path to
        // write back to.
        if !save_as && !crate::files::WEB {
            if let Some(path) = self.file.clone() {
                self.save_to(path);
                return;
            }
        }
        self.save_as_dialog();
    }

    /// The `.cdr` version the active drawing is saved in.
    pub fn cdr_save_version(&self) -> u16 {
        writable_cdr_version(
            self.cdr_version
                .unwrap_or(self.settings.formats.cdr_version),
        )
    }

    /// Save to the `.cdr` file `path` in `version`, which the drawing then
    /// keeps (Save writes it again) and new drawings start with.
    pub(crate) fn save_cdr_to(&mut self, path: PathBuf, version: u16) {
        let version = writable_cdr_version(version);
        self.cdr_version = Some(version);
        self.settings.formats.cdr_version = version;
        self.save_to(path);
    }

    pub(crate) fn save_to(&mut self, path: PathBuf) {
        // Time counted for the active task goes into the file too.
        self.flush_tracked_time();
        let bytes = if is_cdr_path(&path) {
            Ok(tracedraw_cdr::write::document_to_cdr_version(
                self.engine.document(),
                self.cdr_save_version(),
            ))
        } else {
            // Fonts that came with other drawings go along (text imported
            // or pasted from them).
            tracedraw_text::fonts()
                .with_carried_fonts(self.engine.document())
                .to_json()
                .map(String::into_bytes)
                .map_err(|e| e.to_string())
        };
        self.backup_before_save(&path);
        let result = bytes.and_then(|b| crate::files::write(&path, b).map_err(|e| e.to_string()));
        match result {
            Ok(()) => {
                self.engine.mark_saved();
                self.file = Some(path.clone());
                self.status =
                    crate::i18n::trf("status.saved", &[("p", &path.display().to_string())]);
                self.settings.touch_recent(&path);
                self.settings.save();
                self.store_thumbnail(&path);
            }
            Err(e) => {
                self.status = crate::i18n::trf("status.save_failed", &[("e", &e.to_string())])
            }
        }
    }

    /// Options > Save: copy the file about to be replaced to
    /// `backup_of_<name>`, next to it or in the chosen folder.
    fn backup_before_save(&mut self, path: &std::path::Path) {
        let b = &self.settings.backup;
        if !b.before_save || !crate::files::exists(path) {
            return;
        }
        let Some(name) = path.file_name() else {
            return;
        };
        let dir = b
            .before_save_dir
            .clone()
            .or_else(|| path.parent().map(|p| p.to_path_buf()))
            .unwrap_or_default();
        let target = dir.join(format!("backup_of_{}", name.to_string_lossy()));
        if let Err(e) = crate::files::copy(path, &target) {
            log::warn!("backup of {} failed: {e}", path.display());
        }
    }

    /// Options > Save > Auto-backup: every few minutes, save each open
    /// drawing with unsaved changes as `AutoBackup_of_<name>.tdraw` in the
    /// auto-backup folder. Called every frame; returns true when it saved.
    pub fn auto_backup_tick(&mut self, now: web_time::Instant) -> bool {
        let b = self.settings.backup.clone();
        if !b.auto || crate::files::WEB {
            return false;
        }
        let due = std::time::Duration::from_secs(u64::from(b.minutes.max(1)) * 60);
        let last = *self.last_auto_backup.get_or_insert(now);
        if now.duration_since(last) < due {
            return false;
        }
        self.last_auto_backup = Some(now);
        let Some(dir) = b.auto_dir.clone().or_else(App::auto_backup_dir_default) else {
            return false;
        };
        let mut saved = false;
        let mut jobs: Vec<(String, String)> = Vec::new();
        let fonts = tracedraw_text::fonts();
        if self.has_document() && self.engine.is_dirty() {
            if let Ok(json) = fonts.with_carried_fonts(self.doc()).to_json() {
                jobs.push((self.backup_name(None), json));
            }
        }
        for (i, slot) in self.docs.iter().enumerate() {
            if i != self.active_doc && slot.engine.is_dirty() {
                if let Ok(json) = fonts.with_carried_fonts(slot.engine.document()).to_json() {
                    jobs.push((self.backup_name(Some(i)), json));
                }
            }
        }
        for (name, json) in jobs {
            match crate::files::write_creating(dir.join(&name), json) {
                Ok(()) => saved = true,
                Err(e) => log::warn!("auto-backup {name} failed: {e}"),
            }
        }
        saved
    }

    /// `AutoBackup_of_<drawing name>.tdraw` for the active drawing (`None`)
    /// or another open one.
    fn backup_name(&self, slot: Option<usize>) -> String {
        let title = match slot {
            None => self.document_tab_title(self.active_doc),
            Some(i) => self.document_tab_title(i),
        };
        let stem: String = title
            .trim_end_matches('*')
            .trim()
            .trim_end_matches(".tdraw")
            .trim_end_matches(".cdr")
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        format!("AutoBackup_of_{stem}.tdraw")
    }

    /// Ctrl+8 and Ctrl+2: the selected text (or the text being edited)
    /// one keyboard text increment larger or smaller.
    pub fn step_text_size(&mut self, up: bool) {
        let step = self.settings.text_increment_pt.max(0.1);
        let size = if up {
            self.text_size_pt + step
        } else {
            self.text_size_pt - step
        };
        self.text_size_pt = size.clamp(1.0, 3000.0);
        self.apply_text_style();
    }

    /// Put `contents` inside `frame` (a clip frame), centring them in the
    /// frame first when Options > Clip Frames asks for it: always, or when
    /// they lie completely outside the frame (the default).
    pub fn place_inside(&mut self, contents: Vec<ShapeId>, frame: ShapeId) {
        use crate::settings::AutoCenter;
        let bounds = |app: &App, ids: &[ShapeId]| -> Option<Rect> {
            ids.iter()
                .filter_map(|id| app.doc().shape(*id).ok().map(|(_, s)| s.bounds()))
                .reduce(|a, b| a.union(b))
        };
        let mut cmds = Vec::new();
        if let (Some(c), Some(f)) = (bounds(self, &contents), bounds(self, &[frame])) {
            let center = match self.settings.clip_frame.auto_center {
                AutoCenter::Always => true,
                AutoCenter::Never => false,
                AutoCenter::WhenOutside => c.intersect(f).area() <= 0.0,
            };
            if center {
                let d = f.center() - c.center();
                cmds.push(Command::TransformShapes {
                    shapes: contents.clone(),
                    transform: Affine::translate(d),
                });
            }
        }
        cmds.push(Command::PlaceInside { contents, frame });
        if let Err(e) = self.engine.run_batch("Place Inside Frame", &cmds) {
            self.status = format!("ClipFrame: {e}");
        }
    }

    /// File > Export (Ctrl+E, the toolbar button): the Export dialog with
    /// every format, starting with the choices of Options > Global > File
    /// Formats.
    pub fn export(&mut self) {
        self.dialog = crate::ui::dialogs::Dialog::Export(self.settings.formats.state());
    }

    /// After a drawing opened: say which objects could not be read and
    /// which fonts are missing, as far as Options > Global > Warnings asks
    /// for it, unless another dialog is up.
    pub fn show_open_notes(&mut self, name: String, unread: usize) {
        if !matches!(self.dialog, crate::ui::dialogs::Dialog::None) {
            return;
        }
        let w = &self.settings.warnings;
        let notes = crate::ui::dialogs::OpenNotes {
            name,
            unread: if w.unread_objects { unread } else { 0 },
            fonts: if w.missing_fonts {
                self.missing_fonts.clone()
            } else {
                Vec::new()
            },
            dont_show: false,
        };
        if notes.unread > 0 || !notes.fonts.is_empty() {
            self.dialog = crate::ui::dialogs::Dialog::OpenNotes(notes);
        }
    }

    /// File > Print (Ctrl+P, the toolbar button): the Print dialog,
    /// starting with the choices of Options > Global > Printing.
    pub fn print(&mut self) {
        self.dialog = crate::ui::dialogs::Dialog::Print(self.settings.printing.state());
    }

    pub fn import(&mut self) {
        crate::files::Dialog::new()
            .add_filter(
                crate::i18n::tr("file.all_importable"),
                &[
                    "cdr", "svg", "svgz", "pdf", "ai", "eps", "ps", "prn", "dxf", "psd", "psb",
                    "emf", "wmf", "plt", "hpgl", "hgl", "txt", "rtf", "docx", "png", "jpg", "jpeg",
                    "bmp", "gif", "webp", "tif", "tiff",
                ],
            )
            .add_filter("HPGL plotter (*.plt, *.hpgl)", &["plt", "hpgl", "hgl"])
            .add_filter("Text (*.txt, *.rtf, *.docx)", &["txt", "rtf", "docx"])
            .add_filter(crate::i18n::tr("file.cdr_files"), &["cdr"])
            .add_filter("SVG (*.svg, *.svgz)", &["svg", "svgz"])
            .add_filter("PDF, AI (*.pdf, *.ai)", &["pdf", "ai"])
            .add_filter("PostScript (*.ps, *.eps, *.prn)", &["ps", "eps", "prn"])
            .add_filter("DXF (*.dxf)", &["dxf"])
            .add_filter("Windows Metafile (*.emf, *.wmf)", &["emf", "wmf"])
            .add_filter("Photoshop (*.psd, *.psb)", &["psd", "psb"])
            .add_filter(
                crate::i18n::tr("file.images"),
                &["png", "jpg", "jpeg", "bmp", "gif", "webp", "tif", "tiff"],
            )
            .pick_file(self, |app, path| app.import_path(&path));
    }

    /// Import a file into the active layer, by extension.
    /// File > Import of `path`: the objects are added to the active
    /// layer and selected, as one undo step; Options > Global > File
    /// Formats may centre them in the view.
    pub fn import_path(&mut self, path: &std::path::Path) {
        let before = self.selection.clone();
        self.import_path_as_in_file(path);
        if self.settings.formats.import_place != crate::prefs::ImportPlace::ViewCenter
            || self.selection == before
            || self.selection.is_empty()
        {
            return;
        }
        let Some(b) = self.selection_bounds() else {
            return;
        };
        let view = self.view.visible_page_rect(self.canvas_rect).center();
        let d = view - b.center();
        if !(d.x.is_finite() && d.y.is_finite()) {
            return;
        }
        // Part of the import's own undo step.
        let cmd = Command::TransformShapes {
            shapes: self.selection.clone(),
            transform: Affine::translate(d),
        };
        if let Err(e) = self.engine.amend(&cmd) {
            self.status = e.to_string();
        }
    }

    fn import_path_as_in_file(&mut self, path: &std::path::Path) {
        let path = path.to_path_buf();
        let ext = Self::file_kind(&path);
        if ext == "svg" || ext == "svgz" {
            self.import_svg(&path);
            return;
        }
        if ext == "pdf" || ext == "ai" {
            self.import_pdf(&path);
            return;
        }
        if ext == "dxf" {
            self.import_dxf(&path);
            return;
        }
        if ext == "emf" || ext == "wmf" {
            self.import_metafile(&path);
            return;
        }
        if matches!(ext.as_str(), "plt" | "hpgl" | "hgl") {
            self.import_plt(&path);
            return;
        }
        if matches!(ext.as_str(), "txt" | "rtf" | "docx") {
            self.import_text_file(&path);
            return;
        }
        if ext == "eps" || ext == "ps" {
            self.import_eps(&path);
            return;
        }
        if ext == "psd" || ext == "psb" {
            self.import_psd(&path);
            return;
        }
        if ext != "cdr" {
            self.import_bitmap(&path);
            return;
        }
        match Self::open_cdr(&path) {
            Ok((doc, _)) => {
                let Some(layer) = self.active_layer() else {
                    return;
                };
                // The imported text keeps its letters; saving the drawing
                // carries the fonts along.
                self.add_carried_fonts(&doc.fonts);
                let mut cmds = Vec::new();
                let mut ids = Vec::new();
                for s in doc
                    .pages
                    .iter()
                    .flat_map(|p| &p.layers)
                    .flat_map(|l| &l.shapes)
                {
                    let id = self.engine.new_shape_id();
                    let shape = reid(s.clone(), id, &mut self.engine);
                    ids.push(id);
                    cmds.push(Command::AddShape { layer, shape });
                }
                if let Err(e) = self.engine.run_batch("Import", &cmds) {
                    self.status = e.to_string();
                }
                self.select(ids);
            }
            Err(e) => {
                self.status = crate::i18n::trf("status.import_failed", &[("e", &e.to_string())])
            }
        }
    }

    /// Read a `.cdr` file through the file layer (uploads in a browser).
    fn open_cdr(
        path: &std::path::Path,
    ) -> std::result::Result<(tracedraw_core::Document, tracedraw_cdr::ParseReport), String> {
        let bytes = crate::files::read(path).map_err(|e| e.to_string())?;
        let title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled")
            .to_string();
        tracedraw_cdr::open_bytes(&bytes, &title).map_err(|e| e.to_string())
    }

    /// Read an SVG file (plain or gzip-compressed) as text.
    fn read_svg_text(path: &std::path::Path) -> std::result::Result<String, String> {
        let bytes = crate::files::read(path).map_err(|e| e.to_string())?;
        if bytes.len() > 2 && bytes[0] == 0x1f && bytes[1] == 0x8b {
            use std::io::Read;
            let mut out = String::new();
            flate2::read::GzDecoder::new(&bytes[..])
                .read_to_string(&mut out)
                .map_err(|e| e.to_string())?;
            Ok(out)
        } else {
            String::from_utf8(bytes).map_err(|e| e.to_string())
        }
    }

    /// Import an SVG into the active layer, keeping its groups and clips.
    pub fn import_svg(&mut self, path: &std::path::Path) {
        let Some(layer) = self.active_layer() else {
            return;
        };
        let parsed = Self::read_svg_text(path).and_then(|text| {
            tracedraw_io::svg_import::parse(&text, &mut tracedraw_core::id::IdSource::default())
        });
        match parsed {
            Ok(imported) => {
                let mut cmds = Vec::new();
                let mut ids = Vec::new();
                for s in imported.shapes {
                    let id = self.engine.new_shape_id();
                    let shape = reid(s, id, &mut self.engine);
                    ids.push(id);
                    cmds.push(Command::AddShape { layer, shape });
                }
                if let Err(e) = self.engine.run_batch("Import", &cmds) {
                    self.status = e.to_string();
                }
                self.select(ids);
            }
            Err(e) => self.status = crate::i18n::trf("status.import_failed", &[("e", &e)]),
        }
    }

    /// Import the first page of a PDF or AI file into the active layer.
    pub fn import_pdf(&mut self, path: &std::path::Path) {
        self.import_pdf_with_password(path, "");
    }

    /// [`App::import_pdf`] with the password of a protected file.
    pub fn import_pdf_with_password(&mut self, path: &std::path::Path, password: &str) {
        use tracedraw_io::pdf_import::ImportError;
        let Some(layer) = self.active_layer() else {
            return;
        };
        let parsed = crate::files::read(path)
            .map_err(|e| ImportError::Failed(e.to_string()))
            .and_then(|bytes| {
                tracedraw_io::pdf_import::parse_with_password(
                    &bytes,
                    &mut tracedraw_core::id::IdSource::default(),
                    password,
                )
            });
        if matches!(parsed, Err(ImportError::NeedsPassword)) {
            self.ask_pdf_password(path.to_path_buf(), true, !password.is_empty());
            return;
        }
        let parsed = parsed.map_err(|e| e.to_string());
        match parsed {
            Ok(imported) => {
                let mut cmds = Vec::new();
                let mut ids = Vec::new();
                let Some(first) = imported.pages.into_iter().next() else {
                    return;
                };
                for s in first.shapes {
                    let id = self.engine.new_shape_id();
                    let shape = reid(s, id, &mut self.engine);
                    ids.push(id);
                    cmds.push(Command::AddShape { layer, shape });
                }
                if let Err(e) = self.engine.run_batch("Import", &cmds) {
                    self.status = e.to_string();
                }
                if !imported.warnings.is_empty() {
                    log::warn!("pdf import: {}", imported.warnings.join("; "));
                }
                self.select(ids);
            }
            Err(e) => self.status = crate::i18n::trf("status.import_failed", &[("e", &e)]),
        }
    }

    /// Import a Photoshop file: one bitmap per layer, grouped.
    pub fn import_psd(&mut self, path: &std::path::Path) {
        let Some(layer) = self.active_layer() else {
            return;
        };
        let parsed = crate::files::read(path)
            .map_err(|e| e.to_string())
            .and_then(|bytes| {
                tracedraw_io::psd::parse(&bytes, &mut tracedraw_core::id::IdSource::default())
            });
        match parsed {
            Ok(imported) => {
                let mut children = Vec::new();
                for s in imported.shapes {
                    let id = self.engine.new_shape_id();
                    children.push(reid(s, id, &mut self.engine));
                }
                let shape = match children.len() {
                    0 => return,
                    1 => children.remove(0),
                    _ => {
                        let id = self.engine.new_shape_id();
                        let mut g = Shape::new(id, ShapeKind::Group { children });
                        g.fill = Fill::None;
                        g.stroke = None;
                        g.name = path.file_stem().map(|s| s.to_string_lossy().to_string());
                        g
                    }
                };
                let id = shape.id;
                self.run(Command::AddShape { layer, shape });
                self.select(vec![id]);
            }
            Err(e) => self.status = crate::i18n::trf("status.import_failed", &[("e", &e)]),
        }
    }

    /// Import an HPGL plotter file into the active layer.
    pub fn import_plt(&mut self, path: &std::path::Path) {
        let Some(layer) = self.active_layer() else {
            return;
        };
        let parsed = crate::files::read(path)
            .map_err(|e| e.to_string())
            .and_then(|bytes| {
                tracedraw_io::plt::parse(&bytes, &mut tracedraw_core::id::IdSource::default())
            });
        match parsed {
            Ok(imported) => {
                let mut cmds = Vec::new();
                let mut ids = Vec::new();
                for s in imported.shapes {
                    let id = self.engine.new_shape_id();
                    let shape = reid(s, id, &mut self.engine);
                    ids.push(id);
                    cmds.push(Command::AddShape { layer, shape });
                }
                if let Err(e) = self.engine.run_batch("Import", &cmds) {
                    self.status = e.to_string();
                }
                for w in &imported.warnings {
                    log::warn!("plt import: {w}");
                }
                self.select(ids);
            }
            Err(e) => self.status = crate::i18n::trf("status.import_failed", &[("e", &e)]),
        }
    }

    /// Parse a text document (TXT, RTF, DOCX) by extension.
    fn parse_text_file(
        path: &std::path::Path,
    ) -> Result<tracedraw_io::text_import::ImportedText, String> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let bytes = crate::files::read(path).map_err(|e| e.to_string())?;
        match ext.as_str() {
            "rtf" => tracedraw_io::text_import::parse_rtf(&bytes),
            "docx" => tracedraw_io::text_import::parse_docx(&bytes),
            _ => Ok(tracedraw_io::text_import::parse_txt(&bytes)),
        }
    }

    /// Import a text document (TXT, RTF, DOCX) as a paragraph text frame
    /// filling the page inside 20 mm margins.
    pub fn import_text_file(&mut self, path: &std::path::Path) {
        let Some(layer) = self.active_layer() else {
            return;
        };
        match Self::parse_text_file(path) {
            Ok(imported) => {
                for w in &imported.warnings {
                    log::warn!("text import: {w}");
                }
                let page = self.page_rect();
                let margin = 20.0_f64.min(page.width() / 4.0).min(page.height() / 4.0);
                let place = tracedraw_io::text_import::Placement {
                    origin_top_left: Point::new(page.x0 + margin, page.y1 - margin),
                    size: Size::new(page.width() - 2.0 * margin, page.height() - 2.0 * margin),
                };
                let mut ids = tracedraw_core::id::IdSource::default();
                let shape = imported.to_shape(place, &mut ids);
                let id = self.engine.new_shape_id();
                let mut shape = reid(shape, id, &mut self.engine);
                shape.fill = Fill::Solid(Color::BLACK);
                shape.stroke = None;
                shape.name = path.file_stem().map(|s| s.to_string_lossy().to_string());
                self.run(Command::AddShape { layer, shape });
                self.select(vec![id]);
                self.check_missing_fonts();
            }
            Err(e) => self.status = crate::i18n::trf("status.import_failed", &[("e", &e)]),
        }
    }

    /// Import an EPS file into the active layer.
    pub fn import_eps(&mut self, path: &std::path::Path) {
        let Some(layer) = self.active_layer() else {
            return;
        };
        let parsed = crate::files::read(path)
            .map_err(|e| e.to_string())
            .and_then(|bytes| {
                tracedraw_io::eps_import::parse(
                    &bytes,
                    &mut tracedraw_core::id::IdSource::default(),
                )
            });
        match parsed {
            Ok(imported) => {
                let mut cmds = Vec::new();
                let mut ids = Vec::new();
                for s in imported.shapes {
                    let id = self.engine.new_shape_id();
                    let shape = reid(s, id, &mut self.engine);
                    ids.push(id);
                    cmds.push(Command::AddShape { layer, shape });
                }
                if let Err(e) = self.engine.run_batch("Import", &cmds) {
                    self.status = e.to_string();
                }
                if !imported.warnings.is_empty() {
                    log::warn!("eps import: {}", imported.warnings.join("; "));
                }
                self.select(ids);
            }
            Err(e) => self.status = crate::i18n::trf("status.import_failed", &[("e", &e)]),
        }
    }

    /// Import an EMF or WMF metafile into the active layer.
    pub fn import_metafile(&mut self, path: &std::path::Path) {
        let Some(layer) = self.active_layer() else {
            return;
        };
        let parsed = crate::files::read(path)
            .map_err(|e| e.to_string())
            .and_then(|bytes| {
                tracedraw_io::emf::parse(&bytes, &mut tracedraw_core::id::IdSource::default())
            });
        match parsed {
            Ok(imported) => {
                let mut cmds = Vec::new();
                let mut ids = Vec::new();
                for s in imported.shapes {
                    let id = self.engine.new_shape_id();
                    let shape = reid(s, id, &mut self.engine);
                    ids.push(id);
                    cmds.push(Command::AddShape { layer, shape });
                }
                if let Err(e) = self.engine.run_batch("Import", &cmds) {
                    self.status = e.to_string();
                }
                if !imported.warnings.is_empty() {
                    log::warn!("metafile import: {}", imported.warnings.join("; "));
                }
                self.select(ids);
            }
            Err(e) => self.status = crate::i18n::trf("status.import_failed", &[("e", &e)]),
        }
    }

    /// Import a DXF drawing into the active layer.
    pub fn import_dxf(&mut self, path: &std::path::Path) {
        let Some(layer) = self.active_layer() else {
            return;
        };
        let parsed = crate::files::read(path)
            .map_err(|e| e.to_string())
            .and_then(|bytes| {
                let text = String::from_utf8_lossy(&bytes).into_owned();
                tracedraw_io::dxf::parse(&text, &mut tracedraw_core::id::IdSource::default())
            });
        match parsed {
            Ok(imported) => {
                let mut cmds = Vec::new();
                let mut ids = Vec::new();
                for s in imported.shapes {
                    let id = self.engine.new_shape_id();
                    let shape = reid(s, id, &mut self.engine);
                    ids.push(id);
                    cmds.push(Command::AddShape { layer, shape });
                }
                if let Err(e) = self.engine.run_batch("Import", &cmds) {
                    self.status = e.to_string();
                }
                self.select(ids);
            }
            Err(e) => self.status = crate::i18n::trf("status.import_failed", &[("e", &e)]),
        }
    }

    /// Reset rotation/skew/scale, keeping the object where it is.
    pub fn clear_transformations(&mut self) {
        let shapes = self.selected_shapes();
        let cmds: Vec<Command> = shapes
            .iter()
            .map(|s| {
                let c = s.bounds().center();
                let local_c = s.transform.inverse() * c;
                let t = Affine::translate(c - local_c);
                Command::TransformShapes {
                    shapes: vec![s.id],
                    transform: t * s.transform.inverse(),
                }
            })
            .collect();
        if !cmds.is_empty() {
            let _ = self.engine.run_batch("Clear Transformations", &cmds);
        }
    }

    /// File > Publish to PDF: the dialog for the active drawing.
    pub fn publish_pdf(&mut self) {
        self.dialog = crate::ui::dialogs::Dialog::PublishPdf(Box::new(
            crate::ui::publish_pdf::PublishState::new(self),
        ));
    }

    // ----- view --------------------------------------------------------------

    pub fn zoom_to_page(&mut self) {
        self.fit_pending = true;
    }

    /// Zoom so the page's width fills the window, keeping the vertical
    /// position.
    pub fn zoom_to_page_width(&mut self) {
        let page = self.page_rect();
        let cy = self.view.to_page(self.canvas_rect.center()).y;
        let r = Rect::new(page.x0, cy - 1e-3, page.x1, cy + 1e-3);
        self.view.fit(r, self.canvas_rect);
    }

    /// Zoom so the page's height fills the window, keeping the horizontal
    /// position.
    pub fn zoom_to_page_height(&mut self) {
        let page = self.page_rect();
        let cx = self.view.to_page(self.canvas_rect.center()).x;
        let r = Rect::new(cx - 1e-3, page.y0, cx + 1e-3, page.y1);
        self.view.fit(r, self.canvas_rect);
    }

    pub fn zoom_to_fit(&mut self) {
        let Some(b) = self.doc().content_bounds(self.page).ok().flatten() else {
            self.fit_pending = true;
            return;
        };
        self.view.fit(
            b.inflate(b.width() * 0.05 + 1.0, b.height() * 0.05 + 1.0),
            self.canvas_rect,
        );
    }

    pub fn zoom_to_selection(&mut self) {
        if let Some(b) = self.selection_bounds() {
            self.view.fit(
                b.inflate(b.width() * 0.1 + 1.0, b.height() * 0.1 + 1.0),
                self.canvas_rect,
            );
        }
    }

    /// Interface points per mm at 100%: real size at the calibrated screen
    /// resolution (Options > Display > Ruler calibration, 96 per inch by
    /// default), or in the Pixels view one pixel at the system's scaling
    /// per document pixel. Both go through the interface's zoom, which
    /// sizes the menus and bars, never the drawing.
    fn zoom_100(&self) -> f32 {
        let per_inch = if self.view_mode == ViewMode::Pixels {
            self.document_dpi().max(1.0) as f32
                / crate::ui::ui_zoom(crate::settings::effective_ui_size(&self.settings))
        } else {
            crate::settings::screen_points_per_inch(&self.settings)
        };
        per_inch / 25.4
    }

    pub fn zoom_percent(&self) -> f32 {
        self.view.zoom / self.zoom_100() * 100.0
    }

    pub fn set_zoom_percent(&mut self, pct: f32) {
        let anchor: Pos2 = self.canvas_rect.center();
        let target = pct / 100.0 * self.zoom_100();
        self.view.zoom_at(anchor, target / self.view.zoom);
    }

    pub fn add_page(&mut self) {
        let size = self.page_size();
        self.run(Command::AddPage { name: None, size });
        if let Some(p) = self.doc().pages.last() {
            self.page = p.id;
        }
        self.selection.clear();
        self.fit_pending = true;
    }

    /// The document navigator's insert buttons: a page like the current
    /// one, before or after it, made current.
    pub fn insert_page(&mut self, after: bool) {
        let idx = self.page_index();
        let size = self.page_size();
        self.run(Command::AddPage { name: None, size });
        let Some(new) = self.doc().pages.last().map(|p| p.id) else {
            return;
        };
        let to = if after { idx + 1 } else { idx };
        if to + 1 < self.doc().pages.len() {
            self.run(Command::MovePage { page: new, to });
        }
        self.page = new;
        self.selection.clear();
        self.fit_pending = true;
    }

    pub fn delete_page(&mut self) {
        let page = self.page;
        self.run(Command::DeletePage { page });
        if let Some(p) = self.doc().pages.first() {
            self.page = p.id;
        }
        self.selection.clear();
        self.fit_pending = true;
    }

    pub fn goto_page(&mut self, idx: usize) {
        if let Some(p) = self.doc().pages.get(idx) {
            self.page = p.id;
            self.selection.clear();
            self.fit_pending = true;
        }
    }

    pub fn page_index(&self) -> usize {
        self.doc()
            .pages
            .iter()
            .position(|p| p.id == self.page)
            .unwrap_or(0)
    }
}

/// Give a copied shape (and any children) fresh ids.
/// Re-number a shape tree with fresh engine ids (for other modules).
pub fn reid_pub(s: Shape, id: ShapeId, engine: &mut Engine) -> Shape {
    reid(s, id, engine)
}

fn reid(mut s: Shape, id: ShapeId, engine: &mut Engine) -> Shape {
    s.id = id;
    match &mut s.kind {
        ShapeKind::Group { children } => {
            for c in children.iter_mut() {
                let nid = engine.new_shape_id();
                *c = reid(c.clone(), nid, engine);
            }
        }
        ShapeKind::ClipFrame { frame, contents } => {
            let fid = engine.new_shape_id();
            **frame = reid((**frame).clone(), fid, engine);
            for c in contents.iter_mut() {
                let nid = engine.new_shape_id();
                *c = reid(c.clone(), nid, engine);
            }
        }
        _ => {}
    }
    s
}

/// Re-number a shape tree from an id source (documents built outside the engine).
fn reid_with(mut s: Shape, ids: &mut tracedraw_core::id::IdSource) -> Shape {
    s.id = ids.shape();
    match &mut s.kind {
        ShapeKind::Group { children } => {
            for c in children.iter_mut() {
                *c = reid_with(c.clone(), ids);
            }
        }
        ShapeKind::ClipFrame { frame, contents } => {
            **frame = reid_with((**frame).clone(), ids);
            for c in contents.iter_mut() {
                *c = reid_with(c.clone(), ids);
            }
        }
        _ => {}
    }
    s
}

pub fn fill_preview_color(fill: &Fill) -> Option<egui::Color32> {
    match fill {
        Fill::None => None,
        Fill::Solid(c) => Some(crate::canvas::to_color32(*c)),
        other => other.preview_color().map(crate::canvas::to_color32),
    }
}

pub fn fill_description(fill: &Fill) -> String {
    use crate::i18n::{tr, trf};
    match fill {
        Fill::None => tr("fill.none"),
        Fill::Solid(c) => color_description(*c),
        Fill::Fountain(f) => match f.kind {
            tracedraw_core::FountainKind::Linear => tr("fill.linear_fountain"),
            tracedraw_core::FountainKind::Radial => tr("fill.radial_fountain"),
            tracedraw_core::FountainKind::Conical => tr("fill.conical_fountain"),
            tracedraw_core::FountainKind::Square => tr("fill.square_fountain"),
        },
        Fill::Pattern(tracedraw_core::Pattern::TwoColor { tile, .. }) => {
            trf("fill.two_color_pattern", &[("t", tile.name())])
        }
        Fill::Pattern(tracedraw_core::Pattern::Bitmap { .. }) => tr("fill.bitmap_pattern"),
        Fill::Pattern(tracedraw_core::Pattern::Vector { .. }) => tr("fill.vector_pattern"),
        Fill::Texture(_) => tr("fill.texture_fill"),
        Fill::Mesh(m) => trf(
            "fill.mesh_fill_n",
            &[("r", &m.rows.to_string()), ("c", &m.cols.to_string())],
        ),
    }
}

pub fn color_description(c: Color) -> String {
    match c {
        Color::Rgb { r, g, b } => format!(
            "R:{} G:{} B:{}",
            (r * 255.0).round(),
            (g * 255.0).round(),
            (b * 255.0).round()
        ),
        Color::Cmyk { c, m, y, k } => format!(
            "C:{} M:{} Y:{} K:{}",
            (c * 100.0).round(),
            (m * 100.0).round(),
            (y * 100.0).round(),
            (k * 100.0).round()
        ),
        Color::Gray { v } => format!("Gray {}", (v * 255.0).round()),
        Color::Hsb { h, s, b } => format!(
            "H:{} S:{} B:{}",
            h.round(),
            (s * 100.0).round(),
            (b * 100.0).round()
        ),
        Color::Hsl { h, s, l } => format!(
            "H:{} S:{} L:{}",
            h.round(),
            (s * 100.0).round(),
            (l * 100.0).round()
        ),
        Color::Lab { l, a, b } => format!("L:{} a:{} b:{}", l.round(), a.round(), b.round()),
        Color::Yiq { y, i, q } => format!(
            "Y:{} I:{} Q:{}",
            (y * 255.0).round(),
            (i * 255.0).round(),
            (q * 255.0).round()
        ),
        Color::Registration => "Registration".into(),
    }
}

/// True for a path ending in `.cdr` (any case).
pub(crate) fn is_cdr_path(path: &std::path::Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("cdr"))
}

/// The `.cdr` version saving writes for `version`: versions older than
/// the oldest one written become the oldest, newer ones the newest.
pub(crate) fn writable_cdr_version(version: u16) -> u16 {
    version.clamp(
        tracedraw_cdr::write::OLDEST_VERSION,
        tracedraw_cdr::write::NEWEST_VERSION,
    )
}

/// The status after opening a file of `format` with `n` objects.
fn opened_objects(format: &str, n: usize) -> String {
    crate::i18n::trf(
        "status.opened_objects",
        &[("format", format), ("objects", &n.to_string())],
    )
}

/// The CMYK profile a drawing carries, when it reads as a CMYK profile.
pub(crate) fn carried_cmyk_profile(
    doc: &tracedraw_core::Document,
) -> Option<tracedraw_core::icc::Profile> {
    let p = doc.cmyk_profile.as_ref()?;
    let profile = tracedraw_core::icc::Profile::parse(&p.data).ok()?;
    (profile.color_space() == tracedraw_core::icc::ColorSpace::Cmyk).then_some(profile)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_drawing_shows_through_the_cmyk_profile_it_carries() {
        use tracedraw_core::icc::test_profiles as tp;
        let mut doc = tracedraw_core::Document::default();
        assert!(carried_cmyk_profile(&doc).is_none());
        doc.cmyk_profile = Some(tracedraw_core::DocumentProfile::new(
            "Synthetic CMYK",
            tp::lut16_cmyk_profile(),
        ));
        let p = carried_cmyk_profile(&doc).expect("profile");
        assert_eq!(p.description(), Some("Synthetic CMYK"));
        // Not a CMYK profile, or not a profile: the settings decide.
        let rgb = tp::matrix_trc_profile(tp::para_srgb(), tp::para_srgb(), tp::para_srgb());
        doc.cmyk_profile = Some(tracedraw_core::DocumentProfile::new("RGB", rgb));
        assert!(carried_cmyk_profile(&doc).is_none());
        doc.cmyk_profile = Some(tracedraw_core::DocumentProfile::new("Junk", vec![1, 2, 3]));
        assert!(carried_cmyk_profile(&doc).is_none());
    }

    #[test]
    fn resetting_the_settings_brings_back_the_defaults() {
        let mut app = App::headless();
        app.settings.shortcuts = vec![("menu.file.export".into(), "Ctrl+Shift+X".into())];
        app.settings.ui_size = 1.5;
        app.settings.recent_files = vec!["/kept.tdraw".into()];
        app.show_status_bar = false;
        app.show_page_border = false;
        app.set_workspace(Workspace::Lite);
        app.toolbox_last[0] =
            crate::tools::GROUPS[0].tools[crate::tools::GROUPS[0].tools.len() - 1];
        app.reset_settings();
        assert!(app.settings.shortcuts.is_empty());
        assert_eq!(app.settings.ui_size, 0.0);
        assert_eq!(
            app.settings.recent_files,
            vec![PathBuf::from("/kept.tdraw")]
        );
        assert_eq!(app.workspace, Workspace::Default);
        assert!(app.show_status_bar && app.show_page_border && app.show_dockers);
        assert_eq!(app.toolbox_last[0], crate::tools::GROUPS[0].tools[0]);
    }

    fn rect_shape(app: &mut App, r: Rect, filled: bool) -> ShapeId {
        let id = app
            .new_shape(ShapeKind::Rect {
                rect: r,
                radius: 0.0,
                corners: None,
            })
            .expect("a layer");
        app.run(Command::SetFill {
            shapes: vec![id],
            fill: if filled {
                Fill::Solid(Color::BLACK)
            } else {
                Fill::None
            },
        });
        id
    }

    #[test]
    fn clip_frame_content_outside_the_frame_is_centred_in_one_step() {
        let mut app = App::headless();
        let frame = rect_shape(&mut app, Rect::new(0.0, 0.0, 100.0, 100.0), true);
        let far = rect_shape(&mut app, Rect::new(200.0, 200.0, 220.0, 220.0), true);
        let depth = app.engine.history_labels().0.len();
        app.place_inside(vec![far], frame);
        assert_eq!(app.engine.history_labels().0.len(), depth + 1);
        let ShapeKind::ClipFrame { contents, .. } = &app.doc().find_shape(frame).unwrap().kind
        else {
            panic!("not a ClipFrame");
        };
        let c = contents[0].bounds().center();
        assert!((c.x - 50.0).abs() < 1e-9 && (c.y - 50.0).abs() < 1e-9);
        // Content overlapping the frame stays where it is by default.
        let mut app = App::headless();
        let frame = rect_shape(&mut app, Rect::new(0.0, 0.0, 100.0, 100.0), true);
        let near = rect_shape(&mut app, Rect::new(90.0, 90.0, 120.0, 120.0), true);
        app.place_inside(vec![near], frame);
        let ShapeKind::ClipFrame { contents, .. } = &app.doc().find_shape(frame).unwrap().kind
        else {
            panic!("not a ClipFrame");
        };
        assert!((contents[0].bounds().x0 - 90.0).abs() < 1e-9);
    }

    #[test]
    fn unfilled_objects_hit_only_near_the_outline_unless_treated_as_filled() {
        let mut app = App::headless();
        app.view.zoom = 1.0;
        let id = rect_shape(&mut app, Rect::new(10.0, 10.0, 60.0, 60.0), false);
        let inside = Point::new(35.0, 35.0);
        assert_eq!(app.hit_test(inside), None);
        assert_eq!(app.hit_test(Point::new(10.5, 35.0)), Some(id));
        app.settings.treat_all_filled = true;
        assert_eq!(app.hit_test(inside), Some(id));
    }

    #[test]
    fn filled_ellipses_are_hit_inside_their_curve_not_their_box() {
        let mut app = App::headless();
        app.view.zoom = 1.0;
        let id = app
            .new_shape(ShapeKind::Ellipse {
                rect: Rect::new(0.0, 0.0, 100.0, 100.0),
                arc: None,
            })
            .expect("a layer");
        app.run(Command::SetFill {
            shapes: vec![id],
            fill: Fill::Solid(Color::BLACK),
        });
        assert_eq!(app.hit_test(Point::new(50.0, 50.0)), Some(id));
        // The box corner is outside the circle.
        assert_eq!(app.hit_test(Point::new(3.0, 3.0)), None);
    }

    #[test]
    fn undo_levels_limit_the_history() {
        let mut app = App::headless();
        app.settings.undo_levels = 2;
        app.apply_runtime_settings();
        for i in 0..5 {
            rect_shape(&mut app, Rect::new(0.0, 0.0, 10.0 + i as f64, 10.0), true);
        }
        assert_eq!(app.engine.history_labels().0.len(), 2);
    }

    #[test]
    fn saving_over_a_file_keeps_a_backup_and_auto_backup_saves_dirty_drawings() {
        let dir = std::env::temp_dir().join(format!("tracedraw-backup-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut app = App::headless();
        let path = dir.join("art.tdraw");
        app.save_to(path.clone());
        assert!(path.is_file());
        assert!(
            !dir.join("backup_of_art.tdraw").exists(),
            "nothing to back up yet"
        );
        rect_shape(&mut app, Rect::new(0.0, 0.0, 10.0, 10.0), true);
        app.save_to(path.clone());
        assert!(dir.join("backup_of_art.tdraw").is_file());
        // Auto-backup: due after the interval, only for unsaved changes.
        app.settings.backup.auto_dir = Some(dir.join("auto"));
        app.settings.backup.minutes = 1;
        let t0 = web_time::Instant::now();
        assert!(!app.auto_backup_tick(t0));
        rect_shape(&mut app, Rect::new(0.0, 0.0, 20.0, 10.0), true);
        assert!(!app.auto_backup_tick(t0 + std::time::Duration::from_secs(30)));
        assert!(app.auto_backup_tick(t0 + std::time::Duration::from_secs(61)));
        let saved: Vec<_> = std::fs::read_dir(dir.join("auto")).unwrap().collect();
        assert_eq!(saved.len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn postscript_ai_files_open_as_eps() {
        let dir = std::env::temp_dir().join(format!("tracedraw-ai-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let old = dir.join("old.ai");
        std::fs::write(
            &old,
            b"%!PS-Adobe-3.0 EPSF-3.0\n%%BoundingBox: 0 0 100 100\nnewpath 10 10 moveto 90 10 lineto 90 90 lineto closepath fill\n%%EOF\n",
        )
        .unwrap();
        let new = dir.join("new.ai");
        std::fs::write(&new, b"%PDF-1.5\n").unwrap();
        assert_eq!(App::file_kind(&old), "eps");
        assert_eq!(App::file_kind(&new), "ai");
        assert_eq!(App::file_kind(&dir.join("missing.AI")), "ai");
        assert_eq!(App::file_kind(&dir.join("x.PDF")), "pdf");
        // The old file opens through the PostScript interpreter.
        let mut app = App::headless();
        app.open_path(old);
        let n: usize = app.doc().pages[0]
            .layers
            .iter()
            .map(|l| l.shapes.len())
            .sum();
        assert!(n >= 1, "{}", app.status);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn print_files_open_with_every_page() {
        let dir = std::env::temp_dir().join(format!("tracedraw-prn-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        // Two pages written by the exporter, behind a printer job header.
        let mut app = App::headless();
        rect_shape(&mut app, Rect::new(10.0, 10.0, 50.0, 50.0), true);
        app.add_page();
        rect_shape(&mut app, Rect::new(20.0, 20.0, 40.0, 40.0), true);
        let mut prn =
            b"\x1b%-12345X@PJL JOB NAME=\"art\"\n@PJL ENTER LANGUAGE=POSTSCRIPT\n".to_vec();
        prn.extend_from_slice(tracedraw_io::eps::document_to_ps(app.doc()).as_bytes());
        let path = dir.join("art.prn");
        std::fs::write(&path, prn).unwrap();
        let mut back = App::headless();
        back.open_path(path);
        assert_eq!(back.doc().pages.len(), 2, "{}", back.status);
        for p in &back.doc().pages {
            let n: usize = p.layers.iter().map(|l| l.shapes.len()).sum();
            assert!(n >= 1, "{:?}", p.name);
        }
        // Not saved over: a print file is not a drawing.
        assert!(back.file.is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
