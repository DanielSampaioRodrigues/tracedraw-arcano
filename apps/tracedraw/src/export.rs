//! Export (vector and raster formats), print (through a PDF handed to
//! the system viewer), print merge, find and replace.

use crate::app::App;
use crate::i18n::{tr, trf};
use crate::ui::dialogs::{ExportState, PrintState, EXPORT_FORMATS};
use std::path::Path;
use tracedraw_core::{
    document::ShapeKind,
    geometry::{Affine, Rect, Size},
    Command, Document, ShapeId, TextSpan,
};

/// Document limited to the selection or the current page, for exports.
pub(crate) fn export_document(app: &App, st: &ExportState) -> (Document, Rect) {
    let doc = app.engine.document();
    if st.selection_only {
        let bounds = app.selection_bounds().unwrap_or(app.page_rect());
        let mut tmp = Document::new(
            doc.title.clone(),
            Size::new(bounds.width().max(0.1), bounds.height().max(0.1)),
        );
        tmp.symbols = doc.symbols.clone();
        let l = tmp.pages[0].layers[0].id;
        for s in app.selected_shapes() {
            let mut c = s.clone();
            c.transform = Affine::translate((-bounds.x0, -bounds.y0)) * c.transform;
            if let Ok(layer) = tmp.layer_mut(l) {
                layer.shapes.push(c);
            }
        }
        (tmp, bounds)
    } else if st.all_pages {
        (doc.clone(), app.page_rect())
    } else {
        let mut tmp = doc.clone();
        tmp.pages.retain(|p| p.id == app.page);
        (tmp, app.page_rect())
    }
}

pub fn export(app: &mut App, path: &Path, st: &ExportState) -> Result<String, String> {
    let (_, ext) = EXPORT_FORMATS[st.format];
    let (doc, _) = export_document(app, st);
    match ext {
        "svg" => {
            let svg = tracedraw_io::svg::page_to_svg(&doc, 0);
            crate::files::write(path, svg).map_err(|e| e.to_string())?;
        }
        "pdf" | "ai" => {
            use tracedraw_io::pdf::{PdfOptions, PdfStandard};
            let standard = match (ext, st.pdf_standard) {
                ("pdf", 1) => PdfStandard::X1a,
                ("pdf", 2) => PdfStandard::X3,
                ("pdf", 3) => PdfStandard::X4,
                _ => PdfStandard::None,
            };
            let opts = PdfOptions {
                standard,
                output_profile: pdfx_profile(app, standard),
                output_condition: st.output_condition.clone(),
                bleed_mm: if ext == "pdf" { st.bleed_mm } else { 0.0 },
                title: Some(app.document_title().trim_end_matches('*').to_string()),
                flatten_dpi: st.dpi.max(72.0),
                ..Default::default()
            };
            let pdf = tracedraw_io::pdf::document_to_pdf_with(&doc, &opts);
            crate::files::write(path, pdf).map_err(|e| e.to_string())?;
        }
        "eps" => {
            let eps = tracedraw_io::eps::page_to_eps(&doc, 0);
            crate::files::write(path, eps).map_err(|e| e.to_string())?;
        }
        "ps" | "prn" => {
            // Every exported page; a print file is the same PostScript.
            let ps = tracedraw_io::eps::document_to_ps(&doc);
            crate::files::write(path, ps).map_err(|e| e.to_string())?;
        }
        "dxf" => {
            let dxf = tracedraw_io::dxf::page_to_dxf(&doc, 0);
            crate::files::write(path, dxf).map_err(|e| e.to_string())?;
        }
        "emf" => {
            let emf = tracedraw_io::emf::page_to_emf(&doc, 0);
            crate::files::write(path, emf).map_err(|e| e.to_string())?;
        }
        "wmf" => {
            let wmf = tracedraw_io::emf::page_to_wmf(&doc, 0);
            crate::files::write(path, wmf).map_err(|e| e.to_string())?;
        }
        "plt" => {
            let plt = tracedraw_io::plt::page_to_plt(&doc, 0);
            crate::files::write(path, plt).map_err(|e| e.to_string())?;
        }
        "psd" => {
            let psd =
                tracedraw_io::psd::page_to_psd(&doc, 0, st.dpi.max(36.0)).ok_or("render failed")?;
            crate::files::write(path, psd).map_err(|e| e.to_string())?;
        }
        "html" => {
            let html = tracedraw_io::html::document_to_html(&doc);
            crate::files::write(path, html).map_err(|e| e.to_string())?;
        }
        _ => {
            // Raster: every page when all_pages, numbered files.
            let pages: Vec<_> = doc.pages.iter().map(|p| p.id).collect();
            for (i, pid) in pages.iter().enumerate() {
                let mut pm = tracedraw_render::render_page_image(&doc, *pid, st.dpi)
                    .ok_or("render failed")?;
                if st.transparent && matches!(ext, "png" | "webp" | "gif") {
                    // Re-render on a transparent background.
                    let p = doc.page(*pid).map_err(|e| e.to_string())?;
                    let zoom = st.dpi / 25.4;
                    let w = (p.size.width * zoom).ceil() as u32;
                    let h = (p.size.height * zoom).ceil() as u32;
                    pm = tracedraw_render::render_page(
                        &doc,
                        *pid,
                        &tracedraw_render::RenderOptions {
                            width: w,
                            height: h,
                            view: tracedraw_render::ViewTransform {
                                zoom,
                                origin_x: 0.0,
                                origin_y: h as f64,
                            },
                            preview: None,
                            wireframe: false,
                            ..tracedraw_render::RenderOptions::default()
                        },
                    )
                    .ok_or("render failed")?;
                }
                let target = if pages.len() > 1 {
                    let stem = path
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default();
                    path.with_file_name(format!("{stem}-{}.{ext}", i + 1))
                } else {
                    path.to_path_buf()
                };
                write_raster(&pm, &target, ext, st.quality)?;
            }
        }
    }
    Ok(trf(
        "status.exported",
        &[("path", &path.display().to_string())],
    ))
}

/// The output intent's profile for a PDF/X file: the CMYK profile picked
/// under Colour Management, when there is one and it can be read.
pub(crate) fn pdfx_profile(app: &App, standard: tracedraw_io::pdf::PdfStandard) -> Option<Vec<u8>> {
    if standard == tracedraw_io::pdf::PdfStandard::None {
        return None;
    }
    let p = app.settings.color.cmyk_profile_path.trim().to_string();
    (!p.is_empty())
        .then(|| crate::files::read(&p).ok())
        .flatten()
}

fn write_raster(pm: &tiny_skia::Pixmap, path: &Path, ext: &str, quality: u8) -> Result<(), String> {
    let bytes = encode_raster(pm, ext, quality)?;
    crate::files::write(path, bytes).map_err(|e| e.to_string())
}

/// True when the export format `ext` keeps transparency.
pub fn keeps_alpha(ext: &str) -> bool {
    matches!(ext, "png" | "webp" | "gif")
}

/// What the export will look like: the page (or the selection) of `st`
/// rendered to fit `max_px` on its longer side, on a transparent
/// background when the format keeps one, else on white.
pub fn preview(app: &App, st: &ExportState, max_px: u32) -> Option<tiny_skia::Pixmap> {
    let (_, ext) = EXPORT_FORMATS[st.format.min(EXPORT_FORMATS.len() - 1)];
    let (doc, _) = export_document(app, st);
    let page = doc.pages.first()?;
    let longest = page.size.width.max(page.size.height).max(0.1);
    let zoom = max_px.max(16) as f64 / longest;
    let w = (page.size.width * zoom).ceil().max(1.0) as u32;
    let h = (page.size.height * zoom).ceil().max(1.0) as u32;
    let mut pm = tracedraw_render::render_page(
        &doc,
        page.id,
        &tracedraw_render::RenderOptions {
            width: w,
            height: h,
            view: tracedraw_render::ViewTransform {
                zoom,
                origin_x: 0.0,
                origin_y: h as f64,
            },
            preview: None,
            wireframe: false,
            ..tracedraw_render::RenderOptions::default()
        },
    )?;
    if !(st.transparent && keeps_alpha(ext)) {
        let mut white = tiny_skia::Pixmap::new(w, h)?;
        white.fill(tiny_skia::Color::WHITE);
        white.draw_pixmap(
            0,
            0,
            pm.as_ref(),
            &tiny_skia::PixmapPaint::default(),
            tiny_skia::Transform::identity(),
            None,
        );
        pm = white;
    }
    Some(pm)
}

/// About how many bytes the export of `st` takes: a vector format is
/// written in memory; a bitmap format is estimated from `preview` (the
/// output of [`preview`]) encoded the same way and scaled to the export's
/// pixel count.
pub fn estimated_size(app: &App, st: &ExportState, preview: &tiny_skia::Pixmap) -> Option<usize> {
    let (_, ext) = EXPORT_FORMATS[st.format.min(EXPORT_FORMATS.len() - 1)];
    let (doc, bounds) = export_document(app, st);
    match ext {
        "svg" => Some(tracedraw_io::svg::page_to_svg(&doc, 0).len()),
        "eps" => Some(tracedraw_io::eps::page_to_eps(&doc, 0).len()),
        "ps" | "prn" => Some(tracedraw_io::eps::document_to_ps(&doc).len()),
        "pdf" | "ai" => Some(tracedraw_io::pdf::document_to_pdf(&doc).len()),
        "png" | "jpg" | "bmp" | "tif" | "webp" | "gif" | "psd" => {
            let enc = if ext == "psd" { "tif" } else { ext };
            let bytes = encode_raster(preview, enc, st.quality).ok()?.len() as f64;
            let full = (bounds.width() / 25.4 * st.dpi) * (bounds.height() / 25.4 * st.dpi);
            let shown = (preview.width() as f64 * preview.height() as f64).max(1.0);
            let pages = if st.all_pages && !st.selection_only {
                doc.pages.len().max(1) as f64
            } else {
                1.0
            };
            Some((bytes * full / shown * pages) as usize)
        }
        _ => None,
    }
}

/// A byte count for people: "820 KB", "1.4 MB".
pub fn human_size(bytes: usize) -> String {
    let b = bytes as f64;
    if b < 1000.0 * 1024.0 {
        format!("{:.0} KB", (b / 1024.0).max(1.0))
    } else {
        format!("{:.1} MB", b / (1024.0 * 1024.0))
    }
}

/// Encode a rendered page in the bitmap format `ext`.
fn encode_raster(pm: &tiny_skia::Pixmap, ext: &str, quality: u8) -> Result<Vec<u8>, String> {
    let (w, h) = (pm.width(), pm.height());
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for p in pm.pixels() {
        let d = p.demultiply();
        rgba.extend_from_slice(&[d.red(), d.green(), d.blue(), d.alpha()]);
    }
    let img = image::RgbaImage::from_raw(w, h, rgba).ok_or("image buffer")?;
    let dynimg = image::DynamicImage::ImageRgba8(img);
    let format = match ext {
        "png" => image::ImageFormat::Png,
        "jpg" => image::ImageFormat::Jpeg,
        "bmp" => image::ImageFormat::Bmp,
        "tif" => image::ImageFormat::Tiff,
        "webp" => image::ImageFormat::WebP,
        "gif" => image::ImageFormat::Gif,
        _ => image::ImageFormat::Png,
    };
    let mut out = std::io::Cursor::new(Vec::new());
    match format {
        image::ImageFormat::Jpeg => {
            let rgb = dynimg.to_rgb8();
            let mut enc =
                image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, quality.clamp(1, 100));
            enc.encode_image(&rgb).map_err(|e| e.to_string())?;
        }
        image::ImageFormat::Bmp => {
            dynimg
                .to_rgb8()
                .write_to(&mut out, format)
                .map_err(|e| e.to_string())?;
        }
        _ => dynimg
            .write_to(&mut out, format)
            .map_err(|e| e.to_string())?,
    }
    Ok(out.into_inner())
}

/// Print: build a PDF with the print options and open it in the system
/// viewer, which owns the printer dialog.
pub fn print(app: &mut App, st: &PrintState) -> Result<String, String> {
    let mut doc = app.engine.document().clone();
    if !st.all_pages {
        doc.pages.retain(|p| p.id == app.page);
    }
    let options = print_layout(app, st, &doc);
    let mut doc = crate::imposition::impose(&doc, &options);
    for page in doc.pages.iter_mut() {
        let w = page.size.width;
        for layer in page.layers.iter_mut() {
            for s in layer.shapes.iter_mut() {
                // Emulsion down: the whole sheet, marks too, mirrored.
                if st.mirror {
                    s.transform = Affine::translate((w, 0.0))
                        * Affine::new([-1.0, 0.0, 0.0, 1.0, 0.0, 0.0])
                        * s.transform;
                }
                if st.invert {
                    invert_colors(s);
                }
            }
        }
    }
    let pdf = if st.separations {
        separations_pdf(&doc)
    } else {
        tracedraw_io::pdf::document_to_pdf(&doc)
    };
    let name = format!("{}.pdf", app.document_title().trim_end_matches('*'));
    if crate::files::WEB {
        // In a browser the PDF is downloaded; its viewer prints it.
        let path = std::path::PathBuf::from("/download").join(&name);
        crate::files::write(&path, pdf).map_err(|e| e.to_string())?;
        return Ok(trf("status.print_sent", &[("path", &name)]));
    }
    let dir = std::env::temp_dir().join("tracedraw-print");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(name);
    crate::files::write(&path, pdf).map_err(|e| e.to_string())?;
    // The system viewer handles the copy count; open it once.
    open_with_system(&path)?;
    Ok(trf(
        "status.print_sent",
        &[("path", &path.display().to_string())],
    ))
}

/// The print layout for the Print dialog's settings: the imposition, the
/// sheet, the bleed and the marks.
pub fn print_layout(app: &App, st: &PrintState, doc: &Document) -> crate::imposition::Options {
    let bleed = if st.bleed {
        doc.metadata.bleed.max(0.0)
    } else {
        0.0
    };
    let tz = crate::time_tracker::local_zone();
    crate::imposition::Options {
        layout: st.layout(),
        sheet: st.sheet_size(),
        gutter: st.gutter.max(0.0),
        bleed,
        scale: if st.fit_to_page {
            1.0
        } else {
            st.scale / 100.0
        },
        marks: crate::imposition::Marks {
            crop: st.crop_marks,
            registration: st.registration_marks,
            color_bars: st.color_bars,
            densitometer: st.densitometer,
            page_info: st.page_info,
        },
        title: app.document_name(),
        when: crate::time_tracker::date_time(crate::time_tracker::unix_now(), &tz),
    }
}

/// Map every colour of `s` and what it holds through `f`: fills of every
/// kind, outlines, character fills, groups and clip frames.
pub fn map_colors(
    s: &mut tracedraw_core::Shape,
    f: &dyn Fn(tracedraw_core::Color) -> tracedraw_core::Color,
) {
    fn fill(
        x: &mut tracedraw_core::Fill,
        f: &dyn Fn(tracedraw_core::Color) -> tracedraw_core::Color,
    ) {
        use tracedraw_core::{Fill, Pattern};
        match x {
            Fill::Solid(c) => *c = f(*c),
            Fill::Fountain(g) => g.stops.iter_mut().for_each(|s| s.color = f(s.color)),
            Fill::Mesh(m) => m.nodes.iter_mut().for_each(|n| n.color = f(n.color)),
            Fill::Pattern(Pattern::TwoColor { front, back, .. }) => {
                *front = f(*front);
                *back = f(*back);
            }
            _ => {}
        }
    }
    fill(&mut s.fill, f);
    if let Some(st) = &mut s.stroke {
        st.color = f(st.color);
    }
    match &mut s.kind {
        ShapeKind::Group { children } => children.iter_mut().for_each(|c| map_colors(c, f)),
        ShapeKind::ClipFrame { frame, contents } => {
            map_colors(frame, f);
            contents.iter_mut().for_each(|c| map_colors(c, f));
        }
        ShapeKind::Text { spans, .. } => {
            for sp in spans.iter_mut() {
                if let Some(x) = &mut sp.fill {
                    fill(x, f);
                }
            }
        }
        _ => {}
    }
}

fn invert_colors(s: &mut tracedraw_core::Shape) {
    use tracedraw_core::Color;
    map_colors(s, &|c: Color| {
        let [r, g, b] = c.to_rgb8();
        Color::rgb8(255 - r, 255 - g, 255 - b)
    });
}

/// Colour separations: one grayscale page per ink (C, M, Y, K) per page,
/// where the ink coverage of every colour is drawn as a gray value.
fn separations_pdf(doc: &Document) -> Vec<u8> {
    use tracedraw_core::{Color, Fill};
    let mut sep = Document::new(format!("{} (separations)", doc.title), doc.pages[0].size);
    sep.pages.clear();
    sep.symbols = doc.symbols.clone();
    for page in &doc.pages {
        for (ink, name) in [
            (0usize, "Cyan"),
            (1, "Magenta"),
            (2, "Yellow"),
            (3, "Black"),
        ] {
            let mut p = page.clone();
            p.name = format!("{} {}", page.name, name);
            let mut ids = sep.ids().clone();
            p.id = ids.page();
            sep.set_ids(ids);
            let coverage = move |c: Color| -> Color {
                // Registration prints on every plate.
                if matches!(c, Color::Registration) {
                    return Color::Gray { v: 0.0 };
                }
                let v = match c.convert_to("CMYK") {
                    Color::Cmyk { c, m, y, k } => [c, m, y, k][ink],
                    _ => 0.0,
                };
                Color::Gray { v: 1.0 - v }
            };
            for layer in p.layers.iter_mut() {
                for s in layer.shapes.iter_mut() {
                    // Fills the separations cannot split show as their
                    // average colour.
                    if matches!(s.fill, Fill::Texture(_) | Fill::Pattern(_)) {
                        if let Some(c) = s.fill.preview_color() {
                            s.fill = Fill::Solid(c);
                        }
                    }
                    map_colors(s, &coverage);
                }
            }
            sep.pages.push(p);
        }
    }
    tracedraw_io::pdf::document_to_pdf(&sep)
}

pub fn open_with_system(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    let r = std::process::Command::new("cmd")
        .args(["/C", "start", "", &path.display().to_string()])
        .spawn();
    #[cfg(target_os = "macos")]
    let r = std::process::Command::new("open").arg(path).spawn();
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let r = std::process::Command::new("xdg-open").arg(path).spawn();
    r.map(|_| ())
        .map_err(|e| format!("{}: {e}", tr("status.could_not_open_viewer")))
}

impl App {
    /// Open a web page in the system browser (a new tab in the browser
    /// build).
    pub fn open_url(&mut self, url: &str) {
        #[cfg(target_arch = "wasm32")]
        {
            crate::web::open_url(url);
            return;
        }
        #[allow(unreachable_code)]
        if let Err(e) = open_with_system(Path::new(url)) {
            self.status = e;
        }
    }

    /// File name for a PDF sent somewhere: the document title, cleaned.
    fn send_to_file_name(&self) -> String {
        let title = self.document_title();
        let stem: String = title
            .trim_end_matches('*')
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let stem = stem.trim();
        let stem = if stem.is_empty() { "Graphic1" } else { stem };
        format!("{stem}.pdf")
    }

    /// File > Send To: write the document as PDF into `target` and report
    /// where it went.
    pub fn send_to(&mut self, target: SendTarget) {
        if crate::files::WEB {
            // No folders in a browser: every target is a PDF download.
            let name = self.send_to_file_name();
            let pdf = tracedraw_io::pdf::document_to_pdf(self.engine.document());
            let path = std::path::PathBuf::from("/download").join(&name);
            self.status = match crate::files::write(&path, pdf) {
                Ok(()) => trf("status.sent_to", &[("path", &name)]),
                Err(e) => trf("status.export_failed", &[("e", &e.to_string())]),
            };
            let _ = target;
            return;
        }
        let Some(dir) = target.dir() else {
            self.status = tr("status.send_to_no_folder");
            return;
        };
        let path = dir.join(self.send_to_file_name());
        match tracedraw_io::save_pdf(self.engine.document(), &path) {
            Ok(()) => {
                self.status = trf("status.sent_to", &[("path", &path.display().to_string())]);
                if target == SendTarget::Mail {
                    // The platform's mail client gets the PDF as an
                    // attachment; without a mailto attachment standard,
                    // open the file's folder so it can be dragged in.
                    let _ = open_with_system(&dir);
                }
            }
            Err(e) => self.status = trf("status.export_failed", &[("e", &e.to_string())]),
        }
    }
}

/// Destinations of File > Send To.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendTarget {
    Desktop,
    Documents,
    Mail,
}

impl SendTarget {
    /// The folder the PDF is written to; the mail recipient gets it from
    /// the system's temporary folder.
    pub fn dir(self) -> Option<std::path::PathBuf> {
        match self {
            SendTarget::Desktop => user_dir("XDG_DESKTOP_DIR", "Desktop"),
            SendTarget::Documents => user_dir("XDG_DOCUMENTS_DIR", "Documents"),
            SendTarget::Mail => {
                let d = std::env::temp_dir().join("tracedraw-send");
                std::fs::create_dir_all(&d).ok()?;
                Some(d)
            }
        }
    }
}

/// `$XDG_*_DIR`, then `~/<name>` (created when missing), then the home
/// folder itself.
fn user_dir(env: &str, name: &str) -> Option<std::path::PathBuf> {
    if let Some(p) = std::env::var_os(env) {
        let p = std::path::PathBuf::from(p);
        if p.is_dir() {
            return Some(p);
        }
    }
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(std::path::PathBuf::from)?;
    let p = home.join(name);
    if p.is_dir() || std::fs::create_dir_all(&p).is_ok() {
        Some(p)
    } else {
        Some(home)
    }
}

// ----- print merge ---------------------------------------------------------------

pub fn parse_csv(text: &str) -> (Vec<String>, Vec<Vec<String>>) {
    let delim = if text
        .lines()
        .next()
        .map(|l| l.matches(';').count() > l.matches(',').count())
        .unwrap_or(false)
    {
        b';'
    } else {
        b','
    };
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(delim)
        .flexible(true)
        .from_reader(text.as_bytes());
    let headers: Vec<String> = rdr
        .headers()
        .map(|h| h.iter().map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();
    let rows: Vec<Vec<String>> = rdr
        .records()
        .filter_map(|r| r.ok())
        .map(|r| r.iter().map(|s| s.to_string()).collect())
        .collect();
    (headers, rows)
}

/// Duplicate the current page once per data row, substituting `<Field>`
/// placeholders in every text object.
pub fn perform_merge(app: &mut App, headers: &[String], rows: &[Vec<String>]) {
    let page = app.page;
    let template = match app.doc().page(page) {
        Ok(p) => p.clone(),
        Err(_) => return,
    };
    let mut cmds = Vec::new();
    for _ in rows {
        cmds.push(Command::DuplicatePage { page });
    }
    if let Err(e) = app.engine.run_batch("Print Merge", &cmds) {
        app.status = e.to_string();
        return;
    }
    // The duplicates were inserted right after the template page, in order:
    // DuplicatePage inserts at idx + 1 each time, so the last row is nearest.
    let idx = app
        .doc()
        .pages
        .iter()
        .position(|p| p.id == page)
        .unwrap_or(0);
    let new_pages: Vec<tracedraw_core::PageId> = app.doc().pages[idx + 1..idx + 1 + rows.len()]
        .iter()
        .map(|p| p.id)
        .collect();
    let mut cmds = Vec::new();
    for (k, pid) in new_pages.iter().rev().enumerate() {
        let Some(row) = rows.get(k) else { continue };
        let Ok(p) = app.doc().page(*pid) else {
            continue;
        };
        let shapes: Vec<tracedraw_core::Shape> =
            p.layers.iter().flat_map(|l| l.shapes.clone()).collect();
        for s in shapes {
            if let ShapeKind::Text {
                spans,
                origin,
                frame,
                align,
                para,
                on_path,
            } = &s.kind
            {
                let mut changed = false;
                let new_spans: Vec<TextSpan> = spans
                    .iter()
                    .map(|sp| {
                        let mut t = sp.text.clone();
                        for (h, v) in headers.iter().zip(row) {
                            let ph = format!("<{h}>");
                            if t.contains(&ph) {
                                t = t.replace(&ph, v);
                                changed = true;
                            }
                        }
                        TextSpan {
                            text: t,
                            ..sp.clone()
                        }
                    })
                    .collect();
                if changed {
                    cmds.push(Command::SetShapeKind {
                        shape: s.id,
                        kind: ShapeKind::Text {
                            spans: new_spans,
                            origin: *origin,
                            frame: *frame,
                            align: *align,
                            para: para.clone(),
                            on_path: on_path.clone(),
                        },
                    });
                }
            }
        }
        cmds.push(Command::RenamePage {
            page: *pid,
            name: format!("{} {}", template.name, k + 1),
        });
    }
    if let Err(e) = app.engine.run_batch("Print Merge", &cmds) {
        app.status = e.to_string();
    }
    app.status = trf("status.merge_done", &[("n", &rows.len().to_string())]);
}

// ----- find and replace ----------------------------------------------------------

fn matches_text(hay: &str, needle: &str, match_case: bool, whole_word: bool) -> bool {
    if needle.is_empty() {
        return false;
    }
    let (h, n) = if match_case {
        (hay.to_string(), needle.to_string())
    } else {
        (hay.to_lowercase(), needle.to_lowercase())
    };
    if !whole_word {
        return h.contains(&n);
    }
    h.split(|c: char| !c.is_alphanumeric()).any(|w| w == n)
}

/// Find text objects containing `needle` (kind 0) or objects whose kind or
/// name matches it (kind 1).
pub fn find_shapes(
    app: &App,
    needle: &str,
    kind: usize,
    match_case: bool,
    whole_word: bool,
) -> Vec<ShapeId> {
    let Ok(page) = app.doc().page(app.page) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    fn walk(
        s: &tracedraw_core::Shape,
        needle: &str,
        kind: usize,
        mc: bool,
        ww: bool,
        out: &mut Vec<ShapeId>,
    ) {
        let hit = match (kind, &s.kind) {
            (0, ShapeKind::Text { spans, .. }) => {
                let t: String = spans.iter().map(|x| x.text.as_str()).collect();
                matches_text(&t, needle, mc, ww)
            }
            (1, k) => {
                let name = s.name.clone().unwrap_or_default();
                let kind_name = crate::ui::dockers::kind_name(k);
                matches_text(&name, needle, mc, ww) || matches_text(&kind_name, needle, mc, false)
            }
            _ => false,
        };
        if hit {
            out.push(s.id);
        }
        match &s.kind {
            ShapeKind::Group { children } => children
                .iter()
                .for_each(|c| walk(c, needle, kind, mc, ww, out)),
            ShapeKind::ClipFrame { frame, contents } => {
                walk(frame, needle, kind, mc, ww, out);
                contents
                    .iter()
                    .for_each(|c| walk(c, needle, kind, mc, ww, out));
            }
            _ => {}
        }
    }
    for s in page.layers.iter().flat_map(|l| &l.shapes) {
        walk(s, needle, kind, match_case, whole_word, &mut out);
    }
    // Only top-level ids are selectable; keep those.
    out.retain(|id| app.doc().shape(*id).is_ok());
    out
}

pub fn replace_text(
    app: &mut App,
    needle: &str,
    replacement: &str,
    match_case: bool,
    whole_word: bool,
    all: bool,
) {
    let ids = find_shapes(app, needle, 0, match_case, whole_word);
    let ids: Vec<ShapeId> = if all {
        ids
    } else {
        ids.into_iter().take(1).collect()
    };
    let mut cmds = Vec::new();
    for id in ids {
        let Some(s) = app.doc().find_shape(id).cloned() else {
            continue;
        };
        if let ShapeKind::Text {
            spans,
            origin,
            frame,
            align,
            para,
            on_path,
        } = s.kind
        {
            let new_spans: Vec<TextSpan> = spans
                .into_iter()
                .map(|sp| TextSpan {
                    text: replace_in(&sp.text, needle, replacement, match_case, whole_word),
                    ..sp
                })
                .collect();
            cmds.push(Command::SetShapeKind {
                shape: id,
                kind: ShapeKind::Text {
                    spans: new_spans,
                    origin,
                    frame,
                    align,
                    para,
                    on_path,
                },
            });
        }
    }
    if !cmds.is_empty() {
        if let Err(e) = app.engine.run_batch("Replace", &cmds) {
            app.status = e.to_string();
        }
    }
}

fn replace_in(
    text: &str,
    needle: &str,
    replacement: &str,
    match_case: bool,
    whole_word: bool,
) -> String {
    if needle.is_empty() {
        return text.to_string();
    }
    if whole_word {
        let mut out = String::new();
        let mut word = String::new();
        let flush = |word: &mut String, out: &mut String| {
            if !word.is_empty() {
                let eq = if match_case {
                    word == needle
                } else {
                    word.to_lowercase() == needle.to_lowercase()
                };
                out.push_str(if eq { replacement } else { word });
                word.clear();
            }
        };
        for c in text.chars() {
            if c.is_alphanumeric() {
                word.push(c);
            } else {
                flush(&mut word, &mut out);
                out.push(c);
            }
        }
        flush(&mut word, &mut out);
        return out;
    }
    if match_case {
        return text.replace(needle, replacement);
    }
    let lower = text.to_lowercase();
    let nl = needle.to_lowercase();
    let mut out = String::new();
    let mut i = 0;
    while let Some(pos) = lower[i..].find(&nl) {
        out.push_str(&text[i..i + pos]);
        out.push_str(replacement);
        i += pos + nl.len();
    }
    out.push_str(&text[i..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_export_preview_shows_the_page_and_estimates_the_size() {
        let mut app = App::headless();
        app.new_shape(ShapeKind::Rect {
            rect: Rect::new(10.0, 10.0, 100.0, 60.0),
            radius: 0.0,
            corners: None,
        });
        // PNG at 300 dpi: an A4 preview keeps the page's proportions.
        let st = ExportState {
            format: 2,
            dpi: 300.0,
            ..ExportState::default()
        };
        let pm = preview(&app, &st, 200).expect("preview");
        assert_eq!(pm.height(), 200);
        assert!((pm.width() as f64 - 200.0 * 210.0 / 297.0).abs() <= 1.0);
        let png = estimated_size(&app, &st, &pm).expect("estimate");
        assert!(png > 0);
        // The same page at 72 dpi is estimated smaller.
        let small = ExportState {
            dpi: 72.0,
            ..st.clone()
        };
        assert!(estimated_size(&app, &small, &pm).unwrap_or(usize::MAX) < png);
        // A vector format is measured, not estimated.
        let svg = ExportState {
            format: 0,
            ..st.clone()
        };
        assert!(estimated_size(&app, &svg, &pm).is_some());
        assert!(keeps_alpha("png") && !keeps_alpha("jpg"));
    }

    #[test]
    fn colours_are_mapped_inside_groups_frames_and_text() {
        use tracedraw_core::{document::TextSpan, Color, Fill, Rect, Shape, ShapeId};
        let red = Color::Rgb {
            r: 1.0,
            g: 0.0,
            b: 0.0,
        };
        let mut inner = Shape::new(
            ShapeId(2),
            ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, 1.0, 1.0),
                radius: 0.0,
                corners: None,
            },
        );
        inner.fill = Fill::Solid(red);
        let mut span = TextSpan::new("a", "Inter", 10.0);
        span.fill = Some(Fill::Solid(red));
        let text = Shape::new(
            ShapeId(3),
            ShapeKind::Text {
                spans: vec![span],
                origin: tracedraw_core::Point::ZERO,
                frame: None,
                align: Default::default(),
                para: Default::default(),
                on_path: None,
            },
        );
        let group = Shape::new(
            ShapeId(4),
            ShapeKind::Group {
                children: vec![inner, text],
            },
        );
        let frame = Shape::new(
            ShapeId(5),
            ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, 2.0, 2.0),
                radius: 0.0,
                corners: None,
            },
        );
        let mut clip = Shape::new(
            ShapeId(1),
            ShapeKind::ClipFrame {
                frame: Box::new(frame),
                contents: vec![group],
            },
        );
        let green = Color::Gray { v: 0.5 };
        map_colors(&mut clip, &|_| green);
        let ShapeKind::ClipFrame { contents, frame } = &clip.kind else {
            panic!("clip frame");
        };
        assert_eq!(frame.stroke.as_ref().map(|s| s.color), Some(green));
        let ShapeKind::Group { children } = &contents[0].kind else {
            panic!("group");
        };
        assert_eq!(children[0].fill, Fill::Solid(green));
        let ShapeKind::Text { spans, .. } = &children[1].kind else {
            panic!("text");
        };
        assert_eq!(spans[0].fill, Some(Fill::Solid(green)));
    }

    #[test]
    fn the_print_dialog_settings_make_the_layout() {
        let mut app = App::headless();
        let mut st = PrintState {
            layout_kind: 3,
            cols: 4,
            rows: 0,
            sheet: 2,
            landscape: true,
            crop_marks: true,
            ..Default::default()
        };
        let o = print_layout(&app, &st, app.doc());
        assert_eq!(
            o.layout,
            crate::imposition::Layout::Repeat { cols: 4, rows: 1 }
        );
        assert_eq!(o.sheet, Some(Size::new(297.0, 210.0)));
        assert!(o.marks.crop && !o.marks.registration);
        // Bleed only when asked for, from the document.
        let mut meta = app.doc().metadata.clone();
        meta.bleed = 3.0;
        app.run(tracedraw_core::Command::SetMetadata { metadata: meta });
        assert_eq!(print_layout(&app, &st, app.doc()).bleed, 0.0);
        st.bleed = true;
        assert_eq!(print_layout(&app, &st, app.doc()).bleed, 3.0);
        st.sheet = 0;
        assert_eq!(print_layout(&app, &st, app.doc()).sheet, None);
    }

    #[test]
    fn sizes_read_in_kilobytes_and_megabytes() {
        assert_eq!(human_size(10), "1 KB");
        assert_eq!(human_size(820 * 1024), "820 KB");
        assert_eq!(human_size(3 * 1024 * 1024 + 400 * 1024), "3.4 MB");
    }

    #[test]
    fn csv_parses_headers_and_rows() {
        let (h, r) = parse_csv("Name,City\nAna,Porto\nBob,Lisboa\n");
        assert_eq!(h, vec!["Name", "City"]);
        assert_eq!(r.len(), 2);
        assert_eq!(r[1][1], "Lisboa");
        let (h2, _) = parse_csv("a;b\n1;2\n");
        assert_eq!(h2, vec!["a", "b"]);
    }

    #[test]
    fn replace_variants() {
        assert_eq!(replace_in("Hello hello", "hello", "x", false, false), "x x");
        assert_eq!(
            replace_in("Hello hello", "hello", "x", true, false),
            "Hello x"
        );
        assert_eq!(
            replace_in("cat concat", "cat", "dog", false, true),
            "dog concat"
        );
    }
}
