//! The text rows of the property bar: the Character and the Paragraph
//! controls, in two rows with the Text tool and while a text is edited, so
//! the work of setting text happens at the top of the window and not only
//! in the panels. A text selected with the Pick tool keeps the bar's one
//! row, so selecting a text to move it does not change the room the
//! drawing has.
//!
//! Row 1 holds the characters' look: font, character style, size, the
//! variable font's axes, bold, italic and underline, tracking, line
//! spacing, the character offsets and angle, and the paragraphs' word and
//! letter spacing. Row 2 holds the paragraph and its frame: the six
//! alignments, the paragraph style, the indents and spaces, the columns,
//! hyphenation, balance lines, widows, orphans and keep with next, and the
//! Bullets, Tabs and Drop Cap dialogs.
//!
//! Every control edits what the Text docker's and the Paragraph panel's
//! does (the characters chosen while editing, else the selected text
//! objects), through the same commands. A text without a frame (artistic
//! text) has no columns and no keep rules: those are shown switched off.
//!
//! See docs/behavior/text.md, The property bar's text rows.

use crate::app::App;
use crate::i18n::tr;
use crate::tools::Tool;
use crate::ui::field::NumField;
use crate::ui::painel_tipo;
use crate::ui::propbar::{pic_button, Pic};
use crate::ui::text_docker as td;
use crate::ui::tips::{keyed, tip_over, RichTip};
use egui::Ui;
use tracedraw_core::text_styles::TextStyleKind;
use tracedraw_core::{LeadingUnit, ParagraphStyle, ShapeKind, TextAlign};

/// The room the two rows take across the bar: two rows of fields and
/// buttons, close together.
pub(crate) const ROWS_H: f32 = 56.0;

/// Whether the property bar shows the two text rows: the Text tool, the
/// caret in a text, or a table cell being edited. Text selected with the
/// Pick tool keeps the bar's one row, so selecting a text to move it does
/// not change the room the drawing has.
pub(crate) fn two_rows(app: &App) -> bool {
    matches!(app.tool, Tool::Text) || app.text_edit.is_some() || app.table_edit.is_some()
}

/// A number field of the row, under `key`'s tip; true when changed.
fn field(
    ui: &mut Ui,
    key: &str,
    v: &mut f64,
    range: std::ops::RangeInclusive<f64>,
    suffix: &str,
) -> bool {
    let before = *v;
    tip_over(ui, tr(key), |ui| {
        ui.add_sized(
            [64.0, 22.0],
            NumField::new(v)
                .range(range)
                .max_decimals(2)
                .suffix(suffix.to_string()),
        );
    });
    (*v - before).abs() > 1e-12
}

/// A distance field of the row in the ruler unit, under `key`'s tip; true
/// when changed.
fn dist(ui: &mut Ui, app: &App, key: &str, mm: &mut f64) -> bool {
    let u = app.units;
    let mut v = *mm / u.mm();
    let before = v;
    tip_over(ui, tr(key), |ui| {
        ui.add_sized(
            [64.0, 22.0],
            NumField::new(&mut v)
                .max_decimals(app.settings.precision.min(3) as usize)
                .suffix(format!(" {}", u.short()))
                .unit_mm(u.mm()),
        );
    });
    if (v - before).abs() > 1e-12 {
        *mm = v * u.mm();
        true
    } else {
        false
    }
}


/// The arrow beside the Underline and Strikethrough buttons: their
/// options, in the window the panel opens (Underline Options and
/// Strikethrough Options).
fn line_options_arrow(app: &mut App, ui: &mut Ui) {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(12.0, 22.0), egui::Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(rect, 2.0, crate::theme::Tokens::tool_hover());
    }
    let c = rect.center();
    ui.painter().add(egui::Shape::convex_polygon(
        vec![
            egui::pos2(c.x - 3.5, c.y - 2.0),
            egui::pos2(c.x + 3.5, c.y - 2.0),
            egui::pos2(c.x, c.y + 2.5),
        ],
        crate::theme::Tokens::icon(),
        egui::Stroke::NONE,
    ));
    let resp = resp.rich_tip(tr("line_opt.menu"));
    egui::Popup::menu(&resp)
        .id(egui::Id::new("text_bar_line_options"))
        .show(|ui| {
            for (which, key) in [
                (td::Line::Underline, "line_opt.underline"),
                (td::Line::Strike, "line_opt.strike"),
            ] {
                if ui.button(format!("{}...", tr(key))).clicked() {
                    app.dialog = crate::ui::dialogs::Dialog::LineOptions(
                        crate::ui::line_options::State::open(app, which),
                    );
                    ui.close();
                }
            }
        });
}
/// Row 1: the characters' look, and the paragraph's word and letter
/// spacing.
pub(crate) fn characters(app: &mut App, ui: &mut Ui) {
    // With no text to change the bar shows the font and size of new text,
    // not those of the text selected last (#449).
    if !app.has_text_target() && app.live_font.is_none() {
        app.text_font = app.text_default_font.clone();
        app.text_size_pt = app.text_default_size;
    }
    let first = painel_tipo::first_text(app);
    let span = first.as_ref().and_then(|t| app.shown_span(t));
    let mut changed = false;
    changed |= tip_over(ui, keyed("toolbar.font"), |ui| {
        crate::ui::font_list::text_font_list(app, ui, "text_bar_font", 170.0)
    });
    changed |= td::style_list(app, ui);
    changed |= crate::ui::toolbar::text_size_list(app, ui);
    td::font_axes_button(app, ui, "text_bar");
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
    ] {
        if ui
            .add_sized([22.0, 22.0], egui::Button::selectable(on, label.size(14.0)))
            .rich_tip(crate::ui::tips::keyed(key))
            .clicked()
        {
            match key {
                "toolbar.bold" => app.text_bold = !app.text_bold,
                _ => app.text_italic = !app.text_italic,
            }
            changed = true;
        }
    }
    // Underline and Strikethrough: the buttons switch them for the text;
    // the arrow opens their options, as the Control panel's flyout does.
    let strike_on = span.as_ref().is_some_and(|s| s.strikethrough.is_on());
    for (on, label, key, which) in [
        (
            app.text_underline,
            egui::RichText::new("U").underline(),
            "toolbar.underline",
            td::Line::Underline,
        ),
        (
            strike_on,
            egui::RichText::new("S").strikethrough(),
            "docker.strikethrough",
            td::Line::Strike,
        ),
    ] {
        if ui
            .add_sized([22.0, 22.0], egui::Button::selectable(on, label.size(14.0)))
            .rich_tip(crate::ui::tips::keyed(key))
            .clicked()
        {
            match which {
                td::Line::Underline => {
                    app.text_underline = !on;
                    changed = true;
                }
                td::Line::Strike => {
                    let v = !on;
                    td::edit_spans(app, move |s| s.strikethrough = tracedraw_core::LineStyle::from(v));
                }
            }
        }
    }
    line_options_arrow(app, ui);
    if changed {
        app.apply_text_style();
    }
    crate::ui::toolbar::vsep(ui);
    // Tracking of the characters chosen.
    let mut tracking = span.as_ref().map_or(0.0, |s| s.tracking_pct);
    let enabled = span.is_some();
    ui.add_enabled_ui(enabled, |ui| {
        if field(ui, "docker.tracking", &mut tracking, -50.0..=200.0, " %") {
            td::edit_spans(app, move |s| s.tracking_pct = tracking);
        }
    });
    // Line spacing of the paragraph, typed in points as the Panel shows
    // it.
    if let Some(t) = first.as_ref() {
        if let Some((_, p)) = app.paragraph_style_of(t) {
            let size = td::t_size_pt(t);
            let mut pt = td::convert_leading(p.leading_pct, p.leading_unit, LeadingUnit::Points, size);
            if field(ui, "tipo.leading", &mut pt, 0.0..=2000.0, " pt") {
                let mut q = p.clone();
                q.leading_unit = LeadingUnit::Points;
                q.leading_pct = pt;
                painel_tipo::edit_format(app, &p, &q);
            }
        }
    }
    // Word and letter spacing of the paragraph.
    if let Some(t) = first.as_ref() {
        if let Some((_, mut p)) = app.paragraph_style_of(t) {
            let before = p.clone();
            if field(ui, "docker.word_spacing", &mut p.word_spacing_pct, 0.0..=2000.0, " %")
                | field(ui, "docker.char_spacing", &mut p.char_spacing_pct, -100.0..=2000.0, " %")
            {
                painel_tipo::edit_format(app, &before, &p);
            }
        }
    }
    crate::ui::toolbar::vsep(ui);
    // The characters' offsets and angle.
    crate::ui::propbar::char_offset_part(app, ui);
    crate::ui::toolbar::vsep(ui);
    // The panels this row stands for, and editing the text itself.
    let docker_open = app.docker_shown(crate::app::DockerTab::Text);
    if pic_button(
        ui,
        Pic::TextProperties,
        &crate::shortcuts::shown_in(format!("{} (Ctrl+T)", tr("toolbar.text_properties"))),
        true,
        docker_open,
    )
    .clicked()
    {
        app.toggle_docker(crate::app::DockerTab::Text);
    }
    let is_text = first.is_some();
    if pic_button(
        ui,
        Pic::EditText,
        &crate::shortcuts::shown_in(format!("{} (Ctrl+Shift+T)", tr("menu.text.edit_text"))),
        is_text,
        false,
    )
    .clicked()
    {
        app.open_edit_text_dialog();
    }
}

/// Row 2: the paragraph (its alignment, style, indents, spaces,
/// hyphenation, balance lines and keep rules) and its frame (the columns).
pub(crate) fn paragraph(app: &mut App, ui: &mut Ui) {
    let first = painel_tipo::first_text(app);
    let current = first
        .as_ref()
        .and_then(|t| app.paragraph_style_of(t))
        .map_or(app.text_align, |(a, _)| a);
    ui.spacing_mut().item_spacing.x = 2.0;
    for (a, pic, key) in [
        (TextAlign::Left, Pic::TextLeft, "docker.align_left_short"),
        (TextAlign::Center, Pic::TextCenter, "docker.align_center_short"),
        (TextAlign::Right, Pic::TextRight, "docker.align_right_short"),
        (TextAlign::Justify, Pic::TextJustify, "docker.align_justify_short"),
        (
            TextAlign::ForceJustify,
            Pic::TextForceJustify,
            "docker.align_force_justify",
        ),
        (TextAlign::None, Pic::TextNone, "docker.align_none"),
    ] {
        if pic_button(ui, pic, &tr(key), true, current == a).clicked() {
            td::set_align(app, a);
        }
    }
    ui.spacing_mut().item_spacing.x = 4.0;
    // Horizontal or vertical text, offered with a CJK interface language
    // or for text in those scripts (#257).
    if app.vertical_text_offered() {
        let on = app.vertical_text_on();
        if pic_button(
            ui,
            Pic::TextHorizontal,
            &tr("toolbar.text_horizontal"),
            true,
            !on,
        )
        .clicked()
            && on
        {
            app.set_vertical_text(false);
        }
        if pic_button(
            ui,
            Pic::TextVertical,
            &tr("toolbar.text_vertical"),
            true,
            on,
        )
        .clicked()
            && !on
        {
            app.set_vertical_text(true);
        }
    }
    crate::ui::toolbar::vsep(ui);
    paragraph_style_list(app, ui);
    crate::ui::toolbar::vsep(ui);
    let Some(t) = first else {
        return;
    };
    let Some((_, mut p)) = app.paragraph_style_of(&t) else {
        return;
    };
    let before = p.clone();
    painel_tipo::para_icon(ui, td::ParaIcon::LeftIndent, "docker.left_indent");
    dist(ui, app, "docker.left_indent", &mut p.left_indent);
    painel_tipo::para_icon(ui, td::ParaIcon::FirstLineIndent, "docker.first_line_indent");
    dist(ui, app, "docker.first_line_indent", &mut p.first_line_indent);
    painel_tipo::para_icon(ui, td::ParaIcon::RightIndent, "docker.right_indent");
    dist(ui, app, "docker.right_indent", &mut p.right_indent);
    painel_tipo::para_icon(ui, td::ParaIcon::SpaceBefore, "docker.space_before");
    dist(ui, app, "docker.space_before", &mut p.space_before);
    painel_tipo::para_icon(ui, td::ParaIcon::SpaceAfter, "docker.space_after");
    dist(ui, app, "docker.space_after", &mut p.space_after);
    let framed = matches!(t.kind, ShapeKind::Text { frame: Some(_), .. });
    // The columns belong to the frame: they go to the object's own
    // paragraph settings, not to the paragraph's.
    if framed {
        let mut columns = f64::from(p.columns.max(1));
        if field(ui, "dialog.columns", &mut columns, 1.0..=40.0, "") {
            let n = columns.round().max(1.0) as u32;
            if n != p.columns {
                app.edit_paragraph_style(t.id, |q: &mut ParagraphStyle| q.columns = n);
            }
        }
    }
    if ui
        .checkbox(&mut p.hyphenate, tr("docker.hyphenate"))
        .rich_tip(tr("docker.hyphenate"))
        .changed()
        | ui
            .checkbox(&mut p.balance_lines, tr("docker.balance_lines"))
            .rich_tip(crate::ui::tips::pair(
                "docker.balance_lines",
                "docker.balance_lines_desc",
            ))
            .changed()
    {
        // Only what changed reaches the paragraphs.
        painel_tipo::edit_format(app, &before, &p);
    }
    ui.spacing_mut().item_spacing.x = 2.0;
    if framed {
        crate::ui::toolbar::vsep(ui);
        td::keep_rules(app, ui, t.id);
    }
    crate::ui::toolbar::vsep(ui);
    ui.spacing_mut().item_spacing.x = 3.0;
    if pic_button(ui, Pic::Bullets, &tr("menu.text.bullets"), framed, p.bullets).clicked() {
        app.dialog = crate::ui::dialogs::Dialog::TextBullets;
    }
    if pic_button(ui, Pic::Tabs, &tr("menu.text.tabs"), framed, !p.tabs.is_empty()).clicked() {
        app.dialog = crate::ui::dialogs::Dialog::TextTabs;
    }
    if pic_button(
        ui,
        Pic::DropCap,
        &tr("menu.text.drop_cap"),
        framed,
        p.drop_cap_lines > 1,
    )
    .clicked()
    {
        app.dialog = crate::ui::dialogs::Dialog::TextDropCap;
    }
}

/// The paragraph styles the first text may take, chosen here as in the
/// Text styles docker: a click applies it to the paragraphs of the
/// selection, or takes the link off.
fn paragraph_style_list(app: &mut App, ui: &mut Ui) {
    let styles: Vec<tracedraw_core::text_styles::TextStyle> = app
        .doc()
        .text_styles
        .iter()
        .filter(|s| s.kind == TextStyleKind::Paragraph)
        .cloned()
        .collect();
    if styles.is_empty() {
        return;
    }
    let here = app.text_styles_here().0;
    let shown = here
        .and_then(|id| styles.iter().find(|s| s.id == id))
        .map(|s| s.name.clone())
        .unwrap_or_else(|| tr("tstyle.none"));
    let mut picked: Option<Option<u32>> = None;
    tip_over(ui, tr("tstyle.paragraph"), |ui| {
        crate::ui::combo::from_id_salt("text_bar_para_style")
            .width(150.0)
            .selected_text(shown)
            .show_ui(ui, |ui| {
                if ui.selectable_label(here.is_none(), tr("tstyle.none")).clicked() {
                    picked = Some(None);
                }
                for s in &styles {
                    if ui.selectable_label(here == Some(s.id), &s.name).clicked() {
                        picked = Some(Some(s.id));
                    }
                }
            });
    });
    if let Some(id) = picked {
        app.apply_named_style(TextStyleKind::Paragraph, id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::{Point, Size};
    use tracedraw_core::{ShapeId, TextSpan};

    fn with_text(app: &mut App) -> ShapeId {
        let id = app
            .new_shape(ShapeKind::Text {
                spans: vec![TextSpan::new("Um título de duas linhas", "Sans", 12.0)],
                origin: Point::new(10.0, 10.0),
                frame: Some(Size::new(80.0, 40.0)),
                align: TextAlign::Left,
                para: ParagraphStyle::default(),
                on_path: None,
            })
            .expect("text");
        app.select(vec![id]);
        id
    }

    /// The two rows come with the Text tool and while a text is being
    /// edited; a text selected with the Pick tool keeps the bar's one row,
    /// so selecting it to move it does not change the room of the drawing.
    #[test]
    fn the_two_rows_show_with_text() {
        let mut app = App::headless();
        assert!(!two_rows(&app), "nothing selected, the Pick tool");
        with_text(&mut app);
        assert!(!two_rows(&app), "a text selected with the Pick tool");
        app.set_tool(crate::tools::Tool::Text);
        assert!(two_rows(&app), "the Text tool");
        app.select(Vec::new());
        assert!(two_rows(&app), "the Text tool with nothing selected");
        app.set_tool(crate::tools::Tool::Pick);
        app.start_text(Point::new(30.0, 50.0), None);
        assert!(app.text_edit.is_some());
        assert!(two_rows(&app), "the caret in a text");
        app.finish_text();
        assert!(!two_rows(&app), "the editing over, the Pick tool");
    }

    /// The property bar's band grows for the two rows and comes back.
    #[test]
    fn the_bar_band_grows_for_the_two_rows() {
        let mut app = App::headless();
        let one = crate::ui::bars::Bar::Property.row_height();
        assert_eq!(crate::ui::bars::Bar::Property.row_height_for(&app), one);
        with_text(&mut app);
        app.set_tool(crate::tools::Tool::Text);
        let two = crate::ui::bars::Bar::Property.row_height_for(&app);
        assert!(two > one + 10.0, "{two} {one}");
        assert_eq!(two, ROWS_H + 16.0);
    }

    /// The window lays the property bar out in the two rows while text is
    /// selected, and back in one row without text.
    #[test]
    fn the_window_gives_the_bar_two_rows_for_text() {
        let mut app = App::headless();
        let ctx = crate::theme::ui_context();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600.0, 1000.0));
        let frame = |app: &mut App| {
            let mut o = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(screen),
                    ..Default::default()
                },
                |ui| crate::ui::root(app, ui),
            );
            o.textures_delta.clear();
        };
        let property_row = |app: &App| {
            app.bar_rows
                .iter()
                .find(|r| {
                    r.slots
                        .iter()
                        .any(|(b, _)| *b == crate::ui::bars::Bar::Property)
                })
                .map(|r| r.rect.height())
        };
        frame(&mut app);
        let one = property_row(&app).expect("the property bar in a row");
        with_text(&mut app);
        app.set_tool(crate::tools::Tool::Text);
        frame(&mut app);
        let two = property_row(&app).expect("the property bar in a row");
        assert!(two > one + 10.0, "{two} {one}");
        app.select(Vec::new());
        app.set_tool(crate::tools::Tool::Pick);
        frame(&mut app);
        let back = property_row(&app).expect("the property bar in a row");
        assert!((back - one).abs() < 0.5, "{back} {one}");
    }

    /// Both rows draw with a text selected and stay within the bar.
    #[test]
    fn the_rows_draw_and_keep_the_bar_height() {
        let mut app = App::headless();
        with_text(&mut app);
        app.set_tool(crate::tools::Tool::Text);
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600.0, 1000.0));
        for _ in 0..3 {
            let mut o = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(screen),
                    ..Default::default()
                },
                |ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| characters(&mut app, ui));
                        ui.horizontal(|ui| paragraph(&mut app, ui));
                    });
                },
            );
            o.textures_delta.clear();
        }
        assert!(two_rows(&app));
    }
}
