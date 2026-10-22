//! Interactive Fill tool: the fountain's nodes on the canvas and the
//! property bar that edits the selected object's fill in place.
//!
//! Every fountain but a shaped one shows three nodes, placed freely as
//! the reference program places them (#330): the start (a square in the
//! first colour), the end (a square in the last colour, an arrow on the
//! line) and the skew handle (a round node on a thin line from the
//! start). Dragging the start moves it with the skew handle (the centre
//! of an elliptical, conical or square fill moves the whole fill); the end
//! turns and stretches the axis, the skew handle turning with it; the skew
//! handle tilts the bands (and gives an ellipse or a square its other
//! half axis). The nodes are kept relative to the object's box (see
//! `Fountain::handles`).

use crate::app::App;
use tracedraw_core::{
    geometry::{Point, Rect, Vec2},
    style::{Fountain, FountainHandles, FountainKind},
    Command, Fill, ShapeId,
};

/// Which fountain node a drag holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FountainHandle {
    Start,
    End,
    Skew,
}

/// The nodes of a fountain on the object's page bounds: start, end and
/// skew handle; none for a shaped fountain (it follows the outline).
pub fn fountain_handles(f: &Fountain, b: Rect) -> Vec<(FountainHandle, Point)> {
    if f.kind == FountainKind::Shaped {
        return Vec::new();
    }
    let h = f.handles.unwrap_or_else(|| f.default_handles(b));
    vec![
        (FountainHandle::Start, Fountain::rel_to_page(b, h.start)),
        (FountainHandle::End, Fountain::rel_to_page(b, h.end)),
        (FountainHandle::Skew, Fountain::rel_to_page(b, h.skew)),
    ]
}

/// The nodes `h` (page points `s`, `e`, `k`) after `handle` is dragged to
/// `p`: the start carries the skew handle (and, with a centre, the end
/// too); the end turns the skew handle by as much as the axis turned.
pub fn moved_nodes(
    kind: FountainKind,
    (s, e, k): (Point, Point, Point),
    handle: FountainHandle,
    p: Point,
) -> (Point, Point, Point) {
    match handle {
        FountainHandle::Start => {
            let d = p - s;
            if kind == FountainKind::Linear {
                (p, e, k + d)
            } else {
                (p, e + d, k + d)
            }
        }
        FountainHandle::End => {
            let (old, new) = (e - s, p - s);
            let turn = if old.hypot() > 1e-9 && new.hypot() > 1e-9 {
                new.y.atan2(new.x) - old.y.atan2(old.x)
            } else {
                0.0
            };
            let kv = k - s;
            let (c, sn) = (turn.cos(), turn.sin());
            let kv = Vec2::new(kv.x * c - kv.y * sn, kv.x * sn + kv.y * c);
            (s, p, s + kv)
        }
        FountainHandle::Skew => (s, e, p),
    }
}

impl App {
    /// The selected object's fountain, with its id and bounds (those the
    /// renderer places the fill on).
    pub fn selected_fountain(&self) -> Option<(ShapeId, Fountain, Rect)> {
        use tracedraw_core::geometry::Shape as _;
        let s = self.selected_shapes().into_iter().next()?;
        match &s.fill {
            Fill::Fountain(f) => Some((s.id, f.clone(), s.page_path().bounding_box())),
            _ => None,
        }
    }

    /// The node under a page point, within 6 screen pixels (the end and the
    /// start before the skew handle where they meet).
    pub fn fountain_handle_at(&self, p: Point) -> Option<FountainHandle> {
        let (_, f, b) = self.selected_fountain()?;
        let tol = 6.0 / self.view.zoom as f64;
        fountain_handles(&f, b)
            .into_iter()
            .filter(|(_, hp)| (*hp - p).hypot() <= tol)
            .min_by(|a, b| (a.1 - p).hypot().total_cmp(&(b.1 - p).hypot()))
            .map(|(h, _)| h)
    }

    /// Move a fountain node to `p` (see [`moved_nodes`]). The first move
    /// of a drag is an undo step "Fill"; the next ones join it (`begun`).
    pub fn drag_fountain_handle(&mut self, handle: FountainHandle, p: Point, begun: bool) {
        let Some((id, mut f, b)) = self.selected_fountain() else {
            return;
        };
        let pts = fountain_handles(&f, b);
        let [(_, s), (_, e), (_, k)] = pts.as_slice() else {
            return;
        };
        let (s, e, k) = moved_nodes(f.kind, (*s, *e, *k), handle, p);
        let rel = |q: Point| Fountain::page_to_rel(b, q);
        f.set_handles(
            b,
            FountainHandles {
                start: rel(s),
                end: rel(e),
                skew: rel(k),
            },
        );
        let cmd = Command::SetFill {
            shapes: vec![id],
            fill: Fill::Fountain(f),
        };
        if begun && self.engine.undo_label() == Some("Fill") {
            let _ = self.engine.amend(&cmd);
        } else {
            self.run(cmd);
        }
    }

    /// A drag across the object with the Interactive Fill tool: the fill
    /// runs from where the drag began to where it ended (its kind and
    /// colours kept when it is a fountain already; else a linear one from
    /// the default colour to white), the skew handle square to the axis.
    pub fn fill_from_drag(&mut self, id: ShapeId, from: Point, to: Point) {
        use tracedraw_core::geometry::Shape as _;
        let Some(s) = self.doc().shape_on_page(id) else {
            return;
        };
        let b = s.page_path().bounding_box();
        let mut f = match &s.fill {
            Fill::Fountain(f) if f.kind != FountainKind::Shaped => f.clone(),
            _ => {
                let first = match &self.default_fill() {
                    Fill::Solid(c) => *c,
                    _ => tracedraw_core::Color::cmyk_pct(0.0, 0.0, 0.0, 100.0),
                };
                Fountain::two(
                    FountainKind::Linear,
                    first,
                    tracedraw_core::Color::WHITE,
                    0.0,
                )
            }
        };
        let d = to - from;
        let skew = from + Vec2::new(-d.y, d.x);
        let rel = |q: Point| Fountain::page_to_rel(b, q);
        f.set_handles(
            b,
            FountainHandles {
                start: rel(from),
                end: rel(to),
                skew: rel(skew),
            },
        );
        self.run(Command::SetFill {
            shapes: vec![id],
            fill: Fill::Fountain(f),
        });
    }

    /// Apply a fill to the selection, or make it the default when nothing
    /// is selected.
    pub fn apply_fill_or_default(&mut self, fill: Fill) {
        if self.selection.is_empty() {
            self.request_default_change(crate::defaults::DefaultChange::Fill(fill));
        } else {
            self.apply_fill(fill);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::{document::ShapeKind, Color};

    fn square(app: &mut App) -> ShapeId {
        let id = app
            .new_shape(ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, 40.0, 20.0),
                radius: 0.0,
                corners: None,
            })
            .expect("shape");
        app.select(vec![id]);
        id
    }

    fn near(a: Point, b: Point) -> bool {
        (a - b).hypot() < 1e-6
    }

    /// A linear fill shows its start, end and skew handle; the end turns
    /// the axis with the skew handle, the start carries the skew handle;
    /// a drag is one undo step (#330).
    #[test]
    fn linear_nodes_move_as_the_reference_moves_them() {
        let mut app = App::headless();
        square(&mut app);
        app.apply_fill(Fill::linear(Color::BLACK, Color::WHITE, 0.0));
        let (_, f, b) = app.selected_fountain().expect("fountain");
        let hs = fountain_handles(&f, b);
        assert_eq!(hs.len(), 3);
        assert!(near(hs[0].1, Point::new(0.0, 10.0)), "{hs:?}");
        assert!(near(hs[1].1, Point::new(40.0, 10.0)));
        // Drag the end node straight up (two steps of one drag).
        let depth = app.engine.history_labels().0.len();
        app.drag_fountain_handle(FountainHandle::End, Point::new(0.0, 20.0), false);
        app.drag_fountain_handle(FountainHandle::End, Point::new(0.0, 30.0), true);
        assert_eq!(app.engine.history_labels().0.len(), depth + 1);
        let (_, f, b) = app.selected_fountain().expect("fountain");
        assert!((f.angle - 90.0).abs() < 1e-6, "{}", f.angle);
        let hs = fountain_handles(&f, b);
        assert!(near(hs[1].1, Point::new(0.0, 30.0)), "{hs:?}");
        // The skew handle turned with the axis: square to it still.
        let axis = hs[1].1 - hs[0].1;
        let skew = hs[2].1 - hs[0].1;
        assert!(axis.dot(skew).abs() < 1e-6, "{axis:?} {skew:?}");
        // The start moves on its own, carrying the skew handle.
        app.drag_fountain_handle(FountainHandle::Start, Point::new(10.0, 10.0), false);
        let (_, f, b) = app.selected_fountain().expect("fountain");
        let hs2 = fountain_handles(&f, b);
        assert!(near(hs2[0].1, Point::new(10.0, 10.0)));
        assert!(near(hs2[1].1, hs[1].1));
        assert!(((hs2[2].1 - hs2[0].1) - skew).hypot() < 1e-6, "{hs2:?}");
        assert_eq!(
            app.fountain_handle_at(Point::new(0.0, 30.0)),
            Some(FountainHandle::End)
        );
        assert_eq!(app.fountain_handle_at(Point::new(35.0, 2.0)), None);
    }

    #[test]
    fn shaped_fountains_have_no_handles() {
        let f = Fountain::two(FountainKind::Shaped, Color::BLACK, Color::WHITE, 0.0);
        assert!(fountain_handles(&f, Rect::new(0.0, 0.0, 40.0, 20.0)).is_empty());
    }

    /// The centre of an elliptical fill moves the whole fill.
    #[test]
    fn the_centre_of_a_radial_fill_moves_it_whole() {
        let mut app = App::headless();
        square(&mut app);
        app.apply_fill(Fill::radial(Color::BLACK, Color::WHITE));
        let (_, f, b) = app.selected_fountain().expect("fountain");
        let before = fountain_handles(&f, b);
        app.drag_fountain_handle(FountainHandle::Start, Point::new(30.0, 15.0), false);
        let (_, f, b) = app.selected_fountain().expect("fountain");
        let after = fountain_handles(&f, b);
        let d = Point::new(30.0, 15.0) - before[0].1;
        for (a, z) in before.iter().zip(&after) {
            assert!(near(z.1, a.1 + d), "{a:?} {z:?}");
        }
        // The offset follows for the formats that know only that.
        assert!((f.offset.x - 0.5).abs() < 1e-9 && (f.offset.y - 0.5).abs() < 1e-9);
    }

    /// A drag across the object puts the start and the end where the drag
    /// began and ended, keeping a fountain's kind and colours.
    #[test]
    fn a_drag_puts_the_nodes_where_it_began_and_ended() {
        let mut app = App::headless();
        let id = square(&mut app);
        app.apply_fill(Fill::radial(Color::rgb8(200, 0, 0), Color::WHITE));
        app.fill_from_drag(id, Point::new(5.0, 5.0), Point::new(15.0, 5.0));
        let (_, f, b) = app.selected_fountain().expect("fountain");
        assert_eq!(f.kind, FountainKind::Radial);
        assert_eq!(f.first_color(), Color::rgb8(200, 0, 0));
        let hs = fountain_handles(&f, b);
        assert!(near(hs[0].1, Point::new(5.0, 5.0)) && near(hs[1].1, Point::new(15.0, 5.0)));
        assert!(near(hs[2].1, Point::new(5.0, 15.0)), "{hs:?}");
    }
}
