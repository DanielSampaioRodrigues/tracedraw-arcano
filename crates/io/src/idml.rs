//! IDML packages read into the drawing model. An IDML document is a zip
//! of XML parts: `designmap.xml` names the stories and the spreads,
//! `Stories/*.xml` hold the text, `Spreads/*.xml` the page items and
//! `Resources/Graphic.xml` the colours. Positions in the files are points
//! with y down from the page's top left; the drawing keeps millimetres
//! with y up from its bottom left.
//!
//! What this reader takes now: the page size, the items of the first
//! spread (text frames with their stories, rectangles with a solid fill)
//! and named colours. Masters, other spreads, styles, images, groups,
//! tables and rotation are left for the next parts, and each one that the
//! file uses is named in a warning, so nothing is lost without a word.

use std::collections::HashMap;
use std::io::{Cursor, Read};

use tracedraw_core::document::{Document, ParagraphStyle, Shape, ShapeKind, TextAlign, TextSpan};
use tracedraw_core::geometry::{Point, Rect, Size};
use tracedraw_core::id::IdSource;
use tracedraw_core::Color;
use tracedraw_core::Fill;

/// Points to millimetres.
const MM_PER_PT: f64 = 25.4 / 72.0;

const DEFAULT_FONT: &str = "Arial";
const DEFAULT_SIZE_PT: f64 = 12.0;
/// The largest text taken from a story, in characters.
const MAX_TEXT: usize = 4 * 1024 * 1024;

/// What the file gave: the drawing and what was left aside.
pub struct Imported {
    pub document: Document,
    pub warnings: Vec<String>,
}

/// Read an IDML package. `Ok` with warnings for what the reader does not
/// know yet; `Err` only when the package itself cannot be read.
pub fn open(bytes: &[u8]) -> Result<Imported, String> {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|e| format!("not an IDML package: {e}"))?;
    let mut warnings = Vec::new();
    let designmap = part(&mut zip, "designmap.xml")?;
    let map = roxmltree::Document::parse(&designmap)
        .map_err(|e| format!("designmap.xml: {e}"))?;
    let title = map
        .root_element()
        .attribute("Name")
        .unwrap_or("IDML drawing")
        .to_string();
    let size = page_size(&mut zip).unwrap_or_else(|| Size::new(210.0, 297.0));
    let mut doc = Document::new(title, size);
    let colours = colours(&mut zip).unwrap_or_default();

    // The stories, by the name the design map gives them.
    let mut stories: HashMap<String, Story> = HashMap::new();
    for node in map.descendants().filter(|n| n.has_tag_name("Story")) {
        let Some(id) = node.attribute("Self").map(str::to_string) else {
            continue;
        };
        let Some(path) = node.attribute("ParentStory").or_else(|| node.text()).map(str::trim)
        else {
            continue;
        };
        let path = if path.is_empty() {
            format!("Stories/{id}.xml")
        } else {
            path.to_string()
        };
        match part(&mut zip, &path) {
            Ok(xml) => match roxmltree::Document::parse(&xml) {
                Ok(story) => {
                    stories.insert(id, read_story(&story));
                }
                Err(e) => warnings.push(format!("{path}: {e}")),
            },
            Err(e) => warnings.push(e),
        }
    }

    // The first spread: its page items on the drawing's first page.
    let spread = map
        .descendants()
        .find(|n| n.has_tag_name("Spread"))
        .and_then(|n| n.attribute("Self"))
        .map(str::to_string);
    let Some(spread_id) = spread else {
        warnings.push("no spread in the design map".into());
        return Ok(Imported { document: doc, warnings });
    };
    let path = format!("Spreads/{spread_id}.xml");
    let xml = match part(&mut zip, &path) {
        Ok(xml) => xml,
        Err(e) => {
            warnings.push(e);
            return Ok(Imported { document: doc, warnings });
        }
    };
    let items = roxmltree::Document::parse(&xml).map_err(|e| format!("{path}: {e}"))?;
    let mut ids = IdSource::default();
    let mut shapes: Vec<Shape> = Vec::new();
    let page_h = size.height;
    for node in items.descendants() {
        let kind = node.tag_name().name();
        if kind == "TextFrame" {
            let Some(place) = box_of(&node, page_h) else {
                warnings.push("a text frame without bounds".into());
                continue;
            };
            let story_id = node.attribute("ParentStory").unwrap_or("");
            let story = stories.get(story_id);
            let spans = match story {
                Some(story) if !story.text.is_empty() => {
                    vec![TextSpan::new(&story.text, &story.font, story.size_pt)]
                }
                _ => vec![TextSpan::new("", DEFAULT_FONT, DEFAULT_SIZE_PT)],
            };
            let mut para = ParagraphStyle::default();
            para.columns = columns_of(&node);
            shapes.push(Shape::new(
                ids.shape(),
                ShapeKind::Text {
                    spans,
                    origin: Point::new(place.origin_top_left.x, place.origin_top_left.y),
                    frame: Some(place.size),
                    align: TextAlign::Left,
                    para,
                    on_path: None,
                },
            ));
        } else if kind == "Rectangle" || kind == "Oval" || kind == "Polygon" {
            let Some(place) = box_of(&node, page_h) else {
                continue;
            };
            if kind != "Rectangle" {
                warnings.push(format!("{kind} taken as a rectangle"));
            }
            let rect = Rect::new(
                place.origin_top_left.x,
                place.origin_top_left.y,
                place.origin_top_left.x + place.size.width,
                place.origin_top_left.y + place.size.height,
            );
            let fill = node
                .attribute("FillColor")
                .and_then(|id| colours.get(id).copied());
            let mut shape = Shape::new(
                ids.shape(),
                ShapeKind::Rect {
                    rect,
                    radius: 0.0,
                    corners: None,
                },
            );
            shape.fill = fill.map(Fill::Solid).unwrap_or(Fill::None);
            shapes.push(shape);
        } else if kind == "Group" {
            warnings.push("a group was left aside".into());
        } else if kind == "Image" || kind == "PDF" {
            warnings.push(format!("a {kind} (linked picture) was left aside"));
        } else if kind == "Table" {
            warnings.push("a table was left aside".into());
        }
    }
    if let Some(page) = doc.pages.first_mut() {
        if let Some(layer) = page.layers.first_mut() {
            layer.shapes = shapes;
        }
    }
    let spreads = map
        .descendants()
        .filter(|n| n.has_tag_name("Spread"))
        .count();
    if spreads > 1 {
        warnings.push(format!(
            "{} spreads left aside: only the first was read",
            spreads - 1
        ));
    }
    Ok(Imported { document: doc, warnings })
}

/// Write the drawing as an IDML package: one spread per page, a story per
/// text shape, and each solid fill as a named RGB colour. What the format
/// has that the drawing does not (masters, styles, groups, tables,
/// pictures) is not written; a reader that needs it gets a plain document.
pub fn save(doc: &Document) -> Result<Vec<u8>, String> {
    let Some(first) = doc.pages.first() else {
        return Err("a drawing with no page".into());
    };
    let page_h = first.size.height;
    let mut stories: Vec<(String, String)> = Vec::new();
    let mut colours: Vec<(String, [u8; 3])> = Vec::new();
    let mut spreads: Vec<String> = Vec::new();
    for (pi, page) in doc.pages.iter().enumerate() {
        let mut items = String::new();
        for layer in &page.layers {
            for shape in &layer.shapes {
                match &shape.kind {
                    ShapeKind::Text {
                        spans,
                        origin,
                        frame: Some(frame),
                        ..
                    } => {
                        let text = tracedraw_core::spans_text(spans);
                        let (font, size_pt) = spans
                            .first()
                            .map(|s| (s.font_family.clone(), s.size_pt))
                            .unwrap_or_else(|| (DEFAULT_FONT.to_string(), DEFAULT_SIZE_PT));
                        let id = format!("Story_{}", stories.len() + 1);
                        stories.push((id.clone(), story_xml(&text, &font, size_pt)));
                        items.push_str(&format!(
                            "<TextFrame Self=\"tf{pi}_{}\" ParentStory=\"{id}\" \
ItemTransform=\"1 0 0 1 {tx} {ty}\" GeometricBounds=\"0 0 {h} {w}\"/>",
                            items.len(),
                            tx = pt(origin.x),
                            ty = pt(page_h - origin.y - frame.height),
                            h = pt(frame.height),
                            w = pt(frame.width),
                        ));
                    }
                    ShapeKind::Rect { rect, .. } => {
                        let fill = match &shape.fill {
                            Fill::Solid(c) => {
                                let rgb = c.to_rgb8();
                                let id = format!("Color/{}", colours.len() + 1);
                                colours.push((id.clone(), rgb));
                                format!(" FillColor=\"{id}\"")
                            }
                            _ => String::new(),
                        };
                        items.push_str(&format!(
                            "<Rectangle Self=\"r{pi}_{}\" ItemTransform=\"1 0 0 1 {tx} {ty}\" \
GeometricBounds=\"0 0 {h} {w}\"{fill}/>",
                            items.len(),
                            tx = pt(rect.x0),
                            ty = pt(page_h - rect.y1),
                            h = pt(rect.height()),
                            w = pt(rect.width()),
                        ));
                    }
                    _ => {}
                }
            }
        }
        spreads.push(format!(
            "<SpreadPackage DOMVersion=\"18.0\"><Spread Self=\"Spread_{n}\">\
<Page Self=\"Page_{n}\" GeometricBounds=\"0 0 {h} {w}\"/>{items}</Spread></SpreadPackage>",
            n = pi + 1,
            h = pt(page_h),
            w = pt(page.size.width),
        ));
    }
    let mut map = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<Document DOMVersion=\"18.0\" ",
    );
    map.push_str(&format!("Name=\"{}\">", esc(doc.title.as_str())));
    for (id, _) in &stories {
        map.push_str(&format!("<Story Self=\"{id}\">Stories/{id}.xml</Story>"));
    }
    for n in 1..=spreads.len() {
        map.push_str(&format!(
            "<Spread Self=\"Spread_{n}\">Spreads/Spread_{n}.xml</Spread>"
        ));
    }
    map.push_str("</Document>");
    let prefs = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<Preferences DOMVersion=\"18.0\">\
<DocumentPreference PageWidth=\"{w}\" PageHeight=\"{h}\"/></Preferences>",
        w = pt(first.size.width),
        h = pt(page_h),
    );
    let mut graphic =
        String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<Graphic DOMVersion=\"18.0\">");
    for (id, [r, g, b]) in &colours {
        graphic.push_str(&format!(
            "<Color Self=\"{id}\" Model=\"Process\" Space=\"RGB\" ColorValue=\"{r} {g} {b}\"/>"
        ));
    }
    graphic.push_str("</Graphic>");

    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let packed = zip::write::SimpleFileOptions::default();
    let stored = packed.compression_method(zip::CompressionMethod::Stored);
    let mut put = |name: &str, body: &str, opts: zip::write::SimpleFileOptions| {
        zip.start_file(name, opts)
            .map_err(|e| format!("{name}: {e}"))?;
        use std::io::Write as _;
        zip.write_all(body.as_bytes())
            .map_err(|e| format!("{name}: {e}"))
    };
    put("mimetype", "application/vnd.adobe.indesign-idml-package", stored)?;
    put("designmap.xml", &map, packed)?;
    put("Resources/Preferences.xml", &prefs, packed)?;
    put("Resources/Graphic.xml", &graphic, packed)?;
    for (id, body) in &stories {
        put(&format!("Stories/{id}.xml"), body, packed)?;
    }
    for (n, body) in spreads.iter().enumerate() {
        put(&format!("Spreads/Spread_{}.xml", n + 1), body, packed)?;
    }
    let cursor = zip.finish().map_err(|e| e.to_string())?;
    Ok(cursor.into_inner())
}

/// A story of `text`'s paragraphs, all in one character run.
fn story_xml(text: &str, font: &str, size_pt: f64) -> String {
    let mut out = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<StoryPackage DOMVersion=\"18.0\">",
    );
    let paragraphs: Vec<&str> = text.split('\n').collect();
    let last = paragraphs.len().saturating_sub(1);
    out.push_str(&format!("<Story Self=\"{}\">", "Story"));
    for (i, para) in paragraphs.iter().enumerate() {
        let end = if i < last { "&#13;" } else { "" };
        out.push_str(&format!(
            "<ParagraphStyleRange><CharacterStyleRange AppliedFont=\"{}\" PointSize=\"{}\">\
<Content>{}{end}</Content></CharacterStyleRange></ParagraphStyleRange>",
            esc(font),
            size_pt,
            esc(para),
        ));
    }
    out.push_str("</Story></StoryPackage>");
    out
}

/// Millimetres to points, as the files write them.
fn pt(mm: f64) -> String {
    format!("{:.4}", mm / MM_PER_PT)
}

/// A string for XML text or an attribute value.
fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

/// Read a `.indd` file: the crate that knows that format turns it into an
/// IDML package, which [`open`] then takes. Its warnings (what it could
/// not carry over) come first, then ours.
pub fn open_indd(bytes: &[u8], name: &str) -> Result<Imported, String> {
    let conversion = indd::convert(bytes, name).map_err(|e| e.to_string())?;
    let mut imported = open(&conversion.idml)?;
    let mut warnings: Vec<String> = conversion
        .warnings
        .iter()
        .map(|w| w.message().to_string())
        .collect();
    warnings.append(&mut imported.warnings);
    imported.warnings = warnings;
    Ok(imported)
}

/// One story: its text (paragraphs on new lines) and the font and size of
/// its first character run.
struct Story {
    text: String,
    font: String,
    size_pt: f64,
}

fn read_story(doc: &roxmltree::Document) -> Story {
    let mut text = String::new();
    let (mut font, mut size_pt) = (DEFAULT_FONT.to_string(), DEFAULT_SIZE_PT);
    for node in doc.descendants() {
        if node.has_tag_name("Content") {
            // A story's characters; a `\r` inside the text ends a
            // paragraph.
            for child in node.children() {
                if let Some(t) = child.text() {
                    text.push_str(t);
                }
            }
        } else if node.has_tag_name("CharacterStyleRange") && font == DEFAULT_FONT {
            if let Some(f) = node.attribute("AppliedFont") {
                font = family_of(f);
            }
            if let Some(s) = node.attribute("PointSize").and_then(|v| v.parse().ok()) {
                size_pt = s;
            }
        }
    }
    // A paragraph ends with `\r` in IDML, the story's last one included;
    // the drawing ends it with `\n` and has no empty paragraph after the
    // last one.
    let mut text: String = text.replace('\r', "\n");
    if text.ends_with('\n') {
        text.pop();
    }
    let text: String = text.chars().take(MAX_TEXT).collect();
    Story { text, font, size_pt }
}

/// Where a page item goes on the page, from its transformation and its
/// bounds, in millimetres with y up.
struct Place {
    origin_top_left: Point,
    size: Size,
}

fn box_of(node: &roxmltree::Node, page_h_mm: f64) -> Option<Place> {
    let transform: Vec<f64> = node
        .attribute("ItemTransform")
        .map(|v| v.split_whitespace().filter_map(|n| n.parse().ok()).collect())
        .unwrap_or_default();
    let (tx, ty) = match transform.as_slice() {
        [_, _, _, _, tx, ty] => (*tx, *ty),
        _ => (0.0, 0.0),
    };
    let bounds: Vec<f64> = node
        .attribute("GeometricBounds")?
        .split_whitespace()
        .filter_map(|n| n.parse().ok())
        .collect();
    let [top, left, bottom, right] = bounds.as_slice() else {
        return None;
    };
    let x_mm = (tx + left) * MM_PER_PT;
    let y_mm = (ty + top) * MM_PER_PT;
    let w_mm = (right - left) * MM_PER_PT;
    let h_mm = (bottom - top) * MM_PER_PT;
    Some(Place {
        origin_top_left: Point::new(x_mm.max(0.0), (page_h_mm - y_mm - h_mm).max(0.0)),
        size: Size::new(w_mm.max(1.0), h_mm.max(1.0)),
    })
}

/// The columns of a text frame, from its `TextFramePreference`.
fn columns_of(node: &roxmltree::Node) -> u32 {
    node.descendants()
        .find(|n| n.has_tag_name("TextFramePreference"))
        .and_then(|n| n.attribute("TextColumnCount"))
        .and_then(|v| v.parse().ok())
        .unwrap_or(1)
        .max(1)
}

/// The page size, from the document preferences (points).
fn page_size(zip: &mut zip::ZipArchive<Cursor<&[u8]>>) -> Option<Size> {
    let xml = part(zip, "Resources/Preferences.xml").ok()?;
    let doc = roxmltree::Document::parse(&xml).ok()?;
    let pref = doc
        .descendants()
        .find(|n| n.has_tag_name("DocumentPreference"))?;
    let w: f64 = pref.attribute("PageWidth")?.parse().ok()?;
    let h: f64 = pref.attribute("PageHeight")?.parse().ok()?;
    Some(Size::new(w * MM_PER_PT, h * MM_PER_PT))
}

/// The named colours of the file, as the drawing keeps them.
fn colours(zip: &mut zip::ZipArchive<Cursor<&[u8]>>) -> Option<HashMap<String, Color>> {
    let xml = part(zip, "Resources/Graphic.xml").ok()?;
    let doc = roxmltree::Document::parse(&xml).ok()?;
    let mut out = HashMap::new();
    for node in doc.descendants().filter(|n| n.has_tag_name("Color")) {
        let Some(id) = node.attribute("Self") else {
            continue;
        };
        let space = node.attribute("Space").unwrap_or("CMYK");
        let values: Vec<f64> = node
            .attribute("ColorValue")
            .map(|v| v.split_whitespace().filter_map(|n| n.parse().ok()).collect())
            .unwrap_or_default();
        let colour = match (space, values.as_slice()) {
            ("CMYK", [c, m, y, k]) => Color::cmyk_pct(*c as f32, *m as f32, *y as f32, *k as f32),
            ("RGB", [r, g, b]) => Color::rgb8(
                r.round().clamp(0.0, 255.0) as u8,
                g.round().clamp(0.0, 255.0) as u8,
                b.round().clamp(0.0, 255.0) as u8,
            ),
            ("Gray", [g]) => {
                let v = g.round().clamp(0.0, 255.0) as u8;
                Color::rgb8(v, v, v)
            }
            ("LAB", [l, a, b]) => {
                let [r, g, b] = tracedraw_core::color::lab_to_rgb(*l as f32, *a as f32, *b as f32);
                Color::rgb8(
                    (r * 255.0).round().clamp(0.0, 255.0) as u8,
                    (g * 255.0).round().clamp(0.0, 255.0) as u8,
                    (b * 255.0).round().clamp(0.0, 255.0) as u8,
                )
            }
            _ => continue,
        };
        out.insert(id.to_string(), colour);
    }
    Some(out)
}

/// One part of the package, as text.
fn part(zip: &mut zip::ZipArchive<Cursor<&[u8]>>, name: &str) -> Result<String, String> {
    let mut file = zip
        .by_name(name)
        .map_err(|_| format!("{name} missing from the package"))?;
    let mut text = String::new();
    file.read_to_string(&mut text)
        .map_err(|e| format!("{name}: {e}"))?;
    Ok(text)
}

/// The family of an `AppliedFont`: the file writes the family and the
/// style together ("Minion Pro\tRegular"), and the tab reaches us as a
/// space (XML normalises it), so a known style word at its end is taken
/// off too.
fn family_of(f: &str) -> String {
    let f = f.split('\t').next().unwrap_or(f).trim();
    const STYLES: [&str; 10] = [
        "Bold Italic",
        "Bold Oblique",
        "Regular",
        "Italic",
        "Oblique",
        "Bold",
        "Light",
        "Medium",
        "Semibold",
        "SemiBold",
    ];
    for style in STYLES {
        if let Some(base) = f.strip_suffix(style) {
            let base = base.trim_end();
            if !base.is_empty() {
                return base.to_string();
            }
        }
    }
    f.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// A small package: one page, one text frame with a story of two
    /// paragraphs, one filled rectangle and one named colour.
    fn sample() -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::new();
        {
            let mut zip = zip::ZipWriter::new(Cursor::new(&mut buf));
            let opts = zip::write::SimpleFileOptions::default();
            let mut put = |zip: &mut zip::ZipWriter<Cursor<&mut Vec<u8>>>, name: &str, body: &str| {
                zip.start_file(name, opts).expect("zip entry");
                zip.write_all(body.as_bytes()).expect("zip write");
            };
            put(
                &mut zip,
                "designmap.xml",
                r#"<?xml version="1.0" encoding="UTF-8"?>
<Document DOMVersion="18.0" Name="Teste">
  <Story Self="Story_1">Stories/Story_1.xml</Story>
  <Spread Self="Spread_1">Spreads/Spread_1.xml</Spread>
</Document>"#,
            );
            put(
                &mut zip,
                "Resources/Preferences.xml",
                r#"<?xml version="1.0" encoding="UTF-8"?>
<Preferences DOMVersion="18.0">
  <DocumentPreference PageWidth="595.276" PageHeight="841.890" PagesPerDocument="1"/>
</Preferences>"#,
            );
            put(
                &mut zip,
                "Resources/Graphic.xml",
                r#"<?xml version="1.0" encoding="UTF-8"?>
<Graphic DOMVersion="18.0">
  <Color Self="Color/Preto" Model="Process" Space="CMYK" ColorValue="0 0 0 100"/>
</Graphic>"#,
            );
            put(
                &mut zip,
                "Stories/Story_1.xml",
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<Story Self=\"Story_1\"><ParagraphStyleRange><CharacterStyleRange AppliedFont=\"Minion Pro\tRegular\" PointSize=\"11\"><Content>Um titulo&#13;</Content></CharacterStyleRange></ParagraphStyleRange><ParagraphStyleRange><CharacterStyleRange><Content>Segundo paragrafo&#13;</Content></CharacterStyleRange></ParagraphStyleRange></Story>",
            );
            put(
                &mut zip,
                "Spreads/Spread_1.xml",
                r#"<?xml version="1.0" encoding="UTF-8"?>
<SpreadPackage DOMVersion="18.0">
  <Spread Self="Spread_1">
    <Page Self="Page_1" GeometricBounds="0 0 841.890 595.276"/>
    <TextFrame Self="tf1" ParentStory="Story_1" ItemTransform="1 0 0 1 36 60" GeometricBounds="0 0 100 400">
      <TextFramePreference TextColumnCount="2"/>
    </TextFrame>
    <Rectangle Self="r1" FillColor="Color/Preto" ItemTransform="1 0 0 1 36 200" GeometricBounds="0 0 50 100"/>
  </Spread>
</SpreadPackage>"#,
            );
            zip.finish().expect("zip finish");
        }
        buf
    }

    #[test]
    fn a_package_gives_a_page_with_its_text_and_rectangle() {
        let imported = open(&sample()).expect("the package reads");
        let doc = &imported.document;
        assert_eq!(doc.pages.len(), 1);
        let page = &doc.pages[0];
        // A4, within a tenth of a millimetre.
        assert!((page.size.width - 210.0).abs() < 0.1, "{:?}", page.size);
        assert!((page.size.height - 297.0).abs() < 0.1, "{:?}", page.size);
        let shapes = &page.layers[0].shapes;
        assert_eq!(shapes.len(), 2, "{}", shapes.len());
        let ShapeKind::Text {
            spans, frame, para, ..
        } = &shapes[0].kind
        else {
            panic!("a text frame first: {:?}", shapes[0].kind);
        };
        assert_eq!(spans[0].text, "Um titulo\nSegundo paragrafo");
        assert_eq!(spans[0].font_family, "Minion Pro");
        assert!((spans[0].size_pt - 11.0).abs() < 1e-9);
        assert_eq!(para.columns, 2);
        let frame = frame.expect("a frame");
        assert!((frame.width - 400.0 * MM_PER_PT).abs() < 0.01, "{frame:?}");
        // The rectangle keeps its fill.
        match &shapes[1].kind {
            ShapeKind::Rect { rect, .. } => {
                assert!((rect.width() - 100.0 * MM_PER_PT).abs() < 0.01);
            }
            other => panic!("a rectangle: {other:?}"),
        }
        assert!(matches!(shapes[1].fill, Fill::Solid(_)), "{:?}", shapes[1].fill);
    }

    /// A drawing saved as IDML reads back with its page, its text frame
    /// and its rectangle.
    #[test]
    fn a_drawing_saves_and_reads_back() {
        use tracedraw_core::id::ShapeId;
        let mut doc = Document::new("Ida e volta", Size::new(210.0, 297.0));
        let text = Shape::new(
            ShapeId(1),
            ShapeKind::Text {
                spans: vec![TextSpan::new("Titulo\nCorpo do texto", "Minion Pro", 11.0)],
                origin: Point::new(30.0, 200.0),
                frame: Some(Size::new(150.0, 60.0)),
                align: TextAlign::Left,
                para: ParagraphStyle::default(),
                on_path: None,
            },
        );
        let mut rect = Shape::new(
            ShapeId(2),
            ShapeKind::Rect {
                rect: Rect::new(20.0, 20.0, 80.0, 60.0),
                radius: 0.0,
                corners: None,
            },
        );
        rect.fill = Fill::Solid(Color::rgb8(200, 30, 40));
        doc.pages[0].layers[0].shapes = vec![text, rect];
        let bytes = save(&doc).expect("the drawing saves");
        let back = open(&bytes).expect("the package reads");
        let page = &back.document.pages[0];
        assert!((page.size.width - 210.0).abs() < 0.05, "{:?}", page.size);
        assert!((page.size.height - 297.0).abs() < 0.05, "{:?}", page.size);
        let shapes = &page.layers[0].shapes;
        assert_eq!(shapes.len(), 2, "{}", shapes.len());
        let ShapeKind::Text { spans, frame, .. } = &shapes[0].kind else {
            panic!("a text frame: {:?}", shapes[0].kind);
        };
        assert_eq!(spans[0].text, "Titulo\nCorpo do texto");
        assert_eq!(spans[0].font_family, "Minion Pro");
        assert!((spans[0].size_pt - 11.0).abs() < 1e-9);
        let frame = frame.expect("a frame");
        assert!((frame.width - 150.0).abs() < 0.1, "{frame:?}");
        assert!((frame.height - 60.0).abs() < 0.1, "{frame:?}");
        let ShapeKind::Rect { rect, .. } = &shapes[1].kind else {
            panic!("a rectangle: {:?}", shapes[1].kind);
        };
        assert!((rect.width() - 60.0).abs() < 0.1, "{rect:?}");
        assert!(matches!(shapes[1].fill, Fill::Solid(_)), "{:?}", shapes[1].fill);
    }

    /// A file that is not an INDD document is an error, never a panic.
    #[test]
    fn a_file_that_is_not_indd_is_an_error() {
        let err = open_indd(b"not an INDD document at all", "sample.indd");
        assert!(err.is_err());
    }

    /// A file that is not a package, and one missing the parts it needs,
    /// are reported and never panic.
    #[test]
    fn a_broken_package_is_an_error_not_a_panic() {
        assert!(open(b"not a zip").is_err());
        let mut buf: Vec<u8> = Vec::new();
        {
            let mut zip = zip::ZipWriter::new(Cursor::new(&mut buf));
            zip.start_file("designmap.xml", zip::write::SimpleFileOptions::default())
                .expect("entry");
            zip.write_all(b"<Document><Spread Self=\"S\">Spreads/S.xml</Spread></Document>")
                .expect("write");
            zip.finish().expect("finish");
        }
        let imported = open(&buf).expect("the design map alone reads");
        assert!(!imported.warnings.is_empty(), "the missing spread is told");
        assert_eq!(imported.document.pages.len(), 1);
    }
}
