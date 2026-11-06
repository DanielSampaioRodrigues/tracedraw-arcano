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
    let busy = ui.input(|i| {
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
    let x = (at.x - width / 2.0).clamp(
        canvas.left() + 8.0,
        (canvas.right() - width - 8.0).max(canvas.left() + 8.0),
    );
    let y = at.y.clamp(
        canvas.top() + 8.0,
        (canvas.bottom() - 44.0).max(canvas.top() + 8.0),
    );
    let mut chosen = None;
    let mut hide = false;
    egui::Area::new(id)
        .order(egui::Order::Middle)
        .fixed_pos(egui::pos2(x, y))
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
    if let Some(item) = chosen {
        run(app, item);
    }
    if hide {
        app.settings.context_bar = false;
        app.status = tr("ctx.hidden");
    }
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
}
