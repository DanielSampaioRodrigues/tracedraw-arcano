//! Arcano edition: the AI actions, one by one. The Generate with AI
//! panel opens on a page of them, in two groups (make something new;
//! work on the selected picture), each a tile with a picture and its
//! name; a tile opens that action's page, where one list picks the
//! company and the model together. The contextual bar offers the same
//! actions next to the selection. See docs/behavior/ai-actions.md.

use crate::ai_image::Mode;
use crate::app::App;
use crate::i18n::tr;
use crate::studio::Op;
use crate::theme::Tokens;
use crate::ui::tips::RichTip as _;
use egui::{epaint, vec2, Color32, Painter, Pos2, Rect, Sense, Stroke, Ui};

/// One thing the AI can do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiAction {
    Image,
    Vector,
    Draw,
    Remix,
    Expand,
    Upscale,
    RemoveBackground,
    ReplaceBackground,
    Relight,
    Vectorize,
}

impl AiAction {
    /// Make something new.
    pub const CREATE: [AiAction; 3] = [AiAction::Image, AiAction::Vector, AiAction::Draw];
    /// Work on the selected picture.
    pub const ON_PICTURE: [AiAction; 7] = [
        AiAction::RemoveBackground,
        AiAction::Expand,
        AiAction::Upscale,
        AiAction::Remix,
        AiAction::ReplaceBackground,
        AiAction::Relight,
        AiAction::Vectorize,
    ];

    pub fn key(self) -> &'static str {
        match self {
            AiAction::Image => "aiact.image",
            AiAction::Vector => "aiact.vector",
            AiAction::Draw => "aiact.draw",
            AiAction::Remix => "aiact.remix",
            AiAction::Expand => "aiact.expand",
            AiAction::Upscale => "aiact.upscale",
            AiAction::RemoveBackground => "aiact.remove_background",
            AiAction::ReplaceBackground => "aiact.replace_background",
            AiAction::Relight => "aiact.relight",
            AiAction::Vectorize => "aiact.vectorize",
        }
    }

    /// The studio's operation on a picture, for the actions that are one.
    pub fn op(self) -> Option<Op> {
        match self {
            AiAction::Expand => Some(Op::Expand),
            AiAction::Upscale => Some(Op::Upscale),
            AiAction::RemoveBackground => Some(Op::RemoveBackground),
            AiAction::ReplaceBackground => Some(Op::ReplaceBackground),
            AiAction::Relight => Some(Op::Relight),
            AiAction::Vectorize => Some(Op::Vectorize),
            _ => None,
        }
    }

    /// It runs at once from the contextual bar: nothing to type.
    pub fn quick(self) -> bool {
        matches!(
            self,
            AiAction::RemoveBackground | AiAction::Upscale | AiAction::Vectorize
        )
    }

    /// The action the panel shows now.
    pub fn current(app: &App) -> AiAction {
        match app.ai_image.mode {
            Mode::Create => AiAction::Image,
            Mode::Vector => AiAction::Vector,
            Mode::Draw => AiAction::Draw,
            Mode::Remix => AiAction::Remix,
            Mode::Edit => match app.studio.op {
                Op::Expand => AiAction::Expand,
                Op::Upscale => AiAction::Upscale,
                Op::RemoveBackground => AiAction::RemoveBackground,
                Op::ReplaceBackground => AiAction::ReplaceBackground,
                Op::Relight => AiAction::Relight,
                _ => AiAction::Vectorize,
            },
        }
    }
}

/// Open `action`'s page in the panel.
pub fn open(app: &mut App, action: AiAction) {
    app.ai_image.info = false;
    app.ai_image.home = false;
    match action {
        AiAction::Image => app.ai_image.mode = Mode::Create,
        AiAction::Vector => app.ai_image.mode = Mode::Vector,
        AiAction::Draw => app.ai_image.mode = Mode::Draw,
        // The selected picture is the one to remix (the page takes it).
        AiAction::Remix => app.ai_image.mode = Mode::Remix,
        a => {
            app.ai_image.mode = Mode::Edit;
            if let Some(op) = a.op() {
                app.studio.op = op;
            }
        }
    }
}

/// Open the Generate with AI panel at `action` (from a menu or the
/// contextual bar); a quick action with a service ready runs at once.
pub fn start(app: &mut App, action: AiAction) {
    open(app, action);
    if action.quick() && app.ai_source_bitmap().is_some() {
        if let Some(op) = action.op() {
            let ready = app
                .studio_op_service(op)
                .is_some_and(|p| !app.studio_key(&p).trim().is_empty());
            if ready && app.ai_features_on() {
                app.studio_edit();
            }
        }
    }
    app.open_docker(crate::app::DockerTab::AiGenerate);
}

/// The page of actions.
pub fn home(app: &mut App, ui: &mut Ui) {
    let has_picture = app.ai_source_bitmap().is_some();
    group(app, ui, "aiact.group_create", &AiAction::CREATE, true);
    ui.add_space(6.0);
    group(
        app,
        ui,
        "aiact.group_picture",
        &AiAction::ON_PICTURE,
        has_picture,
    );
    if !has_picture {
        ui.label(
            egui::RichText::new(tr("aiact.select_picture"))
                .size(11.0)
                .color(Tokens::text_dim()),
        );
    }
}

fn group(app: &mut App, ui: &mut Ui, title: &str, actions: &[AiAction], enabled: bool) {
    ui.label(
        egui::RichText::new(tr(title).to_uppercase())
            .size(10.5)
            .strong()
            .color(Tokens::text_dim()),
    );
    ui.add_space(2.0);
    let w = ((ui.available_width() - 6.0) / 2.0).max(90.0);
    for row in actions.chunks(2) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            for &a in row {
                if tile(ui, a, w, enabled).clicked() {
                    open(app, a);
                }
            }
        });
        ui.add_space(2.0);
    }
}

/// A tile: the action's picture and its name.
fn tile(ui: &mut Ui, a: AiAction, w: f32, enabled: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(
        vec2(w, 54.0),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    let p = ui.painter();
    let fill = if enabled && resp.hovered() {
        Tokens::tool_hover()
    } else {
        Tokens::field()
    };
    p.rect_filled(rect, 8.0, fill);
    let ink = if enabled {
        Tokens::icon()
    } else {
        Tokens::icon_off()
    };
    let acc = if enabled {
        crate::ui::icons::accent()
    } else {
        Tokens::icon_off()
    };
    draw(
        p,
        Rect::from_center_size(rect.left_center() + vec2(22.0, 0.0), vec2(22.0, 22.0)),
        a,
        ink,
        acc,
    );
    let text_rect = Rect::from_min_max(rect.min + vec2(42.0, 4.0), rect.max - vec2(6.0, 4.0));
    let galley = p.layout(
        tr(a.key()),
        egui::FontId::proportional(12.0),
        if enabled {
            Tokens::text()
        } else {
            Tokens::text_dim()
        },
        text_rect.width(),
    );
    p.galley(
        egui::pos2(text_rect.left(), rect.center().y - galley.size().y / 2.0),
        galley,
        Tokens::text(),
    );
    resp.rich_tip(crate::ui::tips::pair(a.key(), &format!("{}_desc", a.key())))
}

/// The page header of an action: back to the actions, and its name.
pub fn header(app: &mut App, ui: &mut Ui) {
    let a = AiAction::current(app);
    ui.horizontal(|ui| {
        if ui
            .button(format!("\u{2039} {}", tr("aiact.back")))
            .rich_tip(crate::ui::tips::pair("aiact.back", "aiact.back_desc"))
            .clicked()
        {
            app.ai_image.home = true;
        }
        let (r, _) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
        draw(
            ui.painter(),
            r,
            a,
            Tokens::icon(),
            crate::ui::icons::accent(),
        );
        ui.label(egui::RichText::new(tr(a.key())).strong());
    });
}

/// `a`'s picture in `r`, drawn on a 16-unit grid.
pub fn draw(p: &Painter, r: Rect, a: AiAction, ink: Color32, acc: Color32) {
    let k = r.width() / 16.0;
    let at = |x: f32, y: f32| -> Pos2 { r.min + vec2(x, y) * k };
    let s = |c: Color32| Stroke::new(1.3 * k.max(0.8), c);
    let line = |pts: &[(f32, f32)], c: Color32| {
        p.add(epaint::PathShape::line(
            pts.iter().map(|&(x, y)| at(x, y)).collect(),
            s(c),
        ));
    };
    let frame = |x0: f32, y0: f32, x1: f32, y1: f32, c: Color32| {
        p.rect_stroke(
            Rect::from_min_max(at(x0, y0), at(x1, y1)),
            1.5 * k,
            s(c),
            egui::StrokeKind::Middle,
        );
    };
    let sparkle = |x: f32, y: f32, rad: f32, c: Color32| {
        let w = rad * 0.3;
        for pts in [
            [(x, y - rad), (x + w, y), (x, y + rad), (x - w, y)],
            [(x - rad, y), (x, y - w), (x + rad, y), (x, y + w)],
        ] {
            p.add(epaint::PathShape::convex_polygon(
                pts.iter().map(|&(px, py)| at(px, py)).collect(),
                c,
                Stroke::NONE,
            ));
        }
    };
    // A small picture: frame, hills and sun.
    let picture = |x0: f32, y0: f32, x1: f32, y1: f32| {
        frame(x0, y0, x1, y1, ink);
        let (w, h) = (x1 - x0, y1 - y0);
        line(
            &[
                (x0 + w * 0.12, y1 - h * 0.15),
                (x0 + w * 0.4, y0 + h * 0.45),
                (x0 + w * 0.6, y0 + h * 0.7),
                (x0 + w * 0.72, y0 + h * 0.58),
                (x1 - w * 0.1, y1 - h * 0.15),
            ],
            ink,
        );
    };
    match a {
        AiAction::Image => {
            picture(1.0, 3.0, 13.0, 14.0);
            sparkle(13.0, 3.5, 3.0, acc);
        }
        AiAction::Vector => {
            line(&[(2.0, 13.0), (8.0, 3.0), (14.0, 13.0)], ink);
            for (x, y) in [(2.0, 13.0), (14.0, 13.0)] {
                p.rect_filled(
                    Rect::from_center_size(at(x, y), vec2(3.0, 3.0) * k),
                    0.5,
                    ink,
                );
            }
            sparkle(8.0, 3.0, 3.0, acc);
        }
        AiAction::Draw => {
            line(&[(2.0, 14.0), (11.0, 5.0)], ink);
            line(&[(2.0, 14.0), (3.5, 10.5), (5.5, 12.5), (2.0, 14.0)], ink);
            sparkle(12.5, 3.5, 3.0, acc);
        }
        AiAction::Remix => {
            picture(3.0, 4.0, 13.0, 12.5);
            line(&[(1.0, 6.0), (1.0, 1.5), (5.5, 1.5)], acc);
            line(&[(15.0, 10.0), (15.0, 14.5), (10.5, 14.5)], acc);
        }
        AiAction::Expand => {
            picture(5.0, 5.0, 11.0, 11.0);
            frame(1.0, 1.0, 15.0, 15.0, acc);
        }
        AiAction::Upscale => {
            frame(1.0, 7.0, 9.0, 15.0, ink);
            line(&[(8.0, 8.0), (14.5, 1.5)], acc);
            line(&[(9.5, 1.5), (14.5, 1.5), (14.5, 6.5)], acc);
        }
        AiAction::RemoveBackground => {
            for i in 0..4 {
                for j in 0..4 {
                    if (i + j) % 2 == 0 {
                        p.rect_filled(
                            Rect::from_min_size(
                                at(1.0 + i as f32 * 3.5, 1.0 + j as f32 * 3.5),
                                vec2(3.5, 3.5) * k,
                            ),
                            0.0,
                            Tokens::icon_off(),
                        );
                    }
                }
            }
            p.circle_filled(at(8.0, 6.0), 2.6 * k, acc);
            p.add(epaint::PathShape::convex_polygon(
                vec![at(3.5, 15.0), at(5.0, 10.0), at(11.0, 10.0), at(12.5, 15.0)],
                acc,
                Stroke::NONE,
            ));
        }
        AiAction::ReplaceBackground => {
            frame(1.0, 1.0, 15.0, 15.0, ink);
            line(&[(1.5, 11.0), (5.0, 7.0), (9.0, 10.0), (14.5, 5.0)], ink);
            p.circle_filled(at(8.0, 7.0), 2.4 * k, acc);
            p.add(epaint::PathShape::convex_polygon(
                vec![at(4.0, 15.0), at(5.5, 11.0), at(10.5, 11.0), at(12.0, 15.0)],
                acc,
                Stroke::NONE,
            ));
        }
        AiAction::Relight => {
            p.circle_stroke(at(8.0, 8.0), 3.5 * k, s(acc));
            for i in 0..8 {
                let ang = i as f32 * std::f32::consts::FRAC_PI_4;
                let (c, sn) = (ang.cos(), ang.sin());
                line(
                    &[
                        (8.0 + 5.2 * c, 8.0 + 5.2 * sn),
                        (8.0 + 7.0 * c, 8.0 + 7.0 * sn),
                    ],
                    ink,
                );
            }
        }
        AiAction::Vectorize => {
            for i in 0..3 {
                for j in 0..3 {
                    p.rect_filled(
                        Rect::from_min_size(
                            at(1.0 + i as f32 * 2.4, 6.0 + j as f32 * 2.4),
                            vec2(2.0, 2.0) * k,
                        ),
                        0.0,
                        ink,
                    );
                }
            }
            line(&[(9.5, 14.0), (12.0, 3.0), (15.0, 14.0)], acc);
        }
    }
}
