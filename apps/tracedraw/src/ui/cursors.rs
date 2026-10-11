//! The pointer over the drawing window shows the active tool. Tools with a
//! system cursor on every platform use it (the arrow, the crosshair, the
//! text I-beam); the others draw their own: a magnifier with a plus or a
//! minus for Zoom, an open or closed hand for Pan and middle-button
//! panning, a pipette for the eyedroppers and palette sampling, and a
//! paint bucket for Area Fill. Drawn cursors hide the system pointer and
//! are painted above everything at the pointer position.

use crate::app::App;
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
}

/// The cursor for the current tool and input state.
pub fn for_tool(app: &App, shift: bool, primary_down: bool, middle_down: bool) -> ToolCursor {
    if middle_down {
        return ToolCursor::Hand { closed: true };
    }
    if app.pending_palette_sample {
        return ToolCursor::Pipette;
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
        Tool::Pick | Tool::FreeformPick | Tool::Shape => {
            ToolCursor::System(egui::CursorIcon::Default)
        }
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
        assert_eq!(for_tool(&app, false, false, false), ToolCursor::ZoomIn);
        assert_eq!(for_tool(&app, true, false, false), ToolCursor::ZoomOut);
        app.tool = Tool::Pan;
        assert_eq!(
            for_tool(&app, false, false, false),
            ToolCursor::Hand { closed: false }
        );
        assert_eq!(
            for_tool(&app, false, true, false),
            ToolCursor::Hand { closed: true }
        );
        app.tool = Tool::ColorEyedropper;
        assert_eq!(for_tool(&app, false, false, false), ToolCursor::Pipette);
        app.tool = Tool::AreaFill;
        assert_eq!(for_tool(&app, false, false, false), ToolCursor::Bucket);
        app.tool = Tool::Text;
        assert_eq!(
            for_tool(&app, false, false, false),
            ToolCursor::System(egui::CursorIcon::Text)
        );
        app.tool = Tool::Rectangle;
        assert_eq!(
            for_tool(&app, false, false, false),
            ToolCursor::System(egui::CursorIcon::Crosshair)
        );
        app.tool = Tool::Pick;
        assert_eq!(
            for_tool(&app, false, false, false),
            ToolCursor::System(egui::CursorIcon::Default)
        );
        // Middle-button panning shows the closed hand with any tool.
        assert_eq!(
            for_tool(&app, false, false, true),
            ToolCursor::Hand { closed: true }
        );
    }
}
