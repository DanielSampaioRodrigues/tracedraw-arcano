//! The Extrude tool's property bar, the controls it shares with the
//! Extrude panel, and the handles of the selected extrusions. Controls:
//! the kind of body; the vector (parallel bodies) or the depth, the
//! vanishing point and what it is locked to; the colour (the object's
//! fill, solid, or a shade between two colours); the light's angle and
//! intensity. They show the selected extrusion and edit the selected
//! extrusions live.

use crate::app::App;
use crate::extrude_tool::{kind_key, ExtrudeColor};
use crate::i18n::tr;
use crate::theme::Tokens;
use crate::ui::field::NumField;
use crate::ui::fill_tool_bars::color_pick;
use crate::ui::propbar::sep;
use egui::{Color32, Painter, Stroke, Ui};
use tracedraw_core::extrude::ExtrudeKind;

fn dim(ui: &mut Ui, text: String) {
    ui.label(egui::RichText::new(text).color(Tokens::TEXT_DIM).size(11.0));
}

/// A length in the document's units; true when it was applied.
fn length(ui: &mut Ui, app: &App, mm: &mut f64, tip: &str) -> bool {
    let mut v = app.units.world_from_mm(*mm);
    let r = ui
        .add(
            NumField::new(&mut v)
                .fixed_decimals(app.settings.precision.min(10) as usize)
                .unit_mm(app.units.world_to_mm(1.0))
                .suffix(format!(" {}", app.units.short()))
                .width(76.0),
        )
        .on_hover_text(tip);
    if r.changed() {
        *mm = app.units.world_to_mm(v);
        return true;
    }
    false
}

/// The controls, each group in a row of its own (a bar lays the rows
/// side by side, the panel one under the other).
pub fn extrude_controls(app: &mut App, ui: &mut Ui) {
    app.sync_extrude_settings();
    let palette = app.palette.clone();
    let mut s = app.extrude;
    let mut changed = false;
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt("extrude_kind")
            .selected_text(tr(kind_key(s.kind)))
            .width(110.0)
            .show_ui(ui, |ui| {
                for k in ExtrudeKind::ALL {
                    if ui.selectable_label(s.kind == k, tr(kind_key(k))).clicked() && s.kind != k {
                        s.kind = k;
                        changed = true;
                    }
                }
            })
            .response
            .on_hover_text(tr("extrude.kind"));
    });
    ui.horizontal(|ui| {
        if s.kind.parallel() {
            dim(ui, tr("extrude.vector"));
            changed |= length(ui, app, &mut s.vector.x, "x");
            changed |= length(ui, app, &mut s.vector.y, "y");
        } else {
            dim(ui, tr("docker.depth"));
            changed |= ui
                .add(
                    NumField::new(&mut s.depth)
                        .range(1.0..=99.0)
                        .max_decimals(0)
                        .width(44.0),
                )
                .on_hover_text(tr("extrude.depth_tip"))
                .changed();
            dim(ui, tr("docker.vanishing_point"));
            changed |= length(ui, app, &mut s.vanishing.x, "x");
            changed |= length(ui, app, &mut s.vanishing.y, "y");
            egui::ComboBox::from_id_salt("extrude_locked")
                .selected_text(tr(if s.on_object {
                    "extrude.locked_object"
                } else {
                    "extrude.locked_page"
                }))
                .width(110.0)
                .show_ui(ui, |ui| {
                    for (on, key) in [
                        (true, "extrude.locked_object"),
                        (false, "extrude.locked_page"),
                    ] {
                        if ui.selectable_label(s.on_object == on, tr(key)).clicked()
                            && s.on_object != on
                        {
                            s.on_object = on;
                            changed = true;
                        }
                    }
                });
        }
    });
    ui.horizontal(|ui| {
        dim(ui, tr("extrude.color"));
        egui::ComboBox::from_id_salt("extrude_color")
            .selected_text(tr(s.color.key()))
            .width(96.0)
            .show_ui(ui, |ui| {
                for c in ExtrudeColor::ALL {
                    if ui.selectable_label(s.color == c, tr(c.key())).clicked() && s.color != c {
                        s.color = c;
                        changed = true;
                    }
                }
            });
        match s.color {
            ExtrudeColor::Object => {}
            ExtrudeColor::Solid => {
                changed |= color_pick(
                    ui,
                    &palette,
                    "extrude_solid",
                    &mut s.solid,
                    &tr("extrude.color_solid"),
                    true,
                );
            }
            ExtrudeColor::Shade => {
                changed |= color_pick(
                    ui,
                    &palette,
                    "extrude_from",
                    &mut s.shade_from,
                    &tr("docker.from"),
                    true,
                );
                changed |= color_pick(
                    ui,
                    &palette,
                    "extrude_to",
                    &mut s.shade_to,
                    &tr("docker.to"),
                    true,
                );
            }
        }
    });
    ui.horizontal(|ui| {
        dim(ui, tr("docker.light_angle"));
        changed |= ui
            .add(
                NumField::new(&mut s.light_angle)
                    .range(0.0..=360.0)
                    .max_decimals(1)
                    .suffix("\u{b0}")
                    .width(52.0),
            )
            .changed();
        dim(ui, tr("docker.light_intensity"));
        changed |= ui
            .add(
                NumField::new(&mut s.light_intensity)
                    .range(0.0..=100.0)
                    .max_decimals(0)
                    .width(40.0),
            )
            .changed();
    });
    if changed {
        app.extrude = s;
        app.apply_extrude_settings();
    }
}

/// Apply (for selected objects with no extrusion), Clear Extrude and
/// Break Apart.
pub fn extrude_actions(app: &mut App, ui: &mut Ui) {
    let extruded = app.selected_extrusion().is_some();
    let bare = app.selected_shapes().iter().any(|s| {
        !s.effects
            .iter()
            .any(|e| matches!(e, tracedraw_core::live::Effect::Extrude { .. }))
            && !matches!(
                s.kind,
                tracedraw_core::ShapeKind::Bitmap { .. } | tracedraw_core::ShapeKind::Group { .. }
            )
    });
    ui.horizontal(|ui| {
        if bare && ui.button(tr("docker.apply")).clicked() {
            app.add_extrusion();
        }
        if ui
            .add_enabled(extruded, egui::Button::new(tr("extrude.clear")))
            .clicked()
        {
            app.clear_extrusion();
        }
        if ui
            .add_enabled(extruded, egui::Button::new(tr("docker.break_apart")))
            .clicked()
        {
            app.flatten_effects();
        }
    });
}

/// The bar.
pub fn extrude_bar(app: &mut App, ui: &mut Ui) {
    extrude_controls(app, ui);
    sep(ui);
    extrude_actions(app, ui);
    sep(ui);
    dim(ui, tr("extrude.hint"));
}

/// Each selected extrusion's handle: a line from the object's middle to
/// its vanishing point or its copy's middle, and a cross there.
pub fn draw_handles(app: &App, painter: &Painter) {
    if app.tool != crate::tools::Tool::Extrude {
        return;
    }
    let blue = Color32::from_rgb(40, 110, 220);
    for (_, h, from) in app.extrusion_handles() {
        let (h, from) = (app.view.to_screen(h), app.view.to_screen(from));
        painter.add(egui::Shape::dashed_line(
            &[from, h],
            Stroke::new(1.0, blue),
            4.0,
            3.0,
        ));
        let r = 5.0;
        painter.rect_filled(
            egui::Rect::from_center_size(h, egui::vec2(2.0 * r, 2.0 * r)),
            0.0,
            Color32::WHITE,
        );
        painter.rect_stroke(
            egui::Rect::from_center_size(h, egui::vec2(2.0 * r, 2.0 * r)),
            0.0,
            Stroke::new(1.0, blue),
            egui::StrokeKind::Middle,
        );
        painter.line_segment(
            [h + egui::vec2(-r, -r), h + egui::vec2(r, r)],
            Stroke::new(1.0, blue),
        );
        painter.line_segment(
            [h + egui::vec2(-r, r), h + egui::vec2(r, -r)],
            Stroke::new(1.0, blue),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::{geometry::Rect, ShapeKind};

    #[test]
    fn the_bar_and_the_panel_draw_for_every_kind() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        app.tool = crate::tools::Tool::Extrude;
        let id = app
            .new_shape(ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, 20.0, 20.0),
                radius: 0.0,
                corners: None,
            })
            .expect("square");
        let frame = |app: &mut App, panel: bool| {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1600.0, 400.0),
                )),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| {
                if panel {
                    ui.vertical(|ui| {
                        extrude_controls(app, ui);
                        extrude_actions(app, ui);
                    });
                } else {
                    ui.horizontal(|ui| extrude_bar(app, ui));
                }
                draw_handles(app, ui.painter());
            });
            out.textures_delta.clear();
        };
        for k in ExtrudeKind::ALL {
            for c in ExtrudeColor::ALL {
                app.extrude.kind = k;
                app.extrude.color = c;
                frame(&mut app, false);
            }
        }
        // A selected extrusion shows as the controls' values.
        app.select(vec![id]);
        app.extrude.kind = ExtrudeKind::BigBack;
        app.add_extrusion();
        app.extrude.kind = ExtrudeKind::FrontParallel;
        frame(&mut app, true);
        assert_eq!(app.extrude.kind, ExtrudeKind::BigBack);
        assert_eq!(app.extrusion_handles().len(), 1);
    }
}
