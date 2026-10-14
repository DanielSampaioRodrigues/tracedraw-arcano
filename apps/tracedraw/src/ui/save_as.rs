//! File > Save As (and Save of a drawing without a file): the Save
//! Drawing dialog. One window chooses the folder (path bar, Up, New folder,
//! shortcuts and the folder's contents), the file name, the type
//! (TraceDraw or `.cdr`), the `.cdr` version and the document's title,
//! subject and rating, and asks before replacing a file. In a browser,
//! where every save is a download, it keeps the name, type, version and
//! properties. See docs/behavior/files.md.

use crate::app::App;
use crate::files::{self, DirEntry, Place};
use crate::i18n::{tr, trf};
use crate::theme::Tokens;
use crate::ui::dialogs::{enter_pressed, window};
use crate::ui::options::{combo, grid, row_label};
use egui::{Context, RichText, Ui};
use std::path::{Path, PathBuf};
use tracedraw_core::Command;

const ERROR: egui::Color32 = egui::Color32::from_rgb(0xC4, 0x2B, 0x1C);
/// The dialog's width and the height of the folder list.
const WIDTH: f32 = 760.0;
const LIST_H: f32 = 250.0;
const PLACES_W: f32 = 150.0;

/// The file types the drawing can be saved as.
pub const TDRAW: &str = "tdraw";
pub const CDR: &str = "cdr";

/// The dialog's state.
#[derive(Debug, Clone, PartialEq)]
pub struct SaveAsState {
    /// The folder shown (desktop).
    pub dir: PathBuf,
    /// The path bar (a folder typed there opens with Enter).
    pub dir_text: String,
    /// The file name, with or without its extension.
    pub name: String,
    /// `.cdr` rather than TraceDraw.
    pub cdr: bool,
    pub version: u16,
    pub title: String,
    pub subject: String,
    pub rating: u8,
    /// What `dir` holds.
    pub entries: Vec<DirEntry>,
    /// A file that exists, waiting for Replace.
    pub replace: Option<PathBuf>,
    /// Why nothing was saved (no name, no such folder).
    pub note: Option<String>,
}

impl SaveAsState {
    /// Start in the drawing's folder (else the last one used, else the
    /// documents folder) with its name, its type (`.cdr` for a drawing
    /// that came from or went to one), its version and properties.
    pub fn new(app: &App) -> Self {
        let doc = app.doc();
        let file = app.file.as_ref();
        let dir = file
            .and_then(|f| f.parent())
            .map(Path::to_path_buf)
            .filter(|d| files::is_dir(d))
            .or_else(|| app.settings.last_dir.clone().filter(|d| files::is_dir(d)))
            .or_else(|| {
                files::places()
                    .into_iter()
                    .find(|(p, _)| *p == Place::Documents)
                    .map(|(_, d)| d)
            })
            .or_else(files::home_dir)
            .unwrap_or_default();
        let stem = file
            .and_then(|f| f.file_stem())
            .map(|s| s.to_string_lossy().to_string())
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| {
                let t = doc.title.trim();
                if t.is_empty() {
                    "Graphic1".to_string()
                } else {
                    t.to_string()
                }
            });
        let cdr = file.is_some_and(|f| crate::app::is_cdr_path(f)) || app.cdr_version.is_some();
        let mut st = SaveAsState {
            dir_text: dir.display().to_string(),
            dir: dir.clone(),
            name: String::new(),
            cdr,
            version: app.cdr_save_version(),
            title: doc.title.clone(),
            subject: doc.metadata.subject.clone(),
            rating: doc.metadata.rating,
            entries: Vec::new(),
            replace: None,
            note: None,
        };
        st.name = format!("{stem}.{}", st.extension());
        st.open_dir(dir);
        st
    }

    /// The chosen type's extension.
    pub fn extension(&self) -> &'static str {
        if self.cdr {
            CDR
        } else {
            TDRAW
        }
    }

    /// The name with the chosen type's extension (one it already has of
    /// either type is replaced).
    pub fn file_name(&self) -> String {
        let name = self.name.trim();
        let lower = name.to_ascii_lowercase();
        let stem = [TDRAW, CDR]
            .iter()
            .find_map(|ext| {
                lower
                    .strip_suffix(&format!(".{ext}"))
                    .map(|s| &name[..s.len()])
            })
            .unwrap_or(name);
        format!("{stem}.{}", self.extension())
    }

    /// The file to write: in the folder (desktop) or a download named so
    /// (browser).
    pub fn target(&self) -> PathBuf {
        if files::WEB {
            PathBuf::from("/download").join(self.file_name())
        } else {
            self.dir.join(self.file_name())
        }
    }

    /// Show `dir` (a folder that cannot be read shows empty).
    pub fn open_dir(&mut self, dir: PathBuf) {
        self.entries = files::list_entries(&dir);
        self.dir_text = dir.display().to_string();
        self.dir = dir;
        self.replace = None;
        self.note = None;
    }

    /// The type changed: the name takes its extension.
    pub fn set_cdr(&mut self, cdr: bool) {
        self.cdr = cdr;
        self.name = self.file_name();
        self.replace = None;
    }

    /// The entries shown: folders, and files of the chosen type.
    pub fn shown(&self) -> Vec<&DirEntry> {
        let ext = format!(".{}", self.extension());
        self.entries
            .iter()
            .filter(|e| e.is_dir || e.name.to_ascii_lowercase().ends_with(&ext))
            .collect()
    }
}

/// "Version 22" for the version lists.
pub fn version_label(version: u16) -> String {
    trf("dialog.cdr_version_item", &[("v", &version.to_string())])
}

/// The versions offered, newest first.
pub fn version_choices() -> Vec<u16> {
    tracedraw_cdr::write::versions().rev().collect()
}

impl App {
    /// File > Save As: the Save Drawing dialog.
    pub fn save_as_dialog(&mut self) {
        self.dialog = crate::ui::dialogs::Dialog::SaveAs(SaveAsState::new(self));
    }

    /// The dialog's Save: check the name and the folder, ask before
    /// replacing a file, keep the properties, then write. True when the
    /// dialog is done.
    pub fn finish_save_as(&mut self, st: &mut SaveAsState) -> bool {
        let stem_empty = st
            .file_name()
            .strip_suffix(&format!(".{}", st.extension()))
            .is_none_or(|s| s.trim().is_empty());
        if stem_empty {
            st.note = Some(tr("dialog.need_file_name"));
            return false;
        }
        if !files::WEB && !files::is_dir(&st.dir) {
            st.note = Some(tr("dialog.no_such_folder"));
            return false;
        }
        let path = st.target();
        if files::exists(&path) && st.replace.as_ref() != Some(&path) {
            st.replace = Some(path);
            return false;
        }
        // The properties go with the file (one undo step each).
        if st.title.trim() != self.doc().title {
            self.run(Command::SetTitle {
                title: st.title.trim().to_string(),
            });
        }
        let md = &self.doc().metadata;
        if md.subject != st.subject || md.rating != st.rating {
            let mut metadata = md.clone();
            metadata.subject = st.subject.clone();
            metadata.rating = st.rating.min(5);
            self.run(Command::SetMetadata { metadata });
        }
        if !files::WEB {
            self.settings.last_dir = Some(st.dir.clone());
        }
        if st.cdr {
            self.save_cdr_to(path, st.version);
        } else {
            self.save_to(path);
        }
        true
    }
}

/// The shortcut's name.
fn place_name(place: Place, path: &Path) -> String {
    match place {
        Place::Home => tr("dialog.place_home"),
        Place::Desktop => tr("dialog.place_desktop"),
        Place::Documents => tr("dialog.place_documents"),
        Place::Downloads => tr("dialog.place_downloads"),
        Place::Pictures => tr("dialog.place_pictures"),
        Place::Root => path.display().to_string(),
    }
}

/// A size as the list shows it: KB from 1 KB, MB from 1 MB.
pub fn size_text(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{} KB", bytes.div_ceil(1024))
    }
}

pub fn dialog(app: &mut App, ctx: &Context, st: &mut SaveAsState, close: &mut bool) {
    window(ctx, tr("dialog.save_drawing")).show(ctx, |ui| {
        ui.set_width(WIDTH);
        if !files::WEB {
            folder_bar(ui, st);
            ui.add_space(4.0);
            // The shortcuts and the list keep a fixed height (a vertical
            // separator would take all there is).
            ui.allocate_ui(egui::vec2(WIDTH, LIST_H), |ui| {
                ui.set_min_height(LIST_H);
                ui.set_max_height(LIST_H);
                ui.horizontal_top(|ui| {
                    places(ui, st);
                    ui.separator();
                    listing(ui, st);
                });
            });
            ui.add_space(6.0);
        }
        fields(ui, st);
        if let Some(note) = &st.note {
            ui.label(RichText::new(note.as_str()).color(ERROR));
        }
        if files::WEB {
            ui.label(RichText::new(tr("dialog.download_hint")).color(Tokens::text_dim()));
        }
        ui.add_space(6.0);
        if let Some(path) = st.replace.clone() {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            ui.label(trf("dialog.replace_question", &[("name", &name)]));
            let replace = tr("dialog.replace");
            let cancel = tr("dialog.cancel");
            match crate::ui::chrome::button_row(ui, &[&replace, &cancel], false) {
                Some(0) => {
                    if app.finish_save_as(st) {
                        *close = true;
                    }
                }
                Some(_) => st.replace = None,
                None => {}
            }
            return;
        }
        let save = tr("dialog.save");
        let cancel = tr("dialog.cancel");
        match crate::ui::chrome::button_row(ui, &[&save, &cancel], false) {
            Some(0) => {
                if app.finish_save_as(st) {
                    *close = true;
                }
            }
            Some(_) => *close = true,
            None => {
                if enter_pressed(ui) && app.finish_save_as(st) {
                    *close = true;
                }
            }
        }
    });
}

/// Up, the path bar and New folder.
fn folder_bar(ui: &mut Ui, st: &mut SaveAsState) {
    ui.horizontal(|ui| {
        let up = st.dir.parent().map(Path::to_path_buf);
        let resp = ui
            .add_enabled(
                up.is_some(),
                egui::Button::new("  ").min_size(egui::vec2(24.0, 20.0)),
            )
            .on_hover_text(tr("dialog.up_folder"));
        // An arrow pointing up.
        let c = resp.rect.center();
        let colour = if up.is_some() {
            Tokens::icon()
        } else {
            Tokens::text_dim()
        };
        let stroke = egui::Stroke::new(1.5, colour);
        ui.painter().line_segment(
            [c + egui::vec2(0.0, 5.0), c + egui::vec2(0.0, -5.0)],
            stroke,
        );
        ui.painter().line_segment(
            [c + egui::vec2(-4.0, -1.0), c + egui::vec2(0.0, -5.0)],
            stroke,
        );
        ui.painter().line_segment(
            [c + egui::vec2(4.0, -1.0), c + egui::vec2(0.0, -5.0)],
            stroke,
        );
        if resp.clicked() {
            if let Some(up) = up {
                st.open_dir(up);
            }
        }
        let width = ui.available_width() - 120.0;
        let bar = ui.add(
            egui::TextEdit::singleline(&mut st.dir_text)
                .id(egui::Id::new("save_as_path"))
                .desired_width(width.max(200.0)),
        );
        if bar.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            let typed = PathBuf::from(st.dir_text.trim());
            if files::is_dir(&typed) {
                st.open_dir(typed);
            } else {
                st.note = Some(tr("dialog.no_such_folder"));
            }
        }
        if ui.button(tr("dialog.new_folder")).clicked() {
            let base = tr("dialog.new_folder");
            let mut path = st.dir.join(&base);
            let mut n = 2;
            while path.exists() {
                path = st.dir.join(format!("{base} ({n})"));
                n += 1;
            }
            match files::create_dir(&path) {
                Ok(()) => st.open_dir(path),
                Err(e) => st.note = Some(e.to_string()),
            }
        }
    });
}

/// The shortcuts down the left.
fn places(ui: &mut Ui, st: &mut SaveAsState) {
    ui.vertical(|ui| {
        ui.set_width(PLACES_W);
        for (place, path) in files::places() {
            let name = place_name(place, &path);
            if ui
                .selectable_label(st.dir == path, name)
                .on_hover_text(path.display().to_string())
                .clicked()
            {
                st.open_dir(path);
            }
        }
    });
}

/// The folder's contents: a click on a file takes its name, a double
/// click on a folder opens it.
fn listing(ui: &mut Ui, st: &mut SaveAsState) {
    let tz = jiff::tz::TimeZone::system();
    let mut open: Option<PathBuf> = None;
    let mut pick: Option<String> = None;
    ui.vertical(|ui| {
        egui::ScrollArea::vertical()
            .id_salt("save_as_list")
            .max_height(LIST_H)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("save_as_grid")
                    .num_columns(4)
                    .striped(true)
                    .spacing([16.0, 4.0])
                    .show(ui, |ui| {
                        for head in [
                            "dialog.col_name",
                            "dialog.col_modified",
                            "dialog.col_type",
                            "dialog.col_size",
                        ] {
                            ui.label(RichText::new(tr(head)).color(Tokens::text_dim()));
                        }
                        ui.end_row();
                        for e in st.shown() {
                            let selected = !e.is_dir && e.name == st.name;
                            let r = ui
                                .horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 4.0;
                                    entry_icon(ui, e.is_dir);
                                    ui.selectable_label(selected, e.name.as_str())
                                })
                                .inner;
                            if r.double_clicked() && e.is_dir {
                                open = Some(e.path.clone());
                            } else if r.clicked() && !e.is_dir {
                                pick = Some(e.name.clone());
                            }
                            ui.label(
                                e.modified
                                    .map(|s| crate::time_tracker::date_time(s, &tz))
                                    .unwrap_or_default(),
                            );
                            ui.label(if e.is_dir {
                                tr("dialog.file_folder")
                            } else {
                                let ext = Path::new(&e.name)
                                    .extension()
                                    .map(|x| x.to_string_lossy().to_uppercase())
                                    .unwrap_or_default();
                                trf("dialog.file_of_type", &[("ext", &ext)])
                            });
                            ui.label(if e.is_dir {
                                String::new()
                            } else {
                                size_text(e.size)
                            });
                            ui.end_row();
                        }
                    });
            });
    });
    if let Some(dir) = open {
        st.open_dir(dir);
    }
    if let Some(name) = pick {
        st.name = name;
        st.replace = None;
    }
}

/// A small folder (a tab over a box), or a page with a folded corner.
fn entry_icon(ui: &mut Ui, folder: bool) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
    let p = ui.painter();
    let stroke = egui::Stroke::new(1.0, Tokens::icon());
    if folder {
        let fill = egui::Color32::from_rgb(0xF3, 0xC6, 0x5F);
        let body = egui::Rect::from_min_max(
            rect.min + egui::vec2(1.0, 4.0),
            rect.max - egui::vec2(1.0, 2.0),
        );
        let tab = egui::Rect::from_min_size(rect.min + egui::vec2(1.0, 2.0), egui::vec2(5.0, 3.0));
        p.rect_filled(tab, 1.0, fill);
        p.rect_filled(body, 1.0, fill);
        p.rect_stroke(body, 1.0, stroke, egui::StrokeKind::Inside);
    } else {
        let page = egui::Rect::from_min_max(
            rect.min + egui::vec2(3.0, 1.0),
            rect.max - egui::vec2(3.0, 1.0),
        );
        p.rect_filled(page, 0.0, egui::Color32::WHITE);
        p.rect_stroke(page, 0.0, stroke, egui::StrokeKind::Inside);
        let corner = page.right_top();
        p.line_segment(
            [
                corner + egui::vec2(-3.0, 0.0),
                corner + egui::vec2(-3.0, 3.0),
            ],
            stroke,
        );
        p.line_segment(
            [
                corner + egui::vec2(-3.0, 3.0),
                corner + egui::vec2(0.0, 3.0),
            ],
            stroke,
        );
    }
}

/// File name, type, version and the document's properties.
fn fields(ui: &mut Ui, st: &mut SaveAsState) {
    grid("save_as_fields").show(ui, |ui| {
        row_label(ui, &tr("dialog.file_name"));
        let name = ui.add(
            egui::TextEdit::singleline(&mut st.name)
                .id(egui::Id::new("save_as_name"))
                .desired_width(WIDTH - 140.0),
        );
        if name.changed() {
            st.replace = None;
            st.note = None;
        }
        ui.end_row();
        row_label(ui, &tr("dialog.save_as_type"));
        let mut cdr = st.cdr;
        if combo(
            ui,
            "save_as_type",
            &mut cdr,
            &[false, true],
            |cdr| {
                if cdr {
                    tr("file.cdr_files")
                } else {
                    "TraceDraw (*.tdraw)".to_string()
                }
            },
            260.0,
        ) {
            st.set_cdr(cdr);
        }
        ui.end_row();
        row_label(ui, &tr("dialog.cdr_version"));
        ui.horizontal(|ui| {
            ui.add_enabled_ui(st.cdr, |ui| {
                combo(
                    ui,
                    "save_as_version",
                    &mut st.version,
                    &version_choices(),
                    version_label,
                    260.0,
                );
            });
            if st.cdr {
                ui.label(
                    RichText::new(tr("dialog.cdr_version_hint"))
                        .color(Tokens::text_dim())
                        .size(11.0),
                );
            }
        });
        ui.end_row();
        row_label(ui, &tr("dialog.title"));
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut st.title).desired_width(220.0));
            ui.label(tr("dialog.subject"));
            ui.add(egui::TextEdit::singleline(&mut st.subject).desired_width(200.0));
            ui.label(tr("dialog.rating"));
            for i in 1..=5u8 {
                let colour = if st.rating >= i {
                    egui::Color32::from_rgb(0xE8, 0xA3, 0x17)
                } else {
                    egui::Color32::from_gray(0xC0)
                };
                let star = egui::RichText::new("\u{2605}").color(colour).size(15.0);
                if ui
                    .add(egui::Label::new(star).sense(egui::Sense::click()))
                    .clicked()
                {
                    st.rating = if st.rating == i { i - 1 } else { i };
                }
            }
        });
        ui.end_row();
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::dialogs::Dialog;
    use egui::{vec2, Pos2, Rect};
    use tracedraw_core::document::ShapeKind;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("tracedraw-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    /// Draw the open dialog for a frame, pressing Enter when `enter`.
    fn frame(ctx: &Context, app: &mut App, enter: bool) {
        let mut input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1400.0, 900.0))),
            ..Default::default()
        };
        if enter {
            input.events.push(egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            });
        }
        let mut out = ctx.run_ui(input, |ui| crate::ui::dialogs::show(app, ui.ctx()));
        out.textures_delta.clear();
    }

    fn version_of(path: &Path) -> Option<u16> {
        let bytes = std::fs::read(path).ok()?;
        let (_, report) = tracedraw_cdr::open_bytes(&bytes, "t").ok()?;
        report.version.map(|v| v.0)
    }

    fn drawing() -> App {
        let mut app = App::headless();
        app.new_shape(ShapeKind::Rect {
            rect: tracedraw_core::geometry::Rect::new(10.0, 10.0, 60.0, 40.0),
            radius: 0.0,
            corners: None,
        })
        .expect("a layer");
        app
    }

    fn state(app: &mut App) -> &mut SaveAsState {
        match &mut app.dialog {
            Dialog::SaveAs(st) => st,
            other => panic!("no Save Drawing dialog: {other:?}"),
        }
    }

    #[test]
    fn save_as_chooses_folder_name_type_version_and_properties() {
        let dir = temp_dir("save-as");
        std::fs::create_dir_all(dir.join("sub")).expect("folder");
        std::fs::write(dir.join("old.cdr"), b"x").expect("file");
        std::fs::write(dir.join("notes.txt"), b"x").expect("file");
        let mut app = drawing();
        app.save_as_dialog();
        let st = state(&mut app);
        st.open_dir(dir.clone());
        // Folders and files of the type, folders first.
        let names = |st: &SaveAsState| {
            st.shown()
                .iter()
                .map(|e| e.name.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(names(st), vec!["sub".to_string()], "TraceDraw files only");
        st.set_cdr(true);
        assert_eq!(names(st), vec!["sub".to_string(), "old.cdr".into()]);
        assert!(st.name.ends_with(".cdr"));
        st.name = "poster".into();
        st.version = 15;
        st.title = "Poster".into();
        st.subject = "Fair".into();
        st.rating = 4;
        let ctx = crate::theme::ui_context();
        frame(&ctx, &mut app, false);
        frame(&ctx, &mut app, true);
        assert!(matches!(app.dialog, Dialog::None), "{:?}", app.dialog);
        let path = dir.join("poster.cdr");
        assert_eq!(version_of(&path), Some(15));
        assert_eq!(app.doc().title, "Poster");
        assert_eq!(
            (
                app.doc().metadata.subject.as_str(),
                app.doc().metadata.rating
            ),
            ("Fair", 4)
        );
        assert_eq!(app.file.as_deref(), Some(path.as_path()));
        assert_eq!(app.settings.last_dir.as_deref(), Some(dir.as_path()));
        // The next Save As starts there, as .cdr, version 15.
        app.save_as_dialog();
        let st = state(&mut app);
        assert_eq!(
            (st.dir.as_path(), st.name.as_str(), st.cdr, st.version),
            (dir.as_path(), "poster.cdr", true, 15)
        );
        // An existing file asks first; Replace writes it.
        st.name = "old".into();
        frame(&ctx, &mut app, true);
        let st = state(&mut app);
        assert_eq!(st.replace.as_deref(), Some(dir.join("old.cdr").as_path()));
        assert_eq!(std::fs::read(dir.join("old.cdr")).unwrap_or_default(), b"x");
        let mut st = st.clone();
        assert!(app.finish_save_as(&mut st));
        assert_eq!(version_of(&dir.join("old.cdr")), Some(15));
        // TraceDraw files.
        app.save_as_dialog();
        let st = state(&mut app);
        st.set_cdr(false);
        st.name = "native.cdr".into();
        assert_eq!(st.file_name(), "native.tdraw");
        let mut st = st.clone();
        assert!(app.finish_save_as(&mut st));
        assert!(dir.join("native.tdraw").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn names_and_folders_are_checked() {
        let dir = temp_dir("save-as-checks");
        let mut app = drawing();
        app.save_as_dialog();
        let mut st = state(&mut app).clone();
        st.open_dir(dir.clone());
        st.name = "  ".into();
        assert!(!app.finish_save_as(&mut st));
        assert_eq!(st.note, Some(tr("dialog.need_file_name")));
        st.name = ".cdr".into();
        assert!(!app.finish_save_as(&mut st));
        st.name = "ok".into();
        st.dir = dir.join("missing");
        assert!(!app.finish_save_as(&mut st));
        assert_eq!(st.note, Some(tr("dialog.no_such_folder")));
        // Every way of typing the extension ends with the type's.
        st.cdr = true;
        for (typed, saved) in [
            ("a", "a.cdr"),
            ("a.CDR", "a.cdr"),
            ("a.tdraw", "a.cdr"),
            ("a.b", "a.b.cdr"),
        ] {
            st.name = typed.into();
            assert_eq!(st.file_name(), saved);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_dialog_draws_and_opens_folders() {
        let dir = temp_dir("save-as-draw");
        std::fs::create_dir_all(dir.join("inner")).expect("folder");
        let mut app = drawing();
        app.save_as_dialog();
        state(&mut app).open_dir(dir.clone());
        let ctx = crate::theme::ui_context();
        frame(&ctx, &mut app, false);
        frame(&ctx, &mut app, false);
        assert!(matches!(app.dialog, Dialog::SaveAs(_)));
        // New folder makes one and opens it; Up goes back.
        let st = state(&mut app);
        let made = dir.join(tr("dialog.new_folder"));
        files::create_dir(&made).expect("made");
        st.open_dir(made.clone());
        assert_eq!(st.dir, made);
        st.open_dir(dir.clone());
        assert_eq!(st.shown().len(), 2);
        assert_eq!(size_text(10), "1 KB");
        assert_eq!(size_text(3 * 1024 * 1024 + 1), "3.0 MB");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_drawing_opened_from_a_cdr_file_keeps_its_version() {
        let dir = temp_dir("save-as-keep");
        let old = dir.join("old.cdr");
        let doc = drawing().engine.document().clone();
        std::fs::write(&old, tracedraw_cdr::write::document_to_cdr_version(&doc, 9))
            .expect("written");
        let mut app = App::headless();
        app.settings.formats.cdr_version = 21;
        app.open_path(old.clone());
        assert_eq!(app.cdr_version, Some(9));
        app.save_as_dialog();
        let st = state(&mut app);
        assert!(st.cdr && st.version == 9, "{st:?}");
        // Each tab keeps its own: a new drawing offers the option's.
        app.new_document();
        app.save_as_dialog();
        let st = state(&mut app);
        assert!(!st.cdr && st.version == 21);
        app.switch_document(app.docs.len() - 2);
        app.save_as_dialog();
        let st = state(&mut app);
        st.open_dir(dir.clone());
        st.name = "copy".into();
        let mut st = st.clone();
        assert!(app.finish_save_as(&mut st));
        app.dialog = Dialog::None;
        assert_eq!(version_of(&dir.join("copy.cdr")), Some(9));
        // Ctrl+S writes the same version again, without asking.
        app.engine.mark_saved();
        app.save(false);
        assert!(matches!(app.dialog, Dialog::None));
        assert_eq!(version_of(&dir.join("copy.cdr")), Some(9));
        // Versions too old or too new to write become the nearest written.
        app.cdr_version = Some(3);
        assert_eq!(app.cdr_save_version(), tracedraw_cdr::write::OLDEST_VERSION);
        app.cdr_version = Some(99);
        assert_eq!(app.cdr_save_version(), tracedraw_cdr::write::NEWEST_VERSION);
        assert_eq!(version_choices().first(), Some(&22));
        assert!(version_label(15).contains("15"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn linux_folder_names_come_from_the_users_settings() {
        let home = Path::new("/home/ana");
        let dirs = files::parse_user_dirs(
            "# comment\nXDG_DESKTOP_DIR=\"$HOME/Área de Trabalho\"\nXDG_DOWNLOAD_DIR=\"/data/dl\"\n",
            home,
        );
        assert_eq!(
            dirs,
            vec![
                ("XDG_DESKTOP_DIR".to_string(), home.join("Área de Trabalho")),
                ("XDG_DOWNLOAD_DIR".to_string(), PathBuf::from("/data/dl")),
            ]
        );
    }
}
