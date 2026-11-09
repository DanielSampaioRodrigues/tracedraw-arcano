//! The Shaping docker: the ten operations as picture buttons at the
//! top; the one chosen shows its name, a large picture and what it does,
//! what it leaves of the originals (Weld, Trim and Intersect: the source
//! objects and the target, each operation keeping its own choice), and
//! the button that applies it. The last object selected is the target;
//! the others act on it.

use crate::app::App;
use crate::i18n::tr;
use crate::ops::Shaping;
use crate::theme::Tokens;
use crate::ui::propbar::{draw_pic, pic_button, Pic};
use crate::ui::tips::RichTip;
use egui::{Rect, Sense, Ui, Vec2};

/// The picture, name and description of an operation.
fn info(op: Shaping) -> (Pic, &'static str, &'static str) {
    match op {
        Shaping::Weld => (Pic::Weld, "menu.object.weld", "docker.weld_hint"),
        Shaping::Trim => (Pic::Trim, "menu.object.trim", "docker.trim_hint"),
        Shaping::Intersect => (
            Pic::Intersect,
            "menu.object.intersect",
            "docker.intersect_hint",
        ),
        Shaping::Simplify => (
            Pic::Simplify,
            "menu.object.simplify",
            "docker.simplify_hint",
        ),
        Shaping::FrontMinusBack => (
            Pic::FrontMinusBack,
            "menu.object.front_minus_back",
            "docker.front_minus_back_hint",
        ),
        Shaping::BackMinusFront => (
            Pic::BackMinusFront,
            "menu.object.back_minus_front",
            "docker.back_minus_front_hint",
        ),
        Shaping::Boundary => (
            Pic::Boundary,
            "menu.object.boundary",
            "docker.boundary_hint",
        ),
        Shaping::Divide => (
            Pic::Divide,
            "menu.object.divide",
            "docker.divide_hint",
        ),
        Shaping::OutlineStroke => (
            Pic::OutlineStroke,
            "menu.object.outline_stroke",
            "docker.outline_stroke_hint",
        ),
        Shaping::Merge => (Pic::Merge, "menu.object.merge", "docker.merge_hint"),
    }
}

/// Where Weld, Trim and Intersect keep their choice in
/// `App::shaping_leave`.
fn leave_slot(op: Shaping) -> Option<usize> {
    match op {
        Shaping::Weld => Some(0),
        Shaping::Trim => Some(1),
        Shaping::Intersect => Some(2),
        _ => None,
    }
}

/// Apply the docker's operation to the selection.
pub fn apply(app: &mut App) {
    let op = app.shaping_op;
    let (source, target) = leave_slot(op)
        .map(|i| (app.shaping_leave[i][0], app.shaping_leave[i][1]))
        .unwrap_or_else(|| op.default_leave());
    app.begin_shaping(op, source, target);
}

/// Draw the docker.
pub fn shaping(app: &mut App, ui: &mut Ui) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for op in Shaping::ALL {
            let (pic, key, _) = info(op);
            if pic_button(ui, pic, &tr(key), true, app.shaping_op == op).clicked() {
                app.shaping_op = op;
            }
        }
    });
    ui.add_space(6.0);
    let op = app.shaping_op;
    let (pic, key, hint) = info(op);
    ui.label(egui::RichText::new(tr(key)).font(crate::theme::bold(13.0)));
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(72.0), Sense::hover());
        ui.painter().rect_filled(rect, 3.0, egui::Color32::WHITE);
        ui.painter().rect_stroke(
            rect,
            3.0,
            egui::Stroke::new(1.0, Tokens::border()),
            egui::StrokeKind::Inside,
        );
        draw_pic(
            ui.painter(),
            Rect::from_center_size(rect.center(), Vec2::splat(56.0)),
            pic,
            Tokens::icon(),
        );
        ui.add_space(6.0);
        ui.vertical(|ui| {
            ui.label(tr(hint));
            ui.label(
                egui::RichText::new(tr("docker.shaping_target"))
                    .color(Tokens::text_dim())
                    .size(11.0),
            );
            if op.picks_target() {
                ui.label(
                    egui::RichText::new(tr("docker.shaping_pick"))
                        .color(Tokens::text_dim())
                        .size(11.0),
                );
            }
        });
    });
    ui.add_space(6.0);
    if let Some(i) = leave_slot(op) {
        ui.checkbox(
            &mut app.shaping_leave[i][0],
            tr("docker.leave_original_source"),
        )
        .rich_tip(crate::ui::tips::keyed("docker.leave_original_source"));
        ui.checkbox(
            &mut app.shaping_leave[i][1],
            tr("docker.leave_original_target"),
        )
        .rich_tip(crate::ui::tips::keyed("docker.leave_original_target"));
        ui.add_space(6.0);
    }
    let enough = op.ready(app.selection.len());
    let width = ui.available_width().min(240.0);
    let clicked = ui
        .add_enabled_ui(enough, |ui| {
            ui.add_sized([width, 26.0], egui::Button::new(tr(key)))
                .rich_tip_with(|| format!("{}\n{}", tr(key), tr("docker.shaping_apply_desc")))
                .clicked()
        })
        .inner;
    if clicked {
        apply(app);
    }
    if !enough {
        ui.label(
            egui::RichText::new(if op.needs() > 1 && !op.picks_target() {
                tr("docker.shaping_needs_two")
            } else {
                tr("docker.shaping_needs_one")
            })
            .color(Tokens::text_dim())
            .size(11.0),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::{Point, ShapeId, ShapeKind};

    fn rect(app: &mut App, x0: f64, y0: f64, x1: f64, y1: f64) -> ShapeId {
        app.new_shape(ShapeKind::Rect {
            rect: tracedraw_core::Rect::new(x0, y0, x1, y1),
            radius: 0.0,
            corners: None,
        })
        .expect("shape")
    }

    fn count(app: &App) -> usize {
        app.doc().pages[0].layers[0].shapes.len()
    }

    /// Trim keeps the object that cuts, Intersect keeps both, Weld
    /// replaces them; the docker's own choice is what it leaves.
    #[test]
    fn what_the_operations_leave_of_the_originals() {
        let mut app = App::headless();
        let cutter = rect(&mut app, 0.0, 0.0, 20.0, 20.0);
        let target = rect(&mut app, 10.0, 10.0, 30.0, 30.0);
        app.select(vec![cutter, target]);
        app.shaping(Shaping::Trim);
        assert!(app.doc().find_shape(cutter).is_some());
        assert!(app.doc().find_shape(target).is_none());
        assert_eq!(count(&app), 2);
        app.undo();
        app.select(vec![cutter, target]);
        app.shaping(Shaping::Intersect);
        assert_eq!(count(&app), 3);
        // The overlap: 10 x 10.
        let result = app.selection[0];
        let b = app.doc().shape_on_page(result).expect("result").bounds();
        assert!((b.width() - 10.0).abs() < 1e-6 && (b.height() - 10.0).abs() < 1e-6);
        app.undo();
        app.select(vec![cutter, target]);
        app.shaping(Shaping::Weld);
        assert_eq!(count(&app), 1);
        app.undo();
        // The docker: Weld leaving the target.
        app.select(vec![cutter, target]);
        app.shaping_op = Shaping::Weld;
        app.shaping_leave[0] = [false, true];
        apply(&mut app);
        assert!(app.doc().find_shape(cutter).is_none());
        assert!(app.doc().find_shape(target).is_some());
        assert_eq!(count(&app), 2);
        // Nothing found inside the welded shape is lost: it spans both.
        let welded = app.selection[0];
        let b = app.doc().shape_on_page(welded).expect("weld").bounds();
        assert!(b.contains(Point::new(29.0, 29.0)) && b.contains(Point::new(1.0, 1.0)));
    }

    #[test]
    fn the_docker_draws_every_operation() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        for (i, op) in Shaping::ALL.into_iter().enumerate() {
            if i == 3 {
                let a = rect(&mut app, 0.0, 0.0, 10.0, 10.0);
                app.select(vec![a]);
            }
            app.shaping_op = op;
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(320.0, 600.0),
                )),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| shaping(&mut app, ui));
            out.textures_delta.clear();
        }
        assert_eq!(Shaping::from_id("trim"), Shaping::Trim);
        assert_eq!(Shaping::from_id("no.such.key"), Shaping::Weld);
    }

    /// Two overlapping squares of different colours make three regions,
    /// each with the fill of the object on top of it, and the originals
    /// go.
    #[test]
    fn dividing_two_squares_makes_three_shapes_in_their_colours() {
        use tracedraw_core::geometry::Shape as _;
        use tracedraw_core::{Color, Fill};
        let red = Color::rgb8(220, 40, 40);
        let blue = Color::rgb8(40, 90, 220);
        let mut app = App::headless();
        let back = rect(&mut app, 0.0, 0.0, 20.0, 20.0);
        let front = rect(&mut app, 10.0, 10.0, 30.0, 30.0);
        for (id, color) in [(back, red), (front, blue)] {
            app.select(vec![id]);
            app.apply_fill(Fill::Solid(color));
        }
        app.select(vec![back, front]);
        app.shaping(Shaping::Divide);
        assert_eq!(count(&app), 3);
        assert!(app.doc().find_shape(back).is_none() && app.doc().find_shape(front).is_none());
        let shapes = &app.doc().pages[0].layers[0].shapes;
        let of = |c: Color| shapes.iter().filter(|s| s.fill == Fill::Solid(c)).count();
        assert_eq!(of(red), 1);
        assert_eq!(of(blue), 2);
        // Nothing lost and nothing counted twice: the union of the two.
        let area: f64 = shapes.iter().map(|s| s.page_path().area().abs()).sum();
        assert!((area - 700.0).abs() < 0.5, "{area}");

        // Five objects are more regions than Divide works out: nothing
        // changes and the status bar says so.
        let mut app = App::headless();
        let ids: Vec<ShapeId> = (0..5)
            .map(|i| {
                let x = i as f64 * 10.0;
                rect(&mut app, x, 0.0, x + 8.0, 8.0)
            })
            .collect();
        app.select(ids);
        app.shaping(Shaping::Divide);
        assert_eq!(count(&app), 5);
        assert_eq!(app.status, crate::i18n::tr("status.shaping_too_many"));
    }

    /// Outline Stroke: the outline of a square becomes a filled object in
    /// the outline's own colour, a millimetre further out on every side
    /// (a centred 2 mm outline), and the original goes.
    #[test]
    fn outlining_a_square_gives_the_band_of_its_outline() {
        use tracedraw_core::{Color, Fill};
        let ink = Color::rgb8(30, 30, 30);
        let mut app = App::headless();
        let id = rect(&mut app, 0.0, 0.0, 20.0, 20.0);
        app.select(vec![id]);
        app.apply_outline_color(Some(ink));
        app.apply_outline_width(2.0);
        app.select(vec![id]);
        app.shaping(Shaping::OutlineStroke);
        assert!(app.doc().find_shape(id).is_none());
        assert_eq!(count(&app), 1);
        let shape = app.doc().pages[0].layers[0].shapes[0].clone();
        assert_eq!(shape.fill, Fill::Solid(ink));
        let b = shape.bounds();
        assert!((b.width() - 22.0).abs() < 0.05, "{b:?}");
        assert!((b.height() - 22.0).abs() < 0.05, "{b:?}");
        assert!((b.x0 + 1.0).abs() < 0.05, "{b:?}");
    }

    /// Merge: one object per fill colour, and the colour in front keeps
    /// the overlap, so the red one behind loses it.
    #[test]
    fn merging_two_colours_keeps_the_front_one_whole() {
        use tracedraw_core::geometry::Shape as _;
        use tracedraw_core::{Color, Fill};
        let red = Color::rgb8(220, 40, 40);
        let blue = Color::rgb8(40, 90, 220);
        let mut app = App::headless();
        let back = rect(&mut app, 0.0, 0.0, 20.0, 20.0);
        let front = rect(&mut app, 10.0, 10.0, 30.0, 30.0);
        for (id, color) in [(back, red), (front, blue)] {
            app.select(vec![id]);
            app.apply_fill(Fill::Solid(color));
        }
        app.select(vec![back, front]);
        app.shaping(Shaping::Merge);
        assert_eq!(count(&app), 2);
        assert!(app.doc().find_shape(back).is_none() && app.doc().find_shape(front).is_none());
        for s in &app.doc().pages[0].layers[0].shapes {
            let area = s.page_path().area().abs();
            let want = match &s.fill {
                Fill::Solid(c) if *c == blue => 400.0,
                Fill::Solid(c) if *c == red => 300.0,
                other => panic!("unexpected fill {other:?}"),
            };
            assert!((area - want).abs() < 0.5, "{area} where {want} was due");
        }
    }
}
