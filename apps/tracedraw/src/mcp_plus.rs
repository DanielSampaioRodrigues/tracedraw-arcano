//! The Arcano edition's MCP tools (see docs/behavior/mcp.md, "Arcano
//! edition"): an agent works beside the user in the open window, so it
//! needs to know which drawing it has, what the user selected and sees,
//! to look at parts of the page closely, to measure fonts, and to read the
//! object model it scripts. Kept apart from `mcp.rs` so that upstream work
//! on the server merges cleanly; `mcp.rs` asks [`call`] first.

use crate::app::App;
use crate::describe::shape_json;
use serde_json::{json, Value};
use tracedraw_core::{geometry::Rect, PageId, ShapeId};

/// What a tool answers: text, or a picture with a line about it.
pub enum Reply {
    Text(String),
    Image { png_base64: String, text: String },
}

fn tool(name: &str, description: &str, props: Value, required: &[&str]) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": {"type": "object", "properties": props, "required": required},
    })
}

/// The edition's tools, and its versions of `run_script` and
/// `document_info`, which replace upstream's in the list.
pub fn tools() -> Vec<Value> {
    vec![
        tool(
            "run_script",
            "Run a JavaScript script on the drawing the user has open, as one undo step; returns what it printed (print or console.log). All or nothing: if the script throws, nothing it did is kept. model \"tracedraw\" (default): TraceDraw's model, lengths in millimetres, origin at the page's lower-left corner, y up; CreateRectangle(x, y, w, h, radius), CreateEllipse(x, y, w, h) and CreateArtisticText(x, y, text, font, size_pt) take the lower-left corner (text: the baseline start); colours with Color.RGB(r, g, b), Color.CMYK(c, m, y, k) (0 to 100); Fill.ApplyUniformFill(color). model \"vba\": the object model VBA macros use (ActiveDocument.Unit, cdr* constants, Fill.UniformColor.RGBAssign...). Read api_reference before a first script.",
            json!({"source": {"type": "string"}, "model": {"type": "string", "enum": ["tracedraw", "vba"]}}),
            &["source"],
        ),
        tool(
            "document_info",
            "Describe the drawing the user has open: which window and drawing this is, the open drawings (tabs), the active page and layer, the selection, what part of the page the user is looking at, and every object (id, type, name, bounds in millimetres with y up, fill, outline, text and font). page_index limits the objects to one page.",
            json!({"page_index": {"type": "integer"}}),
            &[],
        ),
        tool(
            "api_reference",
            "The scripting object model, exactly as run_script defines it: model \"tracedraw\" (default) or \"vba\" (the macro model).",
            json!({"model": {"type": "string", "enum": ["tracedraw", "vba"]}}),
            &[],
        ),
        tool(
            "get_selection",
            "What the user has selected in the open drawing: each object in full (id, type, name, bounds in mm, fill, outline, text, font, children) and the bounds of the whole selection.",
            json!({}),
            &[],
        ),
        tool(
            "select",
            "Select objects by id (from document_info) in the open drawing, as a click would; add keeps the current selection. An empty list clears it.",
            json!({"ids": {"type": "array", "items": {"type": "integer"}}, "add": {"type": "boolean"}}),
            &["ids"],
        ),
        tool(
            "render_area",
            "Look closely at part of a page as a PNG on white. area: \"selection\" (default when something is selected: the selected objects), \"view\" (what the user sees in the window), \"page\", or give ids (objects) or rect_mm {x0, y0, x1, y1}. padding_mm around objects (default 3); max_px on the longer side (default 1024, 128 to 2048). Answers the area and its resolution.",
            json!({
                "area": {"type": "string", "enum": ["selection", "view", "page"]},
                "ids": {"type": "array", "items": {"type": "integer"}},
                "rect_mm": {"type": "object", "properties": {"x0": {"type": "number"}, "y0": {"type": "number"}, "x1": {"type": "number"}, "y1": {"type": "number"}}},
                "page_index": {"type": "integer"},
                "padding_mm": {"type": "number"},
                "max_px": {"type": "integer"}
            }),
            &[],
        ),
        tool(
            "font_metrics",
            "A font's real measurements at a size, in millimetres: ascent, descent, line height, cap height and x-height, and, for text, its advance width and the ink bounds of its letters (relative to the baseline start). Use it before sizing or fitting text; never estimate. Fails when the font is not installed (rather than measuring a substitute).",
            json!({
                "family": {"type": "string"},
                "size_pt": {"type": "number"},
                "bold": {"type": "boolean"},
                "italic": {"type": "boolean"},
                "text": {"type": "string"}
            }),
            &["family"],
        ),
        tool(
            "list_fonts",
            "The font families installed on this computer (and those the open drawings carry), each with its faces; filter keeps the names that contain it (any case). At most 200.",
            json!({"filter": {"type": "string"}}),
            &[],
        ),
        tool(
            "switch_document",
            "Bring one of the open drawings (index from document_info's open_documents) to the front in the window; the tools then work on it.",
            json!({"index": {"type": "integer"}}),
            &["index"],
        ),
        tool(
            "window_info",
            "Which TraceDraw window answers: its process id, the drawing in front and the open drawings.",
            json!({}),
            &[],
        ),
    ]
}

/// The edition's answer for `name`, or `None` for the tools upstream
/// answers.
pub fn call(app: &mut App, name: &str, args: &Value) -> Option<Result<Reply, String>> {
    let text = |r: Result<String, String>| Some(r.map(Reply::Text));
    match name {
        "run_script" => text(run_script(app, args)),
        "document_info" => text(Ok(document_info(app, args).to_string())),
        "api_reference" => text(api_reference(args)),
        "get_selection" => text(Ok(selection_json(app).to_string())),
        "select" => text(select(app, args)),
        "render_area" => Some(render_area(app, args)),
        "font_metrics" => text(font_metrics(args)),
        "list_fonts" => text(Ok(list_fonts(args).to_string())),
        "switch_document" => text(switch_document(app, args)),
        "window_info" => text(Ok(window_info(app).to_string())),
        _ => None,
    }
}

/// A line naming the window and drawing an answer came from, so an agent
/// always knows where it works.
pub fn whereabouts(app: &App) -> String {
    format!(
        "[TraceDraw window {} · drawing \"{}\" · page {} of {}]",
        std::process::id(),
        app.engine.document().title,
        app.page_index() + 1,
        app.engine.document().pages.len()
    )
}

fn model_is_vba(args: &Value) -> Result<bool, String> {
    match args
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or("tracedraw")
    {
        "tracedraw" | "" => Ok(false),
        "vba" => Ok(true),
        other => Err(format!("unknown model: {other} (tracedraw or vba)")),
    }
}

fn run_script(app: &mut App, args: &Value) -> Result<String, String> {
    let src = args
        .get("source")
        .and_then(|v| v.as_str())
        .ok_or("missing argument: source")?;
    let vg = model_is_vba(args)?;
    let out = crate::scripting::run_agent(app, src, vg, "AI agent").map_err(|e| {
        format!("{e}\n(nothing was changed: a failed script keeps none of its changes)")
    })?;
    Ok(if out.is_empty() {
        "ok".to_string()
    } else {
        out.join("\n")
    })
}

fn api_reference(args: &Value) -> Result<String, String> {
    Ok(if model_is_vba(args)? {
        format!("{}\n\n{}", VBA_NOTES, vba_reference())
    } else {
        format!(
            "{}\n\n{}",
            TRACEDRAW_NOTES,
            crate::scripting::object_model_source()
        )
    })
}

const TRACEDRAW_NOTES: &str = "TraceDraw object model (run_script model \"tracedraw\"). Lengths in millimetres; the page's lower-left corner is (0, 0) and y grows upwards. Creation methods take the lower-left corner and the size; artistic text takes the start of its baseline and the size in points. Colours: Color.RGB(0-255...), Color.CMYK(0-100...), Color.Gray(0-100). print() or console.log() return text. Globals: Application, ActiveDocument, ActivePage, ActiveLayer, ActiveSelection. The model, as defined:";

const VBA_NOTES: &str = "The macro object model (run_script model \"vba\"): the members VBA macros use, in JavaScript. Set ActiveDocument.Unit = cdrMillimeter first (the default unit is the document's); Items are 1-based; colours with Fill.UniformColor.RGBAssign(r, g, b) or CMYKAssign. Its reference:";

/// The object model section of the macro reference (docs/behavior/vba.md).
fn vba_reference() -> &'static str {
    const DOC: &str = include_str!("../../../docs/behavior/vba.md");
    let start = DOC
        .find("## Object model")
        .or_else(|| DOC.find("### Application"))
        .unwrap_or(0);
    let end = DOC[start..]
        .find("### Coverage from published macros")
        .map(|e| start + e)
        .unwrap_or(DOC.len());
    &DOC[start..end]
}

fn round(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

fn rect_json(r: Rect) -> Value {
    json!({"x0": round(r.x0), "y0": round(r.y0), "x1": round(r.x1), "y1": round(r.y1)})
}

/// The open drawings (tabs) of the window.
fn open_documents(app: &App) -> Vec<Value> {
    if app.docs.is_empty() {
        return vec![json!({"index": 0, "title": app.engine.document().title, "active": true})];
    }
    (0..app.docs.len())
        .map(|i| {
            json!({
                "index": i,
                "title": app.document_tab_title(i),
                "active": i == app.active_doc,
            })
        })
        .collect()
}

/// The part of the page the user sees, when there is a window.
fn view_rect(app: &App) -> Option<Rect> {
    let r = app.canvas_rect;
    (r.is_finite() && r.width() > 1.0 && r.height() > 1.0).then(|| app.view.visible_page_rect(r))
}

fn document_info(app: &App, args: &Value) -> Value {
    let mut v = crate::describe::document(app);
    let doc = app.engine.document();
    if let Some(i) = args.get("page_index").and_then(|p| p.as_u64()) {
        if let Some(pages) = v["pages"].as_array_mut() {
            let keep = pages.get(i as usize).cloned();
            *pages = keep.into_iter().collect();
        }
    }
    v["window"] = json!(std::process::id());
    v["open_documents"] = json!(open_documents(app));
    v["active_page_index"] = json!(app.page_index());
    v["active_layer"] = json!(app.active_layer().and_then(|id| {
        doc.pages
            .iter()
            .flat_map(|p| p.layers.iter())
            .find(|l| l.id == id)
            .map(|l| json!({"id": id.0, "name": l.name}))
    }));
    if let Some(r) = view_rect(app) {
        v["view"] = json!({"visible_mm": rect_json(r), "zoom_percent": (app.view.zoom as f64 * 100.0 / (96.0 / 25.4)).round()});
    }
    v["coordinates"] = json!("millimetres, page lower-left corner (0, 0), y up");
    v
}

fn selection_json(app: &App) -> Value {
    let doc = app.engine.document();
    let objects: Vec<Value> = app
        .selection
        .iter()
        .filter_map(|id| doc.find_shape(*id))
        .map(shape_json)
        .collect();
    json!({
        "count": objects.len(),
        "objects": objects,
        "bounds_mm": app.selection_bounds().map(rect_json),
    })
}

fn ids_arg(args: &Value) -> Option<Vec<ShapeId>> {
    args.get("ids")?
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_u64()).map(ShapeId).collect())
}

fn select(app: &mut App, args: &Value) -> Result<String, String> {
    let ids = ids_arg(args).ok_or("ids must be a list of object ids")?;
    let doc = app.engine.document();
    if let Some(bad) = ids.iter().find(|id| doc.find_shape(**id).is_none()) {
        return Err(format!("no object with id {}", bad.0));
    }
    // Objects on another page: that page comes to the front.
    if let Some(page) = ids.first().and_then(|id| page_of(app, *id)) {
        if page != app.page {
            if ids.iter().any(|id| page_of(app, *id) != Some(page)) {
                return Err("the objects are on different pages".into());
            }
            app.page = page;
        }
    }
    let mut all = if args.get("add").and_then(|a| a.as_bool()).unwrap_or(false) {
        app.selection.clone()
    } else {
        Vec::new()
    };
    for id in ids {
        if !all.contains(&id) {
            all.push(id);
        }
    }
    app.select(all);
    Ok(selection_json(app).to_string())
}

/// The page an object is on.
fn page_of(app: &App, id: ShapeId) -> Option<PageId> {
    fn holds(s: &tracedraw_core::document::Shape, id: ShapeId) -> bool {
        use tracedraw_core::document::ShapeKind;
        s.id == id
            || match &s.kind {
                ShapeKind::Group { children } => children.iter().any(|c| holds(c, id)),
                ShapeKind::ClipFrame { frame, contents } => {
                    holds(frame, id) || contents.iter().any(|c| holds(c, id))
                }
                _ => false,
            }
    }
    app.engine
        .document()
        .pages
        .iter()
        .find(|p| {
            p.layers
                .iter()
                .any(|l| l.shapes.iter().any(|s| holds(s, id)))
        })
        .map(|p| p.id)
}

fn render_area(app: &App, args: &Value) -> Result<Reply, String> {
    use base64::Engine as _;
    let doc = app.engine.document();
    let max_px = args
        .get("max_px")
        .and_then(|v| v.as_u64())
        .unwrap_or(1024)
        .clamp(128, 2048) as f64;
    let padding = args
        .get("padding_mm")
        .and_then(|v| v.as_f64())
        .unwrap_or(3.0)
        .clamp(0.0, 1000.0);
    let mut page = match args.get("page_index").and_then(|v| v.as_u64()) {
        Some(i) => doc
            .pages
            .get(i as usize)
            .map(|p| p.id)
            .ok_or_else(|| format!("page_index {i} out of range"))?,
        None => app.page,
    };
    let objects_rect = |ids: &[ShapeId]| -> Result<(Rect, Option<PageId>), String> {
        let mut r: Option<Rect> = None;
        for id in ids {
            let s = doc
                .find_shape(*id)
                .ok_or_else(|| format!("no object with id {}", id.0))?;
            let b = app.visual_bounds(s);
            r = Some(r.map_or(b, |a| a.union(b)));
        }
        let r = r.ok_or("no objects to look at")?;
        Ok((
            r.inflate(padding, padding),
            ids.first().and_then(|id| page_of(app, *id)),
        ))
    };
    let page_rect = |p: PageId| -> Result<Rect, String> {
        let p = doc.page(p).map_err(|e| e.to_string())?;
        Ok(Rect::new(0.0, 0.0, p.size.width, p.size.height))
    };
    let area = args.get("area").and_then(|a| a.as_str());
    let (rect, what) = if let Some(ids) = ids_arg(args).filter(|i| !i.is_empty()) {
        let (r, p) = objects_rect(&ids)?;
        if let Some(p) = p {
            page = p;
        }
        (r, "objects".to_string())
    } else if let Some(r) = args.get("rect_mm") {
        let n = |k: &str| {
            r.get(k)
                .and_then(|v| v.as_f64())
                .ok_or(format!("rect_mm needs {k}"))
        };
        let rect = Rect::new(n("x0")?, n("y0")?, n("x1")?, n("y1")?);
        (
            Rect::new(
                rect.x0.min(rect.x1),
                rect.y0.min(rect.y1),
                rect.x0.max(rect.x1),
                rect.y0.max(rect.y1),
            ),
            "rectangle".into(),
        )
    } else {
        match area.unwrap_or(if app.selection.is_empty() {
            "page"
        } else {
            "selection"
        }) {
            "selection" => {
                if app.selection.is_empty() {
                    return Err("nothing is selected".into());
                }
                let (r, p) = objects_rect(&app.selection.clone())?;
                if let Some(p) = p {
                    page = p;
                }
                (r, "selection".into())
            }
            "view" => (
                view_rect(app).ok_or("no window view to look at")?,
                "view".into(),
            ),
            "page" => (page_rect(page)?, "page".into()),
            other => return Err(format!("unknown area: {other}")),
        }
    };
    let (w_mm, h_mm) = (rect.width(), rect.height());
    if !(w_mm > 0.0 && h_mm > 0.0) {
        return Err("the area is empty".into());
    }
    let zoom = max_px / w_mm.max(h_mm); // pixels per mm
    let w = (w_mm * zoom).round().max(1.0) as u32;
    let h = (h_mm * zoom).round().max(1.0) as u32;
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
    let pm = tracedraw_render::render_page(doc, page, &opts).ok_or("render failed")?;
    let mut white = tiny_skia::Pixmap::new(pm.width(), pm.height()).ok_or("render failed")?;
    white.fill(tiny_skia::Color::WHITE);
    white.draw_pixmap(
        0,
        0,
        pm.as_ref(),
        &Default::default(),
        tiny_skia::Transform::identity(),
        None,
    );
    let png = white.encode_png().map_err(|e| e.to_string())?;
    let index = doc.pages.iter().position(|p| p.id == page).unwrap_or(0);
    Ok(Reply::Image {
        png_base64: base64::engine::general_purpose::STANDARD.encode(png),
        text: format!(
            "{what} on page {}: x {:.2} to {:.2} mm, y {:.2} to {:.2} mm ({:.2} x {:.2} mm), {} x {} px ({:.0} dpi)",
            index + 1,
            rect.x0,
            rect.x1,
            rect.y0,
            rect.y1,
            w_mm,
            h_mm,
            w,
            h,
            zoom * 25.4
        ),
    })
}

fn font_metrics(args: &Value) -> Result<String, String> {
    let family = args
        .get("family")
        .and_then(|f| f.as_str())
        .filter(|f| !f.trim().is_empty())
        .ok_or("missing argument: family")?;
    let size = args.get("size_pt").and_then(|s| s.as_f64()).unwrap_or(12.0);
    if !(size > 0.0 && size <= 5000.0) {
        return Err("size_pt must be above 0 and at most 5000".into());
    }
    let bold = args.get("bold").and_then(|b| b.as_bool()).unwrap_or(false);
    let italic = args
        .get("italic")
        .and_then(|b| b.as_bool())
        .unwrap_or(false);
    let fonts = tracedraw_text::fonts();
    let m = fonts
        .face_metrics(family, bold, italic, size)
        .ok_or_else(|| {
            let near: Vec<String> = fonts
                .families()
                .into_iter()
                .filter(|f| f.to_lowercase().contains(&family.to_lowercase()))
                .take(10)
                .collect();
            if near.is_empty() {
                format!("the font \"{family}\" is not installed")
            } else {
                format!(
                    "the font \"{family}\" is not installed; similar names: {}",
                    near.join(", ")
                )
            }
        })?;
    let mut v = json!({
        "family": m.family,
        "size_pt": size,
        "bold": bold,
        "italic": italic,
        "units_per_em": m.units_per_em,
        "ascent_mm": round(m.ascent),
        "descent_mm": round(m.descent),
        "line_height_mm": round(m.line_height),
        "cap_height_mm": m.cap_height.map(round),
        "x_height_mm": m.x_height.map(round),
    });
    if let Some(t) = args
        .get("text")
        .and_then(|t| t.as_str())
        .filter(|t| !t.is_empty())
    {
        let mut span = tracedraw_core::TextSpan::new(t.to_string(), family.to_string(), size);
        span.bold = bold;
        span.italic = italic;
        let layout = fonts.outline(std::slice::from_ref(&span));
        use kurbo::Shape as _;
        let ink = layout.path.bounding_box();
        v["text"] = json!(t);
        v["advance_width_mm"] = json!(round(layout.bounds.width()));
        if layout.glyphs > 0 && ink.width().is_finite() {
            v["ink_mm"] = json!({"x0": round(ink.x0), "x1": round(ink.x1), "y0": round(ink.y0), "y1": round(ink.y1),
                                  "width": round(ink.width()), "height": round(ink.height())});
        }
        v["note"] = json!("ink_mm is relative to the start of the baseline, y up");
    }
    Ok(v.to_string())
}

fn list_fonts(args: &Value) -> Value {
    let fonts = tracedraw_text::fonts();
    let filter = args
        .get("filter")
        .and_then(|f| f.as_str())
        .unwrap_or("")
        .to_lowercase();
    let all: Vec<String> = fonts
        .families()
        .into_iter()
        .filter(|f| filter.is_empty() || f.to_lowercase().contains(&filter))
        .collect();
    let total = all.len();
    let list: Vec<Value> = all
        .into_iter()
        .take(200)
        .map(|f| {
            let faces: Vec<String> = fonts
                .family_info(&f)
                .map(|i| i.faces.into_iter().map(|s| s.name).collect())
                .unwrap_or_default();
            json!({"family": f, "faces": faces})
        })
        .collect();
    json!({"count": total, "shown": list.len(), "families": list})
}

fn switch_document(app: &mut App, args: &Value) -> Result<String, String> {
    let i = args
        .get("index")
        .and_then(|v| v.as_u64())
        .ok_or("missing argument: index")? as usize;
    if app.docs.is_empty() {
        return if i == 0 {
            Ok(window_info(app).to_string())
        } else {
            Err(format!("index {i} out of range"))
        };
    }
    if i >= app.docs.len() {
        return Err(format!(
            "index {i} out of range (0 to {})",
            app.docs.len() - 1
        ));
    }
    app.switch_document(i);
    Ok(window_info(app).to_string())
}

fn window_info(app: &App) -> Value {
    json!({
        "window": std::process::id(),
        "drawing": app.engine.document().title,
        "file": app.file.as_ref().map(|p| p.display().to_string()),
        "open_documents": open_documents(app),
        "objects_on_page": app.engine.document().page(app.page).map(|p| p.layers.iter().map(|l| l.shapes.len()).sum::<usize>()).unwrap_or(0),
        "selected": app.selection.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(r: Option<Result<Reply, String>>) -> Result<String, String> {
        match r.expect("an edition tool") {
            Ok(Reply::Text(t)) => Ok(t),
            Ok(Reply::Image { text, .. }) => Ok(text),
            Err(e) => Err(e),
        }
    }

    fn draw(app: &mut App, src: &str) -> String {
        text(call(app, "run_script", &json!({"source": src}))).expect("script runs")
    }

    #[test]
    fn a_failed_script_leaves_nothing_behind() {
        let mut app = App::headless();
        let before = app.engine.document().pages[0].layers[0].shapes.len();
        let r = text(call(
            &mut app,
            "run_script",
            &json!({"source": "ActiveLayer.CreateEllipse(10, 10, 20, 20); nope();"}),
        ));
        assert!(r.is_err_and(|e| e.contains("nothing was changed")));
        assert_eq!(
            app.engine.document().pages[0].layers[0].shapes.len(),
            before
        );
        // A good one is kept, as one step.
        draw(&mut app, "ActiveLayer.CreateEllipse(10, 10, 20, 20); ActiveLayer.CreateRectangle(40, 10, 20, 20);");
        assert_eq!(
            app.engine.document().pages[0].layers[0].shapes.len(),
            before + 2
        );
        app.undo();
        assert_eq!(
            app.engine.document().pages[0].layers[0].shapes.len(),
            before
        );
    }

    #[test]
    fn scripts_in_the_macro_model() {
        let mut app = App::headless();
        let out = text(call(
            &mut app,
            "run_script",
            &json!({"model": "vba", "source":
                "ActiveDocument.Unit = cdrMillimeter; const s = ActiveLayer.CreateEllipse2(50, 50, 20, 20); s.Fill.UniformColor.RGBAssign(255, 0, 0); print(s.Type == cdrEllipseShape);"}),
        ))
        .expect("runs");
        assert_eq!(out, "true");
        let info: Value =
            serde_json::from_str(&text(call(&mut app, "document_info", &json!({}))).unwrap())
                .unwrap();
        let o = &info["pages"][0]["layers"][0]["objects"][0];
        assert_eq!(o["fill"]["solid"], "R:255 G:0 B:0");
        assert_eq!(o["bounds_mm"]["x0"], 30.0);
        assert!(text(call(
            &mut app,
            "run_script",
            &json!({"source": "1", "model": "basic"})
        ))
        .is_err());
    }

    #[test]
    fn the_reference_names_the_models() {
        let td = text(call(&mut App::headless(), "api_reference", &json!({}))).unwrap();
        assert!(td.contains("CreateEllipse(x, y, w, h)") && td.contains("lower-left"));
        let vba = text(call(
            &mut App::headless(),
            "api_reference",
            &json!({"model": "vba"}),
        ))
        .unwrap();
        assert!(vba.contains("CreateEllipse2") && vba.contains("RGBAssign"));
    }

    #[test]
    fn selection_is_read_and_set() {
        let mut app = App::headless();
        draw(&mut app, "ActiveLayer.CreateRectangle(10, 10, 20, 20).Name = 'a'; ActiveLayer.CreateRectangle(50, 10, 20, 20).Name = 'b';");
        let info: Value =
            serde_json::from_str(&text(call(&mut app, "document_info", &json!({}))).unwrap())
                .unwrap();
        let objs = info["pages"][0]["layers"][0]["objects"]
            .as_array()
            .unwrap()
            .clone();
        let b = objs.iter().find(|o| o["name"] == "b").unwrap()["id"]
            .as_u64()
            .unwrap();
        let sel: Value =
            serde_json::from_str(&text(call(&mut app, "select", &json!({"ids": [b]}))).unwrap())
                .unwrap();
        assert_eq!(sel["count"], 1);
        assert_eq!(sel["objects"][0]["name"], "b");
        let got: Value =
            serde_json::from_str(&text(call(&mut app, "get_selection", &json!({}))).unwrap())
                .unwrap();
        assert_eq!(
            got["bounds_mm"]["x0"].as_f64().map(|x| x.round()),
            Some(50.0)
        );
        assert!(text(call(&mut app, "select", &json!({"ids": [999999]}))).is_err());
        let all = objs.iter().map(|o| o["id"].clone()).collect::<Vec<_>>();
        let sel: Value =
            serde_json::from_str(&text(call(&mut app, "select", &json!({"ids": all}))).unwrap())
                .unwrap();
        assert_eq!(sel["count"], 2);
        text(call(&mut app, "select", &json!({"ids": []}))).unwrap();
        assert!(app.selection.is_empty());
        assert!(info["open_documents"]
            .as_array()
            .is_some_and(|d| !d.is_empty()));
        assert_eq!(info["window"], std::process::id());
    }

    #[test]
    fn render_area_looks_at_objects_closely() {
        use base64::Engine as _;
        let mut app = App::headless();
        draw(&mut app, "const r = ActiveLayer.CreateRectangle(100, 100, 10, 10); r.Fill.ApplyUniformFill(Color.RGB(0, 0, 255)); r.CreateSelection();");
        let r = call(
            &mut app,
            "render_area",
            &json!({"padding_mm": 0, "max_px": 200}),
        )
        .unwrap()
        .unwrap();
        let Reply::Image {
            png_base64,
            text: line,
        } = r
        else {
            panic!("a picture")
        };
        assert!(
            line.contains("selection") && line.contains("10.00 x 10.00 mm"),
            "{line}"
        );
        let png = base64::engine::general_purpose::STANDARD
            .decode(png_base64)
            .unwrap();
        let img = image::load_from_memory(&png).unwrap().to_rgba8();
        assert_eq!((img.width(), img.height()), (200, 200));
        // The rectangle fills the picture.
        assert_eq!(img.get_pixel(100, 100).0, [0, 0, 255, 255]);
        // A rectangle of the page beside it is white.
        let r = call(
            &mut app,
            "render_area",
            &json!({"rect_mm": {"x0": 10, "y0": 10, "x1": 30, "y1": 20}, "max_px": 128}),
        )
        .unwrap()
        .unwrap();
        let Reply::Image { png_base64, .. } = r else {
            panic!()
        };
        let img = image::load_from_memory(
            &base64::engine::general_purpose::STANDARD
                .decode(png_base64)
                .unwrap(),
        )
        .unwrap()
        .to_rgba8();
        assert_eq!((img.width(), img.height()), (128, 64));
        assert_eq!(img.get_pixel(64, 32).0, [255, 255, 255, 255]);
        app.select(vec![]);
        // Nothing selected: the page.
        let r = text(call(&mut app, "render_area", &json!({"max_px": 300}))).unwrap();
        assert!(r.starts_with("page on page 1"), "{r}");
        assert!(text(call(&mut app, "render_area", &json!({"area": "selection"}))).is_err());
    }

    #[test]
    fn fonts_are_measured_not_guessed() {
        let fonts = tracedraw_text::fonts();
        let Some(family) = fonts
            .families()
            .into_iter()
            .find(|f| fonts.face_metrics(f, false, false, 12.0).is_some())
        else {
            return; // no fonts on this machine
        };
        let m: Value = serde_json::from_str(
            &font_metrics(&json!({"family": family, "size_pt": 72, "text": "HxH"})).unwrap(),
        )
        .unwrap();
        let asc = m["ascent_mm"].as_f64().unwrap();
        assert!(asc > 10.0 && asc < 30.0, "{m}");
        assert!(m["advance_width_mm"].as_f64().unwrap() > 0.0);
        // Twice the size, twice the measures.
        let m2: Value = serde_json::from_str(
            &font_metrics(&json!({"family": family, "size_pt": 144})).unwrap(),
        )
        .unwrap();
        assert!((m2["ascent_mm"].as_f64().unwrap() - 2.0 * asc).abs() < 0.01);
        assert!(font_metrics(&json!({"family": "No Such Font Arcano"})).is_err());
        let list = list_fonts(&json!({"filter": family}));
        assert!(list["count"].as_u64().unwrap() >= 1);
    }
}
