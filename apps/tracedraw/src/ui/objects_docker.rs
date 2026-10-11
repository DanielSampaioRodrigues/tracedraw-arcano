//! The Objects docker: the current page's layers, top first, each with a
//! collapse arrow, an eye (show or hide), a padlock (lock or unlock) and
//! its name; under each open layer its objects, topmost first, each with a
//! swatch of its fill and outline and its name (or what it is when it has
//! no name). Groups open to show their objects. Clicking an object selects
//! it (Shift adds it); double-clicking a name renames the layer or object;
//! a layer's context menu moves, renames or deletes it.

use crate::app::App;
use crate::i18n::{tr, trf};
use crate::theme::Tokens;
use crate::ui::dockers::kind_name;
use egui::{Color32, Pos2, Rect, Sense, Stroke, Ui, Vec2};
use tracedraw_core::{Command, LayerId, Shape, ShapeId, ShapeKind};

/// Height of a row.
const ROW_H: f32 = 22.0;
/// Indent per nesting level.
const INDENT: f32 = 16.0;
const SELECTED: Color32 = Color32::from_rgb(0xCC, 0xE8, 0xFF);
const SELECTED_EDGE: Color32 = Color32::from_rgb(0x99, 0xD1, 0xFF);
const HOVER: Color32 = Color32::from_rgb(0xE5, 0xF3, 0xFF);
const LAYER_BG: Color32 = Color32::from_rgb(0xEA, 0xEA, 0xEA);

/// What the docker asks for after drawing.
#[derive(Default)]
struct Actions {
    toggles: Vec<(LayerId, bool, bool)>,
    select: Option<ShapeId>,
    rename_layer: Option<(LayerId, String)>,
    rename_object: Option<(ShapeId, String)>,
    delete_layer: Option<LayerId>,
    reorder: Option<(LayerId, usize)>,
    collapse: Option<LayerId>,
    expand_group: Option<ShapeId>,
    add_layer: bool,
}

/// The label an object shows: its name, else what it is.
pub fn object_label(s: &Shape) -> String {
    match s.name.as_deref().map(str::trim) {
        Some(n) if !n.is_empty() => n.to_string(),
        _ => kind_name(&s.kind),
    }
}

/// An eye, open when `on`, else closed (a lid line), centred in `r`.
fn paint_eye(ui: &Ui, r: Rect, on: bool) {
    let p = ui.painter();
    let c = r.center();
    let color = if on { Tokens::ICON } else { Tokens::ICON_OFF };
    let w = 6.0;
    let pts_top: Vec<Pos2> = (0..=12)
        .map(|i| {
            let t = i as f32 / 12.0;
            let x = -w + 2.0 * w * t;
            Pos2::new(c.x + x, c.y - 3.6 * (1.0 - (x / w).powi(2)))
        })
        .collect();
    let pts_bottom: Vec<Pos2> = pts_top
        .iter()
        .map(|q| Pos2::new(q.x, c.y + (c.y - q.y)))
        .collect();
    let stroke = Stroke::new(1.1, color);
    p.add(egui::Shape::line(pts_top, stroke));
    if on {
        p.add(egui::Shape::line(pts_bottom, stroke));
        p.circle_filled(c, 1.9, color);
    } else {
        // Closed: only the lower lid with lashes.
        p.add(egui::Shape::line(pts_bottom.clone(), stroke));
        for dx in [-3.5f32, 0.0, 3.5] {
            let base = Pos2::new(c.x + dx, c.y + 3.0 * (1.0 - (dx / w).powi(2)));
            p.line_segment([base, base + Vec2::new(dx * 0.15, 2.0)], stroke);
        }
    }
}

/// A padlock, shut when `locked`, else open, centred in `r`.
fn paint_lock(ui: &Ui, r: Rect, locked: bool) {
    let p = ui.painter();
    let c = r.center();
    let color = if locked {
        Tokens::ICON
    } else {
        Tokens::ICON_OFF
    };
    let body = Rect::from_center_size(c + Vec2::new(0.0, 2.0), Vec2::new(9.0, 7.0));
    p.rect_filled(body, 1.0, color);
    let stroke = Stroke::new(1.3, color);
    let left = body.left() + 2.2;
    let right = body.right() - 2.2;
    let top = body.top() - 4.0;
    let shackle_right = if locked { right } else { right + 3.0 };
    let arc: Vec<Pos2> = (0..=10)
        .map(|i| {
            let a = std::f32::consts::PI * i as f32 / 10.0;
            let mid = (left + shackle_right) / 2.0;
            let rx = (shackle_right - left) / 2.0;
            Pos2::new(mid - rx * a.cos(), top - 0.5 - 2.0 * a.sin() + 2.0)
        })
        .collect();
    p.add(egui::Shape::line(arc, stroke));
    p.line_segment(
        [Pos2::new(left, top + 1.5), Pos2::new(left, body.top())],
        stroke,
    );
    let right_end = if locked { body.top() } else { top + 3.0 };
    p.line_segment(
        [
            Pos2::new(shackle_right, top + 1.5),
            Pos2::new(shackle_right, right_end),
        ],
        stroke,
    );
}

/// Three stacked sheets: the layer icon.
fn paint_layer_icon(ui: &Ui, r: Rect) {
    let p = ui.painter();
    let c = r.center();
    for (i, dy) in [3.0f32, 0.0, -3.0].iter().enumerate() {
        let pts = vec![
            Pos2::new(c.x - 6.0, c.y + dy),
            Pos2::new(c.x, c.y + dy - 2.8),
            Pos2::new(c.x + 6.0, c.y + dy),
            Pos2::new(c.x, c.y + dy + 2.8),
        ];
        let fill = if i == 2 {
            Color32::from_rgb(0x4A, 0x90, 0xD9)
        } else {
            Color32::from_rgb(0xC9, 0xDD, 0xF2)
        };
        p.add(egui::Shape::convex_polygon(
            pts,
            fill,
            Stroke::new(0.8, Color32::from_rgb(0x2F, 0x6D, 0xB5)),
        ));
    }
}

/// The open (pointing down) or closed (pointing right) arrow.
fn paint_arrow(ui: &Ui, r: Rect, open: bool) {
    let c = r.center();
    let pts = if open {
        vec![
            c + Vec2::new(-3.5, -1.8),
            c + Vec2::new(3.5, -1.8),
            c + Vec2::new(0.0, 2.2),
        ]
    } else {
        vec![
            c + Vec2::new(-1.8, -3.5),
            c + Vec2::new(2.2, 0.0),
            c + Vec2::new(-1.8, 3.5),
        ]
    };
    ui.painter().add(egui::Shape::convex_polygon(
        pts,
        Tokens::TEXT_DIM,
        Stroke::NONE,
    ));
}

/// The swatch of an object: its fill (or a crossed box for none) inside
/// its outline colour.
fn paint_swatch(ui: &Ui, r: Rect, s: &Shape) {
    let p = ui.painter();
    let fill = crate::app::fill_preview_color(&s.fill);
    let edge = s
        .stroke
        .as_ref()
        .map(|st| crate::canvas::to_color32(st.color))
        .unwrap_or(Tokens::TEXT_DIM);
    match fill {
        Some(c) => {
            p.rect_filled(r, 1.0, c);
        }
        None => {
            p.rect_filled(r, 1.0, Color32::WHITE);
            p.line_segment(
                [r.left_bottom(), r.right_top()],
                Stroke::new(1.0, Color32::from_rgb(0xE0, 0x40, 0x40)),
            );
        }
    }
    p.rect_stroke(
        r,
        1.0,
        Stroke::new(1.0, edge),
        egui::epaint::StrokeKind::Inside,
    );
}

/// A full-width row; returns its rectangle and its response.
fn row(ui: &mut Ui, selected: bool, layer: bool) -> (Rect, egui::Response) {
    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), ROW_H), Sense::click());
    let p = ui.painter();
    if selected {
        p.rect_filled(rect, 0.0, SELECTED);
        p.rect_stroke(
            rect,
            0.0,
            Stroke::new(1.0, SELECTED_EDGE),
            egui::epaint::StrokeKind::Inside,
        );
    } else if resp.hovered() {
        p.rect_filled(rect, 0.0, HOVER);
    } else if layer {
        p.rect_filled(rect, 0.0, LAYER_BG);
    }
    (rect, resp)
}

/// A clickable square of `size` at `x` in the row `rect`.
fn hot(ui: &mut Ui, rect: Rect, x: f32, size: f32, id: egui::Id) -> (Rect, egui::Response) {
    let r = Rect::from_min_size(
        Pos2::new(x, rect.center().y - size / 2.0),
        Vec2::splat(size),
    );
    (r, ui.interact(r, id, Sense::click()))
}

/// Draw the docker.
pub fn objects(app: &mut App, ui: &mut Ui) {
    let mut act = Actions::default();
    {
        let doc = app.doc();
        let Ok(page) = doc.page(app.page) else { return };
        ui.horizontal(|ui| {
            ui.strong(&page.name);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button(tr("docker.add_layer")).clicked() {
                    act.add_layer = true;
                }
            });
        });
        ui.add_space(2.0);
        ui.spacing_mut().item_spacing.y = 0.0;
        let nlayers = page.layers.len();
        for (li, layer) in page.layers.iter().enumerate().rev() {
            let open = !app.objects_collapsed.contains(&layer.id);
            let (rect, resp) = row(ui, false, true);
            let base = egui::Id::new(("objects_layer", layer.id.raw()));
            let mut x = rect.left() + 3.0;
            let (ar, a) = hot(ui, rect, x, 14.0, base.with("arrow"));
            paint_arrow(ui, ar, open);
            if a.clicked() {
                act.collapse = Some(layer.id);
            }
            x += 16.0;
            let (er, e) = hot(ui, rect, x, 18.0, base.with("eye"));
            paint_eye(ui, er, layer.visible);
            if e.on_hover_text(tr("docker.show_hide")).clicked() {
                act.toggles.push((layer.id, !layer.visible, layer.locked));
            }
            x += 20.0;
            let (lr, l) = hot(ui, rect, x, 18.0, base.with("lock"));
            paint_lock(ui, lr, layer.locked);
            if l.on_hover_text(tr("docker.lock_layer")).clicked() {
                act.toggles.push((layer.id, layer.visible, !layer.locked));
            }
            x += 22.0;
            paint_layer_icon(
                ui,
                Rect::from_min_size(Pos2::new(x, rect.center().y - 8.0), Vec2::splat(16.0)),
            );
            x += 22.0;
            ui.painter().text(
                Pos2::new(x, rect.center().y),
                egui::Align2::LEFT_CENTER,
                &layer.name,
                crate::theme::bold(13.0),
                Tokens::TEXT,
            );
            if resp.double_clicked() {
                act.rename_layer = Some((layer.id, layer.name.clone()));
            }
            resp.context_menu(|ui| {
                if ui.button(tr("docker.rename")).clicked() {
                    act.rename_layer = Some((layer.id, layer.name.clone()));
                    ui.close();
                }
                if ui
                    .add_enabled(li + 1 < nlayers, egui::Button::new(tr("docker.move_up")))
                    .clicked()
                {
                    act.reorder = Some((layer.id, li + 1));
                    ui.close();
                }
                if ui
                    .add_enabled(li > 0, egui::Button::new(tr("docker.move_down")))
                    .clicked()
                {
                    act.reorder = Some((layer.id, li - 1));
                    ui.close();
                }
                if ui
                    .add_enabled(nlayers > 1, egui::Button::new(tr("docker.delete_layer")))
                    .clicked()
                {
                    act.delete_layer = Some(layer.id);
                    ui.close();
                }
            });
            if open {
                for s in layer.shapes.iter().rev() {
                    object_rows(app, ui, s, 1, &mut act);
                }
            }
        }
    }
    apply(app, ui, act);
}

/// The row of `s` at nesting `depth`, then its children when it is an
/// open group.
fn object_rows(app: &App, ui: &mut Ui, s: &Shape, depth: usize, act: &mut Actions) {
    let selected = app.selection.contains(&s.id);
    let (rect, resp) = row(ui, selected, false);
    let mut x = rect.left() + 3.0 + depth as f32 * INDENT;
    let children: Option<&Vec<Shape>> = match &s.kind {
        ShapeKind::Group { children } => Some(children),
        _ => None,
    };
    let expanded = app.objects_open_groups.contains(&s.id);
    if children.is_some() {
        let (ar, a) = hot(
            ui,
            rect,
            x,
            14.0,
            egui::Id::new(("objects_group", s.id.raw())),
        );
        paint_arrow(ui, ar, expanded);
        if a.clicked() {
            act.expand_group = Some(s.id);
        }
    }
    x += 16.0;
    paint_swatch(
        ui,
        Rect::from_min_size(Pos2::new(x, rect.center().y - 6.0), Vec2::splat(12.0)),
        s,
    );
    x += 19.0;
    let label = object_label(s);
    let color = if s.visible {
        Tokens::TEXT
    } else {
        Tokens::TEXT_DIM
    };
    ui.painter().text(
        Pos2::new(x, rect.center().y),
        egui::Align2::LEFT_CENTER,
        &label,
        egui::FontId::proportional(13.0),
        color,
    );
    if resp.double_clicked() {
        act.rename_object = Some((s.id, s.name.clone().unwrap_or_else(|| label.clone())));
    } else if resp.clicked() {
        act.select = Some(s.id);
    }
    if let (Some(children), true) = (children, expanded) {
        for c in children.iter().rev() {
            object_rows(app, ui, c, depth + 1, act);
        }
    }
}

fn apply(app: &mut App, ui: &Ui, act: Actions) {
    for (layer, visible, locked) in act.toggles {
        app.run(Command::SetLayerVisible { layer, visible });
        app.run(Command::SetLayerLocked { layer, locked });
    }
    if let Some(layer) = act.collapse {
        if !app.objects_collapsed.remove(&layer) {
            app.objects_collapsed.insert(layer);
        }
    }
    if let Some(group) = act.expand_group {
        if !app.objects_open_groups.remove(&group) {
            app.objects_open_groups.insert(group);
        }
    }
    if let Some((layer, name)) = act.rename_layer {
        app.dialog = crate::ui::dialogs::Dialog::RenameLayer { layer, name };
    }
    if let Some((shape, name)) = act.rename_object {
        app.dialog = crate::ui::dialogs::Dialog::RenameObject { shape, name };
    }
    if let Some(layer) = act.delete_layer {
        app.selection.clear();
        app.run(Command::DeleteLayer { layer });
    }
    if let Some((layer, index)) = act.reorder {
        app.run(Command::ReorderLayer { layer, index });
    }
    if act.add_layer {
        let page = app.page;
        let n = app
            .doc()
            .page(page)
            .map(|p| p.layers.len() + 1)
            .unwrap_or(1);
        app.run(Command::AddLayer {
            page,
            name: trf("docker.layer_n", &[("n", &n.to_string())]),
        });
    }
    if let Some(id) = act.select {
        if ui.input(|i| i.modifiers.shift) {
            if !app.selection.contains(&id) {
                app.selection.push(id);
            }
        } else {
            app.select(vec![id]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn objects_show_their_names_or_what_they_are() {
        crate::i18n::set_language("en");
        let mut s = Shape::new(
            ShapeId(1),
            ShapeKind::Rect {
                rect: tracedraw_core::Rect::new(0.0, 0.0, 10.0, 10.0),
                radius: 0.0,
                corners: None,
            },
        );
        assert_eq!(object_label(&s), "Rectangle");
        s.name = Some("Water".into());
        assert_eq!(object_label(&s), "Water");
        s.name = Some("   ".into());
        assert_eq!(object_label(&s), "Rectangle");
    }

    #[test]
    fn the_docker_draws_layers_groups_and_selection() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        let id = app
            .new_shape(ShapeKind::Rect {
                rect: tracedraw_core::Rect::new(0.0, 0.0, 10.0, 10.0),
                radius: 0.0,
                corners: None,
            })
            .expect("shape");
        app.select(vec![id]);
        let layer = app.doc().pages[0].layers[0].id;
        for collapsed in [false, true] {
            if collapsed {
                app.objects_collapsed.insert(layer);
            }
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(400.0, 600.0),
                )),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| objects(&mut app, ui));
            out.textures_delta.clear();
        }
        assert_eq!(app.selection, vec![id]);
    }
}
