//! Arcano edition: the house press panel (Window > Panels > House Press):
//! buttons that check and fix a drawing without any AI service.
//!
//! - Typography: curly quotes and apostrophes, the dash between words and
//!   between numbers, the ellipsis, double spaces and spaces before
//!   punctuation. Check counts them; Fix changes them, one undo step.
//! - Local formatting: characters in a paragraph with a paragraph style
//!   whose look differs from the style where the style sets it (and that
//!   have no character style). Clear gives them the style back.
//! - House standard: the book styles (added when missing), the bleed and
//!   guides at the margins and the safety distance on every page (inner
//!   and outer margins alternate, the first page a right-hand one). Its
//!   values are kept with the house library when one is chosen, so every
//!   computer of the house uses the same.
//!
//! See docs/behavior/house-press.md.

use crate::app::App;
use tracedraw_core::document::{Guide, GuideLine};
use tracedraw_core::text_styles as ts;
use tracedraw_core::{Command, Shape, ShapeId, ShapeKind, TextSpan};

// ----- typography -----------------------------------------------------------

fn is_open_context(prev: Option<char>) -> bool {
    match prev {
        None => true,
        Some(c) => c.is_whitespace() || "([{\u{2014}\u{2013}/-\u{201C}\u{2018}".contains(c),
    }
}

/// `text` with the house typography, `prev` the character before it (in
/// the span before); returns the text and how many changes it made.
pub fn typography(text: &str, prev: Option<char>) -> (String, usize) {
    let chars: Vec<char> = text.chars().collect();
    let mut out: Vec<char> = Vec::with_capacity(chars.len());
    let mut n = 0;
    let mut i = 0;
    let last = |out: &Vec<char>| out.last().copied().or(prev);
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        match c {
            '"' => {
                out.push(if is_open_context(last(&out)) {
                    '\u{201C}'
                } else {
                    '\u{201D}'
                });
                n += 1;
            }
            '\'' => {
                // Inside a word an apostrophe; else opening or closing.
                let p = last(&out);
                let q = if p.is_some_and(|p| p.is_alphanumeric()) {
                    '\u{2019}'
                } else if is_open_context(p) {
                    '\u{2018}'
                } else {
                    '\u{2019}'
                };
                out.push(q);
                n += 1;
            }
            '.' if next == Some('.') && chars.get(i + 2) == Some(&'.') => {
                out.push('\u{2026}');
                n += 1;
                i += 3;
                continue;
            }
            '-' if next == Some('-') => {
                out.push('\u{2014}');
                n += 1;
                i += 2;
                continue;
            }
            '-' if last(&out) == Some(' ') && next == Some(' ') => {
                // A hyphen between spaces: the dash.
                out.push('\u{2014}');
                n += 1;
            }
            '-' if last(&out).is_some_and(|p| p.is_ascii_digit())
                && next.is_some_and(|q| q.is_ascii_digit()) =>
            {
                // Between numbers (pages, years): the en dash.
                out.push('\u{2013}');
                n += 1;
            }
            ' ' if out.last() == Some(&' ') => {
                n += 1;
            }
            ',' | '.' | ';' | ':' | '!' | '?' | ')' if out.last() == Some(&' ') => {
                out.pop();
                out.push(c);
                n += 1;
            }
            _ => out.push(c),
        }
        i += 1;
    }
    (out.into_iter().collect(), n)
}

/// [`typography`] over the spans of a text, each keeping its formatting.
pub fn typography_spans(spans: &[TextSpan]) -> (Vec<TextSpan>, usize) {
    let mut out = Vec::with_capacity(spans.len());
    let mut total = 0;
    let mut prev: Option<char> = None;
    for sp in spans {
        let (t, n) = typography(&sp.text, prev);
        total += n;
        prev = t.chars().last().or(prev);
        let mut s = sp.clone();
        s.text = t;
        out.push(s);
    }
    (out, total)
}

// ----- local formatting -----------------------------------------------------

/// The runs of characters in text `kind` whose look differs from their
/// paragraph style where the style sets it, without a character style.
pub fn local_overrides(kind: &ShapeKind, styles: &[ts::TextStyle]) -> usize {
    let ShapeKind::Text {
        spans, align, para, ..
    } = kind
    else {
        return 0;
    };
    let ranges = tracedraw_core::paragraphs::paragraph_ranges(spans);
    let mut n = 0;
    for (i, &(s, e)) in ranges.iter().enumerate() {
        let Some(id) = tracedraw_core::paragraphs::paragraph_format(spans, *align, para, i).style
        else {
            continue;
        };
        let chars = ts::resolve(styles, id).1;
        let mut pos = 0;
        for sp in spans {
            let len = sp.text.chars().count();
            let (a, b) = (pos, pos + len);
            pos = b;
            if b <= s || a > e || len == 0 || sp.char_style.is_some() || sp.text.trim().is_empty() {
                continue;
            }
            let mut want = sp.clone();
            chars.apply(&mut want);
            if want != *sp {
                n += 1;
            }
        }
    }
    n
}

// ----- the whole drawing ----------------------------------------------------

/// Every text in the drawing (or the selected ones), with its id.
fn texts(app: &App, only_selected: bool) -> Vec<(ShapeId, ShapeKind)> {
    fn walk(s: &Shape, out: &mut Vec<(ShapeId, ShapeKind)>) {
        match &s.kind {
            ShapeKind::Group { children } => children.iter().for_each(|c| walk(c, out)),
            ShapeKind::ClipFrame { frame, contents } => {
                walk(frame, out);
                contents.iter().for_each(|c| walk(c, out));
            }
            ShapeKind::Text { .. } => out.push((s.id, s.kind.clone())),
            _ => {}
        }
    }
    let mut out = Vec::new();
    if only_selected {
        for s in app.selected_shapes() {
            walk(&s, &mut out);
        }
    } else {
        for l in app.doc().all_layers() {
            for s in &l.shapes {
                walk(s, &mut out);
            }
        }
    }
    out
}

/// What a check found: how many changes, in how many texts.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Found {
    pub changes: usize,
    pub texts: usize,
}

impl App {
    /// Typography to fix in the selected texts (else the whole drawing);
    /// `fix`: fix it, one undo step.
    pub fn house_typography(&mut self, fix: bool) -> Found {
        let only = !self.selection.is_empty();
        let mut found = Found::default();
        let mut cmds = Vec::new();
        for (id, kind) in texts(self, only) {
            let ShapeKind::Text { spans, .. } = &kind else {
                continue;
            };
            let (fixed, n) = typography_spans(spans);
            if n == 0 {
                continue;
            }
            found.changes += n;
            found.texts += 1;
            if fix {
                let mut k = kind.clone();
                if let ShapeKind::Text { spans, .. } = &mut k {
                    *spans = tracedraw_core::document::merge_equal_spans(fixed);
                }
                cmds.push(Command::SetShapeKind { shape: id, kind: k });
            }
        }
        if fix && !cmds.is_empty() {
            if let Err(e) = self.engine.run_batch("House Typography", &cmds) {
                self.status = e.to_string();
            }
        }
        found
    }

    /// Formatting given by hand over paragraph styles in the selected
    /// texts (else the whole drawing); `fix`: the styles again (character
    /// styles stay), one undo step.
    pub fn house_local_formatting(&mut self, fix: bool) -> Found {
        let only = !self.selection.is_empty();
        let styles = self.doc().text_styles.clone();
        let mut found = Found::default();
        let mut cmds = Vec::new();
        for (id, kind) in texts(self, only) {
            let n = local_overrides(&kind, &styles);
            if n == 0 {
                continue;
            }
            found.changes += n;
            found.texts += 1;
            if fix {
                let mut k = kind.clone();
                ts::refresh_text(&mut k, &styles);
                cmds.push(Command::SetShapeKind { shape: id, kind: k });
            }
        }
        if fix && !cmds.is_empty() {
            if let Err(e) = self.engine.run_batch("House Formatting", &cmds) {
                self.status = e.to_string();
            }
        }
        found
    }

    /// The house standard on the drawing: book styles where missing, the
    /// bleed, and guides at the margins and the safety distance of every
    /// page. One undo step.
    pub fn apply_house_standard(&mut self) {
        let h = house_prefs(self);
        let names: Vec<String> = self
            .doc()
            .text_styles
            .iter()
            .map(|s| s.name.clone())
            .collect();
        if !names.contains(&crate::i18n::tr("tstyle.book_body")) {
            self.add_book_styles();
        }
        let mut cmds = Vec::new();
        let mut meta = self.doc().metadata.clone();
        if (meta.bleed - h.bleed).abs() > 1e-9 {
            meta.bleed = h.bleed;
            cmds.push(Command::SetMetadata { metadata: meta });
        }
        for (i, page) in self.doc().pages.iter().enumerate() {
            let (w, ht) = (page.size.width, page.size.height);
            // The first page is a right-hand one: its inner margin at the left.
            let right_hand = i % 2 == 0;
            let (left, right) = if right_hand {
                (h.inner, h.outer)
            } else {
                (h.outer, h.inner)
            };
            let mut guides: Vec<Guide> = page
                .guides
                .iter()
                .filter(|g| g.color != Some(HOUSE_GUIDE))
                .cloned()
                .collect();
            let mut add = |line: GuideLine| {
                let mut g = Guide::from(line);
                g.color = Some(HOUSE_GUIDE);
                guides.push(g);
            };
            add(GuideLine::Vertical { x: left });
            add(GuideLine::Vertical { x: w - right });
            add(GuideLine::Horizontal { y: h.bottom });
            add(GuideLine::Horizontal { y: ht - h.top });
            if h.safety > 0.0 {
                add(GuideLine::Vertical { x: h.safety });
                add(GuideLine::Vertical { x: w - h.safety });
                add(GuideLine::Horizontal { y: h.safety });
                add(GuideLine::Horizontal { y: ht - h.safety });
            }
            cmds.push(Command::SetGuides {
                page: page.id,
                guides,
            });
        }
        if let Err(e) = self.engine.run_batch("House Standard", &cmds) {
            self.status = e.to_string();
        } else {
            self.status = crate::i18n::tr("prelo.standard_done");
        }
    }
}

/// The colour that marks the house standard's guides (so applying again
/// replaces them and leaves the user's own).
const HOUSE_GUIDE: tracedraw_core::Color = tracedraw_core::Color::Rgb {
    r: 0.85,
    g: 0.35,
    b: 0.75,
};

/// The house standard's values, millimetres.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct HousePrefs {
    pub top: f64,
    pub bottom: f64,
    pub inner: f64,
    pub outer: f64,
    pub bleed: f64,
    pub safety: f64,
}

impl Default for HousePrefs {
    fn default() -> Self {
        HousePrefs {
            top: 20.0,
            bottom: 25.0,
            inner: 22.0,
            outer: 18.0,
            bleed: 3.0,
            safety: 5.0,
        }
    }
}

/// The file the house standard is kept in with the house library.
fn house_file() -> Option<std::path::PathBuf> {
    crate::biblioteca::dir().map(|d| d.join("padrao-da-casa.json"))
}

/// The house standard: the library's file when there is one, else the
/// preferences'.
pub fn house_prefs(app: &App) -> HousePrefs {
    house_file()
        .and_then(|f| std::fs::read_to_string(f).ok())
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_else(|| app.settings.arcano_house.clone())
}

fn save_house_prefs(app: &mut App, h: HousePrefs) {
    if let Some(f) = house_file() {
        if let Ok(t) = serde_json::to_string_pretty(&h) {
            let _ = std::fs::write(f, t);
        }
    }
    app.settings.arcano_house = h;
}

// ----- the panel ------------------------------------------------------------

/// The panel: a section per job, each with Check and Fix.
pub fn docker(app: &mut App, ui: &mut egui::Ui) {
    use crate::i18n::{tr, trf};
    use crate::ui::tips::{pair, RichTip as _};
    let scope = if app.selection.is_empty() {
        tr("prelo.scope_all")
    } else {
        tr("prelo.scope_selection")
    };
    ui.label(egui::RichText::new(scope).color(crate::theme::Tokens::text_dim()));
    let result_id = egui::Id::new("prelo_result");
    let job = |ui: &mut egui::Ui,
               app: &mut App,
               title: &str,
               desc: &str,
               key: &str,
               run: &dyn Fn(&mut App, bool) -> Found| {
        ui.add_space(6.0);
        ui.strong(tr(title)).rich_tip(pair(title, desc));
        ui.horizontal(|ui| {
            if ui
                .button(tr("prelo.check"))
                .rich_tip(pair("prelo.check", "prelo.check_desc"))
                .clicked()
            {
                let f = run(app, false);
                ui.data_mut(|d| d.insert_temp(result_id.with(key), Some(f)));
            }
            if ui
                .button(tr("prelo.fix"))
                .rich_tip(pair("prelo.fix", "prelo.fix_desc"))
                .clicked()
            {
                let f = run(app, true);
                ui.data_mut(|d| d.insert_temp(result_id.with(key), Some(f)));
                app.status = trf("prelo.fixed", &[("n", &f.changes.to_string())]);
            }
        });
        if let Some(f) = ui
            .data(|d| d.get_temp::<Option<Found>>(result_id.with(key)))
            .flatten()
        {
            let msg = if f.changes == 0 {
                tr("prelo.nothing")
            } else {
                trf(
                    "prelo.found",
                    &[("n", &f.changes.to_string()), ("t", &f.texts.to_string())],
                )
            };
            ui.label(
                egui::RichText::new(msg)
                    .small()
                    .color(crate::theme::Tokens::text_dim()),
            );
        }
    };
    job(
        ui,
        app,
        "prelo.typography",
        "prelo.typography_desc",
        "typo",
        &|a, fix| a.house_typography(fix),
    );
    job(
        ui,
        app,
        "prelo.local",
        "prelo.local_desc",
        "local",
        &|a, fix| a.house_local_formatting(fix),
    );
    // House standard: its values and Apply.
    ui.add_space(10.0);
    ui.strong(tr("prelo.standard"))
        .rich_tip(pair("prelo.standard", "prelo.standard_desc"));
    let mut h = house_prefs(app);
    let before = h.clone();
    egui::Grid::new("prelo_standard")
        .num_columns(4)
        .spacing([6.0, 4.0])
        .show(ui, |ui| {
            let field = |ui: &mut egui::Ui, key: &str, v: &mut f64| {
                ui.label(tr(key));
                ui.add(
                    egui::DragValue::new(v)
                        .range(0.0..=100.0)
                        .speed(0.5)
                        .custom_formatter(|v, _| {
                            crate::ui::field::format_number(v, false, 0, Some(1))
                        })
                        .suffix(" mm"),
                )
                .rich_tip(pair(key, "prelo.margin_desc"));
            };
            field(ui, "prelo.top", &mut h.top);
            field(ui, "prelo.bottom", &mut h.bottom);
            ui.end_row();
            field(ui, "prelo.inner", &mut h.inner);
            field(ui, "prelo.outer", &mut h.outer);
            ui.end_row();
            field(ui, "prelo.bleed", &mut h.bleed);
            field(ui, "prelo.safety", &mut h.safety);
            ui.end_row();
        });
    if h != before {
        save_house_prefs(app, h);
    }
    let drawing = app.has_document() && !app.show_welcome;
    if ui
        .add_enabled(drawing, egui::Button::new(tr("prelo.apply_standard")))
        .rich_tip(pair("prelo.apply_standard", "prelo.apply_standard_desc"))
        .clicked()
    {
        app.apply_house_standard();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn house_typography() {
        let (t, n) = typography(
            "Ele disse: \"Vamos...\"  E foi - sem pressa , d'água; páginas 10-20 -- fim .",
            None,
        );
        assert_eq!(t, "Ele disse: \u{201C}Vamos\u{2026}\u{201D} E foi \u{2014} sem pressa, d\u{2019}água; páginas 10\u{2013}20 \u{2014} fim.");
        assert!(n >= 8, "{n}");
        // Already right: nothing to do.
        assert_eq!(typography(&t, None).1, 0);
        // A quote at the start of a span after a space in the span before.
        assert_eq!(typography("\"sim\"", Some(' ')).0, "\u{201C}sim\u{201D}");
    }

    #[test]
    fn fixing_keeps_the_formatting_and_is_one_undo_step() {
        use tracedraw_core::{
            geometry::{Point, Size},
            ParagraphStyle, TextAlign,
        };
        let mut app = App::headless();
        let mut bold = TextSpan::new("\"Olá\"", "Sans", 12.0);
        bold.bold = true;
        let id = app
            .new_shape(ShapeKind::Text {
                spans: vec![TextSpan::new("Ela disse  ", "Sans", 12.0), bold],
                origin: Point::new(10.0, 10.0),
                frame: Some(Size::new(100.0, 50.0)),
                align: TextAlign::Left,
                para: ParagraphStyle::default(),
                on_path: None,
            })
            .unwrap();
        app.select(Vec::new());
        let found = app.house_typography(false);
        assert_eq!(
            found,
            Found {
                changes: 3,
                texts: 1
            }
        );
        let steps = app.engine.history_labels().0.len();
        app.house_typography(true);
        assert_eq!(app.engine.history_labels().0.len(), steps + 1);
        let ShapeKind::Text { spans, .. } = &app.doc().find_shape(id).unwrap().kind else {
            unreachable!()
        };
        assert_eq!(spans[0].text, "Ela disse ");
        assert!(spans[1].bold && spans[1].text == "\u{201C}Olá\u{201D}");
        assert_eq!(app.house_typography(false).changes, 0);
    }

    #[test]
    fn local_formatting_is_found_and_cleared() {
        use tracedraw_core::{
            geometry::{Point, Size},
            ParagraphStyle, TextAlign,
        };
        let mut app = App::headless();
        let id = app
            .new_shape(ShapeKind::Text {
                spans: vec![TextSpan::new("Um texto corrido", "Sans", 12.0)],
                origin: Point::new(10.0, 10.0),
                frame: Some(Size::new(100.0, 50.0)),
                align: TextAlign::Left,
                para: ParagraphStyle::default(),
                on_path: None,
            })
            .unwrap();
        app.add_book_styles();
        let body = app.doc().text_styles[0].id;
        app.select(vec![id]);
        app.apply_named_style(ts::TextStyleKind::Paragraph, Some(body));
        assert_eq!(app.house_local_formatting(false).changes, 0);
        // "texto" made 14 pt by hand.
        let mut k = app.doc().find_shape(id).unwrap().kind.clone();
        if let ShapeKind::Text { spans, .. } = &mut k {
            tracedraw_core::document::spans_apply(spans, 3, 8, |sp| sp.size_pt = 14.0);
        }
        app.run(Command::SetShapeKind { shape: id, kind: k });
        assert_eq!(app.house_local_formatting(false).changes, 1);
        app.house_local_formatting(true);
        assert_eq!(app.house_local_formatting(false).changes, 0);
    }

    #[test]
    fn the_house_standard_sets_guides_and_bleed_and_keeps_the_users_guides() {
        let mut app = App::headless();
        let page = app.doc().pages[0].id;
        app.run(Command::SetGuides {
            page,
            guides: vec![Guide::from(GuideLine::Vertical { x: 50.0 })],
        });
        app.apply_house_standard();
        app.apply_house_standard();
        let p = &app.doc().pages[0];
        assert_eq!(
            p.guides.len(),
            1 + 8,
            "the user's guide and the house's, once"
        );
        assert!((app.doc().metadata.bleed - 3.0).abs() < 1e-9);
        assert!(!app.doc().text_styles.is_empty());
        assert!(
            p.guides
                .iter()
                .any(|g| matches!(g.line, GuideLine::Vertical { x } if (x - 22.0).abs() < 1e-9)),
            "inner margin at the left of page 1"
        );
    }
}
