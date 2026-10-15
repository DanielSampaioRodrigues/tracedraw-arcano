//! The AI Generate docker: Create image, Remix image and Create vectors
//! through the image service the user set up (Tools > Options > AI). The
//! work is in `ai_image.rs`; see docs/behavior/ai-generate.md.

use crate::ai_image::{Aspect, ImageStatus, Mode, MAX_COUNT, PALETTES, STYLES};
use crate::app::App;
use crate::i18n::{tr, trf};
use crate::theme::Tokens;
use crate::ui::ai_ui::{banner, Need};
use egui::{Color32, RichText, Sense, Ui, Vec2};

const ERROR: Color32 = Color32::from_rgb(0xC4, 0x2B, 0x1C);

/// Prompts Inspire Me picks from.
const IDEAS: [&str; 8] = [
    "A lighthouse on a rocky coast at dawn, seagulls, calm sea",
    "A cosy reading corner with plants and warm lamp light",
    "A fox in a snowy forest, looking over its shoulder",
    "A retro bicycle with a basket of flowers against a pastel wall",
    "A mountain cabin under a starry sky with the northern lights",
    "A bowl of ramen seen from above on a wooden table",
    "A hot air balloon festival over green hills",
    "A friendly robot watering plants in a greenhouse",
];

fn heading(ui: &mut Ui, key: &str) {
    ui.add_space(6.0);
    ui.label(RichText::new(tr(key)).strong());
}

pub fn ai_generate(app: &mut App, ui: &mut Ui) {
    let ready = app.ai_image_ready();
    banner(app, ui, Need::Images);
    // The three modes.
    ui.horizontal(|ui| {
        for (mode, key) in [
            (Mode::Create, "aigen.mode_create"),
            (Mode::Remix, "aigen.mode_remix"),
            (Mode::Vector, "aigen.mode_vector"),
        ] {
            if ui
                .selectable_label(app.ai_image.mode == mode, tr(key))
                .clicked()
            {
                app.ai_image.mode = mode;
            }
        }
    });
    let mode = app.ai_image.mode;
    let (title, about) = match mode {
        Mode::Create => ("aigen.create_title", "aigen.create_about"),
        Mode::Remix => ("aigen.remix_title", "aigen.remix_about"),
        Mode::Vector => ("aigen.vector_title", "aigen.vector_about"),
    };
    heading(ui, title);
    ui.label(
        RichText::new(tr(about))
            .color(Tokens::text_dim())
            .size(11.0),
    );
    ui.add_enabled_ui(ready && !app.ai_image.busy(), |ui| body(app, ui, mode));
    ui.add_space(8.0);
    // Generate, or the spinner and Cancel.
    if app.ai_image.busy() {
        ui.horizontal(|ui| {
            ui.spinner();
            if let ImageStatus::Waiting { done, total } = app.ai_image.status {
                ui.label(trf(
                    "aigen.waiting",
                    &[("done", &done.to_string()), ("total", &total.to_string())],
                ));
            }
            if ui.button(tr("dialog.cancel")).clicked() {
                app.ai_image_cancel();
            }
        });
    } else {
        let can = ready
            && match mode {
                Mode::Remix => {
                    app.ai_source_bitmap().is_some() && !app.ai_image.remix_prompt.trim().is_empty()
                }
                _ => !app.ai_image.prompt.trim().is_empty(),
            };
        let button =
            egui::Button::new(RichText::new(format!("\u{2726} {}", tr("aigen.generate"))).strong())
                .min_size(Vec2::new(ui.available_width(), 30.0));
        if ui.add_enabled(can, button).clicked() {
            app.ai_generate();
        }
    }
    match &app.ai_image.status {
        ImageStatus::Done(n) => {
            ui.label(trf("aigen.done", &[("n", &n.to_string())]));
        }
        ImageStatus::Failed(e) => {
            ui.label(RichText::new(e.as_str()).color(ERROR));
        }
        _ => {}
    }
    results(app, ui);
    ui.add_space(6.0);
    ui.separator();
    ui.label(
        RichText::new(tr("aigen.privacy"))
            .color(Tokens::text_dim())
            .size(11.0),
    );
}

fn body(app: &mut App, ui: &mut Ui, mode: Mode) {
    // The model in use, from the image service's list.
    ui.horizontal(|ui| {
        ui.label(tr("ai.model"));
        let prefs = &mut app.settings.ai_image;
        let current = prefs.model();
        let mut list: Vec<String> = if app.ai_image.models.is_empty() {
            prefs
                .provider
                .preset_models()
                .iter()
                .map(|m| m.to_string())
                .collect()
        } else {
            app.ai_image.models.clone()
        };
        if !list.contains(&current) {
            list.insert(0, current.clone());
        }
        egui::ComboBox::from_id_salt("aigen_model")
            .width(ui.available_width() - 8.0)
            .selected_text(current.clone())
            .show_ui(ui, |ui| {
                for m in &list {
                    if ui.selectable_label(*m == current, m).clicked() {
                        prefs.model = m.clone();
                    }
                }
            });
    });
    if mode == Mode::Remix {
        heading(ui, "aigen.source");
        source(app, ui);
    }
    // The prompt, with Reset.
    ui.horizontal(|ui| {
        heading(ui, "aigen.prompt");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .small_button("\u{21BA}")
                .on_hover_text(tr("aigen.reset"))
                .clicked()
            {
                match mode {
                    Mode::Remix => app.ai_image.remix_prompt.clear(),
                    _ => app.ai_image.prompt.clear(),
                }
            }
        });
    });
    let text = match mode {
        Mode::Remix => &mut app.ai_image.remix_prompt,
        _ => &mut app.ai_image.prompt,
    };
    ui.add(
        egui::TextEdit::multiline(text)
            .id(egui::Id::new("aigen_prompt"))
            .hint_text(tr(match mode {
                Mode::Remix => "aigen.remix_hint",
                _ => "aigen.prompt_hint",
            }))
            .desired_rows(5)
            .desired_width(f32::INFINITY),
    );
    if mode != Mode::Remix {
        if ui
            .button(format!("\u{1F4A1} {}", tr("aigen.inspire")))
            .clicked()
        {
            let n = app.ai_image.results.len() + app.ai_image.prompt.len();
            app.ai_image.prompt = IDEAS[n % IDEAS.len()].to_string();
        }
        ui.horizontal(|ui| {
            ui.label(tr("aigen.aspect"));
            let current = app.ai_image.aspect;
            egui::ComboBox::from_id_salt("aigen_aspect")
                .width(ui.available_width() - 8.0)
                .selected_text(tr(current.key()))
                .show_ui(ui, |ui| {
                    for a in Aspect::ALL {
                        ui.selectable_value(&mut app.ai_image.aspect, a, tr(a.key()));
                    }
                });
        });
    }
    if mode != Mode::Vector {
        heading(ui, "aigen.style");
        let cols = 3;
        egui::Grid::new("aigen_styles")
            .num_columns(cols)
            .spacing([4.0, 4.0])
            .show(ui, |ui| {
                for (i, (key, _)) in STYLES.iter().enumerate() {
                    let on = app.ai_image.style == Some(i);
                    if tile(ui, &tr(key), on, |p, r| style_swatch(p, r, i)).clicked() {
                        app.ai_image.style = if on { None } else { Some(i) };
                    }
                    if i % cols == cols - 1 {
                        ui.end_row();
                    }
                }
            });
    }
    heading(ui, "aigen.palette");
    let cols = 4;
    egui::Grid::new("aigen_palettes")
        .num_columns(cols)
        .spacing([4.0, 4.0])
        .show(ui, |ui| {
            for (i, (key, _, colours)) in PALETTES.iter().enumerate() {
                let on = app.ai_image.palette == Some(i);
                if tile(ui, &tr(key), on, |p, r| gradient(p, r, colours)).clicked() {
                    app.ai_image.palette = if on { None } else { Some(i) };
                }
                if i % cols == cols - 1 {
                    ui.end_row();
                }
            }
        });
    if mode == Mode::Create {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(tr("aigen.images"));
            ui.add(egui::DragValue::new(&mut app.ai_image.count).range(1..=MAX_COUNT));
        });
    }
}

/// The selected bitmap, or what to select.
fn source(app: &mut App, ui: &mut Ui) {
    match app.ai_source_bitmap() {
        Some((_, png)) => {
            if let Some(tex) = texture(ui.ctx(), &png) {
                let s = tex.size_vec2();
                let scale = (ui.available_width().min(220.0) / s.x).min(140.0 / s.y);
                ui.vertical_centered(|ui| {
                    ui.add(egui::Image::new((tex.id(), s * scale)));
                });
            }
        }
        None => {
            ui.label(RichText::new(tr("aigen.select_bitmap")).color(Tokens::text_dim()));
        }
    }
}

/// A picture's texture, made once.
fn texture(ctx: &egui::Context, png: &[u8]) -> Option<egui::TextureHandle> {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    png.hash(&mut h);
    let key = egui::Id::new(("aigen_tex", h.finish()));
    if let Some(t) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(key)) {
        return Some(t);
    }
    let img = crate::bitmap_fx::decode(png)?;
    // Thumbnails stay small.
    let img = if img.width().max(img.height()) > 256 {
        let s = 256.0 / img.width().max(img.height()) as f32;
        crate::bitmap_fx::resample(
            &img,
            ((img.width() as f32 * s) as u32).max(1),
            ((img.height() as f32 * s) as u32).max(1),
        )
    } else {
        img
    };
    let image = egui::ColorImage::from_rgba_unmultiplied(
        [img.width() as usize, img.height() as usize],
        img.as_raw(),
    );
    let t = ctx.load_texture("aigen", image, egui::TextureOptions::LINEAR);
    ctx.data_mut(|d| d.insert_temp(key, t.clone()));
    Some(t)
}

/// The pictures made this session; a click puts one on the page again.
fn results(app: &mut App, ui: &mut Ui) {
    if app.ai_image.results.is_empty() {
        return;
    }
    heading(ui, "aigen.results");
    let mut place = None;
    ui.horizontal_wrapped(|ui| {
        for (i, r) in app.ai_image.results.iter().enumerate() {
            if let Some(tex) = texture(ui.ctx(), &r.png) {
                let s = tex.size_vec2();
                let scale = 72.0 / s.x.max(s.y);
                let resp = ui
                    .add(egui::Image::new((tex.id(), s * scale)).sense(Sense::click()))
                    .on_hover_text(tr("aigen.place_again"));
                if resp.clicked() {
                    place = Some(i);
                }
            }
        }
    });
    if let Some(img) = place.and_then(|i| app.ai_image.results.get(i).cloned()) {
        app.ai_place_image(&img, None, false);
    }
}

/// A square tile with a picture and a name under it.
fn tile(
    ui: &mut Ui,
    name: &str,
    on: bool,
    paint: impl FnOnce(&egui::Painter, egui::Rect),
) -> egui::Response {
    let w = ((ui.available_width() - 16.0) / 3.0).clamp(56.0, 84.0);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, w * 0.62 + 16.0), Sense::click());
    let pic = egui::Rect::from_min_size(rect.min, Vec2::new(w, w * 0.62));
    let p = ui.painter();
    paint(p, pic);
    let stroke = if on {
        egui::Stroke::new(2.0, Tokens::ACCENT)
    } else if resp.hovered() {
        egui::Stroke::new(1.0, Tokens::ACCENT)
    } else {
        egui::Stroke::new(1.0, Tokens::border())
    };
    p.rect_stroke(pic, 2.0, stroke, egui::StrokeKind::Inside);
    p.text(
        egui::pos2(rect.center().x, pic.bottom() + 2.0),
        egui::Align2::CENTER_TOP,
        name,
        egui::FontId::proportional(10.0),
        if on { Tokens::ACCENT } else { Tokens::text() },
    );
    resp.on_hover_text(name)
}

fn rgb(c: u32) -> Color32 {
    Color32::from_rgb((c >> 16) as u8, (c >> 8) as u8, c as u8)
}

/// A swatch of colours left to right.
fn gradient(p: &egui::Painter, r: egui::Rect, colours: &[u32; 3]) {
    // One mesh: three columns of vertices, colours blended between.
    let mut mesh = egui::Mesh::default();
    for (i, c) in colours.iter().enumerate() {
        let x = r.left() + r.width() * i as f32 / 2.0;
        mesh.colored_vertex(egui::pos2(x, r.top()), rgb(*c));
        mesh.colored_vertex(egui::pos2(x, r.bottom()), rgb(*c));
    }
    for i in 0..2u32 {
        let a = i * 2;
        mesh.add_triangle(a, a + 1, a + 2);
        mesh.add_triangle(a + 1, a + 3, a + 2);
    }
    p.add(egui::Shape::mesh(mesh));
}

/// A small picture of each style: the same leaf drawn its way.
fn style_swatch(p: &egui::Painter, r: egui::Rect, i: usize) {
    let c = r.center();
    let s = r.height() * 0.32;
    let leaf = |fill: Color32, stroke: egui::Stroke| {
        let pts: Vec<egui::Pos2> = (0..24)
            .map(|k| {
                let a = k as f32 / 24.0 * std::f32::consts::TAU;
                egui::pos2(c.x + a.cos() * s * 1.6, c.y + a.sin() * s * 0.9)
            })
            .collect();
        p.add(egui::Shape::convex_polygon(pts, fill, stroke));
    };
    match i {
        // Photorealistic: soft light background, green leaf.
        0 => {
            p.rect_filled(r, 0.0, Color32::from_rgb(0x9C, 0xB8, 0x8A));
            leaf(Color32::from_rgb(0x3E, 0x7A, 0x2E), egui::Stroke::NONE);
        }
        // Line art.
        1 => {
            p.rect_filled(r, 0.0, Color32::WHITE);
            leaf(Color32::TRANSPARENT, egui::Stroke::new(1.2, Color32::BLACK));
        }
        // Watercolour.
        2 => {
            p.rect_filled(r, 0.0, Color32::from_rgb(0xF6, 0xF0, 0xE4));
            leaf(
                Color32::from_rgba_unmultiplied(0x6A, 0xA8, 0x6A, 150),
                egui::Stroke::NONE,
            );
        }
        // Illustration.
        3 => {
            p.rect_filled(r, 0.0, Color32::from_rgb(0xF9, 0xD5, 0x6E));
            leaf(
                Color32::from_rgb(0x2E, 0x8B, 0x57),
                egui::Stroke::new(1.5, Color32::from_rgb(0x1B, 0x4D, 0x31)),
            );
        }
        // 3D render.
        4 => {
            p.rect_filled(r, 0.0, Color32::from_rgb(0x4A, 0x5A, 0x6A));
            p.circle_filled(
                c + Vec2::new(2.0, 3.0),
                s * 1.2,
                Color32::from_black_alpha(80),
            );
            p.circle_filled(c, s * 1.2, Color32::from_rgb(0x58, 0xB0, 0x6A));
            p.circle_filled(
                c - Vec2::splat(s * 0.4),
                s * 0.35,
                Color32::from_white_alpha(120),
            );
        }
        // Pop art.
        _ => {
            p.rect_filled(r, 0.0, Color32::from_rgb(0xFF, 0x3D, 0x7F));
            for k in 0..6 {
                p.circle_filled(
                    egui::pos2(r.left() + 6.0 + k as f32 * 10.0, r.top() + 6.0),
                    2.0,
                    Color32::from_rgb(0xFF, 0xE0, 0x3D),
                );
            }
            leaf(
                Color32::from_rgb(0x00, 0xC8, 0xFF),
                egui::Stroke::new(2.0, Color32::BLACK),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(ctx: &egui::Context, app: &mut App) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(360.0, 900.0),
            )),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| ai_generate(app, ui));
        out.textures_delta.clear();
    }

    #[test]
    fn the_docker_draws_in_every_mode_and_state() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        // Not set up: the banner, controls greyed.
        frame(&ctx, &mut app);
        app.ai_image.key = "k".into();
        for mode in [Mode::Create, Mode::Remix, Mode::Vector] {
            app.ai_image.mode = mode;
            for status in [
                ImageStatus::Idle,
                ImageStatus::Done(2),
                ImageStatus::Failed("no".into()),
                ImageStatus::Waiting { done: 1, total: 3 },
            ] {
                app.ai_image.status = status;
                frame(&ctx, &mut app);
            }
        }
        // With results to show.
        let png = crate::bitmap_fx::encode(&image::RgbaImage::from_pixel(
            4,
            3,
            image::Rgba([9, 9, 9, 255]),
        ))
        .expect("png");
        app.ai_image.results.push(crate::ai_image::GenImage {
            png,
            width: 4,
            height: 3,
        });
        frame(&ctx, &mut app);
    }
}
