//! The Pick tool's drags in the real window, with pointer events and
//! modifier keys: Ctrl keeps a move straight, a right click during a drag
//! leaves the original, Space drops a copy, Shift scales from the centre,
//! Ctrl scales in steps of 100 % (mirror at the same size), Alt frees a
//! corner, the side handles of rotate mode skew, and the centre of
//! rotation moves. Also the pointer's shape over handles and objects.

use crate::app::{App, Drag, Handle, HandleMods};
use crate::interaction::skew_transform;
use crate::tools::Tool;
use crate::ui::cursors::{for_tool, resize_icon, ShapeMark, ToolCursor};
use egui::{Event, Modifiers, PointerButton, Pos2};
use tracedraw_core::geometry::{Affine, Point, Rect, Vec2};
use tracedraw_core::{ShapeId, ShapeKind};

fn frame(ctx: &egui::Context, app: &mut App, events: Vec<Event>, modifiers: Modifiers) {
    // The keys held come first, as the windowing layer sends them.
    let mut all = vec![Event::ModifiersChanged(modifiers)];
    all.extend(events);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            Pos2::ZERO,
            egui::vec2(1600.0, 1000.0),
        )),
        events: all,
        ..Default::default()
    };
    let mut out = ctx.run_ui(input, |ui| crate::ui::root(app, ui));
    out.textures_delta.clear();
}

fn button(pos: Pos2, pressed: bool, button: PointerButton, modifiers: Modifiers) -> Event {
    Event::PointerButton {
        pos,
        button,
        pressed,
        modifiers,
    }
}

fn key(key: egui::Key, modifiers: Modifiers) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    }
}

/// A primary drag from `from` to `to` with `mods` held, `during` sent in
/// the middle of it.
fn drag(
    ctx: &egui::Context,
    app: &mut App,
    from: Pos2,
    to: Pos2,
    mods: Modifiers,
    during: Vec<Event>,
) {
    frame(ctx, app, vec![Event::PointerMoved(from)], mods);
    frame(
        ctx,
        app,
        vec![button(from, true, PointerButton::Primary, mods)],
        mods,
    );
    for i in 1..=4 {
        let p = from + (to - from) * (i as f32 / 4.0);
        let mut events = vec![Event::PointerMoved(p)];
        if i == 2 {
            events.extend(during.clone());
        }
        frame(ctx, app, events, mods);
    }
    frame(
        ctx,
        app,
        vec![button(to, false, PointerButton::Primary, mods)],
        mods,
    );
    frame(ctx, app, Vec::new(), Modifiers::NONE);
}

/// A drawing with a 50 x 30 mm rectangle at (10, 10), selected, the
/// page in view.
fn setup() -> (egui::Context, App, ShapeId) {
    let ctx = crate::theme::ui_context();
    let mut app = App::headless();
    app.show_welcome = false;
    let id = app
        .new_shape(ShapeKind::Rect {
            rect: Rect::new(10.0, 10.0, 60.0, 40.0),
            radius: 0.0,
            corners: None,
        })
        .expect("rect");
    app.set_tool(Tool::Pick);
    app.select(vec![id]);
    // Filled, so that a press inside it picks it.
    app.apply_fill(tracedraw_core::Fill::Solid(tracedraw_core::Color::rgb8(
        200, 30, 30,
    )));
    // The window's bars settle over the first frames; then the page fits.
    for _ in 0..3 {
        frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    }
    app.zoom_to_page();
    for _ in 0..2 {
        frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    }
    (ctx, app, id)
}

fn bounds(app: &App, id: ShapeId) -> Rect {
    app.doc().find_shape(id).map(|s| s.bounds()).expect("shape")
}

fn handle(app: &App, which: Handle) -> Pos2 {
    let b = app.selection_bounds().expect("selection");
    app.handle_positions(b)
        .iter()
        .find(|(h, _)| *h == which)
        .map(|(_, p)| *p)
        .expect("handle")
}

fn shape_count(app: &App) -> usize {
    app.doc()
        .all_layers()
        .map(|l| l.shapes.len())
        .sum::<usize>()
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 0.05
}

#[test]
fn ctrl_keeps_a_move_straight() {
    let (ctx, mut app, id) = setup();
    let from = app.view.to_screen(Point::new(35.0, 25.0));
    drag(
        &ctx,
        &mut app,
        from,
        from + egui::vec2(120.0, 35.0),
        Modifiers::CTRL,
        Vec::new(),
    );
    let b = bounds(&app, id);
    assert!(b.x0 > 15.0, "moved sideways: {b:?}");
    assert!(close(b.y0, 10.0), "kept its height: {b:?}");
    // Without Ctrl it follows the pointer both ways.
    let from = app.view.to_screen(b.center());
    drag(
        &ctx,
        &mut app,
        from,
        from + egui::vec2(0.0, -60.0),
        Modifiers::NONE,
        Vec::new(),
    );
    assert!(bounds(&app, id).y0 > b.y0 + 5.0);
}

#[test]
fn a_right_click_while_dragging_leaves_the_original() {
    let (ctx, mut app, id) = setup();
    let from = app.view.to_screen(Point::new(35.0, 25.0));
    let to = from + egui::vec2(150.0, 0.0);
    let right = |p: Pos2, down: bool| button(p, down, PointerButton::Secondary, Modifiers::NONE);
    let mid = from + (to - from) * 0.5;
    drag(
        &ctx,
        &mut app,
        from,
        to,
        Modifiers::NONE,
        vec![right(mid, true), right(mid, false)],
    );
    assert_eq!(shape_count(&app), 2);
    assert!(close(bounds(&app, id).x0, 10.0), "the original stays");
    // The copy is the selection, moved.
    let copy = app.selection.first().copied().expect("copy");
    assert_ne!(copy, id);
    assert!(bounds(&app, copy).x0 > 15.0);
    assert!(app.dialog_is_none_or_menu_closed());
}

#[test]
fn esc_cancels_a_drag() {
    let (ctx, mut app, id) = setup();
    let from = app.view.to_screen(Point::new(35.0, 25.0));
    drag(
        &ctx,
        &mut app,
        from,
        from + egui::vec2(150.0, 40.0),
        Modifiers::NONE,
        vec![key(egui::Key::Escape, Modifiers::NONE)],
    );
    assert!(close(bounds(&app, id).x0, 10.0), "nothing moved");
    assert_eq!(app.selection, vec![id], "the selection stays");
    assert!(matches!(app.drag, Drag::None));
}

#[test]
fn space_while_moving_drops_a_copy() {
    let (ctx, mut app, id) = setup();
    let from = app.view.to_screen(Point::new(35.0, 25.0));
    drag(
        &ctx,
        &mut app,
        from,
        from + egui::vec2(160.0, 0.0),
        Modifiers::NONE,
        vec![key(egui::Key::Space, Modifiers::NONE)],
    );
    // The original went all the way; a copy stayed half way.
    assert_eq!(shape_count(&app), 2);
    let moved = bounds(&app, id);
    let copy = app
        .doc()
        .all_layers()
        .flat_map(|l| &l.shapes)
        .find(|s| s.id != id)
        .map(|s| s.bounds())
        .expect("copy");
    assert!(
        copy.x0 > 10.5 && copy.x0 < moved.x0 - 0.5,
        "{copy:?} {moved:?}"
    );
    assert_eq!(app.tool, Tool::Pick);
}

#[test]
fn shift_scales_from_the_centre_and_ctrl_mirrors() {
    let (ctx, mut app, id) = setup();
    // Shift: the right side goes out, the left side as far the other way.
    let e = handle(&app, Handle::E);
    drag(
        &ctx,
        &mut app,
        e,
        e + egui::vec2(60.0, 0.0),
        Modifiers::SHIFT,
        Vec::new(),
    );
    let b = bounds(&app, id);
    assert!(close(b.center().x, 35.0), "{b:?}");
    assert!(b.width() > 55.0);
    app.undo();
    // Ctrl, dragged past the other side: the mirror image at 100 %.
    let e = handle(&app, Handle::E);
    let w = handle(&app, Handle::W);
    let past = w - egui::vec2((e.x - w.x) * 0.8, 0.0);
    drag(&ctx, &mut app, e, past, Modifiers::CTRL, Vec::new());
    let b = bounds(&app, id);
    assert!(close(b.width(), 50.0), "{b:?}");
    assert!(close(b.x1, 10.0) && close(b.x0, -40.0), "{b:?}");
    let t = app
        .doc()
        .find_shape(id)
        .map(|s| s.transform)
        .expect("shape");
    assert!(t.determinant() < 0.0, "mirrored");
}

#[test]
fn rotate_mode_skews_with_the_sides_and_turns_about_the_moved_centre() {
    let (ctx, mut app, id) = setup();
    app.rotate_mode = true;
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    // The top handle slants the rectangle sideways; the bottom stays.
    let n = handle(&app, Handle::N);
    drag(
        &ctx,
        &mut app,
        n,
        n + egui::vec2(80.0, 0.0),
        Modifiers::NONE,
        Vec::new(),
    );
    let b = bounds(&app, id);
    assert!(b.width() > 55.0, "slanted: {b:?}");
    assert!(close(b.y0, 10.0) && close(b.y1, 40.0), "{b:?}");
    app.undo();
    // Move the centre of rotation onto the lower left corner, then turn.
    app.rotate_mode = true;
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    let pivot = app.view.to_screen(Point::new(35.0, 25.0));
    let corner = app.view.to_screen(Point::new(10.0, 10.0));
    drag(&ctx, &mut app, pivot, corner, Modifiers::NONE, Vec::new());
    assert!(app.rotate_mode, "moving the centre keeps rotate mode");
    let c = app.rotation_center().expect("centre");
    assert!(close(c.x, 10.0) && close(c.y, 10.0), "{c:?}");
    // A quarter turn about (10, 10) with Ctrl (15 degree steps).
    let ne = handle(&app, Handle::NE);
    let centre = app.view.to_screen(c);
    let r = ne - centre;
    let quarter = centre + egui::vec2(-r.y, r.x);
    drag(&ctx, &mut app, ne, quarter, Modifiers::CTRL, Vec::new());
    let b = bounds(&app, id);
    // The corner on the centre stays where it was.
    assert!(
        close(b.x0, 10.0) || close(b.x1, 10.0),
        "turned about the corner: {b:?}"
    );
    assert!(close(b.width(), 30.0) && close(b.height(), 50.0), "{b:?}");
}

#[test]
fn handle_modifiers_shape_the_scale() {
    let app = App::headless();
    let b = Rect::new(0.0, 0.0, 40.0, 20.0);
    let scale = |h: Handle, anchor: Point, current: Point, mods: HandleMods| {
        app.preview_transform_of(&Drag::Scale {
            handle: h,
            anchor,
            start_bounds: b,
            current,
            mods,
        })
        .expect("transform")
    };
    let none = HandleMods::default();
    let shift = HandleMods {
        from_center: true,
        ..none
    };
    let ctrl = HandleMods {
        steps: true,
        ..none
    };
    let alt = HandleMods { free: true, ..none };
    let at = |t: Affine, x: f64, y: f64| t * Point::new(x, y);
    // Plain: the left side stays.
    let t = scale(
        Handle::E,
        Point::new(0.0, 10.0),
        Point::new(80.0, 10.0),
        none,
    );
    assert!(close(at(t, 40.0, 10.0).x, 80.0) && close(at(t, 0.0, 10.0).x, 0.0));
    // Shift: about the centre (20).
    let t = scale(
        Handle::E,
        Point::new(0.0, 10.0),
        Point::new(60.0, 10.0),
        shift,
    );
    assert!(close(at(t, 40.0, 10.0).x, 60.0) && close(at(t, 0.0, 10.0).x, -20.0));
    // Ctrl: whole steps; past the anchor the mirror image at 100 %.
    let t = scale(
        Handle::E,
        Point::new(0.0, 10.0),
        Point::new(-35.0, 10.0),
        ctrl,
    );
    assert!(close(at(t, 40.0, 10.0).x, -40.0));
    let t = scale(
        Handle::E,
        Point::new(0.0, 10.0),
        Point::new(70.0, 10.0),
        ctrl,
    );
    assert!(close(at(t, 40.0, 10.0).x, 80.0));
    // Ctrl and Shift: steps about the centre.
    let both = HandleMods {
        from_center: true,
        steps: true,
        ..none
    };
    let t = scale(
        Handle::E,
        Point::new(0.0, 10.0),
        Point::new(-15.0, 10.0),
        both,
    );
    assert!(close(at(t, 40.0, 10.0).x, -20.0));
    // A corner keeps the proportions unless Alt frees it.
    let t = scale(
        Handle::NE,
        Point::new(0.0, 0.0),
        Point::new(80.0, 20.0),
        none,
    );
    assert!(close(at(t, 40.0, 20.0).y, 40.0));
    let t = scale(
        Handle::NE,
        Point::new(0.0, 0.0),
        Point::new(80.0, 20.0),
        alt,
    );
    assert!(close(at(t, 40.0, 20.0).x, 80.0) && close(at(t, 40.0, 20.0).y, 20.0));
}

#[test]
fn side_handles_skew() {
    let b = Rect::new(0.0, 0.0, 40.0, 20.0);
    let none = HandleMods::default();
    let at = |t: Affine, x: f64, y: f64| t * Point::new(x, y);
    // Top handle: the top moves by the drag, the bottom stays.
    let t = skew_transform(Handle::N, b, Vec2::new(20.0, 0.0), none, 15.0).expect("skew");
    assert!(close(at(t, 0.0, 20.0).x, 20.0) && close(at(t, 0.0, 0.0).x, 0.0));
    // Bottom handle: the other way round.
    let t = skew_transform(Handle::S, b, Vec2::new(20.0, 0.0), none, 15.0).expect("skew");
    assert!(close(at(t, 0.0, 0.0).x, 20.0) && close(at(t, 0.0, 20.0).x, 0.0));
    // Right handle: up and down.
    let t = skew_transform(Handle::E, b, Vec2::new(0.0, 10.0), none, 15.0).expect("skew");
    assert!(close(at(t, 40.0, 0.0).y, 10.0) && close(at(t, 0.0, 0.0).y, 0.0));
    // Shift: about the centre line.
    let shift = HandleMods {
        from_center: true,
        ..none
    };
    let t = skew_transform(Handle::N, b, Vec2::new(20.0, 0.0), shift, 15.0).expect("skew");
    assert!(close(at(t, 0.0, 20.0).x, 20.0) && close(at(t, 0.0, 0.0).x, -20.0));
    // Ctrl: the slant in 15 degree steps (37 degrees becomes 30).
    let ctrl = HandleMods {
        steps: true,
        ..none
    };
    let t = skew_transform(Handle::N, b, Vec2::new(15.0, 0.0), ctrl, 15.0).expect("skew");
    let x = at(t, 0.0, 20.0).x;
    assert!((x - 20.0 * 30f64.to_radians().tan()).abs() < 1e-6, "{x}");
    // Corners and flat boxes make none.
    assert!(skew_transform(Handle::NE, b, Vec2::new(5.0, 0.0), none, 15.0).is_none());
    let flat = Rect::new(0.0, 0.0, 40.0, 0.0);
    assert!(skew_transform(Handle::N, flat, Vec2::new(5.0, 0.0), none, 15.0).is_none());
}

#[test]
fn the_pointer_shows_what_a_press_would_do() {
    let (ctx, mut app, id) = setup();
    let cursor = |app: &App, at: Pos2| for_tool(app, false, false, false, Some(at));
    // Scale handles: resize arrows along their direction.
    for h in [
        Handle::N,
        Handle::S,
        Handle::E,
        Handle::W,
        Handle::NE,
        Handle::NW,
        Handle::SE,
        Handle::SW,
    ] {
        assert_eq!(
            cursor(&app, handle(&app, h)),
            ToolCursor::System(resize_icon(h)),
            "{h:?}"
        );
    }
    // The selected object: the move arrows; elsewhere the plain arrow.
    let inside = app.view.to_screen(Point::new(35.0, 25.0));
    assert_eq!(
        cursor(&app, inside),
        ToolCursor::System(egui::CursorIcon::Move)
    );
    let empty = app.view.to_screen(Point::new(150.0, 250.0));
    assert_eq!(
        cursor(&app, empty),
        ToolCursor::System(egui::CursorIcon::Default)
    );
    // Rotate mode: corners turn, sides slant, the centre moves.
    app.rotate_mode = true;
    assert_eq!(cursor(&app, handle(&app, Handle::NE)), ToolCursor::Rotate);
    assert_eq!(
        cursor(&app, handle(&app, Handle::N)),
        ToolCursor::Skew { horizontal: true }
    );
    assert_eq!(
        cursor(&app, handle(&app, Handle::E)),
        ToolCursor::Skew { horizontal: false }
    );
    assert_eq!(
        cursor(&app, inside),
        ToolCursor::System(egui::CursorIcon::Move)
    );
    app.rotate_mode = false;
    // The Shape tool: its arrow, marked over a node and over a segment of
    // a curve.
    app.select(vec![id]);
    app.convert_to_curves();
    app.set_tool(Tool::Shape);
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    let node = app.view.to_screen(Point::new(10.0, 10.0));
    let segment = app.view.to_screen(Point::new(35.0, 10.0));
    assert_eq!(cursor(&app, node), ToolCursor::ShapeArrow(ShapeMark::Node));
    assert_eq!(
        cursor(&app, segment),
        ToolCursor::ShapeArrow(ShapeMark::Segment)
    );
    assert_eq!(
        cursor(&app, empty),
        ToolCursor::ShapeArrow(ShapeMark::Nothing)
    );
    // While dragging, the pointer keeps the drag's shape.
    app.set_tool(Tool::Pick);
    app.drag = Drag::Rotate {
        center: Point::new(0.0, 0.0),
        start_angle: 0.0,
        current_angle: 0.0,
    };
    assert_eq!(cursor(&app, empty), ToolCursor::Rotate);
    app.drag = Drag::None;
}

impl App {
    /// No context menu or drop menu was left open by a right click
    /// during a drag.
    fn dialog_is_none_or_menu_closed(&self) -> bool {
        self.context_menu.is_none() && self.drop_menu.is_none()
    }
}

#[test]
fn the_status_bar_tells_how_far_how_much_and_what_angle() {
    let (_ctx, mut app, _id) = setup();
    let info = |app: &App| crate::ui::status::drag_info(app).unwrap_or_default();
    assert!(info(&app).is_empty(), "nothing while not dragging");
    app.drag = Drag::Move {
        start: Point::new(0.0, 0.0),
        last: Point::new(0.0, 0.0),
        total: Vec2::new(30.0, 40.0),
        start_bounds: Rect::new(10.0, 10.0, 60.0, 40.0),
        source: None,
    };
    let text = info(&app);
    assert!(text.contains("50.000") && text.contains("53.1"), "{text}");
    app.drag = Drag::Scale {
        handle: Handle::E,
        anchor: Point::new(10.0, 25.0),
        start_bounds: Rect::new(10.0, 10.0, 60.0, 40.0),
        current: Point::new(85.0, 25.0),
        mods: HandleMods::default(),
    };
    let text = info(&app);
    assert!(text.contains("150.0") && text.contains("100.0"), "{text}");
    app.drag = Drag::Rotate {
        center: Point::new(0.0, 0.0),
        start_angle: 0.0,
        current_angle: 30f64.to_radians(),
    };
    assert!(info(&app).contains("30.0"), "{}", info(&app));
    app.drag = Drag::Skew {
        handle: Handle::N,
        start_bounds: Rect::new(10.0, 10.0, 60.0, 40.0),
        start: Point::new(35.0, 40.0),
        current: Point::new(65.0, 40.0),
        mods: HandleMods::default(),
    };
    assert!(info(&app).contains("45.0"), "{}", info(&app));
    // A copy says so.
    app.drag_copy = true;
    assert!(info(&app).contains(&crate::i18n::tr("status.drag_copy_short")));
    app.drag = Drag::None;
    app.drag_copy = false;
}
