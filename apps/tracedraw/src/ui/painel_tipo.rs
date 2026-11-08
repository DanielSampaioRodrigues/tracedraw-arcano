//! Arcano edition: the Character and Paragraph panels of the Page
//! workspace, laid out as page layout work wants them: a small picture
//! before every field, two fields to a row, the alignments and the
//! character options as rows of buttons.
//!
//! - Character: font and style across; size and leading (in points, the
//!   paragraph's line spacing), tracking and baseline shift, character
//!   angle and horizontal offset two to a row; buttons for small caps,
//!   superscript, subscript, underline, strikethrough and ligatures (the
//!   first three through the font's OpenType features); the other
//!   OpenType features; the fill, background and outline rows.
//! - Paragraph: the six alignments; left and right indents, first line
//!   indent and drop cap lines, space before and after, two to a row;
//!   hyphenation, balance lines and the baseline grid; Bullets, Tabs and
//!   Drop Cap; the word and letter spacing; widows, orphans and keep with
//!   next.
//!
//! Every field edits what the Text panel's does (the characters chosen
//! while editing, else the selected text objects).
//!
//! See docs/behavior/personas.md.

use crate::app::App;
use crate::i18n::tr;
use crate::theme::Tokens;
use crate::ui::field::NumField;
use crate::ui::propbar::{pic_button, Pic};
use crate::ui::text_docker as td;
use crate::ui::tips::RichTip;
use egui::Ui;
use tracedraw_core::{LeadingUnit, ParagraphFormat, ShapeKind, TextAlign};

/// Width of a field in the two-column grid.
const FIELD_W: f32 = 82.0;

/// The picture before a field: a few letters drawn small and dim, with
/// the field's name as the tip.
pub(crate) fn glyph_icon(ui: &mut Ui, text: &str, key: &str) {
    let (r, resp) = ui.allocate_exact_size(egui::vec2(22.0, 22.0), egui::Sense::hover());
    ui.painter().text(
        r.center(),
        egui::Align2::CENTER_CENTER,
        text,
        egui::FontId::proportional(11.5),
        Tokens::icon(),
    );
    resp.rich_tip(tr(key));
}

/// A paragraph line picture of the Text panel before a field.
pub(crate) fn para_icon(ui: &mut Ui, icon: td::ParaIcon, key: &str) {
    let (r, resp) = ui.allocate_exact_size(egui::vec2(22.0, 22.0), egui::Sense::hover());
    for shape in td::para_icon_shapes(icon, r, Tokens::icon()) {
        ui.painter().add(shape);
    }
    resp.rich_tip(tr(key));
}

/// A number field of the grid; true when changed.
pub(crate) fn num(ui: &mut Ui, v: &mut f64, range: std::ops::RangeInclusive<f64>, suffix: &str) -> bool {
    let before = *v;
    ui.add_sized(
        [FIELD_W, 22.0],
        NumField::new(v)
            .range(range)
            .max_decimals(2)
            .suffix(suffix.to_string()),
    );
    (*v - before).abs() > 1e-12
}

/// A distance field of the grid in the ruler unit; true when changed.
pub(crate) fn dist(ui: &mut Ui, app: &App, mm: &mut f64) -> bool {
    let u = app.units;
    let mut v = *mm / u.mm();
    let before = v;
    ui.add_sized(
        [FIELD_W, 22.0],
        NumField::new(&mut v)
            .max_decimals(app.settings.precision.min(3) as usize)
            .suffix(format!(" {}", u.short()))
            .unit_mm(u.mm()),
    );
    if (v - before).abs() > 1e-12 {
        *mm = v * u.mm();
        true
    } else {
        false
    }
}

/// A toggle button of the row of character options, `text` drawn in it.
fn toggle(ui: &mut Ui, on: bool, text: egui::RichText, key: &str) -> bool {
    ui.add_sized([30.0, 24.0], egui::Button::selectable(on, text))
        .rich_tip(crate::ui::tips::pair(key, &format!("{key}_desc")))
        .clicked()
}

pub(crate) fn first_text(app: &App) -> Option<tracedraw_core::Shape> {
    app.active_cell_text_shape().or_else(|| {
        app.selected_shapes()
            .into_iter()
            .find(|s| matches!(s.kind, ShapeKind::Text { .. }))
    })
}

/// Change only `before` to `after` in every paragraph the edit reaches.
pub(crate) fn edit_format(app: &mut App, before: &tracedraw_core::ParagraphStyle, after: &tracedraw_core::ParagraphStyle) {
    let before = ParagraphFormat::of(TextAlign::Left, before);
    let after = ParagraphFormat::of(TextAlign::Left, after);
    td::edit_paragraphs(app, move |q| {
        let mut f = ParagraphFormat::of(TextAlign::Left, q);
        f.take_changes(&before, &after);
        f.apply_to(q);
    });
}

/// The Character panel.
pub fn character_panel(app: &mut App, ui: &mut Ui) {
    let first = first_text(app);
    let span = first.as_ref().and_then(|t| app.shown_span(t));
    let w = ui.available_width();
    let mut changed = false;
    changed |= crate::ui::tips::tip_over(ui, crate::ui::tips::keyed("toolbar.font"), |ui| {
        crate::ui::font_list::text_font_list(app, ui, "tipo_font", w.max(120.0))
    });
    ui.horizontal(|ui| {
        changed |= td::style_list(app, ui);
        td::font_axes_button(app, ui, "tipo");
    });
    let para = first.as_ref().and_then(|t| app.paragraph_style_of(t).map(|(_, p)| p));
    egui::Grid::new("tipo_char_grid")
        .num_columns(4)
        .spacing([2.0, 4.0])
        .show(ui, |ui| {
            glyph_icon(ui, "tT", "tipo.size");
            changed |= crate::ui::toolbar::text_size_list(app, ui);
            glyph_icon(ui, "A\u{2195}", "tipo.leading");
            // The paragraph's line spacing, shown and typed in points.
            match &para {
                Some(p) => {
                    let size = first.as_ref().map_or(12.0, td::t_size_pt);
                    let mut pt = td::convert_leading(p.leading_pct, p.leading_unit, LeadingUnit::Points, size);
                    if num(ui, &mut pt, 0.0..=2000.0, " pt") {
                        let mut q = p.clone();
                        q.leading_unit = LeadingUnit::Points;
                        q.leading_pct = pt;
                        edit_format(app, p, &q);
                    }
                }
                None => {
                    ui.add_enabled_ui(false, |ui| {
                        let mut z = 0.0;
                        num(ui, &mut z, 0.0..=0.0, " pt");
                    });
                }
            }
            ui.end_row();
            let enabled = span.is_some();
            let sp = span.clone().unwrap_or_else(|| tracedraw_core::TextSpan::new("", "", 12.0));
            glyph_icon(ui, "VA", "tipo.tracking");
            let mut v = sp.tracking_pct;
            ui.add_enabled_ui(enabled, |ui| {
                if num(ui, &mut v, -50.0..=200.0, " %") {
                    td::edit_spans(app, move |s| s.tracking_pct = v);
                }
            });
            glyph_icon(ui, "A\u{2191}", "tipo.baseline_shift");
            let mut b = sp.baseline_shift_pt;
            ui.add_enabled_ui(enabled, |ui| {
                if num(ui, &mut b, -500.0..=500.0, " pt") {
                    td::edit_spans(app, move |s| s.baseline_shift_pt = b);
                }
            });
            ui.end_row();
            glyph_icon(ui, "A\u{00B0}", "tipo.angle");
            let mut a = sp.angle_deg;
            ui.add_enabled_ui(enabled, |ui| {
                if num(ui, &mut a, -360.0..=360.0, "\u{00B0}") {
                    td::edit_spans(app, move |s| s.angle_deg = a);
                }
            });
            glyph_icon(ui, "\u{2194}A", "tipo.offset");
            let mut o = sp.offset_x_pct;
            ui.add_enabled_ui(enabled, |ui| {
                if num(ui, &mut o, -500.0..=500.0, " %") {
                    td::edit_spans(app, move |s| s.offset_x_pct = o);
                }
            });
            ui.end_row();
        });
    if changed {
        app.apply_text_style();
    }
    // The character options as buttons.
    let sp = span.clone();
    ui.add_space(2.0);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 3.0;
        let has = |tag: &str| sp.as_ref().is_some_and(|s| s.features.iter().any(|f| f == tag));
        let feature = |app: &mut App, tag: &'static str, on: bool, off: &'static [&'static str]| {
            td::edit_spans(app, move |s| {
                s.features.retain(|f| f != tag && !off.contains(&f.as_str()));
                if on {
                    s.features.push(tag.to_string());
                }
            });
        };
        ui.add_enabled_ui(sp.is_some(), |ui| {
            let on = has("smcp");
            if toggle(ui, on, egui::RichText::new("Tt").size(13.0), "tipo.small_caps") {
                feature(app, "smcp", !on, &[]);
            }
            let on = has("sups");
            if toggle(ui, on, egui::RichText::new("T\u{00B9}").size(13.0), "tipo.superscript") {
                feature(app, "sups", !on, &["subs"]);
            }
            let on = has("subs");
            if toggle(ui, on, egui::RichText::new("T\u{2081}").size(13.0), "tipo.subscript") {
                feature(app, "subs", !on, &["sups"]);
            }
        });
        let on = app.text_underline;
        if toggle(ui, on, egui::RichText::new("U").underline().size(13.0), "tipo.underline") {
            app.text_underline = !on;
            let u = !on;
            if sp.is_some() {
                td::edit_spans(app, move |s| s.underline = u);
            }
        }
        ui.add_enabled_ui(sp.is_some(), |ui| {
            let on = sp.as_ref().is_some_and(|s| s.strikethrough);
            if toggle(ui, on, egui::RichText::new("S").strikethrough().size(13.0), "tipo.strikethrough") {
                td::edit_spans(app, move |s| s.strikethrough = !on);
            }
            let on = has("liga");
            if toggle(ui, on, egui::RichText::new("fi").size(13.0), "tipo.ligatures") {
                feature(app, "liga", !on, &[]);
            }
        });
    });
    if let Some(s) = sp.as_ref() {
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new(tr("docker.opentype")).small().color(Tokens::text_dim()));
            for (tag, k) in [
                ("onum", "docker.oldstyle_nums"),
                ("frac", "docker.fractions"),
                ("swsh", "docker.swash"),
            ] {
                let on = s.features.iter().any(|f| f == tag);
                if ui.selectable_label(on, tr(k)).clicked() {
                    td::edit_spans(app, move |s| {
                        s.features.retain(|f| f != tag);
                        if !on {
                            s.features.push(tag.to_string());
                        }
                    });
                }
            }
        });
        td::font_axes(app, ui, s);
    }
    ui.separator();
    td::character_colours(app, ui, first.as_ref(), span.as_ref());
}

/// The Paragraph panel.
pub fn paragraph_panel(app: &mut App, ui: &mut Ui) {
    let first = first_text(app);
    let current = first
        .as_ref()
        .and_then(|t| app.paragraph_style_of(t))
        .map_or(app.text_align, |(a, _)| a);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for (a, pic, key) in [
            (TextAlign::Left, Pic::TextLeft, "docker.align_left_short"),
            (TextAlign::Center, Pic::TextCenter, "docker.align_center_short"),
            (TextAlign::Right, Pic::TextRight, "docker.align_right_short"),
            (TextAlign::Justify, Pic::TextJustify, "docker.align_justify_short"),
            (TextAlign::ForceJustify, Pic::TextForceJustify, "docker.align_force_justify"),
            (TextAlign::None, Pic::TextNone, "docker.align_none"),
        ] {
            if pic_button(ui, pic, &tr(key), true, current == a).clicked() {
                td::set_align(app, a);
            }
        }
    });
    let Some(t) = first else {
        return;
    };
    let Some((_, mut p)) = app.paragraph_style_of(&t) else {
        return;
    };
    let before = p.clone();
    ui.add_space(2.0);
    egui::Grid::new("tipo_para_grid")
        .num_columns(4)
        .spacing([2.0, 4.0])
        .show(ui, |ui| {
            para_icon(ui, td::ParaIcon::LeftIndent, "docker.left_indent");
            dist(ui, app, &mut p.left_indent);
            para_icon(ui, td::ParaIcon::RightIndent, "docker.right_indent");
            dist(ui, app, &mut p.right_indent);
            ui.end_row();
            para_icon(ui, td::ParaIcon::FirstLineIndent, "docker.first_line_indent");
            dist(ui, app, &mut p.first_line_indent);
            glyph_icon(ui, "A\u{00B6}", "tipo.drop_cap_lines");
            let mut lines = p.drop_cap_lines as f64;
            if num(ui, &mut lines, 0.0..=10.0, "") {
                // One line is no drop cap.
                p.drop_cap_lines = if lines < 2.0 { 0 } else { lines.round() as u32 };
            }
            ui.end_row();
            para_icon(ui, td::ParaIcon::SpaceBefore, "docker.space_before");
            dist(ui, app, &mut p.space_before);
            para_icon(ui, td::ParaIcon::SpaceAfter, "docker.space_after");
            dist(ui, app, &mut p.space_after);
            ui.end_row();
        });
    ui.checkbox(&mut p.hyphenate, tr("docker.hyphenate"))
        .rich_tip(crate::ui::tips::keyed("docker.hyphenate"));
    ui.checkbox(&mut p.balance_lines, tr("docker.balance_lines")).rich_tip(
        crate::ui::tips::pair("docker.balance_lines", "docker.balance_lines_desc"),
    );
    if p != before {
        edit_format(app, &before, &p);
    }
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 3.0;
        let framed = matches!(t.kind, ShapeKind::Text { frame: Some(_), .. });
        let on_grid = matches!(&t.kind, ShapeKind::Text { para, .. } if para.baseline_grid_mm > 0.0);
        ui.add_enabled_ui(framed, |ui| {
            if pic_button(ui, Pic::AlignBaseline, &tr("menu.text.align_to_baseline_grid"), true, on_grid)
                .clicked()
            {
                app.toggle_baseline_grid();
            }
        });
        if pic_button(ui, Pic::Bullets, &tr("menu.text.bullets"), true, before.bullets).clicked() {
            app.dialog = crate::ui::dialogs::Dialog::TextBullets;
        }
        if pic_button(ui, Pic::Tabs, &tr("menu.text.tabs"), true, !before.tabs.is_empty()).clicked() {
            app.dialog = crate::ui::dialogs::Dialog::TextTabs;
        }
        if pic_button(ui, Pic::DropCap, &tr("menu.text.drop_cap"), true, before.drop_cap_lines > 1)
            .clicked()
        {
            app.dialog = crate::ui::dialogs::Dialog::TextDropCap;
        }
    });
    let id = ui.make_persistent_id("tipo_justify");
    egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false)
        .show_header(ui, |ui| {
            ui.label(tr("tipo.spacing"));
        })
        .body(|ui| {
            let mut q = before.clone();
            egui::Grid::new("tipo_spacing")
                .num_columns(4)
                .spacing([2.0, 4.0])
                .show(ui, |ui| {
                    glyph_icon(ui, "a\u{2423}b", "docker.word_spacing");
                    num(ui, &mut q.word_spacing_pct, 0.0..=2000.0, " %");
                    glyph_icon(ui, "a b", "docker.char_spacing");
                    num(ui, &mut q.char_spacing_pct, -100.0..=2000.0, " %");
                    ui.end_row();
                });
            if q != before {
                edit_format(app, &before, &q);
            }
        });
    if matches!(t.kind, ShapeKind::Text { frame: Some(_), .. }) {
        ui.add_space(2.0);
        td::keep_rules(app, ui, t.id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::{Point, Size};
    use tracedraw_core::{ParagraphStyle, TextSpan};

    fn paragraph_text(app: &mut App) -> tracedraw_core::ShapeId {
        let id = app
            .new_shape(ShapeKind::Text {
                spans: vec![TextSpan::new("Um parÃ¡grafo.\nOutro.", "Sans", 12.0)],
                origin: Point::new(10.0, 10.0),
                frame: Some(Size::new(100.0, 50.0)),
                align: TextAlign::Left,
                para: ParagraphStyle::default(),
                on_path: None,
            })
            .unwrap();
        app.text_edit = None;
        app.select(vec![id]);
        id
    }

    /// Both panels draw with and without text selected, and keep within
    /// the width of a docked column.
    #[test]
    fn the_panels_draw_and_keep_their_width() {
        let mut app = App::headless();
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1400.0, 800.0));
        for with_text in [false, true] {
            if with_text {
                paragraph_text(&mut app);
            }
            for part in 0..2 {
                let mut widths = Vec::new();
                for _ in 0..3 {
                    let mut o = ctx.run_ui(
                        egui::RawInput {
                            screen_rect: Some(screen),
                            ..Default::default()
                        },
                        |ui| {
                            let r = egui::Panel::right("dock").default_size(290.0).show(ui, |ui| {
                                if part == 0 {
                                    character_panel(&mut app, ui)
                                } else {
                                    paragraph_panel(&mut app, ui)
                                }
                            });
                            widths.push(r.response.rect.width());
                        },
                    );
                    o.textures_delta.clear();
                }
                assert!(widths.iter().all(|w| *w < 330.0), "part {part}: {widths:?}");
            }
        }
    }

    /// The leading field types points into the paragraph's line spacing;
    /// a drop cap of one line is none.
    #[test]
    fn leading_in_points_and_drop_caps_reach_the_paragraphs() {
        let mut app = App::headless();
        let id = paragraph_text(&mut app);
        let t = app.doc().find_shape(id).unwrap().clone();
        let (_, p) = app.paragraph_style_of(&t).unwrap();
        let mut q = p.clone();
        q.leading_unit = LeadingUnit::Points;
        q.leading_pct = 17.0;
        q.drop_cap_lines = 3;
        edit_format(&mut app, &p, &q);
        let ShapeKind::Text { spans, align, para, .. } = &app.doc().find_shape(id).unwrap().kind else {
            unreachable!()
        };
        let f = tracedraw_core::paragraphs::paragraph_formats_of(spans, *align, para);
        assert!(f.iter().all(|f| f.leading_unit == LeadingUnit::Points && (f.leading_pct - 17.0).abs() < 1e-9));
        assert!(f.iter().all(|f| f.drop_cap_lines == 3));
    }

    /// The Balance Lines box of the panel reaches the paragraphs the
    /// selection covers, and switching it off takes it away again.
    #[test]
    fn balance_lines_reaches_the_paragraphs() {
        let mut app = App::headless();
        let id = paragraph_text(&mut app);
        let t = app.doc().find_shape(id).unwrap().clone();
        let (_, plain) = app.paragraph_style_of(&t).unwrap();
        let mut balanced = plain.clone();
        balanced.balance_lines = true;
        edit_format(&mut app, &plain, &balanced);
        let formats = |app: &App| {
            let ShapeKind::Text { spans, align, para, .. } =
                &app.doc().find_shape(id).unwrap().kind
            else {
                unreachable!()
            };
            tracedraw_core::paragraphs::paragraph_formats_of(spans, *align, para)
        };
        assert!(formats(&app).iter().all(|f| f.balance_lines));
        edit_format(&mut app, &balanced, &plain);
        assert!(formats(&app).iter().all(|f| !f.balance_lines));
    }
}
