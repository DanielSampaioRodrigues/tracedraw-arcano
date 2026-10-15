//! The AI parts shared by the dockers, menus and Tools > Options > AI:
//! the banner shown while AI features are off or not set up, and the
//! three pages of the AI options (General, Drawing service, Image
//! service). See docs/behavior/ai-options.md.

use crate::ai_image::ImageProvider;
use crate::app::App;
use crate::i18n::tr;
use crate::theme::Tokens;
use crate::ui::dialogs::{Dialog, OptionsPage};
use crate::ui::options::{combo, grid, row_label};
use egui::{Color32, RichText, Ui};

const FIELD_W: f32 = 260.0;
const OK: Color32 = Color32::from_rgb(0x2E, 0x7D, 0x32);
const ERROR: Color32 = Color32::from_rgb(0xC4, 0x2B, 0x1C);

/// Which AI service a part of the interface needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Need {
    /// AI Drawing (a text service).
    Drawing,
    /// AI Generate and Remove Background (an image service).
    Images,
}

/// The banner's message, or None when the service is ready.
pub fn banner_text(app: &App, need: Need) -> Option<String> {
    if !app.ai_features_on() {
        return Some(tr("aiui.banner_off"));
    }
    let ready = match need {
        Need::Drawing => app.ai_drawing_ready(),
        Need::Images => app.ai_image_ready(),
    };
    (!ready).then(|| match need {
        Need::Drawing => tr("aiui.banner_drawing"),
        Need::Images => tr("aiui.banner_images"),
    })
}

/// Open Tools > Options > AI at `page`.
pub fn open_options(app: &mut App, page: OptionsPage) {
    app.dialog = Dialog::Options;
    app.options_page = page;
}

/// The banner: what AI features need and a button to set them up. Returns
/// true when it showed (the service is not ready).
pub fn banner(app: &mut App, ui: &mut Ui, need: Need) -> bool {
    let Some(text) = banner_text(app, need) else {
        return false;
    };
    let dark = crate::theme::theme().is_dark();
    let (fill, line) = if dark {
        (
            Color32::from_rgb(0x1C, 0x33, 0x4A),
            Color32::from_rgb(0x3D, 0x7A, 0xB8),
        )
    } else {
        (Color32::from_rgb(0xE8, 0xF2, 0xFC), Tokens::ACCENT)
    };
    egui::Frame::new()
        .fill(fill)
        .stroke(egui::Stroke::new(1.0, line))
        .corner_radius(4.0)
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_top(|ui| {
                ui.label(RichText::new("\u{2726}").size(18.0).color(line));
                ui.vertical(|ui| {
                    ui.label(RichText::new(tr("aiui.banner_title")).strong());
                    ui.label(text);
                    ui.add_space(4.0);
                    if ui.button(tr("aiui.set_up")).clicked() {
                        let page = if !app.ai_features_on() {
                            OptionsPage::AiGeneral
                        } else if need == Need::Drawing {
                            OptionsPage::AiDrawing
                        } else {
                            OptionsPage::AiImages
                        };
                        open_options(app, page);
                    }
                });
            });
        });
    ui.add_space(6.0);
    true
}

fn dim(ui: &mut Ui, key: &str) {
    ui.label(RichText::new(tr(key)).color(Tokens::text_dim()));
}

fn state_line(ui: &mut Ui, ready: bool) {
    if ready {
        ui.label(RichText::new(tr("aiui.ready")).color(OK));
    } else {
        ui.label(RichText::new(tr("aiui.not_ready")).color(Tokens::text_dim()));
    }
}

/// Tools > Options > AI > General: the switch for all AI features and
/// where each service stands.
pub fn general_page(app: &mut App, ui: &mut Ui) {
    ui.checkbox(&mut app.settings.ai_enabled, tr("aiui.enable"));
    dim(ui, "aiui.enable_hint");
    crate::ui::chrome::section(ui, &tr("aiui.services"));
    grid("ai_services").show(ui, |ui| {
        row_label(ui, &tr("aiui.drawing_service"));
        state_line(ui, app.ai_drawing_ready());
        ui.end_row();
        row_label(ui, &tr("aiui.image_service"));
        state_line(ui, app.ai_image_ready());
        ui.end_row();
    });
    crate::ui::chrome::section(ui, &tr("aiui.features"));
    for (key, need) in [
        ("aiui.feature_drawing", Need::Drawing),
        ("aiui.feature_generate", Need::Images),
        ("aiui.feature_remix", Need::Images),
        ("aiui.feature_vectors", Need::Images),
        ("aiui.feature_cutout", Need::Images),
    ] {
        let on = match need {
            Need::Drawing => app.ai_drawing_ready(),
            Need::Images => app.ai_image_ready(),
        };
        ui.add_enabled(on, egui::Label::new(format!("\u{2022} {}", tr(key))));
    }
    ui.add_space(8.0);
    crate::ui::chrome::section(ui, &tr("aiui.privacy_title"));
    dim(ui, "aiui.privacy");
}

/// Tools > Options > AI > Drawing service: the text service of AI
/// Drawing.
pub fn drawing_page(app: &mut App, ui: &mut Ui) {
    if !app.ai_features_on() {
        banner(app, ui, Need::Drawing);
    }
    dim(ui, "aiui.drawing_hint");
    ui.add_space(6.0);
    crate::ui::ai_docker::connection(app, ui);
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        let busy = app.ai.busy();
        if ui
            .add_enabled(!busy, egui::Button::new(tr("aiui.test")))
            .on_hover_text(tr("aiui.test_hint"))
            .clicked()
        {
            app.ai_fetch_models();
        }
        if busy {
            ui.spinner();
        }
        match &app.ai.status {
            crate::ai::AiStatus::Listed(n) => {
                ui.label(
                    RichText::new(crate::i18n::trf("aiui.test_ok", &[("n", &n.to_string())]))
                        .color(OK),
                );
            }
            crate::ai::AiStatus::Failed(e) => {
                ui.label(RichText::new(e.as_str()).color(ERROR));
            }
            _ => {}
        }
    });
    state_line(ui, app.ai_drawing_ready());
}

/// Tools > Options > AI > Image service: the service of AI Generate and
/// Remove Background.
pub fn images_page(app: &mut App, ui: &mut Ui) {
    if !app.ai_features_on() {
        banner(app, ui, Need::Images);
    }
    dim(ui, "aiui.images_hint");
    ui.add_space(6.0);
    image_connection(app, ui);
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        let busy = app.ai_image.busy();
        if ui
            .add_enabled(!busy, egui::Button::new(tr("aiui.test")))
            .on_hover_text(tr("aiui.test_hint"))
            .clicked()
        {
            app.ai_image_fetch_models();
        }
        if busy {
            ui.spinner();
        }
        match &app.ai_image.status {
            crate::ai_image::ImageStatus::Listed(n) => {
                ui.label(
                    RichText::new(crate::i18n::trf("aiui.test_ok", &[("n", &n.to_string())]))
                        .color(OK),
                );
            }
            crate::ai_image::ImageStatus::Failed(e) => {
                ui.label(RichText::new(e.as_str()).color(ERROR));
            }
            _ => {}
        }
    });
    state_line(ui, app.ai_image_ready());
}

/// Service, address, model and key of the image service.
pub fn image_connection(app: &mut App, ui: &mut Ui) {
    let prefs = &mut app.settings.ai_image;
    grid("ai_image_grid").show(ui, |ui| {
        row_label(ui, &tr("ai.provider"));
        let mut provider = prefs.provider;
        if combo(
            ui,
            "ai_image_provider",
            &mut provider,
            &ImageProvider::ALL,
            |p| tr(p.key()),
            FIELD_W,
        ) {
            prefs.provider = provider;
            prefs.endpoint.clear();
            prefs.model.clear();
            app.ai_image.models.clear();
        }
        ui.end_row();
        row_label(ui, &tr("ai.endpoint"));
        ui.add(
            egui::TextEdit::singleline(&mut prefs.endpoint)
                .hint_text(prefs.provider.default_endpoint())
                .desired_width(FIELD_W),
        );
        ui.end_row();
        row_label(ui, &tr("ai.model"));
        let mut choices: Vec<String> = if app.ai_image.models.is_empty() {
            prefs
                .provider
                .preset_models()
                .iter()
                .map(|m| m.to_string())
                .collect()
        } else {
            app.ai_image.models.clone()
        };
        let current = prefs.model();
        if !choices.contains(&current) {
            choices.insert(0, current.clone());
        }
        egui::ComboBox::from_id_salt("ai_image_model")
            .width(FIELD_W)
            .selected_text(current.clone())
            .show_ui(ui, |ui| {
                for m in &choices {
                    if ui.selectable_label(*m == current, m).clicked() {
                        prefs.model = m.clone();
                    }
                }
            });
        ui.end_row();
        row_label(ui, &tr("ai.key"));
        let key = ui.add(
            egui::TextEdit::singleline(&mut app.ai_image.key)
                .password(true)
                .desired_width(FIELD_W),
        );
        if key.changed() && prefs.remember_key {
            prefs.key = app.ai_image.key.clone();
        }
        ui.end_row();
    });
    if ui
        .checkbox(&mut prefs.remember_key, tr("ai.remember_key"))
        .changed()
    {
        prefs.key = if prefs.remember_key {
            app.ai_image.key.clone()
        } else {
            String::new()
        };
        app.settings.save();
    }
    dim(ui, "aiui.image_key_hint");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(ctx: &egui::Context, app: &mut App, f: impl Fn(&mut App, &mut Ui)) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1200.0, 800.0),
            )),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| f(app, ui));
        out.textures_delta.clear();
    }

    #[test]
    fn the_banner_shows_until_the_service_is_set_up() {
        let mut app = App::headless();
        app.settings.ai_enabled = false;
        assert_eq!(banner_text(&app, Need::Images), Some(tr("aiui.banner_off")));
        app.settings.ai_enabled = true;
        assert_eq!(
            banner_text(&app, Need::Images),
            Some(tr("aiui.banner_images"))
        );
        assert_eq!(
            banner_text(&app, Need::Drawing),
            Some(tr("aiui.banner_drawing"))
        );
        app.ai_image.key = "k".into();
        assert_eq!(banner_text(&app, Need::Images), None);
        app.ai.key = "k".into();
        assert_eq!(banner_text(&app, Need::Drawing), None);
        // Turning AI off hides every feature again.
        app.settings.ai_enabled = false;
        assert!(!app.ai_image_ready() && !app.ai_drawing_ready());
    }

    #[test]
    fn the_pages_and_the_banner_draw_in_every_state() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        for on in [false, true] {
            app.settings.ai_enabled = on;
            for p in ImageProvider::ALL {
                app.settings.ai_image.provider = p;
                frame(&ctx, &mut app, general_page);
                frame(&ctx, &mut app, drawing_page);
                frame(&ctx, &mut app, images_page);
                frame(&ctx, &mut app, |a, ui| {
                    banner(a, ui, Need::Images);
                });
            }
        }
        app.ai_image.status = crate::ai_image::ImageStatus::Failed("bad key".into());
        frame(&ctx, &mut app, images_page);
    }

    #[test]
    fn the_image_key_is_kept_only_when_asked() {
        let mut app = App::headless();
        app.ai_image.key = "img-secret".into();
        let json = serde_json::to_string(&app.settings).expect("settings");
        assert!(!json.contains("img-secret"));
        app.settings.ai_image.remember_key = true;
        app.settings.ai_image.key = app.ai_image.key.clone();
        let json = serde_json::to_string(&app.settings).expect("settings");
        let back: crate::settings::Settings = serde_json::from_str(&json).expect("read");
        assert_eq!(
            crate::ai_image::remembered_key(&back.ai_image),
            "img-secret"
        );
        // Older preference files turn AI features on.
        let old: crate::settings::Settings = serde_json::from_str("{}").expect("read");
        assert!(old.ai_enabled);
    }
}
