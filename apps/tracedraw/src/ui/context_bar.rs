//! Arcano edition: the contextual bar. A small floating bar under the
//! selection (at the foot of the drawing when nothing is selected) with
//! the actions that suit what is selected: for a picture the AI actions,
//! for text its styles and the house's typography, for objects grouping,
//! aligning and shaping; with nothing selected, making a picture or
//! vectors with AI, or importing. It hides while the pointer works on the
//! drawing, while text is typed, and can be turned off (its own button,
//! or Options > Appearance). See docs/behavior/context-bar.md.

use crate::app::{App, DockerTab};
use crate::i18n::tr;
use crate::theme::Tokens;
use crate::tools::Tool;
use crate::ui::ai_actions::AiAction;
use crate::ui::tips::RichTip as _;
use egui::{vec2, Rect, Ui};

/// One button of the bar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Item {
    Ai(AiAction),
    MoreAi,
    Group,
    Ungroup,
    Duplicate,
    Curves,
    Align,
    Shaping,
    TextStyles,
    Microtype,
    Import,
}

impl Item {
    fn key(self) -> &'static str {
        match self {
            Item::Ai(a) => a.key(),
            Item::MoreAi => "ctx.more_ai",
            Item::Group => "ctx.group",
            Item::Ungroup => "ctx.ungroup",
            Item::Duplicate => "ctx.duplicate",
            Item::Curves => "ctx.curves",
            Item::Align => "ctx.align",
            Item::Shaping => "ctx.shaping",
            Item::TextStyles => "ctx.text_styles",
            Item::Microtype => "ctx.microtype",
            Item::Import => "ctx.import",
        }
    }
}

/// What the bar offers for the selection now.
pub fn items(app: &App) -> Vec<Item> {
    let shapes = app.selected_shapes();
    if shapes.is_empty() {
        return vec![
            Item::Ai(AiAction::Image),
            Item::Ai(AiAction::Vector),
            Item::Import,
        ];
    }
    use tracedraw_core::document::ShapeKind;
    let bitmap = |s: &tracedraw_core::Shape| matches!(s.kind, ShapeKind::Bitmap { .. });
    let text = |s: &tracedraw_core::Shape| matches!(s.kind, ShapeKind::Text { .. });
    let group = |s: &tracedraw_core::Shape| matches!(s.kind, ShapeKind::Group { .. });
    if let [one] = shapes.as_slice() {
        if bitmap(one) {
            return vec![
                Item::Ai(AiAction::RemoveBackground),
                Item::Ai(AiAction::Expand),
                Item::Ai(AiAction::Upscale),
                Item::Ai(AiAction::Remix),
                Item::MoreAi,
            ];
        }
        if group(one) {
            return vec![Item::Ungroup, Item::Duplicate, Item::Align];
        }
    }
    if shapes.iter().all(text) {
        return vec![Item::TextStyles, Item::Microtype, Item::Curves];
    }
    if shapes.len() >= 2 {
        vec![Item::Group, Item::Align, Item::Shaping]
    } else {
        vec![Item::Duplicate, Item::Curves, Item::Shaping]
    }
}

/// Do what `item` stands for.
pub fn run(app: &mut App, item: Item) {
    match item {
        Item::Ai(a) => crate::ui::ai_actions::start(app, a),
        Item::MoreAi => {
            app.ai_image.home = true;
            app.ai_image.info = false;
            app.open_docker(DockerTab::AiGenerate);
        }
        Item::Group => app.group_selection(),
        Item::Ungroup => app.ungroup_selection(),
        Item::Duplicate => app.duplicate(),
        Item::Curves => app.convert_to_curves(),
        Item::Align => app.open_docker(DockerTab::AlignDistribute),
        Item::Shaping => app.open_docker(DockerTab::Shaping),
        Item::TextStyles => app.open_docker(DockerTab::TextStyles),
        Item::Microtype => {
            let _ = app.house_typography(true);
        }
        Item::Import => app.import(),
    }
}

/// The bar shows now.
fn shown(app: &App, ui: &Ui) -> bool {
    // A button held on the drawing (not on the bar itself) works there.
    let bar = ui
        .ctx()
        .memory(|m| m.area_rect(egui::Id::new("arcano_context_bar")));
    // The bar moving under its grip has left the place pressed.
    let moving = ui.ctx().is_being_dragged(grip_id());
    let busy = !moving
        && ui.input(|i| {
            i.pointer.any_down()
                && !i
                    .pointer
                    .press_origin()
                    .is_some_and(|p| bar.is_some_and(|r| r.contains(p)))
        });
    app.settings.context_bar
        && app.drawing_shown()
        && app.tool == Tool::Pick
        && app.text_edit.is_none()
        && !app.transforming()
        && app.docker_drag.is_none()
        && !busy
}

/// The bar over the drawing in `canvas`.
pub fn context_bar(app: &mut App, ui: &mut Ui, canvas: Rect) {
    if !shown(app, ui) {
        return;
    }
    let list = items(app);
    if list.is_empty() {
        return;
    }
    // Under the selection, else at the foot of the drawing; kept inside it.
    let at = match app.selection_bounds() {
        Some(b) if !app.selection.is_empty() => {
            let a = app
                .view
                .to_screen(tracedraw_core::geometry::Point::new(b.x0, b.y0));
            let c = app
                .view
                .to_screen(tracedraw_core::geometry::Point::new(b.x1, b.y0));
            egui::pos2((a.x + c.x) / 2.0, a.y.max(c.y) + 18.0)
        }
        _ => egui::pos2(canvas.center().x, canvas.bottom() - 56.0),
    };
    let id = egui::Id::new("arcano_context_bar");
    let width = ui
        .ctx()
        .memory(|m| m.area_rect(id))
        .map_or(360.0, |r| r.width());
    let height = ui
        .ctx()
        .memory(|m| m.area_rect(id))
        .map_or(36.0, |r| r.height());
    // Dragged by its grip: where it was left, else under the selection.
    let wanted = match app.settings.context_bar_at {
        Some([dx, dy]) => egui::pos2(canvas.left() + dx, canvas.top() + dy),
        None => egui::pos2(at.x - width / 2.0, at.y),
    };
    let top_left = keep_inside(wanted, vec2(width, height), canvas);
    let mut chosen = None;
    let mut hide = false;
    let mut moved = None;
    let mut back = false;
    egui::Area::new(id)
        .order(egui::Order::Middle)
        .fixed_pos(top_left)
        .show(ui.ctx(), |ui| {
            egui::Frame::new()
                .fill(Tokens::panel())
                .stroke(egui::Stroke::new(1.0, Tokens::window_border()))
                .corner_radius(10.0)
                .inner_margin(egui::Margin::symmetric(6, 4))
                .shadow(egui::epaint::Shadow {
                    offset: [0, 2],
                    blur: 10,
                    spread: 0,
                    color: egui::Color32::from_black_alpha(60),
                })
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        let g = grip(ui);
                        if g.dragged() {
                            moved = Some(g.drag_delta());
                        }
                        if g.double_clicked() {
                            back = true;
                        }
                        for item in &list {
                            if button(ui, *item).clicked() {
                                chosen = Some(*item);
                            }
                        }
                        ui.add_space(4.0);
                        ui.separator();
                        let close = ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("\u{00D7}").color(Tokens::text_dim()),
                                )
                                .frame(false),
                            )
                            .rich_tip(crate::ui::tips::pair("ctx.hide", "ctx.hide_desc"));
                        if close.clicked() {
                            hide = true;
                        }
                    });
                });
        });
    if let Some(d) = moved {
        let p = keep_inside(top_left + d, vec2(width, height), canvas);
        app.settings.context_bar_at = Some([p.x - canvas.left(), p.y - canvas.top()]);
    }
    if back {
        app.settings.context_bar_at = None;
    }
    if let Some(item) = chosen {
        run(app, item);
    }
    if hide {
        app.settings.context_bar = false;
        app.status = tr("ctx.hidden");
    }
}

/// The bar's top left corner for `wanted`, kept inside the drawing area
/// `canvas` (8 points in from its edges) for a bar of `size`.
fn keep_inside(wanted: egui::Pos2, size: egui::Vec2, canvas: Rect) -> egui::Pos2 {
    let x = wanted.x.clamp(
        canvas.left() + 8.0,
        (canvas.right() - size.x - 8.0).max(canvas.left() + 8.0),
    );
    let y = wanted.y.clamp(
        canvas.top() + 8.0,
        (canvas.bottom() - size.y - 8.0).max(canvas.top() + 8.0),
    );
    egui::pos2(x, y)
}

/// The grip's id, the same each frame wherever the bar is.
fn grip_id() -> egui::Id {
    egui::Id::new("arcano_context_bar_grip")
}

/// The grip at the bar's start: drag it to move the bar, double click it
/// to put the bar back under the selection.
fn grip(ui: &mut Ui) -> egui::Response {
    let (rect, _) = ui.allocate_exact_size(vec2(12.0, 28.0), egui::Sense::hover());
    let resp = ui.interact(rect, grip_id(), egui::Sense::click_and_drag());
    let colour = if resp.hovered() || resp.dragged() {
        Tokens::text()
    } else {
        Tokens::text_dim()
    };
    // Two columns of three dots.
    for col in [-2.0, 2.0] {
        for row in [-5.0, 0.0, 5.0] {
            ui.painter()
                .circle_filled(rect.center() + vec2(col, row), 1.2, colour);
        }
    }
    let resp = if resp.dragged() {
        resp.on_hover_cursor(egui::CursorIcon::Grabbing)
    } else {
        resp.on_hover_cursor(egui::CursorIcon::Grab)
    };
    resp.rich_tip(crate::ui::tips::pair("ctx.grip", "ctx.grip_desc"))
}

/// A button: the AI actions with their picture, the others plain.
fn button(ui: &mut Ui, item: Item) -> egui::Response {
    let label = tr(item.key());
    let galley = ui.painter().layout_no_wrap(
        label.clone(),
        egui::FontId::proportional(12.0),
        Tokens::text(),
    );
    let pic = matches!(item, Item::Ai(_) | Item::MoreAi);
    let w = galley.size().x + if pic { 34.0 } else { 18.0 };
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 28.0), egui::Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(rect, 7.0, Tokens::tool_hover());
    }
    let mut x = rect.left() + 9.0;
    if pic {
        let r = Rect::from_center_size(egui::pos2(x + 7.0, rect.center().y), vec2(15.0, 15.0));
        match item {
            Item::Ai(a) => crate::ui::ai_actions::draw(
                ui.painter(),
                r,
                a,
                Tokens::icon(),
                crate::ui::icons::accent(),
            ),
            _ => crate::ui::panel_icons::draw(ui.painter(), r, DockerTab::AiGenerate),
        }
        x += 18.0;
    }
    ui.painter().galley(
        egui::pos2(x, rect.center().y - galley.size().y / 2.0),
        galley,
        Tokens::text(),
    );
    let desc = format!("{}_desc", item.key());
    resp.rich_tip(crate::ui::tips::pair(item.key(), &desc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::ShapeKind;

    #[test]
    fn the_bar_suits_the_selection() {
        let mut app = App::headless();
        // Nothing selected: make something with AI, or import.
        assert_eq!(
            items(&app),
            vec![
                Item::Ai(AiAction::Image),
                Item::Ai(AiAction::Vector),
                Item::Import
            ]
        );
        let a = app
            .new_shape(ShapeKind::Rect {
                rect: tracedraw_core::Rect::new(0.0, 0.0, 10.0, 10.0),
                radius: 0.0,
                corners: None,
            })
            .unwrap();
        let b = app
            .new_shape(ShapeKind::Rect {
                rect: tracedraw_core::Rect::new(20.0, 0.0, 30.0, 10.0),
                radius: 0.0,
                corners: None,
            })
            .unwrap();
        app.select(vec![a]);
        assert!(items(&app).contains(&Item::Duplicate));
        app.select(vec![a, b]);
        assert!(items(&app).contains(&Item::Group));
        run(&mut app, Item::Group);
        assert_eq!(app.selection.len(), 1);
        assert!(items(&app).contains(&Item::Ungroup));
        // An action opens its page in the panel.
        run(&mut app, Item::Ai(AiAction::Expand));
        assert!(!app.ai_image.home);
        assert_eq!(AiAction::current(&app), AiAction::Expand);
        assert!(app.docker_open(DockerTab::AiGenerate));
        run(&mut app, Item::MoreAi);
        assert!(app.ai_image.home);
    }

    #[test]
    fn a_dragged_bar_stays_inside_the_drawing() {
        let canvas = Rect::from_min_size(egui::pos2(100.0, 50.0), vec2(800.0, 600.0));
        let size = vec2(300.0, 36.0);
        // Where it is wanted, when that is inside.
        let p = keep_inside(egui::pos2(400.0, 300.0), size, canvas);
        assert_eq!(p, egui::pos2(400.0, 300.0));
        // Dragged past the edges: held 8 points in.
        let p = keep_inside(egui::pos2(-500.0, 9000.0), size, canvas);
        assert_eq!(p, egui::pos2(108.0, 650.0 - 36.0 - 8.0));
        let p = keep_inside(egui::pos2(9000.0, -9000.0), size, canvas);
        assert_eq!(p, egui::pos2(900.0 - 300.0 - 8.0, 58.0));
    }

    #[test]
    fn the_grip_moves_the_bar_and_a_double_click_brings_it_back() {
        let mut app = App::headless();
        app.settings.context_bar = true;
        let ctx = crate::theme::ui_context();
        App::setup_context(&ctx);
        let screen = Rect::from_min_size(egui::Pos2::ZERO, vec2(1200.0, 800.0));
        let mut time = 0.0;
        let mut frame = |app: &mut App, events: Vec<egui::Event>| {
            time += 0.05;
            let input = egui::RawInput {
                screen_rect: Some(screen),
                time: Some(time),
                events,
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| context_bar(app, ui, screen));
            out.textures_delta.clear();
            ctx.memory(|m| m.area_rect(egui::Id::new("arcano_context_bar")))
                .expect("the bar shows")
        };
        frame(&mut app, vec![]);
        let bar = frame(&mut app, vec![]);
        let at = egui::pos2(bar.left() + 12.0, bar.center().y);
        let press = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        frame(&mut app, vec![egui::Event::PointerMoved(at)]);
        frame(&mut app, vec![press(at, true)]);
        let to = at + vec2(-200.0, -300.0);
        // Shown all along the drag, though it leaves the place pressed.
        for k in 1..=10 {
            let p = at + vec2(-20.0, -30.0) * k as f32;
            frame(&mut app, vec![egui::Event::PointerMoved(p)]);
        }
        frame(&mut app, vec![press(to, false)]);
        let moved = frame(&mut app, vec![]);
        assert!(app.settings.context_bar_at.is_some());
        assert!((moved.left() - (bar.left() - 200.0)).abs() < 1.0, "{bar:?} -> {moved:?}");
        assert!((moved.top() - (bar.top() - 300.0)).abs() < 1.0, "{bar:?} -> {moved:?}");
        // It stays there on the next frames.
        assert_eq!(frame(&mut app, vec![]).min, moved.min);
        // A double click on the grip: back under the selection (here the
        // foot of the drawing, nothing being selected).
        let g = egui::pos2(moved.left() + 12.0, moved.center().y);
        frame(&mut app, vec![egui::Event::PointerMoved(g)]);
        for _ in 0..2 {
            frame(&mut app, vec![press(g, true)]);
            frame(&mut app, vec![press(g, false)]);
        }
        frame(&mut app, vec![]);
        assert_eq!(app.settings.context_bar_at, None);
        let home = frame(&mut app, vec![]);
        assert!((home.top() - bar.top()).abs() < 1.0, "{bar:?} / {home:?}");
    }
}
