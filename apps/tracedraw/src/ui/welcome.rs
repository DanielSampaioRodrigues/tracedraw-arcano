//! Welcome Screen: Get Started (new, open, recent documents), Workspace,
//! What's New, Learn, Templates.

use crate::app::{App, WelcomeTab};
use crate::i18n::{tr, trf};
use crate::theme::Tokens;
use crate::ui::icons;
use egui::Ui;

const NEWS: &[(&str, &str)] = &[
    ("welcome.news_1_title", "welcome.news_1_body"),
    ("welcome.news_2_title", "welcome.news_2_body"),
    ("welcome.news_3_title", "welcome.news_3_body"),
];

pub fn welcome_screen(app: &mut App, ui: &mut Ui) {
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.add_space(24.0);
        ui.heading(egui::RichText::new("TraceDraw").size(26.0));
        ui.add_space(30.0);
        for (tab, key) in [
            (WelcomeTab::GetStarted, "welcome.get_started"),
            (WelcomeTab::Workspace, "welcome.workspace"),
            (WelcomeTab::News, "welcome.whats_new"),
            (WelcomeTab::Learn, "welcome.learn"),
            (WelcomeTab::Templates, "welcome.templates"),
        ] {
            if ui
                .selectable_label(
                    app.welcome_tab == tab,
                    egui::RichText::new(tr(key)).size(14.0),
                )
                .clicked()
            {
                app.welcome_tab = tab;
            }
        }
    });
    ui.separator();
    ui.add_space(12.0);
    match app.welcome_tab {
        WelcomeTab::GetStarted => get_started(app, ui),
        WelcomeTab::Workspace => workspace(app, ui),
        WelcomeTab::News => news(ui),
        WelcomeTab::Learn => learn(ui),
        WelcomeTab::Templates => templates(app, ui),
    }
}

fn big_button(ui: &mut Ui, icon: icons::Action, title: String, subtitle: String) -> bool {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(240.0, 64.0), egui::Sense::click());
    let bg = if resp.hovered() {
        Tokens::TOOL_HOVER
    } else {
        Tokens::PANEL
    };
    let painter = ui.painter();
    painter.rect_filled(rect, 4.0, bg);
    painter.rect_stroke(
        rect,
        4.0,
        egui::Stroke::new(1.0, Tokens::BORDER),
        egui::epaint::StrokeKind::Inside,
    );
    let icon_rect = egui::Rect::from_center_size(
        egui::pos2(rect.left() + 32.0, rect.center().y),
        egui::vec2(28.0, 28.0),
    );
    icons::draw_action(painter, icon_rect, icon, Tokens::ACCENT);
    let text_x = rect.left() + 64.0;
    painter.text(
        egui::pos2(
            text_x,
            rect.center().y - if subtitle.is_empty() { 0.0 } else { 9.0 },
        ),
        egui::Align2::LEFT_CENTER,
        title,
        egui::FontId::proportional(14.0),
        Tokens::TEXT,
    );
    if !subtitle.is_empty() {
        painter.text(
            egui::pos2(text_x, rect.center().y + 10.0),
            egui::Align2::LEFT_CENTER,
            subtitle,
            egui::FontId::proportional(11.0),
            Tokens::TEXT_DIM,
        );
    }
    resp.clicked()
}

fn get_started(app: &mut App, ui: &mut Ui) {
    ui.horizontal_top(|ui| {
        ui.add_space(24.0);
        ui.vertical(|ui| {
            ui.set_width(260.0);
            if big_button(
                ui,
                icons::Action::New,
                tr("welcome.new_document"),
                "Ctrl+N".into(),
            ) {
                app.request_new_document();
            }
            ui.add_space(8.0);
            if big_button(
                ui,
                icons::Action::Welcome,
                tr("welcome.new_from_template"),
                String::new(),
            ) {
                app.welcome_tab = WelcomeTab::Templates;
            }
            ui.add_space(8.0);
            if big_button(
                ui,
                icons::Action::Open,
                tr("welcome.open_document"),
                "Ctrl+O".into(),
            ) {
                app.open_dialog();
                if app.file.is_some() || app.doc().all_layers().any(|l| !l.shapes.is_empty()) {
                    app.show_welcome = false;
                }
            }
            ui.add_space(20.0);
            ui.label(
                egui::RichText::new(tr("welcome.shortcuts_hint"))
                    .color(Tokens::TEXT_DIM)
                    .size(11.0),
            );
        });
        ui.add_space(40.0);
        ui.vertical(|ui| {
            // A browser keeps no recent documents; what the demo leaves out
            // takes their place.
            if crate::files::WEB {
                web_demo_notice(app, ui);
                return;
            }
            ui.strong(tr("welcome.recent_documents"));
            let recent = app.settings.recent_files.clone();
            if recent.is_empty() {
                ui.label(egui::RichText::new(tr("welcome.no_recent")).color(Tokens::TEXT_DIM));
            }
            for p in recent {
                let name = p
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                let exists = p.exists();
                let r = ui.add_enabled(
                    exists,
                    egui::Button::new(egui::RichText::new(format!("\u{1F5CE} {name}")).size(13.0))
                        .frame(false),
                );
                let r = r.on_hover_text(p.display().to_string());
                if r.clicked() {
                    app.open_path(p.clone());
                    app.show_welcome = false;
                }
            }
            if !app.settings.recent_files.is_empty()
                && ui.small_button(tr("menu.file.clear_recent")).clicked()
            {
                app.settings.recent_files.clear();
                app.settings.save();
            }
        });
    });
}

/// Where the desktop app is downloaded.
pub const DOWNLOADS: &str = "https://github.com/igtoth/tracedraw/releases/latest";

const NOTE_FILL: egui::Color32 = egui::Color32::from_rgb(0xFF, 0xF7, 0xE0);
const NOTE_EDGE: egui::Color32 = egui::Color32::from_rgb(0xE6, 0xC8, 0x7A);

/// In the browser: what this version leaves out, and a button to the
/// desktop downloads.
pub fn web_demo_notice(app: &mut App, ui: &mut Ui) {
    egui::Frame::new()
        .fill(NOTE_FILL)
        .stroke(egui::Stroke::new(1.0, NOTE_EDGE))
        .inner_margin(12)
        .show(ui, |ui| {
            ui.set_max_width(440.0);
            ui.strong(tr("welcome.web_demo_title"));
            ui.add_space(4.0);
            ui.label(tr("welcome.web_demo_text"));
            ui.add_space(8.0);
            if ui.button(tr("welcome.web_demo_download")).clicked() {
                app.open_url(DOWNLOADS);
            }
        });
}

/// In the browser: a link to the desktop downloads at the right end of the
/// menu bar.
pub fn web_demo_link(app: &mut App, ui: &mut Ui) {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.add_space(8.0);
        if ui
            .link(tr("welcome.web_demo_link"))
            .on_hover_text(tr("welcome.web_demo_text"))
            .clicked()
        {
            app.open_url(DOWNLOADS);
        }
    });
}

fn workspace(app: &mut App, ui: &mut Ui) {
    ui.horizontal_top(|ui| {
        ui.add_space(24.0);
        ui.vertical(|ui| {
            ui.strong(tr("welcome.choose_workspace"));
            for (ws, key, desc) in [
                (
                    crate::app::Workspace::Default,
                    "menu.window.workspace_default",
                    "welcome.ws_default_desc",
                ),
                (
                    crate::app::Workspace::Lite,
                    "menu.window.workspace_lite",
                    "welcome.ws_lite_desc",
                ),
                (
                    crate::app::Workspace::Classic,
                    "menu.window.workspace_classic",
                    "welcome.ws_classic_desc",
                ),
                (
                    crate::app::Workspace::Illustration,
                    "menu.window.workspace_illustration",
                    "welcome.ws_illustration_desc",
                ),
                (
                    crate::app::Workspace::PageLayout,
                    "menu.window.workspace_page_layout",
                    "welcome.ws_page_layout_desc",
                ),
            ] {
                ui.horizontal(|ui| {
                    if ui
                        .selectable_label(
                            app.workspace == ws,
                            egui::RichText::new(tr(key)).size(13.0),
                        )
                        .clicked()
                    {
                        app.set_workspace(ws);
                        app.save_settings();
                    }
                    ui.label(
                        egui::RichText::new(tr(desc))
                            .color(Tokens::TEXT_DIM)
                            .size(11.0),
                    );
                });
            }
            ui.add_space(12.0);
            ui.strong(tr("options.language"));
            let current = crate::i18n::language();
            ui.horizontal_wrapped(|ui| {
                for (code, name) in crate::i18n::LANGUAGES {
                    if ui.selectable_label(current == code, name).clicked() {
                        crate::i18n::set_language(code);
                        app.settings.language = code.to_string();
                        app.settings.save();
                    }
                }
            });
        });
    });
}

fn news(ui: &mut Ui) {
    ui.horizontal_top(|ui| {
        ui.add_space(24.0);
        ui.vertical(|ui| {
            ui.set_max_width(640.0);
            ui.strong(trf(
                "welcome.version_n",
                &[("v", env!("CARGO_PKG_VERSION"))],
            ));
            for (t, b) in NEWS {
                ui.add_space(8.0);
                ui.label(egui::RichText::new(tr(t)).strong().size(14.0));
                ui.label(tr(b));
            }
        });
    });
}

fn learn(ui: &mut Ui) {
    ui.horizontal_top(|ui| {
        ui.add_space(24.0);
        ui.vertical(|ui| {
            ui.set_max_width(640.0);
            for (t, b) in [
                ("welcome.learn_1_title", "welcome.learn_1_body"),
                ("welcome.learn_2_title", "welcome.learn_2_body"),
                ("welcome.learn_3_title", "welcome.learn_3_body"),
                ("welcome.learn_4_title", "welcome.learn_4_body"),
                ("welcome.learn_5_title", "welcome.learn_5_body"),
            ] {
                ui.add_space(6.0);
                ui.label(egui::RichText::new(tr(t)).strong().size(14.0));
                ui.label(tr(b));
            }
        });
    });
}

fn templates(app: &mut App, ui: &mut Ui) {
    ui.horizontal_top(|ui| {
        ui.add_space(24.0);
        ui.vertical(|ui| {
            ui.strong(tr("welcome.templates"));
            ui.horizontal_wrapped(|ui| {
                for (key, size) in [
                    (
                        "welcome.tpl_a4_portrait",
                        tracedraw_core::document::paper::A4,
                    ),
                    (
                        "welcome.tpl_a4_landscape",
                        tracedraw_core::geometry::Size::new(297.0, 210.0),
                    ),
                    ("welcome.tpl_a3", tracedraw_core::document::paper::A3),
                    (
                        "welcome.tpl_letter",
                        tracedraw_core::document::paper::LETTER,
                    ),
                    (
                        "welcome.tpl_business_card",
                        tracedraw_core::geometry::Size::new(90.0, 50.0),
                    ),
                    (
                        "welcome.tpl_poster",
                        tracedraw_core::geometry::Size::new(420.0, 594.0),
                    ),
                    (
                        "welcome.tpl_web_1080p",
                        tracedraw_core::geometry::Size::new(
                            1920.0 * 25.4 / 96.0,
                            1080.0 * 25.4 / 96.0,
                        ),
                    ),
                    (
                        "welcome.tpl_social_square",
                        tracedraw_core::geometry::Size::new(
                            1080.0 * 25.4 / 96.0,
                            1080.0 * 25.4 / 96.0,
                        ),
                    ),
                ] {
                    if ui
                        .add_sized(
                            [150.0, 54.0],
                            egui::Button::new(format!(
                                "{}\n{} x {} mm",
                                tr(key),
                                size.width.round(),
                                size.height.round()
                            )),
                        )
                        .clicked()
                    {
                        let name = app.next_untitled_name();
                        let doc = App::localized_document(name, size);
                        app.open_document(doc, None);
                    }
                }
            });
            ui.add_space(12.0);
            ui.strong(tr("welcome.my_templates"));
            if ui.button(tr("welcome.open_template")).clicked() {
                crate::files::Dialog::new()
                    .add_filter("TraceDraw template", &["tdt", "tdraw"])
                    .pick_file(app, |app, p| {
                        app.open_path(p);
                        app.file = None;
                        app.show_welcome = false;
                    });
            }
            let dir = crate::settings::config_dir().map(|d| d.join("templates"));
            if let Some(dir) = dir {
                if let Ok(rd) = std::fs::read_dir(&dir) {
                    for e in rd.flatten() {
                        let p = e.path();
                        if p.extension().and_then(|x| x.to_str()) == Some("tdt") {
                            let name = p
                                .file_stem()
                                .map(|s| s.to_string_lossy().to_string())
                                .unwrap_or_default();
                            if ui.button(name).clicked() {
                                app.open_path(p.clone());
                                app.file = None;
                                app.show_welcome = false;
                            }
                        }
                    }
                }
                ui.label(
                    egui::RichText::new(trf(
                        "welcome.templates_folder",
                        &[("d", &dir.display().to_string())],
                    ))
                    .color(Tokens::TEXT_DIM)
                    .size(11.0),
                );
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The browser's notice and menu bar link draw (they show only in the
    /// browser build, so they are drawn here directly).
    #[test]
    fn the_browser_demo_notice_draws() {
        let ctx = crate::theme::ui_context();
        let mut app = App::headless();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1200.0, 800.0),
            )),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| {
            web_demo_notice(&mut app, ui);
            ui.horizontal(|ui| web_demo_link(&mut app, ui));
        });
        out.textures_delta.clear();
        assert!(DOWNLOADS.starts_with("https://"));
    }
}
