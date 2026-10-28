//! The studio's parts of the interface: the picture service, model,
//! size, shape and choices in the AI Generate docker; its Edit page
//! (operations on the selected bitmap); and the Keys page of Tools >
//! Options > AI, one key per service.

use egui::{RichText, Ui};

use super::app::{model_param_key, op_param_key, KeyTest};
use super::catalog::{text, ModelInfo, Param};
use super::Op;
use crate::app::App;
use crate::i18n::{tr, trf};
use crate::theme::Tokens;

const ERROR: egui::Color32 = egui::Color32::from_rgb(0xC4, 0x2B, 0x1C);
const OK: egui::Color32 = egui::Color32::from_rgb(0x2E, 0x8B, 0x3E);

fn dim(ui: &mut Ui, s: impl Into<String>) {
    ui.add(egui::Label::new(RichText::new(s.into()).color(Tokens::text_dim()).size(11.0)).wrap());
}

/// The text services that are not picture services, for the Keys page.
const TEXT_ONLY: [(&str, &str, &str); 3] = [
    ("anthropic", "Anthropic (Claude)", "https://platform.claude.com/settings/keys"),
    ("maritaca", "Maritaca AI (Sabiá)", "https://plataforma.maritaca.ai/"),
    ("deepseek", "DeepSeek", "https://platform.deepseek.com/api_keys"),
];

/// The service row of the docker's Create, Remix and Create vectors
/// pages, and when a studio service is chosen, its model, size, shape
/// and choices. True when the studio draws the rows (the docker then
/// leaves out its own model and shape rows).
pub fn picture_rows(app: &mut App, ui: &mut Ui, op: Op) -> bool {
    let services: Vec<(String, String)> = app
        .studio
        .catalog
        .services_for(op)
        .iter()
        .map(|p| (p.id.clone(), p.name.clone()))
        .collect();
    let current = app.settings.studio.image_provider.clone();
    let shown = services
        .iter()
        .find(|(id, _)| *id == current)
        .map(|(_, n)| n.clone())
        .unwrap_or_else(|| tr("studio.main_service"));
    ui.horizontal(|ui| {
        ui.label(tr("studio.service"));
        egui::ComboBox::from_id_salt("studio_service")
            .width((ui.available_width() - 8.0).max(80.0))
            .selected_text(shown)
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(current.is_empty(), tr("studio.main_service"))
                    .on_hover_text(tr("studio.main_service_tip"))
                    .clicked()
                {
                    app.settings.studio.image_provider.clear();
                }
                for (id, name) in &services {
                    let has_key = !app.studio_key(id).trim().is_empty();
                    let label = if has_key {
                        name.clone()
                    } else {
                        trf("studio.no_key_suffix", &[("name", name)])
                    };
                    if ui.selectable_label(*id == current, label).clicked() {
                        app.settings.studio.image_provider = id.clone();
                    }
                }
            });
    });
    if !app.studio_active() {
        return false;
    }
    let provider = app.settings.studio.image_provider.clone();
    if app.studio_key(&provider).trim().is_empty() {
        ui.horizontal(|ui| {
            ui.label(RichText::new(tr("studio.key_missing")).color(ERROR));
            if ui.small_button(tr("studio.set_keys")).clicked() {
                crate::ui::ai_ui::open_options(app, crate::ui::dialogs::OptionsPage::AiKeys);
            }
        });
    }
    let Some(p) = app.studio.catalog.provider(&provider).cloned() else {
        return false;
    };
    let models: Vec<ModelInfo> = p.models_for(op).into_iter().cloned().collect();
    let Some((_, model)) = app.studio_model(op) else {
        dim(ui, tr("studio.no_model"));
        return true;
    };
    // The model, each with what it is good at and its price.
    let res_now = model.fit_resolution(&app.settings.studio.resolution);
    ui.horizontal(|ui| {
        ui.label(tr("ai.model"));
        egui::ComboBox::from_id_salt("studio_model")
            .width((ui.available_width() - 8.0).max(80.0))
            .selected_text(model.label.clone())
            .show_ui(ui, |ui| {
                ui.set_min_width(270.0);
                ui.set_max_width(300.0);
                for m in &models {
                    let on = m.id == model.id;
                    let r = ui
                        .vertical(|ui| {
                            let r = ui.radio(on, RichText::new(&m.label).strong());
                            dim(ui, text(&m.note));
                            if let Some(price) = m.price_caption(&m.fit_resolution(&res_now)) {
                                dim(ui, price);
                            }
                            r
                        })
                        .inner;
                    if r.clicked() {
                        app.settings.studio.models.insert(provider.clone(), m.id.clone());
                        ui.close();
                    }
                    ui.add_space(3.0);
                }
            });
    });
    // Size and shape, as the model makes them.
    ui.horizontal(|ui| {
        ui.label(tr("aiimg.resolution"));
        ui.add_enabled_ui(model.resolutions.len() > 1, |ui| {
            egui::ComboBox::from_id_salt("studio_resolution")
                .selected_text(resolution_label(&res_now))
                .show_ui(ui, |ui| {
                    for r in &model.resolutions {
                        if ui.selectable_label(*r == res_now, resolution_label(r)).clicked() {
                            app.settings.studio.resolution = r.clone();
                        }
                    }
                });
        });
    });
    let aspect_now = model.fit_aspect(&app.settings.studio.aspect);
    ui.horizontal(|ui| {
        ui.label(tr("aigen.aspect"));
        egui::ComboBox::from_id_salt("studio_aspect")
            .selected_text(aspect_now.clone())
            .show_ui(ui, |ui| {
                if app.ai_image.source.is_some() {
                    let on = app.ai_image.aspect == crate::ai_image::Aspect::MatchSource;
                    if ui.selectable_label(on, tr("aiimg.aspect_match")).clicked() {
                        app.ai_image.aspect = crate::ai_image::Aspect::MatchSource;
                    }
                }
                for a in &model.aspects {
                    if ui.selectable_label(*a == aspect_now, a).clicked() {
                        app.settings.studio.aspect = a.clone();
                        app.ai_image.aspect = crate::ai_image::Aspect::Square;
                    }
                }
            });
    });
    for prm in &model.params {
        // How much a remix changes means nothing to a new picture.
        if op == Op::Generate && matches!(prm.name.as_str(), "strength" | "structure") {
            continue;
        }
        param_ui(app, ui, &model_param_key(&provider, &model.id, &prm.name), prm);
    }
    if let Some(price) = model.price_caption(&res_now) {
        dim(ui, price);
    }
    if p.english_only {
        dim(ui, tr("studio.translated_note"));
    }
    true
}

fn resolution_label(r: &str) -> String {
    match r {
        "1K" => tr("aiimg.res_1k"),
        "2K" => tr("aiimg.res_2k"),
        "4K" => tr("aiimg.res_4k"),
        other => other.to_string(),
    }
}

/// An editor for one choice, kept in `Studio::params` under `key`.
fn param_ui(app: &mut App, ui: &mut Ui, key: &str, p: &Param) {
    let current = app.studio.params.get(key).cloned().unwrap_or_else(|| p.default.clone());
    ui.horizontal(|ui| {
        ui.label(text(&p.label));
        match p.kind.as_str() {
            "choice" => {
                let shown = p
                    .choices
                    .iter()
                    .find(|(v, _)| *v == current)
                    .map(|(_, l)| text(l))
                    .unwrap_or_else(|| current.to_string());
                egui::ComboBox::from_id_salt(key)
                    .selected_text(shown)
                    .show_ui(ui, |ui| {
                        for (v, l) in &p.choices {
                            if ui.selectable_label(*v == current, text(l)).clicked() {
                                app.studio.params.insert(key.to_string(), v.clone());
                            }
                        }
                    });
            }
            "number" => {
                let lo = p.min.unwrap_or(0.0);
                let hi = p.max.unwrap_or(100.0);
                let whole = lo.fract() == 0.0 && hi.fract() == 0.0 && hi - lo > 1.0
                    && p.default.as_f64().is_some_and(|d| d.fract() == 0.0);
                let mut v = current.as_f64().unwrap_or(lo);
                let speed = if whole { 1.0 } else { (hi - lo) / 100.0 };
                let mut dv = egui::DragValue::new(&mut v).range(lo..=hi).speed(speed);
                if !whole {
                    dv = dv.max_decimals(2);
                }
                if ui.add(dv).changed() {
                    let value = if whole {
                        serde_json::json!(v.round() as i64)
                    } else {
                        serde_json::json!(v)
                    };
                    app.studio.params.insert(key.to_string(), value);
                }
            }
            _ => {
                let mut s = current.as_str().unwrap_or_default().to_string();
                if ui.text_edit_singleline(&mut s).changed() {
                    app.studio.params.insert(key.to_string(), serde_json::json!(s));
                }
            }
        }
    });
}

/// The AI Generate docker's Edit page: what to do to the selected bitmap,
/// with which service, how, and what it costs.
pub fn edit_page(app: &mut App, ui: &mut Ui) {
    ui.label(RichText::new(tr("studio.edit_title")).strong());
    dim(ui, tr("studio.edit_about"));
    if !app.ai_features_on() {
        ui.label(RichText::new(tr("aiimg.off")).color(ERROR));
        return;
    }
    let busy = app.studio.busy();
    egui::ScrollArea::vertical()
        .id_salt("studio_edit")
        .auto_shrink([false, false])
        .max_height((ui.available_height() - 70.0).max(120.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width() - 10.0);
            ui.add_enabled_ui(!busy, |ui| edit_body(app, ui));
        });
    ui.separator();
    // The status and the button.
    match &app.ai_image.status {
        crate::ai_image::ImageStatus::Waiting { .. } => {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(tr("studio.working"));
                if ui.button(tr("dialog.cancel")).clicked() {
                    app.studio_cancel();
                }
            });
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(200));
        }
        crate::ai_image::ImageStatus::Failed(e) => {
            ui.add(egui::Label::new(RichText::new(e.clone()).color(ERROR)).wrap());
        }
        crate::ai_image::ImageStatus::Done(_) => {
            ui.label(RichText::new(tr("studio.done")).color(OK));
        }
        _ => {}
    }
    let can = app.ai_source_bitmap().is_some() && !busy;
    ui.add_enabled_ui(can, |ui| {
        let label = format!("\u{2728} {}", tr(app.studio.op.key()));
        if ui
            .add_sized([ui.available_width(), 28.0], egui::Button::new(RichText::new(label).strong()))
            .clicked()
        {
            app.studio_edit();
        }
    });
}

fn edit_body(app: &mut App, ui: &mut Ui) {
    // The picture worked on.
    match app.ai_source_bitmap() {
        Some((id, _)) => {
            if let Some(shape) = app.doc().find_shape(id).cloned() {
                if let tracedraw_core::document::ShapeKind::Bitmap { rect, width_px, height_px, .. } = &shape.kind {
                    let dpi = if rect.width() > 0.0 { *width_px as f64 / (rect.width() / 25.4) } else { 0.0 };
                    ui.label(trf(
                        "studio.selected_bitmap",
                        &[
                            ("w", &width_px.to_string()),
                            ("h", &height_px.to_string()),
                            ("dpi", &format!("{dpi:.0}")),
                        ],
                    ));
                }
            }
        }
        None => {
            ui.label(RichText::new(tr("aiimg.need_bitmap")).color(ERROR));
        }
    }
    ui.add_space(4.0);
    // The operations.
    ui.horizontal_wrapped(|ui| {
        for op in Op::EDITS {
            if app.studio.catalog.services_for(op).is_empty() {
                continue;
            }
            if ui.selectable_label(app.studio.op == op, tr(op.key())).clicked() {
                app.studio.op = op;
            }
        }
    });
    let op = app.studio.op;
    dim(ui, tr(op.about_key()));
    ui.add_space(4.0);
    // The service.
    let services: Vec<(String, String)> = app
        .studio
        .catalog
        .services_for(op)
        .iter()
        .map(|p| (p.id.clone(), p.name.clone()))
        .collect();
    let Some(provider) = app.studio_op_service(op) else {
        dim(ui, tr("studio.no_service"));
        return;
    };
    let shown = services
        .iter()
        .find(|(id, _)| *id == provider)
        .map(|(_, n)| n.clone())
        .unwrap_or_default();
    ui.horizontal(|ui| {
        ui.label(tr("studio.service"));
        egui::ComboBox::from_id_salt("studio_op_service")
            .width((ui.available_width() - 8.0).max(80.0))
            .selected_text(shown)
            .show_ui(ui, |ui| {
                for (id, name) in &services {
                    let has_key = !app.studio_key(id).trim().is_empty();
                    let label = if has_key { name.clone() } else { trf("studio.no_key_suffix", &[("name", name)]) };
                    if ui.selectable_label(*id == provider, label).clicked() {
                        app.settings.studio.op_providers.insert(op.id().to_string(), id.clone());
                    }
                }
            });
    });
    if app.studio_key(&provider).trim().is_empty() {
        ui.horizontal(|ui| {
            ui.label(RichText::new(tr("studio.key_missing")).color(ERROR));
            if ui.small_button(tr("studio.set_keys")).clicked() {
                crate::ui::ai_ui::open_options(app, crate::ui::dialogs::OptionsPage::AiKeys);
            }
        });
    }
    // How: the variants of the operation at this service.
    let Some(info) = app.studio.catalog.provider(&provider).and_then(|p| p.op(op)).cloned() else {
        return;
    };
    let Some(variant) = app.studio_variant(&provider, op) else {
        return;
    };
    if info.variants.len() > 1 {
        ui.horizontal(|ui| {
            ui.label(tr("studio.how"));
            egui::ComboBox::from_id_salt("studio_variant")
                .width((ui.available_width() - 8.0).max(80.0))
                .selected_text(text(&variant.label))
                .show_ui(ui, |ui| {
                    ui.set_min_width(260.0);
                    for v in &info.variants {
                        let r = ui
                            .vertical(|ui| {
                                let r = ui.radio(v.id == variant.id, RichText::new(text(&v.label)).strong());
                                dim(ui, text(&v.note));
                                if let Some(p) = v.price_caption() {
                                    dim(ui, p);
                                }
                                r
                            })
                            .inner;
                        if r.clicked() {
                            app.settings
                                .studio
                                .variants
                                .insert(format!("{provider}/{}", op.id()), v.id.clone());
                            ui.close();
                        }
                    }
                });
        });
    }
    dim(ui, text(&variant.note));
    // Expand: a quick way to a shape.
    if op == Op::Expand {
        expand_to_shape(app, ui, &provider, &variant.id);
    }
    for p in &variant.params {
        param_ui(app, ui, &op_param_key(&provider, op, &variant.id, &p.name), p);
    }
    // Words.
    if op.takes_prompt() || op == Op::Expand {
        let hint = if op.takes_prompt() { tr("studio.prompt_hint") } else { tr("studio.prompt_optional") };
        ui.add(
            egui::TextEdit::multiline(&mut app.studio.prompt)
                .hint_text(hint)
                .desired_rows(3)
                .desired_width(f32::INFINITY),
        );
    }
    if let Some(p) = variant.price_caption() {
        dim(ui, p);
    }
    if app
        .studio
        .catalog
        .provider(&provider)
        .is_some_and(|p| p.english_only)
    {
        dim(ui, tr("studio.translated_note"));
    }
}

/// Fill the Expand sides so the picture reaches a shape, growing evenly
/// on the two sides that need it.
fn expand_to_shape(app: &mut App, ui: &mut Ui, provider: &str, variant: &str) {
    let Some((w, h)) = app
        .ai_source_bitmap()
        .and_then(|(_, png)| super::image_size(&png))
    else {
        return;
    };
    ui.horizontal(|ui| {
        ui.label(tr("studio.expand_to"));
        egui::ComboBox::from_id_salt("studio_expand_to")
            .selected_text(tr("studio.expand_pick"))
            .show_ui(ui, |ui| {
                for a in ["1:1", "4:5", "3:4", "2:3", "9:16", "5:4", "4:3", "3:2", "16:9", "21:9"] {
                    if ui.selectable_label(false, a).clicked() {
                        let (l, r, t, b) = pads_to(w, h, a);
                        for (name, v) in [("left", l), ("right", r), ("top", t), ("bottom", b)] {
                            app.studio.params.insert(
                                op_param_key(provider, Op::Expand, variant, name),
                                serde_json::json!(v),
                            );
                        }
                    }
                }
            });
    });
}

/// The pixels to add on each side (left, right, top, bottom) for a
/// picture of `w` by `h` to reach `aspect`.
pub fn pads_to(w: u32, h: u32, aspect: &str) -> (u32, u32, u32, u32) {
    let mut it = aspect.split(':').map(|x| x.parse::<f64>().unwrap_or(1.0));
    let r = it.next().unwrap_or(1.0) / it.next().unwrap_or(1.0).max(0.01);
    let (w, h) = (w as f64, h as f64);
    if w / h < r {
        let add = (h * r - w).max(0.0);
        let l = (add / 2.0).floor() as u32;
        (l, add.ceil() as u32 - l, 0, 0)
    } else {
        let add = (w / r - h).max(0.0);
        let t = (add / 2.0).floor() as u32;
        (0, 0, t, add.ceil() as u32 - t)
    }
}

/// Tools > Options > AI > Keys: a key for each service.
pub fn keys_page(app: &mut App, ui: &mut Ui) {
    ui.label(RichText::new(tr("studio.keys_title")).strong());
    dim(ui, tr("studio.keys_about"));
    ui.add_space(6.0);
    let mut rows: Vec<(String, String, String, bool)> = app
        .studio
        .catalog
        .providers
        .iter()
        .map(|p| (p.id.clone(), p.name.clone(), p.key_url.clone(), true))
        .collect();
    for (id, name, url) in TEXT_ONLY {
        rows.push((id.into(), name.into(), url.into(), false));
    }
    egui::Grid::new("studio_keys")
        .num_columns(4)
        .spacing([8.0, 6.0])
        .show(ui, |ui| {
            for (id, name, url, testable) in rows {
                ui.label(RichText::new(&name).strong());
                let mut key = app.studio.keys.get(&id).cloned().unwrap_or_default();
                if ui
                    .add_sized(
                        [240.0, 20.0],
                        egui::TextEdit::singleline(&mut key)
                            .password(true)
                            .hint_text(tr("studio.key_hint")),
                    )
                    .on_hover_text(trf("studio.key_field_tip", &[("name", &name)]))
                    .changed()
                {
                    app.studio_set_key(&id, key.clone());
                }
                ui.horizontal(|ui| {
                    if ui.small_button(tr("studio.get_key")).on_hover_text(url.clone()).clicked() {
                        app.open_url(&url);
                    }
                    if testable
                        && ui
                            .add_enabled(!key.trim().is_empty(), egui::Button::new(tr("studio.test")).small())
                            .on_hover_text(tr("studio.test_tip"))
                            .clicked()
                    {
                        app.studio_test_key(&id);
                    }
                });
                match app.studio.tests.get(&id) {
                    Some(KeyTest::Testing) => {
                        ui.spinner();
                        ui.ctx().request_repaint_after(std::time::Duration::from_millis(200));
                    }
                    Some(KeyTest::Works(note)) => {
                        ui.label(RichText::new(format!("\u{2714} {note}")).color(OK));
                    }
                    Some(KeyTest::Fails(e)) => {
                        ui.add(egui::Label::new(RichText::new(e.clone()).color(ERROR)).truncate())
                            .on_hover_text(e.clone());
                    }
                    None => {
                        ui.label("");
                    }
                }
                ui.end_row();
            }
        });
    ui.add_space(6.0);
    let mut remember = app.settings.studio.remember_keys;
    if ui
        .checkbox(&mut remember, tr("studio.remember_keys"))
        .on_hover_text(tr("studio.remember_keys_tip"))
        .changed()
    {
        app.studio_remember_keys(remember);
    }
    dim(ui, tr("studio.keys_privacy"));
    dim(ui, trf("studio.catalog_line", &[("date", &app.studio.catalog.checked)]));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expanding_to_a_shape_grows_the_two_sides_that_need_it() {
        assert_eq!(pads_to(1000, 1000, "16:9"), (388, 390, 0, 0));
        let (l, r, t, b) = pads_to(1600, 900, "1:1");
        assert_eq!((l, r, t + b), (0, 0, 700));
        assert_eq!(pads_to(900, 1600, "9:16"), (0, 0, 0, 0));
    }

    #[test]
    fn the_pages_draw_in_every_state() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        let run = |app: &mut App, f: &dyn Fn(&mut App, &mut Ui)| {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(420.0, 900.0))),
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| f(app, ui));
            out.textures_delta.clear();
        };
        run(&mut app, &|a, ui| {
            picture_rows(a, ui, Op::Generate);
        });
        for id in ["gemini", "openai", "recraft", "freepik", "stability"] {
            app.settings.studio.image_provider = id.into();
            app.studio_set_key(id, "k".into());
            run(&mut app, &|a, ui| {
                assert!(picture_rows(a, ui, Op::Generate));
                picture_rows(a, ui, Op::Remix);
            });
        }
        for op in Op::EDITS {
            app.studio.op = op;
            run(&mut app, &|a, ui| edit_page(a, ui));
        }
        run(&mut app, &|a, ui| keys_page(a, ui));
    }
}
