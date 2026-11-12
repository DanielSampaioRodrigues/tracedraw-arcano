//! Arcano edition: the MCP tools for book and page work (see
//! docs/behavior/mcp.md, "Book and page tools"): reading a drawing's
//! pages, stories and lines, checking its typography the way a press
//! checks a book (justification, breaks, rag, colour of the page,
//! micro-typography, structure, local formatting, openings), and the
//! tools that compose and fix it (in `mcp_prelo_fix.rs`).
//!
//! A story is a chain of linked paragraph frames, named by the id of its
//! first frame; without `story`, a tool works on the main one (the story
//! with the most characters). Pages are given by index (0 = the first);
//! answers also give each page's number as the drawing numbers it.
//! Lengths are millimetres, the page's lower-left corner at (0, 0), y up.

use crate::app::App;
use crate::mcp_plus::Reply;
use serde_json::{json, Value};
use tracedraw_core::paragraphs::{paragraph_formats_of, paragraph_ranges};
use tracedraw_core::{Shape, ShapeId, ShapeKind, TextAlign};

pub(crate) fn tool(name: &str, description: &str, props: Value, required: &[&str]) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": {"type": "object", "properties": props, "required": required},
    })
}

fn int() -> Value {
    json!({"type": "integer"})
}
fn num() -> Value {
    json!({"type": "number"})
}
fn text() -> Value {
    json!({"type": "string"})
}
fn boolean() -> Value {
    json!({"type": "boolean"})
}

/// The common arguments of the checks: a story and a page range.
fn range_props(mut extra: Value) -> Value {
    let base = json!({
        "story": {"type": "integer", "description": "Id of any frame of the story (from story_map). Default: the main story."},
        "from_page": {"type": "integer", "description": "First page index (0 = the first). Default: 0."},
        "to_page": {"type": "integer", "description": "Last page index. Default: the last page."},
        "max_per_list": {"type": "integer", "description": "Cases shown in each list. Default: 15."},
    });
    if let (Some(e), Some(b)) = (extra.as_object_mut(), base.as_object()) {
        for (k, v) in b {
            e.insert(k.clone(), v.clone());
        }
    }
    extra
}

/// The book and page tools.
pub fn tools() -> Vec<Value> {
    let mut v = vec![
        tool(
            "document_report",
            "Start here on a drawing: format, facing pages, bleed, the house margins, page count, master pages (with their primary text frames), stories, styles, fonts and images, and the problems to fix before printing: text that overflows, missing or restricted fonts, low-resolution images, objects off the page, empty pages. Read only.",
            json!({}),
            &[],
        ),
        tool(
            "page_layout",
            "Geometry of each page: size, side (left or right), master page, the margins and the content area (the primary frame's when the master has one, else the house margins, already mirrored for the side), and each object with id, type, name, layer, bounds, for text frames the story, the next and previous frame and whether it overflows, for images their effective resolution. In parts: from_page and count (10 by default, at most 50). Use it before placing objects or scripting by id. Read only.",
            json!({"from_page": int(), "count": int(), "objects": {"type": "boolean", "description": "false: only the page geometry. Default: true."}}),
            &[],
        ),
        tool(
            "story_map",
            "The stories (chains of linked text frames) with their frames, pages, characters and overflow, the main story apart from short texts (folios, running heads, captions), the paragraph styles used in the main story with their counts, and the styles that look like chapter openings (paragraphs that start at the top of a page), to pass as opening_styles to the checks. Use it at the start on a drawing someone else composed. Read only.",
            json!({"story": int()}),
            &[],
        ),
        tool(
            "export_lines",
            "Every line of a story as laid out: page, frame, paragraph, line, baseline and left and right ends (mm), the text, the paragraph's style and alignment, whether it is the paragraph's first or last line, whether it ends in a hyphen, and for justified lines the width of its spaces against a normal space. The base of the typography checks. save_to writes the whole list as JSON (else at most 400 lines come back). Read only.",
            range_props(json!({"save_to": text()})),
            &[],
        ),
        tool(
            "audit_justification",
            "Checks the justified text: loose and tight lines (spaces much wider or narrower than a normal space, measured in the drawing's own fonts), rivers (gaps lined up over three or more lines), and hyphenation (more than two hyphens in a row, a hyphen at the foot of a frame or column, a hyphen on a paragraph's last line, a one-letter syllable, a proper name split). A list per page, worst first. Only diagnoses: fix_justification fixes.",
            range_props(json!({})),
            &[],
        ),
        tool(
            "audit_breaks",
            "Checks the breaks and rhythm of the pages: widows (a paragraph's last line alone at the top of a frame or column), orphans (its first line alone at the foot), a single word on a paragraph's last line, a title (kept with the next) left at the foot, stacks (the same word starting or ending three or more lines in a row) and, with facing pages, spreads whose two pages end at different heights. opening_styles: paragraph styles that open chapters (the page before them ends short on purpose). Only diagnoses: fix_breaks fixes.",
            range_props(json!({"opening_styles": {"type": "array", "items": {"type": "string"}}})),
            &[],
        ),
        tool(
            "audit_rag",
            "Checks the shape of the lines of text that is not justified (titles, epigraphs, poetry, captions, flush-left text): wedge, holes, short and long lines alternating, a last line too short, and breaks against the sense (an article, preposition or conjunction at the end of a line, a number apart from its unit, a proper name split). With poetry: true it only points, the line breaks of verse being the author's. Only diagnoses.",
            range_props(json!({"poetry": boolean(), "include_justified": {"type": "boolean", "description": "Also the last lines of justified paragraphs."}})),
            &[],
        ),
        tool(
            "audit_density",
            "Measures the grey of each full line (characters per millimetre against the median of lines in the same style and size) and of each page, to find lines and pages lighter or darker than the rest (a light line is often loose spacing or open tracking) and patches of three or more such lines in a row. line_threshold in percent (10), page_threshold (2.5). Only diagnoses.",
            range_props(json!({"line_threshold": num(), "page_threshold": num()})),
            &[],
        ),
        tool(
            "audit_microtypography",
            "Checks the fine details of the text, page by page: straight quotes and apostrophes, a hyphen used as a dash, dashes spaced differently across the drawing, double spaces, a space before punctuation, punctuation stuck to the next word; and suggests an ellipsis for three dots, small caps for long all-capital acronyms and a thousands separator for long numbers. all_texts: every text, not only the story. Only diagnoses: fix_typography fixes quotes, dashes, ellipses and spaces.",
            range_props(json!({"all_texts": boolean()})),
            &[],
        ),
        tool(
            "audit_structure",
            "Checks the drawing's hygiene page by page: objects off the page (on the pasteboard) or crossing its edge beyond the bleed, text frames outside the margins (tolerance_mm, 0.35 by default), text that overflows, empty text frames, frames much taller than their text, and running text in separate frames that perhaps should be threaded. Only diagnoses.",
            json!({"from_page": int(), "to_page": int(), "tolerance_mm": num(), "max_per_list": int()}),
            &[],
        ),
        tool(
            "audit_local_formatting",
            "Lists the text that differs from its paragraph style (local overrides), page by page, in three groups: off the style (font, size, colour, outline), the author's marks set by hand (italic, bold, underline, strike-through, superscript, subscript, OpenType features such as small caps), and composition settings (tracking, character offsets and angles). Shows the local value and the style's. Only diagnoses: clear_local_formatting clears.",
            range_props(json!({})),
            &[],
        ),
        tool(
            "check_openings",
            "Checks the front matter against the house order (half title, a blank page, title page, credits) and the chapter openings (on a right-hand page, with a drop cap in the first paragraph) and that the page count is even. Read only.",
            json!({}),
            &[],
        ),
        tool(
            "check_fidelity",
            "Compares the author's original .docx with the story: how many characters are italic, bold, underlined, struck through, superscript and subscript in each, and how many paragraphs, and points at the marks that went missing on the way in (a silent loss: nothing fails and the book looks ready). Use it after importing the text and before delivering. Read only.",
            json!({"docx": {"type": "string", "description": "Path of the author's .docx on this computer."}, "story": int()}),
            &["docx"],
        ),
        tool(
            "font_available",
            "Whether each font family asked for is installed (and which faces it has), by name, without listing every font. Use it before proposing or applying a font. At most 50 names. Read only.",
            json!({"names": {"type": "array", "items": {"type": "string"}}}),
            &["names"],
        ),
        tool(
            "house_standard",
            "The house standard of Editora Arcano 3 as this computer has it: margins (top, bottom, inner, outer), bleed and safety distance, the text settings new text gets, and the house's book styles. Read it before composing or deciding any of these. Read only.",
            json!({}),
            &[],
        ),
        tool(
            "craft_reference",
            "The craft base of typography and page layout (justification, hyphenation, breaks, rag, the grey of the page, chapter openings, front matter and credits, styles, micro-typography, checking and closing a book, vector drawing), with where each principle comes from. Use it before deciding a matter of craft and to explain to Dan why a fix is made. No arguments: the chapters; chapter: one whole; search: the sections holding all the words. Read only.",
            json!({"chapter": text(), "search": text()}),
            &[],
        ),
    ];
    v.extend(crate::mcp_prelo_fix::tools());
    v
}

/// The answer for `name`, or `None` for tools other modules answer.
pub fn call(app: &mut App, name: &str, args: &Value) -> Option<Result<Reply, String>> {
    let t = |r: Result<Value, String>| Some(r.map(|v| Reply::Text(v.to_string())));
    match name {
        "document_report" => t(Ok(document_report(app))),
        "page_layout" => t(page_layout(app, args)),
        "story_map" => t(story_map(app, args)),
        "export_lines" => t(export_lines(app, args)),
        "audit_justification" => t(audit_justification(app, args)),
        "audit_breaks" => t(audit_breaks(app, args)),
        "audit_rag" => t(audit_rag(app, args)),
        "audit_density" => t(audit_density(app, args)),
        "audit_microtypography" => t(audit_microtypography(app, args)),
        "audit_structure" => t(audit_structure(app, args)),
        "audit_local_formatting" => t(audit_local_formatting(app, args)),
        "check_openings" => t(Ok(check_openings(app))),
        "check_fidelity" => t(check_fidelity(app, args)),
        "font_available" => t(font_available(args)),
        "house_standard" => t(Ok(house_standard(app))),
        "craft_reference" => t(craft_reference(args)),
        _ => crate::mcp_prelo_fix::call(app, name, args),
    }
}

// ----- shared -------------------------------------------------------------------

pub(crate) fn round(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// The page's number as the drawing numbers it (its section's).
pub(crate) fn page_number(app: &App, index: usize) -> String {
    app.doc().field_text(
        &tracedraw_core::document::PageNumberField {
            start_at: 1,
            style: 0,
            prefix: String::new(),
            suffix: String::new(),
            show: 0,
        },
        index,
    )
}

/// A page's index and number for answers.
pub(crate) fn page_ref(app: &App, index: usize) -> Value {
    json!({"page_index": index, "page": page_number(app, index)})
}

pub(crate) fn usize_arg(args: &Value, key: &str) -> Option<usize> {
    args.get(key).and_then(|v| v.as_u64()).map(|v| v as usize)
}

pub(crate) fn f64_arg(args: &Value, key: &str) -> Option<f64> {
    args.get(key).and_then(|v| v.as_f64()).filter(|v| v.is_finite())
}

pub(crate) fn bool_arg(args: &Value, key: &str) -> Option<bool> {
    args.get(key).and_then(|v| v.as_bool())
}

pub(crate) fn str_arg<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key).and_then(|v| v.as_str())
}

/// The page range asked for, in range.
pub(crate) fn page_range(app: &App, args: &Value) -> (usize, usize) {
    let n = app.doc().pages.len().max(1);
    let a = usize_arg(args, "from_page").unwrap_or(0).min(n - 1);
    let b = usize_arg(args, "to_page").unwrap_or(n - 1).min(n - 1);
    (a.min(b), a.max(b))
}

fn max_per_list(args: &Value) -> usize {
    usize_arg(args, "max_per_list").unwrap_or(15).clamp(1, 500)
}

/// Every paragraph frame on the pages (master layers left out), with its
/// page index, in page order.
pub(crate) fn page_frames(app: &App) -> Vec<(usize, Shape)> {
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
                walk(s, pi, &mut out);
            }
        }
    }
    out
}

/// A story: its frames in order, each with its page index.
#[derive(Debug, Clone)]
pub(crate) struct Story {
    pub head: ShapeId,
    pub frames: Vec<(ShapeId, usize)>,
    pub chars: usize,
    pub overflow: bool,
}

fn chars_of(s: &Shape) -> usize {
    match &s.kind {
        ShapeKind::Text { spans, .. } => spans.iter().map(|sp| sp.text.chars().count()).sum(),
        _ => 0,
    }
}

/// The stories of the drawing, the longest first.
pub(crate) fn stories(app: &App) -> Vec<Story> {
    let frames = page_frames(app);
    let mut out: Vec<Story> = Vec::new();
    let mut seen: Vec<ShapeId> = Vec::new();
    for (_, s) in &frames {
        if seen.contains(&s.id) {
            continue;
        }
        let chain = app.chain_of(s.id);
        seen.extend(chain.iter().copied());
        let mut st = Story {
            head: chain.first().copied().unwrap_or(s.id),
            frames: Vec::new(),
            chars: 0,
            overflow: false,
        };
        for id in &chain {
            let Some(page) = app.page_index_of(*id) else {
                continue;
            };
            if let Some(sh) = app.doc().find_shape(*id) {
                st.chars += chars_of(sh);
            }
            st.frames.push((*id, page));
        }
        if let Some(&(last, _)) = st.frames.last() {
            st.overflow = app.text_layout_of(last).is_some_and(|(l, _)| l.overflow);
        }
        if !st.frames.is_empty() {
            out.push(st);
        }
    }
    out.sort_by_key(|s| std::cmp::Reverse(s.chars));
    out
}

/// The story asked for (`story`: any frame of it), else the main one.
pub(crate) fn pick_story(app: &App, args: &Value) -> Result<Story, String> {
    let all = stories(app);
    match usize_arg(args, "story") {
        Some(id) => all
            .into_iter()
            .find(|s| s.frames.iter().any(|(f, _)| f.0 == id as u64))
            .ok_or_else(|| format!("no story has a frame with id {id}")),
        None => all.into_iter().next().ok_or_else(|| "the drawing has no paragraph text frames".to_string()),
    }
}

/// The name of paragraph style `id`.
fn style_name(app: &App, id: Option<u32>) -> Option<String> {
    let id = id?;
    app.doc().text_styles.iter().find(|s| s.id == id).map(|s| s.name.clone())
}

/// One line of a story as laid out.
#[derive(Debug, Clone)]
pub(crate) struct Line {
    pub page: usize,
    pub frame: ShapeId,
    /// The paragraph in the story (0 = the first).
    pub para: usize,
    /// The paragraph's characters in its frame.
    pub para_range: (usize, usize),
    pub text: String,
    pub baseline: f64,
    pub x0: f64,
    /// Where the line's ink ends (trailing spaces left out).
    pub ink_x1: f64,
    /// The width of the frame's columns.
    pub measure: f64,
    pub align: TextAlign,
    pub style: Option<String>,
    pub size_pt: f64,
    pub keep_with_next: bool,
    pub first: bool,
    pub last: bool,
    pub hyphenated: bool,
    /// The gaps of its spaces, page x from and to.
    pub spaces: Vec<(f64, f64)>,
    /// Justified lines: the width of their spaces over a normal space.
    pub space_ratio: Option<f64>,
    /// The column it sits in, counting from 0 in its frame.
    pub column: usize,
}

impl Line {
    pub fn justified(&self) -> bool {
        matches!(self.align, TextAlign::Justify | TextAlign::ForceJustify)
    }
    pub fn width(&self) -> f64 {
        (self.ink_x1 - self.x0).max(0.0)
    }
    pub fn words(&self) -> Vec<&str> {
        self.text.split_whitespace().collect()
    }
}

/// The lines of `story` on the pages `from..=to`; `ratios`: the space
/// width of justified lines too (a second layout of each frame).
pub(crate) fn story_lines(app: &App, story: &Story, (from, to): (usize, usize), ratios: bool) -> Vec<Line> {
    let mut out = Vec::new();
    let mut base = 0usize;
    let mut continues = false;
    for &(id, page) in &story.frames {
        let Some(s) = app.doc().find_shape(id).cloned() else {
            continue;
        };
        let ShapeKind::Text {
            spans,
            align,
            para,
            frame: Some(fsize),
            ..
        } = &s.kind
        else {
            continue;
        };
        let text: Vec<char> = spans.iter().flat_map(|sp| sp.text.chars()).collect();
        let ranges = paragraph_ranges(spans);
        let formats = paragraph_formats_of(spans, *align, para);
        let ends_open = !text.last().is_some_and(|c| *c == '\n');
        let next_base = base + ranges.len() - usize::from(ends_open);
        if page < from || page > to {
            base = next_base;
            continues = ends_open;
            continue;
        }
        let Some((layout, aff)) = app.text_layout_of(id) else {
            base = next_base;
            continues = ends_open;
            continue;
        };
        let left = if ratios && formats.iter().any(|f| matches!(f.align, TextAlign::Justify | TextAlign::ForceJustify)) {
            crate::prelo_quebras::flush_left(&s)
        } else {
            None
        };
        let cols = para.columns.max(1) as f64;
        let measure = ((fsize.width - para.gutter * (cols - 1.0)) / cols).max(0.0);
        let fitted = layout.fitted_chars.max(1);
        let size_at = |c: usize| {
            let mut seen = 0usize;
            for sp in spans {
                let n = sp.text.chars().count();
                if c < seen + n {
                    return sp.size_pt;
                }
                seen += n;
            }
            spans.last().map(|sp| sp.size_pt).unwrap_or(0.0)
        };
        let mut column = 0usize;
        let mut prev_base: Option<f64> = None;
        for b in &layout.lines {
            if b.start_char >= fitted {
                break;
            }
            let Some(p) = ranges.iter().position(|&(ps, pe)| b.start_char >= ps && b.start_char <= pe) else {
                continue;
            };
            let (ps, pe) = ranges[p];
            let line_text: String = text.get(b.start_char..b.end_char).unwrap_or(&[]).iter().collect();
            if line_text.trim().is_empty() {
                continue;
            }
            if prev_base.is_some_and(|pb| b.baseline > pb + 0.01) {
                column += 1;
            }
            prev_base = Some(b.baseline);
            let f = formats.get(p);
            let mut ink_end = b.end_char;
            while ink_end > b.start_char && text.get(ink_end - 1).is_some_and(|c| c.is_whitespace()) {
                ink_end -= 1;
            }
            let at = |k: usize| -> f64 {
                let x = b.edges.get(k.saturating_sub(b.start_char)).copied().unwrap_or(0.0);
                (aff * tracedraw_core::Point::new(x, b.baseline)).x
            };
            let y = (aff * tracedraw_core::Point::new(b.edges.first().copied().unwrap_or(0.0), b.baseline)).y;
            let spaces: Vec<(f64, f64)> = (b.start_char..ink_end)
                .filter(|&i| text.get(i) == Some(&' '))
                .map(|i| (at(i), at(i + 1)))
                .collect();
            let first_para_here = p == 0 && continues;
            let last_para_here = p + 1 == ranges.len() && ends_open && story.frames.last().map(|x| x.0) != Some(id);
            let is_word = |c: Option<&char>| c.is_some_and(|c| c.is_alphanumeric());
            let hyphenated = b.end_char < pe && is_word(text.get(b.end_char.wrapping_sub(1))) && is_word(text.get(b.end_char));
            let mut line = Line {
                page,
                frame: id,
                para: base + p,
                para_range: (ps, pe),
                text: line_text,
                baseline: y,
                x0: at(b.start_char),
                ink_x1: at(ink_end),
                measure,
                align: f.map(|f| f.align).unwrap_or(*align),
                style: style_name(app, f.and_then(|f| f.style)),
                size_pt: size_at(b.start_char),
                keep_with_next: f.is_some_and(|f| f.keep_with_next),
                first: b.start_char <= ps && !first_para_here,
                last: b.end_char >= pe && !last_para_here,
                hyphenated,
                spaces,
                space_ratio: None,
                column,
            };
            let is_last_line = b.end_char >= pe;
            if line.justified() && !(is_last_line && line.align == TextAlign::Justify) && !line.spaces.is_empty() {
                if let Some(natural) = left
                    .as_ref()
                    .and_then(|l| l.lines.iter().find(|n| n.start_char == b.start_char && n.end_char == b.end_char))
                {
                    let actual = crate::prelo_quebras::space_width(&text, b, ink_end);
                    let plain = crate::prelo_quebras::space_width(&text, natural, ink_end);
                    line.space_ratio = crate::prelo_quebras::space_ratio(actual, plain, 0.0, 1);
                }
            }
            out.push(line);
        }
        base = next_base;
        continues = ends_open;
    }
    out
}

fn excerpt(t: &str) -> String {
    let t = t.trim();
    let mut s: String = t.chars().take(60).collect();
    if t.chars().count() > 60 {
        s.push('\u{2026}');
    }
    s
}

fn line_ref(app: &App, l: &Line) -> Value {
    json!({
        "page_index": l.page,
        "page": page_number(app, l.page),
        "frame": l.frame.0,
        "paragraph": l.para,
        "baseline_mm": round(l.baseline),
        "text": excerpt(&l.text),
    })
}

/// A list of cases, at most `max`, with the total.
fn listed(cases: Vec<Value>, max: usize) -> Value {
    let total = cases.len();
    json!({"count": total, "shown": cases.into_iter().take(max).collect::<Vec<_>>()})
}

// ----- reading ---------------------------------------------------------------

fn document_report(app: &App) -> Value {
    let doc = app.doc();
    let h = crate::prelo_casa::house_prefs(app);
    let first = doc.pages.first();
    let sizes: Vec<String> = {
        let mut v: Vec<String> = doc
            .pages
            .iter()
            .map(|p| format!("{} x {} mm", round(p.size.width), round(p.size.height)))
            .collect();
        v.dedup();
        v
    };
    let masters: Vec<Value> = doc
        .masters()
        .iter()
        .map(|m| {
            let pages = doc.pages.iter().filter(|p| doc.master_of(p.id) == m.id).count();
            json!({
                "id": m.id,
                "label": crate::masters::label(m),
                "based_on": m.based_on,
                "pages": pages,
                "primary_frame": m.primary.map(|p| json!({"margins_mm": {"top": p.margins[0], "bottom": p.margins[1], "inner": p.margins[2], "outer": p.margins[3]}, "columns": p.columns, "gutter_mm": p.gutter})),
            })
        })
        .collect();
    let all = stories(app);
    let issues = crate::preflight::run(doc, &app.missing_fonts, &app.settings.preflight);
    let problems: Vec<Value> = issues
        .iter()
        .take(200)
        .map(|i| {
            json!({
                "check": i.check.id(),
                "warning": i.check.severity() == crate::preflight::Severity::Warning,
                "page_index": i.page,
                "object": i.shape.map(|s| s.0),
                "detail": i.detail,
            })
        })
        .collect();
    let mut images = 0;
    let mut texts = 0;
    for p in &doc.pages {
        for l in &p.layers {
            for s in &l.shapes {
                count(s, &mut images, &mut texts);
            }
        }
    }
    fn count(s: &Shape, images: &mut usize, texts: &mut usize) {
        match &s.kind {
            ShapeKind::Bitmap { .. } => *images += 1,
            ShapeKind::Text { .. } => *texts += 1,
            ShapeKind::Group { children } => children.iter().for_each(|c| count(c, images, texts)),
            ShapeKind::ClipFrame { frame, contents } => {
                count(frame, images, texts);
                contents.iter().for_each(|c| count(c, images, texts));
            }
            _ => {}
        }
    }
    json!({
        "title": doc.title,
        "file": app.file.as_ref().map(|p| p.display().to_string()),
        "modified": app.engine.is_dirty(),
        "pages": doc.pages.len(),
        "page_sizes": sizes,
        "first_page_mm": first.map(|p| json!({"width": round(p.size.width), "height": round(p.size.height)})),
        "facing_pages": doc.metadata.facing_pages,
        "bleed_mm": doc.metadata.bleed,
        "house_margins_mm": {"top": h.top, "bottom": h.bottom, "inner": h.inner, "outer": h.outer},
        "master_pages": masters,
        "stories": all.len(),
        "main_story": all.first().map(|s| json!({"story": s.head.0, "frames": s.frames.len(), "characters": s.chars, "overflows": s.overflow})),
        "text_objects": texts,
        "images": images,
        "paragraph_styles": doc.text_styles.iter().filter(|s| s.para.is_some()).count(),
        "text_styles": doc.text_styles.len(),
        "layers": doc.pages.first().map(|p| p.layers.len()).unwrap_or(0),
        "missing_fonts": app.missing_fonts,
        "problems": problems,
        "problem_count": issues.len(),
        "coordinates": "millimetres, page lower-left corner (0, 0), y up",
    })
}

/// The content area of page `idx`: the primary frame's, else the house
/// margins, mirrored for the side (x0, y0, x1, y1).
pub(crate) fn content_area(app: &App, idx: usize) -> Option<(f64, f64, f64, f64)> {
    let doc = app.doc();
    let page = doc.pages.get(idx)?;
    let right_hand = !doc.metadata.facing_pages || idx % 2 == 0;
    let primary = app.primary_of_page(page.id).unwrap_or_else(|| {
        let h = crate::prelo_casa::house_prefs(app);
        tracedraw_core::PrimaryFrame {
            margins: [h.top, h.bottom, h.inner, h.outer],
            columns: 1,
            gutter: 5.0,
        }
    });
    let (o, s) = primary.rect_on(page.size, right_hand);
    Some((o.x, o.y, o.x + s.width, o.y + s.height))
}

fn page_layout(app: &App, args: &Value) -> Result<Value, String> {
    let doc = app.doc();
    let n = doc.pages.len();
    let from = usize_arg(args, "from_page").unwrap_or(0);
    if from >= n {
        return Err(format!("from_page {from} out of range (0 to {})", n.saturating_sub(1)));
    }
    let count = usize_arg(args, "count").unwrap_or(10).clamp(1, 50);
    let objects = bool_arg(args, "objects").unwrap_or(true);
    let to = (from + count).min(n);
    let facing = doc.metadata.facing_pages;
    let mut pages = Vec::new();
    for i in from..to {
        let p = &doc.pages[i];
        let m = doc.master_of(p.id);
        let master = doc.master_page(m).map(|mp| crate::masters::label(&mp));
        let area = content_area(app, i);
        let mut v = json!({
            "page_index": i,
            "page": page_number(app, i),
            "id": p.id.0,
            "name": p.name,
            "size_mm": {"width": round(p.size.width), "height": round(p.size.height)},
            "side": if !facing { "single" } else if i % 2 == 0 { "right" } else { "left" },
            "master": master,
            "content_area_mm": area.map(|(a, b, c, d)| json!({"x0": round(a), "y0": round(b), "x1": round(c), "y1": round(d)})),
        });
        if let Some(pf) = app.primary_of_page(p.id) {
            v["columns"] = json!(pf.columns);
            v["gutter_mm"] = json!(pf.gutter);
        }
        if objects {
            let mut list = Vec::new();
            for l in &p.layers {
                for s in &l.shapes {
                    list.push(object_json(app, s, &l.name));
                }
            }
            v["objects"] = json!(list);
        }
        pages.push(v);
    }
    Ok(json!({
        "pages": pages,
        "next_from_page": (to < n).then_some(to),
        "coordinates": "millimetres, page lower-left corner (0, 0), y up",
    }))
}

fn object_json(app: &App, s: &Shape, layer: &str) -> Value {
    let b = app.visual_bounds(s);
    let kind = match &s.kind {
        ShapeKind::Rect { .. } => "rectangle",
        ShapeKind::Ellipse { .. } => "ellipse",
        ShapeKind::Polygon { .. } => "polygon",
        ShapeKind::Path { .. } => "curve",
        ShapeKind::Text { frame: Some(_), .. } => "text_frame",
        ShapeKind::Text { .. } => "artistic_text",
        ShapeKind::Group { .. } => "group",
        ShapeKind::ClipFrame { .. } => "clip_frame",
        ShapeKind::Table(_) => "table",
        ShapeKind::SymbolInstance { .. } => "symbol",
        ShapeKind::Bitmap { .. } => "image",
    };
    let mut v = json!({
        "id": s.id.0,
        "type": kind,
        "name": s.name,
        "layer": layer,
        "bounds_mm": {"x0": round(b.x0), "y0": round(b.y0), "x1": round(b.x1), "y1": round(b.y1)},
    });
    match &s.kind {
        ShapeKind::Text { frame: Some(_), spans, .. } => {
            v["story"] = json!(app.chain_head(s.id).0);
            v["next_frame"] = json!(app.next_frame(s.id).map(|i| i.0));
            v["previous_frame"] = json!(app.prev_frame(s.id).map(|i| i.0));
            v["overflows"] = json!(app.text_overflows(s));
            v["characters"] = json!(spans.iter().map(|sp| sp.text.chars().count()).sum::<usize>());
            v["primary_frame"] = json!(s.data.iter().any(|(k, _)| k == crate::primary_frames::PRIMARY));
        }
        ShapeKind::Text { spans, .. } => {
            v["text"] = json!(excerpt(&spans.iter().map(|sp| sp.text.as_str()).collect::<String>()));
        }
        ShapeKind::Bitmap { width_px, height_px, .. } => {
            let (w, h) = (b.width().max(1e-9), b.height().max(1e-9));
            v["pixels"] = json!([width_px, height_px]);
            v["effective_ppi"] = json!([((*width_px as f64) / (w / 25.4)).round(), ((*height_px as f64) / (h / 25.4)).round()]);
        }
        ShapeKind::Group { children } => {
            v["children"] = json!(children.iter().map(|c| object_json(app, c, layer)).collect::<Vec<_>>());
        }
        _ => {}
    }
    v
}

fn story_map(app: &App, args: &Value) -> Result<Value, String> {
    let all = stories(app);
    if all.is_empty() {
        return Ok(json!({"stories": [], "note": "the drawing has no paragraph text frames"}));
    }
    let main = match usize_arg(args, "story") {
        Some(_) => pick_story(app, args)?,
        None => all[0].clone(),
    };
    let list: Vec<Value> = all
        .iter()
        .take(100)
        .map(|s| {
            let pages: Vec<usize> = {
                let mut v: Vec<usize> = s.frames.iter().map(|f| f.1).collect();
                v.dedup();
                v
            };
            json!({
                "story": s.head.0,
                "frames": s.frames.iter().map(|f| f.0 .0).collect::<Vec<_>>(),
                "pages": pages,
                "characters": s.chars,
                "overflows": s.overflow,
                "main": s.head == main.head,
                "kind": if s.head == main.head { "main" } else if s.chars < 120 { "short (folio, running head, caption)" } else { "other" },
            })
        })
        .collect();
    // The styles of the main story, with how many paragraphs and how
    // many of them start a page.
    let lines = story_lines(app, &main, (0, usize::MAX), false);
    let mut counts: Vec<(String, usize, usize)> = Vec::new();
    let mut last_para = usize::MAX;
    let mut top_of: std::collections::HashMap<usize, bool> = std::collections::HashMap::new();
    let mut prev_page = usize::MAX;
    for l in &lines {
        let first_on_page = l.page != prev_page;
        prev_page = l.page;
        if l.para != last_para {
            last_para = l.para;
            let name = l.style.clone().unwrap_or_else(|| "(no style)".into());
            let starts_page = first_on_page && l.first;
            top_of.insert(l.para, starts_page);
            match counts.iter_mut().find(|c| c.0 == name) {
                Some(c) => {
                    c.1 += 1;
                    c.2 += usize::from(starts_page);
                }
                None => counts.push((name, 1, usize::from(starts_page))),
            }
        }
    }
    counts.sort_by_key(|c| std::cmp::Reverse(c.1));
    let openings: Vec<&str> = counts
        .iter()
        .filter(|c| c.2 > 0 && c.2 * 2 >= c.1 && c.0 != "(no style)")
        .map(|c| c.0.as_str())
        .collect();
    Ok(json!({
        "stories": list,
        "main_story": main.head.0,
        "paragraph_styles": counts.iter().map(|c| json!({"style": c.0, "paragraphs": c.1, "start_a_page": c.2})).collect::<Vec<_>>(),
        "likely_opening_styles": openings,
    }))
}

fn export_lines(app: &App, args: &Value) -> Result<Value, String> {
    let story = pick_story(app, args)?;
    let range = page_range(app, args);
    let lines = story_lines(app, &story, range, true);
    let rows: Vec<Value> = lines
        .iter()
        .map(|l| {
            json!({
                "page_index": l.page,
                "page": page_number(app, l.page),
                "frame": l.frame.0,
                "column": l.column,
                "paragraph": l.para,
                "first": l.first,
                "last": l.last,
                "baseline_mm": round(l.baseline),
                "x0_mm": round(l.x0),
                "x1_mm": round(l.ink_x1),
                "measure_mm": round(l.measure),
                "style": l.style,
                "align": format!("{:?}", l.align).to_lowercase(),
                "size_pt": l.size_pt,
                "hyphen": l.hyphenated,
                "space_ratio": l.space_ratio.map(round),
                "text": l.text,
            })
        })
        .collect();
    let per_line: Vec<usize> = lines.iter().filter(|l| !l.last).map(|l| l.text.trim().chars().count()).collect();
    let avg = if per_line.is_empty() { 0.0 } else { per_line.iter().sum::<usize>() as f64 / per_line.len() as f64 };
    let mut run = 0;
    let mut worst_run = 0;
    for l in &lines {
        if l.hyphenated {
            run += 1;
            worst_run = worst_run.max(run);
        } else {
            run = 0;
        }
    }
    let mut v = json!({
        "story": story.head.0,
        "lines": rows.len(),
        "characters_per_line": round(avg),
        "hyphenated_lines": lines.iter().filter(|l| l.hyphenated).count(),
        "longest_hyphen_run": worst_run,
    });
    if let Some(path) = str_arg(args, "save_to").filter(|p| !p.trim().is_empty()) {
        let body = serde_json::to_string_pretty(&json!({"story": story.head.0, "lines": rows})).map_err(|e| e.to_string())?;
        std::fs::write(path, body).map_err(|e| format!("could not write {path}: {e}"))?;
        v["saved_to"] = json!(path);
    } else {
        v["shown"] = json!(rows.iter().take(400).cloned().collect::<Vec<_>>());
    }
    Ok(v)
}

// ----- checks ----------------------------------------------------------------

/// Spaces wider than this share of a normal space make a loose line.
pub(crate) const LOOSE: f64 = crate::prelo_quebras::LOOSE;
/// Narrower than this, a tight line.
pub(crate) const TIGHT: f64 = crate::prelo_quebras::TIGHT;

/// Rivers: space gaps that overlap on three or more lines in a row of the
/// same column, as (first line index, line count, x).
pub(crate) fn rivers(lines: &[Line]) -> Vec<(usize, usize, f64)> {
    let mut out = Vec::new();
    // Gaps alive from the line before: (x0, x1, first line, length).
    let mut alive: Vec<(f64, f64, usize, usize)> = Vec::new();
    for (i, l) in lines.iter().enumerate() {
        let same_run = i > 0 && {
            let p = &lines[i - 1];
            p.frame == l.frame && p.column == l.column && !p.last
        };
        let body = l.justified() && !l.last;
        let mut next: Vec<(f64, f64, usize, usize)> = Vec::new();
        if body {
            for &(a, b) in &l.spaces {
                let joined = if same_run {
                    alive.iter().find(|g| g.0.max(a) < g.1.min(b) - 0.05)
                } else {
                    None
                };
                match joined {
                    Some(g) => next.push((a.max(g.0), b.min(g.1).max(a.max(g.0)), g.2, g.3 + 1)),
                    None => next.push((a, b, i, 1)),
                }
            }
        }
        for g in &alive {
            let goes_on = next.iter().any(|n| n.2 == g.2 && n.3 == g.3 + 1);
            if !goes_on && g.3 >= 3 {
                out.push((g.2, g.3, (g.0 + g.1) / 2.0));
            }
        }
        alive = next;
    }
    for g in &alive {
        if g.3 >= 3 {
            out.push((g.2, g.3, (g.0 + g.1) / 2.0));
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)));
    out.dedup_by(|a, b| a.0 == b.0);
    out
}

/// Short words that read badly at the end of a line (Portuguese).
const WEAK_ENDS: &[&str] = &[
    "a", "o", "e", "à", "as", "os", "ao", "aos", "de", "da", "do", "das", "dos", "em", "na", "no", "nas", "nos", "um",
    "uma", "por", "para", "com", "sem", "que", "se", "ou", "nem", "mas", "sob", "pela", "pelo", "num", "numa", "the", "of",
    "and", "to", "in",
];

const UNITS: &[&str] = &["mm", "cm", "m", "km", "g", "kg", "mg", "l", "ml", "h", "min", "s", "%", "pt", "px", "dpi"];

fn is_hyphen_cases(lines: &[Line]) -> Vec<(&'static str, usize)> {
    let mut out = Vec::new();
    let mut run: Vec<usize> = Vec::new();
    let flush = |out: &mut Vec<(&'static str, usize)>, run: &mut Vec<usize>| {
        if run.len() > crate::prelo_quebras::MAX_HYPHENS {
            out.push(("hyphens_in_a_row", run[0]));
        }
        run.clear();
    };
    for (i, l) in lines.iter().enumerate() {
        if l.hyphenated {
            run.push(i);
            let next = lines.get(i + 1);
            let foot = next.is_none_or(|n| n.frame != l.frame || n.column != l.column);
            if foot {
                out.push(("hyphen_at_foot", i));
            }
            if next.is_some_and(|n| n.para == l.para && n.last) {
                // The paragraph's last word split.
                out.push(("last_word_split", i));
            }
            let words = l.words();
            let tail = words.last().copied().unwrap_or("");
            if tail.chars().filter(|c| c.is_alphabetic()).count() == 1 {
                out.push(("one_letter_syllable", i));
            }
            if let Some(n) = next {
                let head: String = n.text.trim_start().chars().take_while(|c| c.is_alphabetic()).collect();
                if head.chars().count() == 1 {
                    out.push(("one_letter_syllable", i));
                }
            }
            if tail.chars().next().is_some_and(|c| c.is_uppercase()) && words.len() > 1 {
                out.push(("proper_name_split", i));
            }
        } else {
            flush(&mut out, &mut run);
        }
    }
    flush(&mut out, &mut run);
    out
}

fn audit_justification(app: &App, args: &Value) -> Result<Value, String> {
    let story = pick_story(app, args)?;
    let lines = story_lines(app, &story, page_range(app, args), true);
    let max = max_per_list(args);
    let mut loose: Vec<(f64, &Line)> = Vec::new();
    let mut tight: Vec<(f64, &Line)> = Vec::new();
    for l in &lines {
        match l.space_ratio {
            Some(r) if r > LOOSE => loose.push((r, l)),
            Some(r) if r < TIGHT => tight.push((r, l)),
            _ => {}
        }
    }
    loose.sort_by(|a, b| b.0.total_cmp(&a.0));
    tight.sort_by(|a, b| a.0.total_cmp(&b.0));
    let spaced = |v: &[(f64, &Line)]| -> Vec<Value> {
        v.iter()
            .map(|(r, l)| {
                let mut j = line_ref(app, l);
                j["space_percent"] = json!((r * 100.0).round());
                j
            })
            .collect()
    };
    let rv: Vec<Value> = rivers(&lines)
        .iter()
        .map(|&(i, n, x)| {
            let mut j = line_ref(app, &lines[i]);
            j["lines"] = json!(n);
            j["x_mm"] = json!(round(x));
            j
        })
        .collect();
    let mut hy: std::collections::BTreeMap<&str, Vec<Value>> = Default::default();
    for (kind, i) in is_hyphen_cases(&lines) {
        hy.entry(kind).or_default().push(line_ref(app, &lines[i]));
    }
    let justified = lines.iter().filter(|l| l.space_ratio.is_some()).count();
    Ok(json!({
        "story": story.head.0,
        "justified_lines": justified,
        "normal_space": "measured on the same line set flush left, per font and size",
        "limits_percent": {"loose_over": (LOOSE * 100.0).round(), "tight_under": (TIGHT * 100.0).round()},
        "loose": listed(spaced(&loose), max),
        "tight": listed(spaced(&tight), max),
        "rivers": listed(rv, max),
        "hyphenation": hy.into_iter().map(|(k, v)| (k.to_string(), listed(v, max))).collect::<serde_json::Map<String, Value>>(),
    }))
}

/// Widows, orphans, runts and titles at the foot of the story's frames
/// on the pages asked for.
pub(crate) fn break_cases(lines: &[Line]) -> Vec<(&'static str, usize)> {
    let mut out = Vec::new();
    for (i, l) in lines.iter().enumerate() {
        let prev = i.checked_sub(1).and_then(|k| lines.get(k));
        let next = lines.get(i + 1);
        let top = prev.is_none_or(|p| p.frame != l.frame || p.column != l.column);
        let foot = next.is_none_or(|n| n.frame != l.frame || n.column != l.column);
        let has_more_after = next.is_some();
        let has_more_before = prev.is_some();
        if foot && has_more_after && l.first && !l.last && next.is_some_and(|n| n.para == l.para) {
            out.push(("orphan", i));
        }
        if top && has_more_before && l.last && !l.first && prev.is_some_and(|p| p.para == l.para) {
            out.push(("widow", i));
        }
        if l.last && !l.first && l.words().len() == 1 {
            out.push(("single_word_last_line", i));
        }
        if foot && has_more_after && l.last && l.keep_with_next {
            out.push(("title_at_foot", i));
        }
    }
    out
}

fn first_word(t: &str) -> String {
    t.split_whitespace().next().unwrap_or("").trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase()
}

fn last_word(t: &str) -> String {
    t.split_whitespace().last().unwrap_or("").trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase()
}

fn stacks(lines: &[Line]) -> Vec<(&'static str, usize, usize, String)> {
    let mut out = Vec::new();
    for (key, word) in [("stack_at_start", first_word as fn(&str) -> String), ("stack_at_end", last_word)] {
        let mut i = 0;
        while i < lines.len() {
            let w = word(&lines[i].text);
            let mut j = i + 1;
            while j < lines.len()
                && !w.is_empty()
                && lines[j].frame == lines[i].frame
                && lines[j].column == lines[i].column
                && word(&lines[j].text) == w
            {
                j += 1;
            }
            if j - i >= 3 {
                out.push((key, i, j - i, w.clone()));
            }
            i = j;
        }
    }
    out
}

fn audit_breaks(app: &App, args: &Value) -> Result<Value, String> {
    let story = pick_story(app, args)?;
    let lines = story_lines(app, &story, page_range(app, args), false);
    let max = max_per_list(args);
    let openings: Vec<String> = args
        .get("opening_styles")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|s| s.as_str().map(str::to_string)).collect())
        .unwrap_or_default();
    let mut groups: std::collections::BTreeMap<&str, Vec<Value>> = Default::default();
    for (kind, i) in break_cases(&lines) {
        groups.entry(kind).or_default().push(line_ref(app, &lines[i]));
    }
    for (kind, i, n, w) in stacks(&lines) {
        let mut j = line_ref(app, &lines[i]);
        j["lines"] = json!(n);
        j["word"] = json!(w);
        groups.entry(kind).or_default().push(j);
    }
    // Spreads ending at different heights.
    let mut uneven = Vec::new();
    if app.doc().metadata.facing_pages {
        let last_base = |page: usize| lines.iter().filter(|l| l.page == page).map(|l| l.baseline).reduce(f64::min);
        let opens = |page: usize| {
            lines
                .iter()
                .find(|l| l.page == page)
                .and_then(|l| l.style.clone())
                .is_some_and(|s| openings.contains(&s))
        };
        let pages: Vec<usize> = {
            let mut v: Vec<usize> = lines.iter().map(|l| l.page).collect();
            v.dedup();
            v
        };
        for &p in &pages {
            // A left-hand page (odd index) and the right-hand one after it.
            // The story's last page ends where the text ends.
            let story_ends = pages.last() == Some(&(p + 1)) && story.frames.last().is_some_and(|f| f.1 == p + 1);
            if p % 2 == 1 && pages.contains(&(p + 1)) && !story_ends && !opens(p + 1) && !opens(p + 2) {
                if let (Some(a), Some(b)) = (last_base(p), last_base(p + 1)) {
                    if (a - b).abs() > 0.5 {
                        uneven.push(json!({"left": page_ref(app, p), "right": page_ref(app, p + 1), "difference_mm": round((a - b).abs())}));
                    }
                }
            }
        }
    }
    let mut out: serde_json::Map<String, Value> = groups.into_iter().map(|(k, v)| (k.to_string(), listed(v, max))).collect();
    out.insert("uneven_spreads".into(), listed(uneven, max));
    Ok(json!({"story": story.head.0, "cases": out}))
}

fn audit_rag(app: &App, args: &Value) -> Result<Value, String> {
    let story = pick_story(app, args)?;
    let lines = story_lines(app, &story, page_range(app, args), false);
    let max = max_per_list(args);
    let poetry = bool_arg(args, "poetry").unwrap_or(false);
    let with_justified = bool_arg(args, "include_justified").unwrap_or(false);
    let mut paras: Vec<Vec<&Line>> = Vec::new();
    for l in &lines {
        match paras.last_mut() {
            Some(p) if p.last().is_some_and(|q| q.para == l.para) => p.push(l),
            _ => paras.push(vec![l]),
        }
    }
    let mut found = Vec::new();
    for p in &paras {
        let Some(first) = p.first() else { continue };
        let justified = first.justified();
        if justified && !with_justified {
            continue;
        }
        let mut problems: Vec<String> = Vec::new();
        let widths: Vec<f64> = p.iter().map(|l| l.width()).collect();
        let measure = first.measure.max(1e-6);
        if !justified && p.len() >= 3 {
            let body = &widths[..widths.len() - 1];
            let up = body.windows(2).all(|w| w[1] > w[0] + measure * 0.03);
            let down = body.windows(2).all(|w| w[1] < w[0] - measure * 0.03);
            if body.len() >= 3 && (up || down) {
                problems.push(if up { "wedge (lines growing)" } else { "wedge (lines shrinking)" }.into());
            }
            for k in 1..widths.len().saturating_sub(1) {
                let around = (widths[k - 1] + widths[k + 1]) / 2.0;
                if widths[k] < around * 0.7 {
                    problems.push(format!("hole at line {}", k + 1));
                }
            }
            if body.len() >= 4 {
                let alt = body.windows(3).all(|w| (w[1] - w[0]) * (w[2] - w[1]) < 0.0 && (w[1] - w[0]).abs() > measure * 0.12);
                if alt {
                    problems.push("short and long lines alternating".into());
                }
            }
        }
        if p.len() >= 2 {
            let longest = widths.iter().copied().fold(0.0, f64::max);
            if let Some(&last) = widths.last() {
                if last < longest * 0.2 {
                    problems.push("last line too short".into());
                }
            }
        }
        if !justified {
            for (k, l) in p.iter().enumerate() {
                if k + 1 == p.len() {
                    break;
                }
                let w = last_word(&l.text);
                if WEAK_ENDS.contains(&w.as_str()) {
                    problems.push(format!("line {} ends in \"{}\"", k + 1, w));
                }
                let ends_number = l.text.trim_end().chars().last().is_some_and(|c| c.is_ascii_digit());
                let next_unit = first_word(&p[k + 1].text);
                if ends_number && UNITS.contains(&next_unit.as_str()) {
                    problems.push(format!("line {}: a number apart from its unit", k + 1));
                }
                if l.hyphenated && l.words().last().and_then(|w| w.chars().next()).is_some_and(|c| c.is_uppercase()) {
                    problems.push(format!("line {}: a proper name split", k + 1));
                }
            }
        }
        if !problems.is_empty() {
            let mut j = line_ref(app, first);
            j["lines"] = json!(p.len());
            j["widths_percent_of_measure"] = json!(widths.iter().map(|w| (w / measure * 100.0).round()).collect::<Vec<_>>());
            j["problems"] = json!(problems);
            found.push(j);
        }
    }
    Ok(json!({
        "story": story.head.0,
        "paragraphs": listed(found, max),
        "note": if poetry { "poetry: the line breaks of verse are the author's; these are only pointers" } else { "" },
    }))
}

fn median(v: &mut [f64]) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.total_cmp(b));
    v[v.len() / 2]
}

fn audit_density(app: &App, args: &Value) -> Result<Value, String> {
    let story = pick_story(app, args)?;
    let lines = story_lines(app, &story, page_range(app, args), false);
    let max = usize_arg(args, "max_per_list").unwrap_or(12).clamp(1, 500);
    let lt = f64_arg(args, "line_threshold").unwrap_or(10.0).clamp(2.0, 40.0) / 100.0;
    let pt = f64_arg(args, "page_threshold").unwrap_or(2.5).clamp(0.5, 20.0) / 100.0;
    // Full lines only: a paragraph's last line is short by nature.
    let full: Vec<&Line> = lines.iter().filter(|l| !l.last && l.width() > l.measure * 0.8).collect();
    let density = |l: &Line| l.text.chars().filter(|c| !c.is_whitespace()).count() as f64 / l.width().max(1e-6);
    let key = |l: &Line| format!("{}|{}", l.style.clone().unwrap_or_default(), l.size_pt);
    let mut medians: std::collections::HashMap<String, f64> = Default::default();
    let mut by: std::collections::HashMap<String, Vec<f64>> = Default::default();
    for l in &full {
        by.entry(key(l)).or_default().push(density(l));
    }
    for (k, mut v) in by {
        medians.insert(k, median(&mut v));
    }
    let dev = |l: &Line| medians.get(&key(l)).map(|m| density(l) / m.max(1e-9) - 1.0).unwrap_or(0.0);
    let mut light = Vec::new();
    let mut dark = Vec::new();
    let mut patches = Vec::new();
    let mut run: Vec<&Line> = Vec::new();
    for l in &full {
        let d = dev(l);
        let mut j = line_ref(app, l);
        j["deviation_percent"] = json!((d * 1000.0).round() / 10.0);
        if d < -lt {
            light.push((d, j));
        } else if d > lt {
            dark.push((d, j));
        }
        if d.abs() > lt && run.last().is_none_or(|p| p.frame == l.frame) {
            run.push(l);
        } else {
            if run.len() >= 3 {
                patches.push(json!({"from": line_ref(app, run[0]), "lines": run.len()}));
            }
            run.clear();
            if d.abs() > lt {
                run.push(l);
            }
        }
    }
    if run.len() >= 3 {
        patches.push(json!({"from": line_ref(app, run[0]), "lines": run.len()}));
    }
    light.sort_by(|a, b| a.0.total_cmp(&b.0));
    dark.sort_by(|a, b| b.0.total_cmp(&a.0));
    // Pages: the mean deviation of their lines.
    let mut pages: std::collections::BTreeMap<usize, Vec<f64>> = Default::default();
    for l in &full {
        pages.entry(l.page).or_default().push(dev(l));
    }
    let mut off_pages = Vec::new();
    for (p, v) in pages {
        let m = v.iter().sum::<f64>() / v.len().max(1) as f64;
        if m.abs() > pt {
            let mut j = page_ref(app, p);
            j["deviation_percent"] = json!((m * 1000.0).round() / 10.0);
            j["tone"] = json!(if m < 0.0 { "lighter" } else { "darker" });
            off_pages.push(j);
        }
    }
    Ok(json!({
        "story": story.head.0,
        "full_lines": full.len(),
        "light_lines": listed(light.into_iter().map(|x| x.1).collect(), max),
        "dark_lines": listed(dark.into_iter().map(|x| x.1).collect(), max),
        "patches": listed(patches, max),
        "pages_off_tone": listed(off_pages, max),
    }))
}

/// The micro-typography cases of a text: (kind, character index, excerpt).
pub(crate) fn micro_cases(t: &[char]) -> Vec<(&'static str, usize)> {
    let mut out = Vec::new();
    let n = t.len();
    let get = |i: usize| t.get(i).copied();
    let mut i = 0;
    while i < n {
        let c = t[i];
        let prev = i.checked_sub(1).and_then(get);
        let next = get(i + 1);
        match c {
            '"' => out.push(("straight_quote", i)),
            '\'' => out.push(("straight_apostrophe", i)),
            '-' if prev == Some(' ') && next == Some(' ') => out.push(("hyphen_as_dash", i)),
            '-' if next == Some('-') => {
                out.push(("hyphen_as_dash", i));
                i += 1;
            }
            ' ' if next == Some(' ') => {
                out.push(("double_space", i));
                while get(i + 1) == Some(' ') {
                    i += 1;
                }
            }
            ' ' if next.is_some_and(|x| ",.;:!?)".contains(x)) && prev.is_some_and(|p| p.is_alphanumeric()) => {
                // "..." after a space is a cut, not a stray space.
                if !(next == Some('.') && get(i + 2) == Some('.')) {
                    out.push(("space_before_punctuation", i));
                }
            }
            ',' | ';' | ':' | '!' | '?'
                if next.is_some_and(|x| x.is_alphabetic()) && prev.is_some_and(|p| p.is_alphabetic()) =>
            {
                out.push(("punctuation_stuck_to_next_word", i));
            }
            '.' if next == Some('.') && get(i + 2) == Some('.') => {
                out.push(("three_dots_for_ellipsis", i));
                while get(i + 1) == Some('.') {
                    i += 1;
                }
            }
            _ => {}
        }
        // Words: long all-capital acronyms and long numbers.
        if c.is_alphanumeric() && prev.is_none_or(|p| !p.is_alphanumeric()) {
            let mut j = i;
            while j < n && t[j].is_alphanumeric() {
                j += 1;
            }
            let w: String = t[i..j].iter().collect();
            let letters = w.chars().filter(|c| c.is_alphabetic()).count();
            if letters >= 4 && letters == w.chars().count() && w.chars().all(|c| c.is_uppercase()) {
                out.push(("acronym_in_capitals", i));
            }
            let glued = |k: usize| get(k).is_some_and(|x| x == '.' || x == ',') && get(k + 1).is_some_and(|x| x.is_ascii_digit());
            if w.chars().all(|c| c.is_ascii_digit()) && w.len() >= 5 && !glued(j) && !(i > 0 && glued(i - 1)) {
                out.push(("long_number_without_separator", i));
            }
        }
        i += 1;
    }
    out
}

fn audit_microtypography(app: &App, args: &Value) -> Result<Value, String> {
    let max = max_per_list(args);
    let (from, to) = page_range(app, args);
    let all_texts = bool_arg(args, "all_texts").unwrap_or(false);
    let mut sources: Vec<(usize, ShapeId, String)> = Vec::new();
    if all_texts {
        fn walk(s: &Shape, page: usize, out: &mut Vec<(usize, ShapeId, String)>) {
            match &s.kind {
                ShapeKind::Group { children } => children.iter().for_each(|c| walk(c, page, out)),
                ShapeKind::ClipFrame { frame, contents } => {
                    walk(frame, page, out);
                    contents.iter().for_each(|c| walk(c, page, out));
                }
                ShapeKind::Text { spans, .. } => out.push((page, s.id, spans.iter().map(|sp| sp.text.as_str()).collect())),
                _ => {}
            }
        }
        for (pi, p) in app.doc().pages.iter().enumerate() {
            if pi < from || pi > to {
                continue;
            }
            for l in &p.layers {
                for s in &l.shapes {
                    walk(s, pi, &mut sources);
                }
            }
        }
    } else {
        let story = pick_story(app, args)?;
        for &(id, page) in &story.frames {
            if page < from || page > to {
                continue;
            }
            if let Some(ShapeKind::Text { spans, .. }) = app.doc().find_shape(id).map(|s| &s.kind) {
                sources.push((page, id, spans.iter().map(|sp| sp.text.as_str()).collect()));
            }
        }
    }
    let mut groups: std::collections::BTreeMap<&str, Vec<Value>> = Default::default();
    let (mut spaced, mut closed) = (0usize, 0usize);
    for (page, id, t) in &sources {
        let chars: Vec<char> = t.chars().collect();
        for (k, i) in micro_cases(&chars) {
            let a = i.saturating_sub(20);
            let b = (i + 20).min(chars.len());
            groups.entry(k).or_default().push(json!({
                "page_index": page,
                "page": page_number(app, *page),
                "object": id.0,
                "at": i,
                "text": chars[a..b].iter().collect::<String>(),
            }));
        }
        for (i, c) in chars.iter().enumerate() {
            if *c == '\u{2014}' {
                let around = i > 0 && chars[i - 1] == ' ' || chars.get(i + 1) == Some(&' ');
                if around {
                    spaced += 1;
                } else {
                    closed += 1;
                }
            }
        }
    }
    let suggestions = ["three_dots_for_ellipsis", "acronym_in_capitals", "long_number_without_separator"];
    let mut problems = serde_json::Map::new();
    let mut ideas = serde_json::Map::new();
    for (k, v) in groups {
        if suggestions.contains(&k) {
            ideas.insert(k.into(), listed(v, max));
        } else {
            problems.insert(k.into(), listed(v, max));
        }
    }
    if spaced > 0 && closed > 0 {
        problems.insert(
            "dashes_spaced_differently".into(),
            json!({"spaced": spaced, "closed": closed, "note": "pick one way for the whole book"}),
        );
    }
    Ok(json!({"texts": sources.len(), "problems": problems, "suggestions": ideas}))
}

fn audit_structure(app: &App, args: &Value) -> Result<Value, String> {
    let (from, to) = page_range(app, args);
    let tol = f64_arg(args, "tolerance_mm").unwrap_or(0.35).clamp(0.0, 10.0);
    let max = max_per_list(args);
    let doc = app.doc();
    let bleed = doc.metadata.bleed.max(0.0);
    let mut g: std::collections::BTreeMap<&str, Vec<Value>> = Default::default();
    let mut loose_frames: Vec<(usize, ShapeId, usize)> = Vec::new();
    for i in from..=to {
        let Some(p) = doc.pages.get(i) else { break };
        let (w, h) = (p.size.width, p.size.height);
        let area = content_area(app, i);
        for l in &p.layers {
            if !l.printable {
                continue;
            }
            for s in &l.shapes {
                let b = app.visual_bounds(s);
                let mut j = page_ref(app, i);
                j["object"] = json!(s.id.0);
                j["bounds_mm"] = json!({"x0": round(b.x0), "y0": round(b.y0), "x1": round(b.x1), "y1": round(b.y1)});
                if b.x1 < 0.0 || b.y1 < 0.0 || b.x0 > w || b.y0 > h {
                    g.entry("off_page").or_default().push(j.clone());
                    continue;
                }
                if b.x0 < -bleed - tol || b.y0 < -bleed - tol || b.x1 > w + bleed + tol || b.y1 > h + bleed + tol {
                    g.entry("beyond_bleed").or_default().push(j.clone());
                }
                let ShapeKind::Text { frame: Some(fs), spans, .. } = &s.kind else {
                    continue;
                };
                if let Some((x0, y0, x1, y1)) = area {
                    if b.x0 < x0 - tol || b.y0 < y0 - tol || b.x1 > x1 + tol || b.y1 > y1 + tol {
                        g.entry("text_frame_outside_margins").or_default().push(j.clone());
                    }
                }
                let chars: usize = spans.iter().map(|sp| sp.text.trim().chars().count()).sum();
                let linked = app.next_frame(s.id).is_some() || app.prev_frame(s.id).is_some();
                if chars == 0 && !linked {
                    g.entry("empty_text_frame").or_default().push(j.clone());
                    continue;
                }
                if app.next_frame(s.id).is_none() && app.text_overflows(s) {
                    g.entry("text_overflows").or_default().push(j.clone());
                }
                if !linked && fs.height > 20.0 {
                    if let Some((lay, _)) = app.text_layout_of(s.id) {
                        let used = lay.lines.iter().map(|b| b.ascent - b.baseline + b.descent).fold(0.0, f64::max);
                        if used > 0.0 && used < fs.height * 0.5 {
                            let mut k = j.clone();
                            k["text_height_mm"] = json!(round(used));
                            k["frame_height_mm"] = json!(round(fs.height));
                            g.entry("frame_much_taller_than_text").or_default().push(k);
                        }
                    }
                }
                if !linked && chars >= 400 {
                    loose_frames.push((i, s.id, chars));
                }
            }
        }
    }
    // Running text in loose frames on pages one after the other.
    for w in loose_frames.windows(2) {
        if w[1].0 == w[0].0 + 1 {
            g.entry("loose_frames_that_could_be_threaded").or_default().push(json!({
                "frames": [w[0].1 .0, w[1].1 .0],
                "pages": [page_number(app, w[0].0), page_number(app, w[1].0)],
            }));
        }
    }
    Ok(json!({
        "pages": {"from": from, "to": to},
        "cases": g.into_iter().map(|(k, v)| (k.to_string(), listed(v, max))).collect::<serde_json::Map<String, Value>>(),
    }))
}

/// How a span property counts for local formatting: the author's mark,
/// a composition setting, or the look the style gives (anything else).
pub(crate) fn override_class(key: &str) -> &'static str {
    match key {
        "bold" | "italic" | "underline" | "strikethrough" | "baseline_shift_pt" | "features" => "mark",
        "tracking_pct" | "offset_x_pct" | "angle_deg" | "variations" => "composition",
        _ => "style",
    }
}

/// The properties where span `have` differs from `want` (the style's),
/// with both values.
pub(crate) fn span_differences(have: &tracedraw_core::TextSpan, want: &tracedraw_core::TextSpan) -> Vec<(String, Value, Value)> {
    let (Ok(Value::Object(a)), Ok(Value::Object(b))) = (serde_json::to_value(have), serde_json::to_value(want)) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let keys: std::collections::BTreeSet<&String> = a.keys().chain(b.keys()).collect();
    for k in keys {
        if matches!(k.as_str(), "text" | "paragraph" | "char_style") {
            continue;
        }
        let (x, y) = (a.get(k).cloned().unwrap_or(Value::Null), b.get(k).cloned().unwrap_or(Value::Null));
        let empty = |v: &Value| v.is_null() || v.as_array().is_some_and(|a| a.is_empty()) || v.as_f64() == Some(0.0) || v.as_bool() == Some(false);
        if x != y && !(empty(&x) && empty(&y)) {
            out.push((k.clone(), x, y));
        }
    }
    out
}

/// Each run of a text whose look differs from its paragraph style:
/// (character start, end, the span, the style's span).
pub(crate) fn overrides_of(
    kind: &ShapeKind,
    styles: &[tracedraw_core::text_styles::TextStyle],
) -> Vec<(usize, usize, tracedraw_core::TextSpan, tracedraw_core::TextSpan, Option<u32>)> {
    let ShapeKind::Text { spans, align, para, .. } = kind else {
        return Vec::new();
    };
    let ranges = paragraph_ranges(spans);
    let formats = paragraph_formats_of(spans, *align, para);
    let mut out = Vec::new();
    for (i, &(s, e)) in ranges.iter().enumerate() {
        let Some(id) = formats.get(i).and_then(|f| f.style) else {
            continue;
        };
        let chars = tracedraw_core::text_styles::resolve(styles, id).1;
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
                out.push((a, b, sp.clone(), want, Some(id)));
            }
        }
    }
    out
}

fn audit_local_formatting(app: &App, args: &Value) -> Result<Value, String> {
    let story = pick_story(app, args)?;
    let (from, to) = page_range(app, args);
    let max = max_per_list(args);
    let styles = app.doc().text_styles.clone();
    let mut g: std::collections::BTreeMap<&str, Vec<Value>> = Default::default();
    for &(id, page) in &story.frames {
        if page < from || page > to {
            continue;
        }
        let Some(s) = app.doc().find_shape(id) else { continue };
        for (a, b, have, want, style) in overrides_of(&s.kind, &styles) {
            let mut by: std::collections::BTreeMap<&str, Vec<Value>> = Default::default();
            for (k, x, y) in span_differences(&have, &want) {
                by.entry(override_class(&k)).or_default().push(json!({"property": k, "local": x, "style": y}));
            }
            for (class, props) in by {
                let group = match class {
                    "mark" => "author_marks_by_hand",
                    "composition" => "composition_settings",
                    _ => "off_the_style",
                };
                g.entry(group).or_default().push(json!({
                    "page_index": page,
                    "page": page_number(app, page),
                    "frame": id.0,
                    "characters": [a, b],
                    "text": excerpt(&have.text),
                    "style": style_name(app, style),
                    "differences": props,
                }));
            }
        }
    }
    Ok(json!({
        "story": story.head.0,
        "groups": g.into_iter().map(|(k, v)| (k.to_string(), listed(v, max))).collect::<serde_json::Map<String, Value>>(),
    }))
}

fn check_openings(app: &App) -> Value {
    let items = app.house_openings();
    json!({
        "items": items.iter().map(|i| json!({
            "check": i.key,
            "what": crate::i18n::tr(i.key),
            "page_index": i.page,
            "page": i.page.map(|p| page_number(app, p)),
            "found": i.detail,
            "ok": i.ok,
        })).collect::<Vec<_>>(),
        "problems": items.iter().filter(|i| !i.ok).count(),
    })
}

/// Characters carrying each mark in `spans`.
fn mark_counts(spans: &[tracedraw_core::TextSpan]) -> serde_json::Map<String, Value> {
    let mut m = serde_json::Map::new();
    let mut add = |k: &str, n: usize| {
        let v = m.get(k).and_then(|v| v.as_u64()).unwrap_or(0) + n as u64;
        m.insert(k.into(), json!(v));
    };
    for k in ["italic", "bold", "underline", "strikethrough", "superscript", "subscript", "small_caps"] {
        add(k, 0);
    }
    for sp in spans {
        let n = sp.text.chars().filter(|c| !c.is_whitespace()).count();
        if n == 0 {
            continue;
        }
        let feat = |f: &str| sp.features.iter().any(|x| x == f);
        if sp.italic {
            add("italic", n);
        }
        if sp.bold {
            add("bold", n);
        }
        if sp.underline.is_on() {
            add("underline", n);
        }
        if sp.strikethrough.is_on() {
            add("strikethrough", n);
        }
        if sp.baseline_shift_pt > 0.0 || feat("sups") || feat("sup") {
            add("superscript", n);
        }
        if sp.baseline_shift_pt < 0.0 || feat("subs") || feat("sinf") {
            add("subscript", n);
        }
        if feat("smcp") || feat("c2sc") {
            add("small_caps", n);
        }
    }
    m
}

fn check_fidelity(app: &App, args: &Value) -> Result<Value, String> {
    let path = str_arg(args, "docx").ok_or("missing argument: docx")?;
    let bytes = std::fs::read(path).map_err(|e| format!("could not read {path}: {e}"))?;
    let orig = tracedraw_io::text_import::parse_docx(&bytes)?;
    let story = pick_story(app, args)?;
    let mut spans = Vec::new();
    for (id, _) in &story.frames {
        if let Some(ShapeKind::Text { spans: s, .. }) = app.doc().find_shape(*id).map(|s| &s.kind) {
            spans.extend(s.iter().cloned());
        }
    }
    let a = mark_counts(&orig.spans);
    let b = mark_counts(&spans);
    let paras = |s: &[tracedraw_core::TextSpan]| {
        paragraph_ranges(s)
            .iter()
            .filter(|(x, y)| y > x)
            .count()
    };
    let mut lost = Vec::new();
    for (k, v) in &a {
        let o = v.as_u64().unwrap_or(0);
        let d = b.get(k).and_then(|v| v.as_u64()).unwrap_or(0);
        if o > 0 && d < o * 95 / 100 {
            lost.push(json!({"mark": k, "original": o, "in_the_drawing": d}));
        }
        if o == 0 && d > 0 {
            lost.push(json!({"mark": k, "original": 0, "in_the_drawing": d, "note": "marks the original does not have"}));
        }
    }
    Ok(json!({
        "story": story.head.0,
        "original": {"characters_with": a, "paragraphs": paras(&orig.spans)},
        "drawing": {"characters_with": b, "paragraphs": paras(&spans)},
        "missing_or_extra": lost,
        "warnings": orig.warnings,
    }))
}

fn font_available(args: &Value) -> Result<Value, String> {
    let names: Vec<String> = args
        .get("names")
        .and_then(|v| v.as_array())
        .ok_or("names must be a list of font family names")?
        .iter()
        .filter_map(|v| v.as_str().map(str::to_string))
        .take(50)
        .collect();
    let fonts = tracedraw_text::fonts();
    let families = fonts.families();
    let out: Vec<Value> = names
        .iter()
        .map(|n| {
            let family = n.split('\t').next().unwrap_or(n).trim();
            let found = families.iter().find(|f| f.eq_ignore_ascii_case(family));
            match found {
                Some(f) => {
                    let faces: Vec<String> = fonts.family_info(f).map(|i| i.faces.into_iter().map(|s| s.name).collect()).unwrap_or_default();
                    json!({"name": n, "installed": true, "family": f, "faces": faces})
                }
                None => {
                    let near: Vec<&String> = families
                        .iter()
                        .filter(|f| f.to_lowercase().contains(&family.to_lowercase()))
                        .take(8)
                        .collect();
                    json!({"name": n, "installed": false, "similar": near})
                }
            }
        })
        .collect();
    Ok(json!({"fonts": out}))
}

fn house_standard(app: &App) -> Value {
    let h = crate::prelo_casa::house_prefs(app);
    let book: Vec<Value> = app
        .doc()
        .text_styles
        .iter()
        .map(|s| json!({"name": s.name, "kind": format!("{:?}", s.kind).to_lowercase(), "font": s.chars.font_family, "size_pt": s.chars.size_pt}))
        .collect();
    json!({
        "margins_mm": {"top": h.top, "bottom": h.bottom, "inner": h.inner, "outer": h.outer},
        "bleed_mm": h.bleed,
        "safety_mm": h.safety,
        "new_text": {"font": app.text_font, "size_pt": app.text_size_pt, "hyphenation": app.text_hyphenation},
        "styles_in_this_drawing": book,
        "rules": [
            "never change the author's text: fix typography, not words",
            "balance the lines of every text of more than one line (titles, epigraphs)",
            "aligned (lining) figures, never old-style figures that dance",
            "no page numbers in planners and diaries",
            "chapters open on a right-hand page",
            "measure fonts with font_metrics before sizing; never estimate",
        ],
        "applies_with": "the house standard button of the House Press panel, or create_document",
    })
}

const CRAFT: &str = include_str!("../../../docs/craft.md");

fn craft_reference(args: &Value) -> Result<Value, String> {
    let chapters: Vec<(&str, &str)> = {
        let mut v = Vec::new();
        let mut rest = CRAFT;
        while let Some(i) = rest.find("\n## ") {
            rest = &rest[i + 1..];
            let end = rest[3..].find("\n## ").map(|e| e + 3).unwrap_or(rest.len());
            let body = &rest[..end];
            let title = body.lines().next().unwrap_or("").trim_start_matches("## ").trim();
            v.push((title, body));
            rest = &rest[end..];
        }
        v
    };
    if let Some(c) = str_arg(args, "chapter").filter(|c| !c.trim().is_empty()) {
        let c = c.to_lowercase();
        return chapters
            .iter()
            .find(|(t, _)| t.to_lowercase().contains(&c))
            .map(|(t, b)| json!({"chapter": t, "text": b}))
            .ok_or_else(|| format!("no chapter \"{c}\"; chapters: {}", chapters.iter().map(|c| c.0).collect::<Vec<_>>().join(", ")));
    }
    if let Some(q) = str_arg(args, "search").filter(|c| !c.trim().is_empty()) {
        let words: Vec<String> = q.to_lowercase().split_whitespace().map(str::to_string).collect();
        let mut hits = Vec::new();
        for (t, body) in &chapters {
            for sec in body.split("\n### ") {
                let low = sec.to_lowercase();
                if words.iter().all(|w| low.contains(w)) {
                    hits.push(json!({"chapter": t, "text": sec.trim()}));
                }
            }
        }
        return Ok(json!({"search": q, "sections": hits}));
    }
    Ok(json!({"chapters": chapters.iter().map(|c| c.0).collect::<Vec<_>>()}))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use tracedraw_core::{Command, TextSpan};

    pub(crate) fn reply(app: &mut App, name: &str, args: Value) -> Value {
        match call(app, name, &args).expect("a book tool") {
            Ok(Reply::Text(t)) => serde_json::from_str(&t).unwrap_or(Value::String(t)),
            Ok(Reply::Image { text, .. }) => Value::String(text),
            Err(e) => panic!("{name}: {e}"),
        }
    }

    /// A drawing with a frame of `text` on page 1.
    pub(crate) fn frame(app: &mut App, text: &str, w: f64, h: f64, align: TextAlign) -> ShapeId {
        let mut para = tracedraw_core::ParagraphStyle::default();
        para.hyphenate = false;
        app.new_shape(ShapeKind::Text {
            spans: vec![TextSpan::new(text, "DejaVu Sans", 10.0)],
            origin: tracedraw_core::Point::new(20.0, 40.0),
            frame: Some(tracedraw_core::Size::new(w, h)),
            align,
            para,
            on_path: None,
        })
        .unwrap()
    }

    #[test]
    fn every_book_tool_has_a_name_a_description_and_an_answer() {
        let mut app = App::headless();
        frame(&mut app, "Um texto curto.", 80.0, 30.0, TextAlign::Left);
        let names: Vec<String> = tools().iter().map(|t| t["name"].as_str().unwrap().to_string()).collect();
        let mut seen = std::collections::HashSet::new();
        for t in tools() {
            assert!(t["description"].as_str().unwrap().len() > 40, "{}", t["name"]);
            assert!(seen.insert(t["name"].as_str().unwrap().to_string()), "twice: {}", t["name"]);
        }
        for n in &names {
            assert!(call(&mut App::headless(), n, &json!({})).is_some(), "{n} unanswered");
        }
        // No name clashes with the other tool lists.
        for t in crate::mcp_plus::base_tools() {
            assert!(!names.contains(&t["name"].as_str().unwrap().to_string()));
        }
    }

    #[test]
    fn the_report_layout_and_map_read_the_drawing() {
        let mut app = App::headless();
        app.add_page();
        app.goto_page(0);
        let id = frame(&mut app, &"Texto corrido. ".repeat(40), 80.0, 60.0, TextAlign::Justify);
        let r = reply(&mut app, "document_report", json!({}));
        assert_eq!(r["pages"], 2);
        assert_eq!(r["main_story"]["story"], id.0);
        let l = reply(&mut app, "page_layout", json!({"from_page": 0, "count": 1}));
        let objs = l["pages"][0]["objects"].as_array().unwrap();
        assert!(objs.iter().any(|o| o["id"] == id.0 && o["type"] == "text_frame"));
        assert!(l["pages"][0]["content_area_mm"]["x0"].as_f64().unwrap() > 0.0);
        let m = reply(&mut app, "story_map", json!({}));
        assert_eq!(m["main_story"], id.0);
    }

    #[test]
    fn lines_come_with_their_place_and_spacing() {
        let mut app = App::headless();
        frame(&mut app, &"Palavras de tamanhos variados enchem cada linha da moldura. ".repeat(12), 70.0, 120.0, TextAlign::Justify);
        let r = reply(&mut app, "export_lines", json!({}));
        let shown = r["shown"].as_array().unwrap();
        assert!(shown.len() > 5);
        assert!(shown[0]["baseline_mm"].as_f64().unwrap() > shown[1]["baseline_mm"].as_f64().unwrap());
        // Justified body lines have their space ratio; the last does not.
        assert!(shown[0]["space_ratio"].as_f64().is_some());
        assert!(shown.last().unwrap()["space_ratio"].is_null());
        let j = reply(&mut app, "audit_justification", json!({}));
        assert!(j["justified_lines"].as_u64().unwrap() > 3);
    }

    #[test]
    fn the_checks_find_their_cases() {
        let chars: Vec<char> = "Ele disse \"oi\" - e foi  embora , sem ver...ABNT 123456".chars().collect();
        let kinds: Vec<&str> = micro_cases(&chars).into_iter().map(|c| c.0).collect();
        for k in [
            "straight_quote",
            "hyphen_as_dash",
            "double_space",
            "space_before_punctuation",
            "three_dots_for_ellipsis",
            "acronym_in_capitals",
            "long_number_without_separator",
        ] {
            assert!(kinds.contains(&k), "{k} in {kinds:?}");
        }
        assert!(micro_cases(&"Tudo certo, sem nada.".chars().collect::<Vec<_>>()).is_empty());
        // A single word on the last line of a paragraph.
        let mut app = App::headless();
        frame(&mut app, "Uma linha que enche a moldura quase toda fim", 60.0, 40.0, TextAlign::Left);
        let r = reply(&mut app, "audit_breaks", json!({}));
        assert!(r["cases"]["single_word_last_line"]["count"].as_u64().unwrap_or(0) <= 1);
        let s = reply(&mut app, "audit_structure", json!({}));
        assert!(s["cases"].is_object());
        let rag = reply(&mut app, "audit_rag", json!({}));
        assert!(rag["paragraphs"]["count"].is_u64());
    }

    #[test]
    fn rivers_are_gaps_lined_up_over_three_lines() {
        let line = |gaps: Vec<(f64, f64)>| Line {
            page: 0,
            frame: ShapeId(1),
            para: 0,
            para_range: (0, 100),
            text: "a b".into(),
            baseline: 0.0,
            x0: 0.0,
            ink_x1: 50.0,
            measure: 50.0,
            align: TextAlign::Justify,
            style: None,
            size_pt: 10.0,
            keep_with_next: false,
            first: false,
            last: false,
            hyphenated: false,
            spaces: gaps,
            space_ratio: None,
            column: 0,
        };
        let lines = vec![
            line(vec![(10.0, 12.0), (30.0, 31.0)]),
            line(vec![(10.5, 12.5)]),
            line(vec![(11.0, 13.0), (40.0, 41.0)]),
            line(vec![(20.0, 21.0)]),
        ];
        let r = rivers(&lines);
        assert_eq!(r.len(), 1);
        assert_eq!((r[0].0, r[0].1), (0, 3));
    }

    #[test]
    fn local_formatting_is_told_apart_from_the_authors_marks() {
        let mut app = App::headless();
        app.add_book_styles();
        let id = frame(&mut app, "Texto com itálico e corpo trocado.", 80.0, 30.0, TextAlign::Left);
        let style = app.doc().text_styles.iter().find(|s| s.para.is_some()).map(|s| s.id).unwrap();
        let mut kind = app.doc().find_shape(id).unwrap().kind.clone();
        tracedraw_core::text_styles::apply_paragraph_style(&mut kind, None, &app.doc().text_styles.clone(), Some(style));
        if let ShapeKind::Text { spans, .. } = &mut kind {
            let t = spans[0].clone();
            let mut a = t.clone();
            a.text = "Texto com ".into();
            let mut b = t.clone();
            b.text = "itálico".into();
            b.italic = true;
            let mut c = t.clone();
            c.text = " e corpo trocado.".into();
            c.size_pt += 3.0;
            *spans = vec![a, b, c];
        }
        app.run(Command::SetShapeKind { shape: id, kind });
        let r = reply(&mut app, "audit_local_formatting", json!({}));
        assert_eq!(r["groups"]["author_marks_by_hand"]["count"], 1);
        assert_eq!(r["groups"]["off_the_style"]["count"], 1);
    }

    #[test]
    fn fonts_and_the_craft_base_answer_by_name() {
        let mut app = App::headless();
        let r = reply(&mut app, "font_available", json!({"names": ["DejaVu Sans", "Nenhuma Fonte Assim"]}));
        assert_eq!(r["fonts"][0]["installed"], true);
        assert_eq!(r["fonts"][1]["installed"], false);
        let c = reply(&mut app, "craft_reference", json!({}));
        assert!(c["chapters"].as_array().unwrap().len() >= 8);
        let s = reply(&mut app, "craft_reference", json!({"search": "viúva"}));
        assert!(!s["sections"].as_array().unwrap().is_empty());
        let h = reply(&mut app, "house_standard", json!({}));
        assert!(h["margins_mm"]["inner"].as_f64().unwrap() > 0.0);
    }
}
