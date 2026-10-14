//! The Text docker (Ctrl+T): Character, Paragraph and Frame sections,
//! each under a header bar with a triangle and opened or closed by the
//! buttons at the top (the sections closed are kept in the preferences).
//!
//! - Character: the font list, the size list, bold, italic and underline;
//!   tracking, the characters' offsets and angle, strikethrough and the
//!   OpenType features.
//! - Paragraph: the alignment buttons; line, character and word spacing,
//!   space before and after paragraphs, first-line, left and right
//!   indents.
//! - Frame (paragraph text): columns and the gutter between them, where
//!   the text sits from top to bottom, bullets and drop caps (their
//!   dialogs), hyphenation and fitting the text to the frame.
//!
//! The font, size, style and alignment become those of new text too; the
//! rest changes the first selected text.

use crate::app::App;
use crate::i18n::tr;
use crate::theme::Tokens;
use crate::ui::field::NumField;
use crate::ui::propbar::{pic_button, Pic};
use egui::{Color32, Rect, Sense, Stroke, Ui, Vec2};
use tracedraw_core::{ParagraphStyle, ShapeKind, TextAlign, VerticalAlign};

/// The sections, by the names kept in the preferences.
const SECTIONS: [(&str, &str); 3] = [
    ("character", "docker.character"),
    ("paragraph", "docker.paragraph"),
    ("frame", "docker.frame"),
];

fn is_open(app: &App, id: &str) -> bool {
    !app.settings.text_sections_closed.iter().any(|c| c == id)
}

fn toggle(app: &mut App, id: &str) {
    let closed = &mut app.settings.text_sections_closed;
    if closed.iter().any(|c| c == id) {
        closed.retain(|c| c != id);
    } else {
        closed.push(id.to_string());
    }
}

/// The picture of a section's button.
fn section_picture(p: &egui::Painter, r: Rect, id: &str) {
    match id {
        "character" => {
            p.text(
                r.center(),
                egui::Align2::CENTER_CENTER,
                "A",
                crate::theme::bold(15.0),
                Tokens::icon(),
            );
        }
        "paragraph" => {
            p.text(
                r.center(),
                egui::Align2::CENTER_CENTER,
                "\u{00B6}",
                egui::FontId::proportional(16.0),
                Tokens::icon(),
            );
        }
        _ => {
            let f = r.shrink(1.5);
            p.rect_stroke(
                f,
                0.0,
                Stroke::new(1.2, Tokens::icon()),
                egui::StrokeKind::Inside,
            );
            for k in 0..3 {
                let y = f.top() + 4.0 + k as f32 * 3.5;
                p.line_segment(
                    [
                        egui::pos2(f.left() + 3.0, y),
                        egui::pos2(f.right() - 3.0, y),
                    ],
                    Stroke::new(1.0, Tokens::icon()),
                );
            }
        }
    }
}

/// A section's header bar; a click opens or closes it. True when open.
fn header(app: &mut App, ui: &mut Ui, id: &str, title: &str) -> bool {
    let open = is_open(app, id);
    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 24.0), Sense::click());
    let fill = if resp.hovered() {
        Tokens::tool_hover()
    } else {
        crate::theme::Tokens::control_fill()
    };
    ui.painter().rect_filled(rect, 2.0, fill);
    let c = egui::pos2(rect.left() + 10.0, rect.center().y);
    let pts = if open {
        vec![
            c + Vec2::new(-4.0, -2.0),
            c + Vec2::new(4.0, -2.0),
            c + Vec2::new(0.0, 3.0),
        ]
    } else {
        vec![
            c + Vec2::new(-2.0, -4.0),
            c + Vec2::new(3.0, 0.0),
            c + Vec2::new(-2.0, 4.0),
        ]
    };
    ui.painter().add(egui::epaint::PathShape::convex_polygon(
        pts,
        Tokens::icon(),
        Stroke::NONE,
    ));
    ui.painter().text(
        egui::pos2(rect.left() + 20.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        title,
        crate::theme::bold(12.5),
        Tokens::text(),
    );
    if resp.clicked() {
        toggle(app, id);
        return !open;
    }
    open
}

/// A field's label in the grids, without the colon some texts end with,
/// so every label of a section reads alike.
fn label(ui: &mut Ui, key: &str) {
    let text = tr(key);
    ui.label(text.trim_end_matches([':', '\u{FF1A}', ' ']));
}

/// A distance field in the ruler unit, 90 points wide; true when changed.
fn length(ui: &mut Ui, app: &App, mm: &mut f64) -> bool {
    let u = app.units;
    let unit = u.mm();
    let mut v = *mm / unit;
    let d = app.settings.precision.min(6) as usize;
    let r = ui.add_sized(
        [90.0, 22.0],
        NumField::new(&mut v)
            .fixed_decimals(d.min(3))
            .max_decimals(d)
            .suffix(format!(" {}", u.short()))
            .unit_mm(u.mm()),
    );
    if r.changed() && (v * unit - *mm).abs() > 1e-12 {
        *mm = v * unit;
        true
    } else {
        false
    }
}

/// A percentage field, 90 points wide, kept in `range`; true when changed.
fn percent(ui: &mut Ui, value: &mut f64, range: std::ops::RangeInclusive<f64>) -> bool {
    let before = *value;
    ui.add_sized(
        [90.0, 22.0],
        NumField::new(value)
            .range(range)
            .max_decimals(1)
            .suffix(" %"),
    );
    (*value - before).abs() > 1e-12
}

/// The picture of a vertical alignment: a frame with three lines at its
/// top, middle or bottom.
fn paint_vertical_align(p: &egui::Painter, r: Rect, v: VerticalAlign, color: Color32) {
    let f = r.shrink(1.0);
    p.rect_stroke(f, 0.0, Stroke::new(1.0, color), egui::StrokeKind::Inside);
    let block = 7.0;
    let top = match v {
        VerticalAlign::Top => f.top() + 2.5,
        VerticalAlign::Center => f.center().y - block / 2.0,
        VerticalAlign::Bottom => f.bottom() - 2.5 - block,
    };
    for k in 0..3 {
        let y = top + k as f32 * 3.0 + 0.5;
        p.line_segment(
            [
                egui::pos2(f.left() + 3.0, y),
                egui::pos2(f.right() - 3.0, y),
            ],
            Stroke::new(1.3, color),
        );
    }
}

/// Draw the docker.
pub fn text_docker(app: &mut App, ui: &mut Ui) {
    let first = app
        .selected_shapes()
        .into_iter()
        .find(|s| matches!(s.kind, ShapeKind::Text { .. }));
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for (id, key) in SECTIONS {
            let open = is_open(app, id);
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(28.0, 26.0), Sense::click());
            if open {
                ui.painter().rect_filled(rect, 2.0, Tokens::tool_active());
            } else if resp.hovered() {
                ui.painter().rect_filled(rect, 2.0, Tokens::tool_hover());
            }
            section_picture(
                ui.painter(),
                Rect::from_center_size(rect.center(), Vec2::splat(18.0)),
                id,
            );
            if resp.on_hover_text(tr(key)).clicked() {
                toggle(app, id);
            }
        }
    });
    ui.add_space(2.0);
    if header(app, ui, "character", &tr("docker.character")) {
        ui.add_space(4.0);
        character(app, ui, first.as_ref());
        ui.add_space(6.0);
    }
    if header(app, ui, "paragraph", &tr("docker.paragraph")) {
        ui.add_space(4.0);
        paragraph(app, ui, first.as_ref());
        ui.add_space(6.0);
    }
    if header(app, ui, "frame", &tr("docker.frame")) {
        ui.add_space(4.0);
        frame(app, ui, first.as_ref());
        ui.add_space(6.0);
    }
    if first.is_none() {
        ui.label(
            egui::RichText::new(tr("docker.text_hint"))
                .color(Tokens::text_dim())
                .size(11.0),
        );
    }
}

fn character(app: &mut App, ui: &mut Ui, first: Option<&tracedraw_core::Shape>) {
    let mut changed = crate::ui::font_list::text_font_list(app, ui, "docker_text_font", 220.0);
    ui.horizontal(|ui| {
        changed |= crate::ui::toolbar::text_size_list(app, ui);
        for (on, label, key) in [
            (
                app.text_bold,
                egui::RichText::new("B").strong(),
                "toolbar.bold",
            ),
            (
                app.text_italic,
                egui::RichText::new("I").italics(),
                "toolbar.italic",
            ),
            (
                app.text_underline,
                egui::RichText::new("U").underline(),
                "toolbar.underline",
            ),
        ] {
            if ui
                .add_sized([24.0, 22.0], egui::Button::selectable(on, label.size(14.0)))
                .on_hover_text(tr(key))
                .clicked()
            {
                match key {
                    "toolbar.bold" => app.text_bold = !app.text_bold,
                    "toolbar.italic" => app.text_italic = !app.text_italic,
                    _ => app.text_underline = !app.text_underline,
                }
                changed = true;
            }
        }
    });
    if changed {
        app.apply_text_style();
    }
    let Some(t) = first else {
        return;
    };
    let ShapeKind::Text { spans, .. } = &t.kind else {
        return;
    };
    let mut sp = spans
        .first()
        .cloned()
        .unwrap_or_else(|| tracedraw_core::TextSpan::new("", &app.text_font, app.text_size_pt));
    let mut sc = false;
    egui::Grid::new("text_character")
        .num_columns(2)
        .spacing([6.0, 4.0])
        .show(ui, |ui| {
            label(ui, "docker.tracking");
            sc |= percent(ui, &mut sp.tracking_pct, -50.0..=200.0);
            ui.end_row();
        });
    // Character offsets and angle: the selected characters while editing,
    // else every character.
    ui.horizontal_wrapped(|ui| {
        crate::ui::propbar::char_offset_part(app, ui);
    });
    sc |= ui
        .checkbox(&mut sp.strikethrough, tr("docker.strikethrough"))
        .changed();
    // Wrapped, so the feature buttons never widen the docker.
    ui.horizontal_wrapped(|ui| {
        ui.label(tr("docker.opentype"));
        for (tag, k) in [
            ("liga", "docker.ligatures"),
            ("smcp", "docker.small_caps"),
            ("onum", "docker.oldstyle_nums"),
            ("frac", "docker.fractions"),
            ("swsh", "docker.swash"),
        ] {
            let on = sp.features.iter().any(|f| f == tag);
            if ui.selectable_label(on, tr(k)).clicked() {
                if on {
                    sp.features.retain(|f| f != tag);
                } else {
                    sp.features.push(tag.to_string());
                }
                sc = true;
            }
        }
    });
    if sc {
        let new_spans: Vec<_> = spans
            .iter()
            .map(|s| tracedraw_core::TextSpan {
                tracking_pct: sp.tracking_pct,
                strikethrough: sp.strikethrough,
                features: sp.features.clone(),
                ..s.clone()
            })
            .collect();
        app.set_text_spans(t.id, new_spans);
    }
}

fn paragraph(app: &mut App, ui: &mut Ui, first: Option<&tracedraw_core::Shape>) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for (a, pic, key) in [
            (TextAlign::Left, Pic::TextLeft, "docker.align_left_short"),
            (
                TextAlign::Center,
                Pic::TextCenter,
                "docker.align_center_short",
            ),
            (TextAlign::Right, Pic::TextRight, "docker.align_right_short"),
            (
                TextAlign::Justify,
                Pic::TextJustify,
                "docker.align_justify_short",
            ),
        ] {
            if pic_button(ui, pic, &tr(key), true, app.text_align == a).clicked() {
                app.text_align = a;
                app.apply_text_style();
            }
        }
    });
    let Some(t) = first else {
        return;
    };
    let ShapeKind::Text { para, .. } = &t.kind else {
        return;
    };
    let mut p: ParagraphStyle = para.clone();
    let mut pc = false;
    egui::Grid::new("text_paragraph")
        .num_columns(2)
        .spacing([6.0, 4.0])
        .show(ui, |ui| {
            label(ui, "docker.leading");
            pc |= percent(ui, &mut p.leading_pct, 0.0..=2000.0);
            ui.end_row();
            label(ui, "docker.char_spacing");
            pc |= percent(ui, &mut p.char_spacing_pct, -100.0..=2000.0);
            ui.end_row();
            label(ui, "docker.word_spacing");
            pc |= percent(ui, &mut p.word_spacing_pct, 0.0..=2000.0);
            ui.end_row();
            for (k, v) in [
                ("docker.space_before", &mut p.space_before),
                ("docker.space_after", &mut p.space_after),
                ("docker.first_line_indent", &mut p.first_line_indent),
                ("docker.left_indent", &mut p.left_indent),
                ("docker.right_indent", &mut p.right_indent),
            ] {
                label(ui, k);
                pc |= length(ui, app, v);
                ui.end_row();
            }
        });
    if pc {
        app.set_paragraph_style(t.id, p);
    }
}

fn frame(app: &mut App, ui: &mut Ui, first: Option<&tracedraw_core::Shape>) {
    let para_text = first.and_then(|t| match &t.kind {
        ShapeKind::Text {
            para,
            frame: Some(_),
            ..
        } => Some((t.id, para.clone())),
        _ => None,
    });
    let Some((id, mut p)) = para_text else {
        ui.label(
            egui::RichText::new(tr("docker.frame_hint"))
                .color(Tokens::text_dim())
                .size(11.0),
        );
        return;
    };
    let mut pc = false;
    egui::Grid::new("text_frame")
        .num_columns(2)
        .spacing([6.0, 4.0])
        .show(ui, |ui| {
            label(ui, "dialog.columns");
            let before = p.columns;
            ui.add_sized([90.0, 22.0], NumField::new(&mut p.columns).range(1..=8));
            pc |= p.columns != before;
            ui.end_row();
            label(ui, "docker.gutter");
            pc |= length(ui, app, &mut p.gutter);
            ui.end_row();
        });
    ui.horizontal(|ui| {
        ui.label(tr("docker.vertical_align"));
        ui.spacing_mut().item_spacing.x = 2.0;
        for (v, key) in [
            (VerticalAlign::Top, "docker.valign_top"),
            (VerticalAlign::Center, "docker.valign_center"),
            (VerticalAlign::Bottom, "docker.valign_bottom"),
        ] {
            let on = p.vertical_align == v;
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(28.0, 26.0), Sense::click());
            if on {
                ui.painter().rect_filled(rect, 2.0, Tokens::tool_active());
            } else if resp.hovered() {
                ui.painter().rect_filled(rect, 2.0, Tokens::tool_hover());
            }
            paint_vertical_align(
                ui.painter(),
                Rect::from_center_size(rect.center(), Vec2::splat(18.0)),
                v,
                Tokens::icon(),
            );
            if resp.on_hover_text(tr(key)).clicked() && !on {
                p.vertical_align = v;
                pc = true;
            }
        }
    });
    ui.horizontal(|ui| {
        if pic_button(ui, Pic::Bullets, &tr("menu.text.bullets"), true, p.bullets).clicked() {
            app.dialog = crate::ui::dialogs::Dialog::TextBullets;
        }
        if pic_button(
            ui,
            Pic::DropCap,
            &tr("menu.text.drop_cap"),
            true,
            p.drop_cap_lines > 1,
        )
        .clicked()
        {
            app.dialog = crate::ui::dialogs::Dialog::TextDropCap;
        }
    });
    pc |= ui
        .checkbox(&mut p.hyphenate, tr("docker.hyphenate"))
        .changed();
    pc |= ui
        .checkbox(&mut p.fit_to_frame, tr("menu.text.frame_fit_text"))
        .changed();
    if pc {
        app.set_paragraph_style(id, p);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::{Point, Size, TextSpan};

    fn draw(app: &mut App) {
        let ctx = crate::theme::ui_context();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(360.0, 900.0),
            )),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| text_docker(app, ui));
        out.textures_delta.clear();
    }

    #[test]
    fn the_sections_open_close_and_draw_for_any_text() {
        let mut app = App::headless();
        draw(&mut app);
        for frame in [None, Some(Size::new(60.0, 40.0))] {
            let id = app
                .new_shape(ShapeKind::Text {
                    spans: vec![TextSpan::new("Text", "Arial", 24.0)],
                    origin: Point::new(10.0, 10.0),
                    frame,
                    align: TextAlign::Left,
                    para: ParagraphStyle::default(),
                    on_path: None,
                })
                .expect("text");
            app.select(vec![id]);
            draw(&mut app);
        }
        for (id, _) in SECTIONS {
            assert!(is_open(&app, id));
            toggle(&mut app, id);
            assert!(!is_open(&app, id));
        }
        draw(&mut app);
        assert_eq!(app.settings.text_sections_closed.len(), 3);
    }

    /// The vertical alignment a paragraph text takes is kept with it and
    /// moves its lines.
    #[test]
    fn vertical_alignment_is_kept_with_the_text() {
        let mut app = App::headless();
        let id = app
            .new_shape(ShapeKind::Text {
                spans: vec![TextSpan::new("Text", "Arial", 24.0)],
                origin: Point::new(10.0, 10.0),
                frame: Some(Size::new(60.0, 80.0)),
                align: TextAlign::Left,
                para: ParagraphStyle::default(),
                on_path: None,
            })
            .expect("text");
        let p = ParagraphStyle {
            vertical_align: VerticalAlign::Bottom,
            ..ParagraphStyle::default()
        };
        app.set_paragraph_style(id, p);
        let s = app.doc().find_shape(id).cloned().expect("text");
        assert!(matches!(
            s.kind,
            ShapeKind::Text { ref para, .. } if para.vertical_align == VerticalAlign::Bottom
        ));
        // Kept in the native format.
        let json = serde_json::to_string(&s).expect("json");
        assert!(json.contains("\"vertical_align\":\"bottom\""), "{json}");
    }
}
