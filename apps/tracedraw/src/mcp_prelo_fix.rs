//! Arcano edition: the MCP book and page tools that compose and fix (see
//! `mcp_prelo.rs` for the reading ones and docs/behavior/mcp.md, "Book
//! and page tools"). Each change is one undo step; none saves the
//! drawing over its file (checkpoint and save_as write copies or new
//! files and never overwrite).

use crate::app::App;
use crate::mcp_plus::Reply;
use crate::mcp_prelo::{
    bool_arg, f64_arg, page_number, page_range, pick_story, round, str_arg, story_lines, tool, usize_arg, Line, Story,
};
use serde_json::{json, Value};
use tracedraw_core::paragraphs::paragraph_ranges;
use tracedraw_core::{Command, Shape, ShapeId, ShapeKind, TextAlign, TextSpan};

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
fn ints() -> Value {
    json!({"type": "array", "items": {"type": "integer"}})
}

pub fn tools() -> Vec<Value> {
    let range = |extra: Value| {
        let mut v = json!({"story": int(), "from_page": int(), "to_page": int()});
        if let (Some(o), Some(e)) = (v.as_object_mut(), extra.as_object()) {
            for (k, x) in e {
                o.insert(k.clone(), x.clone());
            }
        }
        v
    };
    vec![
        tool(
            "create_document",
            "Open a new drawing in the house standard: format, facing pages (default true), margins (default the house's), the house's book styles, bleed and margin guides, and a primary text frame on master A (default true) so every page has a frame threaded on to the next. Then put the text in with place_text. Does not save.",
            json!({
                "width_mm": num(), "height_mm": num(), "pages": int(), "facing": boolean(), "primary_frame": boolean(),
                "margins_mm": {"type": "object", "properties": {"top": num(), "bottom": num(), "inner": num(), "outer": num()}},
                "columns": int(), "gutter_mm": num(), "name": text()
            }),
            &[],
        ),
        tool(
            "place_text",
            "Put a text into the drawing from a page, in a frame exactly in the page's content area (its primary frame when the master has one, else the house margins; no typed coordinates), threading frames on the next pages and adding pages at the end when it does not fit, until all the text fits. style is given to the whole story (what would overflow too; without it the house body style when the drawing has the book styles), first_style to the first paragraph only. text, or file (a UTF-8 .txt; paragraphs by line breaks). One undo step. Does not save.",
            json!({"text": text(), "file": text(), "page_index": int(), "style": text(), "first_style": text(), "create_pages": boolean(), "last_page": int()}),
            &[],
        ),
        tool(
            "thread_frames",
            "Link text frames in reading order so the text runs from one into the next (the text of the later ones joins the first's). Use it when text sits in separate frames. Answers the story, its frames and whether it still overflows. One undo step.",
            json!({"ids": ints()}),
            &["ids"],
        ),
        tool(
            "fit_frame",
            "Fit the height of text frames to the text they show: taller when the text overflows, shorter when the frame is too tall; the width and the top stay. Linked frames stay as they are. For notes, captions and short texts. padding_mm below the last line (0). One undo step.",
            json!({"ids": ints(), "padding_mm": num()}),
            &["ids"],
        ),
        tool(
            "apply_paragraph_style",
            "Give a paragraph style (by name) to a whole story, the text in its overflow too, or to paragraphs from..to of it (story paragraph numbers, 0 = the first, as export_lines numbers them). clear_local_formatting: also clear what differs from the style in them, keeping the author's marks (italic, bold, superscript...). One undo step.",
            json!({"style": text(), "story": int(), "from": int(), "to": int(), "clear_local_formatting": boolean()}),
            &["style"],
        ),
        tool(
            "clear_local_formatting",
            "Give the text that differs from its paragraph style the style's look again WITHOUT losing anything the author marked: font, size, colour and outline go back to the style; the author's marks (italic, bold, underline, strike-through, superscript, subscript, OpenType features such as small caps) are never touched; composition settings (tracking, offsets) only with classes including \"composition\". Checks at the end, character by character, that no mark changed, and changes nothing if one did. One undo step. Use it after audit_local_formatting.",
            range(json!({"classes": {"type": "array", "items": {"type": "string", "enum": ["style", "composition"]}}})),
            &[],
        ),
        tool(
            "fix_typography",
            "Fix the house typography in the story (all_texts: every text): curly quotes and apostrophes, the dash between words and between numbers, the ellipsis, double spaces and spaces before punctuation. Never changes words. One undo step. Use it after audit_microtypography.",
            range(json!({"all_texts": boolean()})),
            &[],
        ),
        tool(
            "fix_justification",
            "Fix loose and tight justified lines paragraph by paragraph: for each paragraph with such lines, tries the tracking of the whole paragraph from -max_tracking to +max_tracking thousandths of an em (5 at most), measures the spacing of each try and keeps the one with the most even lines. The pagination is kept: a try that changes where the paragraph or the frame ends is dropped; a paragraph never ends with more lines out of range. One undo step; answers before and after (loose and tight lines) and what changed in each paragraph. On a whole book, work in ranges of about 40 pages. Use it after audit_justification.",
            range(json!({"max_paragraphs": int(), "max_tracking": int(), "time_limit_seconds": int()})),
            &[],
        ),
        tool(
            "fix_breaks",
            "Fix widows and orphans: makes a paragraph of the frame before (widow) or of the frame itself (orphan) gain or lose a line with the tracking of the whole paragraph (-5 to +5 thousandths of an em), never with lines out of range, kept only when the story's widows and orphans in the range go down. A single word on a paragraph's last line: positive tracking (up to max_tracking_word, 15) that brings one more word down, the paragraph ending where it did; what cannot be solved without touching the text is reported (a question for the author). kinds: widow, orphan, single_word (all by default). One undo step.",
            range(json!({"kinds": {"type": "array", "items": {"type": "string", "enum": ["widow", "orphan", "single_word"]}}, "max_tracking": int(), "max_tracking_word": int(), "time_limit_seconds": int()})),
            &[],
        ),
        tool(
            "checkpoint",
            "Save a copy of the drawing as it is now (unsaved changes included) in Documents/TraceDraw/checkpoints, with the date and time in its name. The open drawing is not saved nor replaced. Use it before fixes in bulk on a client's book, besides undo. Answers the copy's path.",
            json!({"note": text()}),
            &[],
        ),
        tool(
            "save_as",
            "Save the drawing under another name (.tdraw) to work on a copy: the window then shows the new file and the old one stays as last saved. Never saves over a file: refuses the open file and any file that exists. Without folder, the folder of the current file. Use it when Dan asks to work on a copy.",
            json!({"name": text(), "folder": text()}),
            &["name"],
        ),
        tool(
            "run_script_batch",
            "Run a list of scripts (as run_script runs them) one after the other, each its own undo step. First saves a checkpoint (checkpoint: false to skip). After each step, checks whether objects it made fell off the page or overflow; stops at the first error and, by default, at the first problem, without running the rest. For repetitive work on many chapters or pages instead of many run_script calls.",
            json!({"steps": {"type": "array", "items": {"type": "object", "properties": {"source": text(), "model": {"type": "string", "enum": ["tracedraw", "vba"]}, "description": text()}, "required": ["source"]}}, "checkpoint": boolean(), "stop_on_problem": boolean()}),
            &["steps"],
        ),
        tool(
            "script_library",
            "Keep and reuse scripts that worked. Before writing a new script for a common job, look here (search). After a script worked and Dan approved the result, save it with a clear description and the arguments it expects. action: list, search, read, save.",
            json!({"action": {"type": "string", "enum": ["list", "search", "read", "save"]}, "name": text(), "description": text(), "source": text(), "model": {"type": "string", "enum": ["tracedraw", "vba"]}, "arguments": text(), "used_in": text(), "search": text(), "replace": boolean()}),
            &["action"],
        ),
        tool(
            "start_project",
            "At the start of a job from scratch, after asking whether the person wants a briefing (not when changing existing work): the short interview (at most 7 questions) for the kind of job, without what the house standard already decides, and how to run it. When the project has a briefing, answers it instead. Changes nothing.",
            json!({"name": text(), "kind": {"type": "string", "enum": ["book_interior", "book_cover", "graphic_piece", "illustration", "other"]}}),
            &["name", "kind"],
        ),
        tool(
            "save_briefing",
            "Save the project's briefing after start_project's interview: the answers (in the person's words, summed up), the visual direction as shown to the person, the decisions and what is still open. Kept in Documents/TraceDraw/projects as briefing.json and briefing.md; saving again updates it. Answers the path.",
            json!({"name": text(), "kind": text(), "answers": {"type": "array", "items": {"type": "object", "properties": {"id": text(), "question": text(), "answer": text()}}}, "direction": text(), "decisions": {"type": "array", "items": {"type": "string"}}, "open": {"type": "array", "items": {"type": "string"}}}),
            &["name", "direction"],
        ),
        tool(
            "read_briefing",
            "A project's briefing, to ground composing decisions (fonts, grid, colours, images) in what was asked. Without name, the projects with a briefing, the newest first. Changes nothing.",
            json!({"name": text()}),
            &[],
        ),
        tool(
            "compare_snapshots",
            "Compare two PNG pictures of the same page (before and after a change, from render_area with save_to, same area and size) and show where it changed: the after picture with the differences in red and boxed, the share of the picture that changed and the regions (in mm when dpi is given). Use it to check a fix moved only what it should. Does not touch the drawing.",
            json!({"before": text(), "after": text(), "dpi": num(), "threshold": int(), "save_to": text()}),
            &["before", "after"],
        ),
        tool(
            "measure_contrast",
            "Measure pixel by pixel the contrast of the letters of texts (ids, on one page) against what is really behind them (photo, gradient, colour), by the WCAG measure: median, the worst 10 percent, the share of the letters under 3:1 and under 4.5:1, and the contrast in each fifth of the width (where it fails). Warns when a stretch is under 3:1 though the median is good. suggest_position {max_mm, step_mm}: tries moving the text and answers the places where its worst stretch has the most contrast, to solve it by the composition before lightening the picture or adding a box. Changes nothing.",
            json!({"ids": ints(), "dpi": num(), "suggest_position": {"type": "object", "properties": {"max_mm": num(), "step_mm": num()}}}),
            &["ids"],
        ),
    ]
}

pub fn call(app: &mut App, name: &str, args: &Value) -> Option<Result<Reply, String>> {
    let t = |r: Result<Value, String>| Some(r.map(|v| Reply::Text(v.to_string())));
    match name {
        "create_document" => t(create_document(app, args)),
        "place_text" => t(place_text(app, args)),
        "thread_frames" => t(thread_frames(app, args)),
        "fit_frame" => t(fit_frame(app, args)),
        "apply_paragraph_style" => t(apply_paragraph_style(app, args)),
        "clear_local_formatting" => t(clear_local_formatting(app, args)),
        "fix_typography" => t(fix_typography(app, args)),
        "fix_justification" => t(fix_justification(app, args)),
        "fix_breaks" => t(fix_breaks(app, args)),
        "checkpoint" => t(checkpoint(app, args)),
        "save_as" => t(save_as(app, args)),
        "run_script_batch" => t(run_script_batch(app, args)),
        "script_library" => t(script_library(args)),
        "start_project" => t(start_project(args)),
        "save_briefing" => t(save_briefing(args)),
        "read_briefing" => t(read_briefing(args)),
        "compare_snapshots" => Some(compare_snapshots(args)),
        "measure_contrast" => t(measure_contrast(app, args)),
        _ => None,
    }
}

// ----- where files go ----------------------------------------------------------

/// The folder the tools keep their files in: Documents/TraceDraw (a
/// folder of its own in tests).
pub(crate) fn work_dir() -> Result<std::path::PathBuf, String> {
    if cfg!(test) {
        let d = std::env::temp_dir().join(format!("tracedraw-mcp-{}", std::process::id()));
        std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
        return Ok(d);
    }
    let docs = crate::files::places()
        .into_iter()
        .find(|(p, _)| *p == crate::files::Place::Documents)
        .map(|(_, d)| d)
        .or_else(|| crate::files::home_dir().map(|h| h.join("Documents")))
        .ok_or("no Documents folder on this computer")?;
    let d = docs.join("TraceDraw");
    std::fs::create_dir_all(&d).map_err(|e| format!("could not make {}: {e}", d.display()))?;
    Ok(d)
}

fn sub_dir(name: &str) -> Result<std::path::PathBuf, String> {
    let d = work_dir()?.join(name);
    std::fs::create_dir_all(&d).map_err(|e| format!("could not make {}: {e}", d.display()))?;
    Ok(d)
}

/// A file name from free text: letters, digits and dashes.
pub(crate) fn slug(s: &str) -> String {
    let mut out = String::new();
    for c in s.trim().chars() {
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    let out = out.trim_end_matches('-').chars().take(60).collect::<String>();
    if out.is_empty() {
        "sem-nome".into()
    } else {
        out
    }
}

/// The date and time now, as "2026-11-08_14-05-09" (UTC).
pub(crate) fn stamp() -> String {
    let secs = web_time::SystemTime::now()
        .duration_since(web_time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let days = secs.div_euclid(86_400);
    let rest = secs.rem_euclid(86_400);
    // Civil date from days since 1970-01-01.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}_{:02}-{:02}-{:02}", rest / 3600, rest % 3600 / 60, rest % 60)
}

fn save_copy(app: &App, path: &std::path::Path) -> Result<(), String> {
    tracedraw_io::save_native(&tracedraw_text::fonts().with_carried_fonts(app.engine.document()), path)
        .map_err(|e| format!("could not save {}: {e}", path.display()))
}

// ----- composing --------------------------------------------------------------

fn paragraph_style_id(app: &App, name: &str) -> Result<u32, String> {
    let styles = &app.doc().text_styles;
    styles
        .iter()
        .find(|s| s.para.is_some() && s.name == name)
        .or_else(|| styles.iter().find(|s| s.para.is_some() && s.name.eq_ignore_ascii_case(name)))
        .map(|s| s.id)
        .ok_or_else(|| {
            let names: Vec<&str> = styles.iter().filter(|s| s.para.is_some()).map(|s| s.name.as_str()).collect();
            format!("no paragraph style \"{name}\"; the drawing has: {}", names.join(", "))
        })
}

fn margins_arg(app: &App, args: &Value) -> [f64; 4] {
    let h = crate::prelo_casa::house_prefs(app);
    let m = args.get("margins_mm");
    let g = |k: &str, d: f64| m.and_then(|m| m.get(k)).and_then(|v| v.as_f64()).filter(|v| v.is_finite() && *v >= 0.0).unwrap_or(d);
    [g("top", h.top), g("bottom", h.bottom), g("inner", h.inner), g("outer", h.outer)]
}

fn create_document(app: &mut App, args: &Value) -> Result<Value, String> {
    let mut settings = app.settings.new_document.clone();
    if let Some(w) = f64_arg(args, "width_mm") {
        settings.values.width_mm = w.clamp(1.0, 5000.0);
    }
    if let Some(h) = f64_arg(args, "height_mm") {
        settings.values.height_mm = h.clamp(1.0, 5000.0);
    }
    settings.pages = usize_arg(args, "pages").unwrap_or(1).clamp(1, 999) as u32;
    settings.facing = bool_arg(args, "facing").unwrap_or(true);
    settings.primary_frame = false;
    let margins = margins_arg(app, args);
    let mut st = crate::new_document::NewDocState::from_settings(&settings, str_arg(args, "name").unwrap_or("").to_string());
    st.found = Some(Vec::new());
    let keep = app.settings.new_document.clone();
    crate::new_document::create(app, &st);
    app.settings.new_document = keep;
    app.apply_house_standard();
    if bool_arg(args, "primary_frame").unwrap_or(true) {
        if let Some(mut a) = app.doc().master_page(0) {
            a.primary = Some(tracedraw_core::PrimaryFrame {
                margins,
                columns: usize_arg(args, "columns").unwrap_or(1).clamp(1, 20) as u32,
                gutter: f64_arg(args, "gutter_mm").unwrap_or(5.0).clamp(0.0, 100.0),
            });
            app.run(Command::SetMasterPage { master: a });
            app.add_primary_frames_for_master(0);
        }
    }
    let doc = app.doc().clone();
    app.engine.replace(doc);
    Ok(json!({
        "drawing": app.doc().title,
        "pages": app.doc().pages.len(),
        "size_mm": app.doc().pages.first().map(|p| json!({"width": p.size.width, "height": p.size.height})),
        "facing_pages": app.doc().metadata.facing_pages,
        "margins_mm": {"top": margins[0], "bottom": margins[1], "inner": margins[2], "outer": margins[3]},
        "primary_frame": app.primary_frame_on(0).map(|f| f.0),
        "paragraph_styles": app.doc().text_styles.iter().filter(|s| s.para.is_some()).map(|s| s.name.clone()).collect::<Vec<_>>(),
        "next": "place_text puts the text in",
    }))
}

/// The text of `args` (text, or file).
fn text_arg(args: &Value) -> Result<String, String> {
    if let Some(t) = str_arg(args, "text").filter(|t| !t.is_empty()) {
        return Ok(t.replace("\r\n", "\n").replace('\r', "\n"));
    }
    if let Some(f) = str_arg(args, "file").filter(|t| !t.is_empty()) {
        let t = std::fs::read_to_string(f).map_err(|e| format!("could not read {f}: {e}"))?;
        return Ok(t.replace("\r\n", "\n").replace('\r', "\n"));
    }
    Err("give text or file".into())
}

fn frame_is_empty(app: &App, id: ShapeId) -> bool {
    match app.doc().find_shape(id).map(|s| &s.kind) {
        Some(ShapeKind::Text { spans, .. }) => spans.iter().all(|s| s.text.trim().is_empty()),
        _ => false,
    }
}

/// A new frame in the content area of page `idx`, unlinked.
fn frame_in_area(app: &mut App, idx: usize) -> Option<ShapeId> {
    let (x0, y0, x1, y1) = crate::mcp_prelo::content_area(app, idx)?;
    let page = app.doc().pages.get(idx)?.id;
    let (columns, gutter) = app.primary_of_page(page).map(|p| (p.columns, p.gutter)).unwrap_or((1, 5.0));
    let kind = app.blank_frame_kind(
        tracedraw_core::Point::new(x0, y0),
        tracedraw_core::Size::new(x1 - x0, y1 - y0),
        columns,
        gutter,
    );
    let (keep_page, keep_layer) = (app.page, app.picked_layer);
    app.page = page;
    app.picked_layer = None;
    let made = app.new_shape_styled(kind, |s| crate::primary_frames::keep_columns(s, columns, gutter));
    app.page = keep_page;
    app.picked_layer = keep_layer;
    made
}

fn overflows(app: &App, id: ShapeId) -> bool {
    app.chain_of(id).last().and_then(|l| app.text_layout_of(*l)).is_some_and(|(l, _)| l.overflow)
}

fn place_text(app: &mut App, args: &Value) -> Result<Value, String> {
    let body = text_arg(args)?;
    let n = app.doc().pages.len();
    let start = usize_arg(args, "page_index").unwrap_or(0);
    if start >= n {
        return Err(format!("page_index {start} out of range (0 to {})", n.saturating_sub(1)));
    }
    let last = usize_arg(args, "last_page").map(|l| l.min(n - 1));
    let create = bool_arg(args, "create_pages").unwrap_or(true) && last.is_none();
    // Without a style, the house body style when the drawing has it.
    let style = match str_arg(args, "style").filter(|s| !s.is_empty()) {
        Some(s) => Some(paragraph_style_id(app, s)?),
        None => paragraph_style_id(app, &crate::i18n::tr("tstyle.book_body")).ok(),
    };
    let first_style = str_arg(args, "first_style").filter(|s| !s.is_empty()).map(|s| paragraph_style_id(app, s)).transpose()?;
    let pages_before = n;
    let mark = app.engine.begin_group();
    let first = match app.primary_frame_on(start).filter(|f| frame_is_empty(app, *f) && app.prev_frame(*f).is_none_or(|p| frame_is_empty(app, p))) {
        Some(f) => f,
        None => match frame_in_area(app, start) {
            Some(f) => f,
            None => {
                app.engine.end_group(mark, "Place Text");
                return Err("could not make a frame on that page".into());
            }
        },
    };
    let Some(mut kind) = app.doc().find_shape(first).map(|s| s.kind.clone()) else {
        app.engine.end_group(mark, "Place Text");
        return Err("the frame went away".into());
    };
    let styles = app.doc().text_styles.clone();
    if let ShapeKind::Text { spans, .. } = &mut kind {
        let mut sp = spans.first().cloned().unwrap_or_else(|| TextSpan::new("", app.text_font.clone(), app.text_size_pt));
        sp.text = body.clone();
        *spans = vec![sp];
    }
    if style.is_some() {
        tracedraw_core::text_styles::apply_paragraph_style(&mut kind, None, &styles, style);
    }
    if first_style.is_some() {
        let end = body.find('\n').map(|i| body[..i].chars().count()).unwrap_or(body.chars().count());
        tracedraw_core::text_styles::apply_paragraph_style(&mut kind, Some((0, end)), &styles, first_style);
    }
    app.run(Command::SetShapeKind { shape: first, kind });
    app.reflow_chain(first);
    // The pages after: their frames, linked on.
    loop {
        if !overflows(app, first) {
            break;
        }
        let Some(&tail) = app.chain_of(first).last() else { break };
        let Some(p) = app.page_index_of(tail) else { break };
        let next = p + 1;
        if next >= app.doc().pages.len() || last.is_some_and(|l| next > l) {
            break;
        }
        let Some(f) = frame_in_area(app, next) else { break };
        if !app.link_frames(tail, f) {
            break;
        }
    }
    let mut added = 0;
    if create && overflows(app, first) {
        added = app.autoflow(first);
    }
    app.engine.end_group(mark, "Place Text");
    let chain = app.chain_of(first);
    let pages: Vec<String> = {
        let mut v: Vec<usize> = chain.iter().filter_map(|f| app.page_index_of(*f)).collect();
        v.dedup();
        v.into_iter().map(|p| page_number(app, p)).collect()
    };
    Ok(json!({
        "story": first.0,
        "frames": chain.len(),
        "pages": pages,
        "pages_added": app.doc().pages.len().saturating_sub(pages_before).max(added),
        "overflows": overflows(app, first),
        "paragraphs": body.split('\n').count(),
    }))
}

fn ids(args: &Value) -> Result<Vec<ShapeId>, String> {
    let v: Vec<ShapeId> = args
        .get("ids")
        .and_then(|v| v.as_array())
        .ok_or("ids must be a list of object ids")?
        .iter()
        .filter_map(|v| v.as_u64())
        .map(ShapeId)
        .collect();
    if v.is_empty() {
        return Err("ids is empty".into());
    }
    Ok(v)
}

fn thread_frames(app: &mut App, args: &Value) -> Result<Value, String> {
    let list = ids(args)?;
    if list.len() < 2 {
        return Err("give at least two frames".into());
    }
    for id in &list {
        match app.doc().find_shape(*id).map(|s| &s.kind) {
            Some(ShapeKind::Text { frame: Some(_), .. }) => {}
            Some(_) => return Err(format!("{} is not a text frame", id.0)),
            None => return Err(format!("no object with id {}", id.0)),
        }
    }
    let mark = app.engine.begin_group();
    let mut refused = Vec::new();
    for w in list.windows(2) {
        // Each joins the chain of the one before at its end.
        let tail = app.chain_of(w[0]).last().copied().unwrap_or(w[0]);
        let head = app.chain_head(w[1]);
        if tail == head || app.chain_of(w[0]).contains(&w[1]) {
            continue;
        }
        if !app.link_frames(tail, head) {
            refused.push(json!([w[0].0, w[1].0]));
        }
    }
    app.engine.end_group(mark, "Thread Frames");
    let chain = app.chain_of(list[0]);
    Ok(json!({
        "story": chain.first().map(|f| f.0),
        "frames": chain.iter().map(|f| f.0).collect::<Vec<_>>(),
        "overflows": overflows(app, list[0]),
        "not_linked": refused,
    }))
}

/// The frame `s` with height `h`, its top where it was.
fn with_height(s: &Shape, h: f64) -> Shape {
    let mut s = s.clone();
    if let ShapeKind::Text { origin, frame: Some(size), .. } = &mut s.kind {
        let top = origin.y + size.height;
        size.height = h;
        origin.y = top - h;
    }
    s
}

/// Whether frame `s` shows all its text: nothing overflows and the last
/// line's descent is inside the frame (a first line too tall for a
/// frame still shows, so overflow alone does not say it).
fn fits(s: &Shape) -> bool {
    let ShapeKind::Text { spans, frame: Some(size), .. } = &s.kind else {
        return false;
    };
    let total: usize = spans.iter().map(|sp| sp.text.chars().count()).sum();
    crate::text_editing::layout_of_text_shape(s, Default::default()).is_some_and(|(l, _)| {
        let bottom = l.lines.iter().map(|b| -b.baseline + b.descent.abs()).fold(0.0, f64::max);
        !l.overflow && l.fitted_chars >= total && bottom <= size.height + 0.01
    })
}

fn fit_frame(app: &mut App, args: &Value) -> Result<Value, String> {
    let list = ids(args)?;
    let pad = f64_arg(args, "padding_mm").unwrap_or(0.0).clamp(0.0, 20.0);
    let mut cmds = Vec::new();
    let mut out = Vec::new();
    for id in list {
        let Some(s) = app.doc().find_shape(id).cloned() else {
            out.push(json!({"id": id.0, "skipped": "no such object"}));
            continue;
        };
        let ShapeKind::Text { frame: Some(size), .. } = &s.kind else {
            out.push(json!({"id": id.0, "skipped": "not a text frame"}));
            continue;
        };
        if app.next_frame(id).is_some() || app.prev_frame(id).is_some() {
            out.push(json!({"id": id.0, "skipped": "linked frames stay as they are"}));
            continue;
        }
        let was = size.height;
        let mut hi = was.max(1.0);
        let mut grow = 0;
        while !fits(&with_height(&s, hi)) && grow < 12 {
            hi *= 2.0;
            grow += 1;
        }
        if !fits(&with_height(&s, hi)) {
            out.push(json!({"id": id.0, "skipped": "the text does not fit even 4000 times taller"}));
            continue;
        }
        let mut lo = 0.5;
        for _ in 0..30 {
            let mid = (lo + hi) / 2.0;
            if fits(&with_height(&s, mid)) {
                hi = mid;
            } else {
                lo = mid;
            }
            if hi - lo < 0.05 {
                break;
            }
        }
        let h = hi + pad;
        let fitted = with_height(&s, h);
        cmds.push(Command::SetShapeKind { shape: id, kind: fitted.kind });
        out.push(json!({"id": id.0, "height_mm": {"before": round(was), "after": round(h)}}));
    }
    if !cmds.is_empty() {
        app.engine.run_batch("Fit Frame", &cmds).map_err(|e| e.to_string())?;
    }
    Ok(json!({"frames": out}))
}

/// The paragraph number in the story of the first paragraph of each
/// frame.
fn frame_bases(app: &App, story: &Story) -> Vec<(ShapeId, usize, usize)> {
    let mut out = Vec::new();
    let mut base = 0;
    for &(id, page) in &story.frames {
        let Some(ShapeKind::Text { spans, .. }) = app.doc().find_shape(id).map(|s| &s.kind) else {
            continue;
        };
        out.push((id, page, base));
        let n = paragraph_ranges(spans).len();
        let open = !spans.iter().rev().find(|s| !s.text.is_empty()).is_some_and(|s| s.text.ends_with('\n'));
        base += n - usize::from(open);
    }
    out
}

fn apply_paragraph_style(app: &mut App, args: &Value) -> Result<Value, String> {
    let name = str_arg(args, "style").ok_or("missing argument: style")?;
    let id = paragraph_style_id(app, name)?;
    let story = pick_story(app, args)?;
    let from = usize_arg(args, "from").unwrap_or(0);
    let to = usize_arg(args, "to").unwrap_or(usize::MAX);
    let clear = bool_arg(args, "clear_local_formatting").unwrap_or(false);
    let styles = app.doc().text_styles.clone();
    let mut cmds = Vec::new();
    let mut paras = 0;
    for (fid, _, base) in frame_bases(app, &story) {
        let Some(mut kind) = app.doc().find_shape(fid).map(|s| s.kind.clone()) else { continue };
        let ShapeKind::Text { spans, .. } = &kind else { continue };
        let ranges = paragraph_ranges(spans);
        let mut touched = Vec::new();
        for (p, &(s, e)) in ranges.iter().enumerate() {
            let g = base + p;
            if g >= from && g <= to {
                touched.push((s, e));
            }
        }
        if touched.is_empty() {
            continue;
        }
        for &(s, e) in &touched {
            tracedraw_core::text_styles::apply_paragraph_style(&mut kind, Some((s, e)), &styles, Some(id));
            paras += 1;
        }
        if clear {
            let (k, _) = cleared(&kind, &styles, &["style"], Some(&touched));
            kind = k;
        }
        cmds.push(Command::SetShapeKind { shape: fid, kind });
    }
    if cmds.is_empty() {
        return Err("no paragraph in that range".into());
    }
    let mark = app.engine.begin_group();
    let r = app.engine.run_batch("Apply Paragraph Style", &cmds);
    app.reflow_chain(story.head);
    app.engine.end_group(mark, "Apply Paragraph Style");
    r.map_err(|e| e.to_string())?;
    Ok(json!({"story": story.head.0, "style": name, "paragraphs": paras, "overflows": overflows(app, story.head)}))
}

/// Text `kind` with what differs from the paragraph styles taken back
/// to the style, for the property classes `classes` ("style",
/// "composition"); the author's marks never change. `only`: just the
/// characters of these ranges. The count of runs changed.
pub(crate) fn cleared(
    kind: &ShapeKind,
    styles: &[tracedraw_core::text_styles::TextStyle],
    classes: &[&str],
    only: Option<&[(usize, usize)]>,
) -> (ShapeKind, usize) {
    let mut k = kind.clone();
    let ShapeKind::Text { spans, align, para, .. } = &mut k else {
        return (k, 0);
    };
    let ranges = paragraph_ranges(spans);
    let formats = tracedraw_core::paragraphs::paragraph_formats_of(spans, *align, para);
    let mut changed = 0;
    let mut pos = 0;
    for sp in spans.iter_mut() {
        let len = sp.text.chars().count();
        let (a, b) = (pos, pos + len);
        pos = b;
        if len == 0 || sp.char_style.is_some() || sp.text.trim().is_empty() {
            continue;
        }
        if only.is_some_and(|o| !o.iter().any(|&(s, e)| a <= e && b > s)) {
            continue;
        }
        let Some(p) = ranges.iter().position(|&(s, e)| a >= s && a <= e) else { continue };
        let Some(id) = formats.get(p).and_then(|f| f.style) else { continue };
        let chars = tracedraw_core::text_styles::resolve(styles, id).1;
        let mut want = sp.clone();
        chars.apply(&mut want);
        if want == *sp {
            continue;
        }
        let (Ok(Value::Object(mut have)), Ok(Value::Object(w))) = (serde_json::to_value(&*sp), serde_json::to_value(&want)) else {
            continue;
        };
        let keys: Vec<String> = have.keys().chain(w.keys()).cloned().collect();
        for key in keys {
            if matches!(key.as_str(), "text" | "paragraph" | "char_style") {
                continue;
            }
            if classes.contains(&crate::mcp_prelo::override_class(&key)) {
                match w.get(&key) {
                    Some(v) => {
                        have.insert(key, v.clone());
                    }
                    None => {
                        have.remove(&key);
                    }
                }
            }
        }
        if let Ok(new) = serde_json::from_value::<TextSpan>(Value::Object(have)) {
            if new != *sp {
                *sp = new;
                changed += 1;
            }
        }
    }
    *spans = tracedraw_core::document::merge_equal_spans(std::mem::take(spans));
    (k, changed)
}

/// The author's marks of each character, to check nothing was lost.
fn marks(kind: &ShapeKind) -> Vec<(bool, bool, bool, bool, i8, Vec<String>)> {
    let ShapeKind::Text { spans, .. } = kind else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for sp in spans {
        let m = (
            sp.bold,
            sp.italic,
            sp.underline,
            sp.strikethrough,
            (sp.baseline_shift_pt.signum()) as i8,
            sp.features.clone(),
        );
        for _ in sp.text.chars() {
            out.push(m.clone());
        }
    }
    out
}

fn story_frames_in(app: &App, args: &Value) -> Result<(Story, Vec<ShapeId>), String> {
    let story = pick_story(app, args)?;
    let (a, b) = page_range(app, args);
    let frames = story.frames.iter().filter(|(_, p)| *p >= a && *p <= b).map(|(f, _)| *f).collect();
    Ok((story, frames))
}

fn clear_local_formatting(app: &mut App, args: &Value) -> Result<Value, String> {
    let (story, frames) = story_frames_in(app, args)?;
    let classes: Vec<String> = args
        .get("classes")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|c| c.as_str().map(str::to_string)).collect())
        .unwrap_or_else(|| vec!["style".into()]);
    let classes: Vec<&str> = classes.iter().map(String::as_str).filter(|c| *c == "style" || *c == "composition").collect();
    let styles = app.doc().text_styles.clone();
    let mut cmds = Vec::new();
    let mut runs = 0;
    for id in frames {
        let Some(kind) = app.doc().find_shape(id).map(|s| s.kind.clone()) else { continue };
        let (k, n) = cleared(&kind, &styles, &classes, None);
        if n == 0 {
            continue;
        }
        if marks(&k) != marks(&kind) {
            return Err(format!("clearing frame {} would change an author's mark; nothing was changed", id.0));
        }
        runs += n;
        cmds.push(Command::SetShapeKind { shape: id, kind: k });
    }
    if !cmds.is_empty() {
        let mark = app.engine.begin_group();
        let r = app.engine.run_batch("Clear Local Formatting", &cmds);
        app.reflow_chain(story.head);
        app.engine.end_group(mark, "Clear Local Formatting");
        r.map_err(|e| e.to_string())?;
    }
    let left: usize = story
        .frames
        .iter()
        .filter_map(|(f, _)| app.doc().find_shape(*f))
        .map(|s| crate::mcp_prelo::overrides_of(&s.kind, &styles).len())
        .sum();
    Ok(json!({"story": story.head.0, "runs_cleared": runs, "author_marks": "kept", "overrides_left_in_story": left}))
}

fn fix_typography(app: &mut App, args: &Value) -> Result<Value, String> {
    let targets: Vec<ShapeId> = if bool_arg(args, "all_texts").unwrap_or(false) {
        let mut v = Vec::new();
        fn walk(s: &Shape, out: &mut Vec<ShapeId>) {
            match &s.kind {
                ShapeKind::Group { children } => children.iter().for_each(|c| walk(c, out)),
                ShapeKind::ClipFrame { frame, contents } => {
                    walk(frame, out);
                    contents.iter().for_each(|c| walk(c, out));
                }
                ShapeKind::Text { .. } => out.push(s.id),
                _ => {}
            }
        }
        for l in app.doc().all_layers() {
            for s in &l.shapes {
                walk(s, &mut v);
            }
        }
        v
    } else {
        story_frames_in(app, args)?.1
    };
    let mut cmds = Vec::new();
    let mut changes = 0;
    for id in &targets {
        let Some(mut kind) = app.doc().find_shape(*id).map(|s| s.kind.clone()) else { continue };
        let ShapeKind::Text { spans, .. } = &mut kind else { continue };
        let before: Vec<String> = spans.iter().flat_map(|s| s.text.split_whitespace().map(|w| w.chars().filter(|c| c.is_alphanumeric()).collect::<String>())).filter(|w| !w.is_empty()).collect();
        let (fixed, n) = crate::prelo_casa::typography_spans(spans);
        if n == 0 {
            continue;
        }
        let after: Vec<String> = fixed.iter().flat_map(|s| s.text.split_whitespace().map(|w| w.chars().filter(|c| c.is_alphanumeric()).collect::<String>())).filter(|w| !w.is_empty()).collect();
        if before != after {
            return Err(format!("the fix would change words in object {}; nothing was changed", id.0));
        }
        *spans = tracedraw_core::document::merge_equal_spans(fixed);
        changes += n;
        cmds.push(Command::SetShapeKind { shape: *id, kind });
    }
    if !cmds.is_empty() {
        let mark = app.engine.begin_group();
        let r = app.engine.run_batch("House Typography", &cmds);
        if let Some(f) = targets.first() {
            app.reflow_chain(*f);
        }
        app.engine.end_group(mark, "House Typography");
        r.map_err(|e| e.to_string())?;
    }
    Ok(json!({"texts_changed": cmds.len(), "changes": changes}))
}

// ----- fixing justification and breaks ----------------------------------------

/// How paragraph `ps..pe` of frame `s` sits: its lines, the frame's
/// fitted characters, and the space ratio of its justified lines.
struct ParaFit {
    lines: usize,
    fitted: usize,
    ratios: Vec<f64>,
    last_words: usize,
}

fn para_fit(s: &Shape, (ps, pe): (usize, usize)) -> Option<ParaFit> {
    let (layout, _) = crate::text_editing::layout_of_text_shape(s, Default::default())?;
    let ShapeKind::Text { spans, align, para, .. } = &s.kind else {
        return None;
    };
    let text: Vec<char> = spans.iter().flat_map(|sp| sp.text.chars()).collect();
    let formats = tracedraw_core::paragraphs::paragraph_formats_of(spans, *align, para);
    let ranges = paragraph_ranges(spans);
    let p = ranges.iter().position(|&r| r == (ps, pe))?;
    let f_align = formats.get(p).map(|f| f.align).unwrap_or(*align);
    let justified = matches!(f_align, TextAlign::Justify | TextAlign::ForceJustify);
    let left = if justified { crate::prelo_quebras::flush_left(s) } else { None };
    let mine: Vec<&tracedraw_text::LineBox> = layout
        .lines
        .iter()
        .filter(|b| b.start_char >= ps && b.start_char <= pe && b.start_char < layout.fitted_chars.max(1))
        .collect();
    let mut ratios = Vec::new();
    for b in &mine {
        if b.end_char >= pe && f_align == TextAlign::Justify {
            continue;
        }
        let mut e = b.end_char;
        while e > b.start_char && text.get(e - 1).is_some_and(|c| c.is_whitespace()) {
            e -= 1;
        }
        if !text.get(b.start_char..e).is_some_and(|t| t.contains(&' ')) {
            continue;
        }
        if let Some(n) = left.as_ref().and_then(|l| l.lines.iter().find(|n| n.start_char == b.start_char && n.end_char == b.end_char)) {
            let a = crate::prelo_quebras::space_width(&text, b, e);
            let pl = crate::prelo_quebras::space_width(&text, n, e);
            if let Some(r) = crate::prelo_quebras::space_ratio(a, pl, 0.0, 1) {
                ratios.push(r);
            }
        }
    }
    let last_words = mine
        .last()
        .map(|b| text.get(b.start_char..b.end_char).unwrap_or(&[]).iter().collect::<String>().split_whitespace().count())
        .unwrap_or(0);
    Some(ParaFit {
        lines: mine.len(),
        fitted: layout.fitted_chars,
        ratios,
        last_words,
    })
}

fn out_of_range(r: &[f64]) -> usize {
    r.iter().filter(|r| **r > crate::mcp_prelo::LOOSE || **r < crate::mcp_prelo::TIGHT).count()
}

fn evenness(r: &[f64]) -> f64 {
    r.iter().map(|r| (r - 1.0) * (r - 1.0)).sum::<f64>() + 10.0 * out_of_range(r) as f64
}

/// Shape `s` with the tracking of characters `ps..pe` moved by `t`
/// thousandths of an em.
fn tracked(s: &Shape, (ps, pe): (usize, usize), t: i32) -> Shape {
    let mut s = s.clone();
    if let ShapeKind::Text { spans, .. } = &mut s.kind {
        tracedraw_core::document::spans_apply(spans, ps, pe.max(ps + 1), |sp| sp.tracking_pct += t as f64 / 10.0);
    }
    s
}

fn time_limit(args: &Value, default: u64) -> web_time::Instant {
    let secs = usize_arg(args, "time_limit_seconds").map(|v| v as u64).unwrap_or(default).clamp(5, 780);
    web_time::Instant::now() + std::time::Duration::from_secs(secs)
}

fn loose_tight(lines: &[Line]) -> (usize, usize) {
    let l = lines.iter().filter(|l| l.space_ratio.is_some_and(|r| r > crate::mcp_prelo::LOOSE)).count();
    let t = lines.iter().filter(|l| l.space_ratio.is_some_and(|r| r < crate::mcp_prelo::TIGHT)).count();
    (l, t)
}

fn fix_justification(app: &mut App, args: &Value) -> Result<Value, String> {
    let story = pick_story(app, args)?;
    let range = page_range(app, args);
    let max_t = usize_arg(args, "max_tracking").unwrap_or(5).clamp(1, 5) as i32;
    let max_p = usize_arg(args, "max_paragraphs").unwrap_or(300).clamp(1, 2000);
    let deadline = time_limit(args, 240);
    let lines = story_lines(app, &story, range, true);
    let before = loose_tight(&lines);
    // The paragraphs with lines out of range, worst first.
    let mut bad: Vec<(f64, ShapeId, (usize, usize))> = Vec::new();
    for l in &lines {
        if let Some(r) = l.space_ratio {
            if r > crate::mcp_prelo::LOOSE || r < crate::mcp_prelo::TIGHT {
                let d = (r - 1.0).abs();
                match bad.iter_mut().find(|b| b.1 == l.frame && b.2 == l.para_range) {
                    Some(b) => b.0 = b.0.max(d),
                    None => bad.push((d, l.frame, l.para_range)),
                }
            }
        }
    }
    bad.sort_by(|a, b| b.0.total_cmp(&a.0));
    bad.truncate(max_p);
    let mut edits: std::collections::HashMap<ShapeId, Shape> = Default::default();
    let mut changed = Vec::new();
    let mut timed_out = false;
    for (_, frame, pr) in bad {
        if web_time::Instant::now() > deadline {
            timed_out = true;
            break;
        }
        let Some(cur) = edits.get(&frame).cloned().or_else(|| app.doc().find_shape(frame).cloned()) else { continue };
        let Some(base) = para_fit(&cur, pr) else { continue };
        let mut best = (evenness(&base.ratios), 0, None);
        for t in (-max_t..=max_t).filter(|t| *t != 0) {
            let s = tracked(&cur, pr, t);
            let Some(f) = para_fit(&s, pr) else { continue };
            if f.fitted != base.fitted || f.lines != base.lines || out_of_range(&f.ratios) > out_of_range(&base.ratios) {
                continue;
            }
            let e = evenness(&f.ratios);
            if e < best.0 - 1e-9 {
                best = (e, t, Some(s));
            }
        }
        if let (t, Some(s)) = (best.1, best.2) {
            let text: String = match &s.kind {
                ShapeKind::Text { spans, .. } => spans.iter().flat_map(|sp| sp.text.chars()).skip(pr.0).take(40).collect(),
                _ => String::new(),
            };
            changed.push(json!({"frame": frame.0, "page": app.page_index_of(frame).map(|p| page_number(app, p)), "tracking": t, "paragraph_starts": text}));
            edits.insert(frame, s);
        }
    }
    let cmds: Vec<Command> = edits.into_iter().map(|(id, s)| Command::SetShapeKind { shape: id, kind: s.kind }).collect();
    if !cmds.is_empty() {
        app.engine.run_batch("Fix Justification", &cmds).map_err(|e| e.to_string())?;
    }
    let after = loose_tight(&story_lines(app, &story, range, true));
    Ok(json!({
        "story": story.head.0,
        "before": {"loose": before.0, "tight": before.1},
        "after": {"loose": after.0, "tight": after.1},
        "paragraphs_changed": changed.len(),
        "changes": changed,
        "unfinished": timed_out,
        "note": "tracking in thousandths of an em; the pagination did not change",
    }))
}

fn count_widows_orphans(app: &App, story: &Story, range: (usize, usize)) -> usize {
    let lines = story_lines(app, story, range, false);
    crate::mcp_prelo::break_cases(&lines)
        .iter()
        .filter(|(k, _)| *k == "widow" || *k == "orphan")
        .count()
}

fn fix_breaks(app: &mut App, args: &Value) -> Result<Value, String> {
    let story = pick_story(app, args)?;
    let range = page_range(app, args);
    let kinds: Vec<String> = args
        .get("kinds")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|k| k.as_str().map(str::to_string)).collect())
        .unwrap_or_else(|| vec!["widow".into(), "orphan".into(), "single_word".into()]);
    let max_t = usize_arg(args, "max_tracking").unwrap_or(5).clamp(1, 5) as i32;
    let max_w = usize_arg(args, "max_tracking_word").unwrap_or(15).clamp(1, 15) as i32;
    let deadline = time_limit(args, 240);
    let mark = app.engine.begin_group();
    let mut fixed = Vec::new();
    let mut unsolved = Vec::new();
    let mut timed_out = false;
    // A single word on the last line: tracking that brings one more word
    // down, the paragraph and the frame ending where they did.
    if kinds.iter().any(|k| k == "single_word") {
        let lines = story_lines(app, &story, range, false);
        let cases: Vec<Line> = crate::mcp_prelo::break_cases(&lines)
            .into_iter()
            .filter(|(k, _)| *k == "single_word_last_line")
            .map(|(_, i)| lines[i].clone())
            .collect();
        let mut cmds = Vec::new();
        let mut edits: std::collections::HashMap<ShapeId, Shape> = Default::default();
        for l in cases {
            if web_time::Instant::now() > deadline {
                timed_out = true;
                break;
            }
            let Some(cur) = edits.get(&l.frame).cloned().or_else(|| app.doc().find_shape(l.frame).cloned()) else { continue };
            let Some(base) = para_fit(&cur, l.para_range) else { continue };
            let mut done = None;
            for t in 1..=max_w {
                let s = tracked(&cur, l.para_range, t);
                let Some(f) = para_fit(&s, l.para_range) else { continue };
                if f.fitted == base.fitted && f.lines == base.lines && f.last_words >= 2 && out_of_range(&f.ratios) <= out_of_range(&base.ratios) {
                    done = Some((t, s));
                    break;
                }
            }
            match done {
                Some((t, s)) => {
                    fixed.push(json!({"kind": "single_word_last_line", "page": page_number(app, l.page), "frame": l.frame.0, "tracking": t, "text": l.text.trim()}));
                    edits.insert(l.frame, s);
                }
                None => unsolved.push(json!({"kind": "single_word_last_line", "page": page_number(app, l.page), "frame": l.frame.0, "text": l.text.trim(), "why": "no tracking within the limit keeps the paragraph's end; only a change in the text would (ask the author)"})),
            }
        }
        for (id, s) in edits {
            cmds.push(Command::SetShapeKind { shape: id, kind: s.kind });
        }
        if !cmds.is_empty() {
            let _ = app.engine.run_batch("Fix Breaks", &cmds);
        }
    }
    // Widows and orphans: a paragraph gains or loses a line.
    let wo: Vec<&str> = kinds.iter().filter_map(|k| match k.as_str() {
        "widow" => Some("widow"),
        "orphan" => Some("orphan"),
        _ => None,
    }).collect();
    if !wo.is_empty() {
        let mut guard = 0;
        loop {
            guard += 1;
            if guard > 200 {
                break;
            }
            if web_time::Instant::now() > deadline {
                timed_out = true;
                break;
            }
            let story = pick_story(app, &json!({"story": story.head.0})).unwrap_or_else(|_| story.clone());
            let lines = story_lines(app, &story, range, false);
            let cases: Vec<(&str, usize)> = crate::mcp_prelo::break_cases(&lines).into_iter().filter(|(k, _)| wo.contains(k)).collect();
            let already: Vec<String> = unsolved.iter().filter_map(|u| u["text"].as_str().map(str::to_string)).collect();
            let Some(&(kind, i)) = cases.iter().find(|(_, i)| !already.contains(&lines[*i].text.trim().to_string())) else {
                break;
            };
            let l = lines[i].clone();
            // The frame whose paragraph moves: the one before a widow, the
            // orphan's own.
            let host = if kind == "widow" { i.checked_sub(1).map(|k| lines[k].frame) } else { Some(l.frame) };
            let Some(host) = host else { break };
            let count = count_widows_orphans(app, &story, range);
            let candidates: Vec<(usize, usize)> = {
                let mut v: Vec<(usize, usize)> = lines.iter().filter(|x| x.frame == host).map(|x| x.para_range).collect();
                v.dedup();
                v
            };
            let mut solved = false;
            'outer: for pr in candidates {
                let Some(cur) = app.doc().find_shape(host).cloned() else { break };
                let Some(base) = para_fit(&cur, pr) else { continue };
                for t in [-1, 1, -2, 2, -3, 3, -4, 4, -5, 5].into_iter().filter(|t: &i32| t.abs() <= max_t) {
                    if web_time::Instant::now() > deadline {
                        timed_out = true;
                        break 'outer;
                    }
                    let s = tracked(&cur, pr, t);
                    let Some(f) = para_fit(&s, pr) else { continue };
                    if f.lines.abs_diff(base.lines) != 1 || out_of_range(&f.ratios) > out_of_range(&base.ratios) {
                        continue;
                    }
                    let m = app.engine.begin_group();
                    app.run(Command::SetShapeKind { shape: host, kind: s.kind.clone() });
                    app.reflow_chain(host);
                    app.engine.end_group(m, "Fix Breaks");
                    if count_widows_orphans(app, &story, range) < count {
                        fixed.push(json!({"kind": kind, "page": page_number(app, l.page), "frame": host.0, "tracking": t, "text": l.text.trim()}));
                        solved = true;
                        break 'outer;
                    }
                    let _ = app.engine.undo();
                }
            }
            if !solved {
                unsolved.push(json!({"kind": kind, "page": page_number(app, l.page), "frame": l.frame.0, "text": l.text.trim(), "why": "no paragraph of the frame gains or loses a line within the limits without making more widows or orphans"}));
            }
            if timed_out {
                break;
            }
        }
    }
    app.engine.end_group(mark, "Fix Breaks");
    Ok(json!({
        "story": story.head.0,
        "fixed": fixed,
        "unsolved": unsolved,
        "unfinished": timed_out,
        "widows_and_orphans_left": count_widows_orphans(app, &story, range),
    }))
}

// ----- files -------------------------------------------------------------------

fn checkpoint(app: &mut App, args: &Value) -> Result<Value, String> {
    let dir = sub_dir("checkpoints")?;
    let note = str_arg(args, "note").map(slug).filter(|s| s != "sem-nome");
    let name = format!(
        "{}_{}{}.tdraw",
        slug(&app.doc().title),
        stamp(),
        note.map(|n| format!("_{n}")).unwrap_or_default()
    );
    let path = dir.join(name);
    save_copy(app, &path)?;
    Ok(json!({"checkpoint": path.display().to_string(), "the_open_drawing": "unchanged, not saved"}))
}

fn save_as(app: &mut App, args: &Value) -> Result<Value, String> {
    let name = str_arg(args, "name").filter(|n| !n.trim().is_empty()).ok_or("missing argument: name")?;
    let folder = match str_arg(args, "folder").filter(|f| !f.trim().is_empty()) {
        Some(f) => std::path::PathBuf::from(f),
        None => match app.file.as_ref().and_then(|f| f.parent()) {
            Some(p) => p.to_path_buf(),
            None => work_dir()?,
        },
    };
    let mut file = name.trim().to_string();
    if !file.to_lowercase().ends_with(".tdraw") {
        file.push_str(".tdraw");
    }
    let path = folder.join(file);
    if app.file.as_ref() == Some(&path) {
        return Err("that is the open file: give another name".into());
    }
    if path.exists() {
        return Err(format!("{} exists: never saved over", path.display()));
    }
    save_copy(app, &path)?;
    app.file = Some(path.clone());
    app.engine.mark_saved();
    Ok(json!({"saved": path.display().to_string(), "now_open": path.display().to_string()}))
}

fn all_ids(app: &App) -> std::collections::HashSet<ShapeId> {
    let mut out = std::collections::HashSet::new();
    fn walk(s: &Shape, out: &mut std::collections::HashSet<ShapeId>) {
        out.insert(s.id);
        if let ShapeKind::Group { children } = &s.kind {
            children.iter().for_each(|c| walk(c, out));
        }
    }
    for l in app.doc().all_layers() {
        for s in &l.shapes {
            walk(s, &mut out);
        }
    }
    out
}

fn run_script_batch(app: &mut App, args: &Value) -> Result<Value, String> {
    let steps = args.get("steps").and_then(|v| v.as_array()).ok_or("steps must be a list")?.clone();
    if steps.is_empty() || steps.len() > 100 {
        return Err("give 1 to 100 steps".into());
    }
    let stop = bool_arg(args, "stop_on_problem").unwrap_or(true);
    let saved = if bool_arg(args, "checkpoint").unwrap_or(true) {
        Some(checkpoint(app, &json!({"note": "before batch"}))?["checkpoint"].clone())
    } else {
        None
    };
    let mut out = Vec::new();
    let mut stopped = None;
    for (k, st) in steps.iter().enumerate() {
        let Some(src) = st.get("source").and_then(|s| s.as_str()) else {
            stopped = Some(format!("step {} has no source", k + 1));
            break;
        };
        let vba = st.get("model").and_then(|m| m.as_str()) == Some("vba");
        let before = all_ids(app);
        let r = crate::scripting::run_agent(app, src, vba, "AI agent");
        let mut row = json!({"step": k + 1, "description": st.get("description")});
        match r {
            Err(e) => {
                row["error"] = json!(format!("{e} (this step changed nothing)"));
                out.push(row);
                stopped = Some(format!("error in step {}", k + 1));
                break;
            }
            Ok(lines) => row["output"] = json!(lines.join("\n")),
        }
        let new: Vec<ShapeId> = all_ids(app).difference(&before).copied().collect();
        let mut problems = Vec::new();
        for id in &new {
            let Some(s) = app.doc().find_shape(*id) else { continue };
            let Some(pi) = app.page_index_of(*id) else { continue };
            let Some(p) = app.doc().pages.get(pi) else { continue };
            let b = app.visual_bounds(s);
            if b.x1 < 0.0 || b.y1 < 0.0 || b.x0 > p.size.width || b.y0 > p.size.height {
                problems.push(json!({"object": id.0, "problem": "off the page"}));
            }
            if matches!(s.kind, ShapeKind::Text { frame: Some(_), .. }) && app.next_frame(*id).is_none() && app.text_overflows(s) {
                problems.push(json!({"object": id.0, "problem": "text overflows"}));
            }
        }
        row["new_objects"] = json!(new.len());
        let bad = !problems.is_empty();
        row["problems"] = json!(problems);
        out.push(row);
        if bad && stop {
            stopped = Some(format!("problem after step {}", k + 1));
            break;
        }
    }
    Ok(json!({"checkpoint": saved, "steps": out, "stopped": stopped}))
}

fn script_library(args: &Value) -> Result<Value, String> {
    let dir = sub_dir("scripts")?;
    let action = str_arg(args, "action").ok_or("missing argument: action")?;
    let read_all = || -> Vec<Value> {
        let mut v: Vec<Value> = std::fs::read_dir(&dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
            .filter_map(|e| std::fs::read_to_string(e.path()).ok())
            .filter_map(|t| serde_json::from_str::<Value>(&t).ok())
            .collect();
        v.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
        v
    };
    let brief = |s: &Value| json!({"name": s["name"], "description": s["description"], "arguments": s["arguments"], "model": s["model"], "used_in": s["used_in"]});
    match action {
        "list" => Ok(json!({"scripts": read_all().iter().map(brief).collect::<Vec<_>>()})),
        "search" => {
            let q = str_arg(args, "search").or(str_arg(args, "name")).unwrap_or("").to_lowercase();
            let words: Vec<&str> = q.split_whitespace().collect();
            let hits: Vec<Value> = read_all()
                .iter()
                .filter(|s| {
                    let hay = format!("{} {}", s["name"].as_str().unwrap_or(""), s["description"].as_str().unwrap_or("")).to_lowercase();
                    words.iter().all(|w| hay.contains(w))
                })
                .map(brief)
                .collect();
            Ok(json!({"scripts": hits}))
        }
        "read" => {
            let name = str_arg(args, "name").ok_or("missing argument: name")?;
            read_all()
                .into_iter()
                .find(|s| s["name"].as_str().is_some_and(|n| n.eq_ignore_ascii_case(name)))
                .ok_or_else(|| format!("no script \"{name}\""))
        }
        "save" => {
            let name = str_arg(args, "name").filter(|n| !n.trim().is_empty()).ok_or("missing argument: name")?;
            let source = str_arg(args, "source").filter(|n| !n.trim().is_empty()).ok_or("missing argument: source")?;
            let desc = str_arg(args, "description").filter(|n| !n.trim().is_empty()).ok_or("missing argument: description")?;
            let path = dir.join(format!("{}.json", slug(name)));
            if path.exists() && !bool_arg(args, "replace").unwrap_or(false) {
                return Err(format!("a script named \"{name}\" exists; replace: true to replace it"));
            }
            let v = json!({
                "name": name.trim(),
                "description": desc,
                "arguments": str_arg(args, "arguments").unwrap_or(""),
                "model": str_arg(args, "model").unwrap_or("tracedraw"),
                "used_in": str_arg(args, "used_in").unwrap_or(""),
                "saved": stamp(),
                "source": source,
            });
            std::fs::write(&path, serde_json::to_string_pretty(&v).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
            Ok(json!({"saved": path.display().to_string()}))
        }
        other => Err(format!("unknown action: {other} (list, search, read, save)")),
    }
}

// ----- projects and briefings -----------------------------------------------

fn questions(kind: &str) -> Vec<(&'static str, &'static str)> {
    match kind {
        "book_interior" => vec![
            ("genre", "What is the book (fiction, essay, poetry, academic, children's) and who reads it?"),
            ("original", "Where is the author's text (the .docx) and are there images, tables or notes?"),
            ("format", "The format and page count wanted, or the house standard's?"),
            ("tone", "What should the page feel like: classic, contemporary, airy, dense?"),
            ("openings", "How do chapters open (drop of the title, number, epigraph, ornament)?"),
            ("extras", "Front and back matter: dedication, epigraph, contents, notes, index, colophon?"),
            ("deadline", "When does it go to print, and where?"),
        ],
        "book_cover" => vec![
            ("book", "Title, subtitle, author and the publisher's line: exactly as they go on the cover?"),
            ("reader", "Who should pick this book from the shelf?"),
            ("mood", "Three words for what the cover should feel like?"),
            ("references", "Covers you like (or hate) and why?"),
            ("image", "Is there an image, an illustrator, or should the cover be typographic?"),
            ("spine", "Page count and paper (for the spine), and the back cover text?"),
            ("deadline", "When and where is it printed?"),
        ],
        "graphic_piece" => vec![
            ("what", "What piece is it (poster, flyer, invitation, post) and where will it be seen?"),
            ("message", "The one thing someone must take from it in three seconds?"),
            ("texts", "All the texts, exactly, in order of importance?"),
            ("size", "Size and output (print, which printer; screen, which network)?"),
            ("mood", "Three words for the feeling, and the colours to use or avoid?"),
            ("brand", "Logos, fonts or rules of a brand to follow?"),
            ("deadline", "When is it needed?"),
        ],
        "illustration" => vec![
            ("subject", "What is drawn, and for what (book, cover, character, logo)?"),
            ("style", "The line: ink, loose cartoon, clean vector, texture?"),
            ("references", "Drawings you like (to learn from, never to copy)?"),
            ("colour", "In colour or black and white? Which colours?"),
            ("size", "Final size and where it is printed or shown?"),
            ("deadline", "When is it needed?"),
        ],
        _ => vec![
            ("what", "What is the job and who is it for?"),
            ("goal", "What has to be true when it is done?"),
            ("material", "What material is there already?"),
            ("size", "Size and output?"),
            ("deadline", "When is it needed?"),
        ],
    }
}

fn project_dir(name: &str) -> Result<std::path::PathBuf, String> {
    let d = sub_dir("projects")?.join(slug(name));
    std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    Ok(d)
}

fn start_project(args: &Value) -> Result<Value, String> {
    let name = str_arg(args, "name").filter(|n| !n.trim().is_empty()).ok_or("missing argument: name")?;
    let kind = str_arg(args, "kind").unwrap_or("other");
    let existing = sub_dir("projects")?.join(slug(name)).join("briefing.json");
    if let Ok(t) = std::fs::read_to_string(&existing) {
        if let Ok(v) = serde_json::from_str::<Value>(&t) {
            return Ok(json!({"project": name, "briefing": v, "note": "the project has a briefing: build on it, ask only what changed"}));
        }
    }
    Ok(json!({
        "project": name,
        "kind": kind,
        "questions": questions(kind).iter().map(|(id, q)| json!({"id": id, "question": q})).collect::<Vec<_>>(),
        "how": "Ask in Portuguese, one or two questions at a time, in your own words; skip what the person already said; do not ask what the house standard decides (margins, fonts of the book styles, hyphenation). Then show the direction in a few lines and save it with save_briefing.",
    }))
}

fn save_briefing(args: &Value) -> Result<Value, String> {
    let name = str_arg(args, "name").filter(|n| !n.trim().is_empty()).ok_or("missing argument: name")?;
    let direction = str_arg(args, "direction").ok_or("missing argument: direction")?;
    let dir = project_dir(name)?;
    let list = |k: &str| -> Vec<String> {
        args.get(k).and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|s| s.as_str().map(str::to_string)).collect()).unwrap_or_default()
    };
    let answers = args.get("answers").cloned().unwrap_or(json!([]));
    let v = json!({
        "name": name,
        "kind": str_arg(args, "kind").unwrap_or("other"),
        "saved": stamp(),
        "answers": answers,
        "direction": direction,
        "decisions": list("decisions"),
        "open": list("open"),
    });
    std::fs::write(dir.join("briefing.json"), serde_json::to_string_pretty(&v).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    let mut md = format!("# Briefing: {name}\n\n## Direção\n\n{direction}\n\n## Respostas\n\n");
    for a in answers.as_array().into_iter().flatten() {
        md.push_str(&format!("- **{}** {}\n", a["question"].as_str().unwrap_or(""), a["answer"].as_str().unwrap_or("")));
    }
    md.push_str("\n## Decisões\n\n");
    for d in list("decisions") {
        md.push_str(&format!("- {d}\n"));
    }
    md.push_str("\n## Em aberto\n\n");
    for d in list("open") {
        md.push_str(&format!("- {d}\n"));
    }
    std::fs::write(dir.join("briefing.md"), md).map_err(|e| e.to_string())?;
    Ok(json!({"saved": dir.join("briefing.json").display().to_string()}))
}

fn read_briefing(args: &Value) -> Result<Value, String> {
    let root = sub_dir("projects")?;
    if let Some(name) = str_arg(args, "name").filter(|n| !n.trim().is_empty()) {
        let p = root.join(slug(name)).join("briefing.json");
        let t = std::fs::read_to_string(&p).map_err(|_| format!("the project \"{name}\" has no briefing"))?;
        return serde_json::from_str(&t).map_err(|e| e.to_string());
    }
    let mut list: Vec<(std::time::SystemTime, String)> = std::fs::read_dir(&root)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let p = e.path().join("briefing.json");
            let t = std::fs::metadata(&p).and_then(|m| m.modified()).ok()?;
            let v: Value = serde_json::from_str(&std::fs::read_to_string(&p).ok()?).ok()?;
            Some((t, v["name"].as_str().unwrap_or("").to_string()))
        })
        .collect();
    list.sort_by(|a, b| b.0.cmp(&a.0));
    Ok(json!({"projects": list.into_iter().map(|x| x.1).collect::<Vec<_>>()}))
}

// ----- pictures ----------------------------------------------------------------

fn compare_snapshots(args: &Value) -> Result<Reply, String> {
    use base64::Engine as _;
    let a = str_arg(args, "before").ok_or("missing argument: before")?;
    let b = str_arg(args, "after").ok_or("missing argument: after")?;
    let load = |p: &str| image::open(p).map(|i| i.to_rgba8()).map_err(|e| format!("could not read {p}: {e}"));
    let (x, mut y) = (load(a)?, load(b)?);
    if x.dimensions() != y.dimensions() {
        return Err(format!(
            "the pictures differ in size ({}x{} and {}x{}): take both with the same area and size",
            x.width(),
            x.height(),
            y.width(),
            y.height()
        ));
    }
    let thr = usize_arg(args, "threshold").unwrap_or(48).clamp(4, 255) as i32;
    let (w, h) = x.dimensions();
    const CELL: u32 = 8;
    let (cw, ch) = (w.div_ceil(CELL), h.div_ceil(CELL));
    let mut cells = vec![false; (cw * ch) as usize];
    let mut changed = 0u64;
    for j in 0..h {
        for i in 0..w {
            let p = x.get_pixel(i, j).0;
            let q = y.get_pixel(i, j).0;
            let d = (0..3).map(|k| (p[k] as i32 - q[k] as i32).abs()).max().unwrap_or(0);
            if d >= thr {
                changed += 1;
                cells[((j / CELL) * cw + i / CELL) as usize] = true;
                let px = y.get_pixel_mut(i, j);
                px.0 = [
                    ((px.0[0] as u32 + 255 * 2) / 3) as u8,
                    (px.0[1] as u32 / 3) as u8,
                    (px.0[2] as u32 / 3) as u8,
                    255,
                ];
            }
        }
    }
    // Regions: cells that touch, as boxes.
    let mut seen = vec![false; cells.len()];
    let mut regions: Vec<(u32, u32, u32, u32)> = Vec::new();
    for start in 0..cells.len() {
        if !cells[start] || seen[start] {
            continue;
        }
        let mut stack = vec![start];
        seen[start] = true;
        let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
        while let Some(c) = stack.pop() {
            let (cx, cy) = (c as u32 % cw, c as u32 / cw);
            x0 = x0.min(cx);
            y0 = y0.min(cy);
            x1 = x1.max(cx);
            y1 = y1.max(cy);
            for (dx, dy) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                let (nx, ny) = (cx as i32 + dx, cy as i32 + dy);
                if nx < 0 || ny < 0 || nx >= cw as i32 || ny >= ch as i32 {
                    continue;
                }
                let n = (ny as u32 * cw + nx as u32) as usize;
                if cells[n] && !seen[n] {
                    seen[n] = true;
                    stack.push(n);
                }
            }
        }
        regions.push((x0 * CELL, y0 * CELL, ((x1 + 1) * CELL).min(w), ((y1 + 1) * CELL).min(h)));
    }
    regions.sort_by_key(|r| std::cmp::Reverse((r.2 - r.0) * (r.3 - r.1)));
    for &(x0, y0, x1, y1) in &regions {
        for i in x0..x1 {
            for j in [y0, y1.saturating_sub(1)] {
                y.put_pixel(i, j, image::Rgba([220, 0, 0, 255]));
            }
        }
        for j in y0..y1 {
            for i in [x0, x1.saturating_sub(1)] {
                y.put_pixel(i, j, image::Rgba([220, 0, 0, 255]));
            }
        }
    }
    let dpi = f64_arg(args, "dpi").filter(|d| *d >= 10.0);
    let mm = |px: u32| dpi.map(|d| round(px as f64 / d * 25.4));
    let list: Vec<Value> = regions
        .iter()
        .take(30)
        .map(|&(x0, y0, x1, y1)| {
            json!({"px": [x0, y0, x1, y1], "mm_from_top_left": dpi.map(|_| json!([mm(x0), mm(y0), mm(x1), mm(y1)]))})
        })
        .collect();
    let mut png = Vec::new();
    image::DynamicImage::ImageRgba8(y.clone())
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    if let Some(p) = str_arg(args, "save_to").filter(|p| !p.trim().is_empty()) {
        std::fs::write(p, &png).map_err(|e| format!("could not write {p}: {e}"))?;
    }
    let share = changed as f64 / (w as f64 * h as f64).max(1.0) * 100.0;
    Ok(Reply::Image {
        png_base64: base64::engine::general_purpose::STANDARD.encode(&png),
        text: json!({"changed_percent": (share * 100.0).round() / 100.0, "regions": regions.len(), "largest": list}).to_string(),
    })
}

/// The page area `rect` (mm) of `doc` drawn at `dpi`, on transparent.
pub(crate) fn render_rect(
    doc: &tracedraw_core::Document,
    page: tracedraw_core::PageId,
    rect: tracedraw_core::Rect,
    dpi: f64,
) -> Option<tiny_skia::Pixmap> {
    let zoom = dpi / 25.4;
    let w = (rect.width() * zoom).round().max(1.0) as u32;
    let h = (rect.height() * zoom).round().max(1.0) as u32;
    let opts = tracedraw_render::RenderOptions {
        width: w,
        height: h,
        view: tracedraw_render::ViewTransform {
            zoom,
            origin_x: -rect.x0 * zoom,
            origin_y: rect.y1 * zoom,
        },
        ..tracedraw_render::RenderOptions::default()
    };
    tracedraw_render::render_page(doc, page, &opts)
}

fn luminance(r: u8, g: u8, b: u8) -> f64 {
    let f = |c: u8| {
        let c = c as f64 / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b)
}

fn ratio(a: f64, b: f64) -> f64 {
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

/// A pixel of `pm` over white, straight RGB.
fn on_white(pm: &tiny_skia::Pixmap, x: u32, y: u32) -> Option<(u8, u8, u8)> {
    let p = pm.pixel(x, y)?;
    let a = p.alpha() as u32;
    let mix = |c: u8| (c as u32 + (255 - a)).min(255) as u8;
    Some((mix(p.red()), mix(p.green()), mix(p.blue())))
}

fn measure_contrast(app: &App, args: &Value) -> Result<Value, String> {
    let list = ids(args)?;
    let dpi = f64_arg(args, "dpi").unwrap_or(100.0).clamp(36.0, 300.0);
    let doc = app.doc();
    let page_idx = app.page_index_of(list[0]).ok_or("the object is not on a page")?;
    if list.iter().any(|id| app.page_index_of(*id) != Some(page_idx)) {
        return Err("the texts must be on one page".into());
    }
    let page = doc.pages[page_idx].id;
    let mut rect: Option<tracedraw_core::Rect> = None;
    for id in &list {
        let s = doc.find_shape(*id).ok_or_else(|| format!("no object with id {}", id.0))?;
        let b = app.visual_bounds(s);
        rect = Some(rect.map_or(b, |r| r.union(b)));
    }
    let rect = rect.ok_or("no texts")?;
    let shift = args.get("suggest_position");
    let max_mm = shift.and_then(|s| s.get("max_mm")).and_then(|v| v.as_f64()).unwrap_or(0.0).clamp(0.0, 200.0);
    let step_mm = shift.and_then(|s| s.get("step_mm")).and_then(|v| v.as_f64()).unwrap_or(5.0).clamp(0.5, 50.0);
    let wide = rect.inflate(max_mm, max_mm);
    // The drawing without the texts (what is behind them), and the texts
    // alone (where their letters are).
    let keep = |d: &mut tracedraw_core::Document, texts: bool| {
        for l in d.pages.iter_mut().flat_map(|p| p.layers.iter_mut()).chain(d.master.iter_mut()) {
            l.shapes.retain(|s| list.contains(&s.id) == texts);
        }
        if texts {
            for p in d.pages.iter_mut() {
                p.background = None;
            }
        }
    };
    let mut behind = doc.clone();
    keep(&mut behind, false);
    let mut alone = doc.clone();
    keep(&mut alone, true);
    let full = render_rect(doc, page, rect, dpi).ok_or("render failed")?;
    let back = render_rect(&behind, page, wide, dpi).ok_or("render failed")?;
    let mask = render_rect(&alone, page, rect, dpi).ok_or("render failed")?;
    let off = ((max_mm * dpi / 25.4).round()) as u32;
    let (w, h) = (mask.width(), mask.height());
    let mut values: Vec<(u32, f64)> = Vec::new();
    let mut letters: Vec<(u32, u32, f64)> = Vec::new();
    for y in 0..h {
        for x in 0..w {
            if mask.pixel(x, y).is_none_or(|p| p.alpha() < 160) {
                continue;
            }
            let Some((r, g, b)) = on_white(&full, x, y) else { continue };
            let Some((br, bg, bb)) = on_white(&back, x + off, y + off) else { continue };
            let lt = luminance(r, g, b);
            values.push((x, ratio(lt, luminance(br, bg, bb))));
            letters.push((x, y, lt));
        }
    }
    if values.is_empty() {
        return Err("no letters to measure (empty texts?)".into());
    }
    let mut all: Vec<f64> = values.iter().map(|v| v.1).collect();
    all.sort_by(|a, b| a.total_cmp(b));
    let at = |q: f64| all[((all.len() as f64 - 1.0) * q).round() as usize];
    let share = |lim: f64| (all.iter().filter(|v| **v < lim).count() as f64 / all.len() as f64 * 1000.0).round() / 10.0;
    let fifths: Vec<Value> = (0..5)
        .map(|k| {
            let (a, b) = (w * k / 5, w * (k + 1) / 5);
            let mut v: Vec<f64> = values.iter().filter(|(x, _)| *x >= a && *x < b).map(|v| v.1).collect();
            v.sort_by(|a, b| a.total_cmp(b));
            if v.is_empty() {
                Value::Null
            } else {
                json!(round(v[v.len() / 10]))
            }
        })
        .collect();
    let worst10 = at(0.1);
    let mut out = json!({
        "page": page_number(app, page_idx),
        "letter_pixels": all.len(),
        "median": round(at(0.5)),
        "worst_10_percent": round(worst10),
        "percent_under_3": share(3.0),
        "percent_under_4_5": share(4.5),
        "worst_10_percent_by_fifth_of_width": fifths,
        "warning": (at(0.5) >= 4.5 && worst10 < 3.0).then_some("the median is good but a stretch of the text is under 3:1"),
    });
    if max_mm > 0.0 {
        let step = ((step_mm * dpi / 25.4).round() as i64).max(1);
        let mut tries: Vec<(f64, f64, f64)> = Vec::new();
        let o = off as i64;
        let mut dy = -o;
        while dy <= o {
            let mut dx = -o;
            while dx <= o {
                let mut v: Vec<f64> = letters
                    .iter()
                    .filter_map(|&(x, y, lt)| {
                        let (bx, by) = (x as i64 + o + dx, y as i64 + o + dy);
                        if bx < 0 || by < 0 {
                            return None;
                        }
                        let (r, g, b) = on_white(&back, bx as u32, by as u32)?;
                        Some(ratio(lt, luminance(r, g, b)))
                    })
                    .collect();
                if v.len() * 10 >= letters.len() * 9 {
                    v.sort_by(|a, b| a.total_cmp(b));
                    let mm = 25.4 / dpi;
                    // Screen y grows down; the drawing's y grows up.
                    tries.push((v[v.len() / 10], dx as f64 * mm, -(dy as f64) * mm));
                }
                dx += step;
            }
            dy += step;
        }
        tries.sort_by(|a, b| b.0.total_cmp(&a.0));
        out["better_positions"] = json!(tries
            .iter()
            .take(5)
            .map(|t| json!({"move_x_mm": round(t.1), "move_y_mm": round(t.2), "worst_10_percent": round(t.0)}))
            .collect::<Vec<_>>());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp_prelo::tests::{frame, reply};

    #[test]
    fn a_new_book_gets_the_house_standard_and_its_text_runs_through() {
        let mut app = App::headless();
        let r = reply(&mut app, "create_document", json!({"width_mm": 140, "height_mm": 210, "pages": 2}));
        assert_eq!(r["pages"], 2);
        assert_eq!(r["facing_pages"], true);
        assert!(r["primary_frame"].is_u64());
        assert!(!app.engine.is_dirty());
        let style = r["paragraph_styles"][0].as_str().unwrap().to_string();
        let text = "Um parágrafo de texto corrido que enche a página. ".repeat(150);
        let p = reply(&mut app, "place_text", json!({"text": text, "style": style}));
        assert_eq!(p["overflows"], false);
        assert!(p["frames"].as_u64().unwrap() >= 3);
        // One undo takes it all back.
        app.engine.undo().unwrap();
        assert_eq!(app.doc().pages.len(), 2);
    }

    #[test]
    fn frames_thread_and_fit_their_text() {
        let mut app = App::headless();
        let a = frame(&mut app, &"Texto que passa da moldura. ".repeat(30), 60.0, 20.0, TextAlign::Left);
        let b = frame(&mut app, "", 60.0, 80.0, TextAlign::Left);
        let r = reply(&mut app, "thread_frames", json!({"ids": [a.0, b.0]}));
        assert_eq!(r["frames"].as_array().unwrap().len(), 2);
        let c = frame(&mut app, "Legenda curta.", 60.0, 50.0, TextAlign::Left);
        let f = reply(&mut app, "fit_frame", json!({"ids": [c.0]}));
        let after = f["frames"][0]["height_mm"]["after"].as_f64().unwrap();
        assert!(after < 15.0 && after > 2.0, "{after}");
        assert!(fits(app.doc().find_shape(c).unwrap()));
        let linked = reply(&mut app, "fit_frame", json!({"ids": [a.0]}));
        assert!(linked["frames"][0]["skipped"].is_string());
    }

    #[test]
    fn clearing_local_formatting_keeps_the_authors_marks() {
        let mut app = App::headless();
        app.add_book_styles();
        let id = frame(&mut app, "x", 80.0, 30.0, TextAlign::Left);
        let style = app.doc().text_styles.iter().find(|s| s.para.is_some()).map(|s| s.id).unwrap();
        let mut kind = app.doc().find_shape(id).unwrap().kind.clone();
        let styles = app.doc().text_styles.clone();
        tracedraw_core::text_styles::apply_paragraph_style(&mut kind, None, &styles, Some(style));
        if let ShapeKind::Text { spans, .. } = &mut kind {
            let t = spans[0].clone();
            let mut a = t.clone();
            a.text = "Um ".into();
            let mut b = t.clone();
            b.text = "grifo".into();
            b.italic = true;
            b.size_pt += 4.0;
            let mut c = t.clone();
            c.text = " no meio.".into();
            c.size_pt += 4.0;
            *spans = vec![a, b, c];
        }
        app.run(Command::SetShapeKind { shape: id, kind });
        let r = reply(&mut app, "clear_local_formatting", json!({}));
        assert_eq!(r["runs_cleared"], 2);
        let ShapeKind::Text { spans, .. } = &app.doc().find_shape(id).unwrap().kind else { panic!() };
        let italic: String = spans.iter().filter(|s| s.italic).map(|s| s.text.as_str()).collect();
        assert_eq!(italic, "grifo");
        assert!(spans.iter().all(|s| (s.size_pt - spans[0].size_pt).abs() < 1e-9));
    }

    #[test]
    fn typography_is_fixed_without_touching_words() {
        let mut app = App::headless();
        let id = frame(&mut app, "Ele disse \"oi\"  - e foi.", 80.0, 30.0, TextAlign::Left);
        let r = reply(&mut app, "fix_typography", json!({}));
        assert!(r["changes"].as_u64().unwrap() >= 2);
        let ShapeKind::Text { spans, .. } = &app.doc().find_shape(id).unwrap().kind else { panic!() };
        let t: String = spans.iter().map(|s| s.text.as_str()).collect();
        assert!(!t.contains('"') && !t.contains("  "), "{t}");
    }

    #[test]
    fn justification_fixes_keep_the_pagination() {
        let mut app = App::headless();
        let text = "Paralelepípedo inconstitucionalíssimo e outras palavras compridas demais atravessam a coluna estreita. ".repeat(10);
        let id = frame(&mut app, &text, 45.0, 200.0, TextAlign::Justify);
        let fitted = app.text_layout_of(id).unwrap().0.fitted_chars;
        let r = reply(&mut app, "fix_justification", json!({}));
        let (b, a) = (&r["before"], &r["after"]);
        let sum = |v: &Value| v["loose"].as_u64().unwrap() + v["tight"].as_u64().unwrap();
        assert!(sum(a) <= sum(b));
        assert_eq!(app.text_layout_of(id).unwrap().0.fitted_chars, fitted);
        let br = reply(&mut app, "fix_breaks", json!({"kinds": ["single_word"]}));
        assert!(br["fixed"].is_array() && br["unsolved"].is_array());
    }

    #[test]
    fn files_go_to_the_work_folder_and_never_over_a_file() {
        let mut app = App::headless();
        frame(&mut app, "Texto.", 60.0, 20.0, TextAlign::Left);
        let c = reply(&mut app, "checkpoint", json!({"note": "antes do ajuste"}));
        let path = std::path::PathBuf::from(c["checkpoint"].as_str().unwrap());
        assert!(path.exists());
        assert!(app.file.is_none());
        let dir = work_dir().unwrap();
        let s = reply(&mut app, "save_as", json!({"name": "Livro - Prelo", "folder": dir.display().to_string()}));
        assert!(s["saved"].as_str().unwrap().ends_with("Livro - Prelo.tdraw"));
        let again = call(&mut app, "save_as", &json!({"name": "Livro - Prelo", "folder": dir.display().to_string()}));
        assert!(matches!(again, Some(Err(_))));
        assert_eq!(stamp().len(), 19);
        assert_eq!(slug("Queda de abertura: GREP!"), "queda-de-abertura-grep");
    }

    #[test]
    fn scripts_are_kept_and_found_and_batches_stop_at_errors() {
        let mut app = App::headless();
        let name = format!("Retângulo de teste {}", stamp());
        reply(&mut app, "script_library", json!({"action": "save", "name": name, "description": "Desenha um retângulo de prova", "source": "ActiveLayer.CreateRectangle(10, 10, 20, 20, 0);"}));
        let s = reply(&mut app, "script_library", json!({"action": "search", "search": "retângulo prova"}));
        assert!(!s["scripts"].as_array().unwrap().is_empty());
        let r = reply(&mut app, "run_script_batch", json!({"checkpoint": false, "steps": [
            {"source": "ActiveLayer.CreateRectangle(10, 10, 20, 20, 0);", "description": "um"},
            {"source": "throw new Error('falhou');", "description": "dois"},
            {"source": "ActiveLayer.CreateRectangle(40, 10, 20, 20, 0);", "description": "três"}
        ]}));
        assert_eq!(r["steps"].as_array().unwrap().len(), 2);
        assert!(r["stopped"].is_string());
        let p = reply(&mut app, "start_project", json!({"name": "Livro Teste", "kind": "book_interior"}));
        assert!(p["questions"].as_array().unwrap().len() <= 7);
        reply(&mut app, "save_briefing", json!({"name": "Livro Teste", "kind": "book_interior", "direction": "Clássico e arejado.", "answers": [{"id": "tone", "question": "Clima?", "answer": "clássico"}]}));
        let b = reply(&mut app, "read_briefing", json!({"name": "Livro Teste"}));
        assert_eq!(b["direction"], "Clássico e arejado.");
    }

    #[test]
    fn contrast_is_measured_on_what_is_behind_the_letters() {
        let mut app = App::headless();
        let id = app
            .new_shape(ShapeKind::Text {
                spans: vec![TextSpan::new("CONTRASTE", "DejaVu Sans", 40.0)],
                origin: tracedraw_core::Point::new(20.0, 100.0),
                frame: None,
                align: TextAlign::Left,
                para: Default::default(),
                on_path: None,
            })
            .unwrap();
        let r = reply(&mut app, "measure_contrast", json!({"ids": [id.0], "dpi": 72, "suggest_position": {"max_mm": 4, "step_mm": 2}}));
        // Black letters on white paper.
        assert!(r["median"].as_f64().unwrap() > 15.0, "{r}");
        assert!(!r["better_positions"].as_array().unwrap().is_empty());
    }

    #[test]
    fn snapshots_compare_by_pixel() {
        let dir = work_dir().unwrap();
        let mut a = image::RgbaImage::from_pixel(64, 64, image::Rgba([255, 255, 255, 255]));
        let pa = dir.join("a.png");
        a.save(&pa).unwrap();
        for x in 10..20 {
            for y in 10..20 {
                a.put_pixel(x, y, image::Rgba([0, 0, 0, 255]));
            }
        }
        let pb = dir.join("b.png");
        a.save(&pb).unwrap();
        let r = compare_snapshots(&json!({"before": pa.display().to_string(), "after": pb.display().to_string(), "dpi": 72})).unwrap();
        let Reply::Image { text, .. } = r else { panic!() };
        let v: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(v["regions"], 1);
        assert!(v["changed_percent"].as_f64().unwrap() > 2.0);
    }
}
