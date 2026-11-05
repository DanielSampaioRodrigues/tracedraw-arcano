//! Arcano edition: File > New Book Cover. The open cover of a book, laid
//! out from its trim size, its page count and paper, and its binding: the
//! spine worked out from the paper's thickness (the edition's spine
//! calculator), one page as wide as the open cover with the bleed around
//! it, guides on every fold, at the safety margin and, for a hard cover,
//! where the board ends, and a locked layer that does not print naming
//! each panel with its width. See docs/behavior/book-cover.md.
//!
//! Sheet thickness (mm) = grammage (g/m²) × bulk ÷ 1000. A sheet is two
//! pages. A paperback's spine adds its two covers' card; a hard cover's,
//! its two boards and the play the binder leaves.

use serde::{Deserialize, Serialize};
use tracedraw_core::{
    document::{Guide, GuideLine, Shape, ShapeKind},
    geometry::{Point, Size},
    Color, Command, Document, Fill, TextSpan,
};

/// A paper for the book block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Papel {
    pub id: String,
    pub nome: String,
    /// g/m².
    pub gram: f64,
    /// Bulk: cm³/g, the paper's "hand".
    pub volume: f64,
}

impl Papel {
    /// One sheet's thickness, mm.
    pub fn folha(&self) -> f64 {
        self.gram * self.volume / 1000.0
    }
}

/// A cover card or a board, by its thickness.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Material {
    pub id: String,
    pub nome: String,
    /// mm.
    pub esp: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Acabamento {
    /// Paperback (perfect binding).
    #[default]
    Brochura,
    /// Hard cover: boards covered with the printed sheet.
    Dura,
}

/// What the cover is made of and its sizes, mm.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CapaSpec {
    pub nome: String,
    pub paginas: u32,
    pub papel: String,
    pub acabamento: Acabamento,
    pub cartao: String,
    /// The house's own correction to a paperback's spine, mm (may be
    /// negative).
    pub ajuste: f64,
    pub papelao: String,
    /// Play between the block and the boards on a hard cover's spine.
    pub folga: f64,
    /// How far the board passes the block at the fore edge, head and tail.
    pub sobra: f64,
    /// The hinge between board and spine.
    pub canaleta: f64,
    /// The cover sheet turned over the boards' edges.
    pub virada: f64,
    /// Trim size, closed.
    pub largura: f64,
    pub altura: f64,
    pub sangria: f64,
    /// Flap width (paperback); 0: none.
    pub orelha: f64,
    /// Safety margin inside each panel.
    pub seg: f64,
}

impl Default for CapaSpec {
    fn default() -> Self {
        CapaSpec {
            nome: String::new(),
            paginas: 200,
            papel: "soft80".into(),
            acabamento: Acabamento::Brochura,
            cartao: "c300".into(),
            ajuste: 0.0,
            papelao: "p25".into(),
            folga: 1.0,
            sobra: 3.0,
            canaleta: 7.0,
            virada: 15.0,
            largura: 140.0,
            altura: 210.0,
            sangria: 3.0,
            orelha: 0.0,
            seg: 5.0,
        }
    }
}

/// The papers, cards and boards (editable in the dialog) and the last
/// cover made, kept in the preferences.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CapaPrefs {
    pub papeis: Vec<Papel>,
    pub cartoes: Vec<Material>,
    pub papeloes: Vec<Material>,
    pub ultima: CapaSpec,
}

fn papel(id: &str, nome: &str, gram: f64, volume: f64) -> Papel {
    Papel {
        id: id.into(),
        nome: nome.into(),
        gram,
        volume,
    }
}

fn material(id: &str, nome: &str, esp: f64) -> Material {
    Material {
        id: id.into(),
        nome: nome.into(),
        esp,
    }
}

impl Default for CapaPrefs {
    /// The edition's spine calculator's catalogue values.
    fn default() -> Self {
        CapaPrefs {
            papeis: vec![
                papel("soft70", "Pólen Soft 70 g", 70.0, 1.30),
                papel("soft80", "Pólen Soft 80 g", 80.0, 1.30),
                papel("nat70", "Pólen Natural Soft 70 g", 70.0, 1.25),
                papel("nat80", "Pólen Natural Soft 80 g", 80.0, 1.25),
                papel("off75", "Off-set branco 75 g", 75.0, 1.15),
                papel("off90", "Off-set branco 90 g", 90.0, 1.15),
                papel("bold70", "Pólen Bold 70 g", 70.0, 1.80),
                papel("bold90", "Pólen Bold 90 g", 90.0, 1.80),
            ],
            cartoes: vec![
                material("c250", "Cartão 250 g", 0.31),
                material("c300", "Cartão 300 g", 0.37),
                material("c350", "Cartão 350 g", 0.43),
            ],
            papeloes: vec![
                material("p19", "Papelão 1,9 mm", 1.9),
                material("p25", "Papelão 2,5 mm", 2.5),
                material("p30", "Papelão 3,0 mm", 3.0),
            ],
            ultima: CapaSpec::default(),
        }
    }
}

/// A panel of the open cover, left to right.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Painel {
    Orelha,
    Contracapa,
    Lombada,
    Capa,
    Canaleta,
    Virada,
}

impl Painel {
    /// The panel's name in the interface.
    pub fn nome(self) -> String {
        crate::i18n::tr(match self {
            Painel::Orelha => "capa.panel_flap",
            Painel::Contracapa => "capa.panel_back",
            Painel::Lombada => "capa.panel_spine",
            Painel::Capa => "capa.panel_front",
            Painel::Canaleta => "capa.panel_hinge",
            Painel::Virada => "capa.panel_turn_in",
        })
    }
}

/// The worked-out cover.
#[derive(Debug, Clone, PartialEq)]
pub struct Calculo {
    pub paginas: u32,
    pub folhas: u32,
    /// One sheet, mm.
    pub folha: f64,
    /// The book block, mm.
    pub miolo: f64,
    /// The spine, mm.
    pub lombada: f64,
    /// What the spine adds over the block: card or boards and play, and
    /// the house correction.
    pub acrescimo: f64,
    pub paineis: Vec<(Painel, f64)>,
    /// The open cover at the trim (the page), mm.
    pub largura: f64,
    pub altura: f64,
    /// The board of a hard cover, mm.
    pub papelao: Option<(f64, f64)>,
}

fn find<'a, T>(list: &'a [T], id: &str, key: impl Fn(&T) -> &str) -> Option<&'a T> {
    list.iter().find(|x| key(x) == id).or(list.first())
}

/// The cover `spec` makes with these papers, cards and boards.
pub fn calcular(spec: &CapaSpec, prefs: &CapaPrefs) -> Calculo {
    let folha = find(&prefs.papeis, &spec.papel, |p| &p.id)
        .map(Papel::folha)
        .unwrap_or(0.1)
        .max(0.0);
    let paginas = spec.paginas;
    let folhas = paginas.div_ceil(2);
    let miolo = folhas as f64 * folha;
    let pos = |v: f64| if v.is_finite() { v.max(0.0) } else { 0.0 };
    let (l, a) = (pos(spec.largura), pos(spec.altura));
    match spec.acabamento {
        Acabamento::Brochura => {
            let cartao = find(&prefs.cartoes, &spec.cartao, |m| &m.id).map(|m| m.esp).unwrap_or(0.0);
            let ajuste = if spec.ajuste.is_finite() { spec.ajuste } else { 0.0 };
            let lombada = (miolo + 2.0 * cartao + ajuste).max(0.0);
            let o = pos(spec.orelha);
            let mut paineis = Vec::new();
            if o > 0.0 {
                paineis.push((Painel::Orelha, o));
            }
            paineis.extend([(Painel::Contracapa, l), (Painel::Lombada, lombada), (Painel::Capa, l)]);
            if o > 0.0 {
                paineis.push((Painel::Orelha, o));
            }
            Calculo {
                paginas,
                folhas,
                folha,
                miolo,
                lombada,
                acrescimo: lombada - miolo,
                largura: paineis.iter().map(|p| p.1).sum(),
                altura: a,
                paineis,
                papelao: None,
            }
        }
        Acabamento::Dura => {
            let board = find(&prefs.papeloes, &spec.papelao, |m| &m.id).map(|m| m.esp).unwrap_or(0.0);
            let lombada = miolo + 2.0 * board + pos(spec.folga);
            let (bw, bh) = (l + pos(spec.sobra), a + 2.0 * pos(spec.sobra));
            let (v, cn) = (pos(spec.virada), pos(spec.canaleta));
            let paineis: Vec<(Painel, f64)> = [
                (Painel::Virada, v),
                (Painel::Contracapa, bw),
                (Painel::Canaleta, cn),
                (Painel::Lombada, lombada),
                (Painel::Canaleta, cn),
                (Painel::Capa, bw),
                (Painel::Virada, v),
            ]
            .into_iter()
            .filter(|p| p.1 > 0.0)
            .collect();
            Calculo {
                paginas,
                folhas,
                folha,
                miolo,
                lombada,
                acrescimo: lombada - miolo,
                largura: paineis.iter().map(|p| p.1).sum(),
                altura: bh + 2.0 * v,
                paineis,
                papelao: Some((bw, bh)),
            }
        }
    }
}

/// Millimetres the Brazilian way: "11,14".
pub fn mm(v: f64) -> String {
    format!("{v:.2}").replace('.', ",")
}

/// The cover as a drawing: one page the size of the open cover, the bleed
/// set, guides on the folds, at the safety margin and where the boards
/// end, a layer for the artwork and, above it, a locked layer that does
/// not print with each panel's name and width.
pub fn documento(spec: &CapaSpec, c: &Calculo, title: String) -> Document {
    let size = Size::new(c.largura.max(1.0), c.altura.max(1.0));
    let mut doc = crate::app::App::localized_document(title, size);
    doc.metadata.bleed = spec.sangria.max(0.0);
    let page = doc.pages[0].id;
    if let Some(l) = doc.pages[0].layers.first_mut() {
        l.name = crate::i18n::tr("capa.layer_art");
    }
    let mut guides: Vec<Guide> = Vec::new();
    let v = |x: f64| Guide::from(GuideLine::Vertical { x });
    let h = |y: f64| Guide::from(GuideLine::Horizontal { y });
    // The folds, between panels.
    let mut x = 0.0;
    let mut starts = Vec::new();
    for (i, (p, w)) in c.paineis.iter().enumerate() {
        starts.push((x, *p, *w));
        x += w;
        if i + 1 < c.paineis.len() {
            guides.push(v(x));
        }
    }
    let seg = spec.seg.max(0.0);
    // The vertical stretch the artwork has: inside the boards' turn-in on
    // a hard cover, the whole height on a paperback.
    let virada = if spec.acabamento == Acabamento::Dura { spec.virada.max(0.0) } else { 0.0 };
    let (bottom, top) = (virada, c.altura - virada);
    if virada > 0.0 {
        guides.push(h(bottom));
        guides.push(h(top));
    }
    if seg > 0.0 && top - bottom > 2.0 * seg {
        guides.push(h(bottom + seg));
        guides.push(h(top - seg));
    }
    // The safety margin inside the front and back panels.
    for (x0, p, w) in &starts {
        if matches!(p, Painel::Capa | Painel::Contracapa) && w > &(2.0 * seg) && seg > 0.0 {
            guides.push(v(x0 + seg));
            guides.push(v(x0 + w - seg));
        }
    }
    doc.pages[0].guides = guides;
    // The panels' names, on a layer that neither prints nor moves.
    let name = crate::i18n::tr("capa.layer_guides");
    if (Command::AddLayer { page, name: name.clone() }).apply(&mut doc).is_ok() {
        let layer_id = doc.pages[0].layers.last().map(|l| l.id);
        if let Some(layer_id) = layer_id {
            let gray = Color::rgb8(120, 120, 120);
            for (x0, p, w) in &starts {
                let label = format!("{}\n{} mm", p.nome(), mm(*w));
                for (n, line) in label.lines().enumerate() {
                    let id = doc.ids_mut().shape();
                    let mut s = Shape::new(
                        id,
                        ShapeKind::Text {
                            spans: vec![TextSpan::new(line, "Arial", 7.0)],
                            origin: Point::new(x0 + w / 2.0, c.altura / 2.0 - n as f64 * 3.2),
                            frame: None,
                            align: tracedraw_core::TextAlign::Center,
                            para: Default::default(),
                            on_path: None,
                        },
                    );
                    s.name = Some(p.nome());
                    s.fill = Fill::Solid(gray);
                    s.stroke = None;
                    let _ = (Command::AddShape { layer: layer_id, shape: s }).apply(&mut doc);
                }
            }
            if let Ok(l) = doc.layer_mut(layer_id) {
                l.printable = false;
                l.locked = true;
            }
        }
    }
    doc
}

/// What the cover's dialog warns about.
pub fn avisos(spec: &CapaSpec, c: &Calculo) -> Vec<String> {
    let mut out = Vec::new();
    if spec.paginas % 2 == 1 {
        out.push(crate::i18n::trf("capa.warn_odd", &[("n", &spec.paginas.to_string())]));
    }
    if c.lombada < 3.0 && spec.paginas > 0 {
        out.push(crate::i18n::tr("capa.warn_thin_spine"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn the_spine_is_the_calculators() {
        // 200 pages of Pólen Soft 80 g, paperback in 300 g card:
        // 100 sheets × 0,104 mm + 2 × 0,37 mm = 11,14 mm.
        let prefs = CapaPrefs::default();
        let spec = CapaSpec::default();
        let c = calcular(&spec, &prefs);
        assert_eq!(c.folhas, 100);
        assert!(close(c.folha, 0.104));
        assert!(close(c.miolo, 10.4));
        assert!(close(c.lombada, 11.14), "{}", c.lombada);
        // Back, spine, front: 140 + 11,14 + 140.
        assert!(close(c.largura, 291.14));
        assert!(close(c.altura, 210.0));
        assert_eq!(c.paineis.iter().map(|p| p.0).collect::<Vec<_>>(), vec![Painel::Contracapa, Painel::Lombada, Painel::Capa]);
        // An odd page count takes a whole sheet.
        let c = calcular(&CapaSpec { paginas: 201, ..spec.clone() }, &prefs);
        assert_eq!(c.folhas, 101);
        // Flaps on both sides, and the house correction.
        let c = calcular(&CapaSpec { orelha: 80.0, ajuste: 0.5, ..spec.clone() }, &prefs);
        assert!(close(c.lombada, 11.64));
        assert!(close(c.largura, 80.0 + 140.0 + 11.64 + 140.0 + 80.0));
        assert_eq!(c.paineis.first().map(|p| p.0), Some(Painel::Orelha));
        assert_eq!(c.paineis.last().map(|p| p.0), Some(Painel::Orelha));
    }

    #[test]
    fn a_hard_cover_has_boards_hinges_and_turn_ins() {
        // 200 pages of Pólen Bold 90 g: 100 × 0,162 = 16,2 mm; 2,5 mm
        // boards and 1 mm play: 22,2 mm.
        let prefs = CapaPrefs::default();
        let spec = CapaSpec {
            papel: "bold90".into(),
            acabamento: Acabamento::Dura,
            ..CapaSpec::default()
        };
        let c = calcular(&spec, &prefs);
        assert!(close(c.lombada, 22.2), "{}", c.lombada);
        // Boards: 140 + 3 wide, 210 + 6 high.
        assert_eq!(c.papelao, Some((143.0, 216.0)));
        // 15 + 143 + 7 + 22,2 + 7 + 143 + 15.
        assert!(close(c.largura, 352.2), "{}", c.largura);
        assert!(close(c.altura, 216.0 + 30.0));
    }

    #[test]
    fn the_drawing_has_the_folds_the_bleed_and_the_names() {
        let prefs = CapaPrefs::default();
        let spec = CapaSpec { orelha: 80.0, ..CapaSpec::default() };
        let c = calcular(&spec, &prefs);
        let doc = documento(&spec, &c, "Livro".into());
        let page = &doc.pages[0];
        assert!(close(page.size.width, c.largura));
        assert!(close(doc.metadata.bleed, 3.0));
        let xs: Vec<f64> = page
            .guides
            .iter()
            .filter_map(|g| match g.line {
                GuideLine::Vertical { x } => Some(x),
                _ => None,
            })
            .collect();
        // Four folds, and the safety margin inside front and back.
        for fold in [80.0, 220.0, 220.0 + c.lombada, 360.0 + c.lombada] {
            assert!(xs.iter().any(|x| close(*x, fold)), "{fold} in {xs:?}");
        }
        assert!(xs.iter().any(|x| close(*x, 85.0)) && xs.iter().any(|x| close(*x, 215.0)));
        // The art layer, and above it the names, locked and not printing.
        assert_eq!(page.layers.len(), 2);
        let names = &page.layers[1];
        assert!(!names.printable && names.locked);
        assert_eq!(names.shapes.len(), 2 * c.paineis.len());
        assert!(page.layers[0].printable && !page.layers[0].locked);
    }

    #[test]
    fn nothing_breaks_on_odd_values() {
        let prefs = CapaPrefs { papeis: Vec::new(), cartoes: Vec::new(), ..CapaPrefs::default() };
        let spec = CapaSpec {
            paginas: 0,
            largura: f64::NAN,
            sangria: -3.0,
            orelha: -10.0,
            papel: "none".into(),
            ..CapaSpec::default()
        };
        let c = calcular(&spec, &prefs);
        assert!(c.largura.is_finite() && c.lombada >= 0.0);
        let doc = documento(&spec, &c, "x".into());
        assert!(doc.pages[0].size.width >= 1.0);
        assert!(doc.metadata.bleed >= 0.0);
    }
}

// ----- the dialog -----------------------------------------------------------

/// The open File > New Book Cover dialog.
#[derive(Debug, Clone, PartialEq)]
pub struct CapaDialog {
    pub spec: CapaSpec,
    /// The papers, cards and boards table is shown.
    pub tabela: bool,
}

impl crate::app::App {
    /// File > New Book Cover: the dialog, with the last cover's values.
    pub fn open_capa_dialog(&mut self) {
        self.dialog = crate::ui::dialogs::Dialog::Capa(Box::new(CapaDialog {
            spec: self.settings.capa.ultima.clone(),
            tabela: false,
        }));
    }

    /// Make the cover `spec` describes as a new drawing.
    pub fn create_capa(&mut self, spec: &CapaSpec) {
        let c = calcular(spec, &self.settings.capa);
        let title = if spec.nome.trim().is_empty() {
            crate::i18n::tr("capa.untitled")
        } else {
            crate::i18n::trf("capa.titled", &[("name", spec.nome.trim())])
        };
        let doc = documento(spec, &c, title);
        self.open_document(doc, None);
        self.units = crate::app::Units::from_id("mm");
        self.settings.capa.ultima = spec.clone();
        self.status = crate::i18n::trf(
            "capa.created",
            &[("spine", &mm(c.lombada)), ("w", &mm(c.largura)), ("h", &mm(c.altura))],
        );
    }
}

fn tip(key: &str) -> String {
    crate::ui::tips::pair(key, &format!("{key}_desc"))
}

fn mm_field(ui: &mut egui::Ui, v: &mut f64, key: &str, max: f64) {
    use crate::ui::tips::RichTip as _;
    ui.add(
        crate::ui::field::NumField::new(v)
            .range(0.0..=max)
            .max_decimals(2)
            .suffix(" mm")
            .width(90.0),
    )
    .rich_tip(tip(key));
}

fn combo(ui: &mut egui::Ui, id: &str, value: &mut String, items: &[(String, String)], key: &str) {
    use crate::ui::tips::RichTip as _;
    let shown = items
        .iter()
        .find(|(i, _)| i == value)
        .or(items.first())
        .map(|(_, n)| n.clone())
        .unwrap_or_default();
    egui::ComboBox::from_id_salt(id)
        .width(200.0)
        .selected_text(shown)
        .show_ui(ui, |ui| {
            for (i, n) in items {
                ui.selectable_value(value, i.clone(), n);
            }
        })
        .response
        .rich_tip(tip(key));
}

/// The dialog: the cover's values on the left, what they make on the
/// right; Papers opens the table of papers, cards and boards.
pub fn dialog(app: &mut crate::app::App, ctx: &egui::Context, st: &mut CapaDialog, close: &mut bool) {
    use crate::i18n::{tr, trf};
    use crate::ui::tips::RichTip as _;
    let mut create = false;
    crate::ui::dialogs::window(ctx, tr("capa.title")).show(ctx, |ui| {
        let prefs = &mut app.settings.capa;
        let s = &mut st.spec;
        egui::Grid::new("capa_grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
            ui.label(tr("capa.name"));
            ui.add(egui::TextEdit::singleline(&mut s.nome).desired_width(220.0))
                .rich_tip(tip("capa.name"));
            ui.end_row();
            ui.label(tr("capa.trim"));
            ui.horizontal(|ui| {
                mm_field(ui, &mut s.largura, "capa.width", 2000.0);
                ui.label("×");
                mm_field(ui, &mut s.altura, "capa.height", 2000.0);
            });
            ui.end_row();
            ui.label(tr("capa.pages"));
            ui.add(crate::ui::field::NumField::new(&mut s.paginas).range(0..=5000).width(90.0))
                .rich_tip(tip("capa.pages"));
            ui.end_row();
            ui.label(tr("capa.paper"));
            let papers: Vec<(String, String)> =
                prefs.papeis.iter().map(|p| (p.id.clone(), p.nome.clone())).collect();
            combo(ui, "capa_paper", &mut s.papel, &papers, "capa.paper");
            ui.end_row();
            ui.label(tr("capa.binding"));
            ui.horizontal(|ui| {
                ui.radio_value(&mut s.acabamento, Acabamento::Brochura, tr("capa.paperback"))
                    .rich_tip(tip("capa.paperback"));
                ui.radio_value(&mut s.acabamento, Acabamento::Dura, tr("capa.hardcover"))
                    .rich_tip(tip("capa.hardcover"));
            });
            ui.end_row();
            match s.acabamento {
                Acabamento::Brochura => {
                    ui.label(tr("capa.card"));
                    let cards: Vec<(String, String)> = prefs
                        .cartoes
                        .iter()
                        .map(|m| (m.id.clone(), format!("{} ({} mm)", m.nome, mm(m.esp))))
                        .collect();
                    combo(ui, "capa_card", &mut s.cartao, &cards, "capa.card");
                    ui.end_row();
                    ui.label(tr("capa.flap"));
                    mm_field(ui, &mut s.orelha, "capa.flap", 500.0);
                    ui.end_row();
                    ui.label(tr("capa.adjust"));
                    ui.add(
                        crate::ui::field::NumField::new(&mut s.ajuste)
                            .range(-20.0..=20.0)
                            .max_decimals(2)
                            .suffix(" mm")
                            .width(90.0),
                    )
                    .rich_tip(tip("capa.adjust"));
                    ui.end_row();
                }
                Acabamento::Dura => {
                    ui.label(tr("capa.board"));
                    let boards: Vec<(String, String)> =
                        prefs.papeloes.iter().map(|m| (m.id.clone(), m.nome.clone())).collect();
                    combo(ui, "capa_board", &mut s.papelao, &boards, "capa.board");
                    ui.end_row();
                    for (v, key) in [
                        (&mut s.folga, "capa.play"),
                        (&mut s.sobra, "capa.overhang"),
                        (&mut s.canaleta, "capa.hinge"),
                        (&mut s.virada, "capa.turn_in"),
                    ] {
                        ui.label(tr(key));
                        mm_field(ui, v, key, 100.0);
                        ui.end_row();
                    }
                }
            }
            ui.label(tr("capa.bleed"));
            mm_field(ui, &mut s.sangria, "capa.bleed", 50.0);
            ui.end_row();
            ui.label(tr("capa.safety"));
            mm_field(ui, &mut s.seg, "capa.safety", 50.0);
            ui.end_row();
        });
        ui.separator();
        let c = calcular(s, prefs);
        ui.label(
            egui::RichText::new(trf("capa.result_spine", &[("mm", &mm(c.lombada))]))
                .strong()
                .size(16.0),
        );
        ui.label(trf(
            "capa.result_parts",
            &[
                ("sheets", &c.folhas.to_string()),
                ("sheet", &format!("{:.4}", c.folha).replace('.', ",")),
                ("block", &mm(c.miolo)),
                ("extra", &mm(c.acrescimo)),
            ],
        ));
        let sg = s.sangria.max(0.0);
        ui.label(trf(
            "capa.result_open",
            &[
                ("w", &mm(c.largura)),
                ("h", &mm(c.altura)),
                ("wb", &mm(c.largura + 2.0 * sg)),
                ("hb", &mm(c.altura + 2.0 * sg)),
            ],
        ));
        if let Some((bw, bh)) = c.papelao {
            ui.label(trf("capa.result_board", &[("w", &mm(bw)), ("h", &mm(bh))]));
        }
        ui.label(
            egui::RichText::new(
                c.paineis
                    .iter()
                    .map(|(p, w)| format!("{} {}", p.nome(), mm(*w)))
                    .collect::<Vec<_>>()
                    .join(" · "),
            )
            .color(crate::theme::Tokens::text_dim()),
        );
        for a in avisos(s, &c) {
            ui.label(egui::RichText::new(a).color(egui::Color32::from_rgb(220, 140, 40)));
        }
        if st.tabela {
            ui.separator();
            tabela(ui, prefs);
        }
        ui.separator();
        ui.horizontal(|ui| {
            if ui
                .button(tr(if st.tabela { "capa.papers_hide" } else { "capa.papers" }))
                .rich_tip(tip("capa.papers"))
                .clicked()
            {
                st.tabela = !st.tabela;
            }
        });
        // A row of fixed width (see imposicao.rs).
        let create_label = tr("capa.create");
        let cancel_label = tr("dialog.cancel");
        match crate::ui::chrome::button_row(ui, &[&create_label, &cancel_label], false) {
            Some(0) => create = true,
            Some(_) => *close = true,
            None => {}
        }
    });
    if create {
        let spec = st.spec.clone();
        app.create_capa(&spec);
        *close = true;
    }
}

/// The papers (name, grammage, bulk, sheet thickness), cards and boards,
/// editable; a paper can be added and removed.
fn tabela(ui: &mut egui::Ui, prefs: &mut CapaPrefs) {
    use crate::i18n::tr;
    use crate::ui::tips::RichTip as _;
    let mut remove = None;
    egui::Grid::new("capa_papers").num_columns(5).striped(true).show(ui, |ui| {
        for k in ["capa.col_name", "capa.col_gram", "capa.col_bulk", "capa.col_sheet"] {
            ui.label(egui::RichText::new(tr(k)).strong());
        }
        ui.label("");
        ui.end_row();
        for (i, p) in prefs.papeis.iter_mut().enumerate() {
            ui.add(egui::TextEdit::singleline(&mut p.nome).desired_width(170.0))
                .rich_tip(tip("capa.col_name"));
            ui.add(crate::ui::field::NumField::new(&mut p.gram).range(1.0..=1000.0).max_decimals(1).suffix(" g").width(70.0))
                .rich_tip(tip("capa.col_gram"));
            ui.add(crate::ui::field::NumField::new(&mut p.volume).range(0.1..=5.0).max_decimals(3).width(60.0))
                .rich_tip(tip("capa.col_bulk"));
            ui.label(format!("{:.4} mm", p.folha()).replace('.', ","));
            if ui.small_button("✕").rich_tip(tip("capa.remove")).clicked() {
                remove = Some(i);
            }
            ui.end_row();
        }
    });
    if let Some(i) = remove {
        if prefs.papeis.len() > 1 {
            prefs.papeis.remove(i);
        }
    }
    if ui.button(tr("capa.add_paper")).rich_tip(tip("capa.add_paper")).clicked() {
        let n = prefs.papeis.len() + 1;
        prefs.papeis.push(Papel {
            id: format!("u{n}-{}", prefs.papeis.iter().map(|p| p.id.len()).sum::<usize>()),
            nome: tr("capa.new_paper"),
            gram: 80.0,
            volume: 1.2,
        });
    }
    ui.add_space(4.0);
    egui::Grid::new("capa_materials").num_columns(2).show(ui, |ui| {
        for m in prefs.cartoes.iter_mut().chain(prefs.papeloes.iter_mut()) {
            ui.label(&m.nome);
            ui.add(crate::ui::field::NumField::new(&mut m.esp).range(0.01..=20.0).max_decimals(3).suffix(" mm").width(80.0))
                .rich_tip(tip("capa.material_thickness"));
            ui.end_row();
        }
    });
    if ui.button(tr("capa.reset_papers")).rich_tip(tip("capa.reset_papers")).clicked() {
        let d = CapaPrefs::default();
        prefs.papeis = d.papeis;
        prefs.cartoes = d.cartoes;
        prefs.papeloes = d.papeloes;
    }
}
