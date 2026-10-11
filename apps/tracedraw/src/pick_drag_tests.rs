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

#[test]
fn objects_dropped_on_the_document_palette_give_their_colours() {
    let (ctx, mut app, id) = setup();
    app.settings.palette.show_document = true;
    for _ in 0..2 {
        frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    }
    let row = app.doc_palette_rect.expect("the document palette shows");
    let from = app.view.to_screen(Point::new(35.0, 25.0));
    drag(
        &ctx,
        &mut app,
        from,
        row.center(),
        Modifiers::NONE,
        Vec::new(),
    );
    // The object stays; its fill joined the document palette.
    assert!(close(bounds(&app, id).x0, 10.0), "{:?}", bounds(&app, id));
    assert!(app
        .doc()
        .palette
        .contains(&tracedraw_core::Color::rgb8(200, 30, 30)));
}

fn click_at(ctx: &egui::Context, app: &mut App, at: Pos2, mods: Modifiers) {
    frame(ctx, app, vec![Event::PointerMoved(at)], mods);
    frame(
        ctx,
        app,
        vec![button(at, true, PointerButton::Primary, mods)],
        mods,
    );
    frame(
        ctx,
        app,
        vec![button(at, false, PointerButton::Primary, mods)],
        mods,
    );
    // Past the double-click time (frames advance the input clock by a
    // sixtieth of a second each), so the next click is a click of its own.
    for _ in 0..20 {
        frame(ctx, app, Vec::new(), mods);
    }
}

fn last_path(app: &App) -> tracedraw_core::BezPath {
    app.doc()
        .all_layers()
        .flat_map(|l| &l.shapes)
        .filter_map(|s| match &s.kind {
            ShapeKind::Path { .. } => Some(s.page_path()),
            _ => None,
        })
        .last()
        .expect("a curve")
}

#[test]
fn ctrl_keeps_lines_to_the_constrain_angle() {
    use crate::interaction::constrain_from;
    let o = Point::new(0.0, 0.0);
    // 5 degrees off the horizontal comes back to it, at the same reach
    // along it.
    let p = constrain_from(o, Point::new(10.0, 5f64.to_radians().tan() * 10.0), 15.0);
    assert!(p.y.abs() < 1e-9 && (p.x - 10.0).abs() < 1e-9, "{p:?}");
    // 40 degrees goes to 45.
    let a = 40f64.to_radians();
    let p = constrain_from(o, Point::new(a.cos() * 10.0, a.sin() * 10.0), 15.0);
    assert!((p.y.atan2(p.x).to_degrees() - 45.0).abs() < 1e-9);
    assert_eq!(constrain_from(o, o, 15.0), o);

    // Freehand: two clicks make a straight line; with Ctrl a level one.
    let (ctx, mut app, _id) = setup();
    app.set_tool(Tool::Freehand);
    let a = app.view.to_screen(Point::new(100.0, 100.0));
    let b = app.view.to_screen(Point::new(160.0, 104.0));
    click_at(&ctx, &mut app, a, Modifiers::NONE);
    click_at(&ctx, &mut app, b, Modifiers::CTRL);
    let path = last_path(&app);
    let pts: Vec<Point> = path
        .elements()
        .iter()
        .filter_map(|e| match e {
            tracedraw_core::geometry::PathEl::MoveTo(p)
            | tracedraw_core::geometry::PathEl::LineTo(p) => Some(*p),
            _ => None,
        })
        .collect();
    assert_eq!(pts.len(), 2, "{pts:?}");
    assert!((pts[0].y - pts[1].y).abs() < 1e-6, "level: {pts:?}");
    assert!(pts[1].x > pts[0].x + 50.0);

    // Polyline: Ctrl keeps the next segment to 15 degree steps.
    app.set_tool(Tool::Polyline);
    let a = app.view.to_screen(Point::new(100.0, 150.0));
    let b = app.view.to_screen(Point::new(150.0, 196.0));
    click_at(&ctx, &mut app, a, Modifiers::NONE);
    click_at(&ctx, &mut app, b, Modifiers::CTRL);
    app.finish_curve();
    let path = last_path(&app);
    let pts: Vec<Point> = path
        .elements()
        .iter()
        .filter_map(|e| match e {
            tracedraw_core::geometry::PathEl::MoveTo(p)
            | tracedraw_core::geometry::PathEl::LineTo(p) => Some(*p),
            _ => None,
        })
        .collect();
    let d = pts[1] - pts[0];
    assert!(
        (d.y.atan2(d.x).to_degrees() - 45.0).abs() < 1e-6,
        "45 degrees: {pts:?}"
    );
}

/// Two filled 20 x 20 mm squares grouped (at x 100 and 140, y 100) and a
/// third square on its own at (100, 150), nothing selected.
fn grouped() -> (egui::Context, App, [ShapeId; 3], ShapeId) {
    let (ctx, mut app, first) = setup();
    app.select(vec![first]);
    app.delete_selection();
    let mut ids = Vec::new();
    for (x, y) in [(100.0, 100.0), (140.0, 100.0), (100.0, 150.0)] {
        let id = app
            .new_shape(ShapeKind::Rect {
                rect: Rect::new(x, y, x + 20.0, y + 20.0),
                radius: 0.0,
                corners: None,
            })
            .expect("rect");
        app.select(vec![id]);
        app.apply_fill(tracedraw_core::Fill::Solid(tracedraw_core::Color::rgb8(
            30, 30, 200,
        )));
        ids.push(id);
    }
    app.select(vec![ids[0], ids[1]]);
    app.group_selection();
    let group = app.selection[0];
    // The group is moved, so its objects' space is not the page's.
    app.run(tracedraw_core::Command::TransformShapes {
        shapes: vec![group],
        transform: Affine::translate((5.0, 0.0)),
    });
    app.select(Vec::new());
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    (ctx, app, [ids[0], ids[1], ids[2]], group)
}

fn page_bounds(app: &App, id: ShapeId) -> Rect {
    app.doc()
        .shape_on_page(id)
        .map(|s| s.bounds())
        .expect("shape")
}

#[test]
fn ctrl_click_chooses_an_object_inside_a_group() {
    let (ctx, mut app, [a, b, _c], group) = grouped();
    let on_a = app.view.to_screen(Point::new(115.0, 110.0));
    // A plain click takes the group.
    click_at(&ctx, &mut app, on_a, Modifiers::NONE);
    assert_eq!(app.selection, vec![group]);
    // Ctrl+click: the square inside it, with round handles and the status
    // bar saying where it is.
    click_at(&ctx, &mut app, on_a, Modifiers::CTRL);
    assert_eq!(app.selection, vec![a]);
    assert!(app.selection_nested());
    assert!(crate::ui::status::object_info(&app).contains("in a group"));
    let before_b = page_bounds(&app, b);
    // Dragging it moves that square only, by the drag on the page.
    let before = page_bounds(&app, a);
    let to = app.view.to_screen(Point::new(115.0, 130.0));
    drag(&ctx, &mut app, on_a, to, Modifiers::NONE, Vec::new());
    let after = page_bounds(&app, a);
    assert!(close(after.y0 - before.y0, 20.0), "{before:?} {after:?}");
    assert!(close(after.x0, before.x0));
    assert_eq!(page_bounds(&app, b), before_b);
    assert_eq!(app.doc().top_level_of(a), Some(group));
    // Undo and redo keep it chosen.
    app.undo();
    assert_eq!(app.selection, vec![a]);
    assert!(close(page_bounds(&app, a).y0, before.y0));
    app.redo();
    assert_eq!(app.selection, vec![a]);
    // A colour reaches it alone.
    app.apply_fill(tracedraw_core::Fill::Solid(tracedraw_core::Color::rgb8(
        0, 200, 0,
    )));
    let fill_of = |app: &App, id| app.doc().find_shape(id).map(|s| s.fill.clone());
    assert_ne!(fill_of(&app, a), fill_of(&app, b));
    // Ctrl+Shift+click adds the other square; the group is never selected
    // with its own objects.
    let on_b = app.view.to_screen(Point::new(155.0, 110.0));
    click_at(&ctx, &mut app, on_b, Modifiers::CTRL | Modifiers::SHIFT);
    assert_eq!(app.selection, vec![a, b]);
    app.select_add(group);
    assert_eq!(app.selection, vec![group]);
    // Delete takes the chosen square out of the group.
    app.select(vec![a]);
    app.delete_selection();
    assert!(app.doc().find_shape(a).is_none());
    assert_eq!(app.doc().top_level_of(b), Some(group));
}

#[test]
fn alt_click_digs_to_the_object_beneath() {
    let (ctx, mut app, [_a, _b, c], _group) = grouped();
    // A fourth square over the third one.
    let top = app
        .new_shape(ShapeKind::Rect {
            rect: Rect::new(105.0, 155.0, 125.0, 175.0),
            radius: 0.0,
            corners: None,
        })
        .expect("rect");
    app.select(vec![top]);
    app.apply_fill(tracedraw_core::Fill::Solid(tracedraw_core::Color::rgb8(
        200, 200, 0,
    )));
    let both = app.view.to_screen(Point::new(110.0, 160.0));
    click_at(&ctx, &mut app, both, Modifiers::NONE);
    assert_eq!(app.selection, vec![top]);
    click_at(&ctx, &mut app, both, Modifiers::ALT);
    assert_eq!(app.selection, vec![c]);
    // Once more: round to the top again.
    click_at(&ctx, &mut app, both, Modifiers::ALT);
    assert_eq!(app.selection, vec![top]);
    assert!(!app.rotate_mode);
}

#[test]
fn alt_marquees_take_what_they_touch() {
    let (ctx, mut app, [_a, _b, c], group) = grouped();
    // Across the lone square and the group's first square, holding
    // neither wholly.
    let from = app.view.to_screen(Point::new(90.0, 112.0));
    let to = app.view.to_screen(Point::new(112.0, 158.0));
    drag(&ctx, &mut app, from, to, Modifiers::NONE, Vec::new());
    assert!(app.selection.is_empty());
    drag(&ctx, &mut app, from, to, Modifiers::ALT, Vec::new());
    let mut sel = app.selection.clone();
    sel.sort_by_key(|s| s.raw());
    let mut want = vec![group, c];
    want.sort_by_key(|s| s.raw());
    assert_eq!(sel, want);
    // Round all three: everything.
    app.select(Vec::new());
    let from = app.view.to_screen(Point::new(90.0, 90.0));
    let to = app.view.to_screen(Point::new(180.0, 180.0));
    drag(&ctx, &mut app, from, to, Modifiers::NONE, Vec::new());
    assert_eq!(app.selection.len(), 2);
}

/// The screen points of a lasso through `pts` (page), in small steps as a
/// hand draws it.
fn lasso_path(app: &App, pts: &[Point]) -> Vec<Pos2> {
    let mut out = Vec::new();
    for w in pts.windows(2) {
        for i in 0..8 {
            let t = i as f64 / 8.0;
            out.push(app.view.to_screen(w[0] + (w[1] - w[0]) * t));
        }
    }
    if let Some(last) = pts.last() {
        out.push(app.view.to_screen(*last));
    }
    out
}

#[test]
fn the_freehand_pick_lassoes_and_has_the_pick_tools_handles() {
    let (ctx, mut app, [_a, _b, c], group) = grouped();
    app.set_tool(Tool::FreeformPick);
    // A loop round the lone square only.
    let pts = [
        Point::new(90.0, 140.0),
        Point::new(130.0, 140.0),
        Point::new(130.0, 180.0),
        Point::new(90.0, 180.0),
        Point::new(90.0, 141.0),
    ];
    let screen = lasso_path(&app, &pts);
    let lasso = |ctx: &egui::Context, app: &mut App, mods: Modifiers| {
        frame(ctx, app, vec![Event::PointerMoved(screen[0])], mods);
        frame(
            ctx,
            app,
            vec![button(screen[0], true, PointerButton::Primary, mods)],
            mods,
        );
        for p in &screen[1..] {
            frame(ctx, app, vec![Event::PointerMoved(*p)], mods);
        }
        frame(
            ctx,
            app,
            vec![button(
                *screen.last().expect("point"),
                false,
                PointerButton::Primary,
                mods,
            )],
            mods,
        );
        frame(ctx, app, Vec::new(), Modifiers::NONE);
    };
    lasso(&ctx, &mut app, Modifiers::NONE);
    assert_eq!(app.selection, vec![c]);
    // The handles scale as with the Pick tool.
    let before = page_bounds(&app, c);
    let ne = handle(&app, Handle::NE);
    drag(
        &ctx,
        &mut app,
        ne,
        ne + egui::vec2(40.0, -40.0),
        Modifiers::NONE,
        Vec::new(),
    );
    assert!(page_bounds(&app, c).width() > before.width() + 1.0);
    // A loop through the group's first square takes the group with Alt.
    app.select(Vec::new());
    let screen2 = lasso_path(
        &app,
        &[
            Point::new(110.0, 90.0),
            Point::new(130.0, 90.0),
            Point::new(130.0, 130.0),
            Point::new(110.0, 130.0),
        ],
    );
    let mods = Modifiers::ALT;
    frame(&ctx, &mut app, vec![Event::PointerMoved(screen2[0])], mods);
    frame(
        &ctx,
        &mut app,
        vec![button(screen2[0], true, PointerButton::Primary, mods)],
        mods,
    );
    for p in &screen2[1..] {
        frame(&ctx, &mut app, vec![Event::PointerMoved(*p)], mods);
    }
    frame(
        &ctx,
        &mut app,
        vec![button(
            *screen2.last().expect("point"),
            false,
            PointerButton::Primary,
            mods,
        )],
        mods,
    );
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    assert_eq!(app.selection, vec![group]);
}

#[test]
fn clicking_the_selection_or_its_centre_mark_turns_the_handles() {
    let (ctx, mut app, [_a, _b, c], group) = grouped();
    app.select(vec![group, c]);
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    // Any selected object toggles rotate mode for the whole selection.
    let on_c = app.view.to_screen(Point::new(110.0, 160.0));
    click_at(&ctx, &mut app, on_c, Modifiers::NONE);
    assert!(app.rotate_mode);
    assert_eq!(app.selection.len(), 2);
    // A moved centre of rotation goes back to the middle when rotate mode
    // comes on again.
    let middle = app.selection_bounds().expect("bounds").center();
    app.rotation_pivot = Some((app.selection.clone(), Point::new(0.0, 0.0)));
    click_at(&ctx, &mut app, on_c, Modifiers::NONE);
    assert!(!app.rotate_mode);
    click_at(&ctx, &mut app, on_c, Modifiers::NONE);
    assert!(app.rotate_mode);
    assert_eq!(app.rotation_center(), Some(middle));
    // The centre mark of an outline-only ring: it lies on nothing, yet a
    // click there turns the handles and a drag moves the ring.
    app.select(Vec::new());
    let ring = app
        .new_shape(ShapeKind::Ellipse {
            rect: Rect::new(20.0, 200.0, 60.0, 240.0),
            arc: None,
        })
        .expect("ellipse");
    app.select(vec![ring]);
    frame(&ctx, &mut app, Vec::new(), Modifiers::NONE);
    let mark = app.view.to_screen(Point::new(40.0, 220.0));
    assert_eq!(app.hit_test(Point::new(40.0, 220.0)), None);
    assert_eq!(
        for_tool(&app, false, false, false, Some(mark)),
        ToolCursor::System(egui::CursorIcon::Move)
    );
    let before = page_bounds(&app, ring);
    let to = app.view.to_screen(Point::new(70.0, 220.0));
    drag(&ctx, &mut app, mark, to, Modifiers::NONE, Vec::new());
    assert!(close(page_bounds(&app, ring).x0 - before.x0, 30.0));
    assert_eq!(app.selection, vec![ring]);
    let moved = app.view.to_screen(Point::new(70.0, 220.0));
    click_at(&ctx, &mut app, moved, Modifiers::NONE);
    assert!(app.rotate_mode);
}

#[test]
fn the_pointer_shows_moves_for_objects_and_guidelines() {
    let (_ctx, mut app, [a, _b, _c], _group) = grouped();
    let cursor = |app: &App, at: Pos2| for_tool(app, false, false, false, Some(at));
    // Unselected objects show the move arrows too.
    assert!(app.selection.is_empty());
    let on_a = app.view.to_screen(Point::new(115.0, 110.0));
    assert_eq!(
        cursor(&app, on_a),
        ToolCursor::System(egui::CursorIcon::Move)
    );
    assert!(app.doc().is_nested(a));
    // Guidelines: arrows across them; a locked one keeps the arrow.
    app.add_guide(tracedraw_core::document::Guide::horizontal(250.0));
    app.add_guide(tracedraw_core::document::Guide::vertical(250.0));
    let on_h = app.view.to_screen(Point::new(200.0, 250.0));
    let on_v = app.view.to_screen(Point::new(250.0, 200.0));
    assert_eq!(
        cursor(&app, on_h),
        ToolCursor::System(egui::CursorIcon::ResizeVertical)
    );
    assert_eq!(
        cursor(&app, on_v),
        ToolCursor::System(egui::CursorIcon::ResizeHorizontal)
    );
    app.set_guides_locked(&[0], true);
    assert_eq!(
        cursor(&app, on_h),
        ToolCursor::System(egui::CursorIcon::Default)
    );
    // Dragging a new guideline out of a ruler.
    app.drag = Drag::NewGuide {
        horizontal: true,
        pos: Point::ZERO,
    };
    assert_eq!(
        crate::ui::cursors::ruler_drag_cursor(&app),
        Some(egui::CursorIcon::ResizeVertical)
    );
    app.drag = Drag::None;
}
