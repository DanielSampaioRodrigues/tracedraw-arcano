//! The Text Styles panel (Window > Panels > Text Styles): the drawing's
//! paragraph and character styles (`tracedraw_core::text_styles`). A
//! click on a style applies it to the text being edited (the paragraphs
//! the selection touches, or the selected characters) or to the selected
//! text objects; the style at the caret is marked. New takes the look of
//! the text at the caret, Redefine gives a style the look of the text at
//! the caret, and every text linked to a style follows its changes. See
//! docs/behavior/text-styles.md.

use crate::app::App;
use tracedraw_core::paragraphs::ParagraphFormat;
use tracedraw_core::text_styles::{self as ts, CharFormat, TextStyle, TextStyleKind};
use tracedraw_core::{Command, ShapeId, ShapeKind, TextAlign};

/// Where a style goes: each text and the characters (`None`: all).
type Targets = Vec<(ShapeId, Option<(usize, usize)>)>;

impl App {
    /// The texts and characters a style applies to: the selection (or
    /// the caret) of the text being edited, else the selected texts, all
    /// of each (or the characters chosen with the Shape tool).
    pub fn text_style_targets(&self) -> Targets {
        if let Some(te) = &self.text_edit {
            return vec![(te.shape, Some(te.selection()))];
        }
        self.selected_shapes()
            .iter()
            .filter(|s| matches!(s.kind, ShapeKind::Text { .. }))
            .map(|s| (s.id, self.paragraph_range(s.id)))
            .collect()
    }

    /// The paragraph style and character style where the caret is (else
    /// at the start of the first selected text).
    pub fn text_styles_here(&self) -> (Option<u32>, Option<u32>) {
        let Some((id, range)) = self.text_style_targets().into_iter().next() else {
            return (None, None);
        };
        let Some(s) = self.doc().find_shape(id) else {
            return (None, None);
        };
        ts::styles_at(&s.kind, range.map_or(0, |r| r.0))
    }

    /// Apply style `id` of `kind` (`None`: take the link off) to
    /// [`App::text_style_targets`]. One undo step.
    pub fn apply_named_style(&mut self, kind: TextStyleKind, id: Option<u32>) {
        let styles = self.doc().text_styles.clone();
        let mut cmds = Vec::new();
        for (shape, range) in self.text_style_targets() {
            let Some(s) = self.doc().find_shape(shape) else {
                continue;
            };
            let mut k = s.kind.clone();
            match kind {
                TextStyleKind::Paragraph => ts::apply_paragraph_style(&mut k, range, &styles, id),
                TextStyleKind::Character => {
                    let (a, b) = match range {
                        Some((a, b)) if a != b => (a, b),
                        // A caret alone, or a whole text: all of it.
                        Some(_) if self.text_edit.is_some() => continue,
                        _ => (0, char_count(&k)),
                    };
                    ts::apply_char_style(&mut k, a, b, &styles, id);
                }
            }
            if k != s.kind {
                cmds.push(Command::SetShapeKind { shape, kind: k });
            }
        }
        if cmds.is_empty() {
            if kind == TextStyleKind::Character && self.text_edit.is_some() {
                self.status = crate::i18n::tr("tstyle.select_chars");
            }
            return;
        }
        if let Err(e) = self.engine.run_batch("Text Styles", &cmds) {
            self.status = e.to_string();
        }
    }

    /// Replace the drawing's text styles (one undo step); every text
    /// linked to them follows.
    pub fn set_text_styles(&mut self, styles: Vec<TextStyle>) {
        if styles != self.doc().text_styles {
            self.run(Command::SetTextStyles { styles });
        }
    }

    /// The text at the caret (else the first selected text) and where.
    fn text_here(&self) -> Option<(ShapeKind, usize)> {
        let (id, range) = self.text_style_targets().into_iter().next()?;
        let s = self.doc().find_shape(id)?;
        Some((s.kind.clone(), range.map_or(0, |r| r.0)))
    }

    /// A new style of `kind` with the look of the text at the caret,
    /// linked to that text. Returns its id.
    pub fn new_text_style(&mut self, kind: TextStyleKind) -> Option<u32> {
        let (text, at) = self.text_here()?;
        let styles = self.doc().text_styles.clone();
        let n = styles.iter().filter(|s| s.kind == kind).count() + 1;
        let name = crate::i18n::trf(
            match kind {
                TextStyleKind::Paragraph => "tstyle.new_para_name",
                TextStyleKind::Character => "tstyle.new_char_name",
            },
            &[("n", &n.to_string())],
        );
        let mut st = match kind {
            TextStyleKind::Paragraph => ts::paragraph_style_from(&text, at, name)?,
            TextStyleKind::Character => ts::char_style_from(&text, at, &styles, name)?,
        };
        let id = styles.iter().map(|s| s.id).max().unwrap_or(0) + 1;
        st.id = id;
        let mut all = styles;
        all.push(st);
        self.set_text_styles(all);
        self.apply_named_style(kind, Some(id));
        Some(id)
    }

    /// Give style `id` the look of the text at the caret (its paragraph
    /// format and characters), keeping its name and parent.
    pub fn redefine_text_style(&mut self, id: u32) {
        let Some((text, at)) = self.text_here() else {
            return;
        };
        let mut styles = self.doc().text_styles.clone();
        let Some(i) = styles.iter().position(|s| s.id == id) else {
            return;
        };
        let old = styles[i].clone();
        let fresh = match old.kind {
            TextStyleKind::Paragraph => ts::paragraph_style_from(&text, at, old.name.clone()),
            TextStyleKind::Character => {
                let mut others = styles.clone();
                others.retain(|s| s.id != id);
                ts::char_style_from(&text, at, &others, old.name.clone())
            }
        };
        let Some(mut fresh) = fresh else {
            return;
        };
        fresh.id = id;
        fresh.based_on = old.based_on;
        // Only what differs from the parent stays the style's own.
        if let Some(p) = old.based_on {
            let (pp, pc) = ts::resolve(&styles, p);
            fresh.chars = fresh.chars.differing_from(&pc);
            if fresh.para == pp {
                fresh.para = None;
            }
        }
        styles[i] = fresh;
        self.set_text_styles(styles);
    }

    /// The starting set for a book: body text, first paragraph, chapter
    /// title, subtitle, quotation and the character styles for emphasis,
    /// bold and small caps. They set no font: the text keeps its own.
    pub fn add_book_styles(&mut self) {
        let mut styles = self.doc().text_styles.clone();
        let mut next = styles.iter().map(|s| s.id).max().unwrap_or(0);
        let mut add = |styles: &mut Vec<TextStyle>,
                       key: &str,
                       kind,
                       based_on,
                       para: Option<ParagraphFormat>,
                       chars| {
            next += 1;
            styles.push(TextStyle {
                id: next,
                name: crate::i18n::tr(key),
                kind,
                based_on,
                para,
                chars,
            });
            next
        };
        let body_fmt = ParagraphFormat {
            align: TextAlign::Justify,
            first_line_indent: 5.0,
            hyphenate: true,
            orphans: 2,
            widows: 2,
            ..Default::default()
        };
        let body = add(
            &mut styles,
            "tstyle.book_body",
            TextStyleKind::Paragraph,
            None,
            Some(body_fmt.clone()),
            CharFormat {
                size_pt: Some(11.0),
                bold: Some(false),
                italic: Some(false),
                ..Default::default()
            },
        );
        add(
            &mut styles,
            "tstyle.book_first",
            TextStyleKind::Paragraph,
            Some(body),
            Some(ParagraphFormat {
                first_line_indent: 0.0,
                ..body_fmt.clone()
            }),
            CharFormat::default(),
        );
        add(
            &mut styles,
            "tstyle.book_chapter",
            TextStyleKind::Paragraph,
            Some(body),
            Some(ParagraphFormat {
                align: TextAlign::Center,
                first_line_indent: 0.0,
                space_before: 20.0,
                space_after: 12.0,
                hyphenate: false,
                keep_with_next: true,
                ..Default::default()
            }),
            CharFormat {
                size_pt: Some(20.0),
                bold: Some(true),
                ..Default::default()
            },
        );
        add(
            &mut styles,
            "tstyle.book_subtitle",
            TextStyleKind::Paragraph,
            Some(body),
            Some(ParagraphFormat {
                align: TextAlign::Center,
                first_line_indent: 0.0,
                space_after: 8.0,
                hyphenate: false,
                keep_with_next: true,
                ..Default::default()
            }),
            CharFormat {
                size_pt: Some(14.0),
                italic: Some(true),
                ..Default::default()
            },
        );
        add(
            &mut styles,
            "tstyle.book_quote",
            TextStyleKind::Paragraph,
            Some(body),
            Some(ParagraphFormat {
                left_indent: 10.0,
                right_indent: 10.0,
                first_line_indent: 0.0,
                space_before: 4.0,
                space_after: 4.0,
                ..body_fmt
            }),
            CharFormat {
                size_pt: Some(10.0),
                ..Default::default()
            },
        );
        add(
            &mut styles,
            "tstyle.book_emphasis",
            TextStyleKind::Character,
            None,
            None,
            CharFormat {
                italic: Some(true),
                ..Default::default()
            },
        );
        add(
            &mut styles,
            "tstyle.book_strong",
            TextStyleKind::Character,
            None,
            None,
            CharFormat {
                bold: Some(true),
                ..Default::default()
            },
        );
        add(
            &mut styles,
            "tstyle.book_smallcaps",
            TextStyleKind::Character,
            None,
            None,
            CharFormat {
                features: Some(vec!["smcp".into()]),
                ..Default::default()
            },
        );
        self.set_text_styles(styles);
    }
}

fn char_count(k: &ShapeKind) -> usize {
    match k {
        ShapeKind::Text { spans, .. } => spans.iter().map(|s| s.text.chars().count()).sum(),
        _ => 0,
    }
}

/// How often style `id` is used in the drawing (paragraphs, or runs of
/// characters).
fn uses(app: &App, id: u32) -> usize {
    app.doc()
        .all_layers()
        .flat_map(|l| l.shapes.iter())
        .map(|s| ts::count_uses(s, id))
        .sum()
}

/// The panel.
pub fn docker(app: &mut App, ui: &mut egui::Ui) {
    use crate::i18n::tr;
    use crate::ui::tips::{pair, RichTip as _};
    let has_text = !app.text_style_targets().is_empty();
    let (here_p, here_c) = app.text_styles_here();
    if !has_text {
        ui.label(
            egui::RichText::new(tr("tstyle.select_text")).color(crate::theme::Tokens::text_dim()),
        );
    }
    if app.doc().text_styles.is_empty()
        && ui
            .button(tr("tstyle.book_set"))
            .rich_tip(pair("tstyle.book_set", "tstyle.book_set_desc"))
            .clicked()
    {
        app.add_book_styles();
    }
    for kind in [TextStyleKind::Paragraph, TextStyleKind::Character] {
        ui.add_space(6.0);
        let here = match kind {
            TextStyleKind::Paragraph => here_p,
            TextStyleKind::Character => here_c,
        };
        ui.horizontal(|ui| {
            ui.strong(tr(match kind {
                TextStyleKind::Paragraph => "tstyle.paragraph",
                TextStyleKind::Character => "tstyle.character",
            }));
            if ui
                .add_enabled(has_text, egui::Button::new(tr("tstyle.new")).small())
                .rich_tip(pair("tstyle.new", "tstyle.new_desc"))
                .clicked()
            {
                if let Some(id) = app.new_text_style(kind) {
                    ui.data_mut(|d| d.insert_temp(egui::Id::new("tstyle_open"), Some(id)));
                }
            }
        });
        // "None": take the link off.
        if ui
            .add_enabled(
                has_text,
                egui::Button::selectable(here.is_none() && has_text, tr("tstyle.none")),
            )
            .rich_tip(pair("tstyle.none", "tstyle.none_desc"))
            .clicked()
        {
            app.apply_named_style(kind, None);
        }
        let list: Vec<TextStyle> = app
            .doc()
            .text_styles
            .iter()
            .filter(|s| s.kind == kind)
            .cloned()
            .collect();
        for st in list {
            row(app, ui, &st, here == Some(st.id), has_text);
        }
    }
}

/// One style: a click applies it; the arrow opens its settings.
fn row(app: &mut App, ui: &mut egui::Ui, st: &TextStyle, current: bool, has_text: bool) {
    use crate::i18n::{tr, trf};
    use crate::ui::tips::{pair, RichTip as _};
    let open_id = egui::Id::new("tstyle_open");
    let open = ui.data(|d| d.get_temp::<Option<u32>>(open_id)).flatten() == Some(st.id);
    ui.horizontal(|ui| {
        let arrow = if open { "-" } else { "+" };
        if ui
            .add(egui::Button::new(arrow).small().frame(false))
            .rich_tip(pair("tstyle.settings", "tstyle.settings_desc"))
            .clicked()
        {
            ui.data_mut(|d| d.insert_temp(open_id, if open { None } else { Some(st.id) }));
        }
        let parent = st
            .based_on
            .and_then(|p| ts::find(&app.doc().text_styles, p))
            .map(|p| p.name.clone());
        let label = match &parent {
            Some(p) => format!("{}  \u{00AB} {p}", st.name),
            None => st.name.clone(),
        };
        if ui
            .add_enabled(has_text, egui::Button::selectable(current, label))
            .rich_tip(pair("tstyle.apply", "tstyle.apply_desc"))
            .clicked()
        {
            app.apply_named_style(st.kind, Some(st.id));
        }
        ui.label(
            egui::RichText::new(uses(app, st.id).to_string())
                .small()
                .color(crate::theme::Tokens::text_dim()),
        )
        .rich_tip(pair("tstyle.uses", "tstyle.uses_desc"));
    });
    if !open {
        return;
    }
    egui::Frame::new()
        .inner_margin(egui::Margin {
            left: 18,
            right: 4,
            top: 2,
            bottom: 6,
        })
        .show(ui, |ui| {
            let mut styles = app.doc().text_styles.clone();
            let Some(i) = styles.iter().position(|s| s.id == st.id) else {
                return;
            };
            // The name, kept when the field is left.
            let name_id = egui::Id::new(("tstyle_name", st.id));
            let mut name = ui
                .data(|d| d.get_temp::<String>(name_id))
                .unwrap_or_else(|| st.name.clone());
            let r = ui
                .add(egui::TextEdit::singleline(&mut name).desired_width(160.0))
                .rich_tip(pair("tstyle.name", "tstyle.name_desc"));
            if r.changed() {
                ui.data_mut(|d| d.insert_temp(name_id, name.clone()));
            }
            if r.lost_focus() {
                ui.data_mut(|d| d.remove::<String>(name_id));
                let n = name.trim().to_string();
                if !n.is_empty() && n != st.name {
                    styles[i].name = n;
                    app.set_text_styles(styles.clone());
                }
            }
            // Based on.
            let mut parent = st.based_on;
            let shown = parent
                .and_then(|p| ts::find(&styles, p))
                .map(|p| p.name.clone())
                .unwrap_or_else(|| tr("tstyle.based_none"));
            ui.horizontal(|ui| {
                ui.label(tr("tstyle.based_on"));
                crate::ui::combo::from_id_salt(("tstyle_based", st.id))
                    .selected_text(shown)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut parent, None, tr("tstyle.based_none"));
                        for o in styles.iter().filter(|o| {
                            o.kind == st.kind
                                && o.id != st.id
                                && !ts::would_loop(&styles, st.id, o.id)
                        }) {
                            ui.selectable_value(&mut parent, Some(o.id), &o.name);
                        }
                    })
                    .response
                    .rich_tip(pair("tstyle.based_on", "tstyle.based_on_desc"));
            });
            if parent != st.based_on {
                styles[i].based_on = parent;
                app.set_text_styles(styles.clone());
            }
            ui.horizontal_wrapped(|ui| {
                if ui
                    .add_enabled(has_text, egui::Button::new(tr("tstyle.redefine")).small())
                    .rich_tip(pair("tstyle.redefine", "tstyle.redefine_desc"))
                    .clicked()
                {
                    app.redefine_text_style(st.id);
                }
                if ui
                    .add_enabled(has_text, egui::Button::new(tr("tstyle.clear")).small())
                    .rich_tip(pair("tstyle.clear", "tstyle.clear_desc"))
                    .clicked()
                {
                    app.apply_named_style(st.kind, Some(st.id));
                }
                if ui
                    .button(tr("tstyle.delete"))
                    .rich_tip(pair("tstyle.delete", "tstyle.delete_desc"))
                    .clicked()
                {
                    // Its children take its parent; the text keeps its look.
                    let gone = styles.remove(i);
                    for o in styles.iter_mut().filter(|o| o.based_on == Some(gone.id)) {
                        o.based_on = gone.based_on;
                        let (pp, pc) = ts::resolve(&app.doc().text_styles, gone.id);
                        o.chars = pc.with(&o.chars);
                        if o.para.is_none() {
                            o.para = pp;
                        }
                    }
                    app.set_text_styles(styles.clone());
                    ui.data_mut(|d| d.insert_temp(open_id, None::<u32>));
                    app.status = trf("tstyle.deleted", &[("name", &gone.name)]);
                }
            });
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::{Point, Size};
    use tracedraw_core::{ParagraphStyle, TextSpan};

    fn with_text(app: &mut App, s: &str) -> ShapeId {
        let id = app
            .new_shape(ShapeKind::Text {
                spans: vec![TextSpan::new(s, "Sans", 12.0)],
                origin: Point::new(10.0, 10.0),
                frame: Some(Size::new(100.0, 80.0)),
                align: TextAlign::Left,
                para: ParagraphStyle::default(),
                on_path: None,
            })
            .expect("text");
        app.select(vec![id]);
        id
    }

    fn spans(app: &App, id: ShapeId) -> Vec<TextSpan> {
        match &app.doc().find_shape(id).unwrap().kind {
            ShapeKind::Text { spans, .. } => spans.clone(),
            _ => unreachable!(),
        }
    }

    #[test]
    fn the_book_set_styles_a_text_and_follows_a_change() {
        let mut app = App::headless();
        let id = with_text(&mut app, "Capítulo 1\nEra uma vez.");
        app.add_book_styles();
        let body = app
            .doc()
            .text_styles
            .iter()
            .find(|s| s.name == crate::i18n::tr("tstyle.book_body"))
            .unwrap()
            .id;
        let chapter = app
            .doc()
            .text_styles
            .iter()
            .find(|s| s.name == crate::i18n::tr("tstyle.book_chapter"))
            .unwrap()
            .id;
        // The whole text in the body style, then the first paragraph as a chapter title.
        app.apply_named_style(TextStyleKind::Paragraph, Some(body));
        let undo_before = app.engine.history_labels().0.len();
        app.text_edit = Some(crate::text_editing::TextEdit {
            shape: id,
            caret: 2,
            anchor: 2,
            goal_x: None,
            step: None,
            created: None,
        });
        app.apply_named_style(TextStyleKind::Paragraph, Some(chapter));
        assert_eq!(
            app.engine.history_labels().0.len(),
            undo_before + 1,
            "one undo step"
        );
        assert_eq!(app.text_styles_here(), (Some(chapter), None));
        let s = spans(&app, id);
        assert!(s
            .iter()
            .any(|sp| sp.text.starts_with("Cap") && sp.size_pt == 20.0 && sp.bold));
        assert!(s
            .iter()
            .any(|sp| sp.text.contains("Era") && sp.size_pt == 11.0));
        // The body grows: the chapter title (based on it) keeps its own size.
        let mut styles = app.doc().text_styles.clone();
        styles
            .iter_mut()
            .find(|s| s.id == body)
            .unwrap()
            .chars
            .size_pt = Some(12.0);
        app.set_text_styles(styles);
        let s = spans(&app, id);
        assert!(s
            .iter()
            .any(|sp| sp.text.contains("Era") && sp.size_pt == 12.0));
        assert!(s
            .iter()
            .any(|sp| sp.text.starts_with("Cap") && sp.size_pt == 20.0));
        assert_eq!(uses(&app, body), 1);
        assert_eq!(uses(&app, chapter), 1);
    }

    #[test]
    fn a_new_style_takes_the_look_of_the_text() {
        let mut app = App::headless();
        let id = with_text(&mut app, "Um texto");
        let st = app.new_text_style(TextStyleKind::Paragraph).unwrap();
        assert_eq!(app.doc().text_styles[0].chars.size_pt, Some(12.0));
        assert_eq!(app.text_styles_here().0, Some(st));
        // Redefine from text made larger by hand.
        let mut k = app.doc().find_shape(id).unwrap().kind.clone();
        if let ShapeKind::Text { spans, .. } = &mut k {
            spans[0].size_pt = 16.0;
        }
        app.run(Command::SetShapeKind { shape: id, kind: k });
        app.redefine_text_style(st);
        assert_eq!(app.doc().text_styles[0].chars.size_pt, Some(16.0));
    }

    #[test]
    fn the_panel_draws_and_keeps_its_width() {
        let mut app = App::headless();
        with_text(&mut app, "Um texto");
        app.add_book_styles();
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1400.0, 800.0));
        let mut widths = Vec::new();
        for frame in 0..6 {
            if frame == 2 {
                let id = app.doc().text_styles[0].id;
                ctx.data_mut(|d| d.insert_temp(egui::Id::new("tstyle_open"), Some(id)));
            }
            let mut o = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(screen),
                    ..Default::default()
                },
                |ui| {
                    let r = egui::Panel::right("dock")
                        .default_size(280.0)
                        .show(ui, |ui| docker(&mut app, ui));
                    widths.push(r.response.rect.width());
                },
            );
            o.textures_delta.clear();
        }
        assert!(widths.iter().all(|w| *w < 400.0), "{widths:?}");
        assert!((widths[5] - widths[3]).abs() < 1.0, "{widths:?}");
    }
}
