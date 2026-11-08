//! Arcano edition: two more jobs of the House Press panel, both checks
//! with a list that goes to each case (no AI, nothing changed):
//!
//! - Breaks and justification: in paragraph text, looking at the lines as
//!   they are laid out, frame by frame and column by column: a
//!   paragraph's first line alone at the foot of a frame or column
//!   (orphan), its last line alone at the top of the next (widow), a word
//!   alone on a paragraph's last line, more than two hyphenated lines in
//!   a row, a hyphen on the last line of a frame or column, a title (a
//!   paragraph kept with the next) left at the foot, and justified lines
//!   whose spaces are much wider or narrower than a normal space.
//! - Openings and front matter: the house's front matter (half title, a
//!   blank page, title page, credits), chapter openings on right-hand
//!   pages with a drop cap in their first paragraph, an even page count.
//!
//! See docs/behavior/house-press.md.

use crate::app::App;
use crate::i18n::{tr, trf};
use tracedraw_core::paragraphs::paragraph_formats_of;
use tracedraw_core::{Shape, ShapeId, ShapeKind, TextAlign};

/// Spaces wider than this share of a normal space make a loose line.
pub const LOOSE: f64 = 1.33;
/// Spaces narrower than this share of a normal space make a tight line.
pub const TIGHT: f64 = 0.80;
/// More hyphenated lines in a row than this are flagged.
pub const MAX_HYPHENS: usize = 2;

/// What a case of the breaks check is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BreakKind {
    Orphan,
    Widow,
    Runt,
    Hyphens,
    FootHyphen,
    TitleAtFoot,
    Loose,
    Tight,
}

impl BreakKind {
    pub const ALL: [BreakKind; 8] = [
        BreakKind::Orphan,
        BreakKind::Widow,
        BreakKind::Runt,
        BreakKind::Hyphens,
        BreakKind::FootHyphen,
        BreakKind::TitleAtFoot,
        BreakKind::Loose,
        BreakKind::Tight,
    ];

    pub fn key(self) -> &'static str {
        match self {
            BreakKind::Orphan => "prelo.break_orphan",
            BreakKind::Widow => "prelo.break_widow",
            BreakKind::Runt => "prelo.break_runt",
            BreakKind::Hyphens => "prelo.break_hyphens",
            BreakKind::FootHyphen => "prelo.break_foot_hyphen",
            BreakKind::TitleAtFoot => "prelo.break_title_foot",
            BreakKind::Loose => "prelo.break_loose",
            BreakKind::Tight => "prelo.break_tight",
        }
    }
}

/// One line of a laid-out frame: its characters and its baseline (Y up).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineAt {
    pub start: usize,
    pub end: usize,
    pub baseline: f64,
}

/// A case found: what, and the characters of the text it is about.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Case {
    pub kind: BreakKind,
    pub start: usize,
    pub end: usize,
}

/// The paragraphs of `text` as (start, end) character ranges, the line
/// break that ends one not included.
fn paragraphs(text: &[char]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut s = 0;
    for (i, c) in text.iter().enumerate() {
        if *c == '\n' {
            out.push((s, i));
            s = i + 1;
        }
    }
    out.push((s, text.len()));
    out
}

/// The cases of one frame. `from_prev`: the frame's first paragraph began
/// in the frame before; `to_next`: its last one goes on in the next;
/// `keep_next(p)`: paragraph `p` is kept with the next (a title).
pub fn frame_cases(
    text: &[char],
    lines: &[LineAt],
    from_prev: bool,
    to_next: bool,
    keep_next: &dyn Fn(usize) -> bool,
) -> Vec<Case> {
    let paras = paragraphs(text);
    let para_of = |c: usize| paras.iter().position(|&(s, e)| c >= s && c <= e);
    let is_word = |c: Option<&char>| c.is_some_and(|c| c.is_alphanumeric());
    let mut out = Vec::new();
    let push = |out: &mut Vec<Case>, kind, l: &LineAt| {
        out.push(Case {
            kind,
            start: l.start,
            end: l.end,
        })
    };
    // Lines of empty paragraphs carry nothing to check.
    let lines: Vec<LineAt> = lines
        .iter()
        .copied()
        .filter(|l| text.get(l.start..l.end).is_some_and(|t| t.iter().any(|c| !c.is_whitespace())))
        .collect();
    // Columns: a line higher than the one before starts a new column.
    let mut segments: Vec<Vec<LineAt>> = Vec::new();
    for l in &lines {
        match segments.last_mut() {
            Some(seg) if seg.last().is_some_and(|p| l.baseline <= p.baseline + 0.01) => seg.push(*l),
            _ => segments.push(vec![*l]),
        }
    }
    let n = segments.len();
    let mut hyphen_run: Vec<LineAt> = Vec::new();
    let flush = |out: &mut Vec<Case>, run: &mut Vec<LineAt>| {
        if run.len() > MAX_HYPHENS {
            for l in run.iter() {
                push(out, BreakKind::Hyphens, l);
            }
        }
        run.clear();
    };
    for (k, seg) in segments.iter().enumerate() {
        let first_seg = k == 0;
        let last_seg = k + 1 == n;
        for (j, l) in seg.iter().enumerate() {
            let Some(p) = para_of(l.start) else {
                continue;
            };
            let Some(&(ps, pe)) = paras.get(p) else {
                continue;
            };
            let begun_before = l.start > ps || (p == 0 && from_prev);
            let goes_on = l.end < pe || (p + 1 == paras.len() && to_next);
            let seg_first = j == 0;
            let seg_last = j + 1 == seg.len();
            let breaks_after = !last_seg || to_next;
            let breaks_before = !first_seg || from_prev;
            // Broken inside a word: an automatic hyphen ends the line.
            let hyphenated =
                l.end < pe && is_word(text.get(l.end.wrapping_sub(1))) && is_word(text.get(l.end));
            if hyphenated {
                hyphen_run.push(*l);
            } else {
                flush(&mut out, &mut hyphen_run);
            }
            if seg_last && breaks_after {
                if !begun_before && goes_on {
                    push(&mut out, BreakKind::Orphan, l);
                }
                if hyphenated {
                    push(&mut out, BreakKind::FootHyphen, l);
                }
                if l.end >= pe && keep_next(p) && !(p + 1 == paras.len() && to_next) {
                    push(&mut out, BreakKind::TitleAtFoot, l);
                }
            }
            if seg_first && breaks_before && begun_before && !goes_on {
                push(&mut out, BreakKind::Widow, l);
            }
            // A word alone on the last line of a paragraph of more lines.
            if l.end >= pe && !goes_on && begun_before {
                let t: String = text.get(l.start..l.end).unwrap_or(&[]).iter().collect();
                let t = t.trim();
                if !t.is_empty() && !t.contains(char::is_whitespace) {
                    push(&mut out, BreakKind::Runt, l);
                }
            }
        }
    }
    flush(&mut out, &mut hyphen_run);
    out
}

/// How wide the spaces of a justified line are against a normal space:
/// `actual` the line's width, `packed` its width with normal spaces and
/// `bare` without its spaces, over `spaces` spaces (the widths of the
/// spaces alone: `bare` 0 and one space). `None` with no space.
pub fn space_ratio(actual: f64, packed: f64, bare: f64, spaces: usize) -> Option<f64> {
    let normal = (packed - bare) / spaces as f64;
    if spaces == 0 || normal <= 1e-6 || !actual.is_finite() {
        return None;
    }
    Some(((actual - bare) / spaces as f64) / normal)
}

/// A case with where it is, for the list.
#[derive(Debug, Clone, PartialEq)]
pub struct Found {
    pub kind: BreakKind,
    pub shape: ShapeId,
    pub page: usize,
    pub start: usize,
    pub end: usize,
    /// The words of the line, cut short.
    pub excerpt: String,
}

/// Every paragraph text with the index of its page (master layers left
/// out), in page order, front to back on each page.
fn page_texts(app: &App, only: &[ShapeId]) -> Vec<(usize, Shape)> {
    fn walk(s: &Shape, page: usize, out: &mut Vec<(usize, Shape)>) {
        match &s.kind {
            ShapeKind::Group { children } => children.iter().for_each(|c| walk(c, page, out)),
            ShapeKind::ClipFrame { frame, contents } => {
                walk(frame, page, out);
                contents.iter().for_each(|c| walk(c, page, out));
            }
            ShapeKind::Text { frame: Some(_), .. } => out.push((page, s.clone())),
            _ => {}
        }
    }
    let mut out = Vec::new();
    for (pi, page) in app.doc().pages.iter().enumerate() {
        for l in &page.layers {
            for s in &l.shapes {
                if only.is_empty() || only.contains(&s.id) {
                    walk(s, pi, &mut out);
                }
            }
        }
    }
    out
}

fn excerpt(text: &[char], s: usize, e: usize) -> String {
    let t: String = text.get(s..e).unwrap_or(&[]).iter().collect();
    let t = t.trim();
    let mut short: String = t.chars().take(48).collect();
    if t.chars().count() > 48 {
        short.push('\u{2026}');
    }
    short
}

/// The text `s` with every paragraph set flush left: laid out, its lines
/// break where the justified ones do, with spaces of their own width.
pub(crate) fn flush_left(s: &Shape) -> Option<tracedraw_text::TextLayout> {
    let mut s = s.clone();
    if let ShapeKind::Text { spans, align, .. } = &mut s.kind {
        *align = TextAlign::Left;
        for sp in spans.iter_mut() {
            if let Some(p) = sp.paragraph.as_mut() {
                p.align = TextAlign::Left;
            }
        }
    }
    crate::text_editing::layout_of_text_shape(&s, Default::default()).map(|(l, _)| l)
}

/// The total width of the spaces of a line from `start` to `end` (the
/// characters `text` holds), read off its carets.
pub(crate) fn space_width(text: &[char], b: &tracedraw_text::LineBox, end: usize) -> f64 {
    (b.start_char..end)
        .filter(|&i| text.get(i) == Some(&' '))
        .filter_map(|i| {
            let k = i - b.start_char;
            Some(b.edges.get(k + 1)? - b.edges.get(k)?)
        })
        .sum()
}

impl App {
    /// The breaks and justification cases of the selected frames (else
    /// every paragraph text on the pages).
    pub fn house_breaks(&self) -> Vec<Found> {
        let only: Vec<ShapeId> = self.selection.clone();
        let mut out = Vec::new();
        for (page, s) in page_texts(self, &only) {
            let ShapeKind::Text {
                spans, align, para, ..
            } = &s.kind
            else {
                continue;
            };
            let text: Vec<char> = spans.iter().flat_map(|sp| sp.text.chars()).collect();
            let Some((layout, _)) = crate::text_editing::layout_of_text_shape(&s, Default::default())
            else {
                continue;
            };
            let ends_paragraph = |id: ShapeId| match self.doc().find_shape(id).map(|s| &s.kind) {
                Some(ShapeKind::Text { spans, .. }) => {
                    spans.iter().rev().find(|sp| !sp.text.is_empty()).is_none_or(|sp| sp.text.ends_with('\n'))
                }
                _ => true,
            };
            let from_prev = self.prev_frame(s.id).is_some_and(|p| !ends_paragraph(p));
            let to_next = self.next_frame(s.id).is_some() && !text.last().is_some_and(|c| *c == '\n');
            let formats = paragraph_formats_of(spans, *align, para);
            let keep = |p: usize| formats.get(p).is_some_and(|f| f.keep_with_next);
            let lines: Vec<LineAt> = layout
                .lines
                .iter()
                .map(|l| LineAt {
                    start: l.start_char,
                    end: l.end_char,
                    baseline: l.baseline,
                })
                .collect();
            // Only what lies in the frame: the rest overflows.
            let lines: Vec<LineAt> = lines.into_iter().filter(|l| l.start < layout.fitted_chars.max(1)).collect();
            for c in frame_cases(&text, &lines, from_prev, to_next, &keep) {
                out.push(Found {
                    kind: c.kind,
                    shape: s.id,
                    page,
                    start: c.start,
                    end: c.end,
                    excerpt: excerpt(&text, c.start, c.end),
                });
            }
            // Loose and tight justified lines (not a paragraph's last):
            // their spaces against the same line set flush left.
            let left = flush_left(&s);
            let paras = paragraphs(&text);
            for b in &layout.lines {
                let Some(p) = paras.iter().position(|&(ps, pe)| b.start_char >= ps && b.start_char <= pe) else {
                    continue;
                };
                let f = formats.get(p);
                let justified = f.is_some_and(|f| matches!(f.align, TextAlign::Justify | TextAlign::ForceJustify));
                let last = paras.get(p).is_some_and(|&(_, pe)| b.end_char >= pe);
                if !justified || (last && f.is_some_and(|f| f.align == TextAlign::Justify)) {
                    continue;
                }
                let mut e = b.end_char;
                while e > b.start_char && text.get(e - 1).is_some_and(|c| c.is_whitespace()) {
                    e -= 1;
                }
                let spaces = text.get(b.start_char..e).map_or(0, |t| t.iter().filter(|c| **c == ' ').count());
                if spaces == 0 {
                    continue;
                }
                let Some(natural) = left
                    .as_ref()
                    .and_then(|l| l.lines.iter().find(|n| n.start_char == b.start_char && n.end_char == b.end_char))
                else {
                    continue;
                };
                let actual = space_width(&text, b, e);
                let plain = space_width(&text, natural, e);
                if let Some(r) = space_ratio(actual, plain, 0.0, 1) {
                    let kind = if r > LOOSE {
                        BreakKind::Loose
                    } else if r < TIGHT {
                        BreakKind::Tight
                    } else {
                        continue;
                    };
                    out.push(Found {
                        kind,
                        shape: s.id,
                        page,
                        start: b.start_char,
                        end: b.end_char,
                        excerpt: excerpt(&text, b.start_char, b.end_char),
                    });
                }
            }
        }
        out
    }

    /// Go to a case: its page, the text being edited with the line chosen.
    pub fn show_break(&mut self, f: &Found) {
        if f.page != self.page_index() {
            self.goto_page(f.page);
        }
        if self.doc().find_shape(f.shape).is_none() {
            return;
        }
        self.begin_text_edit(f.shape);
        if let Some(te) = self.text_edit.as_mut() {
            te.anchor = f.start;
            te.caret = f.end;
        }
    }
}

// ----- openings and front matter -------------------------------------------

/// One line of the openings checklist.
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    /// The key of what is checked.
    pub key: &'static str,
    /// The page it is about, when there is one.
    pub page: Option<usize>,
    /// The text it found, for openings.
    pub detail: String,
    pub ok: bool,
}

/// Whether a page holds anything that prints (master layers left out).
fn page_has_content(page: &tracedraw_core::Page) -> bool {
    page.layers.iter().any(|l| l.printable && !l.shapes.is_empty())
}

fn page_has_text(page: &tracedraw_core::Page) -> bool {
    fn text(s: &Shape) -> bool {
        match &s.kind {
            ShapeKind::Text { spans, .. } => spans.iter().any(|sp| !sp.text.trim().is_empty()),
            ShapeKind::Group { children } => children.iter().any(text),
            ShapeKind::ClipFrame { frame, contents } => text(frame) || contents.iter().any(text),
            _ => false,
        }
    }
    page.layers.iter().any(|l| l.printable && l.shapes.iter().any(text))
}

impl App {
    /// The checklist of the front matter and the chapter openings.
    pub fn house_openings(&self) -> Vec<Item> {
        let doc = self.doc();
        let pages = &doc.pages;
        let mut out = Vec::new();
        let item = |key, page: Option<usize>, ok: bool| Item {
            key,
            page,
            detail: String::new(),
            ok,
        };
        out.push(item("prelo.open_four_pages", None, pages.len() >= 4));
        let has = |i: usize, f: fn(&tracedraw_core::Page) -> bool| pages.get(i).is_some_and(f);
        out.push(item("prelo.open_half_title", Some(0), has(0, page_has_text)));
        out.push(item(
            "prelo.open_blank_verso",
            Some(1),
            pages.get(1).is_some_and(|p| !page_has_content(p)),
        ));
        out.push(item("prelo.open_title_page", Some(2), has(2, page_has_text)));
        out.push(item("prelo.open_credits", Some(3), has(3, page_has_text)));
        out.push(item("prelo.open_even_count", None, pages.len() % 2 == 0));
        // Chapter openings: paragraphs of a paragraph style named like the
        // book set's chapter title.
        let chapter = tr("tstyle.book_chapter");
        let chapter_ids: Vec<u32> = doc
            .text_styles
            .iter()
            .filter(|s| s.name == chapter)
            .map(|s| s.id)
            .collect();
        if chapter_ids.is_empty() {
            out.push(item("prelo.open_no_chapter_style", None, false));
            return out;
        }
        for (page, s) in page_texts(self, &[]) {
            let ShapeKind::Text {
                spans, align, para, ..
            } = &s.kind
            else {
                continue;
            };
            let text: Vec<char> = spans.iter().flat_map(|sp| sp.text.chars()).collect();
            let paras = paragraphs(&text);
            let formats = paragraph_formats_of(spans, *align, para);
            for (i, f) in formats.iter().enumerate() {
                if !f.style.is_some_and(|id| chapter_ids.contains(&id)) {
                    continue;
                }
                let title = paras
                    .get(i)
                    .map(|&(s, e)| excerpt(&text, s, e))
                    .unwrap_or_default();
                let right = page % 2 == 0;
                out.push(Item {
                    key: "prelo.open_right_hand",
                    page: Some(page),
                    detail: title.clone(),
                    ok: right,
                });
                // The first paragraph with words after the title.
                let next = (i + 1..formats.len()).find(|&j| {
                    paras
                        .get(j)
                        .and_then(|&(s, e)| text.get(s..e))
                        .is_some_and(|t| t.iter().any(|c| !c.is_whitespace()))
                });
                if let Some(j) = next {
                    let cap = formats.get(j).is_some_and(|f| f.drop_cap_lines > 1);
                    out.push(Item {
                        key: "prelo.open_drop_cap",
                        page: Some(page),
                        detail: title,
                        ok: cap,
                    });
                }
            }
        }
        out
    }
}

// ----- the panel's two sections ---------------------------------------------

fn kind_counts(found: &[Found]) -> Vec<(BreakKind, usize)> {
    BreakKind::ALL
        .iter()
        .map(|k| (*k, found.iter().filter(|f| f.kind == *k).count()))
        .filter(|(_, n)| *n > 0)
        .collect()
}

/// The Breaks and justification section of the House Press panel.
pub fn breaks_section(app: &mut App, ui: &mut egui::Ui) {
    use crate::ui::tips::{pair, RichTip as _};
    let id = egui::Id::new("prelo_breaks");
    ui.add_space(10.0);
    ui.strong(tr("prelo.breaks"))
        .rich_tip(pair("prelo.breaks", "prelo.breaks_desc"));
    let drawing = app.has_document() && !app.show_welcome;
    if ui
        .add_enabled(drawing, egui::Button::new(tr("prelo.check")))
        .rich_tip(pair("prelo.check", "prelo.breaks_check_desc"))
        .clicked()
    {
        let found = app.house_breaks();
        ui.data_mut(|d| d.insert_temp(id, Some(found)));
    }
    let Some(found) = ui.data(|d| d.get_temp::<Option<Vec<Found>>>(id)).flatten() else {
        return;
    };
    if found.is_empty() {
        ui.label(egui::RichText::new(tr("prelo.breaks_none")).small().color(crate::theme::Tokens::text_dim()));
        return;
    }
    let summary: Vec<String> = kind_counts(&found)
        .iter()
        .map(|(k, n)| format!("{} {n}", tr(k.key())))
        .collect();
    ui.label(
        egui::RichText::new(summary.join(" \u{00B7} "))
            .small()
            .color(crate::theme::Tokens::text_dim()),
    );
    let mut go = None;
    egui::ScrollArea::vertical()
        .id_salt("prelo_breaks_list")
        .max_height(220.0)
        .show(ui, |ui| {
            for (i, f) in found.iter().enumerate() {
                let label = trf(
                    "prelo.case_line",
                    &[
                        ("kind", &tr(f.kind.key())),
                        ("page", &(f.page + 1).to_string()),
                        ("text", &f.excerpt),
                    ],
                );
                if ui
                    .add(egui::Button::new(egui::RichText::new(label).small()).frame(false).wrap())
                    .rich_tip(pair(f.kind.key(), &format!("{}_desc", f.kind.key())))
                    .clicked()
                {
                    go = Some(i);
                }
            }
        });
    if let Some(f) = go.and_then(|i| found.get(i)) {
        app.show_break(f);
    }
}

/// The Openings and front matter section of the House Press panel.
pub fn openings_section(app: &mut App, ui: &mut egui::Ui) {
    use crate::ui::tips::{pair, RichTip as _};
    let id = egui::Id::new("prelo_openings");
    ui.add_space(10.0);
    ui.strong(tr("prelo.openings"))
        .rich_tip(pair("prelo.openings", "prelo.openings_desc"));
    let drawing = app.has_document() && !app.show_welcome;
    if ui
        .add_enabled(drawing, egui::Button::new(tr("prelo.check")))
        .rich_tip(pair("prelo.check", "prelo.openings_check_desc"))
        .clicked()
    {
        let items = app.house_openings();
        ui.data_mut(|d| d.insert_temp(id, Some(items)));
    }
    let Some(items) = ui.data(|d| d.get_temp::<Option<Vec<Item>>>(id)).flatten() else {
        return;
    };
    let mut go = None;
    for it in &items {
        ui.horizontal(|ui| {
            let (mark, color) = if it.ok {
                ("\u{2713}", crate::theme::Tokens::accent())
            } else {
                ("\u{00D7}", egui::Color32::from_rgb(0xC0, 0x4A, 0x3A))
            };
            ui.label(egui::RichText::new(mark).color(color).strong());
            let mut text = tr(it.key);
            if !it.detail.is_empty() {
                text = format!("{text}: {}", it.detail);
            }
            if let Some(p) = it.page {
                text = trf("prelo.on_page", &[("text", &text), ("page", &(p + 1).to_string())]);
            }
            let r = ui
                .add(egui::Button::new(egui::RichText::new(text).small()).frame(false).wrap())
                .rich_tip(pair(it.key, &format!("{}_desc", it.key)));
            if r.clicked() {
                go = it.page;
            }
        });
    }
    if let Some(p) = go {
        if p < app.doc().pages.len() {
            app.goto_page(p);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chars(s: &str) -> Vec<char> {
        s.chars().collect()
    }

    /// Lines over `text` as given by their (start, end), one column.
    fn lines(spans: &[(usize, usize)]) -> Vec<LineAt> {
        spans
            .iter()
            .enumerate()
            .map(|(i, &(s, e))| LineAt {
                start: s,
                end: e,
                baseline: -(i as f64) * 5.0,
            })
            .collect()
    }

    fn kinds(c: &[Case]) -> Vec<BreakKind> {
        c.iter().map(|c| c.kind).collect()
    }

    #[test]
    fn an_orphan_at_the_foot_and_a_widow_at_the_top() {
        // "aaa bbb ccc" broken in three lines, the frame ending after the
        // first: an orphan at its foot.
        let t = chars("Um.\naaa bbb ccc");
        let l = lines(&[(0, 3), (4, 8)]);
        let c = frame_cases(&t, &l, false, true, &|_| false);
        assert_eq!(kinds(&c), vec![BreakKind::Orphan]);
        assert_eq!((c[0].start, c[0].end), (4, 8));
        // The next frame starts with that paragraph's last line alone.
        let t = chars("ccc ddd\nDois.");
        let l = lines(&[(0, 7), (8, 13)]);
        let c = frame_cases(&t, &l, true, false, &|_| false);
        assert!(kinds(&c).contains(&BreakKind::Widow), "{c:?}");
        // Not when the frame starts a paragraph.
        let c = frame_cases(&t, &l, false, false, &|_| false);
        assert!(!kinds(&c).contains(&BreakKind::Widow));
    }

    #[test]
    fn columns_break_like_frames() {
        // Two columns in one frame: the second starts higher.
        let t = chars("aaa bbb ccc ddd");
        let l = vec![
            LineAt { start: 0, end: 8, baseline: 0.0 },
            LineAt { start: 8, end: 12, baseline: -5.0 },
            LineAt { start: 12, end: 15, baseline: 0.0 },
        ];
        let c = frame_cases(&t, &l, false, false, &|_| false);
        assert_eq!(kinds(&c), vec![BreakKind::Widow, BreakKind::Runt]);
    }

    #[test]
    fn runts_hyphens_and_titles_at_the_foot() {
        // A word alone on a paragraph's last line.
        let t = chars("palavras e mais palavras fim");
        let l = lines(&[(0, 25), (25, 28)]);
        assert_eq!(kinds(&frame_cases(&t, &l, false, false, &|_| false)), vec![BreakKind::Runt]);
        // Three hyphenated lines in a row (broken inside words), the
        // last of them at the foot of a frame that goes on.
        let t = chars("abcdefghijkl mnop");
        let l = lines(&[(0, 3), (3, 6), (6, 9)]);
        let c = frame_cases(&t, &l, false, true, &|_| false);
        let k = kinds(&c);
        assert_eq!(k.iter().filter(|k| **k == BreakKind::Hyphens).count(), 3, "{k:?}");
        assert!(k.contains(&BreakKind::FootHyphen));
        // Two in a row are allowed.
        let l = lines(&[(0, 3), (3, 6), (6, 17)]);
        let c = frame_cases(&t, &l, false, false, &|_| false);
        assert!(!kinds(&c).contains(&BreakKind::Hyphens));
        // A title kept with the next, left as the frame's last line.
        let t = chars("Texto.\nCapítulo 2\nMais");
        let l = lines(&[(0, 6), (7, 17)]);
        let c = frame_cases(&t, &l, false, true, &|p| p == 1);
        assert!(kinds(&c).contains(&BreakKind::TitleAtFoot), "{c:?}");
    }

    #[test]
    fn the_space_ratio_compares_with_a_normal_space() {
        // Two spaces of 1 mm stretched to 3 mm each.
        assert!((space_ratio(16.0, 12.0, 10.0, 2).unwrap() - 3.0).abs() < 1e-9);
        assert!((space_ratio(12.0, 12.0, 10.0, 2).unwrap() - 1.0).abs() < 1e-9);
        assert!(space_ratio(12.0, 12.0, 10.0, 0).is_none());
    }

    /// A justified line opened wide by a long word after it is loose; a
    /// justified text never comes out tight (the spacing only grows), and
    /// flush left text has no loose lines.
    #[test]
    fn loose_lines_are_measured_against_the_same_line_flush_left() {
        use tracedraw_core::{
            geometry::{Point, Size},
            ParagraphStyle, TextSpan,
        };
        if tracedraw_text::fonts().families().is_empty() {
            return;
        }
        let mut app = App::headless();
        let text = "Um dois tr\u{EA}s quatro cinco seis sete oito nove dez onze doze treze. \
                    Ele sem Constitucionalissimamente fim de todo o texto que segue aqui.";
        let make = |app: &mut App, align| {
            app.new_shape(ShapeKind::Text {
                spans: vec![TextSpan::new(text, "DejaVu Serif", 10.0)],
                origin: Point::new(10.0, 10.0),
                frame: Some(Size::new(45.0, 120.0)),
                align,
                para: ParagraphStyle::default(),
                on_path: None,
            })
            .unwrap()
        };
        let justified = make(&mut app, TextAlign::Justify);
        app.text_edit = None;
        app.select(vec![justified]);
        let found = app.house_breaks();
        assert!(found.iter().any(|f| f.kind == BreakKind::Loose), "{found:?}");
        assert!(!found.iter().any(|f| f.kind == BreakKind::Tight), "{found:?}");
        let left = make(&mut app, TextAlign::Left);
        app.text_edit = None;
        app.select(vec![left]);
        assert!(!app.house_breaks().iter().any(|f| matches!(f.kind, BreakKind::Loose | BreakKind::Tight)));
    }

    #[test]
    fn the_openings_list_checks_front_matter_and_chapters() {
        use tracedraw_core::{
            geometry::{Point, Size},
            ParagraphStyle, TextSpan,
        };
        let mut app = App::headless();
        let items = app.house_openings();
        assert!(!items.iter().find(|i| i.key == "prelo.open_four_pages").unwrap().ok);
        for _ in 0..4 {
            app.add_page();
        }
        app.add_book_styles();
        let chapter = app
            .doc()
            .text_styles
            .iter()
            .find(|s| s.name == tr("tstyle.book_chapter"))
            .unwrap()
            .id;
        // A chapter title on page 2 (a left-hand page).
        app.goto_page(1);
        let id = app
            .new_shape(ShapeKind::Text {
                spans: vec![TextSpan::new("Capítulo 1\nEra uma vez.", "Sans", 12.0)],
                origin: Point::new(10.0, 10.0),
                frame: Some(Size::new(100.0, 50.0)),
                align: TextAlign::Left,
                para: ParagraphStyle::default(),
                on_path: None,
            })
            .unwrap();
        app.select(vec![id]);
        app.text_edit = None;
        app.apply_named_style(tracedraw_core::text_styles::TextStyleKind::Paragraph, Some(chapter));
        let items = app.house_openings();
        let right = items.iter().find(|i| i.key == "prelo.open_right_hand").unwrap();
        assert!(!right.ok, "{items:?}");
        assert_eq!(right.page, Some(1));
        assert!(items.iter().find(|i| i.key == "prelo.open_blank_verso").unwrap().ok == false);
    }
}

