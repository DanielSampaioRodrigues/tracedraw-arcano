//! The Blend tool's property bar: Steps, Rotation, Object acceleration and
//! Color acceleration, for the next blend and, live, for the selected one;
//! Clear Blend and Break Apart for a selected blend.

use crate::app::App;
use crate::i18n::tr;
use crate::theme::Tokens;
use crate::ui::field::NumField;
use egui::Ui;
use tracedraw_core::live::Effect;

fn dim(ui: &mut Ui, text: String) {
    ui.label(egui::RichText::new(text).color(Tokens::TEXT_DIM).size(11.0));
}

/// The selected object's blend.
fn selected_blend(app: &App) -> Option<Effect> {
    app.selected_shapes().first().and_then(|s| {
        s.effects
            .iter()
            .find(|e| matches!(e, Effect::Blend { .. }))
            .cloned()
    })
}

/// The bar.
pub fn blend_bar(app: &mut App, ui: &mut Ui) {
    let cur = selected_blend(app);
    let (mut steps, mut rotation, mut accel_o, mut accel_c) = match &cur {
        Some(Effect::Blend {
            steps,
            rotation,
            accel_objects,
            accel_colors,
            ..
        }) => (*steps, *rotation, *accel_objects, *accel_colors),
        _ => (
            app.blend_steps,
            app.blend_rotation,
            app.blend_accel_objects,
            app.blend_accel_colors,
        ),
    };
    let mut changed = false;
    dim(ui, tr("docker.steps"));
    changed |= ui.add(NumField::new(&mut steps).range(1..=999)).changed();
    dim(ui, tr("docker.rotation"));
    changed |= ui
        .add(
            NumField::new(&mut rotation)
                .speed(1.0)
                .range(-3600.0..=3600.0)
                .suffix("\u{b0}"),
        )
        .changed();
    crate::ui::propbar::sep(ui);
    let pct = |ui: &mut Ui, key: &str, v: &mut f64| {
        dim(ui, tr(key));
        let mut p = (*v * 100.0).round() as i32;
        let r = ui.add(crate::ui::Rail(
            egui::Slider::new(&mut p, -100..=100).suffix(" %"),
        ));
        if r.changed() {
            *v = p as f64 / 100.0;
        }
        r.changed()
    };
    changed |= pct(ui, "docker.object_acceleration", &mut accel_o);
    changed |= pct(ui, "docker.color_acceleration", &mut accel_c);
    app.blend_steps = steps.clamp(1, 999);
    app.blend_rotation = rotation;
    app.blend_accel_objects = accel_o;
    app.blend_accel_colors = accel_c;
    crate::ui::propbar::sep(ui);
    match cur {
        Some(Effect::Blend {
            end,
            path,
            rotate_on_path,
            ..
        }) => {
            if changed {
                app.push_effect(
                    Effect::Blend {
                        end,
                        steps: steps.clamp(1, 999),
                        accel_objects: accel_o,
                        accel_colors: accel_c,
                        rotation,
                        path,
                        rotate_on_path,
                    },
                    true,
                );
            }
            if ui.button(tr("docker.clear_blend")).clicked() {
                app.clear_blend();
            }
            if ui.button(tr("docker.break_apart")).clicked() {
                app.flatten_effects();
            }
        }
        _ => dim(ui, tr("toolbar.drag_between_objects")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::{Rect, ShapeKind};

    /// The bar draws with and without a blend; a blend's settings change
    /// live as one undo step for a run of changes.
    #[test]
    fn the_blend_bar_edits_the_selected_blend() {
        let mut app = App::headless();
        let a = app
            .new_shape(ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, 10.0, 10.0),
                radius: 0.0,
                corners: None,
            })
            .expect("a");
        let b = app
            .new_shape(ShapeKind::Rect {
                rect: Rect::new(50.0, 0.0, 60.0, 10.0),
                radius: 0.0,
                corners: None,
            })
            .expect("b");
        let ctx = crate::theme::ui_context();
        let draw = |app: &mut App| {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1400.0, 80.0),
                )),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| {
                ui.horizontal(|ui| blend_bar(app, ui));
            });
            out.textures_delta.clear();
        };
        app.tool = crate::tools::Tool::Blend;
        draw(&mut app);
        app.select(vec![a, b]);
        app.blend_selection(5, 0.0, 0.0, 0.0);
        app.select(vec![a]);
        draw(&mut app);
        let depth = app.engine.history_labels().0.len();
        // Three changes in a row from the bar: one step.
        for steps in [6, 7, 8] {
            let Some(Effect::Blend {
                end,
                path,
                rotate_on_path,
                ..
            }) = selected_blend(&app)
            else {
                panic!("no blend");
            };
            app.push_effect(
                Effect::Blend {
                    end,
                    steps,
                    accel_objects: 0.0,
                    accel_colors: 0.0,
                    rotation: 0.0,
                    path,
                    rotate_on_path,
                },
                true,
            );
        }
        assert_eq!(app.engine.history_labels().0.len(), depth + 1);
        assert!(matches!(
            selected_blend(&app),
            Some(Effect::Blend { steps: 8, .. })
        ));
        app.clear_blend();
        assert!(selected_blend(&app).is_none());
    }
}
