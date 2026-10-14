//! The Extrude tool's property bar, the controls it shares with the
//! Extrude panel, and the handles of the selected extrusions. Controls:
//! the kind of body; the vector (parallel bodies) or the depth, the
//! vanishing point and what it is locked to; the colour (the object's
//! fill, solid, or a shade between two colours); the light's angle,
//! intensity and elevation, two further lights, the turn in space and the
//! bevel. They show the selected extrusion and edit the selected
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
    ui.label(
        egui::RichText::new(text)
            .color(Tokens::text_dim())
            .size(11.0),
    );
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

/// The turn in space, the first light's elevation, the further lights and
/// the bevel (the panel shows them; the bar in a menu).
pub fn extrude_space(app: &mut App, ui: &mut Ui) {
    app.sync_extrude_settings();
    let mut s = app.extrude;
    let mut changed = false;
    ui.horizontal(|ui| {
        dim(ui, tr("extrude.elevation"));
        changed |= degrees(
            ui,
            &mut s.elevation,
            0.0..=90.0,
            &tr("extrude.elevation_tip"),
        );
    });
    ui.horizontal(|ui| {
        dim(ui, tr("extrude.rotation"));
        for (i, axis) in ["x", "y", "z"].into_iter().enumerate() {
            changed |= degrees(
                ui,
                &mut s.rotation[i],
                -360.0..=360.0,
                &format!("{} ({axis})", tr("extrude.rotation_tip")),
            );
        }
        if ui
            .add_enabled(
                s.rotation != [0.0; 3],
                egui::Button::new(tr("docker.reset")),
            )
            .clicked()
        {
            s.rotation = [0.0; 3];
            changed = true;
        }
    });
    for i in 0..s.lights.len() {
        ui.horizontal(|ui| {
            let (on, light) = &mut s.lights[i];
            changed |= ui
                .checkbox(
                    on,
                    crate::i18n::trf("extrude.light_n", &[("n", &(i + 2).to_string())]),
                )
                .changed();
            ui.add_enabled_ui(*on, |ui| {
                changed |= degrees(ui, &mut light.angle, 0.0..=360.0, &tr("docker.light_angle"));
                changed |= degrees(
                    ui,
                    &mut light.elevation,
                    0.0..=90.0,
                    &tr("extrude.elevation"),
                );
                changed |= ui
                    .add(
                        NumField::new(&mut light.intensity)
                            .range(0.0..=100.0)
                            .max_decimals(0)
                            .width(40.0),
                    )
                    .on_hover_text(tr("docker.light_intensity"))
                    .changed();
            });
        });
    }
    ui.horizontal(|ui| {
        dim(ui, tr("docker.bevel"));
        changed |= length(ui, app, &mut s.bevel, &tr("extrude.bevel_tip"));
    });
    if changed {
        app.extrude = s;
        app.apply_extrude_settings();
    }
}

/// An angle field in degrees; true when it changed.
fn degrees(ui: &mut Ui, v: &mut f64, range: std::ops::RangeInclusive<f64>, tip: &str) -> bool {
    ui.add(
        NumField::new(v)
            .range(range)
            .max_decimals(1)
            .suffix("\u{b0}")
            .width(52.0),
    )
    .on_hover_text(tip)
    .changed()
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
    ui.menu_button(tr("extrude.space_menu"), |ui| {
        ui.vertical(|ui| extrude_space(app, ui));
    })
    .response
    .on_hover_text(tr("extrude.space_menu_tip"));
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
    use tracedraw_core::{geometry::Rect, live::Effect, ShapeKind};

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
                        extrude_space(app, ui);
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
        // The turn, the lights and the bevel go to the effect and come
        // back from it.
        app.extrude.rotation = [10.0, 20.0, 30.0];
        app.extrude.elevation = 40.0;
        app.extrude.lights[1] = (true, crate::extrude_tool::NEW_LIGHT);
        app.extrude.bevel = 1.5;
        app.apply_extrude_settings();
        frame(&mut app, true);
        frame(&mut app, false);
        let Some(Effect::Extrude { space, bevel, .. }) = app
            .selected_shapes()
            .first()
            .and_then(|s| {
                s.effects
                    .iter()
                    .find(|e| matches!(e, Effect::Extrude { .. }))
            })
            .cloned()
        else {
            panic!("no extrusion");
        };
        assert_eq!(space.rotation, [10.0, 20.0, 30.0]);
        assert_eq!(space.lights.len(), 1);
        assert!((bevel - 1.5).abs() < 1e-9);
        let mut back = crate::extrude_tool::ExtrudeSettings::default();
        back.take(
            &Effect::Extrude {
                depth: Default::default(),
                vanishing: None,
                amount: 0.0,
                shade_from: None,
                shade_to: None,
                light_angle: 0.0,
                light_intensity: 0.0,
                bevel,
                kind: None,
                on_object: false,
                solid: None,
                space,
            },
            Default::default(),
            Default::default(),
        );
        assert!(back.lights[0].0);
        assert!(!back.lights[1].0);
        assert_eq!(back.elevation, 40.0);
    }
}
