//! Arcano edition: File > Imposition. The drawing's pages laid out on
//! printing sheets for the editora's own products, from a preset, with a
//! preview of the first sheet, made as a new drawing (to print or export
//! as PDF). The layout itself is the Print dialog's (`imposition.rs`):
//! this asks for the few things each product needs. See
//! docs/behavior/imposition-assistant.md.
//!
//! - Book signatures: perfect bound, signatures of a few sheets folded on
//!   their own; the sheet fits the pages (no scaling).
//! - Stapled booklet (notebooks, cadernetas): saddle stitch, with creep.
//! - Cards: each page repeated as many times as fit the sheet, with crop
//!   marks; or the pages in sequence (a deck), as many to a sheet.
//! - Labels and bookmarks: as cards, without gaps.

use crate::imposition::{Binding, Booklet, Layout, Marks, Options, MARK_MARGIN};
use serde::{Deserialize, Serialize};
use tracedraw_core::{geometry::Size, Document};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Produto {
    /// Book block in signatures, perfect bound.
    #[default]
    Cadernos,
    /// One booklet folded and stapled.
    Grampeado,
    /// Each page repeated over the sheet.
    Cartoes,
    /// The pages one after another, as many to a sheet (a deck of cards).
    Baralho,
    /// Each page repeated, no gaps.
    Etiquetas,
}

impl Produto {
    pub const ALL: [Produto; 5] = [
        Produto::Cadernos,
        Produto::Grampeado,
        Produto::Cartoes,
        Produto::Baralho,
        Produto::Etiquetas,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Produto::Cadernos => "impos.signatures",
            Produto::Grampeado => "impos.stapled",
            Produto::Cartoes => "impos.cards",
            Produto::Baralho => "impos.deck",
            Produto::Etiquetas => "impos.labels",
        }
    }

    fn booklet(self) -> bool {
        matches!(self, Produto::Cadernos | Produto::Grampeado)
    }
}

/// The printing sheets offered (name, width, height in mm, portrait).
pub const FOLHAS: [(&str, f64, f64); 5] = [
    ("A4", 210.0, 297.0),
    ("A3", 297.0, 420.0),
    ("SRA3", 320.0, 450.0),
    ("330 x 480", 330.0, 480.0),
    ("Ofício", 216.0, 330.0),
];

/// What the assistant was last set to (kept in the preferences).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ImposPrefs {
    pub produto: Produto,
    /// Index in [`FOLHAS`], or `None`: the sheet fits the pages.
    pub folha: Option<usize>,
    pub sangria: f64,
    pub espaco: f64,
    /// The printer's margin, where it does not print, mm.
    pub margem: f64,
    pub marcas: bool,
    /// Sheets in each signature (book signatures).
    pub folhas_caderno: u32,
    pub creep: f64,
}

impl Default for ImposPrefs {
    fn default() -> Self {
        ImposPrefs {
            produto: Produto::Cadernos,
            folha: Some(0),
            sangria: 3.0,
            espaco: 4.0,
            margem: 5.0,
            marcas: true,
            folhas_caderno: 4,
            creep: 0.0,
        }
    }
}

/// How many cells of `cell` fit across `room` with `gap` between them.
fn fit(room: f64, cell: f64, gap: f64) -> u32 {
    if cell <= 0.0 || room < cell {
        return 0;
    }
    (((room + gap) / (cell + gap)).floor() as u32).min(20)
}

/// The grid of `page` (with its bleed) that fits on `sheet`, and the
/// sheet turned the way more fit: (columns, rows, sheet).
pub fn grade(page: Size, sheet: Size, p: &ImposPrefs) -> (u32, u32, Size) {
    let bleed = if p.produto == Produto::Etiquetas { 0.0 } else { p.sangria.max(0.0) };
    let gap = if p.produto == Produto::Etiquetas { 0.0 } else { p.espaco.max(0.0) };
    let marks = if p.marcas { MARK_MARGIN } else { 0.0 };
    let edge = p.margem.max(0.0) + marks;
    let (w, h) = (page.width + 2.0 * bleed, page.height + 2.0 * bleed);
    let try_on = |s: Size| {
        let (rw, rh) = (s.width - 2.0 * edge, s.height - 2.0 * edge);
        (fit(rw, w, gap), fit(rh, h, gap), s)
    };
    let a = try_on(sheet);
    let b = try_on(Size::new(sheet.height, sheet.width));
    if b.0 * b.1 > a.0 * a.1 {
        b
    } else {
        a
    }
}

/// The layout options for `doc` set as `p`.
pub fn opcoes(doc: &Document, p: &ImposPrefs) -> Options {
    let first = doc.pages.first().map(|pg| pg.size).unwrap_or(Size::new(210.0, 297.0));
    let sheet = p.folha.and_then(|i| FOLHAS.get(i)).map(|(_, w, h)| Size::new(*w, *h));
    let marks = Marks {
        crop: p.marcas,
        ..Marks::default()
    };
    let mut o = Options {
        bleed: p.sangria.max(0.0),
        gutter: p.espaco.max(0.0),
        marks,
        keep_size: true,
        paper_margins: [p.margem.max(0.0); 4],
        title: doc.title.clone(),
        ..Options::default()
    };
    match p.produto {
        Produto::Cadernos | Produto::Grampeado => {
            o.layout = Layout::Booklet;
            o.booklet = Booklet {
                binding: if p.produto == Produto::Cadernos {
                    Binding::PerfectBound
                } else {
                    Binding::SaddleStitch
                },
                signature_sheets: p.folhas_caderno.clamp(1, 32),
                creep: if p.produto == Produto::Grampeado { p.creep.max(0.0) } else { 0.0 },
                ..Booklet::default()
            };
            // Two pages side by side: on a printing sheet when one is
            // chosen and they fit, else a sheet the size of the spread.
            o.sheet = sheet.and_then(|s| {
                let spread = Size::new(2.0 * first.width, first.height);
                let (c, r, turned) = grade(spread, s, &ImposPrefs { produto: Produto::Cartoes, ..p.clone() });
                (c >= 1 && r >= 1).then_some(turned)
            });
            o.gutter = 0.0;
        }
        Produto::Cartoes | Produto::Baralho | Produto::Etiquetas => {
            let s = sheet.unwrap_or(Size::new(210.0, 297.0));
            let (cols, rows, turned) = grade(first, s, p);
            let (cols, rows) = (cols.max(1), rows.max(1));
            o.sheet = Some(turned);
            o.layout = if p.produto == Produto::Baralho {
                Layout::NUp { cols, rows }
            } else {
                Layout::Repeat { cols, rows }
            };
            if p.produto == Produto::Etiquetas {
                o.gutter = 0.0;
                o.bleed = 0.0;
            }
        }
    }
    o
}

/// What the layout makes, for the dialog: sheets, pages a sheet and
/// whether anything runs off.
pub fn resumo(doc: &Document, p: &ImposPrefs) -> (usize, usize, bool, Option<Size>) {
    let o = opcoes(doc, p);
    let sizes: Vec<Size> = doc.pages.iter().map(|pg| pg.size).collect();
    let plans = crate::imposition::plan(&sizes, &o);
    let per = plans.first().map(|s| s.cells.len()).unwrap_or(0);
    let sheet = plans.first().map(|s| s.size);
    (plans.len(), per, crate::imposition::overflows(&plans), sheet)
}

// ----- the dialog -----------------------------------------------------------

/// The open dialog, with its preview.
pub struct ImposDialog {
    pub prefs: ImposPrefs,
    preview: Option<(ImposPrefs, egui::TextureHandle)>,
}

impl std::fmt::Debug for ImposDialog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImposDialog").field("prefs", &self.prefs).finish()
    }
}

impl Clone for ImposDialog {
    fn clone(&self) -> Self {
        ImposDialog {
            prefs: self.prefs.clone(),
            preview: None,
        }
    }
}

impl PartialEq for ImposDialog {
    fn eq(&self, other: &Self) -> bool {
        self.prefs == other.prefs
    }
}

impl crate::app::App {
    /// File > Imposition.
    pub fn open_impos_dialog(&mut self) {
        self.dialog = crate::ui::dialogs::Dialog::Impos(Box::new(ImposDialog {
            prefs: self.settings.imposicao.clone(),
            preview: None,
        }));
    }

    /// The drawing laid out as `p`, opened as a new drawing.
    pub fn create_imposed(&mut self, p: &ImposPrefs) {
        let doc = self.engine.document().clone();
        let mut o = opcoes(&doc, p);
        o.when = String::new();
        let mut out = crate::imposition::impose(&doc, &o);
        out.title = crate::i18n::trf("impos.titled", &[("name", &doc.title)]);
        let n = out.pages.len();
        self.open_document(out, None);
        self.settings.imposicao = p.clone();
        self.status = crate::i18n::trf("impos.created", &[("n", &n.to_string())]);
    }
}

fn tip(key: &str) -> String {
    crate::ui::tips::pair(key, &format!("{key}_desc"))
}

fn preview(ctx: &egui::Context, doc: &Document, p: &ImposPrefs) -> Option<egui::TextureHandle> {
    let o = opcoes(doc, p);
    let out = crate::imposition::impose(doc, &o);
    let page = out.pages.first()?;
    let long = page.size.width.max(page.size.height).max(1.0);
    let dpi = (360.0 * 25.4 / long).clamp(1.0, 300.0);
    let pm = tracedraw_render::render_page_image(&out, page.id, dpi)?;
    let image = egui::ColorImage::from_rgba_unmultiplied([pm.width() as usize, pm.height() as usize], pm.data());
    Some(ctx.load_texture("arcano_impos_preview", image, egui::TextureOptions::LINEAR))
}

pub fn dialog(app: &mut crate::app::App, ctx: &egui::Context, st: &mut ImposDialog, close: &mut bool) {
    use crate::i18n::{tr, trf};
    use crate::ui::tips::RichTip as _;
    let mut create = false;
    let has_doc = app.has_document() && !app.show_welcome;
    crate::ui::dialogs::window(ctx, tr("impos.title")).show(ctx, |ui| {
        if !has_doc {
            ui.label(tr("impos.no_drawing"));
            if ui.button(tr("dialog.close")).rich_tip(tip("impos.cancel")).clicked() {
                *close = true;
            }
            return;
        }
        let p = &mut st.prefs;
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                egui::Grid::new("impos_grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                    ui.label(tr("impos.product"));
                    ui.vertical(|ui| {
                        for pr in Produto::ALL {
                            ui.radio_value(&mut p.produto, pr, tr(pr.key())).rich_tip(tip(pr.key()));
                        }
                    });
                    ui.end_row();
                    ui.label(tr("impos.sheet"));
                    let shown = match p.folha.and_then(|i| FOLHAS.get(i)) {
                        Some((n, w, h)) => format!("{n} ({w} x {h} mm)"),
                        None => tr("impos.sheet_fit"),
                    };
                    egui::ComboBox::from_id_salt("impos_sheet")
                        .width(200.0)
                        .selected_text(shown)
                        .show_ui(ui, |ui| {
                            if p.produto.booklet() {
                                ui.selectable_value(&mut p.folha, None, tr("impos.sheet_fit"));
                            }
                            for (i, (n, w, h)) in FOLHAS.iter().enumerate() {
                                ui.selectable_value(&mut p.folha, Some(i), format!("{n} ({w} x {h} mm)"));
                            }
                        })
                        .response
                        .rich_tip(tip("impos.sheet"));
                    ui.end_row();
                    let field = |ui: &mut egui::Ui, v: &mut f64, key: &str| {
                        ui.add(
                            crate::ui::field::NumField::new(v)
                                .range(0.0..=100.0)
                                .max_decimals(1)
                                .suffix(" mm")
                                .width(80.0),
                        )
                        .rich_tip(tip(key));
                    };
                    if p.produto != Produto::Etiquetas {
                        ui.label(tr("impos.bleed"));
                        field(ui, &mut p.sangria, "impos.bleed");
                        ui.end_row();
                    }
                    if matches!(p.produto, Produto::Cartoes | Produto::Baralho) {
                        ui.label(tr("impos.gap"));
                        field(ui, &mut p.espaco, "impos.gap");
                        ui.end_row();
                    }
                    ui.label(tr("impos.margin"));
                    field(ui, &mut p.margem, "impos.margin");
                    ui.end_row();
                    if p.produto == Produto::Cadernos {
                        ui.label(tr("impos.signature_sheets"));
                        ui.add(crate::ui::field::NumField::new(&mut p.folhas_caderno).range(1..=32).width(80.0))
                            .rich_tip(tip("impos.signature_sheets"));
                        ui.end_row();
                    }
                    if p.produto == Produto::Grampeado {
                        ui.label(tr("impos.creep"));
                        field(ui, &mut p.creep, "impos.creep");
                        ui.end_row();
                    }
                    ui.label("");
                    ui.checkbox(&mut p.marcas, tr("impos.marks")).rich_tip(tip("impos.marks"));
                    ui.end_row();
                });
                let doc = app.engine.document();
                let (sheets, per, over, size) = resumo(doc, p);
                ui.separator();
                ui.label(egui::RichText::new(trf(
                    "impos.summary",
                    &[("sheets", &sheets.to_string()), ("per", &per.to_string())],
                )).strong());
                if let Some(s) = size {
                    ui.label(trf("impos.sheet_size", &[("w", &format!("{:.0}", s.width)), ("h", &format!("{:.0}", s.height))]));
                }
                if p.produto.booklet() {
                    let pages = doc.pages.len();
                    let padded = pages.div_ceil(4) * 4;
                    if padded != pages {
                        ui.label(trf("impos.blank_pages", &[("n", &(padded - pages).to_string())]));
                    }
                    ui.label(egui::RichText::new(tr("impos.duplex")).color(crate::theme::Tokens::text_dim()));
                }
                if over || per == 0 {
                    ui.label(egui::RichText::new(tr("impos.overflow")).color(egui::Color32::from_rgb(220, 140, 40)));
                }
            });
            // The first sheet, drawn again when a value changes.
            if st.preview.as_ref().map(|(k, _)| k) != Some(&*p) {
                st.preview = preview(ctx, app.engine.document(), p).map(|t| (p.clone(), t));
            }
            if let Some((_, tex)) = &st.preview {
                let size = tex.size_vec2();
                let scale = (260.0 / size.x.max(size.y)).min(1.0);
                ui.add(egui::Image::new(tex).fit_to_exact_size(size * scale))
                    .rich_tip(tip("impos.preview"));
            }
        });
        ui.separator();
        // A row of fixed width: a layout reaching for the room left would
        // grow the auto-sized window every frame.
        let create_label = tr("impos.create");
        let cancel_label = tr("dialog.cancel");
        match crate::ui::chrome::button_row(ui, &[&create_label, &cancel_label], false) {
            Some(0) => create = true,
            Some(_) => *close = true,
            None => {}
        }
    });
    if create {
        let p = st.prefs.clone();
        app.create_imposed(&p);
        *close = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(pages: usize, w: f64, h: f64) -> Document {
        let mut d = Document::new("t", Size::new(w, h));
        for _ in 1..pages {
            let _ = (tracedraw_core::Command::AddPage { name: None, size: Size::new(w, h), layer: None }).apply(&mut d);
        }
        d
    }

    #[test]
    fn cards_fill_the_sheet() {
        // 55 x 85 mm cards with 3 mm bleed, 4 mm gaps, on A4 with a 5 mm
        // margin and crop marks: 61 x 91 cells in 162 x 249 mm of room
        // gives 2 x 2 upright, 3 x 1... the larger count wins.
        let p = ImposPrefs { produto: Produto::Cartoes, ..ImposPrefs::default() };
        let (c, r, s) = grade(Size::new(55.0, 85.0), Size::new(210.0, 297.0), &p);
        assert!(c * r >= 4, "{c} x {r} on {s:?}");
        let d = doc(1, 55.0, 85.0);
        let (sheets, per, over, _) = resumo(&d, &p);
        assert_eq!(sheets, 1);
        assert_eq!(per as u32, c * r);
        assert!(!over);
        // A deck: 60 cards, as many to a sheet, in sequence.
        let d = doc(60, 55.0, 85.0);
        let p = ImposPrefs { produto: Produto::Baralho, ..p };
        let (sheets, per, _, _) = resumo(&d, &p);
        assert_eq!(sheets, 60usize.div_ceil(per));
    }

    #[test]
    fn labels_have_no_gaps_or_bleed() {
        let p = ImposPrefs { produto: Produto::Etiquetas, marcas: false, margem: 0.0, ..ImposPrefs::default() };
        let (c, r, _) = grade(Size::new(70.0, 37.0), Size::new(210.0, 297.0), &p);
        assert_eq!((c, r), (3, 8));
    }

    #[test]
    fn a_book_goes_in_signatures_of_whole_sheets() {
        // 30 A5 pages: padded to 32, on sheets two pages wide; four
        // sheets a signature make 16 pages, two signatures.
        let d = doc(30, 148.0, 210.0);
        let p = ImposPrefs { produto: Produto::Cadernos, folha: None, ..ImposPrefs::default() };
        let o = opcoes(&d, &p);
        assert_eq!(o.layout, Layout::Booklet);
        assert_eq!(o.booklet.binding, Binding::PerfectBound);
        let (sheets, per, over, size) = resumo(&d, &p);
        // Each sheet side holds two pages: 32 pages, 16 sides.
        assert_eq!(per, 2);
        assert_eq!(sheets, 16);
        assert!(!over);
        assert!(size.is_some_and(|s| s.width >= 296.0));
        let out = crate::imposition::impose(&d, &o);
        assert_eq!(out.pages.len(), 16);
    }

    #[test]
    fn nothing_fits_says_so() {
        let p = ImposPrefs { produto: Produto::Cartoes, ..ImposPrefs::default() };
        let (c, r, _) = grade(Size::new(400.0, 500.0), Size::new(210.0, 297.0), &p);
        assert_eq!(c * r, 0);
    }

    /// The window keeps its size frame after frame (a layout reaching for
    /// the room left grew it until it left the screen).
    #[test]
    fn the_dialogs_keep_their_size() {
        let mut app = crate::app::App::headless();
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600.0, 900.0));
        let mut widths = Vec::new();
        let mut impos = ImposDialog { prefs: ImposPrefs::default(), preview: None };
        let mut capa = crate::capa::CapaDialog { spec: Default::default(), tabela: false };
        for frame in 0..12 {
            let mut close = false;
            let _ = ctx.run_ui(egui::RawInput { screen_rect: Some(screen), ..Default::default() }, |ui| {
                let ctx = ui.ctx().clone();
                if frame < 6 {
                    dialog(&mut app, &ctx, &mut impos, &mut close);
                } else {
                    crate::capa::dialog(&mut app, &ctx, &mut capa, &mut close);
                }
            });
            let rect = ctx
                .memory(|m| m.layer_ids().find_map(|l| (l.order == egui::Order::Foreground).then(|| m.area_rect(l.id)).flatten()));
            widths.push(rect.map(|r| r.width()).unwrap_or(0.0));
        }
        for w in &widths {
            assert!(*w > 100.0 && *w < 1200.0, "{widths:?}");
        }
        assert!((widths[5] - widths[3]).abs() < 1.0, "{widths:?}");
        assert!((widths[11] - widths[9]).abs() < 1.0, "{widths:?}");
    }
}
