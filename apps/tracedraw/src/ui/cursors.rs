//! The pointer over the drawing window shows the active tool and what a
//! press would do. Tools with a system cursor on every platform use it
//! (the arrow, the crosshair, the text I-beam, the resize and move
//! arrows); the others draw their own: a magnifier with a plus or a minus
//! for Zoom, an open or closed hand for Pan and middle-button panning, a
//! pipette for the eyedroppers and palette sampling, a paint bucket for
//! Area Fill, turning and slanting arrows for the rotate and skew
//! handles, and the Shape tool's solid arrow with a mark for a node, a
//! segment or a control point under it. Drawn cursors hide the system
//! pointer and are painted above everything at the pointer position.
//!
//! The Pick tools: over a corner handle the diagonal resize arrows, over
//! a side handle the horizontal or vertical ones, over the selection (or
//! the centre of rotation) the move arrows; in rotate mode the corners
//! turn and the sides slant. While dragging, the pointer keeps the shape
//! of what is being done.

use crate::app::{App, Drag, Handle};
use crate::tools::Tool;
use egui::{Color32, CornerRadius, Painter, Pos2, Rect, Shape, Stroke, Vec2};

/// What the pointer looks like over the drawing window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolCursor {
    /// A system cursor.
    System(egui::CursorIcon),
    ZoomIn,
    ZoomOut,
    Hand {
        closed: bool,
    },
    Pipette,
    Bucket,
    /// Turning the selection (rotate mode, corner handles).
    Rotate,
    /// Slanting the selection (rotate mode, side handles): sideways for
    /// the top and bottom handles, up and down for the others.
    Skew {
        horizontal: bool,
    },
    /// The Shape tool: a solid arrow with what is under it.
    ShapeArrow(ShapeMark),
}

/// What the Shape tool's arrow points at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeMark {
    Nothing,
    /// A node (of a curve, or of a rectangle, ellipse or polygon).
    Node,
    /// A segment of a selected curve.
    Segment,
    /// A control point of a selected node.
    Control,
}

/// The system resize arrows for a scale handle.
pub fn resize_icon(h: Handle) -> egui::CursorIcon {
    use egui::CursorIcon as C;
    match h {
        Handle::N | Handle::S => C::ResizeVertical,
        Handle::E | Handle::W => C::ResizeHorizontal,
        Handle::NE | Handle::SW => C::ResizeNeSw,
        Handle::NW | Handle::SE => C::ResizeNwSe,
    }
}

/// The Pick tools' pointer for what is being dragged or what is under
/// `pos` (screen); `None` for the tool's plain pointer.
fn pick_cursor(app: &App, pos: Option<Pos2>) -> Option<ToolCursor> {
    let moving = ToolCursor::System(egui::CursorIcon::Move);
    match &app.drag {
        Drag::Move { .. } | Drag::Pivot => return Some(moving),
        Drag::Scale { handle, .. } => return Some(ToolCursor::System(resize_icon(*handle))),
        Drag::Rotate { .. } => return Some(ToolCursor::Rotate),
        Drag::Skew { handle, .. } => {
            return Some(ToolCursor::Skew {
                horizontal: matches!(handle, Handle::N | Handle::S),
            })
        }
        Drag::None => {}
        _ => return None,
    }
    let pos = pos?;
    if app.pivot_at(pos) {
        return Some(moving);
    }
    if let Some(h) = app.handle_at(pos) {
        return Some(if !app.rotate_mode {
            ToolCursor::System(resize_icon(h))
        } else if h.is_corner() {
            ToolCursor::Rotate
        } else {
            ToolCursor::Skew {
                horizontal: matches!(h, Handle::N | Handle::S),
            }
        });
    }
    let id = app.hit_test(app.view.to_page(pos))?;
    app.selection.contains(&id).then_some(moving)
}

/// What the Shape tool's arrow points at: what is being dragged, else
/// what is under `pos` (screen).
fn shape_mark(app: &App, pos: Option<Pos2>) -> ShapeMark {
    match &app.drag {
        Drag::Node { .. } | Drag::NodeMove { .. } | Drag::KindNode { .. } => {
            return ShapeMark::Node
        }
        Drag::Handle { .. } => return ShapeMark::Control,
        Drag::None => {}
        _ => return ShapeMark::Nothing,
    }
    let Some(pos) = pos else {
        return ShapeMark::Nothing;
    };
    let p = app.view.to_page(pos);
    if app.ctrl_handle_at(p).is_some() {
        ShapeMark::Control
    } else if app.node_at(p).is_some() || app.kind_node_at(p).is_some() {
        ShapeMark::Node
    } else if app.segment_at(p).is_some() {
        ShapeMark::Segment
    } else {
        ShapeMark::Nothing
    }
}

/// The cursor for the current tool and input state, the pointer at `pos`
/// (screen).
pub fn for_tool(
    app: &App,
    shift: bool,
    primary_down: bool,
    middle_down: bool,
    pos: Option<Pos2>,
) -> ToolCursor {
    if middle_down {
        return ToolCursor::Hand { closed: true };
    }
    if app.pending_palette_sample {
        return ToolCursor::Pipette;
    }
    if matches!(app.tool, Tool::Pick | Tool::FreeformPick) {
        if let Some(c) = pick_cursor(app, pos) {
            return c;
        }
    }
    match app.tool {
        Tool::Zoom if shift => ToolCursor::ZoomOut,
        Tool::Zoom => ToolCursor::ZoomIn,
        Tool::Pan => ToolCursor::Hand {
            closed: primary_down,
        },
        Tool::ColorEyedropper | Tool::AttributesEyedropper => ToolCursor::Pipette,
        Tool::AreaFill => ToolCursor::Bucket,
        Tool::Text => ToolCursor::System(egui::CursorIcon::Text),
        Tool::Pick | Tool::FreeformPick if app.settings.crosshair_cursor => {
            ToolCursor::System(egui::CursorIcon::Crosshair)
        }
        Tool::Pick | Tool::FreeformPick => ToolCursor::System(egui::CursorIcon::Default),
        Tool::Shape => ToolCursor::ShapeArrow(shape_mark(app, pos)),
        _ => ToolCursor::System(egui::CursorIcon::Crosshair),
    }
}

/// Show `cursor` at `pos` (the pointer): set the system cursor, or hide it
/// and paint the drawn one on top of everything.
pub fn show(ctx: &egui::Context, cursor: ToolCursor, pos: Option<Pos2>) {
    let ToolCursor::System(icon) = cursor else {
        ctx.set_cursor_icon(egui::CursorIcon::None);
        if let Some(p) = pos {
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Tooltip,
                egui::Id::new("tool_cursor"),
            ));
            paint(&painter, cursor, p);
        }
        return;
    };
    ctx.set_cursor_icon(icon);
}

const INK: Color32 = Color32::BLACK;
const HALO: Color32 = Color32::WHITE;

/// Paint a drawn cursor with its hot spot at `p`.
pub fn paint(painter: &Painter, cursor: ToolCursor, p: Pos2) {
    match cursor {
        ToolCursor::System(_) => {}
        ToolCursor::ZoomIn | ToolCursor::ZoomOut => {
            magnifier(painter, p, cursor == ToolCursor::ZoomIn)
        }
        ToolCursor::Hand { closed } => hand(painter, p, closed),
        ToolCursor::Pipette => pipette(painter, p),
        ToolCursor::Bucket => bucket(painter, p),
        ToolCursor::Rotate => rotate_arrows(painter, p),
        ToolCursor::Skew { horizontal } => skew_arrows(painter, p, horizontal),
        ToolCursor::ShapeArrow(mark) => shape_arrow(painter, p, mark),
    }
}

/// Draw `segments` twice: a white halo, then the ink.
fn haloed(painter: &Painter, lines: &[Vec<Pos2>], width: f32) {
    for l in lines {
        painter.add(Shape::line(l.clone(), Stroke::new(width + 2.5, HALO)));
    }
    for l in lines {
        painter.add(Shape::line(l.clone(), Stroke::new(width, INK)));
    }
}

/// An arrow that turns around the hot spot: most of a circle with a head
/// at its end.
fn rotate_arrows(painter: &Painter, c: Pos2) {
    let r = 7.0;
    let arc: Vec<Pos2> = (0..=24)
        .map(|i| {
            let a = (-200.0 + i as f32 * 11.0f32).to_radians();
            c + Vec2::new(a.cos(), a.sin()) * r
        })
        .collect();
    let end = *arc.last().unwrap_or(&c);
    let a = (-200.0f32 + 24.0 * 11.0).to_radians();
    // The tangent at the end, along the turn.
    let t = Vec2::new(-a.sin(), a.cos());
    let n = Vec2::new(a.cos(), a.sin());
    let head = vec![
        end + n * 3.5 - t * 3.5,
        end + t * 1.5,
        end - n * 3.5 - t * 3.5,
    ];
    haloed(painter, &[arc, head], 1.6);
    painter.circle_filled(c, 1.3, INK);
}

/// Two half arrows going opposite ways along the slant: sideways for a
/// horizontal skew, up and down for a vertical one.
fn skew_arrows(painter: &Painter, c: Pos2, horizontal: bool) {
    // Built sideways, then turned for a vertical skew.
    let at = |x: f32, y: f32| {
        if horizontal {
            c + Vec2::new(x, y)
        } else {
            c + Vec2::new(y, -x)
        }
    };
    let top = vec![at(-8.0, -2.5), at(8.0, -2.5), at(4.5, -6.0)];
    let bottom = vec![at(8.0, 2.5), at(-8.0, 2.5), at(-4.5, 6.0)];
    haloed(painter, &[top, bottom], 1.6);
}

/// The Shape tool's pointer: a solid arrow with its tip on the hot spot
/// and, at its lower right, a hollow square over a node, a wave over a
/// segment or a dot over a control point.
fn shape_arrow(painter: &Painter, tip: Pos2, mark: ShapeMark) {
    let at = |x: f32, y: f32| tip + Vec2::new(x, y);
    let head = vec![at(0.0, 0.0), at(0.0, 14.0), at(10.0, 10.0)];
    let tail = vec![at(3.2, 10.6), at(5.6, 9.7), at(8.6, 15.7), at(6.2, 16.6)];
    for poly in [&head, &tail] {
        painter.add(Shape::convex_polygon(
            poly.clone(),
            HALO,
            Stroke::new(2.4, HALO),
        ));
    }
    for poly in [head, tail] {
        painter.add(Shape::convex_polygon(poly, INK, Stroke::new(0.6, INK)));
    }
    let m = at(14.0, 15.0);
    match mark {
        ShapeMark::Nothing => {}
        ShapeMark::Node => {
            let r = Rect::from_center_size(m, Vec2::splat(6.0));
            painter.rect(
                r,
                CornerRadius::ZERO,
                HALO,
                Stroke::new(1.2, INK),
                egui::StrokeKind::Inside,
            );
        }
        ShapeMark::Segment => {
            let wave: Vec<Pos2> = (0..=12)
                .map(|i| {
                    let x = -4.5 + i as f32 * 0.75;
                    m + Vec2::new(x, -2.2 * (x * 0.7).sin())
                })
                .collect();
            haloed(painter, &[wave], 1.4);
        }
        ShapeMark::Control => {
            painter.circle(m, 2.6, INK, Stroke::new(1.2, HALO));
        }
    }
}

/// A lens centred on the hot spot with a handle to the lower right and a
/// plus or a minus inside.
fn magnifier(painter: &Painter, c: Pos2, plus: bool) {
    let r = 6.5;
    let a = c + Vec2::splat(r * 0.72);
    let b = c + Vec2::splat(r * 0.72 + 7.0);
    painter.line_segment([a, b], Stroke::new(5.0, HALO));
    painter.circle(c, r, Color32::from_white_alpha(230), Stroke::new(3.0, HALO));
    painter.line_segment([a, b], Stroke::new(3.0, INK));
    painter.circle_stroke(c, r, Stroke::new(1.5, INK));
    let s = Stroke::new(1.4, INK);
    let d = 3.2;
    painter.line_segment([c - Vec2::new(d, 0.0), c + Vec2::new(d, 0.0)], s);
    if plus {
        painter.line_segment([c - Vec2::new(0.0, d), c + Vec2::new(0.0, d)], s);
    }
}

/// An open hand (fingers up) or a closed one (fingers folded): outlined
/// fingers and thumb drawn as capsules, then the palm over their roots.
/// The hot spot is the middle of the palm.
fn hand(painter: &Painter, c: Pos2, closed: bool) {
    let capsule = |a: Pos2, b: Pos2| {
        painter.line_segment([a, b], Stroke::new(4.2, INK));
        painter.line_segment([a, b], Stroke::new(2.2, HALO));
    };
    let tops: [f32; 4] = if closed {
        [-2.5, -3.5, -3.5, -2.0]
    } else {
        [-8.0, -9.5, -9.0, -6.5]
    };
    for (x, top) in [-4.5f32, -1.5, 1.5, 4.5].iter().zip(tops) {
        capsule(c + Vec2::new(*x, top), c + Vec2::new(*x, 2.0));
    }
    if closed {
        capsule(c + Vec2::new(-7.5, 2.0), c + Vec2::new(-4.5, 4.0));
    } else {
        capsule(c + Vec2::new(-9.5, -0.5), c + Vec2::new(-5.5, 4.0));
    }
    let palm = Rect::from_min_max(c + Vec2::new(-6.2, -1.0), c + Vec2::new(6.2, 8.0));
    painter.rect(
        palm,
        CornerRadius::same(3),
        HALO,
        Stroke::new(1.0, INK),
        egui::StrokeKind::Inside,
    );
    // Hide the palm's top edge between the fingers so they join the palm.
    for x in [-4.5f32, -1.5, 1.5, 4.5] {
        painter.line_segment(
            [c + Vec2::new(x, -0.5), c + Vec2::new(x, 1.5)],
            Stroke::new(2.2, HALO),
        );
    }
}

/// A pipette: the tip at the hot spot, the glass tube rising to the upper
/// right and the rubber bulb at its end.
fn pipette(painter: &Painter, tip: Pos2) {
    let dir = Vec2::new(1.0, -1.0).normalized();
    let tube_start = tip + dir * 1.5;
    let tube_end = tip + dir * 11.0;
    let bulb = tip + dir * 14.5;
    painter.line_segment([tube_start, tube_end], Stroke::new(5.0, HALO));
    painter.circle_filled(bulb, 4.4, HALO);
    painter.line_segment([tube_start, tube_end], Stroke::new(2.6, INK));
    painter.circle_filled(bulb, 3.0, INK);
    painter.circle_filled(tip, 1.4, INK);
}

/// A tipped paint bucket pouring onto the hot spot.
fn bucket(painter: &Painter, spot: Pos2) {
    // The bucket body, a slanted box above and to the right of the spot.
    let o = spot + Vec2::new(4.0, -12.0);
    let pts = [
        o,
        o + Vec2::new(9.0, 4.0),
        o + Vec2::new(5.5, 12.0),
        o + Vec2::new(-3.5, 8.0),
    ];
    painter.add(Shape::convex_polygon(
        pts.to_vec(),
        HALO,
        Stroke::new(3.0, HALO),
    ));
    painter.add(Shape::convex_polygon(
        pts.to_vec(),
        HALO,
        Stroke::new(1.3, INK),
    ));
    // The handle.
    painter.line_segment(
        [o + Vec2::new(1.0, -1.0), o + Vec2::new(7.0, 2.0)],
        Stroke::new(1.2, INK),
    );
    // Paint flowing from the lip to the spot.
    painter.line_segment([o + Vec2::new(-3.0, 7.0), spot], Stroke::new(3.0, HALO));
    painter.line_segment([o + Vec2::new(-3.0, 7.0), spot], Stroke::new(1.6, INK));
    painter.circle_filled(spot, 2.0, INK);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tools_get_their_cursors() {
        let mut app = App::headless();
        app.tool = Tool::Zoom;
        assert_eq!(
            for_tool(&app, false, false, false, None),
            ToolCursor::ZoomIn
        );
        assert_eq!(
            for_tool(&app, true, false, false, None),
            ToolCursor::ZoomOut
        );
        app.tool = Tool::Pan;
        assert_eq!(
            for_tool(&app, false, false, false, None),
            ToolCursor::Hand { closed: false }
        );
        assert_eq!(
            for_tool(&app, false, true, false, None),
            ToolCursor::Hand { closed: true }
        );
        app.tool = Tool::ColorEyedropper;
        assert_eq!(
            for_tool(&app, false, false, false, None),
            ToolCursor::Pipette
        );
        app.tool = Tool::AreaFill;
        assert_eq!(
            for_tool(&app, false, false, false, None),
            ToolCursor::Bucket
        );
        app.tool = Tool::Text;
        assert_eq!(
            for_tool(&app, false, false, false, None),
            ToolCursor::System(egui::CursorIcon::Text)
        );
        app.tool = Tool::Rectangle;
        assert_eq!(
            for_tool(&app, false, false, false, None),
            ToolCursor::System(egui::CursorIcon::Crosshair)
        );
        app.tool = Tool::Pick;
        assert_eq!(
            for_tool(&app, false, false, false, None),
            ToolCursor::System(egui::CursorIcon::Default)
        );
        // Middle-button panning shows the closed hand with any tool.
        assert_eq!(
            for_tool(&app, false, false, true, None),
            ToolCursor::Hand { closed: true }
        );
    }
}
