//! Named text styles: paragraph styles and character styles, kept with
//! the drawing (`Document::text_styles`). A paragraph linked to a
//! paragraph style carries its id in its format
//! ([`ParagraphFormat::style`]); characters linked to a character style
//! carry its id ([`TextSpan::char_style`]).
//!
//! A style sets only what it names: a paragraph style its paragraph
//! format (alignment, spacing, indents, tabs...) and the character
//! properties it lists; a character style only character properties.
//! A style can be based on another: what it does not set it takes from
//! that one, and changing the parent changes it too.
//!
//! Applying a paragraph style gives the paragraphs its format and its
//! characters its character properties, with any character style linked
//! to them on top. Changing a style (`Command::SetTextStyles`) applies
//! it again everywhere it is linked, in every page, the master page,
//! groups and clip frames: what a style sets wins over formatting given
//! by hand; what it does not set stays. A link to a style that is gone is
//! dropped and the text keeps its formatting. See
//! docs/behavior/text-styles.md.

use crate::document::{spans_apply, Shape, ShapeKind, TextSpan};
use crate::paragraphs::{
    edit_paragraph_formats, paragraph_format, paragraph_of, paragraph_ranges, ParagraphFormat,
};
use crate::style::Fill;
use serde::{Deserialize, Serialize};

/// What a style is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextStyleKind {
    #[default]
    Paragraph,
    Character,
}

/// The character properties a style sets; `None` = not set (taken from
/// the style it is based on, else left as the text has it).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CharFormat {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_pt: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bold: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub italic: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub underline: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strikethrough: Option<bool>,
    /// Range kerning, percent of the em.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tracking_pct: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub baseline_shift_pt: Option<f64>,
    /// The characters' colour; `Some(None)` = the object's fill.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fill: Option<Option<Fill>>,
    /// OpenType features (small caps, old-style figures...).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub features: Option<Vec<String>>,
}

impl CharFormat {
    /// `self` with what `over` sets laid on top.
    pub fn with(&self, over: &CharFormat) -> CharFormat {
        CharFormat {
            font_family: over
                .font_family
                .clone()
                .or_else(|| self.font_family.clone()),
            size_pt: over.size_pt.or(self.size_pt),
            bold: over.bold.or(self.bold),
            italic: over.italic.or(self.italic),
            underline: over.underline.or(self.underline),
            strikethrough: over.strikethrough.or(self.strikethrough),
            tracking_pct: over.tracking_pct.or(self.tracking_pct),
            baseline_shift_pt: over.baseline_shift_pt.or(self.baseline_shift_pt),
            fill: over.fill.clone().or_else(|| self.fill.clone()),
            features: over.features.clone().or_else(|| self.features.clone()),
        }
    }

    /// Give `sp` what this sets.
    pub fn apply(&self, sp: &mut TextSpan) {
        if let Some(v) = &self.font_family {
            sp.font_family = v.clone();
        }
        if let Some(v) = self.size_pt {
            sp.size_pt = v;
        }
        if let Some(v) = self.bold {
            sp.bold = v;
        }
        if let Some(v) = self.italic {
            sp.italic = v;
        }
        if let Some(v) = self.underline {
            sp.underline = v;
        }
        if let Some(v) = self.strikethrough {
            sp.strikethrough = v;
        }
        if let Some(v) = self.tracking_pct {
            sp.tracking_pct = v;
        }
        if let Some(v) = self.baseline_shift_pt {
            sp.baseline_shift_pt = v;
        }
        if let Some(v) = &self.fill {
            sp.fill = v.clone();
        }
        if let Some(v) = &self.features {
            sp.features = v.clone();
        }
    }

    /// Everything `sp` has, set (a style made from the text).
    pub fn of(sp: &TextSpan) -> CharFormat {
        CharFormat {
            font_family: Some(sp.font_family.clone()),
            size_pt: Some(sp.size_pt),
            bold: Some(sp.bold),
            italic: Some(sp.italic),
            underline: Some(sp.underline),
            strikethrough: Some(sp.strikethrough),
            tracking_pct: Some(sp.tracking_pct),
            baseline_shift_pt: Some(sp.baseline_shift_pt),
            fill: Some(sp.fill.clone()),
            features: Some(sp.features.clone()),
        }
    }

    /// Only what differs from `base` (a character style made from text
    /// inside a paragraph: what sets it apart).
    pub fn differing_from(&self, base: &CharFormat) -> CharFormat {
        fn keep<T: PartialEq + Clone>(a: &Option<T>, b: &Option<T>) -> Option<T> {
            if a != b {
                a.clone()
            } else {
                None
            }
        }
        CharFormat {
            font_family: keep(&self.font_family, &base.font_family),
            size_pt: keep(&self.size_pt, &base.size_pt),
            bold: keep(&self.bold, &base.bold),
            italic: keep(&self.italic, &base.italic),
            underline: keep(&self.underline, &base.underline),
            strikethrough: keep(&self.strikethrough, &base.strikethrough),
            tracking_pct: keep(&self.tracking_pct, &base.tracking_pct),
            baseline_shift_pt: keep(&self.baseline_shift_pt, &base.baseline_shift_pt),
            fill: keep(&self.fill, &base.fill),
            features: keep(&self.features, &base.features),
        }
    }

    pub fn is_empty(&self) -> bool {
        *self == CharFormat::default()
    }

    /// `self`'s values for the fields `mask` sets, nothing else.
    pub fn only(&self, mask: &CharFormat) -> CharFormat {
        fn pick<T: Clone, U>(v: &Option<T>, m: &Option<U>) -> Option<T> {
            m.as_ref().and(v.clone())
        }
        CharFormat {
            font_family: pick(&self.font_family, &mask.font_family),
            size_pt: pick(&self.size_pt, &mask.size_pt),
            bold: pick(&self.bold, &mask.bold),
            italic: pick(&self.italic, &mask.italic),
            underline: pick(&self.underline, &mask.underline),
            strikethrough: pick(&self.strikethrough, &mask.strikethrough),
            tracking_pct: pick(&self.tracking_pct, &mask.tracking_pct),
            baseline_shift_pt: pick(&self.baseline_shift_pt, &mask.baseline_shift_pt),
            fill: pick(&self.fill, &mask.fill),
            features: pick(&self.features, &mask.features),
        }
    }
}

/// The look of paragraph `s..e`'s plain characters: its first characters
/// outside `skip` without a character style (else its first), with its
/// paragraph style on top.
fn plain_look(
    spans: &[TextSpan],
    s: usize,
    e: usize,
    skip: (usize, usize),
    style: Option<CharFormat>,
) -> Option<CharFormat> {
    let mut pos = 0;
    let mut first = None;
    let mut plain = None;
    for sp in spans {
        let n = sp.text.chars().count();
        let (a, b) = (pos, pos + n);
        pos = b;
        if b <= s || a >= e.max(s + 1) || n == 0 {
            continue;
        }
        first.get_or_insert(sp);
        let outside = b <= skip.0 || a >= skip.1;
        if outside && sp.char_style.is_none() {
            plain = Some(sp);
            break;
        }
    }
    let sp = plain.or(first).or(spans.first())?;
    let mut look = sp.clone();
    if let Some(st) = style {
        st.apply(&mut look);
    }
    Some(CharFormat::of(&look))
}

/// A named text style.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct TextStyle {
    /// Unique in the drawing; 0 = give it a free one.
    pub id: u32,
    pub name: String,
    pub kind: TextStyleKind,
    /// The style this one takes what it does not set from (same kind).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub based_on: Option<u32>,
    /// A paragraph style's paragraph format; `None` = the parent's, else
    /// the paragraph keeps its own.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub para: Option<ParagraphFormat>,
    pub chars: CharFormat,
}

/// The style `id` in `styles`.
pub fn find(styles: &[TextStyle], id: u32) -> Option<&TextStyle> {
    styles.iter().find(|s| s.id == id)
}

/// The chain from the root parent down to `id` (a loop or a missing
/// parent ends it).
fn chain(styles: &[TextStyle], id: u32) -> Vec<&TextStyle> {
    let mut out = Vec::new();
    let mut at = find(styles, id);
    while let Some(s) = at {
        if out.iter().any(|o: &&TextStyle| o.id == s.id) || out.len() > 32 {
            break;
        }
        out.push(s);
        at = s
            .based_on
            .and_then(|p| find(styles, p))
            .filter(|p| p.kind == s.kind);
    }
    out.reverse();
    out
}

/// What style `id` sets, its parents' settings included: the paragraph
/// format (paragraph styles) and the character properties.
pub fn resolve(styles: &[TextStyle], id: u32) -> (Option<ParagraphFormat>, CharFormat) {
    let mut para = None;
    let mut chars = CharFormat::default();
    for s in chain(styles, id) {
        if s.para.is_some() {
            para = s.para.clone();
        }
        chars = chars.with(&s.chars);
    }
    (para, chars)
}

/// Whether making `id` based on `parent` would make a loop.
pub fn would_loop(styles: &[TextStyle], id: u32, parent: u32) -> bool {
    chain(styles, parent).iter().any(|s| s.id == id)
}

/// The styles based on `id`, directly or not.
pub fn children_of(styles: &[TextStyle], id: u32) -> Vec<u32> {
    styles
        .iter()
        .filter(|s| s.id != id && chain(styles, s.id).iter().any(|c| c.id == id))
        .map(|s| s.id)
        .collect()
}

/// Give the paragraphs of text `kind` that the characters `range` touch
/// (`None`: all of them) the paragraph style `id` (`None`: no style, the
/// formatting stays). Character styles linked inside them stay on top.
pub fn apply_paragraph_style(
    kind: &mut ShapeKind,
    range: Option<(usize, usize)>,
    styles: &[TextStyle],
    id: Option<u32>,
) {
    let total = match kind {
        ShapeKind::Text { spans, .. } => crate::document::spans_char_count(spans),
        _ => return,
    };
    // Whole paragraphs, so the link is carried by each.
    let range = range.unwrap_or((0, total));
    let Some(id) = id else {
        edit_paragraph_formats(kind, Some(range), |f| f.style = None);
        return;
    };
    let (para, chars) = resolve(styles, id);
    edit_paragraph_formats(kind, Some(range), |f| {
        if let Some(p) = &para {
            *f = p.clone();
        }
        f.style = Some(id);
    });
    let ShapeKind::Text { spans, .. } = kind else {
        return;
    };
    let ranges = paragraph_ranges(spans);
    for i in crate::paragraphs::paragraphs_in(spans, range.0, range.1) {
        let Some(&(s, e)) = ranges.get(i) else {
            continue;
        };
        let end = (e + 1).min(total);
        spans_apply(spans, s, end.max(s), |sp| {
            chars.apply(sp);
            if let Some(c) = sp.char_style {
                resolve(styles, c).1.apply(sp);
            }
        });
    }
    *spans = crate::document::merge_equal_spans(std::mem::take(spans));
}

/// Give the characters `from..to` of text `kind` the character style
/// `id`; `None` takes the link off and gives them back their
/// paragraph's style.
pub fn apply_char_style(
    kind: &mut ShapeKind,
    from: usize,
    to: usize,
    styles: &[TextStyle],
    id: Option<u32>,
) {
    let ShapeKind::Text {
        spans, align, para, ..
    } = kind
    else {
        return;
    };
    let (from, to) = (from.min(to), from.max(to));
    if from == to {
        return;
    }
    let chars = id.map(|c| resolve(styles, c).1);
    // The plain look of each paragraph, for taking a link off.
    let ranges = paragraph_ranges(spans);
    let paras: Vec<(usize, usize, Option<CharFormat>)> = ranges
        .iter()
        .enumerate()
        .map(|(i, &(s, e))| {
            let f = paragraph_format(spans, *align, para, i);
            let st = f.style.map(|p| resolve(styles, p).1);
            (s, e + 1, plain_look(spans, s, e, (from, to), st))
        })
        .collect();
    match chars {
        Some(c) => spans_apply(spans, from, to, |sp| {
            sp.char_style = id;
            c.apply(sp);
        }),
        None => {
            for (s, e, look) in paras {
                let (a, b) = (from.max(s), to.min(e));
                let Some(look) = look else { continue };
                if a < b {
                    spans_apply(spans, a, b, |sp| {
                        // What the style set goes back to the paragraph's look.
                        if let Some(c) = sp.char_style.take() {
                            look.only(&resolve(styles, c).1).apply(sp);
                        }
                    });
                }
            }
        }
    }
    *spans = crate::document::merge_equal_spans(std::mem::take(spans));
}

/// The paragraph style and the character style at character `at` of
/// text `kind`.
pub fn styles_at(kind: &ShapeKind, at: usize) -> (Option<u32>, Option<u32>) {
    let ShapeKind::Text {
        spans, align, para, ..
    } = kind
    else {
        return (None, None);
    };
    let i = paragraph_of(spans, at);
    let p = paragraph_format(spans, *align, para, i).style;
    let mut pos = 0;
    let mut c = None;
    for sp in spans {
        let n = sp.text.chars().count();
        if at < pos + n || n == 0 && at == pos {
            c = sp.char_style;
            break;
        }
        pos += n;
        c = sp.char_style;
    }
    (p, c)
}

/// A paragraph style made from the paragraph at character `at`: its
/// format and its first character's properties.
pub fn paragraph_style_from(kind: &ShapeKind, at: usize, name: String) -> Option<TextStyle> {
    let ShapeKind::Text {
        spans, align, para, ..
    } = kind
    else {
        return None;
    };
    let i = paragraph_of(spans, at);
    let mut f = paragraph_format(spans, *align, para, i);
    f.style = None;
    let start = paragraph_ranges(spans).get(i).map_or(0, |r| r.0);
    let first = span_at(spans, start)?;
    Some(TextStyle {
        id: 0,
        name,
        kind: TextStyleKind::Paragraph,
        based_on: None,
        para: Some(f),
        chars: CharFormat::of(first),
    })
}

/// A character style made from the character at `at`: what sets it
/// apart from its paragraph's style (everything, when the paragraph has
/// none).
pub fn char_style_from(
    kind: &ShapeKind,
    at: usize,
    styles: &[TextStyle],
    name: String,
) -> Option<TextStyle> {
    let ShapeKind::Text {
        spans, align, para, ..
    } = kind
    else {
        return None;
    };
    let sp = span_at(spans, at)?;
    let i = paragraph_of(spans, at);
    let mine = CharFormat::of(sp);
    let (s, e) = paragraph_ranges(spans).get(i).copied().unwrap_or((0, 0));
    let st = paragraph_format(spans, *align, para, i)
        .style
        .map(|p| resolve(styles, p).1);
    let chars = match plain_look(spans, s, e, (at, at + 1), st) {
        Some(base) => {
            let d = mine.differing_from(&base);
            if d.is_empty() {
                mine
            } else {
                d
            }
        }
        None => mine,
    };
    Some(TextStyle {
        id: 0,
        name,
        kind: TextStyleKind::Character,
        based_on: None,
        para: None,
        chars,
    })
}

/// The span holding character `at` (the last one past the end).
fn span_at(spans: &[TextSpan], at: usize) -> Option<&TextSpan> {
    let mut pos = 0;
    for sp in spans {
        let n = sp.text.chars().count();
        if at < pos + n {
            return Some(sp);
        }
        pos += n;
    }
    spans.last()
}

/// Apply the styles again to text `kind` wherever it is linked to them
/// (after the styles changed); links to styles that are gone are dropped.
pub fn refresh_text(kind: &mut ShapeKind, styles: &[TextStyle]) {
    let ShapeKind::Text {
        spans, align, para, ..
    } = kind
    else {
        return;
    };
    let ranges = paragraph_ranges(spans);
    let links: Vec<(usize, usize, Option<u32>)> = ranges
        .iter()
        .enumerate()
        .map(|(i, &(s, e))| (s, e + 1, paragraph_format(spans, *align, para, i).style))
        .collect();
    let gone_chars = spans
        .iter()
        .any(|sp| sp.char_style.is_some_and(|c| find(styles, c).is_none()));
    if gone_chars {
        for sp in spans.iter_mut() {
            if sp.char_style.is_some_and(|c| find(styles, c).is_none()) {
                sp.char_style = None;
            }
        }
    }
    let any_char = spans.iter().any(|sp| sp.char_style.is_some());
    for (s, e, link) in links.into_iter().rev() {
        match link {
            Some(id) if find(styles, id).is_some() => apply_paragraph_style(
                kind,
                Some((s, e.saturating_sub(1).max(s))),
                styles,
                Some(id),
            ),
            Some(_) => {
                apply_paragraph_style(kind, Some((s, e.saturating_sub(1).max(s))), styles, None)
            }
            None if any_char => {
                // Character styles outside a paragraph style.
                if let ShapeKind::Text { spans, .. } = kind {
                    spans_apply(spans, s, e, |sp| {
                        if let Some(c) = sp.char_style {
                            resolve(styles, c).1.apply(sp);
                        }
                    });
                }
            }
            None => {}
        }
    }
    if let ShapeKind::Text { spans, .. } = kind {
        *spans = crate::document::merge_equal_spans(std::mem::take(spans));
    }
}

/// [`refresh_text`] on `shape` and everything a group or clip frame
/// holds.
pub fn refresh_shape(shape: &mut Shape, styles: &[TextStyle]) {
    match &mut shape.kind {
        ShapeKind::Group { children } => {
            for c in children {
                refresh_shape(c, styles);
            }
        }
        ShapeKind::ClipFrame { frame, contents } => {
            refresh_shape(frame, styles);
            for c in contents {
                refresh_shape(c, styles);
            }
        }
        k @ ShapeKind::Text { .. } => refresh_text(k, styles),
        _ => {}
    }
}

/// How many paragraphs (paragraph styles) or runs of characters
/// (character styles) are linked to style `id` in `shape`.
pub fn count_uses(shape: &Shape, id: u32) -> usize {
    match &shape.kind {
        ShapeKind::Group { children } => children.iter().map(|c| count_uses(c, id)).sum(),
        ShapeKind::ClipFrame { frame, contents } => {
            count_uses(frame, id) + contents.iter().map(|c| count_uses(c, id)).sum::<usize>()
        }
        ShapeKind::Text {
            spans, align, para, ..
        } => {
            let paras = (0..paragraph_ranges(spans).len())
                .filter(|&i| paragraph_format(spans, *align, para, i).style == Some(id))
                .count();
            let runs = spans
                .iter()
                .filter(|sp| sp.char_style == Some(id) && !sp.text.is_empty())
                .count();
            paras + runs
        }
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::Command;
    use crate::document::{Document, ParagraphStyle, TextAlign};
    use crate::geometry::{Point, Size};

    fn text(s: &str) -> ShapeKind {
        ShapeKind::Text {
            spans: vec![TextSpan::new(s, "Sans", 11.0)],
            origin: Point::ZERO,
            frame: Some(Size::new(100.0, 100.0)),
            align: TextAlign::Left,
            para: ParagraphStyle::default(),
            on_path: None,
        }
    }

    fn spans(k: &ShapeKind) -> &[TextSpan] {
        match k {
            ShapeKind::Text { spans, .. } => spans,
            _ => unreachable!(),
        }
    }

    fn corpo() -> TextStyle {
        TextStyle {
            id: 1,
            name: "Texto corrido".into(),
            kind: TextStyleKind::Paragraph,
            based_on: None,
            para: Some(ParagraphFormat {
                align: TextAlign::Justify,
                first_line_indent: 5.0,
                ..Default::default()
            }),
            chars: CharFormat {
                font_family: Some("Serif".into()),
                size_pt: Some(11.5),
                ..Default::default()
            },
        }
    }

    fn titulo() -> TextStyle {
        TextStyle {
            id: 2,
            name: "Título".into(),
            kind: TextStyleKind::Paragraph,
            based_on: Some(1),
            para: Some(ParagraphFormat {
                align: TextAlign::Center,
                space_after: 6.0,
                ..Default::default()
            }),
            chars: CharFormat {
                size_pt: Some(18.0),
                bold: Some(true),
                ..Default::default()
            },
        }
    }

    fn enfase() -> TextStyle {
        TextStyle {
            id: 3,
            name: "Ênfase".into(),
            kind: TextStyleKind::Character,
            chars: CharFormat {
                italic: Some(true),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn a_style_based_on_another_takes_what_it_does_not_set() {
        let styles = [corpo(), titulo()];
        let (p, c) = resolve(&styles, 2);
        assert_eq!(p.unwrap().align, TextAlign::Center);
        assert_eq!(c.font_family.as_deref(), Some("Serif"));
        assert_eq!(c.size_pt, Some(18.0));
        assert_eq!(c.bold, Some(true));
        assert!(would_loop(&styles, 1, 2));
        assert!(!would_loop(&styles, 2, 1));
        assert_eq!(children_of(&styles, 1), vec![2]);
    }

    #[test]
    fn paragraph_and_character_styles_apply_and_stay_linked() {
        let styles = vec![corpo(), titulo(), enfase()];
        let mut k = text("Capítulo um\nEra uma vez");
        apply_paragraph_style(&mut k, Some((0, 0)), &styles, Some(2));
        apply_paragraph_style(&mut k, Some((13, 13)), &styles, Some(1));
        // "uma" in italics with the character style.
        apply_char_style(&mut k, 16, 19, &styles, Some(3));
        assert_eq!(styles_at(&k, 2), (Some(2), None));
        assert_eq!(styles_at(&k, 17), (Some(1), Some(3)));
        let s = spans(&k);
        let title = s.iter().find(|sp| sp.text.starts_with("Capítulo")).unwrap();
        assert!(title.bold && title.size_pt == 18.0 && title.font_family == "Serif");
        let uma = s.iter().find(|sp| sp.text == "uma").unwrap();
        assert!(uma.italic && uma.size_pt == 11.5);
        // Taking the character style off gives back the paragraph's.
        apply_char_style(&mut k, 16, 19, &styles, None);
        assert!(spans(&k)
            .iter()
            .all(|sp| !sp.italic && sp.char_style.is_none()));
    }

    #[test]
    fn changing_a_style_changes_every_text_linked_to_it() {
        let mut doc = Document::new("t", Size::new(210.0, 297.0));
        let mut k = text("Um\nDois");
        let styles = vec![corpo(), titulo(), enfase()];
        apply_paragraph_style(&mut k, None, &styles, Some(1));
        apply_char_style(&mut k, 0, 2, &styles, Some(3));
        let layer = doc.pages[0].layers[0].id;
        let id = doc.ids_mut().shape();
        let shape = Shape::new(id, k);
        Command::AddShape { layer, shape }.apply(&mut doc).unwrap();
        Command::SetTextStyles {
            styles: styles.clone(),
        }
        .apply(&mut doc)
        .unwrap();
        // The body grows to 12 pt and the parent's font changes the title too.
        let mut changed = styles.clone();
        changed[0].chars.size_pt = Some(12.0);
        Command::SetTextStyles { styles: changed }
            .apply(&mut doc)
            .unwrap();
        let s = &doc.find_shape(id).unwrap().kind;
        assert!(
            spans(s).iter().all(|sp| sp.size_pt == 12.0),
            "{:?}",
            spans(s)
        );
        assert!(
            spans(s).iter().any(|sp| sp.italic),
            "the character style stays on top"
        );
        // A style that is gone: the text keeps its look, the link goes.
        Command::SetTextStyles {
            styles: vec![titulo()],
        }
        .apply(&mut doc)
        .unwrap();
        let s = &doc.find_shape(id).unwrap().kind;
        assert_eq!(styles_at(s, 0), (None, None));
        assert!(spans(s).iter().all(|sp| sp.size_pt == 12.0));
    }

    #[test]
    fn styles_are_made_from_the_text() {
        let styles = vec![corpo()];
        let mut k = text("Um texto");
        apply_paragraph_style(&mut k, None, &styles, Some(1));
        let p = paragraph_style_from(&k, 0, "Novo".into()).unwrap();
        assert_eq!(p.para.as_ref().unwrap().align, TextAlign::Justify);
        assert_eq!(p.para.as_ref().unwrap().style, None);
        assert_eq!(p.chars.size_pt, Some(11.5));
        // Bold inside the paragraph: the character style keeps only that.
        if let ShapeKind::Text { spans, .. } = &mut k {
            spans_apply(spans, 3, 8, |sp| sp.bold = true);
        }
        let c = char_style_from(&k, 4, &styles, "Negrito".into()).unwrap();
        assert_eq!(
            c.chars,
            CharFormat {
                bold: Some(true),
                ..Default::default()
            }
        );
    }

    #[test]
    fn a_drawing_with_styles_saves_and_opens() {
        let mut doc = Document::new("t", Size::new(210.0, 297.0));
        Command::SetTextStyles {
            styles: vec![corpo(), titulo(), enfase()],
        }
        .apply(&mut doc)
        .unwrap();
        let json = serde_json::to_string(&doc).unwrap();
        let back: Document = serde_json::from_str(&json).unwrap();
        assert_eq!(back.text_styles, doc.text_styles);
        // Without styles nothing is written.
        let plain = serde_json::to_string(&Document::new("t", Size::new(210.0, 297.0))).unwrap();
        assert!(!plain.contains("text_styles"));
    }
}
