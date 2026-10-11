//! Help > About TraceDraw: a deep blue banner with the application icon,
//! the name, the tagline and the version over a few faint curves with
//! their nodes and handles, then a short description, the facts (author,
//! license, source code, how it is built) as a two-column list, the
//! copyright line and a Close button.

use crate::app::App;
use crate::i18n::{tr, trf};
use crate::theme::Tokens;
use egui::epaint::CubicBezierShape;
use egui::{
    Color32, Context, CornerRadius, FontId, Painter, Pos2, Rect, RichText, Sense, Stroke,
    TextureHandle, Ui, Vec2,
};

/// Banner height in points, including the body margin it covers.
const BANNER_H: f32 = 136.0;
/// The body's top margin, which the banner covers so it meets the title bar.
const BODY_TOP: f32 = 12.0;
/// The body's side margin plus the white edge, which the banner covers so
/// it meets the window border.
const BLEED: f32 = 14.0 + crate::ui::chrome::EDGE;
const BANNER_TOP_LEFT: Color32 = Color32::from_rgb(0x24, 0x44, 0x75);
const BANNER_TOP_RIGHT: Color32 = Color32::from_rgb(0x1A, 0x31, 0x58);
const BANNER_BOTTOM: Color32 = Color32::from_rgb(0x0E, 0x19, 0x2D);
const CURVE: Color32 = Color32::from_rgb(0x5C, 0xAE, 0xFF);
const NAME_ACCENT: Color32 = Color32::from_rgb(0x7F, 0xC3, 0xFF);
const TAGLINE: Color32 = Color32::from_rgb(0xC9, 0xD7, 0xEC);

/// The 128 px application icon, decoded once per session.
fn icon(ctx: &Context) -> Option<TextureHandle> {
    let id = egui::Id::new("about_icon_texture");
    if let Some(t) = ctx.data(|d| d.get_temp::<TextureHandle>(id)) {
        return Some(t);
    }
    let color = icon_image()?;
    let tex = ctx.load_texture("about_icon", color, egui::TextureOptions::LINEAR);
    ctx.data_mut(|d| d.insert_temp(id, tex.clone()));
    Some(tex)
}

/// The embedded 128 px icon as an egui image.
fn icon_image() -> Option<egui::ColorImage> {
    let bytes = include_bytes!("../../../../assets/icon/tracedraw-128.png");
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let (w, h) = (img.width() as usize, img.height() as usize);
    Some(egui::ColorImage::from_rgba_unmultiplied(
        [w, h],
        img.as_raw(),
    ))
}

/// The point at `t` (0 to 1) on the cubic Bezier `p`.
fn cubic_point(p: &[Pos2; 4], t: f32) -> Pos2 {
    let u = 1.0 - t;
    let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
    Pos2::new(
        a * p[0].x + b * p[1].x + c * p[2].x + d * p[3].x,
        a * p[0].y + b * p[1].y + c * p[2].y + d * p[3].y,
    )
}

/// The tangent direction at `t` on the cubic Bezier `p` (not normalised).
fn cubic_tangent(p: &[Pos2; 4], t: f32) -> Vec2 {
    let u = 1.0 - t;
    (p[1] - p[0]) * (3.0 * u * u) + (p[2] - p[1]) * (6.0 * u * t) + (p[3] - p[2]) * (3.0 * t * t)
}

/// Fill `rect` with the banner's diagonal gradient.
fn gradient(painter: &Painter, rect: Rect) {
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(rect.left_top(), BANNER_TOP_LEFT);
    mesh.colored_vertex(rect.right_top(), BANNER_TOP_RIGHT);
    mesh.colored_vertex(rect.right_bottom(), BANNER_BOTTOM);
    mesh.colored_vertex(rect.left_bottom(), BANNER_BOTTOM);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(mesh);
}

/// A selected node on `curve` at `t`: the square node, its two handles
/// along the tangent and their round ends.
fn node(painter: &Painter, curve: &[Pos2; 4], t: f32) {
    let at = cubic_point(curve, t);
    let dir = cubic_tangent(curve, t).normalized() * 30.0;
    let handle = Stroke::new(1.0, Color32::from_white_alpha(110));
    for end in [at - dir, at + dir] {
        painter.line_segment([at, end], handle);
        painter.circle(end, 3.0, CURVE, Stroke::new(1.0, Color32::WHITE));
    }
    let square = Rect::from_center_size(at, Vec2::splat(8.0));
    painter.rect(
        square,
        CornerRadius::ZERO,
        BANNER_TOP_RIGHT,
        Stroke::new(1.3, Color32::WHITE),
        egui::StrokeKind::Inside,
    );
}

/// The faint curves on the right of the banner, one of them with a
/// selected node, like a path being edited.
fn curves(painter: &Painter, r: Rect) {
    let at = |x: f32, y: f32| Pos2::new(r.left() + x * r.width(), r.top() + y * r.height());
    let main = [
        at(0.60, 1.08),
        at(0.73, 0.02),
        at(0.86, 1.12),
        at(1.03, 0.18),
    ];
    let echoes = [
        (
            [
                at(0.68, 1.08),
                at(0.79, 0.30),
                at(0.92, 1.15),
                at(1.03, 0.48),
            ],
            60,
        ),
        (
            [
                at(0.54, -0.08),
                at(0.70, 0.62),
                at(0.83, -0.25),
                at(1.03, -0.02),
            ],
            40,
        ),
    ];
    for (points, alpha) in echoes {
        painter.add(CubicBezierShape::from_points_stroke(
            points,
            false,
            Color32::TRANSPARENT,
            Stroke::new(1.2, CURVE.gamma_multiply_u8(alpha)),
        ));
    }
    painter.add(CubicBezierShape::from_points_stroke(
        main,
        false,
        Color32::TRANSPARENT,
        Stroke::new(2.0, CURVE.gamma_multiply_u8(190)),
    ));
    node(painter, &main, 0.5);
}

/// The banner across the top of the dialog body.
fn banner(ui: &mut Ui, icon: Option<&TextureHandle>) {
    let (slot, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), BANNER_H - BODY_TOP),
        Sense::hover(),
    );
    let rect = Rect::from_min_max(
        Pos2::new(slot.left() - BLEED, slot.top() - BODY_TOP),
        Pos2::new(slot.right() + BLEED, slot.bottom()),
    );
    let painter = ui.ctx().layer_painter(ui.layer_id()).with_clip_rect(rect);
    gradient(&painter, rect);
    curves(&painter, rect);

    let icon_size = 76.0;
    let icon_rect = Rect::from_min_size(
        Pos2::new(rect.left() + 26.0, rect.center().y - icon_size / 2.0),
        Vec2::splat(icon_size),
    );
    if let Some(tex) = icon {
        painter.image(
            tex.id(),
            icon_rect,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );
    }

    let mut name = egui::text::LayoutJob::default();
    let name_font = crate::theme::bold(29.0);
    for (part, color) in [("Trace", Color32::WHITE), ("Draw", NAME_ACCENT)] {
        name.append(
            part,
            0.0,
            egui::TextFormat {
                font_id: name_font.clone(),
                color,
                ..Default::default()
            },
        );
    }
    let name = painter.layout_job(name);
    let tagline = painter.layout_no_wrap(
        tr("dialog.about_tagline"),
        FontId::proportional(14.5),
        TAGLINE,
    );
    let version = painter.layout_no_wrap(
        format!("{} {}", tr("dialog.version"), env!("CARGO_PKG_VERSION")),
        FontId::proportional(11.5),
        Color32::from_rgb(0xE4, 0xEC, 0xF8),
    );
    let chip = version.size() + Vec2::new(16.0, 6.0);
    let block_h = name.size().y + 2.0 + tagline.size().y + 10.0 + chip.y;
    let left = icon_rect.right() + 20.0;
    let mut y = rect.center().y - block_h / 2.0;
    painter.galley(Pos2::new(left, y), name.clone(), Color32::WHITE);
    y += name.size().y + 2.0;
    painter.galley(Pos2::new(left + 1.0, y), tagline.clone(), TAGLINE);
    y += tagline.size().y + 10.0;
    let chip_rect = Rect::from_min_size(Pos2::new(left + 1.0, y), chip);
    painter.rect(
        chip_rect,
        CornerRadius::same(10),
        Color32::from_white_alpha(26),
        Stroke::new(1.0, Color32::from_white_alpha(60)),
        egui::StrokeKind::Inside,
    );
    painter.galley(chip_rect.min + Vec2::new(8.0, 3.0), version, Color32::WHITE);
}

/// One row of the facts list: a dim label and its value.
fn fact(ui: &mut Ui, label: &str, value: impl FnOnce(&mut Ui)) {
    ui.label(RichText::new(label).color(Tokens::TEXT_DIM));
    value(ui);
    ui.end_row();
}

/// Draw the About box; sets `close` when it should close.
pub fn about_dialog(app: &mut App, ctx: &Context, close: &mut bool) {
    let icon = icon(ctx);
    crate::ui::chrome::window(tr("dialog.about")).show(ctx, |ui| {
        ui.set_width(480.0);
        ui.spacing_mut().item_spacing.y = 4.0;
        banner(ui, icon.as_ref());
        ui.add_space(16.0);
        ui.add(
            egui::Label::new(
                RichText::new(tr("dialog.about_text"))
                    .size(13.5)
                    .line_height(Some(20.0)),
            )
            .wrap(),
        );
        ui.add_space(12.0);
        let (line, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
        ui.painter().hline(
            line.x_range(),
            line.center().y,
            Stroke::new(1.0, Color32::from_gray(0xD6)),
        );
        ui.add_space(12.0);
        egui::Grid::new("about_facts")
            .num_columns(2)
            .spacing([22.0, 10.0])
            .show(ui, |ui| {
                fact(ui, &tr("dialog.about_author"), |ui| {
                    ui.label(env!("CARGO_PKG_AUTHORS"));
                });
                fact(ui, &tr("dialog.about_license"), |ui| {
                    ui.label(tr("dialog.about_license_value"));
                });
                fact(ui, &tr("dialog.about_source_label"), |ui| {
                    let repo = env!("CARGO_PKG_REPOSITORY");
                    if ui.link(repo).clicked() {
                        app.open_url(repo);
                    }
                });
                fact(ui, &tr("dialog.about_built"), |ui| {
                    ui.add(egui::Label::new(tr("dialog.about_ai")).wrap());
                });
            });
        ui.add_space(14.0);
        ui.label(
            RichText::new(trf(
                "dialog.about_copyright",
                &[("name", env!("CARGO_PKG_AUTHORS"))],
            ))
            .size(12.0)
            .color(Tokens::TEXT_DIM),
        );
        if crate::ui::chrome::button_row(ui, &[&tr("dialog.close")], false).is_some() {
            *close = true;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_embedded_icon_decodes() {
        let img = icon_image().expect("icon decodes");
        assert_eq!(img.size, [128, 128]);
    }

    #[test]
    fn cubic_helpers_follow_the_curve() {
        let p = [
            Pos2::new(0.0, 0.0),
            Pos2::new(0.0, 10.0),
            Pos2::new(10.0, 10.0),
            Pos2::new(10.0, 0.0),
        ];
        assert_eq!(cubic_point(&p, 0.0), p[0]);
        assert_eq!(cubic_point(&p, 1.0), p[3]);
        let mid = cubic_point(&p, 0.5);
        assert!((mid.x - 5.0).abs() < 1e-4 && (mid.y - 7.5).abs() < 1e-4);
        // At the middle of this symmetric arch the tangent is horizontal.
        let t = cubic_tangent(&p, 0.5);
        assert!(t.y.abs() < 1e-4 && t.x > 0.0);
    }
}
