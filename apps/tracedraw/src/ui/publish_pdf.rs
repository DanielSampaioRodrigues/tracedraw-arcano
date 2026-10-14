//! File > Publish to PDF: which pages go in, the PDF standard, the
//! document information, bookmarks and password protection; OK asks for
//! the file. The PDF itself comes from `tracedraw_io::pdf`.

use crate::app::App;
use crate::i18n::{tr, trf};
use crate::theme::Tokens;
use crate::ui::dialogs::{window, ExportState, PDF_STANDARDS};
use egui::{Color32, Context, RichText, Ui};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracedraw_io::pdf::{PdfOptions, PdfStandard};
use tracedraw_io::pdf_crypt::{Permissions, Security};

/// Width of the dialog's pages.
const WIDTH: f32 = 420.0;
/// Height of the dialog's pages, so switching pages does not move the
/// buttons.
const HEIGHT: f32 = 340.0;
/// Width of the labels in the first column.
const LABEL_W: f32 = 140.0;
const ERROR: Color32 = Color32::from_rgb(0xC4, 0x2B, 0x1C);

/// Which pages go into the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Pages {
    #[default]
    All,
    Current,
    Selection,
}

/// How a protected file may be printed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Printing {
    No,
    LowResolution,
    #[default]
    Full,
}

impl Printing {
    const ALL: [Printing; 3] = [Printing::No, Printing::LowResolution, Printing::Full];
    fn key(self) -> &'static str {
        match self {
            Printing::No => "publish.printing_no",
            Printing::LowResolution => "publish.printing_low",
            Printing::Full => "publish.printing_full",
        }
    }
}

/// The dialog's choices kept in the preferences (`publish_pdf`); never
/// the passwords.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PublishPrefs {
    /// Index in `PDF_STANDARDS`: 0 plain PDF, then the PDF/X levels.
    pub standard: usize,
    pub output_condition: String,
    /// Resolution transparency is flattened at for PDF/X-1a and X-3.
    pub flatten_dpi: f64,
    pub bookmarks: bool,
    pub show_bookmarks: bool,
    pub printing: Printing,
    pub copy: bool,
    pub modify: bool,
    pub annotate: bool,
    pub fill_forms: bool,
    pub assemble: bool,
    pub accessibility: bool,
}

impl Default for PublishPrefs {
    fn default() -> Self {
        PublishPrefs {
            standard: 0,
            output_condition: "FOGRA39".into(),
            flatten_dpi: 300.0,
            bookmarks: true,
            show_bookmarks: false,
            printing: Printing::Full,
            copy: true,
            modify: true,
            annotate: true,
            fill_forms: true,
            assemble: true,
            accessibility: true,
        }
    }
}

impl PublishPrefs {
    pub fn standard(&self) -> PdfStandard {
        match self.standard {
            1 => PdfStandard::X1a,
            2 => PdfStandard::X3,
            3 => PdfStandard::X4,
            _ => PdfStandard::None,
        }
    }

    /// What a file protected with a permissions password allows.
    pub fn permissions(&self) -> Permissions {
        Permissions {
            print: self.printing != Printing::No,
            print_high_quality: self.printing == Printing::Full,
            copy: self.copy,
            modify: self.modify,
            annotate: self.annotate,
            fill_forms: self.fill_forms,
            assemble: self.assemble,
            accessibility: self.accessibility,
        }
    }
}

/// The dialog's pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    General,
    Document,
    Security,
}

/// The dialog's state.
#[derive(Clone, PartialEq)]
pub struct PublishState {
    pub tab: Tab,
    pub prefs: PublishPrefs,
    pub pages: Pages,
    pub bleed_mm: f64,
    pub title: String,
    pub author: String,
    pub subject: String,
    pub keywords: String,
    /// A password is needed to open the file.
    pub lock_open: bool,
    pub open_password: String,
    /// A second password guards printing, copying and changes.
    pub lock_permissions: bool,
    pub permissions_password: String,
    pub show_passwords: bool,
}

impl std::fmt::Debug for PublishState {
    // The passwords stay out of any log.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PublishState")
            .field("tab", &self.tab)
            .field("prefs", &self.prefs)
            .field("pages", &self.pages)
            .field("bleed_mm", &self.bleed_mm)
            .field("lock_open", &self.lock_open)
            .field("lock_permissions", &self.lock_permissions)
            .finish_non_exhaustive()
    }
}

impl PublishState {
    /// The dialog for the active drawing: its name, its properties and
    /// bleed, and the choices used last time.
    pub fn new(app: &App) -> PublishState {
        let m = &app.doc().metadata;
        PublishState {
            tab: Tab::General,
            prefs: app.settings.publish_pdf.clone(),
            pages: Pages::All,
            bleed_mm: m.bleed.max(0.0),
            title: app.document_name(),
            author: m.author.clone(),
            subject: m.subject.clone(),
            keywords: m.keywords.clone(),
            lock_open: false,
            open_password: String::new(),
            lock_permissions: false,
            permissions_password: String::new(),
            show_passwords: false,
        }
    }

    /// PDF/X files cannot be protected: the standards forbid encryption.
    pub fn can_protect(&self) -> bool {
        self.prefs.standard() == PdfStandard::None
    }

    /// Why the file cannot be written yet (an i18n key).
    pub fn problem(&self) -> Option<&'static str> {
        if !self.can_protect() {
            return None;
        }
        if self.lock_open && self.open_password.is_empty() {
            return Some("publish.need_open_password");
        }
        if self.lock_permissions && self.permissions_password.is_empty() {
            return Some("publish.need_permissions_password");
        }
        if self.lock_open
            && self.lock_permissions
            && self.open_password == self.permissions_password
        {
            return Some("publish.passwords_differ");
        }
        None
    }

    /// The protection asked for, if any. Without a permissions password
    /// whoever opens the file may do everything.
    pub fn security(&self) -> Option<Security> {
        if !self.can_protect() || !(self.lock_open || self.lock_permissions) {
            return None;
        }
        let user = if self.lock_open {
            self.open_password.clone()
        } else {
            String::new()
        };
        let (owner, permissions) = if self.lock_permissions {
            (self.permissions_password.clone(), self.prefs.permissions())
        } else {
            (user.clone(), Permissions::default())
        };
        Some(Security {
            user_password: user,
            owner_password: owner,
            permissions,
        })
    }

    /// The writer's options; `output_profile` is the PDF/X output intent.
    pub fn options(&self, output_profile: Option<Vec<u8>>) -> PdfOptions {
        PdfOptions {
            standard: self.prefs.standard(),
            output_profile,
            output_condition: self.prefs.output_condition.clone(),
            bleed_mm: self.bleed_mm.max(0.0),
            title: Some(self.title.clone()),
            flatten_dpi: self.prefs.flatten_dpi.max(72.0),
            author: Some(self.author.clone()),
            subject: Some(self.subject.clone()),
            keywords: Some(self.keywords.clone()),
            bookmarks: self.prefs.bookmarks,
            show_bookmarks: self.prefs.bookmarks && self.prefs.show_bookmarks,
            security: self.security(),
        }
    }
}

/// Write the PDF `st` describes to `path`; the status line's message.
pub fn publish(app: &App, st: &PublishState, path: &Path) -> Result<String, String> {
    let export = ExportState {
        all_pages: st.pages == Pages::All,
        selection_only: st.pages == Pages::Selection && !app.selection.is_empty(),
        ..Default::default()
    };
    let (doc, _) = crate::export::export_document(app, &export);
    let opts = st.options(crate::export::pdfx_profile(app, st.prefs.standard()));
    let pdf = tracedraw_io::pdf::document_to_pdf_with(&doc, &opts);
    crate::files::write(path, pdf).map_err(|e| e.to_string())?;
    Ok(trf(
        "status.exported",
        &[("path", &path.display().to_string())],
    ))
}

pub fn dialog(app: &mut App, ctx: &Context, st: &mut PublishState, close: &mut bool) {
    window(ctx, tr("publish.title")).show(ctx, |ui| {
        ui.horizontal(|ui| {
            for (tab, key) in [
                (Tab::General, "publish.tab_general"),
                (Tab::Document, "publish.tab_document"),
                (Tab::Security, "publish.tab_security"),
            ] {
                if ui.selectable_label(st.tab == tab, tr(key)).clicked() {
                    st.tab = tab;
                }
            }
        });
        ui.separator();
        ui.vertical(|ui| {
            ui.set_width(WIDTH);
            ui.set_min_height(HEIGHT);
            match st.tab {
                Tab::General => general(app, ui, st),
                Tab::Document => document(ui, st),
                Tab::Security => security(ui, st),
            }
        });
        // The line is there even when empty, so the dialog keeps its size.
        let problem = st.problem();
        let message = problem.map(tr).unwrap_or_else(|| " ".into());
        ui.label(RichText::new(message).color(ERROR));
        let ok_label = tr("dialog.ok");
        let cancel_label = tr("dialog.cancel");
        let mut ok = match crate::ui::chrome::button_row(ui, &[&ok_label, &cancel_label], false) {
            Some(0) => true,
            Some(_) => {
                *close = true;
                false
            }
            None => false,
        };
        if !*close && crate::ui::dialogs::enter_pressed(ui) {
            ok = true;
        }
        if !ok {
            return;
        }
        if problem.is_some() {
            // The passwords need fixing: show them.
            st.tab = Tab::Security;
            return;
        }
        *close = true;
        app.settings.publish_pdf = st.prefs.clone();
        let stem = app
            .file
            .as_ref()
            .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()))
            .unwrap_or_else(|| "Graphic1".into());
        crate::files::Dialog::new()
            .add_filter("PDF (*.pdf)", &["pdf"])
            .set_file_name(format!("{stem}.pdf"))
            .save_file(app, |app, path| {
                app.status = match publish(app, st, &path) {
                    Ok(msg) => msg,
                    Err(e) => trf("status.export_failed", &[("e", &e)]),
                };
            });
    });
}

/// A row of the form: a label of fixed width, then the field.
fn row(ui: &mut Ui, label: &str, add: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        label_cell(ui, label, LABEL_W);
        add(ui);
    });
}

/// A label on the left of a cell `width` wide.
fn label_cell(ui: &mut Ui, label: &str, width: f32) {
    ui.allocate_ui_with_layout(
        egui::vec2(width, 20.0),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.set_min_width(width);
            ui.label(label);
        },
    );
}

fn note(ui: &mut Ui, key: &str) {
    ui.label(RichText::new(tr(key)).color(Tokens::text_dim()).size(11.0));
}

/// General: the pages, the standard, the bleed and the PDF/X settings.
fn general(app: &App, ui: &mut Ui, st: &mut PublishState) {
    let pages = app.doc().pages.len();
    let has_selection = !app.selection.is_empty();
    if st.pages == Pages::Selection && !has_selection {
        st.pages = Pages::All;
    }
    ui.horizontal_top(|ui| {
        label_cell(ui, &tr("publish.pages"), LABEL_W);
        ui.vertical(|ui| {
            ui.radio_value(
                &mut st.pages,
                Pages::All,
                trf("publish.pages_all", &[("n", &pages.to_string())]),
            );
            ui.radio_value(&mut st.pages, Pages::Current, tr("publish.pages_current"));
            ui.add_enabled_ui(has_selection, |ui| {
                ui.radio_value(
                    &mut st.pages,
                    Pages::Selection,
                    tr("publish.pages_selection"),
                );
            });
        });
    });
    ui.add_space(4.0);
    row(ui, &tr("publish.compatibility"), |ui| {
        egui::ComboBox::from_id_salt("publish_standard")
            .selected_text(PDF_STANDARDS[st.prefs.standard.min(PDF_STANDARDS.len() - 1)])
            .width(170.0)
            .show_ui(ui, |ui| {
                for (i, n) in PDF_STANDARDS.iter().enumerate() {
                    ui.selectable_value(&mut st.prefs.standard, i, *n);
                }
            });
    });
    row(ui, &tr("dialog.bleed"), |ui| {
        ui.add(
            egui::DragValue::new(&mut st.bleed_mm)
                .range(0.0..=50.0)
                .speed(0.5)
                .suffix(" mm"),
        );
    });
    let standard = st.prefs.standard();
    if standard != PdfStandard::None {
        row(ui, &tr("dialog.output_condition"), |ui| {
            ui.add_sized(
                [170.0, 20.0],
                egui::TextEdit::singleline(&mut st.prefs.output_condition),
            );
        });
        if matches!(standard, PdfStandard::X1a | PdfStandard::X3) {
            row(ui, &tr("publish.flatten_resolution"), |ui| {
                ui.add(
                    egui::DragValue::new(&mut st.prefs.flatten_dpi)
                        .range(72.0..=1200.0)
                        .speed(5.0)
                        .suffix(" dpi"),
                );
            });
        }
        note(ui, "dialog.pdfx_profile_note");
    }
}

/// Document: the document information and the bookmarks.
fn document(ui: &mut Ui, st: &mut PublishState) {
    for (key, value) in [
        ("publish.document_title", &mut st.title),
        ("publish.author", &mut st.author),
        ("publish.subject", &mut st.subject),
        ("publish.keywords", &mut st.keywords),
    ] {
        row(ui, &tr(key), |ui| {
            ui.add_sized(
                [WIDTH - LABEL_W - 12.0, 20.0],
                egui::TextEdit::singleline(value),
            );
        });
    }
    note(ui, "publish.info_from_properties");
    ui.add_space(8.0);
    ui.checkbox(&mut st.prefs.bookmarks, tr("publish.bookmarks"));
    ui.add_enabled_ui(st.prefs.bookmarks, |ui| {
        ui.horizontal(|ui| {
            ui.add_space(22.0);
            ui.checkbox(&mut st.prefs.show_bookmarks, tr("publish.show_bookmarks"));
        });
    });
}

/// Security: the two passwords and what the second one guards.
fn security(ui: &mut Ui, st: &mut PublishState) {
    let can = st.can_protect();
    if !can {
        ui.label(RichText::new(tr("publish.pdfx_no_security")).color(ERROR));
        ui.add_space(4.0);
    }
    ui.add_enabled_ui(can, |ui| {
        let masked = !st.show_passwords;
        ui.checkbox(&mut st.lock_open, tr("publish.lock_open"));
        ui.add_enabled_ui(st.lock_open, |ui| {
            password_row(ui, &mut st.open_password, masked);
        });
        ui.checkbox(&mut st.lock_permissions, tr("publish.lock_permissions"));
        ui.add_enabled_ui(st.lock_permissions, |ui| {
            password_row(ui, &mut st.permissions_password, masked);
        });
        ui.checkbox(&mut st.show_passwords, tr("publish.show_passwords"));
        ui.add_space(4.0);
        ui.add_enabled_ui(st.lock_permissions, |ui| {
            ui.strong(tr("publish.permissions"));
            let p = &mut st.prefs;
            ui.horizontal(|ui| {
                ui.add_space(22.0);
                ui.label(tr("publish.printing"));
                egui::ComboBox::from_id_salt("publish_printing")
                    .selected_text(tr(p.printing.key()))
                    .width(150.0)
                    .show_ui(ui, |ui| {
                        for v in Printing::ALL {
                            ui.selectable_value(&mut p.printing, v, tr(v.key()));
                        }
                    });
            });
            egui::Grid::new("publish_permissions")
                .num_columns(2)
                .spacing([12.0, 2.0])
                .show(ui, |ui| {
                    let boxes = [
                        (&mut p.copy, "publish.allow_copy"),
                        (&mut p.modify, "publish.allow_modify"),
                        (&mut p.annotate, "publish.allow_annotate"),
                        (&mut p.fill_forms, "publish.allow_fill_forms"),
                        (&mut p.assemble, "publish.allow_assemble"),
                        (&mut p.accessibility, "publish.allow_accessibility"),
                    ];
                    for (n, (value, key)) in boxes.into_iter().enumerate() {
                        if n % 2 == 0 {
                            ui.horizontal(|ui| {
                                ui.add_space(22.0);
                                ui.checkbox(value, tr(key));
                            });
                        } else {
                            ui.checkbox(value, tr(key));
                            ui.end_row();
                        }
                    }
                });
        });
        ui.add_space(4.0);
        note(ui, "publish.encryption_note");
    });
}

fn password_row(ui: &mut Ui, value: &mut String, masked: bool) {
    ui.horizontal(|ui| {
        ui.add_space(22.0);
        label_cell(ui, &tr("publish.password"), LABEL_W - 22.0);
        ui.add_sized(
            [200.0, 20.0],
            egui::TextEdit::singleline(value).password(masked),
        );
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{vec2, Pos2, Rect};
    use tracedraw_core::{geometry::Size, Document};

    fn app_with_two_pages() -> App {
        let mut app = App::headless();
        let mut doc = Document::new("Leaflet", Size::new(100.0, 50.0));
        doc.metadata.author = "Ana".into();
        doc.metadata.keywords = "sale".into();
        doc.metadata.bleed = 3.0;
        let mut ids = doc.ids().clone();
        let mut second = doc.pages[0].clone();
        second.id = ids.page();
        second.name = "Back".into();
        doc.set_ids(ids);
        doc.pages.push(second);
        app.open_document(doc, None);
        app
    }

    #[test]
    fn the_dialog_starts_from_the_drawing_and_checks_the_passwords() {
        let app = app_with_two_pages();
        let mut st = PublishState::new(&app);
        assert_eq!(st.author, "Ana");
        assert_eq!(st.keywords, "sale");
        assert_eq!(st.bleed_mm, 3.0);
        assert!(st.problem().is_none() && st.security().is_none());
        // A password to open the file, nothing else: everything allowed.
        st.lock_open = true;
        assert_eq!(st.problem(), Some("publish.need_open_password"));
        st.open_password = "abc".into();
        assert!(st.problem().is_none());
        let sec = st.security().expect("protected");
        assert_eq!(sec.user_password, "abc");
        assert_eq!(sec.owner_password, "abc");
        assert_eq!(sec.permissions, Permissions::default());
        // A permissions password: the choices made apply.
        st.lock_permissions = true;
        assert_eq!(st.problem(), Some("publish.need_permissions_password"));
        st.permissions_password = "abc".into();
        assert_eq!(st.problem(), Some("publish.passwords_differ"));
        st.permissions_password = "owner".into();
        st.prefs.printing = Printing::LowResolution;
        st.prefs.copy = false;
        let sec = st.security().expect("protected");
        assert_eq!(sec.owner_password, "owner");
        assert!(sec.permissions.print && !sec.permissions.print_high_quality);
        assert!(!sec.permissions.copy && sec.permissions.modify);
        // Only a permissions password: the file opens without one.
        st.lock_open = false;
        let sec = st.security().expect("protected");
        assert!(sec.user_password.is_empty());
        // PDF/X: never protected, and no complaint about the passwords.
        st.lock_open = true;
        st.open_password.clear();
        st.prefs.standard = 3;
        assert!(st.problem().is_none() && st.security().is_none());
        assert!(st.options(None).security.is_none());
        // The passwords never reach a log.
        st.permissions_password = "hunter2".into();
        assert!(!format!("{st:?}").contains("hunter2"));
    }

    #[test]
    fn publishing_writes_the_pages_with_bookmarks_and_protection() {
        let mut app = app_with_two_pages();
        let mut st = PublishState::new(&app);
        st.prefs.show_bookmarks = true;
        st.subject = "Spring".into();
        let dir = std::env::temp_dir().join(format!("tracedraw-publish-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("leaflet.pdf");
        let msg = publish(&app, &st, &path).expect("published");
        assert!(msg.contains("leaflet.pdf"));
        // Every page, a bookmark each, the panel open, the information.
        let bytes = std::fs::read(&path).expect("written");
        let doc = lopdf::Document::load_mem(&bytes).expect("parse");
        assert_eq!(doc.get_pages().len(), 2);
        let titles: Vec<String> = doc
            .get_toc()
            .map(|toc| toc.toc.into_iter().map(|t| t.title).collect())
            .unwrap_or_default();
        assert_eq!(titles, vec!["Page 1".to_string(), "Back".to_string()]);
        let text = String::from_utf8_lossy(&bytes).into_owned();
        assert!(text.contains("/PageMode /UseOutlines"));
        assert!(text.contains("/Subject (Spring)") && text.contains("/Author (Ana)"));
        // Permissions only: protected, yet it opens without a password.
        st.lock_permissions = true;
        st.permissions_password = "owner".into();
        st.prefs.copy = false;
        publish(&app, &st, &path).expect("published");
        let bytes = std::fs::read(&path).expect("written");
        let text = String::from_utf8_lossy(&bytes).into_owned();
        assert!(text.starts_with("%PDF-1.7") && text.contains("/Encrypt "));
        assert!(!text.contains("Spring"));
        let mut ids = tracedraw_core::id::IdSource::default();
        let imp = tracedraw_io::pdf_import::parse(&bytes, &mut ids).expect("opens");
        assert_eq!(imp.pages.len(), 2);
        assert_eq!(imp.title.as_deref(), Some("Leaflet"));
        // A password to open it, and the current page alone.
        st.lock_open = true;
        st.open_password = "abc".into();
        st.pages = Pages::Current;
        publish(&app, &st, &path).expect("published");
        let bytes = std::fs::read(&path).expect("written");
        assert!(tracedraw_io::pdf_import::parse(&bytes, &mut ids).is_err());
        let imp = tracedraw_io::pdf_import::parse_with_password(&bytes, &mut ids, "abc")
            .expect("opens with its password");
        assert_eq!(imp.pages.len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
        // The dialog draws on each page.
        let ctx = crate::theme::ui_context();
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1600.0, 1000.0))),
            ..Default::default()
        };
        st.prefs.standard = 1;
        for tab in [Tab::General, Tab::Document, Tab::Security] {
            st.tab = tab;
            for _ in 0..2 {
                let mut out = ctx.run_ui(input.clone(), |ui| {
                    let mut close = false;
                    dialog(&mut app, ui.ctx(), &mut st, &mut close);
                    assert!(!close);
                });
                out.textures_delta.clear();
            }
        }
    }
}
