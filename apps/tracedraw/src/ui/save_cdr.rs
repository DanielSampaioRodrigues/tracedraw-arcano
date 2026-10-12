//! Saving a `.cdr` file asks for the version to write: programs open
//! files of their own version and older ones, so the version decides who
//! can open the file. A browser has no save dialog to choose the file
//! type in (every save is a download), so there the dialog also chooses
//! between the native format and `.cdr`. See docs/behavior/files.md.

use crate::app::App;
use crate::i18n::{tr, trf};
use crate::theme::Tokens;
use crate::ui::dialogs::{ok_cancel, window};
use crate::ui::options::{combo, grid, row_label};
use egui::{Context, RichText};
use std::path::PathBuf;

/// The dialog's state.
#[derive(Debug, Clone, PartialEq)]
pub struct SaveCdrState {
    /// The file to write (in a browser, the name of the download).
    pub path: PathBuf,
    /// The version to write.
    pub version: u16,
    /// The format is chosen here too (a browser): `.cdr` when `cdr`,
    /// else the native format.
    pub choose_format: bool,
    pub cdr: bool,
}

impl SaveCdrState {
    /// Save As chose the `.cdr` file `path`.
    pub fn new(path: PathBuf, version: u16) -> Self {
        SaveCdrState {
            path,
            version,
            choose_format: false,
            cdr: true,
        }
    }

    /// A download named after `path`, offered as `.cdr` when `cdr` (the
    /// drawing came from a `.cdr` file), else in the native format.
    pub fn download(path: PathBuf, cdr: bool, version: u16) -> Self {
        SaveCdrState {
            path,
            version,
            choose_format: true,
            cdr,
        }
    }

    /// The file to write: with the chosen format's extension when the
    /// format is chosen here.
    pub fn target(&self) -> PathBuf {
        if !self.choose_format {
            return self.path.clone();
        }
        self.path
            .with_extension(if self.cdr { "cdr" } else { "tdraw" })
    }
}

/// "Version 22" for the version list.
pub fn version_label(version: u16) -> String {
    trf("dialog.cdr_version_item", &[("v", &version.to_string())])
}

/// The versions offered, newest first.
pub fn version_choices() -> Vec<u16> {
    tracedraw_cdr::write::versions().rev().collect()
}

pub fn dialog(app: &mut App, ctx: &Context, st: &mut SaveCdrState, close: &mut bool) {
    let title = if st.choose_format {
        tr("dialog.save")
    } else {
        tr("dialog.save_cdr")
    };
    window(ctx, title).show(ctx, |ui| {
        ui.set_max_width(380.0);
        grid("save_cdr").show(ui, |ui| {
            if st.choose_format {
                row_label(ui, &tr("dialog.format"));
                combo(
                    ui,
                    "save_cdr_format",
                    &mut st.cdr,
                    &[false, true],
                    |cdr| {
                        if cdr {
                            tr("file.cdr_files")
                        } else {
                            "TraceDraw (*.tdraw)".to_string()
                        }
                    },
                    200.0,
                );
                ui.end_row();
            }
            row_label(ui, &tr("dialog.cdr_version"));
            ui.add_enabled_ui(st.cdr, |ui| {
                combo(
                    ui,
                    "save_cdr_version",
                    &mut st.version,
                    &version_choices(),
                    version_label,
                    200.0,
                );
            });
            ui.end_row();
        });
        ui.add_space(4.0);
        ui.label(RichText::new(tr("dialog.cdr_version_hint")).color(Tokens::TEXT_DIM));
        if ok_cancel(ui, close) {
            let path = st.target();
            if st.cdr {
                app.save_cdr_to(path, st.version);
            } else {
                app.save_to(path);
            }
        }
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
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    /// Draw the open dialog for a frame, pressing Enter when `enter`.
    fn frame(ctx: &Context, app: &mut App, enter: bool) {
        let mut input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200.0, 800.0))),
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

    fn version_of(path: &std::path::Path) -> Option<u16> {
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

    #[test]
    fn saving_a_cdr_file_asks_for_the_version_and_keeps_it() {
        let dir = temp_dir("save-cdr");
        let path = dir.join("drawing.cdr");
        let mut app = drawing();
        let default = app.cdr_save_version();
        assert_eq!(default, tracedraw_cdr::write::DEFAULT_VERSION);
        // Save As to a .cdr file asks first, starting with the version
        // the drawing is saved in.
        app.save_as_to(path.clone());
        let Dialog::SaveCdr(st) = &mut app.dialog else {
            panic!("no version asked: {:?}", app.dialog);
        };
        assert_eq!((st.version, st.target()), (default, path.clone()));
        st.version = 17;
        let ctx = crate::theme::ui_context();
        frame(&ctx, &mut app, false);
        frame(&ctx, &mut app, true);
        assert!(matches!(app.dialog, Dialog::None));
        assert_eq!(version_of(&path), Some(17));
        assert_eq!(app.file.as_deref(), Some(path.as_path()));
        // Save writes the same version again, and new drawings start with
        // the version chosen last.
        app.engine.mark_saved();
        app.save(false);
        assert_eq!(version_of(&path), Some(17));
        assert_eq!(app.settings.formats.cdr_version, 17);
        app.new_document();
        assert_eq!(app.cdr_save_version(), 17);
        // Other files are saved without asking.
        let native = dir.join("drawing.tdraw");
        app.save_as_to(native.clone());
        assert!(matches!(app.dialog, Dialog::None));
        assert!(native.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_drawing_opened_from_a_cdr_file_keeps_its_version() {
        let dir = temp_dir("keep-cdr");
        let old = dir.join("old.cdr");
        let doc = drawing().engine.document().clone();
        std::fs::write(&old, tracedraw_cdr::write::document_to_cdr_version(&doc, 9))
            .expect("written");
        let mut app = App::headless();
        app.settings.formats.cdr_version = 21;
        app.open_path(old.clone());
        assert_eq!(app.cdr_version, Some(9));
        assert_eq!(app.cdr_save_version(), 9);
        // Each tab keeps its own version.
        app.new_document();
        assert_eq!(app.cdr_save_version(), 21);
        app.switch_document(app.docs.len() - 2);
        assert_eq!(app.cdr_save_version(), 9);
        let copy = dir.join("copy.cdr");
        app.save_as_to(copy.clone());
        assert!(matches!(&app.dialog, Dialog::SaveCdr(st) if st.version == 9));
        let ctx = crate::theme::ui_context();
        frame(&ctx, &mut app, false);
        frame(&ctx, &mut app, true);
        assert_eq!(version_of(&copy), Some(9));
        // Versions too old to write become the oldest written.
        app.cdr_version = Some(3);
        assert_eq!(app.cdr_save_version(), tracedraw_cdr::write::OLDEST_VERSION);
        app.cdr_version = Some(99);
        assert_eq!(app.cdr_save_version(), tracedraw_cdr::write::NEWEST_VERSION);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_download_chooses_its_format() {
        let mut st = SaveCdrState::download(PathBuf::from("/download/Plan.tdraw"), false, 12);
        assert_eq!(st.target(), PathBuf::from("/download/Plan.tdraw"));
        st.cdr = true;
        assert_eq!(st.target(), PathBuf::from("/download/Plan.cdr"));
        let st = SaveCdrState::new(PathBuf::from("/x/a.CDR"), 15);
        assert_eq!(st.target(), PathBuf::from("/x/a.CDR"));
        // The dialog draws in both forms.
        let mut app = drawing();
        let ctx = crate::theme::ui_context();
        for st in [
            SaveCdrState::download(PathBuf::from("/download/Plan.tdraw"), true, 22),
            SaveCdrState::new(PathBuf::from("/x/a.cdr"), 7),
        ] {
            app.dialog = Dialog::SaveCdr(st);
            frame(&ctx, &mut app, false);
            frame(&ctx, &mut app, false);
            assert!(matches!(app.dialog, Dialog::SaveCdr(_)));
        }
        assert_eq!(version_choices().first(), Some(&22));
        assert!(version_label(15).contains("15"));
    }
}
